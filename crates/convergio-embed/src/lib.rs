//! # convergio-embed
//!
//! Embeddings storage and pluggable embedder trait for Convergio's
//! Tier-3 retrieval (ADR-0038, F1).
//!
//! This crate is the **storage and policy** seam. By default it ships
//! only [`embedder::testing::DeterministicTestEmbedder`]. Enable the
//! `fastembed` feature for real ONNX models via `fastembed-rs`, which
//! are downloaded lazily into `~/.convergio/v3/models/`.
//!
//! ## Architecture
//!
//! | Module | Responsibility |
//! |--------|----------------|
//! | [`embedder`] | [`Embedder`] trait + deterministic test embedder |
//! | [`source`]   | Build canonical embeddable text + SHA-256 hash |
//! | [`select`]   | [`EmbedPolicy`] decides which targets get embedded |
//! | [`store`]    | SQLite persistence + brute-force cosine KNN |
//! | [`migrate`]  | Migration runner (range 700-799, ADR-0003) |
//!
//! ## Quickstart (deterministic, model-free)
//!
//! ```no_run
//! # async fn run() -> Result<(), Box<dyn std::error::Error>> {
//! use convergio_db::Pool;
//! use convergio_embed::{init, EmbedStore, Embedder, SourceText};
//! use convergio_embed::embedder::testing::DeterministicTestEmbedder;
//!
//! let pool = Pool::connect("sqlite://./state.db").await?;
//! init(&pool).await?;
//! let store = EmbedStore::new(pool);
//! let embedder = DeterministicTestEmbedder::new(8);
//!
//! let text = SourceText::new("the embedded text");
//! if store
//!     .needs_reembed("convergio", "node-42", embedder.model_id(), &text.source_hash)
//!     .await?
//! {
//!     let v = embedder.embed(&text.text)?;
//!     store
//!         .upsert("convergio", "node-42", embedder.model_id(), &v, &text.source_hash)
//!         .await?;
//! }
//! # Ok(()) }
//! ```

#![forbid(unsafe_code)]
#![deny(missing_docs)]

mod codec;

/// Corpus helpers for building [`IngestNode`] inputs from the filesystem.
pub mod corpus;
/// The [`Embedder`] trait and reference test embedder implementations.
pub mod embedder;
/// Error types and the crate-wide [`Result`] alias.
pub mod error;
/// Hybrid fusion utilities (RRF / linear blend) for structural + semantic retrieval.
pub mod hybrid;
/// Ingest pipeline: embed text and persist vectors in the store.
pub mod ingest;
/// Migration runner for the embed store tables.
pub mod migrate;
/// Semantic query helpers over the embedding store.
pub mod query;
/// Selective embedding policy (`EmbedPolicy` / `EmbedTarget`).
pub mod select;
/// Canonical source text builder + SHA-256 hashing (`SourceText`).
pub mod source;
/// SQLite-backed embedding persistence (`EmbedStore`).
pub mod store;

/// `fastembed-rs`-backed real-model embedder implementations (feature-gated).
#[cfg(feature = "fastembed")]
pub mod fastembed_impl;

/// Re-export: filesystem corpus collector + defaults.
pub use corpus::{
    collect_files, collect_files_report, CorpusReport, DEFAULT_MAX_LINES, SOURCE_EXTENSIONS,
};
/// Re-export: embedder trait + embedder-level errors.
pub use embedder::{Embedder, EmbedderError};
/// Re-export: crate error types.
pub use error::{EmbedError, Result};
/// Re-export: hybrid fusion types and functions.
pub use hybrid::{
    linear_blend_fuse, rrf_fuse, MatchSource, RetrievalHit, ScoreComponents, DEFAULT_LINEAR_ALPHA,
    DEFAULT_RRF_K,
};
/// Re-export: ingest entry points + report types.
pub use ingest::{ingest, ingest_one, IngestNode, IngestReport};
/// Re-export: migration runner.
pub use migrate::init;
/// Re-export: semantic-only search helper.
pub use query::semantic_search;
/// Re-export: embedding selection policy.
pub use select::{EmbedPolicy, EmbedTarget};
/// Re-export: canonical source text + hash wrapper.
pub use source::SourceText;
/// Re-export: store handle + row/hit types.
pub use store::{EmbedStore, EmbeddingRow, Neighbor};

/// Re-export: `fastembed` real-model embedder (feature-gated).
#[cfg(feature = "fastembed")]
pub use fastembed_impl::{BgeM3Embedder, MultilingualE5Embedder};
