//! `cvg status --agents` — list live agents from the registry.
//!
//! Calls `GET /v1/agent-registry/agents` and renders a stable table
//! (id, kind, last heartbeat age, leases held). Implements W5 from
//! the production-ready plan.

use super::{Client, OutputMode};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// One row from `/v1/agent-registry/agents`.
#[derive(Debug, Deserialize, Serialize)]
pub struct AgentRow {
    /// Stable agent identifier (e.g. `claude-code-roberdan`).
    pub agent_id: String,
    /// Vendor kind (e.g. `claude-code`, `copilot-local`).
    #[serde(default)]
    pub kind: Option<String>,
    /// Last heartbeat as ISO-8601 (RFC3339).
    #[serde(default)]
    pub last_heartbeat: Option<String>,
    /// Count of leases currently held by this agent.
    #[serde(default)]
    pub leases_held: Option<i64>,
    /// Optional status field forwarded from the daemon.
    #[serde(default)]
    pub status: Option<String>,
}

/// Run `cvg status --agents`.
pub async fn run(client: &Client, output: OutputMode) -> Result<()> {
    let body: Value = client.get("/v1/agent-registry/agents").await?;
    let rows = parse_rows(&body)?;
    match output {
        OutputMode::Json => println!("{}", serde_json::to_string_pretty(&body)?),
        OutputMode::Plain => render_plain(&rows),
        OutputMode::Human => render_human(&rows),
    }
    Ok(())
}

/// Daemon may return either a bare `[…]` array or a `{ "agents": […] }`
/// envelope. Accept both so the CLI does not break when the route
/// gains pagination metadata.
fn parse_rows(body: &Value) -> Result<Vec<AgentRow>> {
    let arr = match body {
        Value::Array(_) => body.clone(),
        Value::Object(map) => map
            .get("agents")
            .cloned()
            .unwrap_or(Value::Array(Vec::new())),
        _ => Value::Array(Vec::new()),
    };
    Ok(serde_json::from_value(arr)?)
}

fn render_plain(rows: &[AgentRow]) {
    println!("agents={}", rows.len());
    for r in rows {
        println!(
            "{}\t{}\t{}\t{}",
            r.agent_id,
            r.kind.as_deref().unwrap_or("-"),
            r.last_heartbeat.as_deref().unwrap_or("-"),
            r.leases_held.unwrap_or(0),
        );
    }
}

fn render_human(rows: &[AgentRow]) {
    if rows.is_empty() {
        println!("No agents registered.");
        return;
    }
    println!(
        "{:<40}  {:<16}  {:>12}  {:>7}",
        "AGENT_ID", "KIND", "LAST_HB", "LEASES"
    );
    for r in rows {
        let age = r
            .last_heartbeat
            .as_deref()
            .and_then(heartbeat_age_secs)
            .map(format_age)
            .unwrap_or_else(|| "-".to_string());
        println!(
            "{:<40}  {:<16}  {:>12}  {:>7}",
            r.agent_id,
            r.kind.as_deref().unwrap_or("-"),
            age,
            r.leases_held.unwrap_or(0),
        );
    }
}

fn heartbeat_age_secs(hb: &str) -> Option<i64> {
    let parsed = chrono::DateTime::parse_from_rfc3339(hb).ok()?;
    let now = chrono::Utc::now();
    Some((now - parsed.with_timezone(&chrono::Utc)).num_seconds())
}

fn format_age(secs: i64) -> String {
    if secs < 0 {
        return "0s".into();
    }
    if secs < 60 {
        return format!("{secs}s");
    }
    if secs < 3600 {
        return format!("{}m", secs / 60);
    }
    if secs < 86_400 {
        return format!("{}h", secs / 3600);
    }
    format!("{}d", secs / 86_400)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_bare_array_envelope() {
        let raw = serde_json::json!([
            {
                "agent_id": "claude-code-rd",
                "kind": "claude-code",
                "last_heartbeat": "2026-05-25T20:00:00Z",
                "leases_held": 2
            }
        ]);
        let rows = parse_rows(&raw).unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].agent_id, "claude-code-rd");
        assert_eq!(rows[0].kind.as_deref(), Some("claude-code"));
        assert_eq!(rows[0].leases_held, Some(2));
    }

    #[test]
    fn parses_object_envelope() {
        let raw = serde_json::json!({
            "agents": [
                {"agent_id": "a1", "kind": "copilot-local"}
            ],
            "total": 1
        });
        let rows = parse_rows(&raw).unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].agent_id, "a1");
    }

    #[test]
    fn parses_missing_agents_field_as_empty() {
        let raw = serde_json::json!({"total": 0});
        let rows = parse_rows(&raw).unwrap();
        assert!(rows.is_empty());
    }

    #[test]
    fn format_age_buckets() {
        assert_eq!(format_age(0), "0s");
        assert_eq!(format_age(45), "45s");
        assert_eq!(format_age(125), "2m");
        assert_eq!(format_age(7200), "2h");
        assert_eq!(format_age(172_800), "2d");
    }

    #[test]
    fn human_render_with_no_agents_is_friendly() {
        // Indirect: just make sure render_human does not panic on empty
        render_human(&[]);
    }
}
