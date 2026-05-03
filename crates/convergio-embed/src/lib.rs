//! # convergio-embed
//!
//! Embeddings storage and pluggable embedder trait for Convergio's
//! Tier-3 retrieval (ADR-0038, F1).
//!
//! This crate is the **storage and policy** seam. It does not bundle
//! a model — F1-α ships only [`embedder::testing::DeterministicTestEmbedder`];
//! a real `fastembed-rs`-backed implementation lands in F1-β alongside
//! the `sqlite-vec` virtual-table swap.
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

mod codec;

pub mod corpus;
pub mod embedder;
pub mod error;
pub mod ingest;
pub mod migrate;
pub mod query;
pub mod select;
pub mod source;
pub mod store;

#[cfg(feature = "fastembed")]
pub mod fastembed_impl;

pub use corpus::{collect_files, DEFAULT_MAX_LINES, SOURCE_EXTENSIONS};
pub use embedder::{Embedder, EmbedderError};
pub use error::{EmbedError, Result};
pub use ingest::{ingest, ingest_one, IngestNode, IngestReport};
pub use migrate::init;
pub use query::semantic_search;
pub use select::{EmbedPolicy, EmbedTarget};
pub use source::SourceText;
pub use store::{EmbedStore, EmbeddingRow, Neighbor};

#[cfg(feature = "fastembed")]
pub use fastembed_impl::MultilingualE5Embedder;
