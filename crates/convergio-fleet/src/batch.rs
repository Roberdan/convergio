//! Cross-repo similarity batch job (ADR-0038, F2-8).
//!
//! [`run_similarity_batch`] reads all embeddings for a given model,
//! computes cosine similarity for every cross-repo pair, and upserts
//! edges into `fleet_similar_edges`.  It is idempotent: the table is
//! cleared before each run so stale edges do not accumulate.

use crate::error::Result;
use crate::similar::{DUPLICATES_THRESHOLD, SIMILAR_TO_THRESHOLD};
use crate::store::FleetStore;
use convergio_embed::EmbedStore;
use sqlx::Row;
use std::collections::HashMap;

/// Summary of one similarity batch run.
#[derive(Debug, Clone, Default)]
pub struct BatchReport {
    /// Cross-repo pairs evaluated.
    pub pairs_checked: u64,
    /// `similar_to` edges written.
    pub similar_to: u64,
    /// `duplicates` edges written.
    pub duplicates: u64,
}

/// Run the cross-repo similarity batch over all embeddings for `model`.
///
/// For each pair of embeddings from **distinct** repos:
/// - cosine ≥ [`SIMILAR_TO_THRESHOLD`] (0.85) → `similar_to` edge
/// - cosine ≥ [`DUPLICATES_THRESHOLD`] (0.95) **and** both nodes share
///   the same structural `kind` in `graph_nodes` → `duplicates` edge
///
/// Edges are persisted in `fleet_similar_edges` with
/// `weight = round(cosine × 1000)`.  The table is cleared first, so
/// re-runs are fully idempotent.
pub async fn run_similarity_batch(
    embed_store: &EmbedStore,
    fleet_store: &FleetStore,
    model: &str,
) -> Result<BatchReport> {
    fleet_store.clear_similar_edges().await?;

    let rows = embed_store.all_for_model(model).await?;
    if rows.is_empty() {
        return Ok(BatchReport::default());
    }

    let node_kinds = load_node_kinds(fleet_store, &rows).await?;

    let mut report = BatchReport::default();
    for (i, (repo_a, node_a, vec_a)) in rows.iter().enumerate() {
        for (repo_b, node_b, vec_b) in &rows[i + 1..] {
            if repo_a == repo_b {
                continue;
            }
            report.pairs_checked += 1;
            let score = cosine_sim(vec_a, vec_b);
            if score < SIMILAR_TO_THRESHOLD {
                continue;
            }
            let kind = classify(score, node_a, node_b, &node_kinds);
            fleet_store
                .upsert_similar_edge_classified(repo_a, node_a, repo_b, node_b, score, kind)
                .await?;
            if kind == "duplicates" {
                report.duplicates += 1;
            } else {
                report.similar_to += 1;
            }
        }
    }
    Ok(report)
}

/// Classify a cross-repo pair: `"duplicates"` when cosine ≥ threshold
/// and both nodes share the same structural kind; `"similar_to"` otherwise.
fn classify<'s>(
    score: f32,
    node_a: &str,
    node_b: &str,
    node_kinds: &HashMap<String, String>,
) -> &'s str {
    if score >= DUPLICATES_THRESHOLD {
        let ka = node_kinds.get(node_a);
        let kb = node_kinds.get(node_b);
        if ka.is_some() && ka == kb {
            return "duplicates";
        }
    }
    "similar_to"
}

/// Load `(id → kind)` from `graph_nodes` for every node in `rows`.
///
/// Queries the shared SQLite pool in chunks of 500 to stay well under
/// the SQLite variable limit.
async fn load_node_kinds(
    fleet_store: &FleetStore,
    rows: &[(String, String, Vec<f32>)],
) -> Result<HashMap<String, String>> {
    let ids: Vec<&str> = rows.iter().map(|(_, id, _)| id.as_str()).collect();
    let mut out = HashMap::with_capacity(ids.len());

    for chunk in ids.chunks(500) {
        let placeholders = chunk.iter().map(|_| "?").collect::<Vec<_>>().join(",");
        let sql = format!("SELECT id, kind FROM graph_nodes WHERE id IN ({placeholders})");
        let mut q = sqlx::query(&sql);
        for id in chunk {
            q = q.bind(*id);
        }
        let db_rows = q
            .fetch_all(fleet_store.pool().inner())
            .await
            .map_err(crate::error::FleetError::Db)?;
        for row in db_rows {
            let id: String = row.get("id");
            let kind: String = row.get("kind");
            out.insert(id, kind);
        }
    }
    Ok(out)
}

/// Cosine similarity between two vectors.  Returns 0.0 for zero vectors
/// or mismatched dimensions.
fn cosine_sim(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() || a.is_empty() {
        return 0.0;
    }
    let dot: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let na: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let nb: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
    if na == 0.0 || nb == 0.0 {
        return 0.0;
    }
    dot / (na * nb)
}

#[cfg(test)]
#[path = "batch_tests.rs"]
mod tests;
