//! Fleet-scope recall benchmark helpers (ADR-0038 § 6, F2-12).
//!
//! Pure, I/O-free utilities for cross-repo fixture classification and
//! recall@K computation. JSON loading lives in each consumer (bench/test).

use serde::{Deserialize, Serialize};

/// Golden fixture for fleet-scope recall benchmarking.
///
/// `expected_files` may use either:
/// - repo-relative paths (`"crates/foo/src/lib.rs"`) — single-repo
/// - fleet-prefixed paths (`"convergio/crates/foo/src/lib.rs"`,
///   `"convergio-edu/src/bar.ts"`) — cross-repo
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct FleetFixture {
    /// Unique fixture identifier (e.g. `"T-edu-001-lesson-heartbeat"`).
    pub task_id: String,
    /// Name of the primary repo this task belongs to.
    pub repo: String,
    /// Short human-readable title of the task.
    pub title: String,
    /// Full task description used as the retrieval query.
    pub task_body: String,
    /// Files expected in the top-K retrieval result.
    /// May use `{repo}/{path}` format for cross-repo fixtures.
    pub expected_files: Vec<String>,
    /// Human rationale explaining which files matter and why.
    pub rationale: Option<String>,
    /// Who created this fixture.
    pub curator: Option<String>,
    /// ISO-8601 date the fixture was created.
    pub curated_at: Option<String>,
    /// Schema version — currently always `1`.
    #[serde(default = "default_schema_version")]
    pub schema_version: u32,
}

fn default_schema_version() -> u32 {
    1
}

impl FleetFixture {
    /// Returns `true` when `expected_files` reference ≥ 2 distinct repo prefixes.
    pub fn is_cross_repo(&self) -> bool {
        distinct_repo_prefixes(&self.expected_files).len() >= 2
    }

    /// Distinct `convergio*` repo names embedded in `expected_files` prefixes.
    pub fn referenced_repos(&self) -> Vec<String> {
        distinct_repo_prefixes(&self.expected_files)
    }
}

/// Extracts unique `convergio*` repo names from paths of the form `{repo}/{rest}`.
///
/// Paths without a matching prefix are silently ignored (treated as
/// repo-relative and belonging to the fixture's own repo).
pub fn distinct_repo_prefixes(paths: &[String]) -> Vec<String> {
    let mut seen = std::collections::BTreeSet::new();
    for p in paths {
        if let Some(r) = repo_prefix(p) {
            seen.insert(r);
        }
    }
    seen.into_iter().collect()
}

/// Returns the leading `convergio*` component from a `{repo}/{rest}` path,
/// or `None` for paths without such a prefix.
pub fn repo_prefix(path: &str) -> Option<String> {
    let (head, _) = path.split_once('/')?;
    if head.starts_with("convergio") {
        Some(head.to_string())
    } else {
        None
    }
}

/// Strips the `{repo}/` prefix from a path, returning the repo-relative remainder.
/// Returns `path` unchanged if no `convergio*` prefix is found.
pub fn strip_repo_prefix(path: &str) -> &str {
    match path.split_once('/') {
        Some((head, rest)) if head.starts_with("convergio") => rest,
        _ => path,
    }
}

/// Recall@K: fraction of `expected` paths found in the top `k` of `retrieved`.
///
/// Returns 0.0 when `expected` is empty.
pub fn recall_at_k(retrieved: &[String], expected: &[String], k: usize) -> f64 {
    if expected.is_empty() {
        return 0.0;
    }
    let top_k: std::collections::HashSet<&String> = retrieved.iter().take(k).collect();
    let hits = expected.iter().filter(|e| top_k.contains(*e)).count();
    hits as f64 / expected.len() as f64
}

/// Aggregate counts for a fleet-scope recall run across one or more repos.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct FleetRecallReport {
    /// Total fixture count across all repo directories.
    pub total: usize,
    /// Count of fixtures whose `expected_files` reference ≥ 2 distinct repos.
    pub cross_repo: usize,
    /// Count of fixtures whose `expected_files` reference ≤ 1 repo (or are unprefixed).
    pub single_repo: usize,
}

impl FleetRecallReport {
    /// Classify `fixtures` into cross-repo and single-repo buckets.
    pub fn from_fixtures(fixtures: &[FleetFixture]) -> Self {
        let cross = fixtures.iter().filter(|f| f.is_cross_repo()).count();
        FleetRecallReport {
            total: fixtures.len(),
            cross_repo: cross,
            single_repo: fixtures.len().saturating_sub(cross),
        }
    }
}

#[cfg(test)]
#[path = "recall_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "recall_report_tests.rs"]
mod report_tests;
