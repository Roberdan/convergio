//! `copilot -p` runner.
//!
//! Copilot CLI today only takes the prompt on argv. When stdin
//! support lands, switch symmetrically to the Claude shape.

use crate::cargo_env;
use crate::command::PreparedCommand;
use crate::error::Result;
use crate::profile::PermissionProfile;
use crate::prompt::{self, PromptInputs};
use crate::runner::{Runner, SpawnContext};
use std::ffi::OsString;
use std::path::PathBuf;

/// Wraps `copilot -p ... --model X`.
///
/// Convergio's worktree boundary plus the daemon's audit chain are
/// the actual safety net behind the Copilot permission flags.
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
        // ADR-0033 + 2026-05-12 audit W1-E: the destructive-command
        // deny-list is documented as "always applied" by
        // `PermissionProfile::copilot_deny_tools`. Pre-W1-E, Sandbox
        // and Unrestricted skipped it, which contradicted the doc
        // and weakened the security-first principle. Now every
        // profile emits the deny patterns; only the allow shape
        // differs.
        match ctx.profile {
            PermissionProfile::Sandbox => {
                args.push("--allow-all".into());
                for pat in PermissionProfile::Standard.copilot_deny_tools() {
                    args.push("--deny-tool".into());
                    args.push(pat.into());
                }
            }
            PermissionProfile::Unrestricted => {
                // `--allow-all` disables Copilot's own confirmation
                // gates so wrapped vendor commands (`git --no-pager
                // status`, etc.) go through. Daemon-side deny-list
                // remains the actual containment.
                args.push("--allow-all".into());
                args.push("--add-dir".into());
                args.push(ctx.cwd.as_os_str().to_owned());
                for pat in PermissionProfile::Standard.copilot_deny_tools() {
                    args.push("--deny-tool".into());
                    args.push(pat.into());
                }
            }
            other => {
                // Copilot CLI in `-p` mode requires `--allow-all-tools`
                // for any tool that would normally prompt for operator
                // confirmation — granular `--allow-tool` only PRE-
                // confirms (it doesn't bypass the confirmation gate in
                // non-interactive). Without this, agents could `write`
                // their report file but every `shell(...)` call (git
                // add / commit / push, gh pr create, curl) returned
                // "Permission denied and could not request permission
                // from user" — surfaced explicitly in the audit plan's
                // bus tail on 2026-05-11.
                //
                // Containment now relies on `--deny-tool` (rm/sudo/
                // push-to-main/force-push/reset --hard/curl-with-data)
                // plus `--add-dir <worktree>` keeping file writes
                // inside the agent's branch.
                args.push("--allow-all-tools".into());
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
            env: cargo_env::env_for(ctx.cwd),
            stdin_prompt: prompt,
        })
    }
}
