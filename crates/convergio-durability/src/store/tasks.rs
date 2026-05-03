//! `tasks` table DAO.

use crate::error::{DurabilityError, Result};
use crate::model::{NewTask, RecentCompletedTask, Task, TaskStatus};
use chrono::{DateTime, Utc};
use convergio_db::Pool;
use uuid::Uuid;

/// Read/write access to the `tasks` table.
#[derive(Clone)]
pub struct TaskStore {
    pool: Pool,
}

impl TaskStore {
    /// Wrap a pool.
    pub fn new(pool: Pool) -> Self {
        Self { pool }
    }

    /// Insert a new task with status `pending`.
    pub async fn create(&self, plan_id: &str, input: NewTask) -> Result<Task> {
        let now = Utc::now();
        let task = Task {
            id: Uuid::new_v4().to_string(),
            plan_id: plan_id.to_string(),
            wave: input.wave,
            sequence: input.sequence,
            title: input.title,
            description: input.description,
            status: TaskStatus::Pending,
            agent_id: None,
            evidence_required: input.evidence_required,
            last_heartbeat_at: None,
            created_at: now,
            updated_at: now,
            started_at: None,
            ended_at: None,
            duration_ms: None,
            runner_kind: input.runner_kind,
            profile: input.profile,
            max_budget_usd: input.max_budget_usd,
        };

        sqlx::query(
            "INSERT INTO tasks (id, plan_id, wave, sequence, title, description, status, \
             agent_id, evidence_required, last_heartbeat_at, created_at, updated_at, \
             runner_kind, profile, max_budget_usd) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(&task.id)
        .bind(&task.plan_id)
        .bind(task.wave)
        .bind(task.sequence)
        .bind(&task.title)
        .bind(&task.description)
        .bind(task.status.as_str())
        .bind(&task.agent_id)
        .bind(serde_json::to_string(&task.evidence_required)?)
        .bind(Option::<String>::None)
        .bind(task.created_at.to_rfc3339())
        .bind(task.updated_at.to_rfc3339())
        .bind(&task.runner_kind)
        .bind(&task.profile)
        .bind(task.max_budget_usd)
        .execute(self.pool.inner())
        .await?;

        Ok(task)
    }

    /// Fetch by id, or `NotFound`.
    pub async fn get(&self, id: &str) -> Result<Task> {
        self.find(id)
            .await?
            .ok_or_else(|| DurabilityError::NotFound {
                entity: "task",
                id: id.to_string(),
            })
    }

    /// Fetch by id, returning `None` if absent.
    pub async fn find(&self, id: &str) -> Result<Option<Task>> {
        let row = sqlx::query_as::<_, TaskRow>(SELECT_TASK)
            .bind(id)
            .fetch_optional(self.pool.inner())
            .await?;
        row.map(TryInto::try_into).transpose()
    }

    /// List tasks of a plan ordered by (wave, sequence).
    pub async fn list_by_plan(&self, plan_id: &str) -> Result<Vec<Task>> {
        let rows = sqlx::query_as::<_, TaskRow>(LIST_BY_PLAN)
            .bind(plan_id)
            .fetch_all(self.pool.inner())
            .await?;
        rows.into_iter().map(TryInto::try_into).collect()
    }

    /// List recently completed tasks with plan context, newest first.
    pub async fn list_recent_done(&self, limit: i64) -> Result<Vec<RecentCompletedTask>> {
        let rows = sqlx::query_as::<_, RecentCompletedTaskRow>(
            "SELECT tasks.id, tasks.title, tasks.plan_id, plans.title AS plan_title, \
             plans.project, tasks.updated_at \
             FROM tasks JOIN plans ON plans.id = tasks.plan_id \
             WHERE tasks.status = 'done' \
             ORDER BY tasks.updated_at DESC LIMIT ?",
        )
        .bind(limit)
        .fetch_all(self.pool.inner())
        .await?;
        rows.into_iter().map(TryInto::try_into).collect()
    }

    /// Update the status column. Caller is responsible for the gate pipeline.
    pub async fn set_status(
        &self,
        id: &str,
        status: TaskStatus,
        agent_id: Option<&str>,
    ) -> Result<()> {
        let n =
            sqlx::query("UPDATE tasks SET status = ?, agent_id = ?, updated_at = ? WHERE id = ?")
                .bind(status.as_str())
                .bind(agent_id)
                .bind(Utc::now().to_rfc3339())
                .bind(id)
                .execute(self.pool.inner())
                .await?
                .rows_affected();
        if n == 0 {
            return Err(DurabilityError::NotFound {
                entity: "task",
                id: id.to_string(),
            });
        }
        Ok(())
    }

    /// List stale tasks for a plan: status `pending` or `failed` whose
    /// `updated_at` is before `before`.
    pub async fn list_stale_by_plan(
        &self,
        plan_id: &str,
        before: DateTime<Utc>,
    ) -> Result<Vec<Task>> {
        let rows = sqlx::query_as::<_, TaskRow>(LIST_STALE_BY_PLAN)
            .bind(plan_id)
            .bind(before.to_rfc3339())
            .fetch_all(self.pool.inner())
            .await?;
        rows.into_iter().map(TryInto::try_into).collect()
    }

    /// Touch the heartbeat column.
    pub async fn heartbeat(&self, id: &str) -> Result<()> {
        let n = sqlx::query("UPDATE tasks SET last_heartbeat_at = ? WHERE id = ?")
            .bind(Utc::now().to_rfc3339())
            .bind(id)
            .execute(self.pool.inner())
            .await?
            .rows_affected();
        if n == 0 {
            return Err(DurabilityError::NotFound {
                entity: "task",
                id: id.to_string(),
            });
        }
        Ok(())
    }
}

const SELECT_TASK: &str =
    "SELECT id, plan_id, wave, sequence, title, description, status, agent_id, \
     evidence_required, last_heartbeat_at, created_at, updated_at, \
     started_at, ended_at, duration_ms, runner_kind, profile, max_budget_usd \
     FROM tasks WHERE id = ? LIMIT 1";

const LIST_BY_PLAN: &str =
    "SELECT id, plan_id, wave, sequence, title, description, status, agent_id, \
     evidence_required, last_heartbeat_at, created_at, updated_at, \
     started_at, ended_at, duration_ms, runner_kind, profile, max_budget_usd \
     FROM tasks WHERE plan_id = ? ORDER BY wave ASC, sequence ASC";

const LIST_STALE_BY_PLAN: &str =
    "SELECT id, plan_id, wave, sequence, title, description, status, agent_id, \
     evidence_required, last_heartbeat_at, created_at, updated_at, \
     started_at, ended_at, duration_ms, runner_kind, profile, max_budget_usd \
     FROM tasks WHERE plan_id = ? AND status IN ('pending', 'failed') \
     AND updated_at < ? ORDER BY wave ASC, sequence ASC";

#[derive(sqlx::FromRow)]
struct TaskRow {
    id: String,
    plan_id: String,
    wave: i64,
    sequence: i64,
    title: String,
    description: Option<String>,
    status: String,
    agent_id: Option<String>,
    evidence_required: String,
    last_heartbeat_at: Option<String>,
    created_at: String,
    updated_at: String,
    started_at: Option<String>,
    ended_at: Option<String>,
    duration_ms: Option<i64>,
    runner_kind: Option<String>,
    profile: Option<String>,
    max_budget_usd: Option<f32>,
}

#[derive(sqlx::FromRow)]
struct RecentCompletedTaskRow {
    id: String,
    title: String,
    plan_id: String,
    plan_title: String,
    project: Option<String>,
    updated_at: String,
}

impl TryFrom<TaskRow> for Task {
    type Error = DurabilityError;
    fn try_from(r: TaskRow) -> Result<Self> {
        Ok(Task {
            id: r.id,
            plan_id: r.plan_id,
            wave: r.wave,
            sequence: r.sequence,
            title: r.title,
            description: r.description,
            status: TaskStatus::parse(&r.status).unwrap_or(TaskStatus::Pending),
            agent_id: r.agent_id,
            evidence_required: serde_json::from_str(&r.evidence_required).unwrap_or_default(),
            last_heartbeat_at: r.last_heartbeat_at.as_deref().and_then(parse_ts_opt),
            created_at: parse_ts(&r.created_at)?,
            updated_at: parse_ts(&r.updated_at)?,
            started_at: r.started_at.as_deref().and_then(parse_ts_opt),
            ended_at: r.ended_at.as_deref().and_then(parse_ts_opt),
            duration_ms: r.duration_ms,
            runner_kind: r.runner_kind,
            profile: r.profile,
            max_budget_usd: r.max_budget_usd,
        })
    }
}

impl TryFrom<RecentCompletedTaskRow> for RecentCompletedTask {
    type Error = DurabilityError;
    fn try_from(r: RecentCompletedTaskRow) -> Result<Self> {
        Ok(Self {
            id: r.id,
            title: r.title,
            plan_id: r.plan_id,
            plan_title: r.plan_title,
            project: r.project,
            updated_at: parse_ts(&r.updated_at)?,
        })
    }
}

fn parse_ts(s: &str) -> Result<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(s)
        .map(|d| d.with_timezone(&Utc))
        .map_err(|_| DurabilityError::NotFound {
            entity: "timestamp",
            id: s.to_string(),
        })
}

fn parse_ts_opt(s: &str) -> Option<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(s)
        .ok()
        .map(|d| d.with_timezone(&Utc))
}
