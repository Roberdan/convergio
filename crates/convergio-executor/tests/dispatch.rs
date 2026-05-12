//! Executor integration tests.

mod support;

use support::fresh;

use convergio_durability::TaskStatus;
use convergio_executor::SpawnTemplate;
use convergio_planner::{Planner, PlannerMode};

#[tokio::test]
async fn tick_skips_failing_task_and_dispatches_next() {
    let (exec, dur, _dir) = fresh().await;
    let plan = dur
        .create_plan(convergio_durability::NewPlan {
            title: "p".into(),
            description: None,
            project: None,
        })
        .await
        .unwrap();

    let failing = dur
        .create_task(
            &plan.id,
            convergio_durability::NewTask {
                wave: 1,
                sequence: 1,
                title: "runner task without repo_path".into(),
                description: None,
                evidence_required: vec![],
                runner_kind: Some("copilot".into()),
                profile: None,
                max_budget_usd: None,
            },
        )
        .await
        .unwrap();
    let ok = dur
        .create_task(
            &plan.id,
            convergio_durability::NewTask {
                wave: 1,
                sequence: 2,
                title: "legacy task".into(),
                description: None,
                evidence_required: vec![],
                runner_kind: None,
                profile: None,
                max_budget_usd: None,
            },
        )
        .await
        .unwrap();

    let n = exec.tick().await.unwrap();
    assert_eq!(n, 1);

    // Audit follow-up 2026-05-12: atomic claim runs BEFORE spawn.
    // If the spawn fails, the executor compensates by transitioning
    // the (now-claimed) task to `Failed` instead of silently rolling
    // back to `Pending` — the operator can `cvg task retry` to put
    // it back into the pending queue.
    let failing_after = dur.tasks().get(&failing.id).await.unwrap();
    assert_eq!(failing_after.status, TaskStatus::Failed);

    let ok_after = dur.tasks().get(&ok.id).await.unwrap();
    assert_eq!(ok_after.status, TaskStatus::InProgress);
    assert!(ok_after.agent_id.is_some());
}

#[tokio::test]
async fn tick_skips_later_waves_until_earlier_done() {
    let (exec, dur, _dir) = fresh().await;
    let plan = dur
        .create_plan(convergio_durability::NewPlan {
            title: "p".into(),
            description: None,
            project: None,
        })
        .await
        .unwrap();
    let _w1 = dur
        .create_task(
            &plan.id,
            convergio_durability::NewTask {
                wave: 1,
                sequence: 1,
                title: "wave1".into(),
                description: None,
                evidence_required: vec![],
                runner_kind: None,
                profile: None,
                max_budget_usd: None,
            },
        )
        .await
        .unwrap();
    let w2 = dur
        .create_task(
            &plan.id,
            convergio_durability::NewTask {
                wave: 2,
                sequence: 1,
                title: "wave2".into(),
                description: None,
                evidence_required: vec![],
                runner_kind: None,
                profile: None,
                max_budget_usd: None,
            },
        )
        .await
        .unwrap();

    // First tick: only wave 1 dispatched.
    let n = exec.tick().await.unwrap();
    assert_eq!(n, 1);
    let after = dur.tasks().get(&w2.id).await.unwrap();
    assert_eq!(after.status, TaskStatus::Pending);

    // Second tick: wave 1 still in_progress so wave 2 still waits.
    let n = exec.tick().await.unwrap();
    assert_eq!(n, 0);
}

#[tokio::test]
async fn tick_dispatches_later_wave_after_earlier_failed() {
    let (exec, dur, _dir) = fresh().await;
    let plan = dur
        .create_plan(convergio_durability::NewPlan {
            title: "p".into(),
            description: None,
            project: None,
        })
        .await
        .unwrap();
    let w1 = dur
        .create_task(
            &plan.id,
            convergio_durability::NewTask {
                wave: 1,
                sequence: 1,
                title: "wave1".into(),
                description: None,
                evidence_required: vec![],
                runner_kind: None,
                profile: None,
                max_budget_usd: None,
            },
        )
        .await
        .unwrap();
    let w2 = dur
        .create_task(
            &plan.id,
            convergio_durability::NewTask {
                wave: 2,
                sequence: 1,
                title: "wave2".into(),
                description: None,
                evidence_required: vec![],
                runner_kind: None,
                profile: None,
                max_budget_usd: None,
            },
        )
        .await
        .unwrap();
    dur.transition_task(&w1.id, TaskStatus::InProgress, Some("agent-1"))
        .await
        .unwrap();
    dur.transition_task(&w1.id, TaskStatus::Failed, Some("agent-1"))
        .await
        .unwrap();

    let n = exec.tick().await.unwrap();
    assert_eq!(n, 1);
    let after = dur.tasks().get(&w2.id).await.unwrap();
    assert_eq!(after.status, TaskStatus::InProgress);
}

