//! Deterministic JSON-Schema export for `ObjectType` records.
//!
//! ADR-0053 § Canonical export and ADR-0047 (actions.json posture):
//! the exporter MUST be byte-identical across reruns and across
//! machines for identical inputs. Determinism is achieved by:
//!
//! - serialising through [`crate::hash::canonical_bytes`] (lexicographic
//!   key ordering, no insignificant whitespace),
//! - sorting the `properties` map and the `required` array by
//!   property name (ASCII ascending),
//! - encoding every datatype via a fixed, exhaustive
//!   [`datatype_fragment`] mapping,
//! - terminating the output with a single trailing `\n` so the
//!   artefact is a well-behaved text file (golden tests pin this).
//!
//! The exporter intentionally produces only the JSON-Schema for the
//! ObjectType + its owned properties. Cross-object link shapes are
//! the job of the SHACL exporter (W1 task 4).

use serde_json::{json, Map, Value};

use crate::error::{Error, Result};
use crate::hash::canonical_bytes;
use crate::model::{ObjectTypeRecord, PropertyTypeRecord};
use crate::store::Store;

/// Stable schema draft URL used by every artefact this exporter
/// produces. Pinned so a future draft bump is an explicit decision.
pub const JSON_SCHEMA_DRAFT: &str = "https://json-schema.org/draft/2020-12/schema";

/// Render a single `ObjectType` revision as a JSON-Schema document.
///
/// The output is canonical bytes plus a trailing newline; callers
/// can write it directly to disk or compare it against a golden
/// fixture without further processing.
pub async fn export_object_schema(
    store: &Store,
    object_name: &str,
    schema_version: i64,
) -> Result<Vec<u8>> {
    let object = store
        .get_object(object_name, schema_version)
        .await?
        .ok_or(Error::NotFound {
            kind: "object_type",
            name: format!("{}@{}", object_name, schema_version),
        })?;
    let properties = store
        .list_object_properties(object_name, schema_version)
        .await?;
    build_object_schema_bytes(&object, &properties)
}

/// Pure-function variant used by golden tests: build the JSON-Schema
/// bytes from already-fetched records. Has no IO.
pub fn build_object_schema_bytes(
    object: &ObjectTypeRecord,
    properties: &[PropertyTypeRecord],
) -> Result<Vec<u8>> {
    let value = build_object_schema_value(object, properties);
    let mut bytes = canonical_bytes(&value)?;
    bytes.push(b'\n');
    Ok(bytes)
}

fn build_object_schema_value(
    object: &ObjectTypeRecord,
    properties: &[PropertyTypeRecord],
) -> Value {
    let mut props_sorted: Vec<&PropertyTypeRecord> = properties.iter().collect();
    props_sorted.sort_by(|a, b| a.name.cmp(&b.name));

    let mut props_map = Map::new();
    let mut required: Vec<String> = Vec::new();
    for p in &props_sorted {
        props_map.insert(p.name.clone(), property_fragment(p));
        if p.required {
            required.push(p.name.clone());
        }
    }
    required.sort();

    let mut doc = Map::new();
    doc.insert("$schema".into(), Value::String(JSON_SCHEMA_DRAFT.into()));
    doc.insert(
        "$id".into(),
        Value::String(format!(
            "convergio:ontology:object:{}:{}",
            object.name, object.schema_version
        )),
    );
    doc.insert("title".into(), Value::String(object.title.clone()));
    if !object.description.is_empty() {
        doc.insert(
            "description".into(),
            Value::String(object.description.clone()),
        );
    }
    doc.insert("type".into(), Value::String("object".into()));
    doc.insert("properties".into(), Value::Object(props_map));
    doc.insert("required".into(), json!(required));
    doc.insert("additionalProperties".into(), Value::Bool(false));
    doc.insert(
        "x-convergio-content-hash".into(),
        Value::String(object.content_hash.clone()),
    );
    doc.insert(
        "x-convergio-schema-version".into(),
        Value::Number(object.schema_version.into()),
    );
    doc.insert("x-convergio-breaking".into(), Value::Bool(object.breaking));
    Value::Object(doc)
}

