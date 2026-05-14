//! Community detection on the per-crate file graph.
//!
//! Builds a file-level subgraph for one crate (nodes = source files,
//! weighted edges = count of cross-file `uses` between items they
//! contain), then runs synchronous label propagation until labels
//! stabilise. Each resulting community is a candidate seam for a
//! crate-split conversation.
//!
//! Algorithm choice: label propagation is O(iterations × |E|), needs
//! no extra dependency, and is deterministic when the iteration order
//! is fixed (we sort files lexicographically). A handful of iterations
//! suffices on the workspace's small graphs (hundreds of files).
//!
//! SQL helpers and IO live in [`super::cluster_io`] so this file stays
//! under the 300-line cap.

use crate::cluster_io::{file_loc, file_uses_edges, files_for_crate, item_counts_per_file};
use crate::error::Result;
use crate::store::Store;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet, HashMap};

/// Default iteration cap for label propagation. Convergence is fast
/// on small file graphs; this only protects against pathological loops.
pub const DEFAULT_LP_ITERATIONS: u32 = 16;

/// Aggregate response for `cvg graph cluster`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClusterReport {
    /// Crate the cluster was computed against.
    pub crate_name: String,
    /// Optional target line count provided by the caller (informational).
    pub target_loc: Option<u64>,
    /// Number of distinct source files in the crate.
    pub total_files: usize,
    /// Number of `item` nodes in the crate.
    pub total_items: usize,
    /// Total source LOC, summed from `wc -l` on each file.
    pub total_loc: u64,
    /// Detected communities, sorted by descending `loc`.
    pub communities: Vec<Community>,
    /// True when at least one community exceeds `target_loc`.
    pub above_target: bool,
    /// Iterations consumed by label propagation (≤ `DEFAULT_LP_ITERATIONS`).
    pub iterations: u32,
}

/// One detected community within a crate.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Community {
    /// Label assigned to every member by label propagation. Stable
    /// across runs given the same inputs (lex-sorted seed file path).
    pub label: String,
    /// Member files, sorted lexicographically.
    pub files: Vec<String>,
    /// Number of `item` nodes inside this community.
    pub item_count: u32,
    /// Approximate LOC summed from member files.
    pub loc: u64,
    /// Edge weight that crosses out of this community.
    pub external_edges: u32,
    /// Edge weight strictly inside this community.
    pub internal_edges: u32,
}

/// Compute clusters for the named crate.
pub async fn cluster_for_crate(
    store: &Store,
    crate_name: &str,
    target_loc: Option<u64>,
) -> Result<ClusterReport> {
    let files = files_for_crate(store, crate_name).await?;
    let item_counts = item_counts_per_file(store, crate_name).await?;
    let edges = file_uses_edges(store, crate_name).await?;

    let total_files = files.len();
    let total_items: usize = item_counts.values().map(|c| *c as usize).sum();

    let mut loc_per_file: BTreeMap<String, u64> = BTreeMap::new();
    let mut total_loc: u64 = 0;
    for f in &files {
        let l = match file_loc(f) {
            Ok(l) => l,
            Err(err) => {
                // A file listed in graph_nodes may have been deleted
                // since the last parse — keep the cluster intact but
                // surface the skip so operators see disk drift.
                tracing::warn!(file = %f, error = %err, "cluster: file_loc failed, treating as 0 LOC");
                0
            }
        };
        loc_per_file.insert(f.clone(), l);
        total_loc += l;
    }

    let (labels, iterations) = label_propagation(&files, &edges, DEFAULT_LP_ITERATIONS);
    let communities = group_communities(&labels, &item_counts, &loc_per_file, &edges);

    let above_target = match target_loc {
        Some(t) => communities.iter().any(|c| c.loc > t),
        None => false,
    };

    Ok(ClusterReport {
        crate_name: crate_name.to_string(),
        target_loc,
        total_files,
        total_items,
        total_loc,
        communities,
        above_target,
        iterations,
    })
}

