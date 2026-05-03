//! `WaveSequenceGate` direct unit tests, split out of `gates.rs` to
//! keep both files under the 300-line cap.
//!
//! The gate refuses an `in_progress` claim if any earlier-wave task
//! is still open. `done` and `failed` count as terminal — neither
//! blocks subsequent waves.

use convergio_db::Pool;
use convergio_durability::gates::{Gate, GateContext, WaveSequenceGate};
use convergio_durability::{init, Durability, DurabilityError, NewPlan, NewTask, TaskStatus};
use tempfile::tempdir;

async fn fresh() -> (Durability, tempfile::TempDir) {
    let dir = tempdir().unwrap();
    let url = format!("sqlite://{}/state.db", dir.path().display());
    let pool: Pool = Pool::connect(&url).await.unwrap();
    init(&pool).await.unwrap();
    (Durability::new(pool), dir)
}

fn ctx(dur: &Durability, task: convergio_durability::Task, target: TaskStatus) -> GateContext {
    GateContext {
        pool: dur.pool().clone(),
        task,
        target_status: target,
        agent_id: Some("agent-1".into()),
    }
}

#[tokio::test]
async fn refuses_when_earlier_wave_open() {
    let (dur, _dir) = fresh().await;
    let plan = dur
        .create_plan(NewPlan {
            title: "p".into(),
            description: None,
            project: None,
        })
        .await
        .unwrap();
    let _wave1_task = dur
        .create_task(
            &plan.id,
            NewTask {
                wave: 1,
                sequence: 1,
                title: "w1".into(),
                description: None,
                evidence_required: vec![],
                runner_kind: None,
                profile: None,
                max_budget_usd: None,
            },
        )
        .await
        .unwrap();
    let wave2_task = dur
        .create_task(
            &plan.id,
            NewTask {
                wave: 2,
                sequence: 1,
                title: "w2".into(),
                description: None,
                evidence_required: vec![],
                runner_kind: None,
                profile: None,
                max_budget_usd: None,
            },
        )
        .await
        .unwrap();

    let err = WaveSequenceGate
        .check(&ctx(&dur, wave2_task, TaskStatus::InProgress))
        .await
        .unwrap_err();
    assert!(matches!(
        err,
        DurabilityError::GateRefused {
            gate: "wave_sequence",
            ..
        }
    ));
}

#[tokio::test]
async fn treats_failed_as_terminal() {
    // A wave-1 task in `failed` should not block wave-2 progress.
    // failed is a terminal state — the plan accepted the failure or
    // moved on. Treating it as "still open" deadlocks plans whose
    // wave 1 contained an intentional probe or rejected task.
    let (dur, _dir) = fresh().await;
    let plan = dur
        .create_plan(NewPlan {
            title: "p".into(),
            description: None,
            project: None,
        })
        .await
        .unwrap();
    let wave1_task = dur
        .create_task(
            &plan.id,
            NewTask {
                wave: 1,
                sequence: 1,
                title: "w1-failed".into(),
                description: None,
                evidence_required: vec![],
                runner_kind: None,
                profile: None,
                max_budget_usd: None,
            },
        )
        .await
        .unwrap();
    dur.tasks()
        .set_status(&wave1_task.id, TaskStatus::InProgress, Some("a"))
        .await
        .unwrap();
    dur.tasks()
        .set_status(&wave1_task.id, TaskStatus::Failed, Some("a"))
        .await
        .unwrap();

    let wave2_task = dur
        .create_task(
            &plan.id,
            NewTask {
                wave: 2,
                sequence: 1,
                title: "w2".into(),
                description: None,
                evidence_required: vec![],
                runner_kind: None,
                profile: None,
                max_budget_usd: None,
            },
        )
        .await
        .unwrap();

    WaveSequenceGate
        .check(&ctx(&dur, wave2_task, TaskStatus::InProgress))
        .await
        .expect("wave-2 in_progress must pass when wave-1 has only done/failed tasks");
}

#[tokio::test]
async fn passes_for_first_wave() {
    let (dur, _dir) = fresh().await;
    let plan = dur
        .create_plan(NewPlan {
            title: "p".into(),
            description: None,
            project: None,
        })
        .await
        .unwrap();
    let task = dur
        .create_task(
            &plan.id,
            NewTask {
                wave: 1,
                sequence: 1,
                title: "w1".into(),
                description: None,
                evidence_required: vec![],
                runner_kind: None,
                profile: None,
                max_budget_usd: None,
            },
        )
        .await
        .unwrap();

    WaveSequenceGate
        .check(&ctx(&dur, task, TaskStatus::InProgress))
        .await
        .unwrap();
}
