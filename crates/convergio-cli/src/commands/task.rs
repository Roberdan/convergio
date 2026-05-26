//! `cvg task ...` — create, inspect and transition local tasks.

use super::task_pr_url::parse_github_pr_url;
use super::task_render::{render_task, render_task_list};
use super::task_templates::{resolve_evidence, TaskTemplate};
use super::{Client, OutputMode};
use anyhow::{bail, Result};
use clap::{Subcommand, ValueEnum};
use convergio_i18n::Bundle;
use serde_json::{json, Value};

/// Task subcommands.
#[derive(Subcommand)]
pub enum TaskCommand {
    /// Create a task under a plan.
    Create {
        /// Owning plan id.
        plan_id: String,
        /// Task title (short).
        title: String,
        /// Optional long description.
        #[arg(long)]
        description: Option<String>,
        /// Wave number (defaults to 1).
        #[arg(long, default_value_t = 1)]
        wave: i64,
        /// Sequence within the wave (defaults to 1).
        #[arg(long, default_value_t = 1)]
        sequence: i64,
        /// Required evidence kinds (comma-separated, e.g. `code,test`).
        #[arg(long, value_delimiter = ',')]
        evidence_required: Vec<String>,
        /// Task category template — pre-populates `evidence_required`
        /// from a category-specific default (P2-10). User-supplied
        /// `--evidence-required` values are merged on top.
        #[arg(long, value_enum)]
        template: Option<TaskTemplate>,
        /// Optional runner kind (ADR-0034) in the wire format
        /// `<vendor>:<model>` — e.g. `claude:sonnet`,
        /// `claude:opus`, `copilot:gpt-5.2`, `qwen:qwen3-coder`.
        /// Omit to let the executor use the daemon default.
        #[arg(long)]
        runner: Option<String>,
        /// Optional permission profile (`standard` / `read_only` /
        /// `sandbox`). Omit to use the daemon default.
        #[arg(long)]
        profile: Option<String>,
        /// Optional session budget cap in USD (Claude only).
        #[arg(long)]
        max_budget_usd: Option<f32>,
        /// Tracker-only mode (A.2): create the task in `pending` but
        /// flag it so the executor never auto-dispatches it. Use to
        /// mirror work that ships from another repository.
        #[arg(long)]
        no_dispatch: bool,
    },
    /// List tasks of a plan.
    List {
        /// Plan id.
        plan_id: String,
    },
    /// Get one task.
    Get {
        /// Task id.
        task_id: String,
    },
    /// Move a task to a new status, running server-side gates.
    Transition {
        /// Task id.
        task_id: String,
        /// Target status.
        target: TaskTarget,
        /// Agent id to record on the transition.
        #[arg(long)]
        agent_id: Option<String>,
        /// GitHub PR URL to attach as `pr_url` evidence and to register
        /// as a plan↔PR link (best-effort).
        #[arg(long, value_name = "URL")]
        pr_url: Option<String>,
    },
    /// Touch a task heartbeat.
    Heartbeat {
        /// Task id.
        task_id: String,
    },
    /// Reopen a failed task: clears its previous owner and moves it
    /// back to `pending` so a new agent can claim it. Refused with
    /// HTTP 409 (`not_failed`) if the task is in any other status.
    Retry {
        /// Task id.
        task_id: String,
        /// Agent id to record on the audit row (optional).
        #[arg(long)]
        agent_id: Option<String>,
    },
    /// Operator-driven post-hoc close: move a task directly to
    /// `done` because the work shipped outside the daemon's evidence
    /// flow. Mandatory `--reason` is recorded in the audit row.
    /// See ADR-0026.
    ClosePostHoc {
        /// Task id.
        task_id: String,
        /// Reason for the post-hoc close (required; must reference a PR, commit, task UUID, ADR, or friction id).
        #[arg(long)]
        reason: String,
        /// Agent id to record on the audit row (optional).
        #[arg(long)]
        agent_id: Option<String>,
    },
    /// Orchestrate full task completion: graph for-task --semantic →
    /// embed for-task → evidence add ×N → transition submitted →
    /// validate plan (Thor) → done.
    Complete {
        /// Task id.
        task_id: String,
        /// GitHub PR number to record as evidence.
        #[arg(long)]
        pr: u64,
        /// Agent id for audit rows.
        #[arg(long)]
        agent_id: Option<String>,
    },
}