fn property_fragment(p: &PropertyTypeRecord) -> Value {
    let (ty, format) = datatype_fragment(&p.datatype);
    let mut frag = Map::new();
    frag.insert("type".into(), Value::String(ty.into()));
    if let Some(fmt) = format {
        frag.insert("format".into(), Value::String(fmt.into()));
    }
    if !p.title.is_empty() {
        frag.insert("title".into(), Value::String(p.title.clone()));
    }
    if !p.description.is_empty() {
        frag.insert("description".into(), Value::String(p.description.clone()));
    }
    if let Value::Object(extra) = &p.body {
        for (k, v) in extra {
            frag.entry(k.clone()).or_insert_with(|| v.clone());
        }
    }
    frag.insert(
        "x-convergio-content-hash".into(),
        Value::String(p.content_hash.clone()),
    );
    Value::Object(frag)
}

/// Map an ontology datatype to a `(json_schema_type, format)` pair.
/// Exhaustive over the strings emitted by the registrar; unknown
/// inputs fall through to `"string"` with no `format` so the export
/// is total. Adding a new datatype is an additive registry change.
pub fn datatype_fragment(datatype: &str) -> (&'static str, Option<&'static str>) {
    match datatype {
        "string" => ("string", None),
        "integer" => ("integer", None),
        "number" | "float" | "double" => ("number", None),
        "boolean" => ("boolean", None),
        "datetime" => ("string", Some("date-time")),
        "date" => ("string", Some("date")),
        "time" => ("string", Some("time")),
        "iri" | "uri" | "url" => ("string", Some("iri")),
        "uuid" => ("string", Some("uuid")),
        _ => ("string", None),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{ObjectTypeRecord, OwnerKind, PropertyTypeRecord};
    use chrono::TimeZone;
    use chrono::Utc;
    use serde_json::json;

    fn obj() -> ObjectTypeRecord {
        ObjectTypeRecord {
            name: "Person".into(),
            schema_version: 1,
            breaking: false,
            title: "Person".into(),
            description: "A natural person.".into(),
            body: json!({}),
            content_hash: "627a3000".into(),
            created_at: Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap(),
            audit_seq: None,
        }
    }

    fn prop(name: &str, datatype: &str, required: bool) -> PropertyTypeRecord {
        PropertyTypeRecord {
            name: name.into(),
            schema_version: 1,
            breaking: false,
            title: String::new(),
            description: String::new(),
            owner_kind: OwnerKind::Object,
            owner_name: "Person".into(),
            datatype: datatype.into(),
            required,
            body: json!({}),
            content_hash: format!("hash-{name}"),
            created_at: Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap(),
            audit_seq: None,
        }
    }

    #[test]
    fn datatype_mapping_is_exhaustive_for_known_inputs() {
        assert_eq!(datatype_fragment("string"), ("string", None));
        assert_eq!(datatype_fragment("integer"), ("integer", None));
        assert_eq!(datatype_fragment("boolean"), ("boolean", None));
        assert_eq!(datatype_fragment("datetime"), ("string", Some("date-time")));
        assert_eq!(datatype_fragment("iri"), ("string", Some("iri")));
        assert_eq!(datatype_fragment("uuid"), ("string", Some("uuid")));
        assert_eq!(datatype_fragment("nonsense"), ("string", None));
    }

    #[test]
    fn export_is_byte_identical_across_runs() {
        let o = obj();
        let p = vec![prop("email", "string", true), prop("age", "integer", false)];
        let a = build_object_schema_bytes(&o, &p).unwrap();
        let b = build_object_schema_bytes(&o, &p).unwrap();
        assert_eq!(a, b);
    }

    #[test]
    fn properties_and_required_are_sorted() {
        let o = obj();
        let p = vec![prop("zeta", "string", true), prop("alpha", "string", true)];
        let bytes = build_object_schema_bytes(&o, &p).unwrap();
        let s = std::str::from_utf8(&bytes).unwrap();
        let alpha_pos = s.find("\"alpha\"").unwrap();
        let zeta_pos = s.find("\"zeta\"").unwrap();
        assert!(alpha_pos < zeta_pos);
        assert!(s.contains("\"required\":[\"alpha\",\"zeta\"]"));
    }

    #[test]
    fn output_ends_with_single_newline() {
        let o = obj();
        let p = vec![prop("x", "string", true)];
        let bytes = build_object_schema_bytes(&o, &p).unwrap();
        assert_eq!(*bytes.last().unwrap(), b'\n');
        assert_ne!(bytes[bytes.len() - 2], b'\n');
    }
}
