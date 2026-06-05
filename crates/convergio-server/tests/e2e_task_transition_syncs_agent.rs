//! HTTP-level verification of P2-1: task transition auto-populates
//! `agents.current_task_id`.
//!
//! Acceptance criteria (from task a1f47c57):
//! - `POST /v1/tasks/:id/transition` with `target=in_progress` and
//!   `agent_id` → subsequent `GET /v1/agent-registry/agents/:id`
//!   shows `current_task_id` pointing at the task and `status=working`.
//! - Submitting the task clears `current_task_id` and flips the agent
//!   back to `idle`.
//! - Unregistered agents do not cause errors.
//! - A second claim on the same agent overwrites the first pointer.

mod common;

use serde_json::{json, Value};

async fn register_agent(client: &reqwest::Client, base: &str, id: &str) {
    client
        .post(format!("{base}/v1/agent-registry/agents"))
        .json(&json!({
            "id": id,
            "kind": "claude-code",
            "capabilities": [],
            "metadata": {}
        }))
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap();
}

async fn make_task(client: &reqwest::Client, base: &str) -> String {
    let plan: Value = client
        .post(format!("{base}/v1/plans"))
        .json(&json!({"title": "test plan"}))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let plan_id = plan["id"].as_str().unwrap();
    let task: Value = client
        .post(format!("{base}/v1/plans/{plan_id}/tasks"))
        .json(&json!({"title": "task", "wave": 1, "sequence": 1}))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    task["id"].as_str().unwrap().to_string()
}

#[tokio::test]
async fn claim_sets_current_task_id_and_working_status() {
    let (base, _pool, _dir) = common::boot().await;
    let client = common::client();
    register_agent(&client, &base, "agent-x").await;
    let task_id = make_task(&client, &base).await;

    client
        .post(format!("{base}/v1/tasks/{task_id}/transition"))
        .json(&json!({"target": "in_progress", "agent_id": "agent-x"}))
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap();

    let agent: Value = client
        .get(format!("{base}/v1/agent-registry/agents/agent-x"))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();

    assert_eq!(agent["status"], "working");
    assert_eq!(agent["current_task_id"], task_id.as_str());
}

#[tokio::test]
async fn submit_clears_current_task_id_and_marks_idle() {
    let (base, _pool, _dir) = common::boot().await;
    let client = common::client();
    register_agent(&client, &base, "agent-y").await;
    let task_id = make_task(&client, &base).await;

    client
        .post(format!("{base}/v1/tasks/{task_id}/transition"))
        .json(&json!({"target": "in_progress", "agent_id": "agent-y"}))
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap();

    client
        .post(format!("{base}/v1/tasks/{task_id}/transition"))
        .json(&json!({"target": "submitted", "agent_id": "agent-y"}))
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap();

    let agent: Value = client
        .get(format!("{base}/v1/agent-registry/agents/agent-y"))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();

    assert_eq!(agent["status"], "idle");
    assert_eq!(agent["current_task_id"], Value::Null);
}

#[tokio::test]
async fn unregistered_agent_transition_does_not_error() {
    let (base, _pool, _dir) = common::boot().await;
    let client = common::client();
    let task_id = make_task(&client, &base).await;

    let resp = client
        .post(format!("{base}/v1/tasks/{task_id}/transition"))
        .json(&json!({"target": "in_progress", "agent_id": "ghost-agent"}))
        .send()
        .await
        .unwrap();

    assert!(
        resp.status().is_success(),
        "unregistered agent must not cause 5xx"
    );
}

#[tokio::test]
async fn second_claim_overwrites_current_task_pointer() {
    let (base, _pool, _dir) = common::boot().await;
    let client = common::client();
    register_agent(&client, &base, "agent-z").await;
    let task_a = make_task(&client, &base).await;
    let task_b = make_task(&client, &base).await;

    client
        .post(format!("{base}/v1/tasks/{task_a}/transition"))
        .json(&json!({"target": "in_progress", "agent_id": "agent-z"}))
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap();
    client
        .post(format!("{base}/v1/tasks/{task_b}/transition"))
        .json(&json!({"target": "in_progress", "agent_id": "agent-z"}))
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap();

    let agent: Value = client
        .get(format!("{base}/v1/agent-registry/agents/agent-z"))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();

    assert_eq!(agent["current_task_id"], task_b.as_str());
    assert_eq!(agent["status"], "working");
}
