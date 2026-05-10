//! Best-effort token telemetry capture for `cvg agent spawn`.
//!
//! Claude `--output-format stream-json` emits one JSON event per line.
//! We opportunistically parse those lines to extract a final-ish token
//! usage snapshot, then attach it as `evidence.kind = "usage"`.

use super::Client;
use anyhow::Result;
use convergio_runner::RunnerKind;
use serde_json::{json, Value};

#[derive(Debug, Default, Clone)]
pub(crate) struct UsageObservation {
    pub(crate) input_tokens: Option<u64>,
    pub(crate) output_tokens: Option<u64>,
    pub(crate) cost_usd: Option<f64>,
    pub(crate) model: Option<String>,
}

impl UsageObservation {
    fn bump_max_u64(slot: &mut Option<u64>, candidate: Option<u64>) {
        if let Some(v) = candidate {
            match slot {
                Some(prev) => {
                    if v > *prev {
                        *slot = Some(v);
                    }
                }
                None => *slot = Some(v),
            }
        }
    }

    fn bump_max_f64(slot: &mut Option<f64>, candidate: Option<f64>) {
        if let Some(v) = candidate.filter(|c| c.is_finite() && *c >= 0.0) {
            match slot {
                Some(prev) => {
                    if v > *prev {
                        *slot = Some(v);
                    }
                }
                None => *slot = Some(v),
            }
        }
    }

    fn bump_model(slot: &mut Option<String>, candidate: Option<&str>) {
        if slot.is_none() {
            if let Some(m) = candidate.map(str::trim).filter(|s| !s.is_empty()) {
                *slot = Some(m.to_string());
            }
        }
    }
}

pub(crate) fn observe_claude_stream_json_line(line: &str, obs: &mut UsageObservation) {
    let trimmed = line.trim();
    if !trimmed.starts_with('{') {
        return;
    }
    let Ok(v) = serde_json::from_str::<Value>(trimmed) else {
        return;
    };

    // Try to find a nested { usage: { input_tokens, output_tokens, ... } } object.
    if let Some(u) = find_usage_object(&v) {
        UsageObservation::bump_max_u64(
            &mut obs.input_tokens,
            u.get("input_tokens").and_then(Value::as_u64),
        );
        UsageObservation::bump_max_u64(
            &mut obs.output_tokens,
            u.get("output_tokens").and_then(Value::as_u64),
        );
        UsageObservation::bump_max_f64(
            &mut obs.cost_usd,
            u.get("cost_usd").and_then(Value::as_f64),
        );
    }

    UsageObservation::bump_max_f64(&mut obs.cost_usd, find_cost_usd(&v));
    UsageObservation::bump_model(&mut obs.model, find_model(&v));
}

fn find_usage_object(v: &Value) -> Option<&serde_json::Map<String, Value>> {
    match v {
        Value::Object(map) => {
            if let Some(Value::Object(u)) = map.get("usage") {
                if u.contains_key("input_tokens") || u.contains_key("output_tokens") {
                    return Some(u);
                }
            }
            for child in map.values() {
                if let Some(found) = find_usage_object(child) {
                    return Some(found);
                }
            }
            None
        }
        Value::Array(arr) => arr.iter().find_map(find_usage_object),
        _ => None,
    }
}

fn find_cost_usd(v: &Value) -> Option<f64> {
    match v {
        Value::Object(map) => {
            for key in ["cost_usd", "cost", "total_cost", "usd_cost"] {
                if let Some(n) = map.get(key).and_then(Value::as_f64) {
                    return Some(n);
                }
            }
            map.values().find_map(find_cost_usd)
        }
        Value::Array(arr) => arr.iter().find_map(find_cost_usd),
        _ => None,
    }
}

fn find_model(v: &Value) -> Option<&str> {
    match v {
        Value::Object(map) => {
            if let Some(m) = map.get("model").and_then(Value::as_str) {
                return Some(m);
            }
            map.values().find_map(find_model)
        }
        Value::Array(arr) => arr.iter().find_map(find_model),
        _ => None,
    }
}

pub(crate) fn usage_payload(kind: &RunnerKind, obs: &UsageObservation) -> Option<Value> {
    let input_tokens = obs.input_tokens?;
    let output_tokens = obs.output_tokens?;
    let model = obs.model.clone().unwrap_or_else(|| kind.to_string());
    Some(json!({
        "input_tokens": input_tokens,
        "output_tokens": output_tokens,
        "model": model,
        "cost_usd": obs.cost_usd,
    }))
}

pub(crate) async fn attach_usage_evidence(
    client: &Client,
    task_id: &str,
    payload: Value,
) -> Result<()> {
    let body = json!({
        "kind": "usage",
        "payload": payload,
    });
    let _res: Value = client
        .post(&format!("/v1/tasks/{task_id}/evidence"), &body)
        .await?;
    Ok(())
}
