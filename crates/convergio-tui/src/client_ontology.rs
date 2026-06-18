//! Read-only ontology HTTP fetchers for the Inspector (W6, ADR-0059).
//!
//! These methods extend [`crate::client::Client`] with `GET` calls
//! against the ontology surface the daemon already serves on `main`
//! (ADR-0053 / ADR-0060):
//!
//! - `GET /v1/ontology/types` — every registered object / link type.
//! - `GET /v1/ontology/lineage/object/:name` — the schema-version
//!   chain for one object type.
//! - `GET /v1/ontology/branches` — scenario branch overlays.
//!
//! The DTOs below mirror the server response shapes. They are
//! deliberately local to this crate (no dependency on
//! `convergio-server` / `convergio-durability`) so the TUI keeps
//! compiling in isolation against the HTTP contract, per the crate
//! invariants in `AGENTS.md`.

use crate::client::Client;
use anyhow::Result;
use serde::Deserialize;

/// One registered object or link type, as returned by
/// `GET /v1/ontology/types`.
#[derive(Debug, Clone, Default, Deserialize, PartialEq, Eq)]
pub struct OntologyTypeRow {
    /// `"object"` or `"link"`.
    #[serde(default)]
    pub kind: String,
    /// Registry name (stable identifier).
    pub name: String,
    /// Latest schema (semver-major) version.
    #[serde(default)]
    pub schema_version: i64,
    /// Display title.
    #[serde(default)]
    pub title: String,
    /// Human description.
    #[serde(default)]
    pub description: String,
    /// Content hash of the latest revision.
    #[serde(default)]
    pub content_hash: String,
}

/// Response of `GET /v1/ontology/types`: objects and links split into
/// two lists.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct OntologyTypes {
    /// Registered object types.
    #[serde(default)]
    pub objects: Vec<OntologyTypeRow>,
    /// Registered link types.
    #[serde(default)]
    pub links: Vec<OntologyTypeRow>,
}

impl OntologyTypes {
    /// Total number of registered types (objects + links).
    pub fn total(&self) -> usize {
        self.objects.len() + self.links.len()
    }
}

/// One revision in an object type's lineage chain.
#[derive(Debug, Clone, Default, Deserialize, PartialEq, Eq)]
pub struct LineageNode {
    /// Schema version at this point in the chain.
    #[serde(default)]
    pub schema_version: i64,
    /// Content hash of the revision.
    #[serde(default)]
    pub content_hash: String,
    /// `true` when this revision was tagged as a breaking change.
    #[serde(default)]
    pub breaking: bool,
    /// Display title at this revision.
    #[serde(default)]
    pub title: String,
}

/// Response of `GET /v1/ontology/lineage/object/:name`. Nodes are
/// ordered oldest → newest by the daemon.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct OntologyLineage {
    /// Registry name of the object the chain belongs to.
    #[serde(default)]
    pub object_name: String,
    /// Chain of revisions, oldest first.
    #[serde(default)]
    pub nodes: Vec<LineageNode>,
}

/// One scenario branch overlay, as returned by
/// `GET /v1/ontology/branches`.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct OntologyBranchRow {
    /// Branch UUID.
    pub id: String,
    /// Stable branch name.
    pub name: String,
    /// Lifecycle status (`draft` / `review` / `merged` / `discarded`).
    #[serde(default)]
    pub status: String,
    /// Creation timestamp (RFC 3339).
    #[serde(default)]
    pub created_at: String,
    /// Last-update timestamp (RFC 3339).
    #[serde(default)]
    pub updated_at: String,
}

impl Client {
    /// Fetch every registered object and link type.
    pub async fn fetch_ontology_types(&self) -> Result<OntologyTypes> {
        self.get_json("/v1/ontology/types").await
    }

    /// Fetch the lineage chain of one object type by registry name.
    pub async fn fetch_ontology_lineage(&self, name: &str) -> Result<OntologyLineage> {
        let path = format!("/v1/ontology/lineage/object/{name}?format=json");
        self.get_json(&path).await
    }

    /// Fetch the list of scenario branch overlays.
    pub async fn fetch_ontology_branches(&self) -> Result<Vec<OntologyBranchRow>> {
        self.get_json("/v1/ontology/branches").await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn types_total_sums_objects_and_links() {
        let t = OntologyTypes {
            objects: vec![OntologyTypeRow::default(); 3],
            links: vec![OntologyTypeRow::default(); 2],
        };
        assert_eq!(t.total(), 5);
    }

    #[test]
    fn type_row_deserializes_with_defaults() {
        let row: OntologyTypeRow =
            serde_json::from_str(r#"{"name":"Person"}"#).expect("parse minimal row");
        assert_eq!(row.name, "Person");
        assert_eq!(row.schema_version, 0);
        assert!(row.kind.is_empty());
    }

    #[test]
    fn lineage_deserializes_node_chain() {
        let json = r#"{"object_name":"Person","nodes":[
            {"schema_version":1,"content_hash":"aa","breaking":false,"title":"v1"},
            {"schema_version":2,"content_hash":"bb","breaking":true,"title":"v2"}
        ]}"#;
        let l: OntologyLineage = serde_json::from_str(json).expect("parse lineage");
        assert_eq!(l.object_name, "Person");
        assert_eq!(l.nodes.len(), 2);
        assert!(l.nodes[1].breaking);
    }

    #[test]
    fn branch_row_deserializes() {
        let json = r#"{"id":"b1","name":"scenario","status":"draft",
            "created_at":"2026-06-01T00:00:00Z","updated_at":"2026-06-02T00:00:00Z"}"#;
        let b: OntologyBranchRow = serde_json::from_str(json).expect("parse branch");
        assert_eq!(b.name, "scenario");
        assert_eq!(b.status, "draft");
    }
}
