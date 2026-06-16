//! W8 routing algorithm (ADR-0062): the executor picks the runner with
//! the highest historical `pass_rate / cost` from the `dispatch.choice`
//! audit history, and records the winner with rationale `pareto_winner`.

mod support;

use convergio_durability::audit::EntityKind;
use convergio_durability::{Durability, NewPlan, NewTask, TaskStatus};
use serde_json::json;
use support::fresh;

/// Seed one finished task dispatched to `runner_kind`, leaving a
/// `dispatch.choice` audit row joined to a terminal task status. This
/// is exactly the shape the live executor writes, so the routing query
/// reads real rows rather than a mock.
async fn seed_history(dur: &Durability, plan_id: &str, runner_kind: &str, status: TaskStatus) {
    let task = dur
        .create_task(
            plan_id,
            NewTask {
                wave: 1,
                sequence: 1,
                title: format!("history {runner_kind}"),
                description: None,
                evidence_required: vec![],
                runner_kind: Some(runner_kind.to_string()),
                profile: None,
                max_budget_usd: None,
            },
        )
        .await
        .unwrap();
    dur.audit()
        .append(
            EntityKind::Task,
            &task.id,
            "dispatch.choice",
            &json!({
                "runner_kind": runner_kind,
                "profile": serde_json::Value::Null,
                "rationale": "task_override",
                "plan_id": plan_id,
            }),
            None,
        )
        .await
        .unwrap();
    // `done` is Thor-only via the facade; set the terminal status
    // directly for the test fixture.
    sqlx::query("UPDATE tasks SET status = ? WHERE id = ?")
        .bind(status.as_str())
        .bind(&task.id)
        .execute(dur.pool().inner())
        .await
        .unwrap();
}

/// Append one extra `dispatch.choice` audit row for an existing task —
/// the shape a retry leaves behind.
async fn append_choice(dur: &Durability, task_id: &str, plan_id: &str, runner_kind: &str) {
    dur.audit()
        .append(
            EntityKind::Task,
            task_id,
            "dispatch.choice",
            &json!({
                "runner_kind": runner_kind,
                "profile": serde_json::Value::Null,
                "rationale": "pareto_winner",
                "plan_id": plan_id,
            }),
            None,
        )
        .await
        .unwrap();
}

async fn pending_task(dur: &Durability, plan_id: &str) -> String {
    dur.create_task(
        plan_id,
        NewTask {
            wave: 2,
            sequence: 1,
            title: "route me".into(),
            description: None,
            evidence_required: vec![],
            runner_kind: None, // no override → routing decides
            profile: None,
            max_budget_usd: None,
        },
    )
    .await
    .unwrap()
    .id
}

/// Read the `dispatch.choice` payload the executor wrote for `task_id`.
async fn choice_for(dur: &Durability, task_id: &str) -> serde_json::Value {
    let events = dur.audit().list_since(0, 10_000).await.unwrap();
    let row = events
        .iter()
        .rev()
        .find(|e| e.transition == "dispatch.choice" && e.entity_id == task_id)
        .expect("dispatch.choice row for routed task");
    serde_json::from_str(&row.payload).unwrap()
}

// Both scenarios live in one test: `CONVERGIO_EXECUTOR_USE_RUNNER` is
// process-global, so keeping them in a single function avoids a flaky
// cross-test env race (and the `await_holding_lock` a Mutex would
// trigger). Each scenario uses its own fresh DB.
#[tokio::test]
async fn routing_picks_pareto_winner_then_cold_start_default() {
    // --- Scenario 1: history present → highest pass_rate / cost wins.
    {
        let (exec, dur, _dir) = fresh().await;
        std::env::set_var("CONVERGIO_EXECUTOR_USE_RUNNER", "1");
        let plan = dur
            .create_plan(NewPlan {
                title: "routing".into(),
                description: None,
                project: None,
            })
            .await
            .unwrap();

        // claude:sonnet 2/2 done; copilot:gpt-5.2 0/2 done.
        seed_history(&dur, &plan.id, "claude:sonnet", TaskStatus::Done).await;
        seed_history(&dur, &plan.id, "claude:sonnet", TaskStatus::Done).await;
        seed_history(&dur, &plan.id, "copilot:gpt-5.2", TaskStatus::Failed).await;
        seed_history(&dur, &plan.id, "copilot:gpt-5.2", TaskStatus::Failed).await;

        let task_id = pending_task(&dur, &plan.id).await;
        // Spawn fails (no repo_path); the dispatch.choice row is written
        // before the spawn, which is what we assert.
        let _ = exec.tick().await;

        let payload = choice_for(&dur, &task_id).await;
        assert_eq!(payload["rationale"], "pareto_winner");
        assert_eq!(payload["runner_kind"], "claude:sonnet");
    }

    // --- Scenario 2: no history → compiled-in default fallback.
    {
        let (exec, dur, _dir) = fresh().await;
        std::env::set_var("CONVERGIO_EXECUTOR_USE_RUNNER", "1");
        let plan = dur
            .create_plan(NewPlan {
                title: "cold".into(),
                description: None,
                project: None,
            })
            .await
            .unwrap();

        let task_id = pending_task(&dur, &plan.id).await;
        let _ = exec.tick().await;

        let payload = choice_for(&dur, &task_id).await;
        assert_eq!(payload["rationale"], "default");
        assert_eq!(payload["runner_kind"], "claude:sonnet");
    }

    // --- Scenario 3: a retried task re-dispatched onto a different
    // runner leaves two dispatch.choice rows but one final status; only
    // the latest dispatch is credited (older runner must not steal the
    // success). All scenarios share one test to avoid a global-env race.
    {
        let (exec, dur, _dir) = fresh().await;
        std::env::set_var("CONVERGIO_EXECUTOR_USE_RUNNER", "1");
        let plan = dur
            .create_plan(NewPlan {
                title: "retry".into(),
                description: None,
                project: None,
            })
            .await
            .unwrap();

        // First dispatched to copilot (failed), then retried onto claude
        // (succeeded). Final status `done`.
        let retried = dur
            .create_task(
                &plan.id,
                NewTask {
                    wave: 1,
                    sequence: 1,
                    title: "retried".into(),
                    description: None,
                    evidence_required: vec![],
                    runner_kind: None,
                    profile: None,
                    max_budget_usd: None,
                },
            )
            .await
            .unwrap();
        append_choice(&dur, &retried.id, &plan.id, "copilot:gpt-5.2").await; // older
        append_choice(&dur, &retried.id, &plan.id, "claude:sonnet").await; // latest
        sqlx::query("UPDATE tasks SET status = 'done' WHERE id = ?")
            .bind(&retried.id)
            .execute(dur.pool().inner())
            .await
            .unwrap();

        // copilot's only row is the stale dispatch → not credited.
        let task_id = pending_task(&dur, &plan.id).await;
        let _ = exec.tick().await;

        let payload = choice_for(&dur, &task_id).await;
        assert_eq!(payload["rationale"], "pareto_winner");
        assert_eq!(payload["runner_kind"], "claude:sonnet");
    }

    std::env::remove_var("CONVERGIO_EXECUTOR_USE_RUNNER");
}
