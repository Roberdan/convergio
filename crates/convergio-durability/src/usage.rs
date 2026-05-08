//! Protocol-level parsing + aggregation helpers for `evidence.kind = "usage"`.
//!
//! The evidence row itself is immutable, but dashboards need quick access
//! to token/cost totals. The durability layer therefore maintains a
//! best-effort cache under `agents.metadata.usage`.

use crate::error::{DurabilityError, Result};
use serde_json::{Map, Value};

/// Parsed payload for `evidence.kind = "usage"`.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct UsagePayload {
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub model: String,
    pub cost_usd: Option<f64>,
}

/// Parse and validate the `usage` evidence payload.
///
/// Contract:
/// - `input_tokens`: integer >= 0
/// - `output_tokens`: integer >= 0
/// - `model`: non-empty string
/// - `cost_usd`: number or null (optional)
pub(crate) fn parse_usage_payload(payload: &Value) -> Result<UsagePayload> {
    let Value::Object(map) = payload else {
        return Err(DurabilityError::InvalidEvidence {
            reason: "usage payload must be a JSON object".into(),
        });
    };

    let input_tokens = map.get("input_tokens").and_then(parse_u64).ok_or_else(|| {
        DurabilityError::InvalidEvidence {
            reason: "usage payload missing/invalid input_tokens".into(),
        }
    })?;
    let output_tokens = map
        .get("output_tokens")
        .and_then(parse_u64)
        .ok_or_else(|| DurabilityError::InvalidEvidence {
            reason: "usage payload missing/invalid output_tokens".into(),
        })?;

    let model = map
        .get("model")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .ok_or_else(|| DurabilityError::InvalidEvidence {
            reason: "usage payload missing/invalid model".into(),
        })?
        .to_string();

    let cost_usd = match map.get("cost_usd") {
        None | Some(Value::Null) => None,
        Some(v) => Some(
            parse_f64(v).ok_or_else(|| DurabilityError::InvalidEvidence {
                reason: "usage payload cost_usd must be number or null".into(),
            })?,
        ),
    };

    Ok(UsagePayload {
        input_tokens,
        output_tokens,
        model,
        cost_usd,
    })
}

/// Merge one `usage` payload into `metadata` in-place.
///
/// Result schema (all numbers are integers unless noted):
///
/// ```json
/// {
///   "usage": {
///     "calls": 2,
///     "total_input_tokens": 12,
///     "total_output_tokens": 8,
///     "total_tokens": 20,
///     "total_cost_usd": 0.3,
///     "last_model": "opus",
///     "by_model": {
///       "opus": {
///         "calls": 2,
///         "input_tokens": 12,
///         "output_tokens": 8,
///         "total_tokens": 20,
///         "cost_usd": 0.3
///       }
///     }
///   }
/// }
/// ```
pub(crate) fn merge_usage_into_agent_metadata(metadata: &mut Value, usage: &UsagePayload) {
    ensure_object_root(metadata);
    let root = metadata.as_object_mut().unwrap_or_else(|| unreachable!());

    let usage_entry = root
        .entry("usage")
        .or_insert_with(|| Value::Object(Map::new()));
    if !usage_entry.is_object() {
        *usage_entry = Value::Object(Map::new());
    }
    let usage_obj = usage_entry
        .as_object_mut()
        .unwrap_or_else(|| unreachable!());

    let calls = i64_field(usage_obj, "calls").unwrap_or(0).saturating_add(1);
    usage_obj.insert("calls".into(), Value::from(calls));

    let in_tok = i64_field(usage_obj, "total_input_tokens").unwrap_or(0);
    let out_tok = i64_field(usage_obj, "total_output_tokens").unwrap_or(0);
    let in_next = in_tok.saturating_add(u64_to_i64(usage.input_tokens));
    let out_next = out_tok.saturating_add(u64_to_i64(usage.output_tokens));

    usage_obj.insert("total_input_tokens".into(), Value::from(in_next));
    usage_obj.insert("total_output_tokens".into(), Value::from(out_next));
    usage_obj.insert(
        "total_tokens".into(),
        Value::from(in_next.saturating_add(out_next)),
    );
    usage_obj.insert("last_model".into(), Value::from(usage.model.clone()));

    if let Some(cost) = usage.cost_usd {
        let current = f64_field(usage_obj, "total_cost_usd").unwrap_or(0.0);
        usage_obj.insert("total_cost_usd".into(), Value::from(current + cost));
    }

    let by_model = usage_obj
        .entry("by_model")
        .or_insert_with(|| Value::Object(Map::new()));
    if !by_model.is_object() {
        *by_model = Value::Object(Map::new());
    }
    let by_model_obj = by_model.as_object_mut().unwrap_or_else(|| unreachable!());

    let model_entry = by_model_obj
        .entry(usage.model.clone())
        .or_insert_with(|| Value::Object(Map::new()));
    if !model_entry.is_object() {
        *model_entry = Value::Object(Map::new());
    }
    let model_obj = model_entry
        .as_object_mut()
        .unwrap_or_else(|| unreachable!());

    let m_calls = i64_field(model_obj, "calls").unwrap_or(0).saturating_add(1);
    model_obj.insert("calls".into(), Value::from(m_calls));

    let m_in = i64_field(model_obj, "input_tokens").unwrap_or(0) + u64_to_i64(usage.input_tokens);
    let m_out =
        i64_field(model_obj, "output_tokens").unwrap_or(0) + u64_to_i64(usage.output_tokens);
    model_obj.insert("input_tokens".into(), Value::from(m_in));
    model_obj.insert("output_tokens".into(), Value::from(m_out));
    model_obj.insert(
        "total_tokens".into(),
        Value::from(m_in.saturating_add(m_out)),
    );

    if let Some(cost) = usage.cost_usd {
        let current = f64_field(model_obj, "cost_usd").unwrap_or(0.0);
        model_obj.insert("cost_usd".into(), Value::from(current + cost));
    }
}

