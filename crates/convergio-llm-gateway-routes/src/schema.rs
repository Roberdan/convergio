//! Dependency-light structural validator for the optional output schema.
//!
//! Supports the JSON Schema keywords the gateway needs to fence provider
//! responses (`type`, `required`, `properties`, `items`, `enum`,
//! numeric/length bounds, and boolean `additionalProperties`). Building a
//! focused validator here keeps the crate clear of a heavy json-schema
//! dependency that could trip `cargo deny`.

use serde_json::Value;

/// Validate `instance` against the (subset) JSON `schema`.
///
/// Returns a human-readable, path-prefixed message on the first mismatch.
pub(super) fn validate(schema: &Value, instance: &Value) -> Result<(), String> {
    check(schema, instance, "$")
}

fn check(schema: &Value, instance: &Value, path: &str) -> Result<(), String> {
    let Some(obj) = schema.as_object() else {
        // A non-object schema (e.g. `true`) accepts anything.
        return Ok(());
    };

    if let Some(ty) = obj.get("type").and_then(Value::as_str) {
        check_type(ty, instance, path)?;
    }

    if let Some(allowed) = obj.get("enum").and_then(Value::as_array) {
        if !allowed.iter().any(|candidate| candidate == instance) {
            return Err(format!("{path}: value not in enum"));
        }
    }

    check_numeric_bounds(obj, instance, path)?;
    check_string_bounds(obj, instance, path)?;

    if let Some(props) = obj.get("properties").and_then(Value::as_object) {
        if let Some(map) = instance.as_object() {
            for (key, subschema) in props {
                if let Some(child) = map.get(key) {
                    check(subschema, child, &format!("{path}.{key}"))?;
                }
            }
        }
    }

    if let Some(required) = obj.get("required").and_then(Value::as_array) {
        let map = instance.as_object();
        for field in required.iter().filter_map(Value::as_str) {
            if map.map(|m| m.contains_key(field)).unwrap_or(false) {
                continue;
            }
            return Err(format!("{path}: missing required property '{field}'"));
        }
    }

    if obj.get("additionalProperties") == Some(&Value::Bool(false)) {
        check_no_additional(obj, instance, path)?;
    }

    if let Some(items) = obj.get("items") {
        if let Some(arr) = instance.as_array() {
            for (idx, elem) in arr.iter().enumerate() {
                check(items, elem, &format!("{path}[{idx}]"))?;
            }
        }
    }

    Ok(())
}

fn check_type(ty: &str, instance: &Value, path: &str) -> Result<(), String> {
    let ok = match ty {
        "object" => instance.is_object(),
        "array" => instance.is_array(),
        "string" => instance.is_string(),
        "boolean" => instance.is_boolean(),
        "null" => instance.is_null(),
        "number" => instance.is_number(),
        "integer" => instance.is_i64() || instance.is_u64(),
        _ => true,
    };
    if ok {
        Ok(())
    } else {
        Err(format!("{path}: expected type '{ty}'"))
    }
}

fn check_numeric_bounds(
    obj: &serde_json::Map<String, Value>,
    instance: &Value,
    path: &str,
) -> Result<(), String> {
    if let Some(n) = instance.as_f64() {
        if let Some(min) = obj.get("minimum").and_then(Value::as_f64) {
            if n < min {
                return Err(format!("{path}: {n} below minimum {min}"));
            }
        }
        if let Some(max) = obj.get("maximum").and_then(Value::as_f64) {
            if n > max {
                return Err(format!("{path}: {n} above maximum {max}"));
            }
        }
    }
    Ok(())
}

fn check_string_bounds(
    obj: &serde_json::Map<String, Value>,
    instance: &Value,
    path: &str,
) -> Result<(), String> {
    if let Some(s) = instance.as_str() {
        let len = s.chars().count() as u64;
        if let Some(min) = obj.get("minLength").and_then(Value::as_u64) {
            if len < min {
                return Err(format!("{path}: string shorter than minLength {min}"));
            }
        }
        if let Some(max) = obj.get("maxLength").and_then(Value::as_u64) {
            if len > max {
                return Err(format!("{path}: string longer than maxLength {max}"));
            }
        }
    }
    Ok(())
}

fn check_no_additional(
    obj: &serde_json::Map<String, Value>,
    instance: &Value,
    path: &str,
) -> Result<(), String> {
    let Some(map) = instance.as_object() else {
        return Ok(());
    };
    let known = obj.get("properties").and_then(Value::as_object);
    for key in map.keys() {
        let allowed = known.map(|p| p.contains_key(key)).unwrap_or(false);
        if !allowed {
            return Err(format!("{path}: unexpected property '{key}'"));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn schema() -> Value {
        json!({
            "type": "object",
            "required": ["output"],
            "properties": {
                "output": {"type": "string", "minLength": 1},
                "score": {"type": "integer", "minimum": 0, "maximum": 100}
            }
        })
    }

    #[test]
    fn accepts_conforming_payload() {
        let ok = json!({"output": "echo: hi", "score": 42});
        assert!(validate(&schema(), &ok).is_ok());
    }

    #[test]
    fn rejects_missing_required_and_wrong_type() {
        let missing = json!({"score": 1});
        assert!(validate(&schema(), &missing)
            .unwrap_err()
            .contains("missing required property 'output'"));
        let wrong = json!({"output": 123});
        assert!(validate(&schema(), &wrong)
            .unwrap_err()
            .contains("expected type 'string'"));
    }

    #[test]
    fn rejects_out_of_range_number() {
        let bad = json!({"output": "ok", "score": 500});
        assert!(validate(&schema(), &bad)
            .unwrap_err()
            .contains("above maximum"));
    }

    #[test]
    fn enforces_enum_and_no_additional_properties() {
        let enum_schema = json!({"enum": ["a", "b"]});
        assert!(validate(&enum_schema, &json!("c")).is_err());
        assert!(validate(&enum_schema, &json!("a")).is_ok());
        let strict = json!({
            "type": "object",
            "properties": {"a": {"type": "string"}},
            "additionalProperties": false
        });
        assert!(validate(&strict, &json!({"a": "x", "b": 1})).is_err());
    }
}
