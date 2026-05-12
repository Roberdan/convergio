//! `claude -p` runner.
//!
//! Reads the prompt from stdin (`--input-format text`) so very long
//! prompts (graph context-pack can be 30+ KB) survive argv limits.

use crate::cargo_env;
use crate::command::PreparedCommand;
use crate::error::Result;
use crate::profile::PermissionProfile;
use crate::prompt::{self, PromptInputs};
use crate::runner::{Runner, SpawnContext};
use std::ffi::OsString;
use std::path::PathBuf;

/// Wraps `claude -p ... --model X --output-format stream-json`.
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
            env: cargo_env::env_for(ctx.cwd),
            stdin_prompt: prompt,
        })
    }
}
