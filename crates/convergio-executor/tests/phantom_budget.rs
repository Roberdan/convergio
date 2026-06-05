//! Regression test for issue #405.
//!
//! Phantom `in_progress` tasks (rows with `agent_id IS NULL` left over
//! from draft plans, manual SQL, or pre-claim crashes) must not be
//! counted against `CONVERGIO_EXECUTOR_MAX_PARALLEL` — otherwise the
//! dispatch budget saturates to zero and the executor stops firing
//! even though no agent is actually working.
//!
//! Lives in its own integration test file because `tick` reads a
//! process-wide environment variable; co-locating with other tick
//! tests caused races in CI.

mod support;

use convergio_durability::TaskStatus;
use support::fresh;

#[tokio::test]
async fn tick_ignores_phantom_in_progress_tasks_in_budget() {
    let (exec, dur, _dir) = fresh().await;
    let plan = dur
        .create_plan(convergio_durability::NewPlan {
            title: "p".into(),
            description: None,
            project: None,
        })
        .await
        .unwrap();

    // Two phantom tasks: status=in_progress, agent_id=NULL.
    // Simulates a draft plan whose rows were left in the bad state.
    for seq in 1..=2 {
        let phantom = dur
            .create_task(
                &plan.id,
                convergio_durability::NewTask {
                    wave: 1,
                    sequence: seq,
                    title: format!("phantom{seq}"),
                    description: None,
                    evidence_required: vec![],
                    runner_kind: None,
                    profile: None,
                    max_budget_usd: None,
                },
            )
            .await
            .unwrap();
        sqlx::query("UPDATE tasks SET status = 'in_progress', agent_id = NULL WHERE id = ?")
            .bind(&phantom.id)
            .execute(dur.pool().inner())
            .await
            .unwrap();
    }

    // One real pending task in the same wave.
    let real = dur
        .create_task(
            &plan.id,
            convergio_durability::NewTask {
                wave: 1,
                sequence: 3,
                title: "real".into(),
                description: None,
                evidence_required: vec![],
                runner_kind: None,
                profile: None,
                max_budget_usd: None,
            },
        )
        .await
        .unwrap();

    // Cap of 1. With the pre-fix SQL, the two phantoms would consume
    // the budget (saturating_sub → 0) and nothing dispatches.
    std::env::set_var("CONVERGIO_EXECUTOR_MAX_PARALLEL", "1");
    let dispatched = exec.tick().await;
    std::env::remove_var("CONVERGIO_EXECUTOR_MAX_PARALLEL");
    let dispatched = dispatched.unwrap();

    assert_eq!(dispatched, 1, "real task should have dispatched");
    let after = dur.tasks().get(&real.id).await.unwrap();
    assert_eq!(after.status, TaskStatus::InProgress);
    assert!(after.agent_id.is_some());
}
