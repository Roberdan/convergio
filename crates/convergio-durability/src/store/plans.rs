//! `plans` table DAO.

use crate::error::{DurabilityError, Result};
use crate::model::{Plan, PlanStatus};
use chrono::{DateTime, Utc};
use convergio_db::Pool;

const PLAN_SELECT: &str = "SELECT id, number, title, description, project, status, \
    created_at, updated_at, started_at, ended_at, duration_ms, tokens, cost_usd FROM plans ";

/// Read/write access to the `plans` table.
#[derive(Clone)]
pub struct PlanStore {
    pool: Pool,
}

impl PlanStore {
    /// Wrap a pool.
    pub fn new(pool: Pool) -> Self {
        Self { pool }
    }

    /// Fetch by id, or `NotFound`.
    pub async fn get(&self, id: &str) -> Result<Plan> {
        self.find(id)
            .await?
            .ok_or_else(|| DurabilityError::NotFound {
                entity: "plan",
                id: id.to_string(),
            })
    }

    /// Fetch by id, returning `None` if absent.
    pub async fn find(&self, id: &str) -> Result<Option<Plan>> {
        let q = format!("{PLAN_SELECT}WHERE id = ? LIMIT 1");
        let row = sqlx::query_as::<_, PlanRow>(&q)
            .bind(id)
            .fetch_optional(self.pool.inner())
            .await?;
        row.map(TryInto::try_into).transpose()
    }

    /// Fetch by plan number; when multiple projects share the number returns the oldest.
    pub async fn find_by_number(&self, number: i64) -> Result<Option<Plan>> {
        let q = format!("{PLAN_SELECT}WHERE number = ? ORDER BY created_at ASC LIMIT 1");
        let row = sqlx::query_as::<_, PlanRow>(&q)
            .bind(number)
            .fetch_optional(self.pool.inner())
            .await?;
        row.map(TryInto::try_into).transpose()
    }

    /// List plans, newest first.
    pub async fn list(&self, limit: i64) -> Result<Vec<Plan>> {
        let q = format!("{PLAN_SELECT}ORDER BY created_at DESC LIMIT ?");
        let rows = sqlx::query_as::<_, PlanRow>(&q)
            .bind(limit)
            .fetch_all(self.pool.inner())
            .await?;
        rows.into_iter().map(TryInto::try_into).collect()
    }

    /// Allocate the next plan number within the given project group, scoped
    /// to an open SQLite transaction. Caller must have already issued
    /// `BEGIN IMMEDIATE` so the read → INSERT pair is atomic.
    pub async fn next_number_in_tx(
        tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
        project: Option<&str>,
    ) -> Result<i64> {
        let n = if let Some(p) = project {
            sqlx::query_scalar("SELECT COALESCE(MAX(number), 0) + 1 FROM plans WHERE project = ?")
                .bind(p)
                .fetch_one(&mut **tx)
                .await?
        } else {
            sqlx::query_scalar(
                "SELECT COALESCE(MAX(number), 0) + 1 FROM plans WHERE project IS NULL",
            )
            .fetch_one(&mut **tx)
            .await?
        };
        Ok(n)
    }

    /// Update the status column. Caller is responsible for running the
    /// gate pipeline before calling.
    pub async fn set_status(&self, id: &str, status: PlanStatus) -> Result<()> {
        let n = sqlx::query("UPDATE plans SET status = ?, updated_at = ? WHERE id = ?")
            .bind(status.as_str())
            .bind(Utc::now().to_rfc3339())
            .bind(id)
            .execute(self.pool.inner())
            .await?
            .rows_affected();
        if n == 0 {
            return Err(DurabilityError::NotFound {
                entity: "plan",
                id: id.to_string(),
            });
        }
        Ok(())
    }
}

#[derive(sqlx::FromRow)]
struct PlanRow {
    id: String,
    number: i64,
    title: String,
    description: Option<String>,
    project: Option<String>,
    status: String,
    created_at: String,
    updated_at: String,
    started_at: Option<String>,
    ended_at: Option<String>,
    duration_ms: Option<i64>,
    tokens: i64,
    cost_usd: f64,
}

impl TryFrom<PlanRow> for Plan {
    type Error = DurabilityError;
    fn try_from(r: PlanRow) -> Result<Self> {
        Ok(Plan {
            id: r.id,
            number: r.number,
            title: r.title,
            description: r.description,
            project: r.project,
            status: PlanStatus::parse(&r.status).unwrap_or(PlanStatus::Draft),
            created_at: parse_ts(&r.created_at)?,
            updated_at: parse_ts(&r.updated_at)?,
            started_at: r.started_at.as_deref().map(parse_ts).transpose()?,
            ended_at: r.ended_at.as_deref().map(parse_ts).transpose()?,
            duration_ms: r.duration_ms,
            tokens: r.tokens,
            cost_usd: r.cost_usd,
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
