//! `cvg session register-and-poll` — SessionStart wiring.
//!
//! Registers in `agent_registry`, heartbeats, lists active plans,
//! pulls direct + plan-wide inbox slices, and (P1-3) auto-acks each
//! unicast `agent:<id>` direct message before rendering so the inbox
//! does not re-surface on every poll. Broadcast topics
//! (`plan:*`, `coordination/*`) are never auto-acked.

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
    /// When `true`, skip auto-acking unicast direct messages.
    pub no_auto_ack: bool,
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
    for plan in &plans {
        let pid = plan.id.as_str();
        let mut inbox = poll_topic(client, pid, &format!("agent:{agent_id}")).await?;
        if !inbox.is_empty() {
            if !args.no_auto_ack {
                auto_ack_unicast(client, &agent_id, &mut inbox).await;
            }
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

/// Ack each unicast message and annotate it with `"consumed": true`
/// on success. Errors are swallowed so a stale row can't abort poll.
async fn auto_ack_unicast(client: &Client, consumer: &str, inbox: &mut [Value]) {
    for m in inbox.iter_mut() {
        let Some(id) = m.get("id").and_then(Value::as_str) else {
            continue;
        };
        let body = serde_json::json!({"consumer": consumer});
        let path = format!("/v1/messages/{id}/ack");
        if client.post::<_, Value>(&path, &body).await.is_ok() {
            if let Some(obj) = m.as_object_mut() {
                obj.insert("consumed".to_string(), Value::Bool(true));
            }
        }
    }
}

async fn poll_topic(client: &Client, plan_id: &str, topic: &str) -> Result<Vec<Value>> {
    let path = format!("/v1/plans/{plan_id}/messages?topic={topic}&limit=20");
    client
        .get::<Vec<Value>>(&path)
        .await
        .with_context(|| format!("GET {path}"))
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

/// One plan entry returned by `/v1/plans`.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PlanRef {
    /// Plan UUID.
    pub id: String,
    /// Display title.
    #[serde(default)]
    pub title: String,
    /// Plan status (`draft` / `active` after [`active_plans`]).
    #[serde(default)]
    pub status: String,
}

/// Borrowed inputs for the three render modes.
pub struct SessionReport<'a> {
    /// JSON from `POST /v1/agent-registry/agents`.
    pub agent: Value,
    /// JSON from `POST /v1/agent-registry/agents/:id/heartbeat`.
    pub heartbeat: Value,
    /// Active plans visible to the agent.
    pub plans: &'a [PlanRef],
    /// `(plan_id, messages)` for direct (`agent:<id>`) traffic.
    pub direct: &'a [(String, Vec<Value>)],
    /// `(plan_id, messages)` for plan-wide (`plan:<id>`) traffic.
    pub announcements: &'a [(String, Vec<Value>)],
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    // Tests in a single binary share process-global env. Without
    // serialization the `USER` / `CONVERGIO_AGENT_ID` writes race
    // and the assertions are flaky.
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
