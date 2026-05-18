//! Semantic dead-code detection (ADR-0038, F3-5).
//!
//! [`find_rot`] ranks `item`-kind graph nodes with no inbound
//! `uses`/`re_exports`/`mentions` edges and a best cross-node cosine
//! below the supplied threshold. Confidence is role-weighted
//! (engine, library, downstream, sandbox in descending order).
//! Advisory only — humans decide what to remove.

use crate::error::Result;
use crate::store::FleetStore;
use convergio_embed::EmbedStore;
use serde::{Deserialize, Serialize};
use sqlx::Row;
use std::collections::HashMap;

/// Default cosine ceiling for the "semantically isolated" filter.
///
/// Anchored on the ADR-0038 CLI surface (`cvg fleet rot
/// [--threshold 0.3]`). Best-similar scores **below** this threshold
/// keep a node as a rot candidate.
pub const DEFAULT_ROT_THRESHOLD: f32 = 0.3;

fn role_weight(role: &str) -> f32 {
    match role {
        "engine" => 1.0,
        "library" => 0.85,
        "downstream" => 0.65,
        "sandbox" => 0.4,
        _ => 0.6,
    }
}

/// One advisory dead-code candidate.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RotCandidate {
    /// Repository owning the node.
    pub repo: String,
    /// Stable graph node ID.
    pub node_id: String,
    /// Human-readable name from `graph_nodes`.
    pub name: String,
    /// Node kind tag (always `"item"` in this listing).
    pub kind: String,
    /// Item flavour (struct/enum/fn/trait/impl/const/type/macro), if known.
    pub item_kind: Option<String>,
    /// Owning crate name.
    pub crate_name: String,
    /// Source file path, if known.
    pub file_path: Option<String>,
    /// Role string of the owning repo.
    pub role: String,
    /// Number of inbound `uses` / `re_exports` / `mentions` edges.
    pub inbound_uses: u32,
    /// Maximum cosine to any **other** embedding in the same model.
    pub best_similar_score: f32,
    /// Confidence in `[0, 1]` — higher means more likely truly dead.
    pub confidence: f32,
    /// Why this row landed in the result set (human-readable evidence).
    pub reasons: Vec<String>,
}

/// Rank semantic dead-code candidates across the fleet.
///
/// `threshold` is a cosine **ceiling**: nodes with a best-similar
/// score below `threshold` are surfaced. `repo_filter` restricts the
/// scan to a single repo. `explain_node` short-circuits the filter
/// and returns the requested node regardless of whether it would
/// normally qualify, with a richer [`RotCandidate::reasons`] trail.
pub async fn find_rot(
    fleet: &FleetStore,
    embed: &EmbedStore,
    model: &str,
    threshold: f32,
    repo_filter: Option<&str>,
    explain_node: Option<&str>,
) -> Result<Vec<RotCandidate>> {
    let nodes = load_item_nodes(fleet, repo_filter).await?;
    if nodes.is_empty() {
        return Ok(Vec::new());
    }
    let inbound = load_inbound_counts(fleet).await?;
    let roles = load_roles(fleet).await?;
    let best = compute_best_similar(embed, model).await?;

    let mut out = Vec::new();
    for n in nodes {
        let inbound_uses = inbound.get(&n.id).copied().unwrap_or(0);
        let is_explained = explain_node.is_some_and(|id| id == n.id);
        let role = roles
            .get(&n.repo)
            .cloned()
            .unwrap_or_else(|| "downstream".to_owned());

        let score = best.get(&(n.repo.clone(), n.id.clone())).copied();
        let qualifies = score.is_some_and(|s| inbound_uses == 0 && s < threshold);
        // Unscored nodes (pre-build or EmbedPolicy-excluded) are
        // silently dropped from the default listing; `--explain`
        // still surfaces them with a clear reason trail.
        if qualifies || is_explained {
            out.push(make_candidate(
                n,
                role,
                inbound_uses,
                score,
                threshold,
                is_explained,
            ));
        }
    }

    out.sort_by(|a, b| {
        b.confidence
            .partial_cmp(&a.confidence)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.repo.cmp(&b.repo))
            .then_with(|| a.node_id.cmp(&b.node_id))
    });
    Ok(out)
}

struct ItemNode {
    id: String,
    kind: String,
    name: String,
    file_path: Option<String>,
    crate_name: String,
    item_kind: Option<String>,
    repo: String,
}

async fn load_item_nodes(fleet: &FleetStore, repo: Option<&str>) -> Result<Vec<ItemNode>> {
    let sql = match repo {
        Some(_) => {
            "SELECT id, kind, name, file_path, crate_name, item_kind, repo \
             FROM graph_nodes \
             WHERE kind = 'item' AND repo = ?"
        }
        None => {
            "SELECT id, kind, name, file_path, crate_name, item_kind, repo \
             FROM graph_nodes \
             WHERE kind = 'item'"
        }
    };
    let mut q = sqlx::query(sql);
    if let Some(r) = repo {
        q = q.bind(r);
    }
    let rows = match q.fetch_all(fleet.pool().inner()).await {
        Ok(rows) => rows,
        // graph store has not migrated yet; rot is advisory so we
        // return an empty result instead of erroring out.
        Err(sqlx::Error::Database(ref e)) if e.message().contains("no such table") => {
            return Ok(Vec::new())
        }
        Err(e) => return Err(crate::error::FleetError::Db(e)),
    };
    Ok(rows
        .into_iter()
        .map(|row| ItemNode {
            id: row.get("id"),
            kind: row.get("kind"),
            name: row.get("name"),
            file_path: row.get("file_path"),
            crate_name: row.get("crate_name"),
            item_kind: row.get("item_kind"),
            repo: row.get("repo"),
        })
        .collect())
}

