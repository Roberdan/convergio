//! SQLite persistence for embeddings.
//!
//! Schema in `migrations/0700_embeddings.sql`. F1-α uses pure-Rust
//! brute-force cosine for nearest-neighbor — `sqlite-vec`'s `vec0`
//! virtual table arrives in F1-β alongside the extension load path.
//! The [`EmbedStore`] signature is stable across the swap so callers
//! do not change.

use crate::codec::{blob_to_floats, cosine, floats_to_blob, norm};
use crate::error::Result;
use chrono::Utc;
use convergio_db::Pool;
use sqlx::Row;

/// One stored embedding row.
#[derive(Debug, Clone, PartialEq)]
pub struct EmbeddingRow {
    /// Repo identifier (e.g. `"convergio"`).
    pub repo: String,
    /// Node identifier from the caller's graph.
    pub node_id: String,
    /// Model identifier (e.g. `"deterministic-test-d8"`).
    pub model: String,
    /// Vector dimension.
    pub dim: usize,
    /// Embedding values. Length equals [`Self::dim`].
    pub vec: Vec<f32>,
    /// SHA-256 of the source text, hex-encoded.
    pub source_hash: String,
    /// RFC 3339 timestamp of when this row was last embedded.
    pub embedded_at: String,
}

/// One nearest-neighbor hit returned by [`EmbedStore::nearest_brute_force`].
#[derive(Debug, Clone, PartialEq)]
pub struct Neighbor {
    /// Repo of the matched node.
    pub repo: String,
    /// Node id of the matched node.
    pub node_id: String,
    /// Cosine similarity in `[-1.0, 1.0]`. Higher is more similar.
    pub score: f32,
}

/// Storage handle: thin wrapper around the shared SQLite pool.
#[derive(Clone)]
pub struct EmbedStore {
    pool: Pool,
}

impl EmbedStore {
    /// Bind to the existing SQLite pool.
    pub fn new(pool: Pool) -> Self {
        Self { pool }
    }

