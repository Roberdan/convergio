//! Unit tests for `actions` — split to stay under the 300-line cap.
use crate::actions::{audit_path, remove_key, resolve_agent_id};
use crate::bridge::Bridge;
use convergio_api::{ActRequest, Action, AgentCode, NextHint};
use serde_json::json;

#[tokio::test]
async fn act_rejects_schema_mismatch_before_network() {
    let bridge = Bridge::new("http://127.0.0.1:1".into());
    let response = bridge
        .dispatch(ActRequest {
            schema_version: "999".into(),
            action: Action::Status,
            params: json!({}),
        })
        .await;
    assert!(!response.ok);
    assert_eq!(response.code, AgentCode::SchemaVersionMismatch);
    assert_eq!(response.next, Some(NextHint::RefreshHelp));
}

#[test]
fn audit_path_validates_numbers() {
    let path = audit_path(&json!({"from": 1, "to": 9})).unwrap();
    assert_eq!(path, "/v1/audit/verify?from=1&to=9");
    let err = audit_path(&json!({"from": "bad"})).unwrap_err();
    assert_eq!(err.code, AgentCode::InvalidRequest);
}

#[test]
fn resolve_agent_id_prefers_id_field() {
    let mut params = json!({"id": "my-agent", "current_task_id": "t1"});
    let id = resolve_agent_id(&mut params).unwrap();
    assert_eq!(id, "my-agent");
    assert!(
        params.get("id").is_none(),
        "id must be removed from remaining params"
    );
    assert_eq!(params["current_task_id"], "t1");
}

#[test]
fn resolve_agent_id_accepts_deprecated_agent_id() {
    let mut params = json!({"agent_id": "legacy-agent", "status": "idle"});
    let id = resolve_agent_id(&mut params).unwrap();
    assert_eq!(id, "legacy-agent");
    assert!(params.get("agent_id").is_none());
    assert_eq!(params["status"], "idle");
}

#[test]
fn resolve_agent_id_errors_when_both_absent() {
    let mut params = json!({"status": "idle"});
    let err = resolve_agent_id(&mut params).unwrap_err();
    assert_eq!(err.code, AgentCode::InvalidRequest);
}

#[test]
fn remove_key_on_non_object_is_noop() {
    let mut v = json!([1, 2, 3]);
    remove_key(&mut v, "a");
    assert_eq!(v, json!([1, 2, 3]));
}
