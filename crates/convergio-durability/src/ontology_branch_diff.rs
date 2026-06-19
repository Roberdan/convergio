//! Ontology branch diff + merge-as-plan generator (ADR-0056, W4).
//!
//! A scenario branch is a key-value overlay on the mainline ontology
//! store: each overlay row is either a `set` (override the base value)
//! or a `delete` (tombstone that shadows a base key). This module
//! compares a branch overlay against the mainline base and produces:
//!
//! - [`BranchDiff`] — the per-key classification (Added / Modified /
//!   Removed), with both the base and branch values attached.
//! - [`MergePlan`] — an ordered, explicit list of [`MergeOp`]s that,
//!   applied to the base, would make the base equal the branch overlay.
//!   This *generates* the plan; it never mutates the base.
//!
//! Both outputs are deterministically ordered by key (ascending) so
//! callers get stable, diffable results.

use crate::error::{DurabilityError, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// How a key changed in a branch overlay relative to the base.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BranchChange {
    /// Key is set in the branch but absent from the base.
    Added,
    /// Key exists in both, but the branch value differs from the base.
    Modified,
    /// Branch tombstones (deletes) a key that exists in the base.
    Removed,
}

/// One classified key difference between a branch and the base.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BranchDiffEntry {
    /// Logical entry key.
    pub key: String,
    /// Classification of the change.
    pub change: BranchChange,
    /// Base (mainline) value, or `None` when the key is absent in base.
    pub base: Option<Value>,
    /// Branch (overlay) value, or `None` when the branch removes the key.
    pub branch: Option<Value>,
}

/// A deterministic, key-sorted diff of a branch overlay against base.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BranchDiff {
    /// Branch the diff was computed for.
    pub branch_id: String,
    /// Per-key differences, sorted by `key` ascending.
    pub entries: Vec<BranchDiffEntry>,
}

/// A single merge operation against the base store.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MergeOpKind {
    /// Write (insert or overwrite) the value at `key` into the base.
    Set,
    /// Remove `key` from the base.
    Unset,
}

/// One explicit operation in a [`MergePlan`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MergeOp {
    /// Key the operation targets.
    pub key: String,
    /// Whether to set or unset the key.
    pub op: MergeOpKind,
    /// Value to write for [`MergeOpKind::Set`]; `None` for `Unset`.
    pub value: Option<Value>,
}

/// An ordered list of operations that, applied to the base, makes the
/// base equal the branch overlay. Generated, not applied.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MergePlan {
    /// Branch the plan was generated for.
    pub branch_id: String,
    /// Operations to apply to the base, sorted by `key` ascending.
    pub ops: Vec<MergeOp>,
}

impl BranchDiff {
    /// Turn this diff into an ordered [`MergePlan`]. `Added`/`Modified`
    /// become [`MergeOpKind::Set`] of the branch value; `Removed`
    /// becomes [`MergeOpKind::Unset`]. Ordering follows the diff.
    pub fn to_merge_plan(&self) -> MergePlan {
        let ops = self
            .entries
            .iter()
            .map(|entry| match entry.change {
                BranchChange::Added | BranchChange::Modified => MergeOp {
                    key: entry.key.clone(),
                    op: MergeOpKind::Set,
                    value: entry.branch.clone(),
                },
                BranchChange::Removed => MergeOp {
                    key: entry.key.clone(),
                    op: MergeOpKind::Unset,
                    value: None,
                },
            })
            .collect();
        MergePlan {
            branch_id: self.branch_id.clone(),
            ops,
        }
    }
}

/// A raw overlay row as stored in `ontology_branch_entries`.
struct OverlayRow {
    key: String,
    op_kind: String,
    value: Option<String>,
}

impl crate::Durability {
    /// Diff a branch overlay against the mainline base (ADR-0056).
    ///
    /// Returns one [`BranchDiffEntry`] per key the branch actually
    /// changes, sorted by key. A `set` whose value equals the base is
    /// omitted (no change); a `delete` of a key absent in base is also
    /// omitted (tombstone over nothing). A non-existent `branch_id`
    /// yields [`DurabilityError::NotFound`].
    pub async fn diff_ontology_branch(&self, branch_id: &str) -> Result<BranchDiff> {
        // Validate the branch exists (NotFound otherwise).
        self.ontology().get_branch(branch_id).await?;

        let overlay = sqlx::query_as::<_, (String, String, Option<String>)>(
            "SELECT key, op_kind, value FROM ontology_branch_entries WHERE branch_id = ? ORDER BY key ASC",
        )
        .bind(branch_id)
        .fetch_all(self.pool().inner())
        .await?
        .into_iter()
        .map(|(key, op_kind, value)| OverlayRow {
            key,
            op_kind,
            value,
        });

        let mut entries = Vec::new();
        for row in overlay {
            let base = self.base_entry_value(&row.key).await?;
            match row.op_kind.as_str() {
                "set" => {
                    let raw = row
                        .value
                        .ok_or_else(|| DurabilityError::InvalidOntologyEntry {
                            reason: "overlay op_kind=set requires value".into(),
                        })?;
                    let branch_value = serde_json::from_str::<Value>(&raw)?;
                    match base {
                        None => entries.push(BranchDiffEntry {
                            key: row.key,
                            change: BranchChange::Added,
                            base: None,
                            branch: Some(branch_value),
                        }),
                        Some(base_value) if base_value != branch_value => {
                            entries.push(BranchDiffEntry {
                                key: row.key,
                                change: BranchChange::Modified,
                                base: Some(base_value),
                                branch: Some(branch_value),
                            })
                        }
                        Some(_) => {} // identical override: not a change
                    }
                }
                "delete" => {
                    if let Some(base_value) = base {
                        entries.push(BranchDiffEntry {
                            key: row.key,
                            change: BranchChange::Removed,
                            base: Some(base_value),
                            branch: None,
                        });
                    }
                    // delete of a key absent in base is a no-op
                }
                other => {
                    return Err(DurabilityError::InvalidOntologyEntry {
                        reason: format!("invalid overlay op_kind: {other}"),
                    })
                }
            }
        }

        Ok(BranchDiff {
            branch_id: branch_id.to_string(),
            entries,
        })
    }

    /// Generate the [`MergePlan`] for a branch (ADR-0056).
    ///
    /// This is the "merge-as-plan generator": it diffs the branch and
    /// converts the diff into an ordered list of [`MergeOp`]s that would
    /// make the base equal the branch overlay. It does NOT mutate the
    /// base. A non-existent `branch_id` yields
    /// [`DurabilityError::NotFound`].
    pub async fn branch_merge_as_plan(&self, branch_id: &str) -> Result<MergePlan> {
        Ok(self.diff_ontology_branch(branch_id).await?.to_merge_plan())
    }

    /// Read the parsed base (mainline) value for `key`, or `None` when
    /// the key is absent from `ontology_entries`.
    async fn base_entry_value(&self, key: &str) -> Result<Option<Value>> {
        let row = sqlx::query_as::<_, (String,)>(
            "SELECT value FROM ontology_entries WHERE key = ? LIMIT 1",
        )
        .bind(key)
        .fetch_optional(self.pool().inner())
        .await?;
        match row {
            Some((raw,)) => Ok(Some(serde_json::from_str::<Value>(&raw)?)),
            None => Ok(None),
        }
    }
}
