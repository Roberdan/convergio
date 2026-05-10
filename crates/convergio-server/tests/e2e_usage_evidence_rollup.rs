//! E2E: `evidence.kind == "usage"` updates token/cost rollups.

mod common;

use reqwest::Client;
use serde_json::{json, Value};

#[tokio::test]
async fn usage_evidence_accumulates_task_and_recomputes_plan_and_agent_rollups() {
    let (base, _pool, _dir) = common::boot().await;
    let c = Client::new();

    // Plan + task.
    let plan: Value = c
        .post(format!("{base}/v1/plans"))
        .json(&json!({"title": "usage rollup plan"}))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let plan_id = plan["id"].as_str().unwrap();

    let task: Value = c
        .post(format!("{base}/v1/plans/{plan_id}/tasks"))
        .json(&json!({"title": "usage rollup task"}))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let task_id = task["id"].as_str().unwrap();

    // Register agent so the rollup update can persist.
    let agent_id = "test-agent-usage";
    let _agent: Value = c
        .post(format!("{base}/v1/agent-registry/agents"))
        .json(&json!({
            "id": agent_id,
            "kind": "test",
            "name": "usage test agent",
            "host": "e2e",
            "capabilities": [],
            "metadata": {}
        }))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();

    // Claim task (sets tasks.agent_id).
    let _: Value = c
        .post(format!("{base}/v1/tasks/{task_id}/transition"))
        .json(&json!({"target": "in_progress", "agent_id": agent_id}))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();

    // Attach usage evidence.
    let usage_cost = 0.001_f64;
    let usage_tokens = 30_i64;
    let attached: Value = c
        .post(format!("{base}/v1/tasks/{task_id}/evidence"))
        .json(&json!({
            "kind": "usage",
            "payload": {
                "input_tokens": 10,
                "output_tokens": 20,
                "model": "copilot:gpt-5.2",
                "cost_usd": usage_cost
            },
            "exit_code": 0
        }))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(attached["kind"], "usage");

    // Task rollup updated.
    let got_task: Value = c
        .get(format!("{base}/v1/tasks/{task_id}"))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(got_task["tokens"].as_i64().unwrap(), usage_tokens);
    let got_cost = got_task["cost_usd"].as_f64().unwrap();
    assert!(
        (got_cost - usage_cost).abs() < 1e-9,
        "task cost: {got_cost}"
    );

    // Plan rollup recomputed.
    let got_plan: Value = c
        .get(format!("{base}/v1/plans/{plan_id}"))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(got_plan["tokens"].as_i64().unwrap(), usage_tokens);
    let plan_cost = got_plan["cost_usd"].as_f64().unwrap();
    assert!(
        (plan_cost - usage_cost).abs() < 1e-9,
        "plan cost: {plan_cost}"
    );

    // Agent rollup recomputed.
    let got_agent: Value = c
        .get(format!("{base}/v1/agent-registry/agents/{agent_id}"))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(got_agent["tokens"].as_i64().unwrap(), usage_tokens);
    let agent_cost = got_agent["cost_usd"].as_f64().unwrap();
    assert!(
        (agent_cost - usage_cost).abs() < 1e-9,
        "agent cost: {agent_cost}"
    );
}
