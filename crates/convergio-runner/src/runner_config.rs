//! `ConfigRunner` — registry-driven runner for vendors declared in
//! `~/.convergio/runners.toml`. ADR-0035.
//!
//! Built on the same `Runner` contract as `ClaudeRunner` /
//! `CopilotRunner`: pure preparation, no spawn, no network. The
//! argv shape comes from a [`RunnerSpec`] supplied by the registry.

use crate::command::PreparedCommand;
use crate::error::{Result, RunnerError};
use crate::prompt::{self, PromptInputs};
use crate::registry::{PromptVia, RunnerSpec};
use crate::runner::{Runner, SpawnContext};
use std::ffi::OsString;
use std::path::PathBuf;

/// A runner whose argv shape is read from a [`RunnerSpec`].
///
/// One `ConfigRunner` is built per dispatch: it owns the spec
/// (cloned from the registry) plus the model the planner picked.
#[derive(Debug)]
pub struct ConfigRunner {
    /// Vendor tag — used only for error messages.
    pub vendor: String,
    /// The TOML-loaded spec.
    pub spec: RunnerSpec,
    /// The model the planner picked.
    pub model: String,
}

impl ConfigRunner {
    /// Validate the model against the spec's allowlist and build
    /// the runner. Empty allowlist accepts any model.
    pub fn try_new(vendor: &str, spec: RunnerSpec, model: &str) -> Result<Self> {
        if !spec.models.is_empty() && !spec.models.iter().any(|m| m == model) {
            return Err(RunnerError::UnknownModel {
                vendor: vendor.to_string(),
                model: model.to_string(),
                allowed: spec.models.clone(),
            });
        }
        Ok(Self {
            vendor: vendor.to_string(),
            spec,
            model: model.to_string(),
        })
    }
}

impl Runner for ConfigRunner {
    fn prepare(&self, ctx: &SpawnContext<'_>) -> Result<PreparedCommand> {
        let prompt = prompt::build(&PromptInputs {
            task: ctx.task,
            plan_id: ctx.plan_id,
            plan_title: ctx.plan_title,
            daemon_url: ctx.daemon_url,
            agent_id: ctx.agent_id,
            graph_context: ctx.graph_context,
        });

        let mut args: Vec<OsString> = Vec::new();

        // Permission profile flags first (the agent's leash).
        let profile_key = ctx.profile.tag();
        if let Some(p) = self.spec.profiles.get(profile_key) {
            args.extend(p.args.iter().map(OsString::from));
        }

        // --model <model>
        args.push(self.spec.model_flag.clone().into());
        args.push(self.model.clone().into());

        // Always-on extras (e.g. --no-stream, --output-format json).
        args.extend(self.spec.extra_args.iter().map(OsString::from));

        // Prompt: either piped on stdin (preferred) or as
        // `<prompt_argv_flag> <prompt>` on argv.
        match self.spec.prompt_via {
            PromptVia::Argv => {
                args.push(self.spec.prompt_argv_flag.clone().into());
                args.push(prompt.clone().into());
            }
            PromptVia::Stdin => {
                // The supervisor pipes `stdin_prompt` for us.
            }
        }

        Ok(PreparedCommand {
            program: OsString::from(&self.spec.cli),
            args,
            cwd: PathBuf::from(ctx.cwd),
            stdin_prompt: prompt,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn qwen_spec() -> RunnerSpec {
        let toml = r#"
[vendors.qwen]
cli = "qwen"
prompt_via = "stdin"
model_flag = "--model"
extra_args = ["--no-stream"]
models = ["qwen-coder", "qwen-max"]

[vendors.qwen.profiles.standard]
args = ["--read-only=false"]

[vendors.qwen.profiles.sandbox]
args = ["--allow-all"]
"#;
        let r = crate::registry::RunnerRegistry::parse(toml).unwrap();
        r.get("qwen").unwrap().clone()
    }

    #[test]
    fn rejects_model_outside_allowlist() {
        let err = ConfigRunner::try_new("qwen", qwen_spec(), "fake-model").unwrap_err();
        assert!(matches!(err, RunnerError::UnknownModel { .. }));
    }

    #[test]
    fn empty_allowlist_accepts_any_model() {
        let mut spec = qwen_spec();
        spec.models.clear();
        ConfigRunner::try_new("qwen", spec, "anything").unwrap();
    }

    #[test]
    fn accepts_model_in_allowlist() {
        ConfigRunner::try_new("qwen", qwen_spec(), "qwen-coder").unwrap();
    }
}
