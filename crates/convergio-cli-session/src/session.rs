//! `cvg session ...` — cold-start brief from the daemon.
//!
//! Replaces the static handoff markdown that goes stale. Every value
//! printed here comes from a live daemon query (health, audit, plan
//! tasks) plus an optional `gh pr list` shell-out for the queue.
//!
//! The markdown packet (`docs/agent-resume-packet.md`) is now the
//! TIMELESS half of the cold-start; this command is the time-specific
//! half — read both, in that order, after a session reset.
//!
//! Renderers live in the sibling [`crate::render`] module to
//! keep both files under the 300-line cap.

use crate::pre_stop_run;
use crate::render::{self, Brief};
use crate::{Client, OutputMode};
use anyhow::{anyhow, Context, Result};
use clap::Subcommand;
use convergio_i18n::Bundle;
use serde::{Deserialize, Serialize};
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
            pre_stop_run::handle(client, output, agent_id, force)
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

/// A plan is "open" — i.e. valid for `cvg session resume` to focus
/// on — when its status is `draft` or `active`. `completed` and
/// `cancelled` are terminal and would yield misleading next-task
/// guidance even if their `updated_at` is more recent than the
/// real active plan.
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

#[derive(Debug, Deserialize, Serialize)]
pub(crate) struct Plan {
    pub(crate) id: String,
    pub(crate) title: String,
    #[serde(default)]
    pub(crate) description: Option<String>,
    #[serde(default)]
    pub(crate) project: Option<String>,
    pub(crate) status: String,
    pub(crate) updated_at: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub(crate) struct Task {
    pub(crate) id: String,
    pub(crate) title: String,
    pub(crate) status: String,
    pub(crate) wave: i64,
    pub(crate) sequence: i64,
    pub(crate) created_at: String,
}

#[derive(Debug, Default, Serialize)]
pub(crate) struct TaskCounts {
    pub(crate) total: usize,
    pub(crate) done: usize,
    pub(crate) pending: usize,
    pub(crate) in_progress: usize,
    pub(crate) submitted: usize,
    pub(crate) failed: usize,
}

impl From<&[Task]> for TaskCounts {
    fn from(tasks: &[Task]) -> Self {
        let mut c = TaskCounts {
            total: tasks.len(),
            ..Default::default()
        };
        for t in tasks {
            match t.status.as_str() {
                "done" => c.done += 1,
                "pending" => c.pending += 1,
                "in_progress" => c.in_progress += 1,
                "submitted" => c.submitted += 1,
                "failed" => c.failed += 1,
                _ => {}
            }
        }
        c
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub(crate) struct PrSummary {
    pub(crate) number: i64,
    pub(crate) title: String,
    #[serde(rename = "headRefName")]
    pub(crate) head_ref_name: String,
    #[serde(rename = "isDraft", default)]
    pub(crate) is_draft: bool,
}

#[cfg(test)]
#[path = "session_tests.rs"]
mod tests;
