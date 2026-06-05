//! Lineage of an `ObjectType`: the ordered chain of schema_versions
//! that have been registered (ADR-0060, W1 T9).
//!
//! Determinism contract: `lineage_object` always returns versions in
//! ascending `schema_version` order. Two calls against the same store
//! return identical structures, so rendering them via
//! `graph_render::render_lineage_*` produces byte-identical Mermaid /
//! Graphviz output.

use crate::error::{Error, Result};
use crate::store::Store;
use serde::{Deserialize, Serialize};

/// One node in the lineage chain.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LineageNode {
    /// Schema version at this point in the chain.
    pub schema_version: i64,
    /// `content_hash` of the revision (full hash).
    pub content_hash: String,
    /// `true` when this revision was tagged as a breaking change.
    pub breaking: bool,
    /// Display title at this revision.
    pub title: String,
}

/// Full lineage of an `ObjectType`. `nodes` is sorted by
/// `schema_version` ASC.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Lineage {
    /// Registry name of the object.
    pub object_name: String,
    /// Chain of revisions oldest → newest.
    pub nodes: Vec<LineageNode>,
}

impl Lineage {
    /// `true` when no revisions exist (callers can short-circuit
    /// rendering an empty diagram).
    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }
}

/// Build the lineage chain for `name`. Returns `NotFound` when the
/// object has never been registered.
pub async fn lineage_object(store: &Store, name: &str) -> Result<Lineage> {
    let versions = store.list_object_versions(name).await?;
    if versions.is_empty() {
        return Err(Error::NotFound {
            kind: "object_type",
            name: name.to_string(),
        });
    }
    let nodes = versions
        .into_iter()
        .map(|o| LineageNode {
            schema_version: o.schema_version,
            content_hash: o.content_hash,
            breaking: o.breaking,
            title: o.title,
        })
        .collect();
    Ok(Lineage {
        object_name: name.to_string(),
        nodes,
    })
}