    /// Insert or replace an embedding for `(repo, node_id, model)`.
    ///
    /// The vector is stored as a tightly-packed little-endian `f32`
    /// blob; `dim` is recorded explicitly so the read path can
    /// detect corruption.
    pub async fn upsert(
        &self,
        repo: &str,
        node_id: &str,
        model: &str,
        vec: &[f32],
        source_hash: &str,
    ) -> Result<()> {
        let blob = floats_to_blob(vec);
        let now = Utc::now().to_rfc3339();
        sqlx::query(
            "INSERT OR REPLACE INTO graph_node_embeddings \
             (repo, node_id, model, dim, vec, embedded_at, source_hash) \
             VALUES (?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(repo)
        .bind(node_id)
        .bind(model)
        .bind(vec.len() as i64)
        .bind(&blob)
        .bind(&now)
        .bind(source_hash)
        .execute(self.pool.inner())
        .await?;
        Ok(())
    }

    /// Fetch one embedding by composite key.
    pub async fn get(
        &self,
        repo: &str,
        node_id: &str,
        model: &str,
    ) -> Result<Option<EmbeddingRow>> {
        let row = sqlx::query(
            "SELECT repo, node_id, model, dim, vec, embedded_at, source_hash \
             FROM graph_node_embeddings \
             WHERE repo = ? AND node_id = ? AND model = ?",
        )
        .bind(repo)
        .bind(node_id)
        .bind(model)
        .fetch_optional(self.pool.inner())
        .await?;
        let Some(row) = row else { return Ok(None) };
        let dim_i: i64 = row.get("dim");
        let dim = dim_i as usize;
        let blob: Vec<u8> = row.get("vec");
        let vec = blob_to_floats(&blob, dim)?;
        Ok(Some(EmbeddingRow {
            repo: row.get("repo"),
            node_id: row.get("node_id"),
            model: row.get("model"),
            dim,
            vec,
            source_hash: row.get("source_hash"),
            embedded_at: row.get("embedded_at"),
        }))
    }

    /// Delete one embedding by composite key. Returns the number of
    /// rows actually removed (0 or 1).
    pub async fn delete(&self, repo: &str, node_id: &str, model: &str) -> Result<u64> {
        let res = sqlx::query(
            "DELETE FROM graph_node_embeddings \
             WHERE repo = ? AND node_id = ? AND model = ?",
        )
        .bind(repo)
        .bind(node_id)
        .bind(model)
        .execute(self.pool.inner())
        .await?;
        Ok(res.rows_affected())
    }

    /// True when no embedding exists for `(repo, node_id, model)`,
    /// or when the stored `source_hash` differs from the supplied
    /// one. This is the canonical re-embed trigger (ADR-0038 § 5.4).
    pub async fn needs_reembed(
        &self,
        repo: &str,
        node_id: &str,
        model: &str,
        source_hash: &str,
    ) -> Result<bool> {
        let existing: Option<String> = sqlx::query_scalar(
            "SELECT source_hash FROM graph_node_embeddings \
             WHERE repo = ? AND node_id = ? AND model = ?",
        )
        .bind(repo)
        .bind(node_id)
        .bind(model)
        .fetch_optional(self.pool.inner())
        .await?;
        Ok(match existing {
            Some(h) => h != source_hash,
            None => true,
        })
    }

    /// Brute-force cosine KNN over all stored embeddings of the
    /// given `model`. Returns rows sorted by descending similarity,
    /// truncated to `limit`.
    ///
    /// Rows whose `dim` does not match the query length are skipped
    /// rather than aborting the search — different embedders may
    /// coexist legitimately.
    ///
    /// F1-β replaces this implementation with `sqlite-vec`'s `vec0`
    /// for sub-second 200K-vec queries; the function signature is
    /// kept stable.
    pub async fn nearest_brute_force(
        &self,
        query: &[f32],
        model: &str,
        limit: usize,
    ) -> Result<Vec<Neighbor>> {
        if query.is_empty() || limit == 0 {
            return Ok(Vec::new());
        }
        let q_norm = norm(query);
        if q_norm == 0.0 {
            return Ok(Vec::new());
        }

        let rows = sqlx::query(
            "SELECT repo, node_id, dim, vec FROM graph_node_embeddings WHERE model = ?",
        )
        .bind(model)
        .fetch_all(self.pool.inner())
        .await?;

        let mut hits: Vec<Neighbor> = Vec::with_capacity(rows.len());
        for row in rows {
            let dim_i: i64 = row.get("dim");
            let dim = dim_i as usize;
            if dim != query.len() {
                continue;
            }
            let blob: Vec<u8> = row.get("vec");
            let v = blob_to_floats(&blob, dim)?;
            let score = cosine(query, &v, q_norm);
            hits.push(Neighbor {
                repo: row.get("repo"),
                node_id: row.get("node_id"),
                score,
            });
        }
        hits.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        hits.truncate(limit);
        Ok(hits)
    }

    /// Return all stored embeddings for a given model as
    /// `(repo, node_id, vec)` triples.
    ///
    /// Used by the fleet similarity rebuild (ADR-0038, F2-7) to build
    /// cross-repo edges in a single pass without re-querying per node.
    pub async fn all_for_model(&self, model: &str) -> Result<Vec<(String, String, Vec<f32>)>> {
        let rows = sqlx::query(
            "SELECT repo, node_id, dim, vec \
             FROM graph_node_embeddings WHERE model = ?",
        )
        .bind(model)
        .fetch_all(self.pool.inner())
        .await?;

        let mut out = Vec::with_capacity(rows.len());
        for row in rows {
            let dim_i: i64 = row.get("dim");
            let dim = dim_i as usize;
            let blob: Vec<u8> = row.get("vec");
            let vec = blob_to_floats(&blob, dim)?;
            let repo: String = row.get("repo");
            let node_id: String = row.get("node_id");
            out.push((repo, node_id, vec));
        }
        Ok(out)
    }

    /// Total number of stored embeddings, optionally filtered by
    /// repo. Cheap — used by the `/v1/embed/stats` route.
    pub async fn count(&self, repo: Option<&str>) -> Result<u64> {
        let count: i64 = match repo {
            Some(r) => {
                sqlx::query_scalar("SELECT COUNT(*) FROM graph_node_embeddings WHERE repo = ?")
                    .bind(r)
                    .fetch_one(self.pool.inner())
                    .await?
            }
            None => {
                sqlx::query_scalar("SELECT COUNT(*) FROM graph_node_embeddings")
                    .fetch_one(self.pool.inner())
                    .await?
            }
        };
        Ok(count as u64)
    }
}
