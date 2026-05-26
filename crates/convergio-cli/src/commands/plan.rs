//! `cvg plan ...` — create / list / get / show / run plans.

use super::{Client, OutputMode};
use anyhow::Result;
use clap::{Subcommand, ValueEnum};
use convergio_i18n::Bundle;
use serde_json::{json, Value};

/// Plan subcommands.
#[derive(Subcommand)]
pub enum PlanCommand {
    /// Create a new plan.
    Create {
        /// Plan title.
        title: String,
        /// Optional description.
        #[arg(long)]
        description: Option<String>,
        /// Optional project or repository this plan belongs to.
        #[arg(long)]
        project: Option<String>,
        /// Tracker-only default (A.2): every task created under this
        /// plan inherits `no_dispatch = true` unless its own request
        /// body explicitly overrides it. Use for plans that mirror
        /// work shipping from another repository.
        #[arg(long)]
        no_dispatch: bool,
    },
    /// List plans.
    List {
        /// Max rows to return.
        #[arg(long, default_value_t = 50)]
        limit: i64,
    },
    /// Get a plan by id (UUID only). Prefer `show` for number or UUID.
    Get {
        /// UUID of the plan.
        id: String,
    },
    /// Show a plan by number or UUID.
    Show {
        /// Plan number (integer) or UUID.
        id: String,
    },
    /// Rename a plan in place. Writes one `plan.renamed` audit row.
    /// See ADR-0026.
    Rename {
        /// UUID of the plan.
        id: String,
        /// New title (non-empty).
        title: String,
        /// Agent id to record on the audit row (optional).
        #[arg(long)]
        agent_id: Option<String>,
    },
    /// Move a plan to a new lifecycle status.
    ///
    /// Allowed transitions: `draft → active`, `draft → cancelled`,
    /// `active → completed`, `active → cancelled`. Anything else is
    /// rejected with HTTP 409 / `illegal_plan_transition`.
    Transition {
        /// UUID of the plan.
        id: String,
        /// Target status.
        target: PlanTransitionTarget,
    },
    /// Surface pending/failed tasks not touched for N days.
    ///
    /// Use `--auto-close` to retire all listed tasks after confirmation.
    Triage {
        /// UUID of the plan.
        id: String,
        /// Number of days without an update before a task is considered stale.
        #[arg(long, default_value_t = 7)]
        stale_days: i64,
        /// Close all listed stale tasks after operator confirmation.
        #[arg(long)]
        auto_close: bool,
    },
    /// Drives the wave/sequence state machine. Does NOT execute task
    /// bodies — assumes an external agent attaches evidence via
    /// `cvg evidence add` between transitions.
    ///
    /// Iterates pending tasks in wave/seq order, claiming each one and
    /// transitioning it to `submitted`. Halts on the first failure and
    /// prints a resume hint. `--max-parallel <K>` (default 1, clamped to
    /// [1, 16]) runs that many tasks of the same wave at once; across
    /// waves we stay sequential so the wave-sequence gate holds.
    Run {
        /// Plan number (integer) or UUID.
        id: String,
        /// Agent id to record on task transitions.
        #[arg(long, env = "CONVERGIO_AGENT_ID")]
        agent_id: Option<String>,
        /// Max concurrent tasks within the same wave.
        #[arg(long, default_value_t = 1)]
        max_parallel: u8,
    },
}

/// Target status for `cvg plan transition`. Mirrors the server-side
/// `PlanStatus` enum in `convergio-durability`.
#[derive(Clone, Copy, Debug, ValueEnum)]
#[value(rename_all = "kebab-case")]
pub enum PlanTransitionTarget {
    /// Newly created — no work yet.
    Draft,
    /// Tasks may be claimed.
    Active,
    /// All tasks complete and validated.
    Completed,
    /// Abandoned.
    Cancelled,
}

impl PlanTransitionTarget {
    fn as_wire(self) -> &'static str {
        match self {
            Self::Draft => "draft",
            Self::Active => "active",
            Self::Completed => "completed",
            Self::Cancelled => "cancelled",
        }
    }
}

