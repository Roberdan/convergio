//! Response DTOs and CLI value-enums shared by `cvg ontology`
//! subcommands. Extracted from `ontology.rs` (ADR-0053, W1 T9) to
//! keep that file below the 300-line cap once the diff / lineage
//! handlers landed.

use clap::ValueEnum;
use serde::{Deserialize, Serialize};

/// Export format selector for `cvg ontology export`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, ValueEnum)]
pub enum ExportFormatArg {
    /// Canonical JSON-Schema document (draft 2020-12).
    Jsonschema,
    /// SHACL shape graph encoded as JSON-LD.
    Shacl,
}

impl ExportFormatArg {
    /// URL fragment expected by the daemon.
    pub fn as_path(self) -> &'static str {
        match self {
            Self::Jsonschema => "jsonschema",
            Self::Shacl => "shacl",
        }
    }
}

/// Render format selector for `cvg ontology diff|lineage|branch-diff`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, ValueEnum)]
pub enum GraphFormatArg {
    /// Stable canonical JSON.
    Json,
    /// Mermaid flowchart.
    Mermaid,
    /// Graphviz DOT.
    Dot,
}

impl GraphFormatArg {
    /// Query-string value expected by the daemon.
    pub fn as_query(self) -> &'static str {
        match self {
            Self::Json => "json",
            Self::Mermaid => "mermaid",
            Self::Dot => "dot",
        }
    }
}

/// One row in `cvg ontology list-types`.
#[derive(Deserialize, Serialize)]
pub struct TypeRow {
    /// "object" or "link".
    pub kind: String,
    /// Registry name.
    pub name: String,
    /// Schema version of the row.
    pub schema_version: i64,
    /// Display title (may be empty).
    pub title: String,
    /// Free-form description (may be empty).
    pub description: String,
    /// Stable content hash.
    pub content_hash: String,
}

/// Body of `GET /v1/ontology/types`.
#[derive(Deserialize, Serialize)]
pub struct ListResponse {
    /// Registered `ObjectType` rows (latest revision per name).
    pub objects: Vec<TypeRow>,
    /// Registered `LinkType` rows (latest revision per name).
    pub links: Vec<TypeRow>,
}

/// One property row in `cvg ontology describe object …`.
#[derive(Deserialize, Serialize)]
pub struct PropertyRow {
    /// Property name.
    pub name: String,
    /// Schema version.
    pub schema_version: i64,
    /// Primitive datatype (string, integer, …).
    pub datatype: String,
    /// `true` when the property is required on its owner.
    pub required: bool,
    /// Display title.
    pub title: String,
    /// Free-form description.
    pub description: String,
    /// Stable content hash.
    pub content_hash: String,
}

/// Body of `GET /v1/ontology/types/object/:name`.
#[derive(Deserialize, Serialize)]
pub struct DescribeObject {
    /// Object name.
    pub name: String,
    /// Schema version returned.
    pub schema_version: i64,
    /// Display title.
    pub title: String,
    /// Free-form description.
    pub description: String,
    /// `true` when this revision is tagged breaking.
    pub breaking: bool,
    /// Stable content hash.
    pub content_hash: String,
    /// Inlined properties of this object.
    pub properties: Vec<PropertyRow>,
}

/// Body of `GET /v1/ontology/types/link/:name`.
#[derive(Deserialize, Serialize)]
pub struct DescribeLink {
    /// Link name.
    pub name: String,
    /// Schema version returned.
    pub schema_version: i64,
    /// Display title.
    pub title: String,
    /// Free-form description.
    pub description: String,
    /// Source object name.
    pub from_object: String,
    /// Destination object name.
    pub to_object: String,
    /// `true` when this revision is tagged breaking.
    pub breaking: bool,
    /// Stable content hash.
    pub content_hash: String,
}
