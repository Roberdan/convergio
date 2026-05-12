//! Claimed-task projections for the durable agent registry.
//!
//! Powers the operator UX gap "what is the agent doing?" by showing
//! tasks still owned by an agent even when `agents.current_task_id`
//! is missing or stale.

use crate::error::Result;
use crate::store::agent_summary::{ClaimedTask, ClaimedTasks};
use crate::store::AgentStore;
use chrono::{DateTime, Utc};
use sqlx::Row;

/// Parse a persisted RFC-3339 timestamp from a projection row.
/// Corrupt values are surfaced as a typed [`crate::DurabilityError`]
/// rather than masked with `Utc::now()`, which would make a stale or
/// tampered task ownership row look freshly written.
fn parse_ts(value: &str) -> Result<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(value)
        .map(|d| d.with_timezone(&Utc))
        .map_err(|_| crate::error::DurabilityError::NotFound {
            entity: "timestamp",
            id: value.to_string(),
        })
}

fn parse_ts_opt(value: Option<String>) -> Result<Option<DateTime<Utc>>> {
    match value {
        None => Ok(None),
        Some(s) => parse_ts(&s).map(Some),
    }
}

impl AgentStore {
    /// Tasks still owned by `agent_id` in a non-terminal state.
    ///
    /// Returns total count + up to `limit` most recently updated rows.
    pub async fn claimed_tasks_for_agent(
        &self,
        agent_id: &str,
        limit: i64,
    ) -> Result<ClaimedTasks> {
        let limit = limit.clamp(0, 50);
        let count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM tasks WHERE agent_id = ? AND status IN ('in_progress', 'submitted')",
        )
        .bind(agent_id)
        .fetch_one(self.pool().inner())
        .await?;

        if limit == 0 {
            return Ok(ClaimedTasks {
                count,
                tasks: Vec::new(),
            });
        }

        let rows = sqlx::query(
            "SELECT t.id, t.title, t.status, t.plan_id, p.title AS plan_title, \
                    t.started_at, t.updated_at \
             FROM tasks t JOIN plans p ON p.id = t.plan_id \
             WHERE t.agent_id = ? AND t.status IN ('in_progress', 'submitted') \
             ORDER BY t.updated_at DESC LIMIT ?",
        )
        .bind(agent_id)
        .bind(limit)
        .fetch_all(self.pool().inner())
        .await?;

        let mut tasks = Vec::with_capacity(rows.len());
        for r in rows {
            let started_at: Option<String> = r.try_get("started_at")?;
            let updated_at: String = r.try_get("updated_at")?;
            tasks.push(ClaimedTask {
                id: r.try_get("id")?,
                title: r.try_get("title")?,
                status: r.try_get("status")?,
                plan_id: r.try_get("plan_id")?,
                plan_title: r.try_get("plan_title")?,
                started_at: parse_ts_opt(started_at)?,
                updated_at: parse_ts(&updated_at)?,
            });
        }

        Ok(ClaimedTasks { count, tasks })
    }
}
