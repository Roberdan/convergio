//! `openai-cli` runner (W7, ADR-0053).
//!
//! Wraps an OpenAI-compatible vendor CLI. The default binary is
//! `openai-cli` but operators can point at any compatible binary via
//! `OPENAI_CLI_BIN` (e.g. `codex`, `o3-cli`) without rebuilding
//! Convergio. The prompt is delivered on stdin to survive long graph
//! context-packs, mirroring [`ClaudeRunner`].
//!
//! Per ADR-0032 this runner **shells out to a local vendor CLI** —
//! it never opens a direct connection to `api.openai.com`. The vendor
//! CLI owns the operator's auth + cost surface.
//!
//! Permission profiles are forwarded to the CLI as a single
//! `--permission-mode` argument. The exact flag names vary across
//! third-party CLIs; the runner only emits widely-compatible flags
//! (`-p`, `--model`, `--permission-mode`) and lets the operator
//! configure anything more exotic through a registry entry
//! (ADR-0035).

use crate::cargo_env;
use crate::command::PreparedCommand;
use crate::error::Result;
use crate::profile::PermissionProfile;
use crate::prompt::{self, PromptInputs};
use crate::runner::{Runner, SpawnContext};
use std::ffi::OsString;
use std::path::PathBuf;

/// Env var that overrides the default `openai-cli` binary name.
pub const OPENAI_CLI_BIN_ENV: &str = "OPENAI_CLI_BIN";

/// Default binary name when `OPENAI_CLI_BIN` is unset.
pub const DEFAULT_OPENAI_CLI: &str = "openai-cli";

/// Wraps `openai-cli -p --model X` (or compatible binary).
pub struct OpenaiRunner {
    /// `--model` value (e.g. `gpt-4.1`, `o4-mini`). Forwarded as-is
    /// so new model names surface without a Convergio release.
    pub model: String,
}

impl Runner for OpenaiRunner {
    fn prepare(&self, ctx: &SpawnContext<'_>) -> Result<PreparedCommand> {
        let prompt = prompt::build(&PromptInputs {
            task: ctx.task,
            plan_id: ctx.plan_id,
            plan_title: ctx.plan_title,
            daemon_url: ctx.daemon_url,
            agent_id: ctx.agent_id,
            graph_context: ctx.graph_context,
        });

        let mut args: Vec<OsString> =
            vec!["-p".into(), "--model".into(), self.model.clone().into()];
        // Mode flag mirrors Claude's `--permission-mode`. Sandbox
        // keeps the vendor CLI's permissive default; everything else
        // requests an explicit mode the vendor CLI is expected to
        // honour. Operators who need vendor-specific flags should
        // register the binary through the registry (ADR-0035)
        // instead of expanding this enum.
        match ctx.profile {
            PermissionProfile::Sandbox => {}
            other => {
                args.push("--permission-mode".into());
                args.push(other.claude_permission_mode().into());
            }
        }
        if let Some(b) = ctx.max_budget_usd {
            args.push("--max-budget-usd".into());
            args.push(format!("{b}").into());
        }

        let program = std::env::var_os(OPENAI_CLI_BIN_ENV)
            .filter(|v| !v.is_empty())
            .unwrap_or_else(|| OsString::from(DEFAULT_OPENAI_CLI));

        Ok(PreparedCommand {
            program,
            args,
            cwd: PathBuf::from(ctx.cwd),
            env: cargo_env::env_for(ctx.cwd),
            stdin_prompt: prompt,
        })
    }
}
