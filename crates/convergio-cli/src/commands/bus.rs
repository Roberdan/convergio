//! `cvg bus ...` — human-facing reader (and minimal writer) for the
//! Layer 2 plan-scoped message bus. Agents go through MCP
//! `poll_messages` / `publish_message`; humans land here.
//!
//! All three subcommands (`tail`, `topics`, `post`) accept an optional
//! `--plan <id>` and otherwise resolve the most-recently-updated open
//! plan for `--project <name>` (default `convergio-local`), the same
//! resolver `cvg session resume` uses.

use super::bus_render::BusOutput;
use super::{bus_extra, bus_tail, Client, OutputMode};
use anyhow::{anyhow, Context, Result};
use clap::Subcommand;
use convergio_i18n::Bundle;
use serde::Deserialize;
use serde_json::Value;

/// Bus subcommands.
#[derive(Subcommand)]
pub enum BusCommand {
    /// Print messages on a plan, oldest first. With `--follow`,
    /// subscribe to the SSE feed (P1.1) and print events as they
    /// arrive; on disconnect we reconnect with the last-seen seq;
    /// if the daemon does not advertise streaming we fall back to
    /// polling. Ctrl-C exits.
    Tail {
        /// Plan id. Defaults to the most recent open plan in `--project`.
        #[arg(long)]
        plan: Option<String>,
        /// Project filter when no plan id is given.
        #[arg(long, default_value = "convergio-local")]
        project: String,
        /// Literal-match topic filter (glob is wave-2).
        #[arg(long)]
        topic: Option<String>,
        /// Only return messages with `seq > since` (exclusive).
        #[arg(long, default_value_t = 0)]
        since: i64,
        /// Cap on the number of messages in non-follow mode (1..=100).
        #[arg(long, default_value_t = 50)]
        limit: i64,
        /// Stream new messages live via SSE (P1.1).
        #[arg(long, short = 'f')]
        follow: bool,
    },
    /// Print the latest N messages on a plan and exit. Static-dump
    /// companion to `tail` without --follow; explicit for scripting.
    List {
        /// Plan id. Defaults to the most recent open plan in `--project`.
        #[arg(long)]
        plan: Option<String>,
        /// Project filter when no plan id is given.
        #[arg(long, default_value = "convergio-local")]
        project: String,
        /// Literal-match topic filter.
        #[arg(long)]
        topic: Option<String>,
        /// Only return messages with `seq > since` (exclusive).
        #[arg(long, default_value_t = 0)]
        since: i64,
        /// Cap on the number of messages returned (1..=100).
        #[arg(long, default_value_t = 50)]
        limit: i64,
    },
    /// Print every topic that has at least one message on a plan,
    /// with count + last_seq + last_at.
    Topics {
        #[arg(long)]
        plan: Option<String>,
        #[arg(long, default_value = "convergio-local")]
        project: String,
    },
    /// Publish a JSON payload to a topic. Mostly for ad-hoc human
    /// posts; agents should use the MCP `publish_message` action.
    Post {
        /// Topic to publish on.
        #[arg(long)]
        topic: String,
        /// JSON payload.
        #[arg(long, default_value = "{}")]
        payload: String,
        /// Optional sender (agent id).
        #[arg(long)]
        sender: Option<String>,
        #[arg(long)]
        plan: Option<String>,
        #[arg(long, default_value = "convergio-local")]
        project: String,
    },
}

