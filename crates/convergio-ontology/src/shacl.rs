//! Deterministic SHACL/JSON-LD shape export for `ObjectType`
//! records.
//!
//! Same posture as [`crate::jsonschema`]: lexicographic key
//! ordering via [`crate::hash::canonical_bytes`], explicit property
//! sort by name, single trailing `\n`. ADR-0053 § Canonical export.
//!
//! The output is one JSON-LD document per `ObjectType` revision,
//! holding a `sh:NodeShape` plus a sorted list of `sh:PropertyShape`
//! children. Linked-data consumers (e.g. validators that speak
//! SHACL) can ingest it without further normalisation.

use serde_json::{json, Map, Value};

use crate::error::{Error, Result};
use crate::hash::canonical_bytes;
use crate::model::{ObjectTypeRecord, PropertyTypeRecord};
use crate::store::Store;

/// JSON-LD `@context` used by every SHACL artefact this exporter
/// produces. Kept stable so a golden test can pin the bytes.
const SHACL_CONTEXT_SH: &str = "http://www.w3.org/ns/shacl#";
const SHACL_CONTEXT_XSD: &str = "http://www.w3.org/2001/XMLSchema#";
const SHACL_CONTEXT_CONVERGIO: &str = "https://convergio.local/ontology#";

/// Render a single `ObjectType` revision as a SHACL/JSON-LD
/// document. Canonical bytes + trailing newline; safe to write
/// straight to disk and compare against a golden fixture.
pub async fn export_object_shacl(
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
    build_object_shacl_bytes(&object, &properties)
}

/// Pure-function variant used by golden tests: no IO.
pub fn build_object_shacl_bytes(
    object: &ObjectTypeRecord,
    properties: &[PropertyTypeRecord],
) -> Result<Vec<u8>> {
    let value = build_object_shacl_value(object, properties);
    let mut bytes = canonical_bytes(&value)?;
    bytes.push(b'\n');
    Ok(bytes)
}

