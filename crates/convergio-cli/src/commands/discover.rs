//! `cvg discover` — one-shot peer + bus + plan snapshot for fresh agents.
//!
//! Answers "who can I talk to right now?" without curl + jq plumbing.
//! Three sections (active peers, top-N bus topics, the caller's plans)
//! across human / json / plain. Identity resolution mirrors
//! `cvg status --mine`: `--agent-id` flag → `CONVERGIO_AGENT_ID` env →
//! `claude-code-${USER}`. Uses `serde_json::Value` directly to keep
//! the file under the 300-line cap (CONSTITUTION § 13); rendering
//! lives in [`super::discover_render`] for the same reason.

use super::discover_render::{render_human, render_plain};
use super::{Client, OutputMode};
use anyhow::{anyhow, Result};
use chrono::{DateTime, Duration, Utc};
use convergio_i18n::Bundle;
use serde_json::{json, Value};

/// Args parsed by clap for `cvg discover`.
#[derive(Debug, Clone)]
pub struct DiscoverArgs {
    /// Lookback window. Default `30m`.
    pub since: String,
    /// Identity override (else env, else `claude-code-${USER}`).
    pub agent_id: Option<String>,
}

/// Entry point for `cvg discover`.
pub async fn run(
    client: &Client,
    bundle: &Bundle,
    output: OutputMode,
    args: DiscoverArgs,
) -> Result<()> {
    let cutoff = Utc::now() - parse_since(&args.since)?;
    let me = resolve_agent_id(args.agent_id.as_deref());
    let now = Utc::now();
    let peers = fetch_peers(client, &cutoff).await?;
    let plans: Vec<Value> = client.get("/v1/plans").await.unwrap_or_default();
    let topics = aggregate_topics(client, &plans).await;
    let your_plans = aggregate_your_plans(client, &plans, &me).await;
    match output {
        OutputMode::Human => {
            render_human(bundle, &now, &args.since, &me, &peers, &topics, &your_plans)
        }
        OutputMode::Json => println!(
            "{}",
            serde_json::to_string_pretty(&json!({
                "as_of": now.to_rfc3339(), "since": args.since,
                "agent_id": me, "peers": peers,
                "recent_topics": topics, "your_plans": your_plans,
            }))?
        ),
        OutputMode::Plain => render_plain(&peers, &topics, &your_plans, &now),
    }
    Ok(())
}

/// Parse a `--since` token (`90s`/`30m`/`1h`/`2d`).
pub fn parse_since(input: &str) -> Result<Duration> {
    let t = input.trim();
    if t.is_empty() {
        return Err(anyhow!("empty --since"));
    }
    let (num, unit) = t.split_at(t.len().saturating_sub(1));
    let n: i64 = num
        .parse()
        .map_err(|_| anyhow!("invalid --since: {input}"))?;
    if n < 0 {
        return Err(anyhow!("--since must be non-negative"));
    }
    match unit {
        "s" => Ok(Duration::seconds(n)),
        "m" => Ok(Duration::minutes(n)),
        "h" => Ok(Duration::hours(n)),
        "d" => Ok(Duration::days(n)),
        _ => Err(anyhow!("--since unit must be s/m/h/d, got {unit:?}")),
    }
}

/// Resolve the caller's agent id. Flag → env → `claude-code-${USER}`.
pub fn resolve_agent_id(flag: Option<&str>) -> String {
    if let Some(v) = flag.filter(|s| !s.trim().is_empty()) {
        return v.to_string();
    }
    if let Ok(v) = std::env::var("CONVERGIO_AGENT_ID") {
        if !v.trim().is_empty() {
            return v;
        }
    }
    let user = std::env::var("USER").unwrap_or_else(|_| "anon".into());
    format!("claude-code-{user}")
}

async fn fetch_peers(client: &Client, cutoff: &DateTime<Utc>) -> Result<Vec<Value>> {
    let raw: Vec<Value> = client.get("/v1/agent-registry/agents").await?;
    let mut active: Vec<Value> = raw
        .into_iter()
        .filter(|a| {
            let s = a["status"].as_str().unwrap_or("");
            s != "terminated"
                && s != "retired"
                && a["last_heartbeat_at"]
                    .as_str()
                    .and_then(|t| DateTime::parse_from_rfc3339(t).ok())
                    .map(|t| t.with_timezone(&Utc) >= *cutoff)
                    .unwrap_or(false)
        })
        .collect();
    active.sort_by_key(|a| std::cmp::Reverse(a["last_heartbeat_at"].as_str().map(String::from)));
    Ok(active)
}

async fn aggregate_topics(client: &Client, plans: &[Value]) -> Vec<Value> {
    let mut all: Vec<Value> = Vec::new();
    for p in plans.iter().take(10) {
        let pid = p["id"].as_str().unwrap_or("").to_string();
        let rows: Vec<Value> = client
            .get(&format!("/v1/plans/{pid}/topics"))
            .await
            .unwrap_or_default();
        for r in rows {
            all.push(json!({
                "topic": r["topic"], "plan_id": pid,
                "count": r["count"], "last_at": r["last_at"],
            }));
        }
    }
    all.sort_by_key(|t| std::cmp::Reverse(t["count"].as_i64().unwrap_or(0)));
    all.into_iter().take(5).collect()
}

async fn aggregate_your_plans(client: &Client, plans: &[Value], me: &str) -> Vec<Value> {
    let mut out: Vec<Value> = Vec::new();
    for p in plans.iter().take(10) {
        let pid = p["id"].as_str().unwrap_or("").to_string();
        let tasks: Vec<Value> = client
            .get(&format!("/v1/plans/{pid}/tasks"))
            .await
            .unwrap_or_default();
        let mine: Vec<&Value> = tasks
            .iter()
            .filter(|t| t["agent_id"].as_str() == Some(me))
            .collect();
        if mine.is_empty() {
            continue;
        }
        let open = mine
            .iter()
            .filter(|t| {
                !matches!(
                    t["status"].as_str().unwrap_or(""),
                    "done" | "submitted" | "cancelled"
                )
            })
            .count() as i64;
        out.push(json!({
            "plan_id": pid, "title": p["title"],
            "status": p["status"], "your_tasks_open": open,
        }));
    }
    out
}

// `render_human` and `render_plain` live in `discover_render`.

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn parse_since_units_and_rejects() {
        assert_eq!(parse_since("30m").unwrap(), Duration::minutes(30));
        assert!(parse_since("").is_err() && parse_since("10x").is_err());
    }
    #[test]
    fn resolve_agent_id_prefers_flag() {
        assert_eq!(resolve_agent_id(Some("x")), "x");
    }
}
