//! `cvg agent show <id>` — rich, multi-section view of one agent.
//!
//! Pulls `/v1/agent-registry/agents/:id/details`, which already
//! aggregates current task, plan, leases, recent audit and recent
//! PR links server-side. We just render.

use super::agent_show_render;
use super::{Client, OutputMode};
use anyhow::Result;
use chrono::{DateTime, Utc};
use convergio_i18n::Bundle;
use serde::Deserialize;
use serde_json::Value;

#[derive(Debug, Deserialize)]
pub(super) struct Details {
    pub(super) id: String,
    pub(super) kind: String,
    pub(super) status: String,
    #[serde(default)]
    pub(super) capabilities: Vec<String>,
    #[serde(default)]
    pub(super) last_heartbeat_at: Option<DateTime<Utc>>,
    #[serde(default)]
    pub(super) created_at: Option<DateTime<Utc>>,
    #[serde(default)]
    pub(super) current_task_id: Option<String>,
    #[serde(default)]
    pub(super) current_task_title: Option<String>,
    #[serde(default)]
    pub(super) current_task_status: Option<String>,
    #[serde(default)]
    pub(super) current_task_plan_id: Option<String>,
    #[serde(default)]
    pub(super) current_task_plan_title: Option<String>,
    #[serde(default)]
    pub(super) current_task_started_at: Option<DateTime<Utc>>,

    #[serde(default)]
    pub(super) claimed_tasks: ClaimedTasks,
    #[serde(default)]
    pub(super) last_topic: Option<LastTopic>,

    #[serde(default)]
    pub(super) leases: Vec<LeaseRow>,
    #[serde(default)]
    pub(super) recent_audit: Vec<AuditRow>,
    #[serde(default)]
    pub(super) recent_prs: Vec<PrRow>,
}

#[derive(Debug, Default, Deserialize)]
pub(super) struct ClaimedTasks {
    #[serde(default)]
    pub(super) count: i64,
    #[serde(default)]
    pub(super) tasks: Vec<ClaimedTask>,
}

#[derive(Debug, Deserialize)]
pub(super) struct ClaimedTask {
    pub(super) id: String,
    pub(super) title: String,
    pub(super) status: String,
    pub(super) plan_title: String,
    #[serde(default)]
    pub(super) started_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Deserialize)]
pub(super) struct LastTopic {
    #[serde(default)]
    pub(super) plan_id: Option<String>,
    pub(super) topic: String,
    pub(super) kind: String,
    pub(super) at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub(super) struct LeaseRow {
    pub(super) resource_label: String,
    #[serde(default)]
    pub(super) purpose: Option<String>,
    pub(super) created_at: DateTime<Utc>,
    pub(super) expires_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub(super) struct AuditRow {
    pub(super) seq: i64,
    pub(super) transition: String,
    pub(super) entity_type: String,
    pub(super) entity_id: String,
    pub(super) created_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub(super) struct PrRow {
    pub(super) repo_slug: String,
    pub(super) pr_number: i64,
    #[serde(default)]
    pub(super) branch: Option<String>,
    #[serde(default)]
    pub(super) plan_id: Option<String>,
}

/// Entry point.
pub async fn run(client: &Client, bundle: &Bundle, output: OutputMode, id: &str) -> Result<()> {
    match client
        .get::<Value>(&format!("/v1/agent-registry/agents/{id}/details"))
        .await
    {
        Ok(v) => {
            let parsed: Details = serde_json::from_value(v.clone())?;
            match output {
                OutputMode::Human => agent_show_render::render(bundle, &parsed),
                OutputMode::Json => println!("{}", serde_json::to_string_pretty(&v)?),
                OutputMode::Plain => agent_show_render::render_plain(&parsed),
            }
            Ok(())
        }
        Err(e) => {
            eprintln!("{}", bundle.t("agent-not-found", &[("id", id)]));
            Err(e)
        }
    }
}
