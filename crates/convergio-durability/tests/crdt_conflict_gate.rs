//! CRDT conflict gate tests.

use convergio_db::Pool;
use convergio_durability::{
    init, Durability, DurabilityError, NewCrdtOp, NewPlan, NewTask, TaskStatus,
};
use serde_json::json;
use tempfile::tempdir;

async fn fresh() -> (Durability, tempfile::TempDir) {
    let dir = tempdir().unwrap();
    let url = format!("sqlite://{}/state.db", dir.path().display());
    let pool = Pool::connect(&url).await.unwrap();
    init(&pool).await.unwrap();
    (Durability::new(pool), dir)
}

fn op(actor_id: &str, counter: i64, task_id: &str, value: &str) -> NewCrdtOp {
    NewCrdtOp {
        actor_id: actor_id.to_string(),
        counter,
        entity_type: "task".into(),
        entity_id: task_id.into(),
        field_name: "title".into(),
        crdt_type: "mv_register".into(),
        op_kind: "set".into(),
        value: json!(value),
        hlc: "2026-04-29T00:00:00Z/0".into(),
    }
}

#[tokio::test]
async fn unresolved_task_crdt_conflict_blocks_submit() {
    let (dur, _dir) = fresh().await;
    let plan = dur
        .create_plan(NewPlan {
            title: "crdt plan".into(),
            description: None,
            project: None,
        })
        .await
        .unwrap();
    let task = dur
        .create_task(
            &plan.id,
            NewTask {
                title: "merge title".into(),
                description: None,
                wave: 0,
                sequence: 0,
                evidence_required: vec![],
                runner_kind: None,
                profile: None,
                max_budget_usd: None,
            },
        )
        .await
        .unwrap();

    dur.crdt()
        .append_op(op("actor-a", 1, &task.id, "alpha"))
        .await
        .unwrap();
    dur.crdt()
        .append_op(op("actor-b", 1, &task.id, "beta"))
        .await
        .unwrap();
    dur.crdt()
        .merge_cell("task", &task.id, "title")
        .await
        .unwrap();

    let err = dur
        .transition_task(&task.id, TaskStatus::Submitted, Some("agent-crdt"))
        .await
        .unwrap_err();

    assert!(matches!(
        err,
        DurabilityError::GateRefused { gate, reason }
            if gate == "crdt_conflict" && reason.contains("title")
    ));
}
