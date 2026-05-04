//! Cross-repo duplicate pair query (ADR-0038, F2-10).
//!
//! [`find_duplicates`] queries `fleet_similar_edges` for edges classified
//! as `duplicates` with `score >= cosine_threshold`, enriches each pair with
//! node metadata from `graph_nodes`, and optionally adds a 1–3 line semantic
//! diff preview.

use crate::error::Result;
use crate::store::FleetStore;
use serde::{Deserialize, Serialize};
use sqlx::Row;
use std::collections::HashMap;

/// One cross-repo duplicate pair enriched with graph-node metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DuplicatePair {
    /// Repo owning the first node.
    pub repo_a: String,
    /// Node identifier in `repo_a`.
    pub node_id_a: String,
    /// Human-readable name from `graph_nodes`.
    pub name_a: String,
    /// Node kind (module, item, …).
    pub kind_a: String,
    /// Source file, if known.
    pub file_a: Option<String>,
    /// Repo owning the second node.
    pub repo_b: String,
    /// Node identifier in `repo_b`.
    pub node_id_b: String,
    /// Human-readable name from `graph_nodes`.
    pub name_b: String,
    /// Node kind (module, item, …).
    pub kind_b: String,
    /// Source file, if known.
    pub file_b: Option<String>,
    /// Cosine similarity in [0.0, 1.0].
    pub score: f32,
    /// 1–3 line semantic delta (empty unless `diff_preview` requested).
    pub diff_preview: Vec<String>,
}

/// Find cross-repo duplicate pairs.
///
/// Returns all `duplicates` edges with `score >= cosine_threshold`, sorted
/// by descending score.  Pass `repo_pair` to restrict results to one
/// (undirected) repo pair.  Set `diff_preview = true` to populate
/// [`DuplicatePair::diff_preview`] with up to 3 comparison lines.
pub async fn find_duplicates(
    store: &FleetStore,
    cosine_threshold: f32,
    repo_pair: Option<(&str, &str)>,
    diff_preview: bool,
) -> Result<Vec<DuplicatePair>> {
    let edge_rows = fetch_edges(store, cosine_threshold, repo_pair).await?;
    if edge_rows.is_empty() {
        return Ok(Vec::new());
    }

    let node_ids: Vec<&str> = edge_rows
        .iter()
        .flat_map(|(_, na, _, nb, _)| [na.as_str(), nb.as_str()])
        .collect();
    let meta = load_node_meta(store, &node_ids).await?;

    let pairs = edge_rows
        .into_iter()
        .map(|(repo_a, node_id_a, repo_b, node_id_b, score)| {
            let (name_a, kind_a, file_a) = meta
                .get(&node_id_a)
                .cloned()
                .unwrap_or_else(|| (node_id_a.clone(), "unknown".into(), None));
            let (name_b, kind_b, file_b) = meta
                .get(&node_id_b)
                .cloned()
                .unwrap_or_else(|| (node_id_b.clone(), "unknown".into(), None));
            let diff_lines = if diff_preview {
                build_preview(
                    &name_a,
                    &kind_a,
                    file_a.as_deref(),
                    &name_b,
                    &kind_b,
                    file_b.as_deref(),
                )
            } else {
                Vec::new()
            };
            DuplicatePair {
                repo_a,
                node_id_a,
                name_a,
                kind_a,
                file_a,
                repo_b,
                node_id_b,
                name_b,
                kind_b,
                file_b,
                score,
                diff_preview: diff_lines,
            }
        })
        .collect();
    Ok(pairs)
}

type EdgeRow = (String, String, String, String, f32);

async fn fetch_edges(
    store: &FleetStore,
    threshold: f32,
    repo_pair: Option<(&str, &str)>,
) -> Result<Vec<EdgeRow>> {
    let rows = if let Some((ra, rb)) = repo_pair {
        sqlx::query(
            "SELECT repo_a, node_id_a, repo_b, node_id_b, score \
             FROM fleet_similar_edges \
             WHERE kind = 'duplicates' AND score >= ? \
             AND ((repo_a = ? AND repo_b = ?) OR (repo_a = ? AND repo_b = ?)) \
             ORDER BY score DESC",
        )
        .bind(threshold)
        .bind(ra)
        .bind(rb)
        .bind(rb)
        .bind(ra)
        .fetch_all(store.pool().inner())
        .await?
    } else {
        sqlx::query(
            "SELECT repo_a, node_id_a, repo_b, node_id_b, score \
             FROM fleet_similar_edges \
             WHERE kind = 'duplicates' AND score >= ? \
             ORDER BY score DESC",
        )
        .bind(threshold)
        .fetch_all(store.pool().inner())
        .await?
    };

    Ok(rows
        .iter()
        .map(|r| {
            (
                r.get::<String, _>("repo_a"),
                r.get::<String, _>("node_id_a"),
                r.get::<String, _>("repo_b"),
                r.get::<String, _>("node_id_b"),
                r.get::<f64, _>("score") as f32,
            )
        })
        .collect())
}

/// Batch-load `(name, kind, file_path)` from `graph_nodes` for a slice of IDs.
async fn load_node_meta(
    store: &FleetStore,
    ids: &[&str],
) -> Result<HashMap<String, (String, String, Option<String>)>> {
    let unique: Vec<&str> = {
        let mut v = ids.to_vec();
        v.sort_unstable();
        v.dedup();
        v
    };
    let mut out = HashMap::with_capacity(unique.len());
    for chunk in unique.chunks(500) {
        let placeholders = chunk.iter().map(|_| "?").collect::<Vec<_>>().join(",");
        let sql = format!(
            "SELECT id, name, kind, file_path FROM graph_nodes WHERE id IN ({placeholders})"
        );
        let mut q = sqlx::query(&sql);
        for id in chunk {
            q = q.bind(*id);
        }
        let db_rows = q
            .fetch_all(store.pool().inner())
            .await
            .map_err(crate::error::FleetError::Db)?;
        for row in db_rows {
            let id: String = row.get("id");
            let name: String = row.get("name");
            let kind: String = row.get("kind");
            let file_path: Option<String> = row.get("file_path");
            out.insert(id, (name, kind, file_path));
        }
    }
    Ok(out)
}

/// Produce up to 3 lines of semantic diff between two duplicate nodes.
fn build_preview(
    name_a: &str,
    kind_a: &str,
    file_a: Option<&str>,
    name_b: &str,
    kind_b: &str,
    file_b: Option<&str>,
) -> Vec<String> {
    let mut lines = Vec::new();
    if name_a != name_b {
        lines.push(format!("name: {name_a} ↔ {name_b}"));
    }
    if kind_a != kind_b {
        lines.push(format!("kind: {kind_a} ↔ {kind_b}"));
    }
    if let (Some(fa), Some(fb)) = (file_a, file_b) {
        if fa != fb {
            lines.push(format!("path: {fa} ↔ {fb}"));
        }
    }
    if lines.is_empty() {
        lines.push(format!("name: {name_a} (identical across repos)"));
    }
    lines.truncate(3);
    lines
}

#[cfg(test)]
#[path = "duplicates_tests.rs"]
mod tests;
