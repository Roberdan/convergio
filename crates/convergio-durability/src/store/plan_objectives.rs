//! `plan_objectives` table DAO. Single-row-per-plan OKR objective.
//!
//! Introduced by W4 (ADR-0055). The `PlanCoherenceGate` reads from
//! this store to refuse `task.submitted` when the plan has no
//! objective set.

use crate::error::Result;
use chrono::Utc;
use convergio_db::Pool;

/// Read/write access to the `plan_objectives` table.
#[derive(Clone)]
pub struct PlanObjectiveStore {
    pool: Pool,
}

/// One row from `plan_objectives`.
#[derive(Debug, Clone)]
pub struct PlanObjective {
    /// Plan id the objective applies to.
    pub plan_id: String,
    /// The objective statement.
    pub objective: String,
    /// Unix seconds; first time the objective was set.
    pub created_at: i64,
    /// Unix seconds; last time it was changed.
    pub updated_at: i64,
}

impl PlanObjectiveStore {
    /// Wrap a pool.
    pub fn new(pool: Pool) -> Self {
        Self { pool }
    }

    /// Fetch the objective for a plan, or `None`.
    pub async fn get(&self, plan_id: &str) -> Result<Option<PlanObjective>> {
        let row = sqlx::query_as::<_, (String, String, i64, i64)>(
            "SELECT plan_id, objective, created_at, updated_at \
             FROM plan_objectives WHERE plan_id = ? LIMIT 1",
        )
        .bind(plan_id)
        .fetch_optional(self.pool.inner())
        .await?;
        Ok(row.map(
            |(plan_id, objective, created_at, updated_at)| PlanObjective {
                plan_id,
                objective,
                created_at,
                updated_at,
            },
        ))
    }

    /// Upsert the objective for a plan.
    pub async fn set(&self, plan_id: &str, objective: &str) -> Result<PlanObjective> {
        let now = Utc::now().timestamp();
        sqlx::query(
            "INSERT INTO plan_objectives (plan_id, objective, created_at, updated_at) \
             VALUES (?, ?, ?, ?) \
             ON CONFLICT(plan_id) DO UPDATE SET \
               objective = excluded.objective, \
               updated_at = excluded.updated_at",
        )
        .bind(plan_id)
        .bind(objective)
        .bind(now)
        .bind(now)
        .execute(self.pool.inner())
        .await?;
        // Re-read to surface canonical created_at when row pre-existed.
        Ok(self.get(plan_id).await?.unwrap_or(PlanObjective {
            plan_id: plan_id.to_string(),
            objective: objective.to_string(),
            created_at: now,
            updated_at: now,
        }))
    }

    /// True when an objective row exists.
    pub async fn exists(&self, plan_id: &str) -> Result<bool> {
        Ok(self.get(plan_id).await?.is_some())
    }
}
