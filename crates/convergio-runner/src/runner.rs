//! `Runner` trait + the two vendor implementations.
//!
//! Each runner is a *pure* preparer: given a [`SpawnContext`] it
//! returns a [`PreparedCommand`]. The actual subprocess lifecycle
//! (spawn, supervise, reap) is the executor's concern.

use crate::command::PreparedCommand;
use crate::error::{Result, RunnerError};
use crate::kind::{Family, RunnerKind};
use crate::profile::PermissionProfile;
use crate::prompt::{self, PromptInputs};
use crate::registry::RunnerRegistry;
use crate::runner_config::ConfigRunner;
use convergio_durability::Task;
use std::ffi::OsString;
use std::path::{Path, PathBuf};

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

/// Wraps `claude -p ... --model X --output-format json`.
///
/// Reads the prompt from stdin (`--input-format text`) so very long
/// prompts (graph context-pack can be 30+ KB) survive argv limits.
pub struct ClaudeRunner {
    /// `--model` value.
    pub model: String,
}

impl Runner for ClaudeRunner {
    fn prepare(&self, ctx: &SpawnContext<'_>) -> Result<PreparedCommand> {
        let prompt = prompt::build(&PromptInputs {
            task: ctx.task,
            plan_id: ctx.plan_id,
            plan_title: ctx.plan_title,
            daemon_url: ctx.daemon_url,
            agent_id: ctx.agent_id,
            graph_context: ctx.graph_context,
        });
        // ADR-0033: only `Sandbox` keeps the legacy
        // `--dangerously-skip-permissions`. `Standard` and
        // `ReadOnly` use `--permission-mode` + an explicit
        // `--allowed-tools` whitelist (least privilege).
        let mut args: Vec<OsString> = Vec::new();
        match ctx.profile {
            PermissionProfile::Sandbox => {
                args.push("--dangerously-skip-permissions".into());
            }
            other => {
                args.push("--permission-mode".into());
                args.push(other.claude_permission_mode().into());
                if let Some(allowed) = other.claude_allowed_tools() {
                    args.push("--allowed-tools".into());
                    args.push(allowed.into());
                }
            }
        }
        // stream-json + verbose so the executor can pipe each
        // assistant turn / tool_use to the operator in real time
        // (`--output-format json` buffers the whole run).
        args.extend([
            "-p".into(),
            "--model".into(),
            self.model.clone().into(),
            "--output-format".into(),
            "stream-json".into(),
            "--verbose".into(),
            "--input-format".into(),
            "text".into(),
        ]);
        if let Some(b) = ctx.max_budget_usd {
            args.push("--max-budget-usd".into());
            args.push(format!("{b}").into());
        }
        Ok(PreparedCommand {
            program: OsString::from("claude"),
            args,
            cwd: PathBuf::from(ctx.cwd),
            stdin_prompt: prompt,
        })
    }
}

/// Wraps `copilot -p ... --model X --allow-all-tools`.
///
/// `--allow-all-tools` is required by the Copilot CLI for any
/// non-interactive run (the equivalent of "this script accepts all
/// tool consents in advance"). Convergio's worktree boundary plus
/// the daemon's audit chain are the actual safety net.
pub struct CopilotRunner {
    /// `--model` value.
    pub model: String,
}

impl Runner for CopilotRunner {
    fn prepare(&self, ctx: &SpawnContext<'_>) -> Result<PreparedCommand> {
        let prompt = prompt::build(&PromptInputs {
            task: ctx.task,
            plan_id: ctx.plan_id,
            plan_title: ctx.plan_title,
            daemon_url: ctx.daemon_url,
            agent_id: ctx.agent_id,
            graph_context: ctx.graph_context,
        });
        let mut args: Vec<OsString> = vec![
            "-p".into(),
            prompt.clone().into(),
            "--model".into(),
            self.model.clone().into(),
        ];
        // ADR-0033: replace `--allow-all-tools` with a per-tool
        // whitelist + an always-on deny list for destructive
        // commands. Sandbox keeps the nuke for sealed environments.
        match ctx.profile {
            PermissionProfile::Sandbox => {
                args.push("--allow-all".into());
            }
            other => {
                for pat in other.copilot_allow_tools() {
                    args.push("--allow-tool".into());
                    args.push(pat.into());
                }
                for pat in PermissionProfile::Standard.copilot_deny_tools() {
                    args.push("--deny-tool".into());
                    args.push(pat.into());
                }
                args.push("--add-dir".into());
                args.push(ctx.cwd.as_os_str().to_owned());
            }
        }
        Ok(PreparedCommand {
            program: OsString::from("copilot"),
            args,
            cwd: PathBuf::from(ctx.cwd),
            stdin_prompt: prompt,
        })
    }
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
    // `crates/convergio-runner/tests/runner_argv.rs` to keep this
    // file under the 300-line cap. Only smoke-level type checks
    // belong here.

    use super::*;

    #[test]
    fn for_kind_returns_a_dyn_runner_for_each_family() {
        // Compilation-level coverage: the dispatch surface
        // resolves both vendors without panicking.
        for_kind(&RunnerKind::claude_sonnet()).unwrap();
        for_kind(&RunnerKind::copilot_gpt()).unwrap();
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
