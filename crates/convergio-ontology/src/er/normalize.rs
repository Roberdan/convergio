//! Deterministic value normalization for entity resolution.
//!
//! Both helpers are pure and side-effect free so the resolver stays
//! byte-identical across reruns and machines for identical inputs
//! (the crate-wide determinism posture; see [`crate`]).

/// Normalize a textual property value for deterministic matching.
///
/// The transform is fixed and order-stable: trim surrounding whitespace,
/// collapse every internal run of (Unicode) whitespace to a single ASCII
/// space, then lowercase. Two values that differ only in case or spacing
/// therefore produce the same normalized key.
pub(crate) fn normalize(raw: &str) -> String {
    raw.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
}

/// Extract the comparable text of a stored property value.
///
/// `object_properties.value_json` holds a JSON-encoded payload. A JSON
/// string yields its inner text; any other JSON scalar or structure yields
/// its compact JSON form, so numbers and booleans still match
/// deterministically. A non-JSON payload is returned verbatim.
pub(crate) fn value_text(value_json: &str) -> String {
    match serde_json::from_str::<serde_json::Value>(value_json) {
        Ok(serde_json::Value::String(s)) => s,
        Ok(other) => other.to_string(),
        Err(_) => value_json.to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trims_collapses_and_lowercases() {
        assert_eq!(normalize("  Alice   Smith "), "alice smith");
        assert_eq!(normalize("ALICE\tSMITH"), "alice smith");
        assert_eq!(normalize("alice smith"), "alice smith");
    }

    #[test]
    fn empty_and_whitespace_only_normalize_to_empty() {
        assert_eq!(normalize(""), "");
        assert_eq!(normalize("   \t\n "), "");
    }

    #[test]
    fn value_text_unwraps_json_string() {
        assert_eq!(value_text("\"Alice Smith\""), "Alice Smith");
    }

    #[test]
    fn value_text_keeps_scalars_and_raw() {
        assert_eq!(value_text("42"), "42");
        assert_eq!(value_text("true"), "true");
        assert_eq!(value_text("not json"), "not json");
    }
}
