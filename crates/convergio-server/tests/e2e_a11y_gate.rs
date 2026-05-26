//! HTTP-level verification that `A11yGate` refuses invalid evidence on
//! a `submitted` transition.

mod common;

use serde_json::{json, Value};

async fn make_task(client: &reqwest::Client, base: &str) -> String {
    let plan: Value = client
        .post(format!("{base}/v1/plans"))
        .json(&json!({"title": "a11y plan"}))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let plan_id = plan["id"].as_str().unwrap();

    let task: Value = client
        .post(format!("{base}/v1/plans/{plan_id}/tasks"))
        .json(&json!({"title": "a11y task", "wave": 1, "sequence": 1}))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();

    task["id"].as_str().unwrap().to_string()
}

#[tokio::test]
async fn refuses_submitted_when_markdown_heading_skips() {
    let (base, _pool, _dir) = common::boot().await;
    let client = reqwest::Client::new();

    let task_id = make_task(&client, &base).await;

    client
        .post(format!("{base}/v1/tasks/{task_id}/evidence"))
        .json(&json!({
            "kind": "markdown_doc",
            "payload": {"body": "# Title\n\n### Skips H2"},
            "exit_code": 0,
        }))
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap();

    client
        .post(format!("{base}/v1/tasks/{task_id}/transition"))
        .json(&json!({"target": "in_progress", "agent_id": "agent-a11y"}))
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap();

    let resp = client
        .post(format!("{base}/v1/tasks/{task_id}/transition"))
        .json(&json!({"target": "submitted", "agent_id": "agent-a11y"}))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 409, "a11y gate should refuse");
    let body: Value = resp.json().await.unwrap();
    assert_eq!(body["error"]["code"], "gate_refused");

    let msg = body["error"]["message"].as_str().unwrap_or_default();
    assert!(msg.contains("a11y_violation_found"), "message: {msg}");
    assert!(
        msg.contains("markdown_doc#md_heading_skip"),
        "message: {msg}"
    );
}