/// Dispatch.
pub async fn run(
    client: &Client,
    bundle: &Bundle,
    output: OutputMode,
    cmd: PlanCommand,
) -> Result<()> {
    match cmd {
        PlanCommand::Create {
            title,
            description,
            project,
            no_dispatch,
        } => {
            let body = json!({
                "title": title,
                "description": description,
                "project": project,
                "no_dispatch_default": no_dispatch,
            });
            let plan: Value = client.post("/v1/plans", &body).await?;
            let id = plan.get("id").and_then(Value::as_str).unwrap_or("?");
            match output {
                OutputMode::Human => {
                    println!("{}", bundle.t("plan-created", &[("id", id)]));
                }
                OutputMode::Json => {
                    println!("{}", serde_json::to_string_pretty(&plan)?);
                }
                OutputMode::Plain => {
                    println!("{id}");
                }
            }
        }
        PlanCommand::List { limit } => {
            let path = format!("/v1/plans?limit={limit}");
            let plans: Value = client.get(&path).await?;
            let count = plans.as_array().map(|a| a.len() as i64).unwrap_or(0);
            match output {
                OutputMode::Human => {
                    if count == 0 {
                        println!("{}", bundle.t("plan-list-empty", &[]));
                    } else {
                        println!("{}", bundle.t_n("plan-list-header", count));
                        if let Some(arr) = plans.as_array() {
                            for p in arr {
                                let num = p.get("number").and_then(Value::as_i64).unwrap_or(0);
                                let title = p.get("title").and_then(Value::as_str).unwrap_or("?");
                                let status = p.get("status").and_then(Value::as_str).unwrap_or("?");
                                println!(
                                    "{}",
                                    bundle.t(
                                        "plan-list-line",
                                        &[
                                            ("number", &num.to_string()),
                                            ("title", title),
                                            ("status", status),
                                        ]
                                    )
                                );
                            }
                        }
                    }
                }
                OutputMode::Json => {
                    println!("{}", serde_json::to_string_pretty(&plans)?);
                }
                OutputMode::Plain => {
                    if let Some(arr) = plans.as_array() {
                        for plan in arr {
                            if let Some(id) = plan.get("id").and_then(Value::as_str) {
                                println!("{id}");
                            }
                        }
                    }
                }
            }
        }
        PlanCommand::Rename {
            id,
            title,
            agent_id,
        } => {
            let body = json!({ "title": title, "agent_id": agent_id });
            let plan: Value = client.patch(&format!("/v1/plans/{id}"), &body).await?;
            let new_title = plan.get("title").and_then(Value::as_str).unwrap_or(&title);
            match output {
                OutputMode::Human => {
                    println!(
                        "{}",
                        bundle.t("plan-renamed", &[("id", &id), ("title", new_title)])
                    );
                }
                OutputMode::Json => {
                    println!("{}", serde_json::to_string_pretty(&plan)?);
                }
                OutputMode::Plain => {
                    println!("{id}");
                }
            }
        }
        PlanCommand::Transition { id, target } => {
            let body = json!({ "target": target.as_wire() });
            let plan: Value = client
                .post(&format!("/v1/plans/{id}/transition"), &body)
                .await?;
            let status = plan.get("status").and_then(Value::as_str).unwrap_or("?");
            match output {
                OutputMode::Human => {
                    println!(
                        "{}",
                        bundle.t("plan-transitioned", &[("id", &id), ("status", status)])
                    );
                }
                OutputMode::Json => {
                    println!("{}", serde_json::to_string_pretty(&plan)?);
                }
                OutputMode::Plain => {
                    println!("{id} {status}");
                }
            }
        }
        PlanCommand::Get { id } | PlanCommand::Show { id } => {
            match client.get::<Value>(&format!("/v1/plans/{id}")).await {
                Ok(plan) => match output {
                    OutputMode::Human | OutputMode::Json => {
                        println!("{}", serde_json::to_string_pretty(&plan)?);
                    }
                    OutputMode::Plain => {
                        if let Some(plan_id) = plan.get("id").and_then(Value::as_str) {
                            println!("{plan_id}");
                        }
                    }
                },
                Err(e) => {
                    eprintln!("{}", bundle.t("plan-not-found", &[("id", &id)]));
                    return Err(e);
                }
            }
        }
        PlanCommand::Triage {
            id,
            stale_days,
            auto_close,
        } => {
            super::plan_triage::run(client, bundle, output, &id, stale_days, auto_close).await?;
        }
        PlanCommand::Run {
            id,
            agent_id,
            max_parallel,
        } => {
            super::plan_run::run(
                client,
                bundle,
                output,
                &id,
                agent_id.as_deref(),
                max_parallel,
            )
            .await?;
        }
    }
    Ok(())
}