/// Synchronous label propagation. Each file initially carries its own
/// path as a label; on each iteration every file adopts the
/// neighbour-weight-majority label. Ties are broken by picking the
/// lexicographically smallest label (deterministic).
fn label_propagation(
    files: &[String],
    edges: &BTreeMap<(String, String), u32>,
    max_iter: u32,
) -> (HashMap<String, String>, u32) {
    let mut labels: HashMap<String, String> =
        files.iter().map(|f| (f.clone(), f.clone())).collect();

    let mut adj: HashMap<String, Vec<(String, u32)>> = HashMap::new();
    for ((a, b), w) in edges {
        adj.entry(a.clone()).or_default().push((b.clone(), *w));
        adj.entry(b.clone()).or_default().push((a.clone(), *w));
    }

    let mut iter = 0;
    while iter < max_iter {
        iter += 1;
        let mut changed = false;
        for f in files {
            let Some(neighbors) = adj.get(f) else {
                continue;
            };
            if neighbors.is_empty() {
                continue;
            }
            let mut tally: BTreeMap<String, u32> = BTreeMap::new();
            for (n, w) in neighbors {
                let lbl = labels.get(n).cloned().unwrap_or_else(|| n.clone());
                *tally.entry(lbl).or_default() += *w;
            }
            let best = tally
                .into_iter()
                .max_by(|a, b| a.1.cmp(&b.1).then_with(|| b.0.cmp(&a.0)))
                .map(|x| x.0);
            if let Some(l) = best {
                if labels.get(f) != Some(&l) {
                    labels.insert(f.clone(), l);
                    changed = true;
                }
            }
        }
        if !changed {
            break;
        }
    }
    (labels, iter)
}

fn group_communities(
    labels: &HashMap<String, String>,
    item_counts: &BTreeMap<String, u32>,
    loc_per_file: &BTreeMap<String, u64>,
    edges: &BTreeMap<(String, String), u32>,
) -> Vec<Community> {
    let mut buckets: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for (file, label) in labels {
        buckets.entry(label.clone()).or_default().push(file.clone());
    }

    let mut out: Vec<Community> = Vec::with_capacity(buckets.len());
    for (label, mut files) in buckets {
        files.sort();
        let item_count: u32 = files
            .iter()
            .map(|f| *item_counts.get(f).unwrap_or(&0))
            .sum();
        let loc: u64 = files
            .iter()
            .map(|f| *loc_per_file.get(f).unwrap_or(&0))
            .sum();
        let member_set: BTreeSet<&String> = files.iter().collect();

        let mut internal: u32 = 0;
        let mut external: u32 = 0;
        for ((a, b), w) in edges {
            let a_in = member_set.contains(a);
            let b_in = member_set.contains(b);
            match (a_in, b_in) {
                (true, true) => internal += *w,
                (true, false) | (false, true) => external += *w,
                _ => {}
            }
        }
        out.push(Community {
            label,
            files,
            item_count,
            loc,
            external_edges: external,
            internal_edges: internal,
        });
    }
    out.sort_by(|a, b| b.loc.cmp(&a.loc).then(a.label.cmp(&b.label)));
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn label_propagation_stabilises_singleton_when_no_edges() {
        let files = vec!["a.rs".to_string(), "b.rs".to_string()];
        let edges = BTreeMap::new();
        let (labels, iter) = label_propagation(&files, &edges, 4);
        assert_eq!(iter, 1);
        assert_eq!(labels.get("a.rs").unwrap(), "a.rs");
        assert_eq!(labels.get("b.rs").unwrap(), "b.rs");
    }

    #[test]
    fn label_propagation_groups_connected_files() {
        let files: Vec<String> = ["a.rs", "b.rs", "c.rs", "d.rs"]
            .iter()
            .map(|s| s.to_string())
            .collect();
        let mut edges = BTreeMap::new();
        edges.insert(("a.rs".into(), "b.rs".into()), 5);
        edges.insert(("c.rs".into(), "d.rs".into()), 5);
        let (labels, _) = label_propagation(&files, &edges, 8);
        assert_eq!(labels["a.rs"], labels["b.rs"]);
        assert_eq!(labels["c.rs"], labels["d.rs"]);
        assert_ne!(labels["a.rs"], labels["c.rs"]);
    }

    #[test]
    fn group_communities_counts_internal_external_edges() {
        let files: Vec<String> = ["a.rs", "b.rs", "c.rs"]
            .iter()
            .map(|s| s.to_string())
            .collect();
        let mut edges = BTreeMap::new();
        edges.insert(("a.rs".into(), "b.rs".into()), 4);
        edges.insert(("b.rs".into(), "c.rs".into()), 1);
        let (labels, _) = label_propagation(&files, &edges, 8);
        let mut item_counts = BTreeMap::new();
        for f in &files {
            item_counts.insert(f.clone(), 1);
        }
        let mut loc = BTreeMap::new();
        for f in &files {
            loc.insert(f.clone(), 100);
        }
        let comms = group_communities(&labels, &item_counts, &loc, &edges);
        let total_internal: u32 = comms.iter().map(|c| c.internal_edges).sum();
        let total_external: u32 = comms.iter().map(|c| c.external_edges).sum();
        assert_eq!(total_internal + total_external / 2, 5);
    }
}
