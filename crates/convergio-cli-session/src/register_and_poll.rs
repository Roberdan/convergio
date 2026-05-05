//! `cvg session register-and-poll` — automatic SessionStart wiring.
//!
//! Forces every Claude Code (and other harness) session to register
//! itself in `agent_registry`, send a heartbeat, list active plans,
//! and pull the first slice of inbox messages. The Claude Code
//! `SessionStart` hook in `.claude/settings.json` calls this command
//! before the first user prompt, closing the gap that let two
//! parallel sessions work in the repo on 2026-05-04 without ever
//! showing up in the registry.

use crate::register_and_poll_render;
use crate::{Client, OutputMode};
use anyhow::{Context, Result};
use convergio_i18n::Bundle;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

/// Default capabilities for a Claude Code session.
const DEFAULT_CAPABILITIES: &[&str] = &["code", "test", "doc"];

/// Topic on which session-start announcements are published. Both
/// halves of a coordinating swarm must use the same string.
const COORDINATION_TOPIC: &str = "coordination/agents";

/// Arguments for `cvg session register-and-poll`.
pub struct Args {
    /// Stable agent id. `None` means: try `CONVERGIO_AGENT_ID`, then
    /// fall back to `claude-code-${USER}`.
    pub agent_id: Option<String>,
    /// Capabilities. Empty means use [`DEFAULT_CAPABILITIES`].
    pub capabilities: Vec<String>,
    /// Host kind (e.g. `claude`, `copilot`).
    pub kind: String,
    /// Optional host label. `None` means: shell out to `uname -n`.
    pub host: Option<String>,
    /// When `true`, skip publishing the `session-started` bus
    /// announcement on every active plan.
    pub quiet: bool,
}

/// Entry point.
pub async fn run(client: &Client, bundle: &Bundle, output: OutputMode, args: Args) -> Result<()> {
    let agent_id = resolve_agent_id(args.agent_id);
    let capabilities = if args.capabilities.is_empty() {
        DEFAULT_CAPABILITIES
            .iter()
            .map(|s| (*s).to_string())
            .collect()
    } else {
        args.capabilities
    };
    let host = args.host.unwrap_or_else(uname_n);

    let registered = register(client, &agent_id, &args.kind, &capabilities, &host).await?;
    let beat = heartbeat(client, &agent_id).await?;
    let plans = active_plans(client).await?;

    let mut direct: Vec<(String, Vec<Value>)> = Vec::new();
    let mut announcements: Vec<(String, Vec<Value>)> = Vec::new();
    let mut acked_direct: usize = 0;
    for plan in &plans {
        let pid = plan.id.as_str();
        let inbox = poll_topic(client, pid, &format!("agent:{agent_id}")).await?;
        if !inbox.is_empty() {
            acked_direct += ack_messages(client, &inbox, &agent_id).await;
            direct.push((pid.to_string(), inbox));
        }
        let plan_topic = poll_topic(client, pid, &format!("plan:{pid}")).await?;
        if !plan_topic.is_empty() {
            announcements.push((pid.to_string(), plan_topic));
        }
    }

    if !args.quiet {
        announce_session_start(client, &plans, &agent_id, &args.kind, &capabilities, &host).await?;
    }

    let report = SessionReport {
        agent: registered,
        heartbeat: beat,
        plans: &plans,
        direct: &direct,
        announcements: &announcements,
        acked_direct,
    };
    register_and_poll_render::render(output, bundle, &report)
}

fn resolve_agent_id(flag: Option<String>) -> String {
    if let Some(id) = flag {
        return id;
    }
    if let Ok(id) = std::env::var("CONVERGIO_AGENT_ID") {
        if !id.is_empty() {
            return id;
        }
    }
    let user = std::env::var("USER").unwrap_or_else(|_| "anon".to_string());
    format!("claude-code-{user}")
}

/// `uname -n` with a safe fallback when the binary is missing or the
/// command fails (e.g. exotic CI sandboxes).
fn uname_n() -> String {
    std::process::Command::new("uname")
        .arg("-n")
        .output()
        .ok()
        .and_then(|o| {
            if o.status.success() {
                Some(String::from_utf8_lossy(&o.stdout).trim().to_string())
            } else {
                None
            }
        })
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "localhost".to_string())
}

async fn register(
    client: &Client,
    agent_id: &str,
    kind: &str,
    capabilities: &[String],
    host: &str,
) -> Result<Value> {
    let body = json!({
        "id": agent_id,
        "kind": kind,
        "host": host,
        "capabilities": capabilities,
    });
    client
        .post::<_, Value>("/v1/agent-registry/agents", &body)
        .await
        .context("POST /v1/agent-registry/agents")
}

async fn heartbeat(client: &Client, agent_id: &str) -> Result<Value> {
    let body = json!({"status": "idle"});
    client
        .post::<_, Value>(
            &format!("/v1/agent-registry/agents/{agent_id}/heartbeat"),
            &body,
        )
        .await
        .with_context(|| format!("POST heartbeat for {agent_id}"))
}