/// Entry point.
pub async fn run(
    client: &Client,
    bundle: &Bundle,
    output: OutputMode,
    cmd: BusCommand,
) -> Result<()> {
    match cmd {
        BusCommand::Tail {
            plan,
            project,
            topic,
            since,
            limit,
            follow,
        } => {
            let plan = resolve_plan(client, plan.as_deref(), &project).await?;
            if follow {
                bus_tail::follow(
                    client,
                    bundle,
                    BusOutput::from_global(output),
                    &plan.id,
                    topic.as_deref(),
                    since,
                )
                .await
            } else {
                tail(client, output, &plan, topic.as_deref(), since, limit).await
            }
        }
        BusCommand::List {
            plan,
            project,
            topic,
            since,
            limit,
        } => {
            let plan = resolve_plan(client, plan.as_deref(), &project).await?;
            bus_tail::list(
                client,
                bundle,
                BusOutput::from_global(output),
                &plan.id,
                topic.as_deref(),
                since,
                limit,
            )
            .await
        }
        BusCommand::Topics { plan, project } => {
            bus_extra::topics(client, output, plan.as_deref(), &project).await
        }
        BusCommand::Post {
            topic,
            payload,
            sender,
            plan,
            project,
        } => {
            bus_extra::post(
                client,
                output,
                plan.as_deref(),
                &project,
                &topic,
                &payload,
                sender.as_deref(),
            )
            .await
        }
    }
}

async fn tail(
    client: &Client,
    output: OutputMode,
    plan: &Plan,
    topic: Option<&str>,
    since: i64,
    limit: i64,
) -> Result<()> {
    let mut path = format!(
        "/v1/plans/{}/messages/tail?cursor={since}&limit={limit}",
        plan.id
    );
    if let Some(t) = topic {
        path.push_str(&format!("&topic={t}"));
    }
    let messages: Vec<Value> = client.get(&path).await?;
    match output {
        OutputMode::Json => println!("{}", serde_json::to_string_pretty(&messages)?),
        OutputMode::Plain => {
            for m in &messages {
                let seq = m.get("seq").and_then(Value::as_i64).unwrap_or(0);
                let topic = m.get("topic").and_then(Value::as_str).unwrap_or("?");
                let sender = m.get("sender").and_then(Value::as_str).unwrap_or("-");
                println!("seq={seq} topic={topic} sender={sender}");
            }
        }
        OutputMode::Human => render_tail_human(plan, &messages),
    }
    Ok(())
}

fn render_tail_human(plan: &Plan, messages: &[Value]) {
    println!("Plan {} — {} message(s)", plan.id, messages.len());
    for m in messages {
        let seq = m.get("seq").and_then(Value::as_i64).unwrap_or(0);
        let topic = m.get("topic").and_then(Value::as_str).unwrap_or("?");
        let sender = m.get("sender").and_then(Value::as_str).unwrap_or("-");
        let consumed = m.get("consumed_at").and_then(Value::as_str).is_some();
        let kind = m
            .get("payload")
            .and_then(|p| p.get("kind"))
            .and_then(Value::as_str)
            .unwrap_or("");
        let mark = if consumed { " [acked]" } else { "" };
        let kind_part = if kind.is_empty() {
            String::new()
        } else {
            format!(" {kind}")
        };
        println!("  seq {seq:>3} [{topic}] sender={sender}{kind_part}{mark}");
    }
}

pub(super) async fn resolve_plan(
    client: &Client,
    plan_id: Option<&str>,
    project: &str,
) -> Result<Plan> {
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
        .filter(|p| matches!(p.status.as_str(), "draft" | "active"))
        .max_by(|a, b| a.updated_at.cmp(&b.updated_at))
        .ok_or_else(|| anyhow!("no open plan found for project={project}"))
}

/// Minimal plan shape used by the bus dispatcher to resolve
/// `--plan` / `--project` shorthand into a concrete plan id.
///
/// Re-exported `pub(super)` so [`super::bus_extra`] can call
/// [`resolve_plan`] without duplicating the resolver logic.
#[derive(Debug, Deserialize)]
pub(super) struct Plan {
    /// Plan UUID.
    pub id: String,
    /// Project name (None for orphan plans).
    #[serde(default)]
    pub project: Option<String>,
    /// Lifecycle status string (`draft` / `active` / `completed`...).
    pub status: String,
    /// RFC3339 timestamp used to pick the most-recent open plan.
    pub updated_at: String,
}
