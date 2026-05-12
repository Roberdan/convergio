//! Cross-repo similarity edge store (ADR-0038, F2-7/F2-8).
//!
//! Methods are added to [`FleetStore`] to persist and query the
//! `fleet_similar_edges` table that is populated by the fleet build.

use crate::error::Result;
use crate::store::FleetStore;
use sqlx::Row;

/// Cosine similarity threshold for a `similar_to` edge.
pub const SIMILAR_TO_THRESHOLD: f32 = 0.85;
/// Cosine similarity threshold for a `duplicates` edge (stronger).
pub const DUPLICATES_THRESHOLD: f32 = 0.95;

/// One cross-repo similarity edge.
#[derive(Debug, Clone)]
pub struct SimilarEdge {
    /// Repo of the source node.
    pub repo_a: String,
    /// Node identifier in `repo_a`.
    pub node_id_a: String,
    /// Repo of the target node.
    pub repo_b: String,
    /// Node identifier in `repo_b`.
    pub node_id_b: String,
    /// Cosine similarity score in `[0.0, 1.0]`.
    pub score: f32,
    /// Integer weight: `round(score × 1000)`.
    pub weight: u32,
    /// Edge kind: `"similar_to"` or `"duplicates"`.
    pub kind: String,
    /// ISO-8601 timestamp of when this edge was computed.
    pub built_at: String,
}

