//! Ingest text files into the embedding store.
//!
//! Each input is `(repo, node_id, raw_text)`. The function:
//! 1. Wraps `raw_text` in [`SourceText`] (trim + SHA-256 hash).
//! 2. Skips when `EmbedStore::needs_reembed` reports an unchanged hash
//!    (idempotent over re-runs — ADR-0038 § 5.4).
//! 3. Calls the embedder; on failure logs a warning and bumps the
//!    `failed` counter (FR-3.9 graceful degradation).
//! 4. Upserts the resulting vector.
//!
//! This module is intentionally generic — it does not walk a
//! filesystem or query a graph. The orchestrator (e.g. the daemon's
//! `/v1/embed/build` route) decides what corpus to feed in.

use crate::embedder::{Embedder, EmbedderError};
use crate::error::EmbedError;
use crate::store::EmbedStore;
use crate::SourceText;

/// One unit of ingestion: a stable `(repo, node_id)` key plus the
/// raw text to embed. The raw text is trimmed and hashed inside
/// [`ingest_one`] — callers do not need to pre-process.
#[derive(Debug, Clone)]
pub struct IngestNode {
    /// Repo identifier (e.g. `"convergio"`).
    pub repo: String,
    /// Stable node id within the repo (typically the file path).
    pub node_id: String,
    /// Raw embeddable text. Will be trimmed before hashing.
    pub source: String,
}

/// Tally of what happened during a [`ingest`] call.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct IngestReport {
    /// Total inputs the orchestrator handed over.
    pub considered: usize,
    /// Inputs whose stored hash matched the input hash; skipped.
    pub skipped_unchanged: usize,
    /// Inputs that produced a new or refreshed embedding.
    pub embedded: usize,
    /// Inputs the embedder refused (e.g. model load failure). The
    /// store row, if any, is left untouched.
    pub failed: usize,
}

/// Embed a single node, respecting the source-hash idempotence rule.
///
/// Returns `Ok(true)` when a new embedding was written, `Ok(false)`
/// when the row was already up to date.
pub async fn ingest_one(
    store: &EmbedStore,
    embedder: &dyn Embedder,
    node: &IngestNode,
) -> Result<bool, EmbedError> {
    let text = SourceText::new(&node.source);
    if !store
        .needs_reembed(
            &node.repo,
            &node.node_id,
            embedder.model_id(),
            &text.source_hash,
        )
        .await?
    {
        return Ok(false);
    }
    let vec = embedder.embed(&text.text).map_err(map_embedder_error)?;
    store
        .upsert(
            &node.repo,
            &node.node_id,
            embedder.model_id(),
            &vec,
            &text.source_hash,
        )
        .await?;
    Ok(true)
}

/// Embed a batch of nodes. Embedder failures are logged and counted
/// in [`IngestReport::failed`] but never abort the loop — partial
/// progress is preferable to losing a long ingest run on one bad row.
pub async fn ingest(
    store: &EmbedStore,
    embedder: &dyn Embedder,
    nodes: impl IntoIterator<Item = IngestNode>,
) -> Result<IngestReport, EmbedError> {
    let mut report = IngestReport::default();
    for node in nodes {
        report.considered += 1;
        match ingest_one(store, embedder, &node).await {
            Ok(true) => report.embedded += 1,
            Ok(false) => report.skipped_unchanged += 1,
            Err(EmbedError::EmbedderFailed(msg)) => {
                tracing::warn!(node = %node.node_id, error = %msg, "embedder failed; row left as-is");
                report.failed += 1;
            }
            Err(e) => return Err(e),
        }
    }
    Ok(report)
}

fn map_embedder_error(e: EmbedderError) -> EmbedError {
    EmbedError::EmbedderFailed(e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::embedder::testing::DeterministicTestEmbedder;
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
    async fn idempotent_over_unchanged_source() {
        let (pool, _dir) = boot().await;
        let store = EmbedStore::new(pool);
        let embedder = DeterministicTestEmbedder::new(8);
        let node = IngestNode {
            repo: "convergio".into(),
            node_id: "src/main.rs".into(),
            source: "fn main() {}".into(),
        };
        let r1 = ingest(&store, &embedder, [node.clone()])
            .await
            .expect("first");
        let r2 = ingest(&store, &embedder, [node]).await.expect("second");
        assert_eq!(r1.embedded, 1);
        assert_eq!(r1.skipped_unchanged, 0);
        assert_eq!(r2.embedded, 0);
        assert_eq!(r2.skipped_unchanged, 1);
    }

    #[tokio::test]
    async fn re_embeds_when_source_changes() {
        let (pool, _dir) = boot().await;
        let store = EmbedStore::new(pool);
        let embedder = DeterministicTestEmbedder::new(8);
        let mut node = IngestNode {
            repo: "convergio".into(),
            node_id: "src/main.rs".into(),
            source: "fn main() { println!(\"v1\"); }".into(),
        };
        ingest(&store, &embedder, [node.clone()]).await.expect("v1");
        node.source = "fn main() { println!(\"v2\"); }".into();
        let r2 = ingest(&store, &embedder, [node]).await.expect("v2");
        assert_eq!(r2.embedded, 1);
        assert_eq!(r2.skipped_unchanged, 0);
    }

    #[tokio::test]
    async fn ingest_one_returns_false_when_unchanged() {
        let (pool, _dir) = boot().await;
        let store = EmbedStore::new(pool);
        let embedder = DeterministicTestEmbedder::new(8);
        let node = IngestNode {
            repo: "convergio".into(),
            node_id: "x".into(),
            source: "y".into(),
        };
        assert!(ingest_one(&store, &embedder, &node).await.expect("first"));
        assert!(!ingest_one(&store, &embedder, &node).await.expect("second"));
    }
}