fn build_object_shacl_value(object: &ObjectTypeRecord, properties: &[PropertyTypeRecord]) -> Value {
    let mut props_sorted: Vec<&PropertyTypeRecord> = properties.iter().collect();
    props_sorted.sort_by(|a, b| a.name.cmp(&b.name));

    let property_shapes: Vec<Value> = props_sorted
        .iter()
        .map(|p| property_shape(object.name.as_str(), p))
        .collect();

    let mut ctx = Map::new();
    ctx.insert("sh".into(), Value::String(SHACL_CONTEXT_SH.into()));
    ctx.insert("xsd".into(), Value::String(SHACL_CONTEXT_XSD.into()));
    ctx.insert(
        "convergio".into(),
        Value::String(SHACL_CONTEXT_CONVERGIO.into()),
    );

    let mut doc = Map::new();
    doc.insert("@context".into(), Value::Object(ctx));
    doc.insert(
        "@id".into(),
        Value::String(format!(
            "convergio:ontology:object:{}:{}:shape",
            object.name, object.schema_version
        )),
    );
    doc.insert("@type".into(), Value::String("sh:NodeShape".into()));
    doc.insert(
        "sh:targetClass".into(),
        Value::String(format!("convergio:ontology:object:{}", object.name)),
    );
    doc.insert("sh:name".into(), Value::String(object.title.clone()));
    if !object.description.is_empty() {
        doc.insert(
            "sh:description".into(),
            Value::String(object.description.clone()),
        );
    }
    doc.insert("sh:closed".into(), Value::Bool(true));
    doc.insert("sh:property".into(), json!(property_shapes));
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

fn property_shape(object_name: &str, p: &PropertyTypeRecord) -> Value {
    let mut shape = Map::new();
    shape.insert(
        "@id".into(),
        Value::String(format!(
            "convergio:ontology:object:{}:{}:{}:shape",
            object_name, p.name, p.schema_version
        )),
    );
    shape.insert("@type".into(), Value::String("sh:PropertyShape".into()));
    shape.insert(
        "sh:path".into(),
        Value::String(format!("convergio:property:{}", p.name)),
    );

    let mapping = shacl_datatype(&p.datatype);
    match mapping {
        ShaclType::Datatype(d) => {
            shape.insert("sh:datatype".into(), Value::String(format!("xsd:{d}")));
        }
        ShaclType::Iri => {
            shape.insert("sh:nodeKind".into(), Value::String("sh:IRI".into()));
        }
    }

    if p.required {
        shape.insert("sh:minCount".into(), Value::Number(1.into()));
    }
    if !p.title.is_empty() {
        shape.insert("sh:name".into(), Value::String(p.title.clone()));
    }
    if !p.description.is_empty() {
        shape.insert(
            "sh:description".into(),
            Value::String(p.description.clone()),
        );
    }
    shape.insert(
        "x-convergio-content-hash".into(),
        Value::String(p.content_hash.clone()),
    );
    Value::Object(shape)
}

/// Either an XSD datatype name (literal-valued property) or the
/// marker for an IRI-valued property (`sh:nodeKind sh:IRI`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShaclType {
    /// Literal property with the given XSD datatype (rendered as
    /// `sh:datatype xsd:<name>`).
    Datatype(&'static str),
    /// IRI-valued property; emitted as `sh:nodeKind sh:IRI`.
    Iri,
}

/// Map an ontology datatype to its SHACL counterpart. Exhaustive
/// over known inputs; unknown falls through to `xsd:string` so the
/// export stays total.
pub fn shacl_datatype(datatype: &str) -> ShaclType {
    match datatype {
        "string" => ShaclType::Datatype("string"),
        "integer" => ShaclType::Datatype("integer"),
        "number" | "float" | "double" => ShaclType::Datatype("decimal"),
        "boolean" => ShaclType::Datatype("boolean"),
        "datetime" => ShaclType::Datatype("dateTime"),
        "date" => ShaclType::Datatype("date"),
        "time" => ShaclType::Datatype("time"),
        "iri" | "uri" | "url" => ShaclType::Iri,
        "uuid" => ShaclType::Datatype("string"),
        _ => ShaclType::Datatype("string"),
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
            content_hash: "0bj-hash".into(),
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
            content_hash: format!("prop-{name}"),
            created_at: Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap(),
            audit_seq: None,
        }
    }

    #[test]
    fn datatype_mapping_covers_known_inputs() {
        assert_eq!(shacl_datatype("string"), ShaclType::Datatype("string"));
        assert_eq!(shacl_datatype("integer"), ShaclType::Datatype("integer"));
        assert_eq!(shacl_datatype("number"), ShaclType::Datatype("decimal"));
        assert_eq!(shacl_datatype("boolean"), ShaclType::Datatype("boolean"));
        assert_eq!(shacl_datatype("datetime"), ShaclType::Datatype("dateTime"));
        assert_eq!(shacl_datatype("iri"), ShaclType::Iri);
        assert_eq!(shacl_datatype("uuid"), ShaclType::Datatype("string"));
        assert_eq!(shacl_datatype("nonsense"), ShaclType::Datatype("string"));
    }

    #[test]
    fn export_is_byte_identical_across_runs() {
        let o = obj();
        let p = vec![prop("email", "string", true), prop("id", "iri", true)];
        let a = build_object_shacl_bytes(&o, &p).unwrap();
        let b = build_object_shacl_bytes(&o, &p).unwrap();
        assert_eq!(a, b);
    }

    #[test]
    fn property_shapes_are_sorted() {
        let o = obj();
        let p = vec![prop("zeta", "string", true), prop("alpha", "string", false)];
        let bytes = build_object_shacl_bytes(&o, &p).unwrap();
        let s = std::str::from_utf8(&bytes).unwrap();
        let alpha = s.find(":Person:alpha").unwrap();
        let zeta = s.find(":Person:zeta").unwrap();
        assert!(alpha < zeta);
    }

    #[test]
    fn output_ends_with_single_newline() {
        let o = obj();
        let p = vec![prop("x", "string", true)];
        let bytes = build_object_shacl_bytes(&o, &p).unwrap();
        assert_eq!(*bytes.last().unwrap(), b'\n');
        assert_ne!(bytes[bytes.len() - 2], b'\n');
    }
}
