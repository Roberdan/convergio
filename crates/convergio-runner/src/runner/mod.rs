//! `Runner` trait + dispatch to the two vendor implementations.
//!
//! Each runner is a *pure* preparer: given a [`SpawnContext`] it
//! returns a [`PreparedCommand`]. The actual subprocess lifecycle
//! (spawn, supervise, reap) is the executor's concern.
//!
//! The vendor-specific argv shaping lives in two submodules:
//! [`claude`] and [`copilot`]. Keeping this dispatch file slim is
//! intentional — see the 2026-05-12 audit of `convergio-runner`
//! (low-severity refactor finding: original `runner.rs` was 292
//! lines and mixed dispatch, Claude argv, Copilot argv, and PATH
//! checks near the 300-line cap).

mod claude;
mod copilot;
mod openai;

pub use claude::ClaudeRunner;
pub use copilot::CopilotRunner;
pub use openai::{OpenaiRunner, DEFAULT_OPENAI_CLI, OPENAI_CLI_BIN_ENV};

use crate::command::PreparedCommand;
use crate::error::{Result, RunnerError};
use crate::kind::{Family, RunnerKind};
use crate::profile::PermissionProfile;
use crate::registry::RunnerRegistry;
use crate::runner_config::ConfigRunner;
use convergio_durability::Task;
use std::path::Path;

/// Everything a runner needs to assemble its command + prompt.
pub struct SpawnContext<'a> {
    /// The task to be worked on.
    pub task: &'a Task,
    /// Plan id this task belongs to.
    pub plan_id: &'a str,
    /// Plan title.
    pub plan_title: &'a str,
    /// Daemon HTTP base URL the agent will hit for state changes.
    pub daemon_url: &'a str,
    /// Stable agent identity to register under.
    pub agent_id: &'a str,
    /// Optional graph context (`convergio_graph::for_task_text`).
    pub graph_context: Option<&'a str>,
    /// Working directory — always a worktree under
    /// `.claude/worktrees/<branch>/`.
    pub cwd: &'a Path,
    /// Per-session budget cap (USD). Forwarded to `claude`'s
    /// `--max-budget-usd`. Ignored by Copilot (no equivalent flag).
    pub max_budget_usd: Option<f32>,
    /// Permission envelope (ADR-0033). Each runner translates this
    /// into vendor-specific flags so the spawned agent runs with
    /// least privilege rather than `--dangerously-skip-permissions`
    /// / `--allow-all-tools`.
    pub profile: PermissionProfile,
}

/// One runner == one vendor CLI wrapping.
pub trait Runner {
    /// Build the [`PreparedCommand`] for `ctx`. Pure: does not run
    /// the binary, does not touch the filesystem, does not call HTTP.
    fn prepare(&self, ctx: &SpawnContext<'_>) -> Result<PreparedCommand>;
}

/// Pick a concrete runner for `kind`. Built-in vendors only.
///
/// Custom vendors require a registry — call
/// [`for_kind_with_registry`] instead. Kept as the simple entry
/// point for tests and tools that never load the operator's
/// `runners.toml`.
pub fn for_kind(kind: &RunnerKind) -> Result<Box<dyn Runner>> {
    for_kind_with_registry(kind, &RunnerRegistry::empty())
}

/// Pick a concrete runner for `kind`, consulting `registry` for
/// vendors that aren't built-in. ADR-0035.
pub fn for_kind_with_registry(
    kind: &RunnerKind,
    registry: &RunnerRegistry,
) -> Result<Box<dyn Runner>> {
    if let Some(family) = kind.family() {
        return Ok(match family {
            Family::Claude => Box::new(ClaudeRunner {
                model: kind.model.clone(),
            }),
            Family::Copilot => Box::new(CopilotRunner {
                model: kind.model.clone(),
            }),
            Family::Openai => Box::new(OpenaiRunner {
                model: kind.model.clone(),
            }),
        });
    }
    let spec = registry
        .get(&kind.vendor)
        .ok_or_else(|| RunnerError::UnknownVendor {
            vendor: kind.vendor.clone(),
        })?
        .clone();
    let cfg = ConfigRunner::try_new(&kind.vendor, spec, &kind.model)?;
    Ok(Box::new(cfg))
}

/// Convenience: surface a clear error when the vendor CLI is not
/// on `PATH`. Callers may invoke this before `prepare` to fail fast.
pub fn assert_cli_on_path(family: Family) -> Result<()> {
    let cli = family.cli();
    let found = std::env::var_os("PATH")
        .map(|paths| {
            std::env::split_paths(&paths).any(|p| {
                let candidate = p.join(cli);
                candidate.is_file() || candidate.with_extension("exe").is_file()
            })
        })
        .unwrap_or(false);
    if found {
        Ok(())
    } else {
        Err(RunnerError::CliMissing { cli })
    }
}

#[cfg(test)]
mod tests {
    // The full argv-shape suite lives in
    // `crates/convergio-runner/tests/runner_argv.rs`. Only
    // smoke-level type checks belong here.

    use super::*;

    #[test]
    fn for_kind_returns_a_dyn_runner_for_each_family() {
        // Compilation-level coverage: the dispatch surface
        // resolves both vendors without panicking.
        for_kind(&RunnerKind::claude_sonnet()).unwrap();
        for_kind(&RunnerKind::copilot_gpt()).unwrap();
        for_kind(&RunnerKind::openai_gpt()).unwrap();
    }

    #[test]
    fn for_kind_rejects_unknown_vendor_without_registry() {
        let kind: RunnerKind = "qwen:qwen3-coder".parse().unwrap();
        let err = match for_kind(&kind) {
            Ok(_) => panic!("expected UnknownVendor error"),
            Err(e) => e,
        };
        assert!(matches!(err, RunnerError::UnknownVendor { .. }));
    }
}
