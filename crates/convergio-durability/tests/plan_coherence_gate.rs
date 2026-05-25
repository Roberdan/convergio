//! Tests for `PlanCoherenceGate` (W4 / ADR-0055).

use convergio_db::Pool;
use convergio_durability::gates::{Gate, GateContext, PlanCoherenceGate};
use convergio_durability::store::PlanObjectiveStore;
use convergio_durability::{init, Durability, DurabilityError, NewPlan, NewTask, TaskStatus};
use tempfile::tempdir;

async fn fresh() -> (Durability, tempfile::TempDir) {
    std::env::set_var("CONVERGIO_REQUIRE_PLAN_OBJECTIVE", "1");
    let dir = tempdir().unwrap();
    let url = format!("sqlite://{}/state.db", dir.path().display());
    let pool = Pool::connect(&url).await.unwrap();
    init(&pool).await.unwrap();
    (Durability::new(pool), dir)
}

async fn make_task(dur: &Durability) -> (String, convergio_durability::Task) {
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
                title: "t".into(),
                description: None,
                evidence_required: vec![],
                runner_kind: None,
                profile: None,
                max_budget_usd: None,
            },
        )
        .await
        .unwrap();
    (plan.id, task)
}

fn ctx(dur: &Durability, task: convergio_durability::Task, target: TaskStatus) -> GateContext {
    GateContext {
        pool: dur.pool().clone(),
        task,
        target_status: target,
        agent_id: None,
    }
}

#[tokio::test]
async fn refuses_submit_when_objective_missing() {
    let (dur, _d) = fresh().await;
    let (_plan_id, task) = make_task(&dur).await;
    let err = PlanCoherenceGate::new()
        .check(&ctx(&dur, task, TaskStatus::Submitted))
        .await
        .unwrap_err();
    match err {
        DurabilityError::GateRefused { gate, reason } => {
            assert_eq!(gate, "plan_coherence");
            assert!(reason.contains("plan_missing_objective"), "got: {reason}");
        }
        other => panic!("expected GateRefused, got {other:?}"),
    }
}

#[tokio::test]
async fn allows_submit_when_objective_set() {
    let (dur, _d) = fresh().await;
    let (plan_id, task) = make_task(&dur).await;
    PlanObjectiveStore::new(dur.pool().clone())
        .set(&plan_id, "Ship v1.0 with zero P0 bugs")
        .await
        .unwrap();
    PlanCoherenceGate::new()
        .check(&ctx(&dur, task, TaskStatus::Submitted))
        .await
        .unwrap();
}

#[tokio::test]
async fn ignores_non_submit_transitions() {
    let (dur, _d) = fresh().await;
    let (_plan_id, task) = make_task(&dur).await;
    // No objective set, but moving to InProgress must not refuse.
    PlanCoherenceGate::new()
        .check(&ctx(&dur, task.clone(), TaskStatus::InProgress))
        .await
        .unwrap();
    PlanCoherenceGate::new()
        .check(&ctx(&dur, task, TaskStatus::Done))
        .await
        .unwrap();
}
