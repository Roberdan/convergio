//! `cvg graph drift` — compares ADR-claimed crates against the crates
//! actually touched in a git diff (ADR-0014 § Drift semantics).
//!
//! v0 surfaces three sets:
//!   - **declared**: union of `touches_crates` from selected ADRs
//!     (default: all ADRs whose status is `proposed` or `accepted`,
//!     scoped via the `claims` edges in the graph store).
//!   - **actual**: crates that own any file in the supplied diff.
//!   - **drift**: `actual ∖ declared` — crates touched but never
//!     declared in any ADR claim. Real signal for a code-vs-doc gap.
//!   - **ghosts**: `declared ∖ actual` — crates the ADRs promised
//!     to touch but the diff never did. Lower-severity (a future
//!     PR may follow up), but worth surfacing.
//!
//! Advisory at v0 (CI surfaces, never blocks); promote to gate when
//! we have data on false positives across a few PRs.

use crate::drift_git::git_changed_files;
use crate::error::Result;
use crate::store::Store;
use serde::{Deserialize, Serialize};
use sqlx::Row;
use std::collections::BTreeSet;
use std::path::Path;

/// What `cvg graph drift` returns.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DriftReport {
    /// Git ref the diff is computed against (e.g. `origin/main`).
    pub since: String,
    /// Optional ADR scope (None = all proposed/accepted ADRs).
    pub adr_scope: Option<String>,
    /// Files actually changed in the diff.
    pub files_changed: Vec<String>,
    /// Crates touched (resolved from `files_changed` via the graph).
    pub actual_crates: Vec<String>,
    /// Crates declared by `claims` edges in the selected ADR scope.
    pub declared_crates: Vec<String>,
    /// `actual ∖ declared` — touched but never claimed.
    pub drift: Vec<String>,
    /// `declared ∖ actual` — claimed but never touched.
    pub ghosts: Vec<String>,
}

/// Compute drift for a workspace at `repo_root`, comparing the graph
/// state against `git diff --name-only <since>...HEAD`.
pub async fn drift_since(
    store: &Store,
    repo_root: &Path,
    since: &str,
    adr_scope: Option<&str>,
) -> Result<DriftReport> {
    let files = git_changed_files(repo_root, since)?;
    let actual = resolve_crates_for_files(store, &files).await?;
    let declared = declared_crates(store, adr_scope).await?;

    let drift: Vec<String> = actual.difference(&declared).cloned().collect();
    let ghosts: Vec<String> = declared.difference(&actual).cloned().collect();

    Ok(DriftReport {
        since: since.to_string(),
        adr_scope: adr_scope.map(|s| s.to_string()),
        files_changed: files,
        actual_crates: actual.into_iter().collect(),
        declared_crates: declared.into_iter().collect(),
        drift,
        ghosts,
    })
}

async fn resolve_crates_for_files(store: &Store, files: &[String]) -> Result<BTreeSet<String>> {
    let mut out: BTreeSet<String> = BTreeSet::new();
    if files.is_empty() {
        return Ok(out);
    }
    // Single-shot lookup via IN (?, ?, ?, ...).
    let placeholders = files.iter().map(|_| "?").collect::<Vec<_>>().join(",");
    let sql = format!(
        "SELECT DISTINCT crate_name FROM graph_nodes \
         WHERE file_path IN ({placeholders}) AND crate_name != '__docs__'"
    );
    let mut q = sqlx::query(&sql);
    for f in files {
        q = q.bind(f);
    }
    let rows = q.fetch_all(store.pool().inner()).await?;
    for row in rows {
        let c: String = row.try_get("crate_name")?;
        out.insert(c);
    }
    // Fallback: also accept files under `crates/<name>/...` even if
    // the graph has not seen them (e.g. new file in this very diff).
    for f in files {
        if let Some(rest) = f.strip_prefix("crates/") {
            if let Some(name) = rest.split('/').next() {
                out.insert(name.to_string());
            }
        }
    }
    Ok(out)
}

