//! `task_taxonomy` + `eval_outcomes` DAO. Skeleton for W10
//! Cost-of-Pass (ADR-0063). W10-full will add the
//! `model_evaluations` view + Thor recorder + MCP `eval.recommend`.

use crate::error::Result;
use chrono::Utc;
use convergio_db::Pool;
use uuid::Uuid;

/// Read-only access to the closed task taxonomy.
#[derive(Clone)]
pub struct TaxonomyStore {
    pool: Pool,
}

/// One row in `eval_outcomes`.
#[derive(Debug, Clone)]
pub struct EvalOutcome {
    /// UUID.
    pub id: String,
    /// Task the verdict was about.
    pub task_id: String,
    /// Plan the task belonged to.
    pub plan_id: String,
    /// `vendor:model` wire string.
    pub runner_kind: String,
    /// Closed taxonomy kind.
    pub taxonomy_kind: String,
    /// Did Thor's pipeline pass overall?
    pub passed: bool,
    /// USD cost reported by the vendor.
    pub cost_usd: Option<f64>,
    /// End-to-end latency in milliseconds.
    pub latency_ms: Option<i64>,
    /// Unix seconds.
    pub recorded_at: i64,
}

/// Input for `EvalOutcomeStore::record`.
#[derive(Debug, Clone)]
pub struct NewEvalOutcome {
    /// Task the verdict applies to.
    pub task_id: String,
    /// Plan id.
    pub plan_id: String,
    /// Runner kind wire string (`vendor:model`).
    pub runner_kind: String,
    /// Closed taxonomy kind — must exist in `task_taxonomy`.
    pub taxonomy_kind: String,
    /// Did the pipeline pass?
    pub passed: bool,
    /// Optional cost in USD.
    pub cost_usd: Option<f64>,
    /// Optional latency in milliseconds.
    pub latency_ms: Option<i64>,
}

impl TaxonomyStore {
    /// Wrap a pool.
    pub fn new(pool: Pool) -> Self {
        Self { pool }
    }
    /// All known taxonomy kinds, sorted.
    pub async fn list(&self) -> Result<Vec<String>> {
        let rows = sqlx::query_as::<_, (String,)>(
            "SELECT kind FROM task_taxonomy ORDER BY kind ASC",
        )
        .fetch_all(self.pool.inner())
        .await?;
        Ok(rows.into_iter().map(|(k,)| k).collect())
    }
    /// True if `kind` is in the closed list.
    pub async fn contains(&self, kind: &str) -> Result<bool> {
        let row = sqlx::query_as::<_, (i64,)>(
            "SELECT COUNT(1) FROM task_taxonomy WHERE kind = ?",
        )
        .bind(kind)
        .fetch_one(self.pool.inner())
        .await?;
        Ok(row.0 > 0)
    }
}

/// Read/write access to `eval_outcomes`.
#[derive(Clone)]
pub struct EvalOutcomeStore {
    pool: Pool,
}

impl EvalOutcomeStore {
    /// Wrap a pool.
    pub fn new(pool: Pool) -> Self {
        Self { pool }
    }
    /// Insert a new eval outcome. Returns the persisted row.
    pub async fn record(&self, input: NewEvalOutcome) -> Result<EvalOutcome> {
        let id = Uuid::new_v4().to_string();
        let recorded_at = Utc::now().timestamp();
        sqlx::query(
            "INSERT INTO eval_outcomes \
             (id, task_id, plan_id, runner_kind, taxonomy_kind, passed, cost_usd, latency_ms, recorded_at) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(&id)
        .bind(&input.task_id)
        .bind(&input.plan_id)
        .bind(&input.runner_kind)
        .bind(&input.taxonomy_kind)
        .bind(if input.passed { 1i64 } else { 0i64 })
        .bind(input.cost_usd)
        .bind(input.latency_ms)
        .bind(recorded_at)
        .execute(self.pool.inner())
        .await?;
        Ok(EvalOutcome {
            id,
            task_id: input.task_id,
            plan_id: input.plan_id,
            runner_kind: input.runner_kind,
            taxonomy_kind: input.taxonomy_kind,
            passed: input.passed,
            cost_usd: input.cost_usd,
            latency_ms: input.latency_ms,
            recorded_at,
        })
    }
    /// Count rows scoped by `(runner_kind, taxonomy_kind)`.
    pub async fn count_for(&self, runner_kind: &str, taxonomy_kind: &str) -> Result<i64> {
        let row = sqlx::query_as::<_, (i64,)>(
            "SELECT COUNT(1) FROM eval_outcomes WHERE runner_kind = ? AND taxonomy_kind = ?",
        )
        .bind(runner_kind)
        .bind(taxonomy_kind)
        .fetch_one(self.pool.inner())
        .await?;
        Ok(row.0)
    }
}
