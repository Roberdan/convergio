//! Integration test for `evidence.kind = "usage"` aggregation.

use convergio_db::Pool;
use convergio_durability::{init, Durability, NewAgent, NewPlan, NewTask, TaskStatus};
use serde_json::json;
use tempfile::tempdir;

async fn fresh() -> (Durability, tempfile::TempDir) {
    let dir = tempdir().unwrap();
    let url = format!("sqlite://{}/state.db", dir.path().display());
    let pool: Pool = Pool::connect(&url).await.unwrap();
    init(&pool).await.unwrap();
    (Durability::new(pool), dir)
}

#[tokio::test]
async fn usage_evidence_updates_agent_registry_metadata() {
    let (dur, _dir) = fresh().await;

    let agent_id = "agent-usage-1";
    dur.register_agent(NewAgent {
        id: agent_id.to_string(),
        kind: "claude".into(),
        name: None,
        host: None,
        capabilities: vec!["code".into()],
        metadata: json!({}),
    })
    .await
    .unwrap();

    let plan = dur
        .create_plan(NewPlan {
            title: "usage plan".into(),
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
                title: "usage task".into(),
                description: None,
                evidence_required: vec![],
                runner_kind: None,
                profile: None,
                max_budget_usd: None,
            },
        )
        .await
        .unwrap();

    // Attribute the task to the agent so evidence can aggregate into that agent's metadata.
    dur.transition_task(&task.id, TaskStatus::InProgress, Some(agent_id))
        .await
        .unwrap();

    dur.attach_evidence(
        &task.id,
        "usage",
        json!({
            "input_tokens": 10u64,
            "output_tokens": 5u64,
            "model": "opus",
            "cost_usd": 0.25,
        }),
        None,
    )
    .await
    .unwrap();

    let agent = dur.agents().get(agent_id).await.unwrap();
    let usage = &agent.metadata["usage"];
    assert_eq!(usage["calls"], 1);
    assert_eq!(usage["total_input_tokens"], 10);
    assert_eq!(usage["total_output_tokens"], 5);
    assert_eq!(usage["last_model"], "opus");
    assert_eq!(usage["by_model"]["opus"]["calls"], 1);

    // Second attach increments totals.
    dur.attach_evidence(
        &task.id,
        "usage",
        json!({
            "input_tokens": 2,
            "output_tokens": 3,
            "model": "opus",
            "cost_usd": 0.05,
        }),
        None,
    )
    .await
    .unwrap();

    let agent = dur.agents().get(agent_id).await.unwrap();
    let usage = &agent.metadata["usage"];
    assert_eq!(usage["calls"], 2);
    assert_eq!(usage["total_input_tokens"], 12);
    assert_eq!(usage["total_output_tokens"], 8);
    assert_eq!(usage["by_model"]["opus"]["input_tokens"], 12);
    assert_eq!(usage["by_model"]["opus"]["output_tokens"], 8);
}
