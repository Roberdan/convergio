//! # convergio-fleet
//!
//! Fleet abstraction for Convergio v4 (ADR-0038, F2).
//!
//! Owns:
//! - [`config`]  — `fleet.toml` schema (ADR-0038 § 5.6)
//! - [`store`]   — `fleet_repos`, `fleet_plans`, `fleet_plan_repos` DB layer
//! - [`similar`] — cross-repo similarity edge store (ADR-0038, F2-7)
//! - [`migrate`] — migration runner (range 800-899, ADR-0003)
//!
//! ## Architecture
//!
//! | Module | Responsibility |
//! |--------|----------------|
//! | [`config`]  | Typed `fleet.toml` structs — deserialize/serialize only |
//! | [`store`]   | [`FleetStore`] — CRUD over `fleet_repos` and fleet plans |
//! | [`similar`] | Cross-repo similarity edges on [`FleetStore`] |
//! | [`migrate`] | Run pending migrations (idempotent) |
//!
//! ## Quickstart
//!
//! ```no_run
//! # async fn run() -> Result<(), Box<dyn std::error::Error>> {
//! use convergio_db::Pool;
//! use convergio_fleet::{init, FleetStore};
//! use convergio_fleet::config::{FleetConfig, RepoEntry, RepoRole};
//!
//! let pool = Pool::connect("sqlite://./state.db").await?;
//! init(&pool).await?;
//!
//! let store = FleetStore::new(pool);
//! store.add_repo(&RepoEntry {
//!     name: "convergio".into(),
//!     path: "/repos/convergio".into(),
//!     language: "rust".into(),
//!     parser: "syn".into(),
//!     role: RepoRole::Engine,
//!     derives_from: None,
//! }).await?;
//!
//! let repos = store.list_repos().await?;
//! println!("{} repos in fleet", repos.len());
//! # Ok(()) }
//! ```

#![forbid(unsafe_code)]

pub mod batch;
pub mod config;
pub mod error;
pub mod migrate;
pub mod similar;
pub mod store;

pub use batch::{run_similarity_batch, BatchReport};
pub use config::{FleetConfig, FleetSection, RepoEntry, RepoRole, RetrievalSection};
pub use error::{FleetError, Result};
pub use migrate::init;
pub use similar::{SimilarEdge, DUPLICATES_THRESHOLD, SIMILAR_TO_THRESHOLD};
pub use store::{FleetRepo, FleetStore};