fn ensure_object_root(v: &mut Value) {
    if !v.is_object() {
        *v = Value::Object(Map::new());
    }
}

fn u64_to_i64(v: u64) -> i64 {
    i64::try_from(v).unwrap_or(i64::MAX)
}

fn parse_u64(v: &Value) -> Option<u64> {
    match v {
        Value::Number(n) => n
            .as_u64()
            .or_else(|| n.as_i64().and_then(|i| u64::try_from(i).ok())),
        Value::String(s) => s.trim().parse().ok(),
        _ => None,
    }
}

fn parse_f64(v: &Value) -> Option<f64> {
    match v {
        Value::Number(n) => n
            .as_f64()
            .or_else(|| n.as_i64().map(|i| i as f64))
            .or_else(|| n.as_u64().map(|u| u as f64)),
        Value::String(s) => s.trim().parse().ok(),
        _ => None,
    }
}

fn i64_field(map: &Map<String, Value>, key: &str) -> Option<i64> {
    map.get(key).and_then(|v| match v {
        Value::Number(n) => n
            .as_i64()
            .or_else(|| n.as_u64().and_then(|u| i64::try_from(u).ok())),
        Value::String(s) => s.trim().parse().ok(),
        _ => None,
    })
}

fn f64_field(map: &Map<String, Value>, key: &str) -> Option<f64> {
    map.get(key).and_then(parse_f64)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn parse_usage_accepts_null_cost() {
        let payload = json!({"input_tokens":1,"output_tokens":2,"model":"opus","cost_usd":null});
        let got = parse_usage_payload(&payload).unwrap();
        assert_eq!(got.cost_usd, None);
    }

    #[test]
    fn merge_usage_accumulates_totals_and_by_model() {
        let mut meta = json!({});
        let u = UsagePayload {
            input_tokens: 3,
            output_tokens: 4,
            model: "opus".into(),
            cost_usd: Some(0.1),
        };
        merge_usage_into_agent_metadata(&mut meta, &u);
        merge_usage_into_agent_metadata(&mut meta, &u);

        let usage = meta.get("usage").unwrap().as_object().unwrap();
        assert_eq!(usage.get("calls").and_then(|v| v.as_i64()), Some(2));
        assert_eq!(
            usage.get("total_input_tokens").and_then(|v| v.as_i64()),
            Some(6)
        );
        assert_eq!(
            usage.get("total_output_tokens").and_then(|v| v.as_i64()),
            Some(8)
        );
        assert_eq!(
            usage.get("last_model").and_then(|v| v.as_str()),
            Some("opus")
        );

        let by_model = usage.get("by_model").unwrap().as_object().unwrap();
        let opus = by_model.get("opus").unwrap().as_object().unwrap();
        assert_eq!(opus.get("calls").and_then(|v| v.as_i64()), Some(2));
        assert_eq!(opus.get("input_tokens").and_then(|v| v.as_i64()), Some(6));
        assert_eq!(opus.get("output_tokens").and_then(|v| v.as_i64()), Some(8));
    }
}
