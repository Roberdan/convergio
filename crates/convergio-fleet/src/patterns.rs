//! Cluster detection over `similar_to` / `duplicates` edges (ADR-0038, F2-9).
//!
//! [`find_patterns`] groups cross-repo nodes into clusters via Union-Find
//! over `fleet_similar_edges`, then annotates each cluster with names and
//! kinds from `graph_nodes`. Clusters spanning fewer than `min_repos`
//! distinct repositories are dropped.

use crate::error::Result;
use crate::store::FleetStore;
use serde::{Deserialize, Serialize};
use sqlx::Row;
use std::collections::HashMap;

/// A single member of a pattern cluster.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClusterMember {
    /// Repository the node belongs to.
    pub repo: String,
    /// Human-readable name from `graph_nodes`, or raw node ID if not found.
    pub name: String,
    /// Node kind from `graph_nodes` (e.g. `module`, `item`).
    pub kind: String,
}

/// A detected cross-repo pattern cluster.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PatternCluster {
    /// Stable identifier derived from sorted member node IDs.
    pub cluster_id: String,
    /// All nodes that form this cluster, sorted by repo then name.
    pub members: Vec<ClusterMember>,
    /// Average cosine similarity across edges that link cluster members.
    pub confidence: f32,
    /// Candidate crate name for hoisting the shared pattern.
    pub hoist_target: String,
}

/// Detect cross-repo pattern clusters from similarity edges.
///
/// Clusters spanning fewer than `min_repos` distinct repos are excluded.
pub async fn find_patterns(store: &FleetStore, min_repos: usize) -> Result<Vec<PatternCluster>> {
    let edges = store.list_all_similar_edges().await?;
    if edges.is_empty() {
        return Ok(vec![]);
    }

    let (parent, edge_scores) = build_union_find(&edges);

    let mut buckets: HashMap<String, Vec<(String, String)>> = HashMap::new();
    for key in parent.keys() {
        let root = find_root(&parent, key);
        let (repo, node_id) = split_key(key);
        buckets.entry(root).or_default().push((repo, node_id));
    }

    let node_meta = load_node_meta(store, &parent).await?;

    let mut clusters = Vec::new();
    for (root, mut nodes) in buckets {
        let distinct_repos: std::collections::HashSet<_> =
            nodes.iter().map(|(r, _)| r.as_str()).collect();
        if distinct_repos.len() < min_repos {
            continue;
        }

        nodes.sort();
        let members: Vec<ClusterMember> = nodes
            .iter()
            .map(|(repo, nid)| {
                let (name, kind) = node_meta
                    .get(nid.as_str())
                    .cloned()
                    .unwrap_or_else(|| (nid.clone(), "unknown".to_owned()));
                ClusterMember {
                    repo: repo.clone(),
                    name,
                    kind,
                }
            })
            .collect();

        let confidence = cluster_confidence(&nodes, &edge_scores);
        let hoist_target = derive_hoist_target(&members);
        let cluster_id = stable_id(&root, &nodes);

        clusters.push(PatternCluster {
            cluster_id,
            members,
            confidence,
            hoist_target,
        });
    }

    clusters.sort_by(|a, b| {
        b.confidence
            .partial_cmp(&a.confidence)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then(a.cluster_id.cmp(&b.cluster_id))
    });
    Ok(clusters)
}

// ── Union-Find ────────────────────────────────────────────────────────────────

fn make_key(repo: &str, node_id: &str) -> String {
    format!("{repo}\x00{node_id}")
}

fn split_key(key: &str) -> (String, String) {
    let mut parts = key.splitn(2, '\x00');
    let repo = parts.next().unwrap_or("").to_owned();
    let nid = parts.next().unwrap_or("").to_owned();
    (repo, nid)
}

fn build_union_find(
    edges: &[crate::similar::SimilarEdge],
) -> (HashMap<String, String>, HashMap<(String, String), f32>) {
    let mut parent: HashMap<String, String> = HashMap::new();
    let mut edge_scores: HashMap<(String, String), f32> = HashMap::new();

    for e in edges {
        let ka = make_key(&e.repo_a, &e.node_id_a);
        let kb = make_key(&e.repo_b, &e.node_id_b);
        parent.entry(ka.clone()).or_insert_with(|| ka.clone());
        parent.entry(kb.clone()).or_insert_with(|| kb.clone());
        union(&mut parent, &ka, &kb);
        let pair = if ka < kb {
            (ka.clone(), kb.clone())
        } else {
            (kb.clone(), ka.clone())
        };
        edge_scores.insert(pair, e.score);
    }
    (parent, edge_scores)
}

