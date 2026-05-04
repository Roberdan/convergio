//! Agent enrichment view types — read-only joins over `agents` /
//! `tasks` / `workspace_leases` / `audit_log` / `plan_pr_links`. Powers
//! `cvg agent list/show/retire-stale --dry-run`. Never mutates state.

use crate::store::AgentRecord;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Enriched `cvg agent list` row. `None` when nothing to link.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(missing_docs)]
pub struct AgentSummary {
    #[serde(flatten)]
    pub agent: AgentRecord,
    pub current_task_title: Option<String>,
    pub current_task_status: Option<String>,
    pub recent_branch: Option<String>,
    pub recent_pr_number: Option<i64>,
    pub active_leases: i64,
    pub last_audit_kind: Option<String>,
    pub last_audit_at: Option<DateTime<Utc>>,
}

/// Active workspace lease (show view).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(missing_docs)]
pub struct AgentLease {
    pub id: String,
    pub resource_label: String,
    pub purpose: Option<String>,
    pub created_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
}

/// Audit entry (show view).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(missing_docs)]
pub struct AgentAuditEntry {
    pub seq: i64,
    pub transition: String,
    pub entity_type: String,
    pub entity_id: String,
    pub created_at: DateTime<Utc>,
    pub payload: Value,
}

/// PR link (show view).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(missing_docs)]
pub struct AgentPrLink {
    pub repo_slug: String,
    pub pr_number: i64,
    pub branch: Option<String>,
    pub plan_id: String,
    pub task_id: Option<String>,
    pub created_at: DateTime<Utc>,
}

/// Per-agent action from `retire_stale_agents`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(missing_docs)]
pub struct StaleAgentReport {
    pub agent_id: String,
    pub last_heartbeat_at: Option<DateTime<Utc>>,
    pub previous_status: String,
    pub retired: bool,
}

/// Aggregate result of `retire_stale_agents`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(missing_docs)]
pub struct RetireStaleResult {
    pub threshold_seconds: i64,
    pub applied: bool,
    pub agents: Vec<StaleAgentReport>,
}

/// Plan metadata for current task — used by show aggregator.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(missing_docs)]
pub struct CurrentTaskMeta {
    pub plan_id: String,
    pub plan_title: String,
    pub started_at: Option<DateTime<Utc>>,
}

/// Decode a JSON payload pulled from the audit log; degrade
/// gracefully on legacy rows.
pub(super) fn decode_payload(raw: &str) -> Value {
    serde_json::from_str(raw).unwrap_or_else(|_| Value::String(raw.to_string()))
}

/// Build a `kind:project:path[#symbol]` resource label, matching
/// what `cvg workspace lease` already prints.
pub(super) fn resource_label(
    kind: &str,
    project: Option<&str>,
    path: &str,
    symbol: Option<&str>,
) -> String {
    let mut label = format!("{kind}:{}:{path}", project.unwrap_or(""));
    if let Some(sym) = symbol {
        label.push('#');
        label.push_str(sym);
    }
    label
}
