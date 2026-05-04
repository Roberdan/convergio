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
//!
//! Extracted from `convergio-cli` per ADR-0040 to honour the
//! 11k-line per-crate hard cap (CONSTITUTION § 13). The verifiers
//! are pure (no daemon dependency), so they remain agent-callable
//! from any CLI, skill, or runner.

pub mod coherence;

mod body;
mod body_scan;
mod parse;
mod routes;
mod routes_diff;
mod routes_parse;

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
