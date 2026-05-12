//! `cvg session ...` — cold-start brief and lifecycle subcommands.
//!
//! Every value printed by `resume` comes from a live daemon query
//! (health, audit, plan tasks) plus an optional `gh pr list`. The
//! TIMELESS handoff lives in `docs/agent-resume-packet.md`. Renderers
//! live in the sibling [`crate::render`] module to keep both files
//! under the 300-line cap.

use crate::pre_stop_run;
use crate::register_and_poll;
use crate::render::{self, Brief};
use crate::session_models::{Plan, PrSummary, Task, TaskCounts};
use crate::{Client, OutputMode};
use anyhow::{anyhow, Context, Result};
use clap::Subcommand;
use convergio_i18n::Bundle;
use serde_json::Value;
use std::process::Command;

/// Session subcommands.
#[derive(Subcommand)]
pub enum SessionCommand {
    /// Print a cold-start brief: daemon health, audit chain, the
    /// active plan, top pending tasks, and open PRs.
    Resume {
        /// Plan id. If omitted, resolves the most recently updated
        /// plan in `--project`.
        plan_id: Option<String>,
        /// Project filter when no plan id is given.
        #[arg(long, default_value = "convergio")]
        project: String,
        /// Number of next-priority pending tasks to surface.
        #[arg(long, default_value_t = 5)]
        next_limit: usize,
        /// Optional task id. When set, the brief is preceded by a
        /// graph context-pack scoped to that task (ADR-0014).
        #[arg(long)]
        task_id: Option<String>,
    },
    /// Run the end-of-session safety net (PRD-001 Artefact 4).
    /// See plan `db88bc17` (W0b.2) for the six per-check tasks.
    PreStop {
        /// Stable agent id (matches what was registered).
        #[arg(long)]
        agent_id: String,
        /// Detach despite findings.
        #[arg(long, default_value_t = false)]
        force: bool,
    },
    /// Register, heartbeat, and poll inbox on every active plan.
    /// Wired as the Claude Code SessionStart hook so every session
    /// shows up in `cvg agent list` before the first user prompt.
    RegisterAndPoll {
        /// Stable agent id. Defaults to `CONVERGIO_AGENT_ID`, then
        /// `claude-code-${USER}`.
        #[arg(long)]
        agent_id: Option<String>,
        /// Capability tag (repeatable). Default: code, test, doc.
        #[arg(long = "capability", value_name = "NAME")]
        capabilities: Vec<String>,
        /// Host kind. Default: `claude`.
        #[arg(long, default_value = "claude")]
        kind: String,
        /// Host label. Default: `uname -n`.
        #[arg(long)]
        host: Option<String>,
        /// Suppress the `session-started` bus announcement.
        #[arg(long, default_value_t = false)]
        quiet: bool,
        /// Skip auto-ack of unicast `agent:<id>` direct messages.
        /// Default (P1-3) acks each direct after rendering. Broadcast
        /// topics (`plan:*`, `coordination/*`) are never auto-acked.
        #[arg(long, default_value_t = false)]
        no_auto_ack: bool,
    },
    /// Idempotent heartbeat for the Claude Code `PreToolUse` hook
    /// (P1-3). Throttled by a per-pid timestamp file under
    /// `~/.convergio/state/sessions/`. Errors are swallowed.
    HeartbeatSinceLastTurn {
        /// Stable agent id. Defaults to `CONVERGIO_AGENT_ID`, then
        /// `claude-code-${USER}`.
        #[arg(long)]
        agent_id: Option<String>,
        /// Status to send. Defaults to `working`.
        #[arg(long, default_value = "working")]
        status: String,
    },
}