async fn load_inbound_counts(fleet: &FleetStore) -> Result<HashMap<String, u32>> {
    let rows = match sqlx::query(
        "SELECT dst, COUNT(*) AS c \
         FROM graph_edges \
         WHERE kind IN ('uses', 're_exports', 'mentions') \
         GROUP BY dst",
    )
    .fetch_all(fleet.pool().inner())
    .await
    {
        Ok(r) => r,
        Err(sqlx::Error::Database(ref e)) if e.message().contains("no such table") => {
            return Ok(HashMap::new())
        }
        Err(e) => return Err(crate::error::FleetError::Db(e)),
    };
    let mut out = HashMap::with_capacity(rows.len());
    for row in rows {
        let dst: String = row.get("dst");
        let c: i64 = row.get("c");
        out.insert(dst, c.max(0) as u32);
    }
    Ok(out)
}

async fn load_roles(fleet: &FleetStore) -> Result<HashMap<String, String>> {
    let repos = fleet.list_repos().await?;
    Ok(repos.into_iter().map(|r| (r.name, r.role)).collect())
}

async fn compute_best_similar(
    embed: &EmbedStore,
    model: &str,
) -> Result<HashMap<(String, String), f32>> {
    let rows = embed.all_for_model(model).await?;
    // Key by `(repo, node_id)` so two repos that happen to share a
    // node_id (path-like IDs, identical hash collisions) keep their
    // own scores. Even though current node IDs encode repo into the
    // hash, the storage contract is (repo, node_id, model) — mirror
    // that here to stay collision-proof.
    let mut out: HashMap<(String, String), f32> = HashMap::with_capacity(rows.len());
    let norms: Vec<f32> = rows.iter().map(|(_, _, v)| vec_norm(v)).collect();
    for (i, (repo_i, id_i, vi)) in rows.iter().enumerate() {
        if norms[i] == 0.0 {
            continue;
        }
        let mut best = 0.0f32;
        for (j, (_, _, vj)) in rows.iter().enumerate() {
            if i == j || norms[j] == 0.0 || vi.len() != vj.len() {
                continue;
            }
            let s = cosine_with_norm(vi, vj, norms[i], norms[j]);
            if s > best {
                best = s;
            }
        }
        out.insert((repo_i.clone(), id_i.clone()), best);
    }
    Ok(out)
}

fn vec_norm(v: &[f32]) -> f32 {
    v.iter().map(|x| x * x).sum::<f32>().sqrt()
}

fn cosine_with_norm(a: &[f32], b: &[f32], na: f32, nb: f32) -> f32 {
    let denom = na * nb;
    if denom == 0.0 {
        return 0.0;
    }
    let dot: f32 = a.iter().zip(b).map(|(x, y)| x * y).sum();
    (dot / denom).clamp(-1.0, 1.0)
}

fn make_candidate(
    n: ItemNode,
    role: String,
    inbound_uses: u32,
    score: Option<f32>,
    threshold: f32,
    explained: bool,
) -> RotCandidate {
    let weight = role_weight(&role);
    let (best_similar_score, confidence, reasons) = match score {
        Some(s) => {
            let raw = (1.0 - s).clamp(0.0, 1.0) * weight;
            let confidence = if inbound_uses == 0 { raw } else { 0.0 };
            let mut r = vec![
                format!("inbound uses/re_exports/mentions = {inbound_uses}"),
                format!("best cross-node cosine = {s:.3}"),
                format!("threshold = {threshold:.3} (below = semantically isolated)"),
                format!("role = {role} (weight = {weight:.2})"),
            ];
            if explained && (inbound_uses > 0 || s >= threshold) {
                r.push(
                    "explain: returned because of --explain; would not normally qualify".to_owned(),
                );
            }
            (s, confidence, r)
        }
        None => (
            0.0,
            0.0,
            vec![
                format!("inbound uses/re_exports/mentions = {inbound_uses}"),
                "no embedding stored for this node (skipped from default listing)".to_owned(),
            ],
        ),
    };
    RotCandidate {
        repo: n.repo,
        node_id: n.id,
        name: n.name,
        kind: n.kind,
        item_kind: n.item_kind,
        crate_name: n.crate_name,
        file_path: n.file_path,
        role,
        inbound_uses,
        best_similar_score,
        confidence,
        reasons,
    }
}

#[cfg(test)]
#[path = "rot_tests.rs"]
mod tests;
