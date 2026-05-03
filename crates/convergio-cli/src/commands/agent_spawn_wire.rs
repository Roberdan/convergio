//! Wire shape and parsing helpers shared with `agent_spawn`. Split
//! out so `agent_spawn.rs` stays under the 300-line cap.

use chrono::{DateTime, Utc};
use convergio_durability::{Task, TaskStatus};
use serde::Deserialize;

/// Wire shape returned by `GET /v1/tasks/:id`. Mirrors
/// [`convergio_durability::Task`] but accepts string statuses
/// (the daemon serialises the enum as snake_case).
#[derive(Deserialize)]
pub(crate) struct TaskWire {
    id: String,
    plan_id: String,
    wave: i64,
    sequence: i64,
    title: String,
    description: Option<String>,
    status: String,
    agent_id: Option<String>,
    #[serde(default)]
    evidence_required: Vec<String>,
    last_heartbeat_at: Option<String>,
    created_at: String,
    updated_at: String,
    #[serde(default)]
    started_at: Option<String>,
    #[serde(default)]
    ended_at: Option<String>,
    #[serde(default)]
    duration_ms: Option<i64>,
    #[serde(default)]
    runner_kind: Option<String>,
    #[serde(default)]
    profile: Option<String>,
    #[serde(default)]
    max_budget_usd: Option<f32>,
}

impl TaskWire {
    pub(crate) fn into_task(self) -> Task {
        Task {
            id: self.id,
            plan_id: self.plan_id,
            wave: self.wave,
            sequence: self.sequence,
            title: self.title,
            description: self.description,
            status: TaskStatus::parse(&self.status).unwrap_or(TaskStatus::Pending),
            agent_id: self.agent_id,
            evidence_required: self.evidence_required,
            last_heartbeat_at: parse_ts_opt(self.last_heartbeat_at.as_deref()),
            created_at: parse_ts(&self.created_at).unwrap_or_else(Utc::now),
            updated_at: parse_ts(&self.updated_at).unwrap_or_else(Utc::now),
            started_at: parse_ts_opt(self.started_at.as_deref()),
            ended_at: parse_ts_opt(self.ended_at.as_deref()),
            duration_ms: self.duration_ms,
            runner_kind: self.runner_kind,
            profile: self.profile,
            max_budget_usd: self.max_budget_usd,
        }
    }
}

fn parse_ts(s: &str) -> Option<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(s)
        .ok()
        .map(|t| t.with_timezone(&Utc))
}

fn parse_ts_opt(s: Option<&str>) -> Option<DateTime<Utc>> {
    s.and_then(parse_ts)
}
