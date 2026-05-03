//! Pluggable embedder.
//!
//! An [`Embedder`] turns text into a fixed-dimension vector. The trait
//! is the seam between this crate and the actual model — F1-α ships
//! only [`testing::DeterministicTestEmbedder`]; a real
//! `fastembed-rs`-backed implementation lands in F1-β.

use sha2::{Digest, Sha256};
use thiserror::Error;

/// Errors an [`Embedder`] may produce.
#[derive(Debug, Error)]
pub enum EmbedderError {
    /// Model could not be loaded (e.g. download failed, file missing).
    #[error("model load failed: {0}")]
    ModelLoad(String),
    /// Inference failed (e.g. tokenizer error, OOM).
    #[error("inference failed: {0}")]
    Inference(String),
}

/// Pluggable text → vector embedder.
///
/// Implementations must be **deterministic** for the same input on
/// the same hardware family. See ADR-0038 § 7.2 and
/// `docs/spec/fleet-retrieval-golden-methodology.md` § 6 for the
/// determinism rules CI relies on.
pub trait Embedder: Send + Sync {
    /// Embedding dimension; the same for every output of this
    /// embedder. Stored alongside the vector in
    /// `graph_node_embeddings.dim` so the read path can detect
    /// corruption.
    fn dim(&self) -> usize;

    /// Stable model identifier, written into
    /// `graph_node_embeddings.model`. Different models keyed
    /// separately means their KNN results never pollute each other.
    fn model_id(&self) -> &str;

    /// Embed one text into a [`Self::dim`]-length vector.
    fn embed(&self, text: &str) -> Result<Vec<f32>, EmbedderError>;
}

/// Test-only embedders. Public so other crates' test suites can use
/// them — **never** suitable for production retrieval.
pub mod testing {
    use super::{Digest, Embedder, EmbedderError, Sha256};

    /// Deterministic SHA-256-based embedder. **NOT a semantic model.**
    ///
    /// Hashes the input text into a `dim`-length `f32` vector with
    /// values in `[-1.0, 1.0]`. Two distinct strings always produce
    /// distinct vectors; the same string always produces the same
    /// vector regardless of platform.
    ///
    /// Use only in tests, fixtures, and CI determinism checks.
    /// Production retrieval uses an [`Embedder`] backed by a learned
    /// model — that lands in F1-β.
    pub struct DeterministicTestEmbedder {
        dim: usize,
        model_id: String,
    }

    impl DeterministicTestEmbedder {
        /// Build a test embedder with the given output dimension.
        ///
        /// Tests pick a small dim (8 or 16) to keep fixtures tight.
        /// Panics in debug builds if `dim < 1`.
        pub fn new(dim: usize) -> Self {
            debug_assert!(dim >= 1, "dim must be ≥ 1");
            Self {
                dim,
                model_id: format!("deterministic-test-d{dim}"),
            }
        }
    }

    impl Embedder for DeterministicTestEmbedder {
        fn dim(&self) -> usize {
            self.dim
        }

        fn model_id(&self) -> &str {
            &self.model_id
        }

        fn embed(&self, text: &str) -> Result<Vec<f32>, EmbedderError> {
            // Repeatedly hash to fill `dim` floats. Each round
            // ingests the previous digest so consecutive floats are
            // not correlated.
            let mut out: Vec<f32> = Vec::with_capacity(self.dim);
            let mut seed: Vec<u8> = text.as_bytes().to_vec();
            while out.len() < self.dim {
                let mut hasher = Sha256::new();
                hasher.update(&seed);
                seed = hasher.finalize().to_vec();
                for chunk in seed.chunks_exact(4) {
                    if out.len() == self.dim {
                        break;
                    }
                    let bits = u32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]);
                    let scaled = (bits as f64) / (u32::MAX as f64);
                    out.push((scaled * 2.0 - 1.0) as f32);
                }
            }
            Ok(out)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::testing::DeterministicTestEmbedder;
    use super::Embedder;

    #[test]
    fn deterministic_embedder_returns_correct_dim() {
        let e = DeterministicTestEmbedder::new(8);
        let v = e.embed("hello").expect("embed");
        assert_eq!(v.len(), 8);
        assert_eq!(e.dim(), 8);
    }

    #[test]
    fn deterministic_embedder_is_stable() {
        let e = DeterministicTestEmbedder::new(16);
        let v1 = e.embed("hello").expect("embed");
        let v2 = e.embed("hello").expect("embed");
        assert_eq!(v1, v2);
    }

    #[test]
    fn deterministic_embedder_distinguishes_inputs() {
        let e = DeterministicTestEmbedder::new(16);
        let a = e.embed("alpha").expect("embed");
        let b = e.embed("beta").expect("embed");
        assert_ne!(a, b);
    }

    #[test]
    fn deterministic_embedder_values_in_unit_range() {
        let e = DeterministicTestEmbedder::new(64);
        for f in e.embed("test").expect("embed") {
            assert!((-1.0..=1.0).contains(&f), "value {f} out of [-1, 1]");
        }
    }

    #[test]
    fn distinct_dims_have_distinct_model_ids() {
        let a = DeterministicTestEmbedder::new(8);
        let b = DeterministicTestEmbedder::new(16);
        assert_ne!(a.model_id(), b.model_id());
    }
}
