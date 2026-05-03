//! Facade-level gate refusal audit tests.

use convergio_db::Pool;
use convergio_durability::{init, Durability, DurabilityError, NewPlan, NewTask, TaskStatus};
use tempfile::tempdir;

async fn fresh() -> (Durability, tempfile::TempDir) {
    let dir = tempdir().unwrap();
    let url = format!("sqlite://{}/state.db", dir.path().display());
    let pool: Pool = Pool::connect(&url).await.unwrap();
    init(&pool).await.unwrap();
    (Durability::new(pool), dir)
}

#[tokio::test]
async fn facade_persists_gate_refusal_for_explanation() {
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
                title: "needs evidence".into(),
                description: None,
                evidence_required: vec!["test".into()],
                runner_kind: None,
                profile: None,
                max_budget_usd: None,
            },
        )
        .await
        .unwrap();

    dur.transition_task(&task.id, TaskStatus::InProgress, Some("agent"))
        .await
        .unwrap();
    let err = dur
        .transition_task(&task.id, TaskStatus::Submitted, Some("agent"))
        .await
        .unwrap_err();
    assert!(matches!(
        err,
        DurabilityError::GateRefused {
            gate: "evidence",
            ..
        }
    ));

    let refusal = dur
        .audit()
        .latest_refusal(Some(&task.id))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(refusal.transition, "task.refused");
    assert_eq!(refusal.entity_id, task.id);
    assert!(refusal.payload.contains("\"gate\":\"evidence\""));
}
