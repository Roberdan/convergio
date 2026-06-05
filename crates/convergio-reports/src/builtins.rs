//! Built-in ontology types shipped with the report engine.
//!
//! Convergio generally avoids shipping domain ontologies in core, but the
//! report engine benefits from having its own schema available for typed
//! registration and downstream tooling.

use serde_json::json;

/// Ontology `ObjectType` name for Convergio report templates.
pub const REPORT_TEMPLATE_OBJECT_TYPE_ID: &str = "cvg.report_template.v1";

/// Built-in `ReportTemplate` ontology schema version.
pub const REPORT_TEMPLATE_SCHEMA_VERSION: i64 = 1;

/// Canonical `ObjectType` definition for [`crate::types::ReportTemplate`].
///
/// This is intended for tooling and for validating template registration
/// payloads. It does **not** describe the params schema of a specific report;
/// that schema is referenced per-template via `params_object_type_id`.
pub fn report_template_object_type() -> serde_json::Value {
    json!({
        "$schema": "http://json-schema.org/draft-07/schema#",
        "title": "ReportTemplate",
        "type": "object",
        "properties": {
            "id": {"type": "string", "minLength": 1},
            "title": {"type": "string"},
            "description": {"type": "string"},
            "template_html": {"type": "string"},
            "template_typst": {"type": "string"},
            "template_docx": {"type": "string"},
            "params_object_type_id": {"type": "string", "minLength": 1}
        },
        "required": ["id", "title", "description", "params_object_type_id"],
        "anyOf": [
            {"required": ["template_html"]},
            {"required": ["template_typst"]},
            {"required": ["template_docx"]}
        ],
        "additionalProperties": false
    })
}
