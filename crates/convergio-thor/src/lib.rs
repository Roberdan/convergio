//! # convergio-thor — Layer 4 (basic)
//!
//! Validator agent. Reads a plan + its tasks + their evidence and
//! returns a verdict before the plan is closed. Reference
//! implementation — replace with your own validator if your domain
//! (healthcare, finance, ...) needs custom checks.
//!
//! ## MVP rules
//!
//! - **Pass** iff every task in the plan has `status = submitted` or
//!   `status = done` AND every kind in `evidence_required` has at
//!   least one matching evidence row. On a passing verdict, every
//!   still-`submitted` task is atomically promoted to `done` through
//!   `Durability::complete_validated_tasks` (CONSTITUTION §6,
//!   ADR-0011) — Thor is the **only** path to `done`.
//! - **Fail** otherwise; the verdict carries a `Vec<String>` of
//!   reasons (one per failing task).
//!
//! Thor is intentionally simple. The point is that **it's separate
//! from the executor** — the same code that ran the work is not the
//! one that signs off on it.
//!
//! ## Quickstart
//!
//! ```no_run
//! use convergio_db::Pool;
//! use convergio_durability::{init, Durability};
//! use convergio_thor::{Thor, Verdict};
//!
//! # async fn run() -> Result<(), Box<dyn std::error::Error>> {
//! let pool = Pool::connect("sqlite://./state.db").await?;
//! init(&pool).await?;
//! let dur = Durability::new(pool);
//! let thor = Thor::new(dur);
//! match thor.validate("plan-uuid").await? {
//!     Verdict::Pass => println!("safe to close"),
//!     Verdict::Fail { reasons } => println!("nope: {reasons:?}"),
//! }
//! # Ok(()) }
//! ```

#![forbid(unsafe_code)]

mod error;
mod pipeline;
mod steps;
mod thor;

pub use error::{Result, ThorError};
pub use thor::{Thor, Verdict, DEFAULT_PIPELINE_TIMEOUT_SECS, PIPELINE_ENV};
