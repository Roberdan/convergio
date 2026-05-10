//! Row mapping for durable agent registry persistence.

use crate::error::{DurabilityError, Result};
use crate::store::AgentRecord;
use chrono::{DateTime, Utc};

pub(super) const AGENT_SELECT: &str = "SELECT id, kind, name, host, status, capabilities, \
     current_task_id, metadata, tokens, cost_usd, last_heartbeat_at, created_at, updated_at FROM agents";

#[derive(sqlx::FromRow)]
pub(super) struct AgentRow {
    id: String,
    kind: String,
    name: Option<String>,
    host: Option<String>,
    status: String,
    capabilities: String,
    current_task_id: Option<String>,
    metadata: String,
    tokens: i64,
    cost_usd: f64,
    last_heartbeat_at: Option<String>,
    created_at: String,
    updated_at: String,
}

impl TryFrom<AgentRow> for AgentRecord {
    type Error = DurabilityError;
    fn try_from(row: AgentRow) -> Result<Self> {
        Ok(Self {
            id: row.id,
            kind: row.kind,
            name: row.name,
            host: row.host,
            status: row.status,
            capabilities: serde_json::from_str(&row.capabilities)?,
            current_task_id: row.current_task_id,
            metadata: serde_json::from_str(&row.metadata)?,
            tokens: row.tokens,
            cost_usd: row.cost_usd,
            last_heartbeat_at: parse_optional_time(row.last_heartbeat_at)?,
            created_at: parse_time(&row.created_at)?,
            updated_at: parse_time(&row.updated_at)?,
        })
    }
}

fn parse_optional_time(value: Option<String>) -> Result<Option<DateTime<Utc>>> {
    value.as_deref().map(parse_time).transpose()
}

fn parse_time(value: &str) -> Result<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(value)
        .map(|d| d.with_timezone(&Utc))
        .map_err(|_| DurabilityError::NotFound {
            entity: "timestamp",
            id: value.to_string(),
        })
}
