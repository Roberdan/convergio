//! Read-only process listing for dashboard surfaces.

use crate::model::{AgentProcess, ProcessStatus};
use crate::{LifecycleError, Result, Supervisor};
use chrono::{DateTime, Utc};

impl Supervisor {
    /// List supervised processes, newest first.
    pub async fn list(&self, limit: i64) -> Result<Vec<AgentProcess>> {
        let rows = sqlx::query_as::<_, ProcessListRow>(
            "SELECT id, kind, command, plan_id, task_id, pid, status, exit_code, \
             last_heartbeat_at, started_at, ended_at FROM agent_processes \
             ORDER BY started_at DESC LIMIT ?",
        )
        .bind(limit.max(0))
        .fetch_all(self.pool().inner())
        .await?;
        rows.into_iter().map(TryInto::try_into).collect()
    }
}

#[derive(sqlx::FromRow)]
struct ProcessListRow {
    id: String,
    kind: String,
    command: String,
    plan_id: Option<String>,
    task_id: Option<String>,
    pid: Option<i64>,
    status: String,
    exit_code: Option<i64>,
    last_heartbeat_at: Option<String>,
    started_at: String,
    ended_at: Option<String>,
}

impl TryFrom<ProcessListRow> for AgentProcess {
    type Error = LifecycleError;
    fn try_from(r: ProcessListRow) -> Result<Self> {
        Ok(AgentProcess {
            id: r.id,
            kind: r.kind,
            command: r.command,
            plan_id: r.plan_id,
            task_id: r.task_id,
            pid: r.pid,
            status: ProcessStatus::parse(&r.status).unwrap_or(ProcessStatus::Failed),
            exit_code: r.exit_code,
            last_heartbeat_at: parse_ts_opt("last_heartbeat_at", r.last_heartbeat_at.as_deref())?,
            started_at: parse_ts("started_at", &r.started_at)?,
            ended_at: parse_ts_opt("ended_at", r.ended_at.as_deref())?,
        })
    }
}

fn parse_ts(field: &'static str, s: &str) -> Result<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(s)
        .map(|d| d.with_timezone(&Utc))
        .map_err(|_| LifecycleError::InvalidTimestamp {
            field,
            value: s.to_string(),
        })
}

fn parse_ts_opt(field: &'static str, s: Option<&str>) -> Result<Option<DateTime<Utc>>> {
    s.map(|value| parse_ts(field, value)).transpose()
}
