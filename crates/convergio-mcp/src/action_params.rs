//! Shared param helpers for `convergio.act` action handlers.
//!
//! Extracted from `actions.rs` to keep that file under the 300-line
//! Rust cap (audit finding L5) while letting both `actions.rs` and
//! `bus_actions.rs` keep their thin path/body wiring next to the
//! dispatch they implement.

use crate::http::invalid;
use convergio_api::AgentResponse;
use serde_json::Value;

/// ADR-0043: `id` is canonical for entity-self; `agent_id` is a deprecated alias.
pub(crate) fn resolve_agent_id(params: &mut Value) -> Result<String, AgentResponse> {
    for key in ["id", "agent_id"] {
        if let Some(v) = params
            .get(key)
            .and_then(Value::as_str)
            .map(ToOwned::to_owned)
        {
            if key == "agent_id" {
                tracing::warn!("deprecated 'agent_id'; use 'id' (ADR-0043, removed 0.4.0)");
            }
            remove_key(params, key);
            return Ok(v);
        }
    }
    Err(invalid("missing string param: id".to_owned()))
}

pub(crate) fn required_str(params: &Value, key: &str) -> Result<String, AgentResponse> {
    params
        .get(key)
        .and_then(Value::as_str)
        .map(ToOwned::to_owned)
        .ok_or_else(|| invalid(format!("missing string param: {key}")))
}

pub(crate) fn audit_path(params: &Value) -> Result<String, AgentResponse> {
    let mut query = Vec::new();
    for key in ["from", "to"] {
        if let Some(value) = params.get(key) {
            let number = value
                .as_i64()
                .ok_or_else(|| invalid(format!("{key} must be an integer")))?;
            query.push(format!("{key}={number}"));
        }
    }
    if query.is_empty() {
        Ok("/v1/audit/verify".into())
    } else {
        Ok(format!("/v1/audit/verify?{}", query.join("&")))
    }
}

pub(crate) fn remove_key(value: &mut Value, key: &str) {
    if let Value::Object(map) = value {
        map.remove(key);
    }
}

/// Caller-supplied `task_id` for `Action::ExplainLastRefusal`.
pub(crate) fn caller_task_id(params: &Value) -> Option<String> {
    params
        .get("task_id")
        .and_then(Value::as_str)
        .map(ToOwned::to_owned)
}

/// Recover `task_id` from a previously-stored gate refusal.
///
/// `Bridge::daemon_response` stores refusals as
/// `{"path": ..., "status": ..., "error": {"code": ..., "data": {"task_id": ...}}}`,
/// so we look inside `error.data.task_id` first and fall back to a
/// top-level `task_id` for forward-compat with future stored shapes.
pub(crate) fn memory_task_id(local: Option<&Value>) -> Option<String> {
    let local = local?;
    local
        .pointer("/error/data/task_id")
        .and_then(Value::as_str)
        .or_else(|| local.get("task_id").and_then(Value::as_str))
        .map(ToOwned::to_owned)
}
