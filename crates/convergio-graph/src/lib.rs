//! # convergio-graph — Tier-3 code-graph layer
//!
//! Walks the workspace via [`syn`] + [`cargo_metadata`], persists a
//! node/edge graph in SQLite, and exposes queries that drive
//! context-pack delegation, drift detection, and cluster suggestions.
//!
//! See `docs/adr/0014-code-graph-tier3-retrieval.md` for the rationale.
//!
//! ## Layout
//!
//! - [`model`] — [`Node`], [`Edge`], [`NodeKind`], [`EdgeKind`].
//! - [`parse`] — file-level syn walker.
//! - [`meta`] — `cargo metadata` wrapper for crate-level edges.
//! - [`doc_link`] — ADR/markdown frontmatter → claims/mentions edges.
//! - [`store`] — SQLite persistence (migration range 600-699).
//! - [`build`] — top-level orchestrator: meta + parse + doc_link + store.
//! - [`query`] — read-side helpers (`for_task_text`, ...).
//!
//! ## Quickstart
//!
//! ```no_run
//! use convergio_db::Pool;
//! use convergio_graph::{build, Store};
//! use std::path::Path;
//!
//! # async fn run() -> anyhow::Result<()> {
//! let pool = Pool::connect("sqlite://./state.db?mode=rwc").await?;
//! let store = Store::new(pool);
//! let report = build(Path::new("."), &store, false).await?;
//! println!("nodes={} edges={}", report.nodes, report.edges);
//! # Ok(()) }
//! ```

#![forbid(unsafe_code)]

pub mod build;
pub mod cluster;
mod cluster_io;
pub mod doc_link;
pub mod drift;
mod drift_git;
pub mod error;
pub mod meta;
pub mod model;
pub mod parse;
mod parse_use;
pub mod query;
mod query_adrs;
pub mod query_hints;
mod query_match;
pub mod store;
pub mod tokens;

pub use build::build;
pub use cluster::{cluster_for_crate, ClusterReport, Community, DEFAULT_LP_ITERATIONS};
pub use drift::{drift_since, DriftReport};
pub use error::{GraphError, Result};
pub use model::{BuildReport, Edge, EdgeKind, Node, NodeKind, DOCS_CRATE};
pub use query::{
    for_task_text, for_task_text_with_metadata, ContextPack, MatchedFile, MatchedNode, RelatedAdr,
    DEFAULT_NODE_LIMIT, DEFAULT_TOKEN_BUDGET, MAX_NODE_LIMIT, MAX_TOKEN_BUDGET,
};
pub use query_hints::StructuredContextMetadata;
pub use store::Store;
