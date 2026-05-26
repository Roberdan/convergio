//! Canonical JSON encoding for stable hashing.

use serde_json::Value;
use std::collections::BTreeMap;

/// Convert a JSON value into a canonical form by sorting object keys.
pub fn canonicalize(v: Value) -> Value {
    match v {
        Value::Object(map) => {
            let mut out: BTreeMap<String, Value> = BTreeMap::new();
            for (k, v) in map {
                out.insert(k, canonicalize(v));
            }
            Value::Object(out.into_iter().collect())
        }
        Value::Array(xs) => Value::Array(xs.into_iter().map(canonicalize).collect()),
        other => other,
    }
}

/// Serialize a value as canonical JSON bytes.
pub fn to_canonical_bytes(v: &impl serde::Serialize) -> Result<Vec<u8>, serde_json::Error> {
    let value = serde_json::to_value(v)?;
    let canon = canonicalize(value);
    serde_json::to_vec(&canon)
}
