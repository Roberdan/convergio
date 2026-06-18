//! Record shapes for ontology types.
//!
//! These are persistence-facing structs. The exporters in W1 tasks 3
//! and 4 convert them into JSON-Schema and SHACL artefacts; the CLI
//! in task 5 renders them for humans. No domain content is encoded
//! here per ADR-0053 — the registry is a primitive.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Discriminator for the three peer families in the registry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TypeKind {
    /// A typed thing the user can talk about.
    Object,
    /// A typed relation between two `Object` types.
    Link,
    /// A typed attribute of an `Object` or `Link`.
    Property,
}

impl TypeKind {
    pub(crate) fn as_static_str(self) -> &'static str {
        match self {
            TypeKind::Object => "object",
            TypeKind::Link => "link",
            TypeKind::Property => "property",
        }
    }
}

/// Owner of a `PropertyType`: either an `ObjectType` or a `LinkType`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OwnerKind {
    /// Attached to an `ObjectType`.
    Object,
    /// Attached to a `LinkType`.
    Link,
}

impl OwnerKind {
    pub(crate) fn as_db_str(self) -> &'static str {
        match self {
            OwnerKind::Object => "object",
            OwnerKind::Link => "link",
        }
    }

    pub(crate) fn from_db_str(s: &str) -> Option<Self> {
        match s {
            "object" => Some(OwnerKind::Object),
            "link" => Some(OwnerKind::Link),
            _ => None,
        }
    }
}

/// A schema record for a typed domain object.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ObjectTypeRecord {
    /// Registry name, unique within `(name, schema_version)`.
    pub name: String,
    /// Monotonic version inside this `name`. The first row for a
    /// name MUST land at `1`.
    pub schema_version: i64,
    /// `true` if this revision is breaking relative to the previous
    /// version of the same `name`; gates use this to enforce the
    /// consumer-facing semver bump (see ADR-0053).
    pub breaking: bool,
    /// Short human title.
    pub title: String,
    /// Long human description; never `Option` so the canonical body
    /// stays deterministic — empty string is the absent sentinel.
    pub description: String,
    /// Free-form semantic body. Must be a stable, canonical shape;
    /// hashing happens before persistence.
    pub body: Value,
    /// Set by the store on insert; consumers should treat
    /// reads as authoritative.
    pub content_hash: String,
    /// Set by the store on insert.
    pub created_at: DateTime<Utc>,
    /// Audit row that introduced this version. `None` until the
    /// daemon writes the `ontology.object_type.registered` row.
    pub audit_seq: Option<i64>,
}

impl ObjectTypeRecord {
    /// Whether instances of this type may only be created/mutated under a
    /// declared, registered purpose (ADR-0082). Stored as a `requires_purpose`
    /// boolean in the schema `body`; defaults to `false` (opt-in).
    pub fn requires_purpose(&self) -> bool {
        self.body
            .get("requires_purpose")
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(false)
    }
}

/// A schema record for a typed relation between two object types.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LinkTypeRecord {
    /// Registry name.
    pub name: String,
    /// Monotonic version, same rules as [`ObjectTypeRecord`].
    pub schema_version: i64,
    /// `true` for breaking revisions.
    pub breaking: bool,
    /// Short human title.
    pub title: String,
    /// Long human description; empty string means "absent".
    pub description: String,
    /// Source object type name.
    pub from_object: String,
    /// Target object type name.
    pub to_object: String,
    /// Free-form semantic body.
    pub body: Value,
    /// Content hash, populated on insert.
    pub content_hash: String,
    /// Populated on insert.
    pub created_at: DateTime<Utc>,
    /// Audit row that introduced this version.
    pub audit_seq: Option<i64>,
}

/// A schema record for a typed attribute of an object or link.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PropertyTypeRecord {
    /// Registry name.
    pub name: String,
    /// Monotonic version, same rules as [`ObjectTypeRecord`].
    pub schema_version: i64,
    /// `true` for breaking revisions.
    pub breaking: bool,
    /// Short human title.
    pub title: String,
    /// Long human description; empty string means "absent".
    pub description: String,
    /// Whether the owner is an object type or a link type.
    pub owner_kind: OwnerKind,
    /// Name of the owning object or link type.
    pub owner_name: String,
    /// Datatype tag — opaque string for W1, normalised against the
    /// JSON-Schema datatype set in W1 task 3.
    pub datatype: String,
    /// Whether the property is required on instances of the owner.
    pub required: bool,
    /// Free-form semantic body.
    pub body: Value,
    /// Content hash, populated on insert.
    pub content_hash: String,
    /// Populated on insert.
    pub created_at: DateTime<Utc>,
    /// Audit row that introduced this version.
    pub audit_seq: Option<i64>,
}

/// Tagged reference to one of the three record shapes — useful for
/// read-side APIs that iterate the whole registry.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum TypeRecordRef {
    /// `ObjectType` row.
    Object(ObjectTypeRecord),
    /// `LinkType` row.
    Link(LinkTypeRecord),
    /// `PropertyType` row.
    Property(PropertyTypeRecord),
}
