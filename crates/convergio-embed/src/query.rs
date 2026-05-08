//! Semantic query: embed the query text, run nearest-neighbor against
//! the stored corpus.
//!
//! This is the read-side counterpart to [`crate::ingest::ingest`].
//! Hybrid fusion (RRF / linear over substring + semantic) is wired in
//! the daemon orchestrator; this module owns only the semantic-only
//! path and stays pure (no graph dep, no HTTP dep).

use crate::embedder::{Embedder, EmbedderError};
use crate::error::EmbedError;
use crate::store::{EmbedStore, Neighbor};

/// Run a semantic-only nearest-neighbor query.
///
/// Embeds `query` with `embedder`, then asks the store for the top
/// `limit` neighbors keyed by [`Embedder::model_id`]. Returns an
/// empty list (not an error) when the store has no rows for the
/// configured model — this happens before the first ingest run.
pub async fn semantic_search(
    store: &EmbedStore,
    embedder: &dyn Embedder,
    query: &str,
    limit: usize,
) -> Result<Vec<Neighbor>, EmbedError> {
    if query.trim().is_empty() || limit == 0 {
        return Ok(Vec::new());
    }

    // FR-3.9 graceful degradation is implemented by the daemon
    // orchestrator: when the embedder fails (model unavailable, OOM),
    // callers should fall back to structural-only retrieval.
    let q = embedder.embed(query).map_err(map_embedder_error)?;

    store
        .nearest_brute_force(&q, embedder.model_id(), limit)
        .await
}

fn map_embedder_error(e: EmbedderError) -> EmbedError {
    EmbedError::EmbedderFailed(e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::embedder::testing::DeterministicTestEmbedder;
    use crate::embedder::EmbedderError;
    use crate::ingest::{ingest, IngestNode};
    use convergio_db::Pool;
    use tempfile::tempdir;

    async fn boot() -> (Pool, tempfile::TempDir) {
        let dir = tempdir().expect("tempdir");
        let url = format!(
            "sqlite://{}?mode=rwc",
            dir.path().join("state.db").display()
        );
        let pool = Pool::connect(&url).await.expect("connect");
        crate::init(&pool).await.expect("migrate");
        (pool, dir)
    }

    #[tokio::test]
    async fn empty_query_returns_empty() {
        let (pool, _dir) = boot().await;
        let store = EmbedStore::new(pool);
        let e = DeterministicTestEmbedder::new(8);
        assert!(semantic_search(&store, &e, "", 5).await.unwrap().is_empty());
        assert!(semantic_search(&store, &e, "   ", 5)
            .await
            .unwrap()
            .is_empty());
        assert!(semantic_search(&store, &e, "any", 0)
            .await
            .unwrap()
            .is_empty());
    }

    #[tokio::test]
    async fn empty_store_returns_empty() {
        let (pool, _dir) = boot().await;
        let store = EmbedStore::new(pool);
        let e = DeterministicTestEmbedder::new(8);
        assert!(semantic_search(&store, &e, "query", 5)
            .await
            .unwrap()
            .is_empty());
    }

    #[tokio::test]
    async fn finds_seeded_match() {
        let (pool, _dir) = boot().await;
        let store = EmbedStore::new(pool);
        let e = DeterministicTestEmbedder::new(16);
        let nodes = ["alpha", "beta", "gamma"]
            .into_iter()
            .map(|s| IngestNode {
                repo: "convergio".into(),
                node_id: format!("n-{s}"),
                source: s.into(),
            })
            .collect::<Vec<_>>();
        ingest(&store, &e, nodes).await.expect("ingest");
        // Querying the exact text returns its own node first with
        // cosine ≈ 1.
        let hits = semantic_search(&store, &e, "gamma", 3)
            .await
            .expect("search");
        assert_eq!(hits.len(), 3);
        assert_eq!(hits[0].node_id, "n-gamma");
        assert!((hits[0].score - 1.0).abs() < 1e-5);
    }

    struct FailingEmbedder;

    impl Embedder for FailingEmbedder {
        fn dim(&self) -> usize {
            384
        }

        fn model_id(&self) -> &str {
            "failing-embedder"
        }

        fn embed(&self, _text: &str) -> Result<Vec<f32>, EmbedderError> {
            Err(EmbedderError::ModelLoad("download failed".into()))
        }
    }

    #[tokio::test]
    async fn embedder_failure_surfaces_as_embedder_failed() {
        let (pool, _dir) = boot().await;
        let store = EmbedStore::new(pool);
        let e = FailingEmbedder;
        let err = semantic_search(&store, &e, "query", 10)
            .await
            .expect_err("expected semantic_search to surface embedder failure");
        assert!(matches!(err, EmbedError::EmbedderFailed(_)));
    }
}
