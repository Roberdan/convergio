//! SQLite persistence for graph nodes + edges.
//!
//! All writes happen in a single transaction per call so that a
//! failed parse mid-build does not leave a partial graph.

use crate::error::Result;
use crate::model::{Edge, EdgeKind, Node, NodeKind};
use chrono::{DateTime, Utc};
use convergio_db::Pool;
use sqlx::{Row, Sqlite, Transaction};
use std::collections::HashMap;
use std::path::Path;

// Migration range 600-699 reserved for convergio-graph (ADR-0003).
// `set_ignore_missing(true)` in `migrate()` lets this migrator
// coexist with sibling crates on the same `_sqlx_migrations` table.

/// Storage handle: thin wrapper around the shared SQLite pool.
#[derive(Clone)]
pub struct Store {
    pool: Pool,
}

impl Store {
    /// Bind to the existing SQLite pool.
    pub fn new(pool: Pool) -> Self {
        Self { pool }
    }

    /// Borrow the underlying pool — used by query helpers that build raw SQL.
    pub fn pool(&self) -> &Pool {
        &self.pool
    }

    /// Run pending migrations (range 600-699). Idempotent — safe to
    /// call on every daemon start. Coexists with sibling crates'
    /// migrators thanks to `set_ignore_missing(true)`.
    pub async fn migrate(&self) -> Result<()> {
        let mut migrator = sqlx::migrate!("./migrations");
        migrator.set_ignore_missing(true);
        migrator.run(self.pool.inner()).await?;
        Ok(())
    }

    /// Replace all nodes + edges for a given file with the supplied
    /// set, atomically. Stores the file's mtime so subsequent calls
    /// can detect staleness.
    pub async fn upsert_file(
        &self,
        file_path: &str,
        source_mtime: DateTime<Utc>,
        nodes: &[Node],
        edges: &[Edge],
    ) -> Result<()> {
        let mut tx: Transaction<'_, Sqlite> = self.pool.inner().begin().await?;

        // Drop the previous nodes for this file (cascades to edges).
        sqlx::query("DELETE FROM graph_nodes WHERE file_path = ?")
            .bind(file_path)
            .execute(&mut *tx)
            .await?;

        let now = Utc::now().to_rfc3339();
        let mtime = source_mtime.to_rfc3339();
        for n in nodes {
            sqlx::query(
                "INSERT OR REPLACE INTO graph_nodes \
                 (id, kind, name, file_path, crate_name, repo, item_kind, span_start, span_end, last_parsed, source_mtime) \
                 VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
            )
            .bind(&n.id)
            .bind(n.kind.as_str())
            .bind(&n.name)
            .bind(n.file_path.as_deref())
            .bind(&n.crate_name)
            .bind(&n.repo)
            .bind(n.item_kind)
            .bind(n.span.map(|(s, _)| s as i64))
            .bind(n.span.map(|(_, e)| e as i64))
            .bind(&now)
            .bind(&mtime)
            .execute(&mut *tx)
            .await?;
        }
        for e in edges {
            sqlx::query(
                "INSERT OR REPLACE INTO graph_edges (src, dst, kind, weight) VALUES (?, ?, ?, ?)",
            )
            .bind(&e.src)
            .bind(&e.dst)
            .bind(e.kind.as_str())
            .bind(e.weight as i64)
            .execute(&mut *tx)
            .await?;
        }

        tx.commit().await?;
        Ok(())
    }

    /// Insert (or replace) a single non-file-bound node — useful for
    /// crate-level + ADR/doc nodes that don't have a parsed source.
    pub async fn upsert_node(&self, node: &Node) -> Result<()> {
        let now = Utc::now().to_rfc3339();
        sqlx::query(
            "INSERT OR REPLACE INTO graph_nodes \
             (id, kind, name, file_path, crate_name, repo, item_kind, span_start, span_end, last_parsed, source_mtime) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(&node.id)
        .bind(node.kind.as_str())
        .bind(&node.name)
        .bind(node.file_path.as_deref())
        .bind(&node.crate_name)
        .bind(&node.repo)
        .bind(node.item_kind)
        .bind(node.span.map(|(s, _)| s as i64))
        .bind(node.span.map(|(_, e)| e as i64))
        .bind(&now)
        .bind(&now)
        .execute(self.pool.inner())
        .await?;
        Ok(())
    }

    /// Insert (or replace) a single edge.
    pub async fn upsert_edge(&self, edge: &Edge) -> Result<()> {
        sqlx::query(
            "INSERT OR REPLACE INTO graph_edges (src, dst, kind, weight) VALUES (?, ?, ?, ?)",
        )
        .bind(&edge.src)
        .bind(&edge.dst)
        .bind(edge.kind.as_str())
        .bind(edge.weight as i64)
        .execute(self.pool.inner())
        .await?;
        Ok(())
    }

    /// Total node count.
    pub async fn count_nodes(&self) -> Result<usize> {
        let row = sqlx::query("SELECT COUNT(*) AS c FROM graph_nodes")
            .fetch_one(self.pool.inner())
            .await?;
        let c: i64 = row.try_get("c")?;
        Ok(c as usize)
    }

    /// Total edge count.
    pub async fn count_edges(&self) -> Result<usize> {
        let row = sqlx::query("SELECT COUNT(*) AS c FROM graph_edges")
            .fetch_one(self.pool.inner())
            .await?;
        let c: i64 = row.try_get("c")?;
        Ok(c as usize)
    }

    /// Per-file mtime map for staleness checks. Empty map = first build.
    pub async fn file_mtimes(&self) -> Result<HashMap<String, String>> {
        let rows = sqlx::query(
            "SELECT file_path, MAX(source_mtime) AS m FROM graph_nodes \
             WHERE file_path IS NOT NULL GROUP BY file_path",
        )
        .fetch_all(self.pool.inner())
        .await?;
        let mut out = HashMap::with_capacity(rows.len());
        for row in rows {
            let p: Option<String> = row.try_get("file_path")?;
            let m: Option<String> = row.try_get("m")?;
            if let (Some(p), Some(m)) = (p, m) {
                out.insert(p, m);
            }
        }
        Ok(out)
    }
}

/// Returns true if the on-disk file has a newer mtime than the
/// stored value (or no stored value exists).
pub fn is_stale(file: &Path, stored: Option<&String>) -> bool {
    let Ok(meta) = std::fs::metadata(file) else {
        return true;
    };
    let Ok(modified) = meta.modified() else {
        return true;
    };
    let on_disk: DateTime<Utc> = modified.into();
    match stored {
        None => true,
        Some(s) => match DateTime::parse_from_rfc3339(s) {
            Ok(parsed) => on_disk > parsed.with_timezone(&Utc),
            Err(_) => true,
        },
    }
}

// Re-export for the lib root.
pub use sqlx::Row as _SqlxRow;

/// Convenience helper used by tests + tooling.
pub fn edge_kind_to_str(k: EdgeKind) -> &'static str {
    k.as_str()
}

/// Convenience helper used by tests + tooling.
pub fn node_kind_to_str(k: NodeKind) -> &'static str {
    k.as_str()
}
