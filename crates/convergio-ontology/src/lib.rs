//! Ontology Runtime Core for Convergio.
//!
//! Implements the platform-side primitive described in ADR-0053: a
//! schema registry of typed domain objects, links, and properties
//! that becomes the peer of the Modulor `(task, evidence, gate,
//! audit_row)` tuple. The shape here is `(object, link, property,
//! schema_version)`.
//!
//! # Scope
//!
//! - **In scope (this crate):** `ObjectType`, `LinkType`,
//!   `PropertyType` records, evolution rules, deterministic
//!   JSON-Schema and SHACL export, diff between schema versions.
//! - **Out of scope (verticals own these):** concrete domain
//!   instances. Convergio ships **zero** built-in `ObjectType`
//!   instances. Accelerators such as `convergio-edu`,
//!   `convergio-research`, `convergio-healthcare-compliance`
//!   register their YAML at plan-create time.
//!
//! # Status
//!
//! W1 task 2 adds the SQLite schema (`migrations/1000_*.sql`,
//! range 1000-1099 per ADR-0003) and the [`Store`] handle that
//! upserts and reads `ObjectType` / `LinkType` / `PropertyType`
//! rows. Later tasks add the deterministic exporters (W1 tasks 3
//! and 4), the `cvg ontology` CLI surface (W1 task 5), and the MCP
//! `ontology.*` actions (W1 task 6).
//!
//! # Determinism
//!
//! Every export produced by this crate MUST be byte-identical across
//! reruns and across machines for identical inputs. The posture
//! mirrors `actions.json` (ADR-0047) and the graph output formats
//! (ADR-0060). Golden tests enforce the invariant per export
//! surface.

#![forbid(unsafe_code)]

pub mod actions;
mod diff;
mod er;
mod error;
mod graph_render;
mod hash;
mod import;
mod jsonschema;
mod lineage;
mod migrate;
mod model;
mod object_events;
mod object_storage;
mod purposes;
mod reads;
mod semantic;
mod shacl;
mod store;

pub use diff::{diff_object, ObjectDiff, PropertyChange, PropertyRef};
pub use er::{EntityResolver, MatchGroup, MatchKey, MatchRule, MatchStrategy, SAME_AS_LINK_TYPE};
pub use error::{Error, Result};
pub use graph_render::{
    render_diff_dot, render_diff_mermaid, render_lineage_dot, render_lineage_mermaid,
};
pub use hash::content_hash;
pub use import::{
    import_draft, ImportDraft, ImportLink, ImportObject, ImportProperty, ImportReport,
};
pub use jsonschema::{
    build_object_schema_bytes, datatype_fragment, export_object_schema, JSON_SCHEMA_DRAFT,
};
pub use lineage::{lineage_object, Lineage, LineageNode};
pub use migrate::init;
pub use model::{
    LinkTypeRecord, ObjectTypeRecord, OwnerKind, PropertyTypeRecord, TypeKind, TypeRecordRef,
};
pub use object_events::{NewObjectEvent, ObjectEvent, ObjectEventsStore};
pub use object_storage::{
    LinkOp, ObjectInstance, ObjectLinkEvent, ObjectPropertyEvent, OntologyStore, PropertyOp,
};
pub use purposes::{PurposeRecord, PurposeStore};
pub use shacl::{build_object_shacl_bytes, export_object_shacl, shacl_datatype, ShaclType};
pub use store::Store;
