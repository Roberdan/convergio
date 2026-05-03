//! # convergio-planner — Layer 4 (Opus-backed)
//!
//! Turns a natural-language mission into a plan + tasks stored via
//! Layer 1.
//!
//! ## Two backends
//!
//! - **Opus (default)** — spawns `claude -p --model opus
//!   --output-format json --permission-mode plan` (vendor CLI only,
//!   ADR-0032). The model returns a JSON plan with `runner_kind`,
//!   `profile`, `wave`, `sequence` and `evidence_required` set per
//!   task; the planner persists it through `convergio-durability`.
//!   ADR-0036.
//! - **Heuristic** — deterministic line-split. One task per
//!   non-blank line, all wave 1. Used as the fallback when `claude`
//!   is not on `PATH` and as the explicit choice when
//!   `CONVERGIO_PLANNER_MODE=heuristic` (CI, unit tests).
//!
//! Mode selection: `CONVERGIO_PLANNER_MODE` env var
//! (`auto` (default) | `opus` | `heuristic`).
//!
//! ## Quickstart
//!
//! ```no_run
//! use convergio_db::Pool;
//! use convergio_durability::{init, Durability};
//! use convergio_planner::{Planner, PlannerMode};
//!
//! # async fn run() -> Result<(), Box<dyn std::error::Error>> {
//! let pool = Pool::connect("sqlite://./state.db").await?;
//! init(&pool).await?;
//! let dur = Durability::new(pool);
//! let planner = Planner::new(dur).with_mode(PlannerMode::Heuristic);
//! let plan_id = planner
//!     .solve("ship the mvp\nwrite docs\nopen the source")
//!     .await?;
//! # Ok(()) }
//! ```

#![forbid(unsafe_code)]

mod error;
mod heuristic;
pub mod opus;
mod planner;
pub mod schema;

pub use error::{PlannerError, Result};
pub use planner::{Planner, PlannerMode};
pub use schema::{PlanShape, TaskShape};