/// CLI-friendly task status values that an agent may request.
///
/// `done` is intentionally absent: it is set only by the validator
/// (`cvg validate <plan_id>`). See CONSTITUTION §6 and ADR-0011.
#[derive(Clone, Copy, ValueEnum)]
pub enum TaskTarget {
    /// Claimed and being worked on.
    InProgress,
    /// Agent claims completion; awaiting validation.
    Submitted,
    /// Failed and not retryable.
    Failed,
    /// Release back to pending.
    Pending,
}

impl TaskTarget {
    fn as_api(self) -> &'static str {
        match self {
            Self::InProgress => "in_progress",
            Self::Submitted => "submitted",
            Self::Failed => "failed",
            Self::Pending => "pending",
        }
    }
}

/// Run a task subcommand.
pub async fn run(
    client: &Client,
    bundle: &Bundle,
    output: OutputMode,
    cmd: TaskCommand,
) -> Result<()> {
    match cmd {
        TaskCommand::Create {
            plan_id,
            title,
            description,
            wave,
            sequence,
            evidence_required,
            template,
            runner,
            profile,
            max_budget_usd,
            no_dispatch,
        } => {
            let evidence_required = resolve_evidence(evidence_required, template);
            let body = json!({
                "title": title,
                "description": description,
                "wave": wave,
                "sequence": sequence,
                "evidence_required": evidence_required,
                "runner_kind": runner,
                "profile": profile,
                "max_budget_usd": max_budget_usd,
                "no_dispatch": if no_dispatch { Some(true) } else { None::<bool> },
            });
            let task: Value = client
                .post(&format!("/v1/plans/{plan_id}/tasks"), &body)
                .await?;
            render_task(&task, output)
        }
        TaskCommand::List { plan_id } => {
            let tasks: Value = client.get(&format!("/v1/plans/{plan_id}/tasks")).await?;
            render_task_list(&tasks, output)
        }
        TaskCommand::Get { task_id } => {
            let task: Value = client.get(&format!("/v1/tasks/{task_id}")).await?;
            render_task(&task, output)
        }
        TaskCommand::Transition {
            task_id,
            target,
            agent_id,
            pr_url,
        } => {
            if let Some(url) = pr_url.as_deref() {
                // Evidence required by EvidenceGate for most real tasks.
                let _: Value = client
                    .post(
                        &format!("/v1/tasks/{task_id}/evidence"),
                        &json!({
                            "kind": "pr_url",
                            "payload": {"url": url},
                        }),
                    )
                    .await?;

                // Ownership mapping (best-effort): if we can parse the URL
                // and resolve plan_id, populate plan_pr_links.
                if let Some((repo_slug, pr_number)) = parse_github_pr_url(url) {
                    let task: Value = client.get(&format!("/v1/tasks/{task_id}")).await?;
                    if let Some(plan_id) = task.get("plan_id").and_then(Value::as_str) {
                        let _: Value = client
                            .post(
                                &format!("/v1/plans/{plan_id}/pr-links"),
                                &json!({
                                    "pr_number": pr_number,
                                    "repo_slug": repo_slug,
                                    "task_id": task_id.clone(),
                                    "agent_id": agent_id.clone(),
                                }),
                            )
                            .await?;
                    }
                }
            }

            let body = json!({
                "target": target.as_api(),
                "agent_id": agent_id,
            });
            let task: Value = client
                .post(&format!("/v1/tasks/{task_id}/transition"), &body)
                .await?;
            render_task(&task, output)
        }
        TaskCommand::Retry { task_id, agent_id } => {
            let body = json!({ "agent_id": agent_id });
            let task: Value = client
                .post(&format!("/v1/tasks/{task_id}/retry"), &body)
                .await?;
            render_task(&task, output)
        }
        TaskCommand::ClosePostHoc {
            task_id,
            reason,
            agent_id,
        } => {
            let body = json!({ "reason": reason, "agent_id": agent_id });
            let task: Value = client
                .post(&format!("/v1/tasks/{task_id}/close-post-hoc"), &body)
                .await?;
            render_task(&task, output)
        }
        TaskCommand::Heartbeat { task_id } => {
            let ok: Value = client
                .post(&format!("/v1/tasks/{task_id}/heartbeat"), &json!({}))
                .await?;
            if ok.get("ok").and_then(Value::as_bool) != Some(true) {
                bail!("unexpected heartbeat response: {ok}");
            }
            match output {
                OutputMode::Plain => println!("ok"),
                _ => println!("{}", serde_json::to_string_pretty(&ok)?),
            }
            Ok(())
        }
        TaskCommand::Complete {
            task_id,
            pr,
            agent_id,
        } => {
            let done = super::task_complete::run(client, bundle, &task_id, pr, agent_id.as_deref())
                .await?;
            render_task(&done, output)
        }
    }
}