async fn declared_crates(store: &Store, adr_scope: Option<&str>) -> Result<BTreeSet<String>> {
    let mut out: BTreeSet<String> = BTreeSet::new();
    let sql = match adr_scope {
        Some(_) => {
            "SELECT DISTINCT c.crate_name \
             FROM graph_edges e \
             JOIN graph_nodes adr ON e.src = adr.id \
             JOIN graph_nodes c ON e.dst = c.id \
             WHERE e.kind = 'claims' AND adr.kind = 'adr' \
               AND adr.name = ? AND c.kind = 'crate'"
        }
        None => {
            "SELECT DISTINCT c.crate_name \
             FROM graph_edges e \
             JOIN graph_nodes adr ON e.src = adr.id \
             JOIN graph_nodes c ON e.dst = c.id \
             WHERE e.kind = 'claims' AND adr.kind = 'adr' AND c.kind = 'crate'"
        }
    };
    let mut q = sqlx::query(sql);
    if let Some(id) = adr_scope {
        q = q.bind(id);
    }
    let rows = q.fetch_all(store.pool().inner()).await?;
    for row in rows {
        let c: String = row.try_get("crate_name")?;
        out.insert(c);
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Edge, EdgeKind, Node, NodeKind, DOCS_CRATE};
    use convergio_db::Pool;

    async fn fresh() -> (Store, tempfile::TempDir) {
        let dir = tempfile::tempdir().unwrap();
        let url = format!("sqlite://{}?mode=rwc", dir.path().join("g.db").display());
        let pool = Pool::connect(&url).await.unwrap();
        let store = Store::new(pool);
        store.migrate().await.unwrap();
        (store, dir)
    }

    fn crate_node(name: &str) -> Node {
        Node {
            id: Node::compute_id(NodeKind::Crate, "convergio", name, None, name, None),
            kind: NodeKind::Crate,
            name: name.into(),
            file_path: None,
            crate_name: name.into(),
            repo: "convergio".into(),
            item_kind: None,
            span: None,
        }
    }

    fn adr_node(name: &str) -> Node {
        Node {
            id: Node::compute_id(NodeKind::Adr, "convergio", DOCS_CRATE, None, name, None),
            kind: NodeKind::Adr,
            name: name.into(),
            file_path: None,
            crate_name: DOCS_CRATE.into(),
            repo: "convergio".into(),
            item_kind: None,
            span: None,
        }
    }

    #[tokio::test]
    async fn declared_returns_union_when_no_scope() {
        let (store, _dir) = fresh().await;
        let a = crate_node("alpha");
        let b = crate_node("beta");
        let adr1 = adr_node("0001");
        let adr2 = adr_node("0002");
        for n in [&a, &b, &adr1, &adr2] {
            store.upsert_node(n).await.unwrap();
        }
        store
            .upsert_edge(&Edge {
                src: adr1.id.clone(),
                dst: a.id.clone(),
                kind: EdgeKind::Claims,
                weight: 1,
            })
            .await
            .unwrap();
        store
            .upsert_edge(&Edge {
                src: adr2.id.clone(),
                dst: b.id.clone(),
                kind: EdgeKind::Claims,
                weight: 1,
            })
            .await
            .unwrap();

        let declared = declared_crates(&store, None).await.unwrap();
        assert!(declared.contains("alpha") && declared.contains("beta"));
    }

    #[tokio::test]
    async fn declared_scopes_to_one_adr() {
        let (store, _dir) = fresh().await;
        let a = crate_node("alpha");
        let b = crate_node("beta");
        let adr1 = adr_node("0001");
        let adr2 = adr_node("0002");
        for n in [&a, &b, &adr1, &adr2] {
            store.upsert_node(n).await.unwrap();
        }
        store
            .upsert_edge(&Edge {
                src: adr1.id.clone(),
                dst: a.id.clone(),
                kind: EdgeKind::Claims,
                weight: 1,
            })
            .await
            .unwrap();
        store
            .upsert_edge(&Edge {
                src: adr2.id.clone(),
                dst: b.id.clone(),
                kind: EdgeKind::Claims,
                weight: 1,
            })
            .await
            .unwrap();

        let just_0001 = declared_crates(&store, Some("0001")).await.unwrap();
        assert!(just_0001.contains("alpha"));
        assert!(!just_0001.contains("beta"));
    }

    #[tokio::test]
    async fn resolve_crates_falls_back_to_path_prefix() {
        let (store, _dir) = fresh().await;
        // No graph_nodes for the file — we fall back to the
        // crates/<name>/ path prefix so a file added in this diff
        // (and not yet parsed) still resolves.
        let resolved =
            resolve_crates_for_files(&store, &["crates/convergio-graph/src/drift.rs".to_string()])
                .await
                .unwrap();
        assert!(resolved.contains("convergio-graph"));
    }
}
