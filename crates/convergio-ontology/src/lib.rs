//! Ontology DSL + versioned schema registry primitives.
//!
//! This crate is deliberately **IO-free** at runtime. It defines the data
//! model (`ObjectType`, `LinkType`, `PropertyType`), a strict semver type,
//! a small in-memory registry with a migration policy, and deterministic
//! JSON-Schema export.

mod diff;
mod ids;
mod iri_mapping;
mod json_schema;
mod model;
mod registry;
mod version;

pub use crate::diff::{ChangeClass, ChangeReport};
pub use crate::ids::{PropertyKey, TypeId};
pub use crate::iri_mapping::{
    ExternalSystem, InternalKind, IriMappingError, IriMappingRow, IriMappingTable,
};
pub use crate::json_schema::{export_object_json_schema, JsonSchemaExportError};
pub use crate::model::{LinkType, ObjectProperty, ObjectType, PropertyKind, PropertyType};
pub use crate::registry::{
    RegisteredSchema, RegistryError, SchemaRegistry, SchemaSpec, SchemaSpecMeta,
};
pub use crate::version::{SchemaVersion, SchemaVersionParseError};
