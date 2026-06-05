//! Diff between two schema versions of an `ObjectType` (ADR-0060,
//! W1 T9).
//!
//! The diff is a deterministic, serializable structure: rendering it
//! to Mermaid / Graphviz / JSON in `graph_render.rs` MUST produce
//! byte-identical output across reruns and across machines, same
//! posture as `actions.json` (ADR-0047) and the schema exporters.
//!
//! Diff semantics (W1):
//!
//! - A property is **present at version V** when there exists at
//!   least one row with `schema_version <= V`. We take the highest
//!   such revision (see `Store::list_object_properties_at`).
//! - **Added** = present at `to_version`, absent at `from_version`.
//! - **Removed** = present at `from_version`, absent at `to_version`.
//! - **Modified** = present at both with different `content_hash`.
//! - Object metadata changes (body / title / description) surface as
//!   `object_changed: true` plus the pair of object hashes.
//!
//! Out of scope for W1: link diffs and bitemporal branching — both
//! land with later ADRs.

use crate::error::{Error, Result};
use crate::model::{ObjectTypeRecord, PropertyTypeRecord};
use crate::store::Store;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Deterministic diff between two snapshots of the same
/// `ObjectType`. Field order matches the serialized output (sorted
/// names, sorted change buckets) so JSON / Mermaid / Graphviz
/// renderings stay byte-identical across reruns.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ObjectDiff {
    /// Registry name of the diffed object.
    pub object_name: String,
    /// Older schema_version end of the comparison.
    pub from_version: i64,
    /// Newer schema_version end of the comparison.
    pub to_version: i64,
    /// `true` when the object body / title / description itself
    /// changed between the two revisions (not just the properties).
    pub object_changed: bool,
    /// Content hash of the object at `from_version`. `None` when
    /// `from_version` does not exist in the registry.
    pub from_object_hash: Option<String>,
    /// Content hash of the object at `to_version`.
    pub to_object_hash: Option<String>,
    /// Property names present at `to_version` but absent at
    /// `from_version`. Sorted ASC for determinism.
    pub added: Vec<PropertyRef>,
    /// Property names removed between the two snapshots.
    pub removed: Vec<PropertyRef>,
    /// Property names present at both ends with a different
    /// `content_hash`.
    pub modified: Vec<PropertyChange>,
}

/// Stable identifier for a property at a given revision.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PropertyRef {
    /// Property registry name.
    pub name: String,
    /// `content_hash` of the property revision.
    pub content_hash: String,
}

/// One property whose `content_hash` changed between the diffed
/// snapshots.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PropertyChange {
    /// Property registry name.
    pub name: String,
    /// Hash at the `from_version` snapshot.
    pub from_hash: String,
    /// Hash at the `to_version` snapshot.
    pub to_hash: String,
}

impl ObjectDiff {
    /// `true` when nothing actually changed between the two
    /// snapshots — useful for callers that want to skip rendering an
    /// empty diagram.
    pub fn is_empty(&self) -> bool {
        !self.object_changed
            && self.added.is_empty()
            && self.removed.is_empty()
            && self.modified.is_empty()
    }
}

/// Compute the diff between two schema versions of an `ObjectType`.
///
/// Refuses to run when `from_version > to_version` — callers must
/// pass the older revision first so the rendered output reads
/// left-to-right in time.
pub async fn diff_object(
    store: &Store,
    name: &str,
    from_version: i64,
    to_version: i64,
) -> Result<ObjectDiff> {
    if from_version > to_version {
        return Err(Error::NotImplemented {
            feature: "diff_object with from_version > to_version",
        });
    }

    let from_obj: Option<ObjectTypeRecord> = store.get_object(name, from_version).await?;
    let to_obj: Option<ObjectTypeRecord> = store.get_object(name, to_version).await?;

    if from_obj.is_none() && to_obj.is_none() {
        return Err(Error::NotFound {
            kind: "object_type",
            name: name.to_string(),
        });
    }

    let from_props = store.list_object_properties_at(name, from_version).await?;
    let to_props = store.list_object_properties_at(name, to_version).await?;

    let from_map: BTreeMap<&str, &PropertyTypeRecord> =
        from_props.iter().map(|p| (p.name.as_str(), p)).collect();
    let to_map: BTreeMap<&str, &PropertyTypeRecord> =
        to_props.iter().map(|p| (p.name.as_str(), p)).collect();

    let mut added = Vec::new();
    let mut removed = Vec::new();
    let mut modified = Vec::new();

    for (n, p) in &to_map {
        match from_map.get(n) {
            None => added.push(PropertyRef {
                name: (*n).to_string(),
                content_hash: p.content_hash.clone(),
            }),
            Some(fp) if fp.content_hash != p.content_hash => modified.push(PropertyChange {
                name: (*n).to_string(),
                from_hash: fp.content_hash.clone(),
                to_hash: p.content_hash.clone(),
            }),
            Some(_) => {}
        }
    }
    for (n, p) in &from_map {
        if !to_map.contains_key(n) {
            removed.push(PropertyRef {
                name: (*n).to_string(),
                content_hash: p.content_hash.clone(),
            });
        }
    }

    let from_hash = from_obj.as_ref().map(|o| o.content_hash.clone());
    let to_hash = to_obj.as_ref().map(|o| o.content_hash.clone());
    let object_changed = match (&from_hash, &to_hash) {
        (Some(a), Some(b)) => a != b,
        _ => from_hash != to_hash,
    };

    Ok(ObjectDiff {
        object_name: name.to_string(),
        from_version,
        to_version,
        object_changed,
        from_object_hash: from_hash,
        to_object_hash: to_hash,
        added,
        removed,
        modified,
    })
}
