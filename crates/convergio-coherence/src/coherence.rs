//! `cvg coherence ...` — local cross-document coherence checks.
//!
//! Walks `docs/adr/`, parses YAML frontmatter, and refuses any of:
//!   (a) referenced ADR id that does not exist on disk
//!   (b) referenced crate name that is not in `workspace.members`
//!   (c) status mismatch between the ADR file and `docs/adr/README.md`
//!   (d) body of any `*.md` mentions a `convergio-X` identifier not in
//!       `workspace.members`, or a path under `crates|docs|scripts|examples|tests/`
//!       that does not exist.
//!
//! Local-only (Check/Routes/Adrs/Agents/Fleet); Handshake and
//! PlanExecution require a running daemon at `--daemon`.

use crate::adrs;
use crate::agents;
use crate::check as check_impl;
use crate::close_post_hoc;
use crate::fleet;
use crate::handshake;
use crate::plan_execution;
use crate::routes;
use crate::OutputMode;
use anyhow::Result;
use clap::Subcommand;
use convergio_i18n::Bundle;
use std::path::PathBuf;

/// Coherence subcommands.
#[derive(Subcommand)]
pub enum CoherenceCommand {
    /// Verify ADR frontmatter against the index and the workspace.
    Check {
        /// Repo root (defaults to cwd).
        #[arg(long, default_value = ".")]
        root: PathBuf,
    },
    /// Diff actual axum routes against `ARCHITECTURE.md` / `AGENTS.md`.
    Routes {
        /// Repo root (defaults to cwd).
        #[arg(long, default_value = ".")]
        root: PathBuf,
    },
    /// Cross-check ADR `status:` frontmatter against implementation.
    Adrs {
        /// Repo root (defaults to cwd).
        #[arg(long, default_value = ".")]
        root: PathBuf,
        /// Exit non-zero on `accepted_no_evidence` and
        /// `broken_supersession` findings (advisory by default).
        #[arg(long, default_value_t = false)]
        strict: bool,
    },
    /// Flag merged PRs whose author skipped the multi-agent protocol
    /// (no `agent_registry` entry, no heartbeat, no coordination
    /// messages on the bus).
    Agents {
        /// Repo root (defaults to cwd).
        #[arg(long, default_value = ".")]
        root: PathBuf,
        /// Window for merged PRs to scan. Accepts `Nd` (days, e.g.
        /// `7d`), `Nh` (hours), or a git revision range (e.g.
        /// `origin/main~50..origin/main`). Defaults to `7d`.
        #[arg(long, default_value = "7d")]
        since: String,
        /// Exit non-zero on `no_registered_agent` and
        /// `no_heartbeat_in_window` findings (advisory by default).
        #[arg(long, default_value_t = false)]
        strict: bool,
        /// Daemon base URL. Empty / unreachable → daemon checks are
        /// skipped (advisory only).
        #[arg(long, default_value = "http://127.0.0.1:8420")]
        daemon: String,
    },
    /// Cross-repo schema check on `~/.convergio/v3/fleet.toml`.
    /// Reports missing paths, missing retrieval-golden fixtures,
    /// dangling `derives_from`, and multiple `engine` roots
    /// (P1-7 / issue #177).
    Fleet {
        /// Path to fleet.toml. Defaults to
        /// `~/.convergio/v3/fleet.toml`.
        #[arg(long)]
        config: Option<PathBuf>,
        /// Exit non-zero on any finding (advisory by default).
        #[arg(long, default_value_t = false)]
        strict: bool,
    },
    /// Surface bypass-the-gate volume: list every
    /// `task.closed_post_hoc` audit row in the window, grouped by
    /// agent + plan, with reasons. Subsumes retrospective finding H5
    /// (P0-4 of the 2026-05-04 fix plan).
    ClosePostHoc {
        /// Window. `Nd` (days) or `Nh` (hours). Default 7d.
        #[arg(long, default_value = "7d")]
        since: String,
        /// Exit non-zero when count > `--threshold`.
        #[arg(long, default_value_t = false)]
        strict: bool,
        /// Strict-mode threshold. Default 0 (any close-post-hoc fails).
        #[arg(long, default_value_t = 0)]
        threshold: usize,
        /// Daemon base URL.
        #[arg(long, default_value = "http://127.0.0.1:8420")]
        daemon: String,
    },
    /// 2-session E2E smoke test: register A+B + heartbeat, A→ping,
    /// B→pong, A receives pong, both ack, both retire. Exits non-zero
    /// if any seam fails or times out (F1 in db812b00 plan).
    Handshake {
        /// Daemon base URL.
        #[arg(long, default_value = "http://127.0.0.1:8420")]
        daemon: String,
        /// Per-phase timeout in seconds. Default 5.
        #[arg(long, default_value_t = 5)]
        timeout_seconds: u64,
    },
    /// Per-plan mechanism compliance score (ADR-0044). Reports whether
    /// every closed task attached the required evidence kinds.
    PlanExecution {
        /// Plan id to score.
        plan_id: String,
        /// Daemon base URL.
        #[arg(long, default_value = "http://127.0.0.1:8420")]
        daemon: String,
        /// Exit non-zero when compliance < 100% or plan-level checks fail.
        #[arg(long, default_value_t = false)]
        strict: bool,
    },
}

/// Entry point.
pub async fn run(bundle: &Bundle, output: OutputMode, cmd: CoherenceCommand) -> Result<()> {
    match cmd {
        CoherenceCommand::Check { root } => check(output, &root).await,
        CoherenceCommand::Routes { root } => routes::run(bundle, output, &root).await,
        CoherenceCommand::Adrs { root, strict } => adrs::run(bundle, output, &root, strict).await,
        CoherenceCommand::Agents {
            root,
            since,
            strict,
            daemon,
        } => agents::run(bundle, output, &root, &since, strict, &daemon).await,
        CoherenceCommand::ClosePostHoc {
            since,
            strict,
            threshold,
            daemon,
        } => close_post_hoc::run(bundle, output, &daemon, &since, strict, threshold).await,
        CoherenceCommand::Fleet { config, strict } => {
            fleet::run(bundle, output, config, strict).await
        }
        CoherenceCommand::Handshake {
            daemon,
            timeout_seconds,
        } => handshake::run(bundle, output, &daemon, timeout_seconds).await,
        CoherenceCommand::PlanExecution {
            plan_id,
            daemon,
            strict,
        } => plan_execution::run(bundle, output, &daemon, &plan_id, strict).await,
    }
}

async fn check(output: OutputMode, root: &std::path::Path) -> Result<()> {
    let report = check_impl::run_check(root)?;
    match output {
        OutputMode::Json => println!("{}", serde_json::to_string_pretty(&report)?),
        OutputMode::Plain => check_impl::render_plain(&report),
        OutputMode::Human => check_impl::render_human(&report),
    }
    if report.violations.is_empty() {
        Ok(())
    } else {
        std::process::exit(1)
    }
}
