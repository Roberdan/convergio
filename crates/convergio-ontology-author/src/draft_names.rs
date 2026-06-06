//! Machine-name grammar and datatype normalization.
//!
//! LLM output cannot be trusted to produce stable, RDF-safe
//! identifiers. Object / link / property *names* are the machine
//! identifiers that flow into IRIs, CURIE local-names, JSON-Schema
//! keys, and SHACL shapes, so they must match a strict grammar.
//! Human-facing labels live in `title`/`description` and are free.

/// The canonical, RDF-safe datatypes accepted by the ontology
/// exporters. Anything outside this set (after normalization) is a
/// validation violation rather than a silent fallback to `string`.
pub const CANONICAL_DATATYPES: &[&str] = &[
    "string", "integer", "number", "boolean", "datetime", "date", "time", "iri", "uuid",
];

/// `true` when `name` is a stable, RDF-safe machine identifier:
/// `^[A-Za-z][A-Za-z0-9_]*$`.
pub fn is_valid_name(name: &str) -> bool {
    let mut chars = name.chars();
    match chars.next() {
        Some(c) if c.is_ascii_alphabetic() => {}
        _ => return false,
    }
    chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
}

/// Normalize a datatype alias to its canonical form, or `None` when it
/// is not recognised. Aliases cover the common shapes an LLM emits.
pub fn normalize_datatype(raw: &str) -> Option<&'static str> {
    let key = raw.trim().to_ascii_lowercase();
    let canonical = match key.as_str() {
        "string" | "str" | "text" | "varchar" => "string",
        "integer" | "int" | "long" | "bigint" => "integer",
        "number" | "float" | "double" | "decimal" | "real" => "number",
        "boolean" | "bool" => "boolean",
        "datetime" | "date-time" | "timestamp" => "datetime",
        "date" => "date",
        "time" => "time",
        "iri" | "uri" | "url" => "iri",
        "uuid" | "guid" => "uuid",
        _ => return None,
    };
    Some(canonical)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_unsafe_names() {
        assert!(is_valid_name("Student"));
        assert!(is_valid_name("course_offering"));
        assert!(!is_valid_name("Student Record"));
        assert!(!is_valid_name("1Student"));
        assert!(!is_valid_name("course-offering"));
        assert!(!is_valid_name(""));
        assert!(!is_valid_name("a:b"));
    }

    #[test]
    fn normalizes_datatype_aliases() {
        assert_eq!(normalize_datatype("Text"), Some("string"));
        assert_eq!(normalize_datatype("int"), Some("integer"));
        assert_eq!(normalize_datatype("dateTime"), Some("datetime"));
        assert_eq!(normalize_datatype("bool"), Some("boolean"));
        assert_eq!(normalize_datatype("url"), Some("iri"));
        assert_eq!(normalize_datatype("blob"), None);
    }
}
