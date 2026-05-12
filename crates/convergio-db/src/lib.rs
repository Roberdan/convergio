//! # convergio-db
//!
//! SQLite database pool for the local Convergio runtime.
//!
//! Convergio is intentionally local-first: one daemon, one user, one
//! SQLite database file. Higher layers (`convergio-durability`,
//! `convergio-bus`, `convergio-lifecycle`) depend on this crate for
//! connection ownership via [`Pool`]; they still use `sqlx` directly
//! for query execution and for running their own per-crate migrations
//! (see ADR-0003 for the migration-version partitioning).
//!
//! ## Database URL
//!
//! [`Pool::connect`] accepts only `sqlite://` URLs. The server defaults
//! to `sqlite://$HOME/.convergio/v3/state.db?mode=rwc`.
//!
//! ## Example
//!
//! ```no_run
//! use convergio_db::Pool;
//!
//! # async fn run() -> Result<(), Box<dyn std::error::Error>> {
//! let pool = Pool::connect("sqlite://./state.db").await?;
//! // pass `pool` to the higher-layer stores
//! # Ok(())
//! # }
//! ```

// NOTE: Most Convergio crates forbid `unsafe_code`. `convergio-db` is the
// one exception because SQLite extension registration (`sqlite3_auto_extension`)
// requires a tiny, well-contained FFI call.
#![deny(unsafe_op_in_unsafe_fn)]

mod error;
mod pool;

pub use error::{DbError, Result};
pub use pool::{Backend, Pool};