#[tokio::test]
async fn tick_does_not_steal_already_claimed_task() {
    let (exec, dur, _dir) = fresh().await;
    let planner = Planner::new(dur.clone()).with_mode(PlannerMode::Heuristic);
    let plan_id = planner.solve("claimed").await.unwrap();
    let task = dur.tasks().list_by_plan(&plan_id).await.unwrap().remove(0);
    dur.transition_task(&task.id, TaskStatus::InProgress, Some("manual-agent"))
        .await
        .unwrap();

    let n = exec.tick().await.unwrap();
    let after = dur.tasks().get(&task.id).await.unwrap();
    assert_eq!(n, 0);
    assert_eq!(after.status, TaskStatus::InProgress);
    assert_eq!(after.agent_id.as_deref(), Some("manual-agent"));
}

#[tokio::test]
async fn tick_is_idempotent_on_already_dispatched_tasks() {
    let (exec, dur, _dir) = fresh().await;
    let planner = Planner::new(dur.clone()).with_mode(PlannerMode::Heuristic);
    planner.solve("only one").await.unwrap();

    let n1 = exec.tick().await.unwrap();
    let n2 = exec.tick().await.unwrap();
    assert_eq!(n1, 1);
    assert_eq!(n2, 0);
}

#[tokio::test]
async fn tick_marks_task_failed_when_spawn_fails() {
    let (exec, dur, _dir) = support::fresh_with(SpawnTemplate {
        command: "/definitely-not-convergio-executor-test".into(),
        args: vec![],
        kind: "missing".into(),
    })
    .await;
    assert!(
        std::env::var("CONVERGIO_EXECUTOR_USE_RUNNER").is_err(),
        "test must control CONVERGIO_EXECUTOR_USE_RUNNER"
    );

    // The planner may set `runner_kind`, which would bypass the legacy
    // `SpawnTemplate` path. Create a task with `runner_kind: None` to
    // exercise the spawn-failure seam deterministically.
    let plan = dur
        .create_plan(convergio_durability::NewPlan {
            title: "p".into(),
            description: None,
            project: None,
        })
        .await
        .unwrap();
    let task = dur
        .create_task(
            &plan.id,
            convergio_durability::NewTask {
                wave: 1,
                sequence: 1,
                title: "spawn-failure".into(),
                description: None,
                evidence_required: vec![],
                runner_kind: None,
                profile: None,
                max_budget_usd: None,
            },
        )
        .await
        .unwrap();
    let fetched = dur.tasks().get(&task.id).await.unwrap();
    assert!(fetched.runner_kind.is_none(), "runner_kind must be NULL");

    let n = exec.tick().await.unwrap();
    assert_eq!(n, 0);
    // Audit follow-up 2026-05-12: spawn failure after atomic claim
    // compensates by transitioning to `Failed`. The agent_id from the
    // claim stays on the row so the operator can see which dispatch
    // tried it; `cvg task retry` puts it back to pending.
    let after = dur.tasks().get(&task.id).await.unwrap();
    assert_eq!(after.status, TaskStatus::Failed);
    assert!(after.agent_id.is_some());
    assert!(dur.audit().verify(None, None).await.unwrap().ok);
}

#[tokio::test]
async fn dispatch_writes_audit_chain_that_verifies() {
    let (exec, dur, _dir) = fresh().await;
    let planner = Planner::new(dur.clone()).with_mode(PlannerMode::Heuristic);
    planner.solve("x\ny").await.unwrap();
    exec.tick().await.unwrap();

    let r = dur.audit().verify(None, None).await.unwrap();
    assert!(r.ok, "{r:?}");
    // 1 plan.created + 2 task.created + 2 task.in_progress = 5+
    assert!(r.checked >= 5);
}
