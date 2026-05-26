//! E2E for `POST /v1/audit/append` (P2-2, ADR-0002 § Custom kinds).
//!
//! Asserts the agent-emitted custom audit row contract:
//!
//! 1. Successful append returns 201 with `seq` + `hash`, the row shows
//!    up in `GET /v1/audit/events`, and `GET /v1/audit/verify` keeps
//!    returning ok with the new row counted.
//! 2. Reserved daemon-owned kinds (`task.*`, `plan.*`, ...) are
//!    rejected with 422 `kind_reserved`.
//! 3. Vendor-prefixed kinds (e.g. `myapp.foo`) are accepted with 201.
//! 4. Malformed kinds (missing dot, leading digit, etc.) are rejected
//!    with 400 `kind_invalid`.

mod common;

use common::boot;
use serde_json::{json, Value};

#[tokio::test]
async fn agent_custom_row_appends_and_chain_still_verifies() {
    let (base, _pool, _dir) = boot().await;
    let client = common::client();

    // Produce a small history first so the chain isn't empty.
    let plan: Value = client
        .post(format!("{base}/v1/plans"))
        .json(&json!({"title": "audit append e2e"}))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let plan_id = plan["id"].as_str().unwrap().to_string();

    let baseline: Value = client
        .get(format!("{base}/v1/audit/verify"))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(baseline["ok"], true);
    let baseline_checked = baseline["checked"].as_i64().unwrap();

    // Custom row, well-formed.
    let response = client
        .post(format!("{base}/v1/audit/append"))
        .json(&json!({
            "kind": "session.pre_stop.check.1",
            "entity_kind": "free",
            "entity_id": plan_id.clone(),
            "agent_id": "subagent-p2-2",
            "payload": {"check": "context_ok", "result": "pass"},
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(response.status().as_u16(), 201, "expected 201 Created");
    let body: Value = response.json().await.unwrap();
    let new_seq = body["seq"].as_i64().expect("seq");
    let new_hash = body["hash"].as_str().expect("hash").to_string();
    assert!(new_seq >= 1);
    assert!(!new_hash.is_empty());

    // The row shows up in /v1/audit/events.
    let after = (new_seq - 1).max(0);
    let events: Value = client
        .get(format!("{base}/v1/audit/events?after_seq={after}"))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let entry = events
        .as_array()
        .unwrap()
        .iter()
        .find(|e| e["seq"].as_i64() == Some(new_seq))
        .expect("custom row must appear in /v1/audit/events");
    assert_eq!(entry["transition"], "session.pre_stop.check.1");
    assert_eq!(entry["entity_type"], "free");
    assert_eq!(entry["entity_id"], plan_id);
    assert_eq!(entry["agent_id"], "subagent-p2-2");
    assert_eq!(entry["hash"], new_hash);

    // Chain still verifies, with the new row counted.
    let after_verify: Value = client
        .get(format!("{base}/v1/audit/verify"))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(after_verify["ok"], true);
    assert_eq!(after_verify["broken_at"], Value::Null);
    assert!(after_verify["checked"].as_i64().unwrap() > baseline_checked);
}

#[tokio::test]
async fn reserved_daemon_kinds_are_refused_with_422() {
    let (base, _pool, _dir) = boot().await;
    let client = common::client();

    for reserved in &[
        "task.foo",
        "plan.created",
        "evidence.attached",
        "agent.session_started",
    ] {
        let response = client
            .post(format!("{base}/v1/audit/append"))
            .json(&json!({
                "kind": reserved,
                "entity_kind": "free",
                "entity_id": "x",
                "payload": {},
            }))
            .send()
            .await
            .unwrap();
        assert_eq!(
            response.status().as_u16(),
            422,
            "kind '{reserved}' must be 422 kind_reserved"
        );
        let body: Value = response.json().await.unwrap();
        assert_eq!(body["error"]["code"], "kind_reserved");
    }
}

#[tokio::test]
async fn vendor_prefixed_kind_is_accepted() {
    let (base, _pool, _dir) = boot().await;
    let client = common::client();

    let response = client
        .post(format!("{base}/v1/audit/append"))
        .json(&json!({
            "kind": "myapp.foo",
            "entity_kind": "free",
            "entity_id": "correlation-key-123",
            "payload": {"hello": "world"},
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(response.status().as_u16(), 201);
}

#[tokio::test]
async fn malformed_kinds_are_refused_with_400() {
    let (base, _pool, _dir) = boot().await;
    let client = common::client();

    for bad in &[
        "noNamespace",
        "0bad.start",
        "myapp..double",
        "myapp.WithCaps",
        "myapp.with-dash",
    ] {
        let response = client
            .post(format!("{base}/v1/audit/append"))
            .json(&json!({
                "kind": bad,
                "entity_kind": "free",
                "entity_id": "x",
                "payload": {},
            }))
            .send()
            .await
            .unwrap();
        assert_eq!(
            response.status().as_u16(),
            400,
            "kind '{bad}' must be 400 kind_invalid"
        );
        let body: Value = response.json().await.unwrap();
        assert_eq!(body["error"]["code"], "kind_invalid");
    }
}

#[tokio::test]
async fn payload_must_be_object() {
    let (base, _pool, _dir) = boot().await;
    let client = common::client();

    // Array payload.
    let response = client
        .post(format!("{base}/v1/audit/append"))
        .json(&json!({
            "kind": "myapp.foo",
            "entity_kind": "free",
            "entity_id": "x",
            "payload": [1, 2, 3],
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(response.status().as_u16(), 422);
    let body: Value = response.json().await.unwrap();
    assert_eq!(body["error"]["code"], "payload_not_object");
}
