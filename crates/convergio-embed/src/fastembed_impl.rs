//! Real text embedder backed by `fastembed-rs` (local ONNX inference).
//!
//! ## Model choice
//!
//! ADR-0038 § 5.4 documents BGE-M3-small (384-dim, multilingual) as
//! the F1 default. `fastembed-rs` 5.x exposes:
//! - `EmbeddingModel::BGEM3` — 1024-dim (not 384)
//! - `EmbeddingModel::MultilingualE5Small` — 384-dim, multilingual,
//!   ~120MB ONNX
//!
//! There is no native 384-dim BGE-M3 variant in `fastembed-rs` (the
//! "small" suffix is `intfloat/multilingual-e5-small` in their model
//! zoo, not a BAAI/BGE small variant). To honour both the documented
//! 384-dim AND the multilingual requirement (CONSTITUTION P5 IT/EN),
//! F1-β ships `MultilingualE5Embedder` as the **default real model**.
//! The deviation from the literal "BGE-M3-small" name is recorded in
//! ADR-0038 decision log § F1-β. The `Embedder` trait is the seam:
//! swapping to a `BgeM3Embedder` (1024-dim) is a local change.
//!
//! ## Lifecycle
//!
//! Construction does not touch the network. The first `embed()` call
//! lazily downloads the model into `cache_dir` (recommended:
//! `~/.convergio/v3/models/`). Subsequent calls reuse the loaded
//! `TextEmbedding` instance. The embedder is `Send + Sync` so a
//! single instance can be shared across the daemon's tokio runtime.

use crate::embedder::{Embedder, EmbedderError};
use fastembed::{EmbeddingModel, InitOptions, TextEmbedding};
use std::path::PathBuf;
use std::sync::Mutex;

/// `intfloat/multilingual-e5-small` via `fastembed-rs`.
/// Multilingual (≥100 languages), 384-dim, ~120MB ONNX.
///
/// The underlying `fastembed::TextEmbedding` requires `&mut self` for
/// inference (each ONNX session keeps mutable internal state). To
/// keep [`Embedder::embed`] taking `&self` like every other impl, the
/// model is wrapped in a `std::sync::Mutex`. Embed calls therefore
/// serialise; for parallelism instantiate multiple embedders or wrap
/// in a connection-pool pattern.
pub struct MultilingualE5Embedder {
    inner: Mutex<Option<TextEmbedding>>,
    cache_dir: PathBuf,
}

impl MultilingualE5Embedder {
    /// Build an embedder that caches the ONNX model under `cache_dir`.
    /// The model is **not** downloaded until [`Embedder::embed`] is
    /// called.
    pub fn new(cache_dir: PathBuf) -> Self {
        Self {
            inner: Mutex::new(None),
            cache_dir,
        }
    }

    /// Returns `true` once the model is loaded into memory; useful for
    /// tests that want to assert lazy-load semantics without
    /// triggering a network download.
    pub fn is_loaded(&self) -> bool {
        self.inner.lock().map(|g| g.is_some()).unwrap_or(false)
    }
}

impl Embedder for MultilingualE5Embedder {
    fn dim(&self) -> usize {
        384
    }

    fn model_id(&self) -> &str {
        "multilingual-e5-small"
    }

    fn embed(&self, text: &str) -> Result<Vec<f32>, EmbedderError> {
        let mut guard = self
            .inner
            .lock()
            .map_err(|_| EmbedderError::ModelLoad("embedder mutex poisoned".into()))?;
        if guard.is_none() {
            let opts = InitOptions::new(EmbeddingModel::MultilingualE5Small)
                .with_cache_dir(self.cache_dir.clone())
                .with_show_download_progress(false);
            let model = TextEmbedding::try_new(opts)
                .map_err(|e| EmbedderError::ModelLoad(e.to_string()))?;
            *guard = Some(model);
        }
        let model = guard
            .as_mut()
            .ok_or_else(|| EmbedderError::ModelLoad("embedder slot empty".into()))?;
        let mut embeddings = model
            .embed(vec![text], None)
            .map_err(|e| EmbedderError::Inference(e.to_string()))?;
        embeddings
            .pop()
            .ok_or_else(|| EmbedderError::Inference("model returned no embeddings".into()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn construction_does_not_load_model() {
        let dir = tempdir().expect("tempdir");
        let e = MultilingualE5Embedder::new(dir.path().to_path_buf());
        assert!(!e.is_loaded());
        assert_eq!(e.dim(), 384);
        assert_eq!(e.model_id(), "multilingual-e5-small");
    }

    /// Real-model integration test. Ignored by default — running it
    /// downloads ~120MB into the supplied cache_dir on first run.
    /// To exercise locally:
    ///
    /// ```bash
    /// cargo test -p convergio-embed --features fastembed \
    ///     --test fastembed_impl -- --ignored
    /// ```
    #[test]
    #[ignore = "downloads ~120MB ONNX model on first run"]
    fn embeds_real_text_with_correct_dim() {
        let dir = tempdir().expect("tempdir");
        let e = MultilingualE5Embedder::new(dir.path().to_path_buf());
        let v_en = e.embed("hello fleet").expect("embed en");
        let v_it = e.embed("ciao flotta").expect("embed it");
        assert_eq!(v_en.len(), 384);
        assert_eq!(v_it.len(), 384);
        assert!(e.is_loaded(), "embed() should populate the OnceLock");
        // Multilingual model: parallel translations should be more
        // similar than two unrelated phrases.
        let v_unrelated = e.embed("a completely different topic").expect("embed");
        let cos_parallel = cosine(&v_en, &v_it);
        let cos_unrelated = cosine(&v_en, &v_unrelated);
        assert!(
            cos_parallel > cos_unrelated,
            "parallel translations should outscore unrelated text: \
             parallel={cos_parallel}, unrelated={cos_unrelated}"
        );
    }

    fn cosine(a: &[f32], b: &[f32]) -> f32 {
        let dot: f32 = a.iter().zip(b).map(|(x, y)| x * y).sum();
        let na: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
        let nb: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
        if na == 0.0 || nb == 0.0 {
            0.0
        } else {
            dot / (na * nb)
        }
    }
}
