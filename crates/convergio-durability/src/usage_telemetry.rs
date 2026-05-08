//! Token-usage evidence aggregation.
//!
//! Evidence rows are immutable; this module maintains a convenience
//! aggregate in the agent registry metadata so dashboards can display
//! cost/tokens without scanning evidence history.

use crate::error::Result;
use crate::usage::{merge_usage_into_agent_metadata, UsagePayload};
use chrono::Utc;
use serde_json::{Map, Value};
use sqlx::{Row, Sqlite, Transaction};

/// Best-effort aggregation: returns `Ok(false)` when the task has no agent
/// or the agent row is missing.
pub(crate) async fn apply_usage_evidence_tx(
    tx: &mut Transaction<'_, Sqlite>,
    agent_id: Option<&str>,
    usage: &UsagePayload,
) -> Result<bool> {
    let Some(agent_id) = agent_id else {
        return Ok(false);
    };

    let row = sqlx::query("SELECT metadata FROM agents WHERE id = ? LIMIT 1")
        .bind(agent_id)
        .fetch_optional(&mut **tx)
        .await?;
    let Some(row) = row else {
        return Ok(false);
    };

    let raw: String = row.try_get("metadata")?;
    let mut metadata: Value =
        serde_json::from_str(&raw).unwrap_or_else(|_| Value::Object(Map::new()));
    merge_usage_into_agent_metadata(&mut metadata, usage);

    sqlx::query("UPDATE agents SET metadata = ?, updated_at = ? WHERE id = ?")
        .bind(serde_json::to_string(&metadata)?)
        .bind(Utc::now().to_rfc3339())
        .bind(agent_id)
        .execute(&mut **tx)
        .await?;

    Ok(true)
}