impl FleetStore {
    /// Insert or replace a cross-repo similarity edge, with explicit kind.
    ///
    /// `kind` must be `"similar_to"` or `"duplicates"` — the DB CHECK
    /// constraint enforces this.  `weight` is stored as
    /// `round(score × 1000)`.
    pub async fn upsert_similar_edge_classified(
        &self,
        repo_a: &str,
        node_id_a: &str,
        repo_b: &str,
        node_id_b: &str,
        score: f32,
        kind: &str,
    ) -> Result<()> {
        let weight = (score * 1000.0).round() as i64;
        sqlx::query(
            "INSERT OR REPLACE INTO fleet_similar_edges \
             (repo_a, node_id_a, repo_b, node_id_b, score, kind, weight) \
             VALUES (?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(repo_a)
        .bind(node_id_a)
        .bind(repo_b)
        .bind(node_id_b)
        .bind(score)
        .bind(kind)
        .bind(weight)
        .execute(self.pool().inner())
        .await?;
        Ok(())
    }

    /// Insert or replace a cross-repo similarity edge, classifying by threshold.
    ///
    /// Scores ≥ [`DUPLICATES_THRESHOLD`] → `"duplicates"`;
    /// scores ≥ [`SIMILAR_TO_THRESHOLD`] → `"similar_to"`.
    /// Scores **below** [`SIMILAR_TO_THRESHOLD`] are silently dropped:
    /// the helper is a no-op and no row is written. This keeps the
    /// invariant that every persisted edge is at or above the documented
    /// 0.85 floor. For structural-shape-aware classification use
    /// [`Self::upsert_similar_edge_classified`] directly.
    pub async fn upsert_similar_edge(
        &self,
        repo_a: &str,
        node_id_a: &str,
        repo_b: &str,
        node_id_b: &str,
        score: f32,
    ) -> Result<()> {
        if score < SIMILAR_TO_THRESHOLD {
            return Ok(());
        }
        let kind = if score >= DUPLICATES_THRESHOLD {
            "duplicates"
        } else {
            "similar_to"
        };
        self.upsert_similar_edge_classified(repo_a, node_id_a, repo_b, node_id_b, score, kind)
            .await
    }

    /// Remove all similarity edges (called before a full rebuild).
    pub async fn clear_similar_edges(&self) -> Result<()> {
        sqlx::query("DELETE FROM fleet_similar_edges")
            .execute(self.pool().inner())
            .await?;
        Ok(())
    }

    /// Count stored similarity edges, optionally filtered by kind.
    pub async fn count_similar_edges(&self, kind: Option<&str>) -> Result<u64> {
        let count: i64 = match kind {
            Some(k) => {
                sqlx::query_scalar("SELECT COUNT(*) FROM fleet_similar_edges WHERE kind = ?")
                    .bind(k)
                    .fetch_one(self.pool().inner())
                    .await?
            }
            None => {
                sqlx::query_scalar("SELECT COUNT(*) FROM fleet_similar_edges")
                    .fetch_one(self.pool().inner())
                    .await?
            }
        };
        Ok(count as u64)
    }

    /// List **all** similarity edges without a limit, ordered by descending score.
    ///
    /// Used by the cluster-detection pass in [`crate::patterns`].
    pub async fn list_all_similar_edges(&self) -> Result<Vec<SimilarEdge>> {
        let rows = sqlx::query(
            "SELECT repo_a, node_id_a, repo_b, node_id_b, score, weight, kind, built_at \
             FROM fleet_similar_edges \
             ORDER BY score DESC",
        )
        .fetch_all(self.pool().inner())
        .await?;

        Ok(rows
            .iter()
            .map(|r| SimilarEdge {
                repo_a: r.get("repo_a"),
                node_id_a: r.get("node_id_a"),
                repo_b: r.get("repo_b"),
                node_id_b: r.get("node_id_b"),
                score: r.get::<f64, _>("score") as f32,
                weight: r.get::<i64, _>("weight") as u32,
                kind: r.get("kind"),
                built_at: r.get("built_at"),
            })
            .collect())
    }

    /// List all similarity edges, ordered by descending score.
    pub async fn list_similar_edges(&self, limit: usize) -> Result<Vec<SimilarEdge>> {
        let rows = sqlx::query(
            "SELECT repo_a, node_id_a, repo_b, node_id_b, score, weight, kind, built_at \
             FROM fleet_similar_edges \
             ORDER BY score DESC \
             LIMIT ?",
        )
        .bind(limit as i64)
        .fetch_all(self.pool().inner())
        .await?;

        Ok(rows
            .iter()
            .map(|r| SimilarEdge {
                repo_a: r.get("repo_a"),
                node_id_a: r.get("node_id_a"),
                repo_b: r.get("repo_b"),
                node_id_b: r.get("node_id_b"),
                score: r.get::<f64, _>("score") as f32,
                weight: r.get::<i64, _>("weight") as u32,
                kind: r.get("kind"),
                built_at: r.get("built_at"),
            })
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use crate::config::{RepoEntry, RepoRole};
    use crate::migrate::init;
    use crate::store::FleetStore;

    async fn test_store() -> (FleetStore, tempfile::NamedTempFile) {
        let tmp = tempfile::NamedTempFile::new().unwrap();
        let url = format!("sqlite://{}", tmp.path().display());
        let pool = convergio_db::Pool::connect(&url).await.unwrap();
        init(&pool).await.unwrap();
        (FleetStore::new(pool), tmp)
    }

    fn repo(name: &str) -> RepoEntry {
        RepoEntry {
            name: name.to_owned(),
            path: format!("/repos/{name}"),
            language: "rust".to_owned(),
            parser: "syn".to_owned(),
            role: RepoRole::Engine,
            derives_from: None,
        }
    }

    #[tokio::test]
    async fn upsert_and_count() {
        let (store, _tmp) = test_store().await;
        store.add_repo(&repo("alpha")).await.unwrap();
        store.add_repo(&repo("beta")).await.unwrap();
        store
            .upsert_similar_edge("alpha", "src/lib.rs", "beta", "src/lib.rs", 0.90)
            .await
            .unwrap();
        assert_eq!(store.count_similar_edges(None).await.unwrap(), 1);
        assert_eq!(
            store.count_similar_edges(Some("similar_to")).await.unwrap(),
            1
        );
        assert_eq!(
            store.count_similar_edges(Some("duplicates")).await.unwrap(),
            0
        );
    }

    #[tokio::test]
    async fn upsert_idempotent_updates_score() {
        let (store, _tmp) = test_store().await;
        store
            .upsert_similar_edge("a", "n1", "b", "n2", 0.86)
            .await
            .unwrap();
        store
            .upsert_similar_edge("a", "n1", "b", "n2", 0.96)
            .await
            .unwrap();
        let edges = store.list_similar_edges(10).await.unwrap();
        assert_eq!(edges.len(), 1);
        assert!(edges[0].score >= 0.95, "score should be updated to 0.96");
        assert_eq!(edges[0].kind, "duplicates");
        assert_eq!(edges[0].weight, 960);
    }

    #[tokio::test]
    async fn clear_removes_all_edges() {
        let (store, _tmp) = test_store().await;
        store
            .upsert_similar_edge("a", "n1", "b", "n2", 0.88)
            .await
            .unwrap();
        store.clear_similar_edges().await.unwrap();
        assert_eq!(store.count_similar_edges(None).await.unwrap(), 0);
    }

    #[tokio::test]
    async fn weight_computed_correctly() {
        let (store, _tmp) = test_store().await;
        store
            .upsert_similar_edge("x", "n1", "y", "n2", 0.875)
            .await
            .unwrap();
        let edges = store.list_similar_edges(1).await.unwrap();
        assert_eq!(edges[0].weight, 875);
    }

    #[tokio::test]
    async fn upsert_below_similar_to_threshold_is_noop() {
        // Regression: `upsert_similar_edge` used to store any score below
        // DUPLICATES_THRESHOLD as `similar_to`, even when the score was
        // below the documented 0.85 `SIMILAR_TO_THRESHOLD`. Callers should
        // not be able to slip sub-threshold edges into the table.
        let (store, _tmp) = test_store().await;
        store
            .upsert_similar_edge("a", "n1", "b", "n2", 0.10)
            .await
            .unwrap();
        store
            .upsert_similar_edge("a", "n1", "b", "n2", 0.84)
            .await
            .unwrap();
        assert_eq!(
            store.count_similar_edges(None).await.unwrap(),
            0,
            "scores below SIMILAR_TO_THRESHOLD must not produce edges"
        );
    }

    #[tokio::test]
    async fn classified_upsert_respects_explicit_kind() {
        let (store, _tmp) = test_store().await;
        // Score would normally give "duplicates" (≥0.95), but we force "similar_to".
        store
            .upsert_similar_edge_classified("x", "n1", "y", "n2", 0.97, "similar_to")
            .await
            .unwrap();
        let edges = store.list_similar_edges(1).await.unwrap();
        assert_eq!(edges[0].kind, "similar_to");
    }
}
