//! Write-side store for `plan_pr_links` — records which agent opened
//! a PR for a given plan/task. Read queries live in `agent_queries`
//! (they share the pool; stores are cheap to construct).

use crate::error::{DurabilityError, Result};
use chrono::Utc;
use convergio_db::Pool;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Input for inserting a new `plan_pr_links` row.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewPlanPrLink {
    /// Plan this PR belongs to.
    pub plan_id: String,
    /// Task this PR closes (optional).
    pub task_id: Option<String>,
    /// GitHub PR number.
    pub pr_number: i64,
    /// `owner/repo` slug, e.g. `Roberdan/convergio`.
    pub repo_slug: String,
    /// Branch name (best-effort).
    pub branch: Option<String>,
    /// Agent that opened the PR.
    pub agent_id: Option<String>,
}

/// Write-side store for `plan_pr_links`.
#[derive(Clone)]
pub struct PlanPrLinksStore {
    pool: Pool,
}

impl PlanPrLinksStore {
    /// Build from a shared pool.
    pub fn new(pool: Pool) -> Self {
        Self { pool }
    }

    /// Insert or update a plan↔PR link.
    ///
    /// On conflict (same `repo_slug`, `pr_number`, `plan_id`) the
    /// existing row is updated with the latest `agent_id` and `branch`
    /// so re-runs are idempotent.
    pub async fn add(&self, link: NewPlanPrLink) -> Result<()> {
        // Verify plan exists.
        let exists: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM plans WHERE id = ?)")
            .bind(&link.plan_id)
            .fetch_one(self.pool.inner())
            .await?;
        if !exists {
            return Err(DurabilityError::NotFound {
                entity: "plan",
                id: link.plan_id,
            });
        }

        let id = Uuid::new_v4().to_string();
        let now = Utc::now().to_rfc3339();

        sqlx::query(
            "INSERT INTO plan_pr_links \
               (id, plan_id, task_id, pr_number, repo_slug, branch, agent_id, created_at) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?) \
             ON CONFLICT(repo_slug, pr_number, plan_id) \
             DO UPDATE SET agent_id = excluded.agent_id, \
                           branch   = excluded.branch, \
                           task_id  = COALESCE(excluded.task_id, plan_pr_links.task_id)",
        )
        .bind(&id)
        .bind(&link.plan_id)
        .bind(&link.task_id)
        .bind(link.pr_number)
        .bind(&link.repo_slug)
        .bind(&link.branch)
        .bind(&link.agent_id)
        .bind(&now)
        .execute(self.pool.inner())
        .await?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use convergio_db::Pool;
    use tempfile::tempdir;

    async fn fresh_pool() -> (Pool, tempfile::TempDir) {
        let dir = tempdir().unwrap();
        let url = format!("sqlite://{}/state.db", dir.path().display());
        let pool = Pool::connect(&url).await.unwrap();
        crate::migrate::init(&pool).await.unwrap();
        (pool, dir)
    }

    #[tokio::test]
    async fn add_creates_link() {
        let (pool, _dir) = fresh_pool().await;
        // Create a plan first.
        sqlx::query(
            "INSERT INTO plans (id, title, status, created_at, updated_at) \
             VALUES ('plan-1', 'Test', 'draft', '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z')",
        )
        .execute(pool.inner())
        .await
        .unwrap();

        let store = PlanPrLinksStore::new(pool.clone());
        store
            .add(NewPlanPrLink {
                plan_id: "plan-1".into(),
                task_id: None,
                pr_number: 42,
                repo_slug: "owner/repo".into(),
                branch: Some("feat/my-branch".into()),
                agent_id: Some("agent-abc".into()),
            })
            .await
            .unwrap();

        let count: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM plan_pr_links WHERE plan_id = 'plan-1'")
                .fetch_one(pool.inner())
                .await
                .unwrap();
        assert_eq!(count, 1);
    }

    #[tokio::test]
    async fn add_is_idempotent() {
        let (pool, _dir) = fresh_pool().await;
        sqlx::query(
            "INSERT INTO plans (id, title, status, created_at, updated_at) \
             VALUES ('plan-2', 'Test', 'draft', '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z')",
        )
        .execute(pool.inner())
        .await
        .unwrap();

        let store = PlanPrLinksStore::new(pool.clone());
        let link = NewPlanPrLink {
            plan_id: "plan-2".into(),
            task_id: None,
            pr_number: 7,
            repo_slug: "owner/repo".into(),
            branch: None,
            agent_id: Some("agent-xyz".into()),
        };
        store.add(link.clone()).await.unwrap();
        store.add(link).await.unwrap();

        let count: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM plan_pr_links WHERE plan_id = 'plan-2'")
                .fetch_one(pool.inner())
                .await
                .unwrap();
        assert_eq!(count, 1, "upsert must not duplicate rows");
    }

    #[tokio::test]
    async fn add_unknown_plan_returns_error() {
        let (pool, _dir) = fresh_pool().await;
        let store = PlanPrLinksStore::new(pool);
        let err = store
            .add(NewPlanPrLink {
                plan_id: "nonexistent".into(),
                task_id: None,
                pr_number: 1,
                repo_slug: "owner/repo".into(),
                branch: None,
                agent_id: None,
            })
            .await;
        assert!(err.is_err());
    }
}
