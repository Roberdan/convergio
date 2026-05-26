//! W8 slice — `dispatch.choice` audit row is emitted on every spawn.

mod support;

use support::fresh;

#[tokio::test]
async fn legacy_spawn_records_dispatch_choice_audit_row() {
    let (exec, dur, _dir) = fresh().await;
    let plan = dur
        .create_plan(convergio_durability::NewPlan {
            title: "p".into(),
            description: None,
            project: None,
        })
        .await
        .unwrap();

    dur.create_task(
        &plan.id,
        convergio_durability::NewTask {
            wave: 1,
            sequence: 1,
            title: "legacy".into(),
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

    let events = dur.audit().list_since(0, 1000).await.unwrap();
    let choice = events
        .iter()
        .find(|e| e.transition == "dispatch.choice")
        .expect("dispatch.choice row");
    let payload: serde_json::Value = serde_json::from_str(&choice.payload).unwrap();
    assert_eq!(payload["runner_kind"], "legacy-shell");
    assert_eq!(payload["rationale"], "legacy");
    assert_eq!(payload["plan_id"], plan.id);
}
