//! Usage evidence aggregation tests.

use convergio_db::Pool;
use convergio_durability::{init, Durability, NewAgent, NewPlan, NewTask, TaskStatus};
use serde_json::json;
use tempfile::TempDir;

async fn fresh() -> (Durability, TempDir) {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("state.db");
    let pool = Pool::connect(&format!("sqlite://{}", db.display()))
        .await
        .unwrap();
    init(&pool).await.unwrap();
    (Durability::new(pool), dir)
}

fn agent(id: &str) -> NewAgent {
    NewAgent {
        id: id.into(),
        kind: "claude".into(),
        name: None,
        host: None,
        capabilities: vec!["code".into()],
        metadata: json!({"runner": "claude-shell-wrapper"}),
    }
}

#[tokio::test]
async fn usage_evidence_updates_agent_metadata_totals() {
    let (dur, _dir) = fresh().await;
    dur.register_agent(agent("agent-usage-1")).await.unwrap();

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

    // Simulate claim: set task.agent_id so aggregation has a target.
    dur.transition_task(&task.id, TaskStatus::InProgress, Some("agent-usage-1"))
        .await
        .unwrap();

    dur.attach_evidence(
        &task.id,
        "usage",
        json!({
            "input_tokens": 10,
            "output_tokens": 20,
            "model": "claude-opus-overnight",
            "cost_usd": 0.0123
        }),
        None,
    )
    .await
    .unwrap();

    let agent = dur.agents().get("agent-usage-1").await.unwrap();
    let usage = agent.metadata.get("usage").expect("usage object");
    assert_eq!(usage.get("calls").and_then(|v| v.as_i64()), Some(1));
    assert_eq!(
        usage.get("total_input_tokens").and_then(|v| v.as_i64()),
        Some(10)
    );
    assert_eq!(
        usage.get("total_output_tokens").and_then(|v| v.as_i64()),
        Some(20)
    );
    assert_eq!(usage.get("total_tokens").and_then(|v| v.as_i64()), Some(30));
    assert_eq!(
        usage.get("last_model").and_then(|v| v.as_str()),
        Some("claude-opus-overnight")
    );
    assert!(
        (usage
            .get("total_cost_usd")
            .and_then(|v| v.as_f64())
            .unwrap()
            - 0.0123)
            .abs()
            < 1e-9
    );

    let by_model = usage.get("by_model").and_then(|v| v.as_object()).unwrap();
    let m = by_model
        .get("claude-opus-overnight")
        .and_then(|v| v.as_object())
        .unwrap();
    assert_eq!(m.get("calls").and_then(|v| v.as_i64()), Some(1));
    assert_eq!(m.get("input_tokens").and_then(|v| v.as_i64()), Some(10));
    assert_eq!(m.get("output_tokens").and_then(|v| v.as_i64()), Some(20));
    assert_eq!(m.get("total_tokens").and_then(|v| v.as_i64()), Some(30));
}

#[tokio::test]
async fn usage_evidence_is_no_op_when_agent_missing() {
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
    dur.transition_task(&task.id, TaskStatus::InProgress, Some("missing-agent"))
        .await
        .unwrap();

    dur.attach_evidence(
        &task.id,
        "usage",
        json!({
            "input_tokens": 1,
            "output_tokens": 2,
            "model": "claude",
            "cost_usd": null
        }),
        None,
    )
    .await
    .unwrap();

    assert!(dur.audit().verify(None, None).await.unwrap().ok);
}