/// Entry point.
pub async fn run(
    client: &Client,
    bundle: &Bundle,
    output: OutputMode,
    cmd: SessionCommand,
) -> Result<()> {
    match cmd {
        SessionCommand::Resume {
            plan_id,
            project,
            next_limit,
            task_id,
        } => {
            resume(
                client,
                bundle,
                output,
                plan_id,
                &project,
                next_limit,
                task_id.as_deref(),
            )
            .await
        }
        SessionCommand::PreStop { agent_id, force } => {
            pre_stop_run::handle(client, bundle, output, agent_id, force)
        }
        SessionCommand::RegisterAndPoll {
            agent_id,
            capabilities,
            kind,
            host,
            quiet,
            no_auto_ack,
        } => {
            let args = register_and_poll::Args {
                agent_id,
                capabilities,
                kind,
                host,
                quiet,
                no_auto_ack,
            };
            register_and_poll::run(client, bundle, output, args).await
        }
        SessionCommand::HeartbeatSinceLastTurn { agent_id, status } => {
            crate::heartbeat_since_last_turn::run(client, agent_id, status).await
        }
    }
}

async fn resume(
    client: &Client,
    bundle: &Bundle,
    output: OutputMode,
    plan_id: Option<String>,
    project: &str,
    next_limit: usize,
    task_id: Option<&str>,
) -> Result<()> {
    let health: Value = client.get("/v1/health").await.context("GET /v1/health")?;
    let audit: Value = client
        .get("/v1/audit/verify")
        .await
        .context("GET /v1/audit/verify")?;

    let plan = resolve_plan(client, plan_id.as_deref(), project).await?;
    let tasks: Vec<Task> = client
        .get(&format!("/v1/plans/{}/tasks", plan.id))
        .await
        .context("GET plan tasks")?;
    let counts = TaskCounts::from(tasks.as_slice());
    let next = top_pending(&tasks, next_limit);

    let prs = fetch_open_prs().ok();
    let pack = match task_id {
        Some(id) => fetch_pack(client, id).await.ok(),
        None => None,
    };

    let brief = Brief {
        health: &health,
        audit: &audit,
        plan: &plan,
        counts: &counts,
        next: &next,
        prs: prs.as_deref(),
        pack: pack.as_ref(),
    };
    render::render(bundle, output, &brief)
}

async fn fetch_pack(client: &Client, task_id: &str) -> Result<Value> {
    client
        .get(&format!("/v1/graph/for-task/{task_id}"))
        .await
        .with_context(|| format!("GET /v1/graph/for-task/{task_id}"))
}

async fn resolve_plan(client: &Client, plan_id: Option<&str>, project: &str) -> Result<Plan> {
    if let Some(id) = plan_id {
        return client
            .get(&format!("/v1/plans/{id}"))
            .await
            .with_context(|| format!("GET /v1/plans/{id}"));
    }
    let plans: Vec<Plan> = client.get("/v1/plans").await.context("GET /v1/plans")?;
    plans
        .into_iter()
        .filter(|p| p.project.as_deref() == Some(project))
        .filter(|p| is_open_status(&p.status))
        .max_by(|a, b| a.updated_at.cmp(&b.updated_at))
        .ok_or_else(|| anyhow!("no open plan found for project={project}"))
}

/// `draft` / `active` are open; `completed` / `cancelled` are
/// terminal and would yield misleading next-task guidance.
fn is_open_status(status: &str) -> bool {
    matches!(status, "draft" | "active")
}

fn top_pending(tasks: &[Task], limit: usize) -> Vec<Task> {
    let mut pending: Vec<Task> = tasks
        .iter()
        .filter(|t| t.status == "pending")
        .cloned()
        .collect();
    pending.sort_by(|a, b| {
        a.wave
            .cmp(&b.wave)
            .then(a.sequence.cmp(&b.sequence))
            .then(a.created_at.cmp(&b.created_at))
    });
    pending.truncate(limit);
    pending
}

fn fetch_open_prs() -> Result<Vec<PrSummary>> {
    let out = Command::new("gh")
        .args([
            "pr",
            "list",
            "--state",
            "open",
            "--json",
            "number,title,headRefName,isDraft",
        ])
        .output()
        .context("spawn gh")?;
    if !out.status.success() {
        anyhow::bail!(
            "gh pr list failed: {}",
            String::from_utf8_lossy(&out.stderr)
        );
    }
    serde_json::from_slice(&out.stdout).context("parse gh output")
}

#[cfg(test)]
#[path = "session_tests.rs"]
mod tests;
