//! Semantic drift between docs and code (ADR-0038, F3-6).
//!
//! [`snapshot_doc_alignment`] persists the current average cosine
//! between each ADR/Doc node and the code it `claims`/`mentions`.
//! [`find_doc_drift`] recomputes the alignment and surfaces nodes
//! whose alignment has dropped by at least the supplied threshold
//! since the snapshot. Advisory only.

use crate::doc_drift_store::{
    average_cosine, load_doc_links, load_doc_meta, load_embeddings, load_snapshots,
};
use crate::error::Result;
use crate::store::FleetStore;
use convergio_embed::EmbedStore;
use serde::{Deserialize, Serialize};

/// Default cosine-delta floor for surfacing drift (`snapshot − current`).
///
/// Anchored on the ADR-0038 D-6 design choice.
pub const DEFAULT_DOC_DRIFT_THRESHOLD: f32 = 0.2;

/// One drift candidate surfaced by [`find_doc_drift`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocDriftCandidate {
    /// Owning repo.
    pub repo: String,
    /// Stable graph node ID of the ADR / doc node.
    pub node_id: String,
    /// Human-readable name from `graph_nodes`.
    pub name: String,
    /// Node kind tag (`"adr"` or `"doc"`).
    pub kind: String,
    /// Source file path, if known.
    pub file_path: Option<String>,
    /// Avg cosine to linked code at snapshot time.
    pub snapshot_score: f32,
    /// Avg cosine to linked code now.
    pub current_score: f32,
    /// `snapshot − current` (positive = drift).
    pub delta: f32,
    /// `claims`/`mentions` targets considered.
    pub linked_count: u32,
    /// ISO-8601 snapshot timestamp.
    pub snapshot_at: String,
}

/// Summary of one snapshot run.
#[derive(Debug, Clone, Default, Serialize)]
pub struct SnapshotReport {
    /// ADR/Doc nodes considered.
    pub nodes_considered: u64,
    /// Nodes for which a row was written.
    pub nodes_snapshotted: u64,
}

/// Persist the current ADR↔code embedding alignment for every
/// ADR/Doc graph node that has an embedding and at least one
/// `claims` / `mentions` edge to a code node with an embedding.
///
/// Idempotent: re-running rewrites every row with `snapshot_at = now`.
pub async fn snapshot_doc_alignment(
    fleet: &FleetStore,
    embed: &EmbedStore,
    model: &str,
) -> Result<SnapshotReport> {
    let embed_by_key = load_embeddings(embed, model).await?;
    let docs = load_doc_meta(fleet).await?;
    let mut links = load_doc_links(fleet).await?;
    let now = chrono::Utc::now().to_rfc3339();
    let mut report = SnapshotReport {
        nodes_considered: docs.len() as u64,
        ..Default::default()
    };

    for (id, meta) in &docs {
        let Some(doc_vec) = embed_by_key.get(&(meta.repo.clone(), id.clone())) else {
            continue;
        };
        let targets = links.remove(id).unwrap_or_default();
        let Some((avg, count)) = average_cosine(doc_vec, &targets, &embed_by_key) else {
            continue;
        };
        sqlx::query(
            "INSERT OR REPLACE INTO fleet_doc_snapshots \
             (repo, node_id, model, snapshot_score, linked_count, snapshot_at) \
             VALUES (?, ?, ?, ?, ?, ?)",
        )
        .bind(&meta.repo)
        .bind(id)
        .bind(model)
        .bind(avg as f64)
        .bind(count as i64)
        .bind(&now)
        .execute(fleet.pool().inner())
        .await
        .map_err(crate::error::FleetError::Db)?;
        report.nodes_snapshotted += 1;
    }
    Ok(report)
}

/// Surface ADR/Doc nodes whose ADR↔code alignment has dropped by at
/// least `threshold` since the last [`snapshot_doc_alignment`] call.
///
/// `repo_filter` restricts the scan to a single repo. Rows with no
/// snapshot are silently skipped — the snapshot must exist first.
pub async fn find_doc_drift(
    fleet: &FleetStore,
    embed: &EmbedStore,
    model: &str,
    threshold: f32,
    repo_filter: Option<&str>,
) -> Result<Vec<DocDriftCandidate>> {
    let snapshots = load_snapshots(fleet, model, repo_filter).await?;
    if snapshots.is_empty() {
        return Ok(Vec::new());
    }
    let embed_by_key = load_embeddings(embed, model).await?;
    let doc_meta = load_doc_meta(fleet).await?;
    let mut links = load_doc_links(fleet).await?;

    let mut out = Vec::new();
    for snap in snapshots {
        let Some(meta) = doc_meta.get(&snap.node_id) else {
            continue;
        };
        if repo_filter.is_some_and(|r| r != meta.repo) {
            continue;
        }
        let Some(doc_vec) = embed_by_key.get(&(meta.repo.clone(), snap.node_id.clone())) else {
            continue;
        };
        let targets = links.remove(&snap.node_id).unwrap_or_default();
        let Some((current, count)) = average_cosine(doc_vec, &targets, &embed_by_key) else {
            continue;
        };
        let delta = snap.snapshot_score - current;
        if delta < threshold {
            continue;
        }
        out.push(DocDriftCandidate {
            repo: meta.repo.clone(),
            node_id: snap.node_id,
            name: meta.name.clone(),
            kind: meta.kind.clone(),
            file_path: meta.file_path.clone(),
            snapshot_score: snap.snapshot_score,
            current_score: current,
            delta,
            linked_count: count,
            snapshot_at: snap.snapshot_at,
        });
    }

    out.sort_by(|a, b| {
        b.delta
            .partial_cmp(&a.delta)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.repo.cmp(&b.repo))
            .then_with(|| a.node_id.cmp(&b.node_id))
    });
    Ok(out)
}

#[cfg(test)]
#[path = "doc_drift_tests.rs"]
mod tests;
