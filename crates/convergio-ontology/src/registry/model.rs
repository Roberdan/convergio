use crate::diff::ChangeClass;
use crate::{LinkType, ObjectType, PropertyType, SchemaVersion, TypeId};
use serde::Serialize;
use uuid::Uuid;

/// Thin kind wrapper for heterogeneous specs.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", content = "spec", rename_all = "snake_case")]
pub enum SchemaSpec {
    /// An [`ObjectType`](crate::ObjectType) spec.
    Object(ObjectType),
    /// A [`LinkType`](crate::LinkType) spec.
    Link(LinkType),
    /// A [`PropertyType`](crate::PropertyType) spec.
    Property(PropertyType),
}

/// Metadata for a schema spec.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SchemaSpecMeta {
    /// Kind label (`object`, `link`, `property`).
    pub kind: &'static str,
    /// Stable identifier.
    pub id: TypeId,
    /// Schema version.
    pub schema_version: SchemaVersion,
}

impl SchemaSpec {
    /// Extract metadata needed by the registry.
    pub fn meta(&self) -> SchemaSpecMeta {
        match self {
            SchemaSpec::Object(v) => SchemaSpecMeta {
                kind: "object",
                id: v.id.clone(),
                schema_version: v.schema_version,
            },
            SchemaSpec::Link(v) => SchemaSpecMeta {
                kind: "link",
                id: v.id.clone(),
                schema_version: v.schema_version,
            },
            SchemaSpec::Property(v) => SchemaSpecMeta {
                kind: "property",
                id: v.id.clone(),
                schema_version: v.schema_version,
            },
        }
    }
}

/// Registry write result with audit-friendly metadata.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RegisteredSchema {
    /// Kind label (`object`, `link`, `property`).
    pub kind: &'static str,
    /// Stable identifier.
    pub id: TypeId,
    /// Schema version.
    pub schema_version: SchemaVersion,
    /// SHA-256 of the canonical JSON representation.
    pub content_hash: String,
    /// Whether this registration is explicitly marked as breaking.
    pub breaking: bool,
    /// Migration plan reference required for breaking changes.
    pub migration_plan: Option<Uuid>,
    /// Computed change class.
    pub change_class: ChangeClass,
}
