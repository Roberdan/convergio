//! `cvg fleet plan ...` — fleet plan management (ADR-0038, F3-2).
//! Pure HTTP. The daemon owns `fleet_plans` + `fleet_plan_repos`.
//! Subcommands: `create`, `ls`, `show`, `link-repo`, `add-task`.

use super::{Client, OutputMode};
use anyhow::{Context, Result};
use clap::Subcommand;
use serde_json::{json, Value};

/// Fleet plan subcommands.
#[derive(Subcommand)]
pub enum FleetPlanCommand {
    /// Create a new fleet plan.
    Create {
        /// Plan title.
        title: String,
        /// Scope: "fleet" (cross-repo) or a repo name.
        #[arg(long, default_value = "fleet")]
        scope: String,
    },
    /// List fleet plans (newest first).
    Ls {
        /// Filter by scope.
        #[arg(long)]
        scope: Option<String>,
    },
    /// Show a fleet plan with its per-repo links.
    Show {
        /// Fleet plan id (UUID).
        id: String,
    },
    /// Link a per-repo plan into a fleet plan. Idempotent.
    LinkRepo {
        /// Fleet plan id.
        id: String,
        /// Repo name (must exist in `fleet_repos`).
        #[arg(long)]
        repo: String,
        /// Per-repo plan id (must already exist in `convergio-durability`).
        #[arg(long)]
        repo_plan_id: String,
    },
    /// Add a task to the per-repo plan linked to a fleet plan.
    AddTask {
        /// Fleet plan id.
        id: String,
        /// Repo name.
        #[arg(long)]
        repo: String,
        /// Task title.
        #[arg(long)]
        title: String,
        /// Task description (optional).
        #[arg(long)]
        description: Option<String>,
        /// Wave (default 1).
        #[arg(long, default_value_t = 1)]
        wave: i64,
        /// Sequence within wave (default 1).
        #[arg(long, default_value_t = 1)]
        sequence: i64,
        /// Evidence kinds required (repeatable).
        #[arg(long = "evidence", action = clap::ArgAction::Append)]
        evidence_required: Vec<String>,
    },
}

/// Entry point.
pub async fn run(client: &Client, output: OutputMode, cmd: FleetPlanCommand) -> Result<()> {
    match cmd {
        FleetPlanCommand::Create { title, scope } => create(client, output, &title, &scope).await,
        FleetPlanCommand::Ls { scope } => ls(client, output, scope.as_deref()).await,
        FleetPlanCommand::Show { id } => show(client, output, &id).await,
        FleetPlanCommand::LinkRepo {
            id,
            repo,
            repo_plan_id,
        } => link_repo(client, output, &id, &repo, &repo_plan_id).await,
        FleetPlanCommand::AddTask {
            id,
            repo,
            title,
            description,
            wave,
            sequence,
            evidence_required,
        } => {
            add_task(
                client,
                output,
                &id,
                &repo,
                &title,
                description.as_deref(),
                wave,
                sequence,
                &evidence_required,
            )
            .await
        }
    }
}

async fn create(client: &Client, output: OutputMode, title: &str, scope: &str) -> Result<()> {
    let body: Value = client
        .post(
            "/v1/fleet/plans",
            &json!({ "title": title, "scope": scope }),
        )
        .await
        .context("create fleet plan")?;
    render_value(output, &body, "created");
    Ok(())
}

async fn ls(client: &Client, output: OutputMode, scope: Option<&str>) -> Result<()> {
    let path = match scope {
        Some(s) => format!("/v1/fleet/plans?scope={s}"),
        None => "/v1/fleet/plans".into(),
    };
    let plans: Vec<Value> = client.get(&path).await.context("list fleet plans")?;
    match output {
        OutputMode::Json => println!("{}", serde_json::to_string_pretty(&plans)?),
        OutputMode::Plain => {
            for p in &plans {
                println!("{}", p.get("id").and_then(|v| v.as_str()).unwrap_or(""));
            }
        }
        OutputMode::Human => {
            if plans.is_empty() {
                println!("No fleet plans.");
            } else {
                println!("{} fleet plan(s):", plans.len());
                for p in &plans {
                    println!(
                        "  {}  [{}]  {}",
                        p.get("id").and_then(|v| v.as_str()).unwrap_or("?"),
                        p.get("scope").and_then(|v| v.as_str()).unwrap_or("?"),
                        p.get("title").and_then(|v| v.as_str()).unwrap_or("?"),
                    );
                }
            }
        }
    }
    Ok(())
}

async fn show(client: &Client, output: OutputMode, id: &str) -> Result<()> {
    let body: Value = client
        .get(&format!("/v1/fleet/plans/{id}"))
        .await
        .context("show fleet plan")?;
    match output {
        OutputMode::Json => println!("{}", serde_json::to_string_pretty(&body)?),
        OutputMode::Plain => {
            if let Some(p) = body
                .get("plan")
                .and_then(|v| v.get("id"))
                .and_then(|v| v.as_str())
            {
                println!("{p}");
            }
        }
        OutputMode::Human => {
            let plan = body.get("plan").unwrap_or(&Value::Null);
            let links = body
                .get("links")
                .and_then(|v| v.as_array())
                .cloned()
                .unwrap_or_default();
            println!(
                "Fleet plan: {}",
                plan.get("title").and_then(|v| v.as_str()).unwrap_or("?")
            );
            println!(
                "  id    : {}",
                plan.get("id").and_then(|v| v.as_str()).unwrap_or("?")
            );
            println!(
                "  scope : {}",
                plan.get("scope").and_then(|v| v.as_str()).unwrap_or("?")
            );
            if links.is_empty() {
                println!("  links : (none)");
            } else {
                println!("  links :");
                for l in links {
                    println!(
                        "    - {} → {}",
                        l.get("repo").and_then(|v| v.as_str()).unwrap_or("?"),
                        l.get("repo_plan_id")
                            .and_then(|v| v.as_str())
                            .unwrap_or("?"),
                    );
                }
            }
        }
    }
    Ok(())
}

async fn link_repo(
    client: &Client,
    output: OutputMode,
    id: &str,
    repo: &str,
    repo_plan_id: &str,
) -> Result<()> {
    let body: Value = client
        .post(
            &format!("/v1/fleet/plans/{id}/repos"),
            &json!({ "repo": repo, "repo_plan_id": repo_plan_id }),
        )
        .await
        .context("link repo to fleet plan")?;
    render_value(output, &body, "linked");
    Ok(())
}

#[allow(clippy::too_many_arguments)]
async fn add_task(
    client: &Client,
    output: OutputMode,
    id: &str,
    repo: &str,
    title: &str,
    description: Option<&str>,
    wave: i64,
    sequence: i64,
    evidence_required: &[String],
) -> Result<()> {
    let body: Value = client
        .post(
            &format!("/v1/fleet/plans/{id}/repos/{repo}/tasks"),
            &json!({
                "title": title,
                "description": description,
                "wave": wave,
                "sequence": sequence,
                "evidence_required": evidence_required,
            }),
        )
        .await
        .context("add task to fleet plan repo")?;
    render_value(output, &body, "added");
    Ok(())
}

fn render_value(output: OutputMode, body: &Value, verb: &str) {
    match output {
        OutputMode::Json => println!("{}", serde_json::to_string_pretty(body).unwrap_or_default()),
        OutputMode::Plain => {
            if let Some(id) = body.get("id").and_then(|v| v.as_str()) {
                println!("{id}");
            }
        }
        OutputMode::Human => println!("{verb}: {body}"),
    }
}
