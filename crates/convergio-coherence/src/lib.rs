//! Documentation/code coherence verifiers for Convergio.
//!
//! This crate hosts the `cvg coherence` suite — local-only checks
//! that the repository's docs and code agree:
//!
//! - [`coherence::run`] with [`coherence::CoherenceCommand::Check`] —
//!   ADR frontmatter, workspace membership, ADR index status, and
//!   markdown body drift (unknown `convergio-*` identifiers, missing
//!   repo-relative paths).
//! - [`coherence::run`] with [`coherence::CoherenceCommand::Routes`] —
//!   diff actual axum route declarations under
//!   `crates/convergio-server/src/routes/` against the documented
//!   surface in `ARCHITECTURE.md` / `AGENTS.md`.
//! - [`coherence::run`] with [`coherence::CoherenceCommand::PlanExecution`] —
//!   per-plan mechanism compliance score (ADR-0044).
//!
//! Extracted from `convergio-cli` per ADR-0040 to honour the
//! 11k-line per-crate hard cap (CONSTITUTION § 13). Most verifiers
//! are pure (no daemon dependency), so they remain agent-callable
//! from any CLI, skill, or runner. Exceptions: `Agents`,
//! `ClosePostHoc`, `Handshake`, and `PlanExecution` require a
//! running daemon.

pub mod coherence;

mod adrs;
mod adrs_scan;
#[cfg(test)]
mod adrs_tests;
mod agents;
mod agents_judge;
mod agents_parse;
mod agents_scan;
#[cfg(test)]
mod agents_tests;
mod body;
mod body_scan;
pub(crate) mod check;
pub mod close_post_hoc;
mod close_post_hoc_scan;
pub mod fleet;
#[cfg(test)]
mod fleet_tests;
pub mod handshake;
mod handshake_http;
mod handshake_render;
mod handshake_run;
#[cfg(test)]
mod handshake_tests;
mod parse;
pub mod plan_execution;
mod plan_execution_scan;
#[cfg(test)]
mod plan_execution_tests;
mod routes;
mod routes_diff;
mod routes_parse;
#[cfg(test)]
mod routes_parse_tests;

pub use coherence::{run, CoherenceCommand};

/// Output rendering mode for coherence verifiers.
///
/// Mirrors `convergio_cli::commands::OutputMode` so this crate has no
/// dependency back on the CLI. The CLI's enum is converted at the
/// shim boundary.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OutputMode {
    /// Localized human-readable output.
    Human,
    /// Pretty JSON for scripts and agents.
    Json,
    /// Minimal plain text for shell pipelines.
    Plain,
}
