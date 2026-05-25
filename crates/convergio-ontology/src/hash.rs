//! Canonical hashing for ontology records.
//!
//! Every record carries a `content_hash` that is a sha256 over the
//! canonical JSON serialization of the record's semantic fields
//! (everything that defines the schema, excluding bookkeeping like
//! `created_at` or `audit_seq`). The serializer used by the
//! exporters in W1 tasks 3 and 4 produces the same canonical bytes,
//! so the hash is comparable across the database and the published
//! artefacts. See ADR-0053 § Determinism and ADR-0060 § Canonical
//! ordering for the wider posture.

use serde_json::Value;
use sha2::{Digest, Sha256};

use crate::error::Result;

/// Compute the lowercase-hex sha256 of the canonical JSON
/// serialization of `value`. "Canonical" here means objects are
/// emitted with keys in lexicographic order; all callers are
/// expected to pass values that were themselves built from a stable
/// shape (see [`crate::model`] helpers).
pub fn content_hash(value: &Value) -> Result<String> {
    let canonical = canonical_bytes(value)?;
    let digest = Sha256::digest(&canonical);
    Ok(hex_lower(&digest))
}

pub(crate) fn canonical_bytes(value: &Value) -> Result<Vec<u8>> {
    let mut buf = Vec::with_capacity(128);
    write_canonical(&mut buf, value)?;
    Ok(buf)
}

fn write_canonical(buf: &mut Vec<u8>, value: &Value) -> Result<()> {
    match value {
        Value::Null | Value::Bool(_) | Value::Number(_) | Value::String(_) => {
            serde_json::to_writer(&mut *buf, value)?;
        }
        Value::Array(items) => {
            buf.push(b'[');
            for (i, item) in items.iter().enumerate() {
                if i > 0 {
                    buf.push(b',');
                }
                write_canonical(buf, item)?;
            }
            buf.push(b']');
        }
        Value::Object(map) => {
            let mut keys: Vec<&String> = map.keys().collect();
            keys.sort();
            buf.push(b'{');
            for (i, k) in keys.iter().enumerate() {
                if i > 0 {
                    buf.push(b',');
                }
                serde_json::to_writer(&mut *buf, k)?;
                buf.push(b':');
                write_canonical(buf, &map[*k])?;
            }
            buf.push(b'}');
        }
    }
    Ok(())
}

fn hex_lower(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        out.push(HEX[(b >> 4) as usize] as char);
        out.push(HEX[(b & 0x0f) as usize] as char);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn key_order_independence() {
        let a = json!({"a": 1, "b": [{"x": 1, "y": 2}], "c": null});
        let b = json!({"c": null, "b": [{"y": 2, "x": 1}], "a": 1});
        assert_eq!(content_hash(&a).unwrap(), content_hash(&b).unwrap());
    }

    #[test]
    fn deterministic_across_runs() {
        let v = json!({"k": [1, 2, 3], "n": "x"});
        let h1 = content_hash(&v).unwrap();
        let h2 = content_hash(&v).unwrap();
        assert_eq!(h1, h2);
        assert_eq!(h1.len(), 64);
    }

    #[test]
    fn null_and_empty_are_distinct() {
        let a = content_hash(&json!({})).unwrap();
        let b = content_hash(&json!({"k": null})).unwrap();
        assert_ne!(a, b);
    }
}