fn find_root(parent: &HashMap<String, String>, key: &str) -> String {
    let mut current = key.to_owned();
    loop {
        let p = parent
            .get(&current)
            .cloned()
            .unwrap_or_else(|| current.clone());
        if p == current {
            return current;
        }
        current = p;
    }
}

fn union(parent: &mut HashMap<String, String>, a: &str, b: &str) {
    let ra = find_root(parent, a);
    let rb = find_root(parent, b);
    if ra != rb {
        parent.insert(rb, ra);
    }
}

// ── Confidence ────────────────────────────────────────────────────────────────

fn cluster_confidence(
    nodes: &[(String, String)],
    edge_scores: &HashMap<(String, String), f32>,
) -> f32 {
    let mut total = 0.0f32;
    let mut count = 0u32;
    for (i, (ra, na)) in nodes.iter().enumerate() {
        for (rb, nb) in &nodes[i + 1..] {
            let ka = make_key(ra, na);
            let kb = make_key(rb, nb);
            let pair = if ka < kb { (ka, kb) } else { (kb, ka) };
            if let Some(&score) = edge_scores.get(&pair) {
                total += score;
                count += 1;
            }
        }
    }
    if count == 0 {
        0.0
    } else {
        total / count as f32
    }
}

// ── Node metadata from graph_nodes ───────────────────────────────────────────

async fn load_node_meta(
    store: &FleetStore,
    parent: &HashMap<String, String>,
) -> Result<HashMap<String, (String, String)>> {
    let ids: Vec<String> = parent.keys().map(|k| split_key(k).1).collect();
    let mut out = HashMap::with_capacity(ids.len());
    for chunk in ids.chunks(500) {
        let placeholders = chunk.iter().map(|_| "?").collect::<Vec<_>>().join(",");
        let sql = format!("SELECT id, name, kind FROM graph_nodes WHERE id IN ({placeholders})");
        let mut q = sqlx::query(&sql);
        for id in chunk {
            q = q.bind(id);
        }
        let rows = match q.fetch_all(store.pool().inner()).await {
            Ok(r) => r,
            // graph not built yet — members fall back to node_id as name
            Err(sqlx::Error::Database(ref e)) if e.message().contains("no such table") => {
                continue;
            }
            Err(e) => return Err(crate::error::FleetError::Db(e)),
        };
        for row in rows {
            let id: String = row.get("id");
            let name: String = row.get("name");
            let kind: String = row.get("kind");
            out.insert(id, (name, kind));
        }
    }
    Ok(out)
}

// ── Hoist target heuristic ────────────────────────────────────────────────────

fn derive_hoist_target(members: &[ClusterMember]) -> String {
    let stopwords = [
        "src", "lib", "mod", "main", "test", "tests", "rs", "ts", "py",
    ];
    let mut freq: HashMap<&str, usize> = HashMap::new();
    for m in members {
        for word in m.name.split(|c: char| !c.is_alphanumeric()) {
            if word.len() >= 3 && !stopwords.contains(&word) {
                *freq.entry(word).or_default() += 1;
            }
        }
    }
    let common = freq.into_iter().max_by_key(|(_, c)| *c).map(|(w, _)| w);
    match common {
        Some(w) => format!("convergio-{w}-core"),
        None => members
            .first()
            .map(|m| format!("convergio-{}-core", m.repo))
            .unwrap_or_else(|| "convergio-shared-core".to_owned()),
    }
}

// ── Stable cluster id ─────────────────────────────────────────────────────────

fn stable_id(root: &str, nodes: &[(String, String)]) -> String {
    let mut h: u64 = 14695981039346656037;
    for b in root.bytes() {
        h ^= b as u64;
        h = h.wrapping_mul(1099511628211);
    }
    for (r, n) in nodes {
        for b in r.bytes().chain(b":".iter().copied()).chain(n.bytes()) {
            h ^= b as u64;
            h = h.wrapping_mul(1099511628211);
        }
    }
    format!("{h:016x}")
}

#[cfg(test)]
#[path = "patterns_tests.rs"]
mod tests;
