//! `cvg agent retire-stale` — bulk-retire agents whose heartbeat is
//! older than `--threshold-min` minutes.
//!
//! Defaults to dry-run: prints the agents that *would* be retired,
//! but writes nothing. Pass `--apply` to actually retire and emit
//! the audit row.

use super::agent_format::relative_ago_opt;
use super::{Client, OutputMode};
use anyhow::Result;
use chrono::{DateTime, Utc};
use convergio_i18n::Bundle;
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Arguments parsed by clap.
#[derive(Debug, Clone)]
pub struct RetireArgs {
    /// Heartbeat staleness threshold, minutes.
    pub threshold_min: i64,
    /// `true` to actually retire (default: dry-run).
    pub apply: bool,
}

impl Default for RetireArgs {
    fn default() -> Self {
        Self {
            threshold_min: 60,
            apply: false,
        }
    }
}

#[derive(Debug, Serialize)]
struct Body {
    threshold_seconds: i64,
    apply: bool,
}

#[derive(Debug, Deserialize)]
struct StaleAgent {
    agent_id: String,
    #[serde(default)]
    last_heartbeat_at: Option<DateTime<Utc>>,
    previous_status: String,
    retired: bool,
}

#[derive(Debug, Deserialize)]
struct Response {
    threshold_seconds: i64,
    applied: bool,
    agents: Vec<StaleAgent>,
}

/// Entry point.
pub async fn run(
    client: &Client,
    bundle: &Bundle,
    output: OutputMode,
    args: RetireArgs,
) -> Result<()> {
    let body = Body {
        threshold_seconds: args.threshold_min.saturating_mul(60),
        apply: args.apply,
    };
    let raw: Value = client
        .post("/v1/agent-registry/agents/retire-stale", &body)
        .await?;
    let parsed: Response = serde_json::from_value(raw.clone())?;
    match output {
        OutputMode::Human => render_human(bundle, &parsed),
        OutputMode::Json => println!("{}", serde_json::to_string_pretty(&raw)?),
        OutputMode::Plain => render_plain(&parsed),
    }
    Ok(())
}

fn render_human(bundle: &Bundle, r: &Response) {
    let mins = (r.threshold_seconds / 60).to_string();
    let key = if r.applied {
        "agent-retire-stale-summary"
    } else {
        "agent-retire-stale-dry-run"
    };
    let count = r.agents.len().to_string();
    println!(
        "{}",
        bundle.t_n_with(
            key,
            r.agents.len() as i64,
            &[("count", &count), ("threshold_min", &mins)]
        )
    );
    if r.agents.is_empty() {
        println!("  {}", bundle.t("agent-retire-stale-none", &[]));
        return;
    }
    let now = Utc::now();
    for a in &r.agents {
        let hb = relative_ago_opt(a.last_heartbeat_at.as_ref(), &now);
        let marker = if a.retired { "✓" } else { "·" };
        println!(
            "  {marker} {:<32} {:<12} last_hb={}",
            a.agent_id, a.previous_status, hb
        );
    }
}

fn render_plain(r: &Response) {
    for a in &r.agents {
        let hb = a
            .last_heartbeat_at
            .map(|t| t.to_rfc3339())
            .unwrap_or_else(|| "-".into());
        println!(
            "{}\t{}\t{}\t{}",
            a.agent_id, a.previous_status, hb, a.retired
        );
    }
}