async fn active_plans(client: &Client) -> Result<Vec<PlanRef>> {
    let plans: Vec<PlanRef> = client.get("/v1/plans").await.context("GET /v1/plans")?;
    // The HTTP route does not filter; an `active` plan is `draft`
    // or `active` in durability terms. Terminal plans are skipped
    // because polling them would only surface stale traffic.
    Ok(plans
        .into_iter()
        .filter(|p| matches!(p.status.as_str(), "draft" | "active"))
        .collect())
}

async fn poll_topic(client: &Client, plan_id: &str, topic: &str) -> Result<Vec<Value>> {
    let path = format!("/v1/plans/{plan_id}/messages?topic={topic}&limit=20");
    client
        .get::<Vec<Value>>(&path)
        .await
        .with_context(|| format!("GET {path}"))
}

/// Best-effort ack of each message; logs failures, returns success count.
async fn ack_messages(client: &Client, msgs: &[Value], consumer: &str) -> usize {
    let mut count = 0usize;
    for msg in msgs {
        let Some(id) = msg.get("id").and_then(Value::as_str) else {
            continue;
        };
        let body = json!({"consumer": consumer});
        let res: Result<Value> = client.post(&format!("/v1/messages/{id}/ack"), &body).await;
        if let Err(e) = res {
            eprintln!("warning: ack {id} failed: {e}");
        } else {
            count += 1;
        }
    }
    count
}

async fn announce_session_start(
    client: &Client,
    plans: &[PlanRef],
    agent_id: &str,
    kind: &str,
    capabilities: &[String],
    host: &str,
) -> Result<()> {
    if plans.is_empty() {
        return Ok(());
    }
    let repo_root = std::env::current_dir()
        .map(|p| p.display().to_string())
        .unwrap_or_default();
    let branch = current_branch().unwrap_or_default();
    let payload = json!({
        "type": "session-started",
        "agent_id": agent_id,
        "kind": kind,
        "host": host,
        "capabilities": capabilities,
        "started_at_utc": chrono::Utc::now().to_rfc3339(),
        "repo_root": repo_root,
        "branch": branch,
    });
    for plan in plans {
        let body = json!({
            "topic": COORDINATION_TOPIC,
            "sender": agent_id,
            "payload": payload.clone(),
        });
        client
            .post::<_, Value>(&format!("/v1/plans/{}/messages", plan.id), &body)
            .await
            .with_context(|| format!("publish session-started on plan {}", plan.id))?;
    }
    Ok(())
}

fn current_branch() -> Option<String> {
    let out = std::process::Command::new("git")
        .args(["rev-parse", "--abbrev-ref", "HEAD"])
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    Some(String::from_utf8_lossy(&out.stdout).trim().to_string())
}

/// One plan entry returned by `/v1/plans`. Public to the sibling
/// renderer module so it can format it without re-cloning.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PlanRef {
    /// Plan UUID.
    pub id: String,
    /// Display title.
    #[serde(default)]
    pub title: String,
    /// Plan status (`draft` / `active` after the filter in
    /// [`active_plans`]).
    #[serde(default)]
    pub status: String,
}

/// Inputs the renderer needs to produce one of the three output
/// modes. Borrows everything to avoid cloning the message vectors.
pub struct SessionReport<'a> {
    /// JSON returned by `POST /v1/agent-registry/agents`.
    pub agent: Value,
    /// JSON returned by `POST /v1/agent-registry/agents/:id/heartbeat`.
    pub heartbeat: Value,
    /// Active plans the agent is now visible on.
    pub plans: &'a [PlanRef],
    /// `(plan_id, messages)` for direct (`agent:<id>`) traffic.
    pub direct: &'a [(String, Vec<Value>)],
    /// `(plan_id, messages)` for plan-wide (`plan:<id>`) traffic.
    pub announcements: &'a [(String, Vec<Value>)],
    /// Direct messages acked this run.
    pub acked_direct: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    // Mutex: env-var mutations in a single binary race without serialization.
    static ENV_LOCK: Mutex<()> = Mutex::new(());

    #[test]
    fn agent_id_uses_explicit_flag_first() {
        let _g = ENV_LOCK.lock().unwrap();
        std::env::set_var("CONVERGIO_AGENT_ID", "from-env");
        let id = resolve_agent_id(Some("from-flag".to_string()));
        assert_eq!(id, "from-flag");
        std::env::remove_var("CONVERGIO_AGENT_ID");
    }

    #[test]
    fn agent_id_falls_back_to_user() {
        let _g = ENV_LOCK.lock().unwrap();
        std::env::remove_var("CONVERGIO_AGENT_ID");
        std::env::set_var("USER", "alice");
        let id = resolve_agent_id(None);
        assert_eq!(id, "claude-code-alice");
    }

    #[test]
    fn uname_n_returns_non_empty() {
        let n = uname_n();
        assert!(!n.is_empty());
    }
}
