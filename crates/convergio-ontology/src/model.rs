use crate::{PropertyKey, SchemaVersion, TypeId};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// The value domain of a [`PropertyType`].
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum PropertyKind {
    /// UTF-8 string.
    String,
    /// Signed integer.
    Integer,
    /// Floating-point number.
    Number,
    /// Boolean.
    Boolean,
    /// IRI/URI string.
    Iri,
    /// RFC 3339 full-date.
    Date,
    /// RFC 3339 date-time.
    DateTime,
    /// Arbitrary JSON value.
    Json,
}

/// A reusable property type definition.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct PropertyType {
    /// Stable identifier.
    pub id: TypeId,
    /// Version of this property definition.
    pub schema_version: SchemaVersion,
    /// Human-facing title.
    pub title: String,
    /// Human-facing description.
    pub description: Option<String>,
    /// External IRI (CEDS/ELMO/ESCO-aligned, when applicable).
    pub iri: Option<String>,
    /// Value kind.
    pub kind: PropertyKind,
}

/// One property slot inside an [`ObjectType`].
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct ObjectProperty {
    /// Slot name inside the object.
    pub key: PropertyKey,
    /// References a [`PropertyType::id`].
    pub property_type: TypeId,
    /// Whether the slot must be present for a valid object instance.
    pub required: bool,
    /// Optional slot-specific description override.
    pub description: Option<String>,
}

/// A domain object type.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct ObjectType {
    /// Stable identifier.
    pub id: TypeId,
    /// Version of this object type definition.
    pub schema_version: SchemaVersion,
    /// Human-facing title.
    pub title: String,
    /// Human-facing description.
    pub description: Option<String>,

    /// Property slots.
    ///
    /// Deterministic (sorted) by key.
    pub properties: BTreeMap<PropertyKey, ObjectProperty>,

    /// When false (default), exports set `additionalProperties=false`.
    #[serde(default)]
    pub allow_additional_properties: bool,
}

/// A typed edge between two [`ObjectType`]s.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct LinkType {
    /// Stable identifier.
    pub id: TypeId,
    /// Version of this link type definition.
    pub schema_version: SchemaVersion,
    /// Human-facing title.
    pub title: String,
    /// Human-facing description.
    pub description: Option<String>,
    /// External IRI (when applicable).
    pub iri: Option<String>,

    /// Source object type id.
    pub from: TypeId,
    /// Target object type id.
    pub to: TypeId,
}
