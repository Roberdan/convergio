//! Best-effort token telemetry capture for `cvg agent spawn`.
//!
//! When spawning Claude Code with `--output-format stream-json`, each
//! stdout line is a JSON object. We opportunistically scan those events
//! for `{usage:{input_tokens,output_tokens}}` and optional `cost_usd`,
//! then attach one `evidence.kind="usage"` row to the task.

use super::Client;
use anyhow::Result;
use serde_json::{json, Map, Value};
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone)]
pub(crate) struct UsageEvidence {
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub model: String,
    pub cost_usd: Option<f64>,
}

pub(crate) type UsageSlot = Arc<Mutex<Option<UsageEvidence>>>;

pub(crate) fn observe_claude_stdout_line(slot: &UsageSlot, line: &str, model_fallback: &str) {
    let trimmed = line.trim();
    if !trimmed.starts_with('{') {
        return;
    }
    let Ok(v) = serde_json::from_str::<Value>(trimmed) else {
        return;
    };
    let Some(usage_obj) = find_usage_object(&v) else {
        return;
    };
    let Some(input_tokens) = usage_obj.get("input_tokens").and_then(as_u64) else {
        return;
    };
    let Some(output_tokens) = usage_obj.get("output_tokens").and_then(as_u64) else {
        return;
    };
    let model = find_string_by_key(&v, "model")
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| model_fallback.to_string());
    let cost_usd = find_cost_usd(&v);

    let mut guard = slot.lock().unwrap_or_else(|e| e.into_inner());
    *guard = Some(UsageEvidence {
        input_tokens,
        output_tokens,
        model,
        cost_usd,
    });
}

pub(crate) async fn post_usage_evidence_best_effort(
    client: &Client,
    task_id: &str,
    usage: UsageEvidence,
) {
    let body = json!({
        "kind": "usage",
        "payload": {
            "input_tokens": usage.input_tokens,
            "output_tokens": usage.output_tokens,
            "model": usage.model,
            "cost_usd": usage.cost_usd,
        }
    });
    let path = format!("/v1/tasks/{task_id}/evidence");
    let res: Result<Value> = client.post(&path, &body).await;
    if let Err(e) = res {
        eprintln!("warning: usage evidence attach failed: {e}");
    }
}

fn as_u64(v: &Value) -> Option<u64> {
    match v {
        Value::Number(n) => n
            .as_u64()
            .or_else(|| n.as_i64().and_then(|i| u64::try_from(i).ok())),
        Value::String(s) => s.trim().parse().ok(),
        _ => None,
    }
}

fn find_cost_usd(v: &Value) -> Option<f64> {
    // Common shapes observed across vendor CLIs.
    find_f64_by_key(v, "cost_usd").or_else(|| {
        v.pointer("/cost_usd_micros")
            .and_then(|m| {
                m.as_i64()
                    .or_else(|| m.as_u64().and_then(|u| i64::try_from(u).ok()))
            })
            .map(|micros| micros as f64 / 1_000_000.0)
    })
}

fn find_usage_object(v: &Value) -> Option<&Map<String, Value>> {
    let Value::Object(map) = v else {
        return None;
    };
    if map.contains_key("input_tokens") && map.contains_key("output_tokens") {
        return Some(map);
    }
    if let Some(Value::Object(u)) = map.get("usage") {
        if u.contains_key("input_tokens") && u.contains_key("output_tokens") {
            return Some(u);
        }
    }

    for child in map.values() {
        if let Some(found) = find_usage_object_any(child) {
            return Some(found);
        }
    }
    None
}

fn find_usage_object_any(v: &Value) -> Option<&Map<String, Value>> {
    match v {
        Value::Object(_) => find_usage_object(v),
        Value::Array(items) => items.iter().find_map(find_usage_object_any),
        _ => None,
    }
}

fn find_string_by_key(v: &Value, key: &str) -> Option<String> {
    match v {
        Value::Object(map) => {
            if let Some(Value::String(s)) = map.get(key) {
                return Some(s.to_string());
            }
            for child in map.values() {
                if let Some(found) = find_string_by_key(child, key) {
                    return Some(found);
                }
            }
            None
        }
        Value::Array(items) => items.iter().find_map(|i| find_string_by_key(i, key)),
        _ => None,
    }
}

fn find_f64_by_key(v: &Value, key: &str) -> Option<f64> {
    match v {
        Value::Object(map) => {
            if let Some(n) = map.get(key).and_then(Value::as_f64) {
                return Some(n);
            }
            if let Some(n) = map.get(key).and_then(Value::as_i64) {
                return Some(n as f64);
            }
            if let Some(n) = map.get(key).and_then(Value::as_u64) {
                return Some(n as f64);
            }
            for child in map.values() {
                if let Some(found) = find_f64_by_key(child, key) {
                    return Some(found);
                }
            }
            None
        }
        Value::Array(items) => items.iter().find_map(|i| find_f64_by_key(i, key)),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn observes_usage_in_usage_field() {
        let slot: UsageSlot = Arc::new(Mutex::new(None));
        observe_claude_stdout_line(
            &slot,
            r#"{"type":"message_stop","message":{"usage":{"input_tokens":7,"output_tokens":3},"model":"opus"}}"#,
            "opus",
        );
        let got = slot.lock().unwrap().clone().unwrap();
        assert_eq!(got.input_tokens, 7);
        assert_eq!(got.output_tokens, 3);
        assert_eq!(got.model, "opus");
    }
}
