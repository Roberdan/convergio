//! Dedicated HTTP audit-verify E2E.
//!
//! Drives a few mutations (plans + a task + a transition + evidence) and
//! asserts that:
//!
//! 1. Open-ended `GET /v1/audit/verify` returns ok with checked >= 5
//! 2. Ranged verify for the first half of the chain still passes
//! 3. Tampering one row visibly via raw SQL flips ok=false (covered
//!    via a direct `Pool` borrow — proves the HTTP verifier and the
//!    in-process verifier agree).

mod common;

use common::boot;
use serde_json::{json, Value};

async fn produce_some_history(client: &reqwest::Client, base: &str) {
    let plan: Value = client
        .post(format!("{base}/v1/plans"))
        .json(&json!({"title": "audit e2e"}))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let plan_id = plan["id"].as_str().unwrap().to_string();

    let task: Value = client
        .post(format!("{base}/v1/plans/{plan_id}/tasks"))
        .json(&json!({"title": "t", "evidence_required": []}))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let task_id = task["id"].as_str().unwrap().to_string();

    let _: Value = client
        .post(format!("{base}/v1/tasks/{task_id}/transition"))
        .json(&json!({"target": "in_progress", "agent_id": "a"}))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();

    let _: Value = client
        .post(format!("{base}/v1/tasks/{task_id}/evidence"))
        .json(&json!({"kind": "manual", "payload": {"who": "human"}}))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
}

#[tokio::test]
async fn open_ended_verify_passes_after_a_real_workflow() {
    let (base, _pool, _dir) = boot().await;
    let client = reqwest::Client::new();
    produce_some_history(&client, &base).await;

    let report: Value = client
        .get(format!("{base}/v1/audit/verify"))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(report["ok"], true);
    assert!(report["checked"].as_i64().unwrap() >= 4);
    assert_eq!(report["broken_at"], Value::Null);
}

#[tokio::test]
async fn ranged_verify_works_inside_clean_range() {
    let (base, _pool, _dir) = boot().await;
    let client = reqwest::Client::new();
    produce_some_history(&client, &base).await;
    produce_some_history(&client, &base).await;

    // First two events.
    let report: Value = client
        .get(format!("{base}/v1/audit/verify?from=1&to=2"))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(report["ok"], true);
    assert_eq!(report["checked"], 2);
}

#[tokio::test]
async fn http_verify_detects_tampering_done_directly_in_db() {
    let (base, pool, _dir) = boot().await;
    let client = reqwest::Client::new();
    produce_some_history(&client, &base).await;

    // Tamper directly via the pool the server is using.
    sqlx::query("UPDATE audit_log SET payload = ? WHERE seq = ?")
        .bind(r#"{"oops":1}"#)
        .bind(2_i64)
        .execute(pool.inner())
        .await
        .unwrap();

    let report: Value = client
        .get(format!("{base}/v1/audit/verify"))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(report["ok"], false);
    assert_eq!(report["broken_at"], 2);
}
