//! Daemon-side embedder selection.
//!
//! Extracted from `main.rs` so the entry point stays under the
//! 300-line crate-wide cap. Behaviour is unchanged from the original
//! inline `make_embedder` — see ADR-0038, F1 for the model contract
//! and FR-3.9 for the graceful-degradation rule.

use std::sync::Arc;

/// Construct the daemon's embedder based on `CONVERGIO_EMBED_MODEL`.
///
/// - Unset / `deterministic-test` (default): `DeterministicTestEmbedder`
///   with the canonical 384-dim shape (matches the real-model dim
///   so a swap doesn't break stored vectors).
/// - `bge-m3-small-int8` (only when built with `--features fastembed`):
///   **requested** default from ADR/spec. `fastembed` 5.x does not ship
///   a 384-dim BGE-M3-small variant, so we currently map this to
///   `multilingual-e5-small` (384-dim, multilingual) with a warning.
/// - `multilingual-e5-small` (only when built with `--features fastembed`):
///   real ONNX via `fastembed-rs`.
///
/// FR-3.9 graceful degradation: an unknown value falls back to the test
/// embedder, never crashes the daemon.
pub fn make_embedder() -> Arc<dyn convergio_embed::Embedder> {
    let model = std::env::var("CONVERGIO_EMBED_MODEL").unwrap_or_default();
    match model.as_str() {
        "" | "deterministic-test" => {
            tracing::info!("embedder: deterministic-test (no model loaded)");
            Arc::new(convergio_embed::embedder::testing::DeterministicTestEmbedder::new(384))
        }
        #[cfg(feature = "fastembed")]
        requested @ ("bge-m3-small-int8" | "bge-m3-small") => {
            let cache = home_models_dir();
            tracing::warn!(
                requested,
                "fastembed does not provide a 384-dim bge-m3-small model; using multilingual-e5-small as a compatibility alias"
            );
            tracing::info!(?cache, "embedder: multilingual-e5-small (fastembed-rs)");
            Arc::new(convergio_embed::MultilingualE5Embedder::new(cache))
        }
        #[cfg(feature = "fastembed")]
        "bge-m3" => {
            let cache = home_models_dir();
            tracing::info!(?cache, "embedder: bge-m3 (fastembed-rs)");
            Arc::new(convergio_embed::BgeM3Embedder::new(cache))
        }
        #[cfg(feature = "fastembed")]
        "multilingual-e5-small" => {
            let cache = home_models_dir();
            tracing::info!(?cache, "embedder: multilingual-e5-small (fastembed-rs)");
            Arc::new(convergio_embed::MultilingualE5Embedder::new(cache))
        }
        other => {
            tracing::warn!(
                model = other,
                "unknown CONVERGIO_EMBED_MODEL; falling back to deterministic-test"
            );
            Arc::new(convergio_embed::embedder::testing::DeterministicTestEmbedder::new(384))
        }
    }
}

#[cfg(feature = "fastembed")]
fn home_models_dir() -> std::path::PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
    let dir = std::path::Path::new(&home).join(".convergio/v3/models");
    if let Err(e) = std::fs::create_dir_all(&dir) {
        tracing::warn!(path = %dir.display(), error = %e, "failed to create model cache dir");
    }
    dir
}
