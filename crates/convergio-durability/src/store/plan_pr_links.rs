//! Write-side store for `plan_pr_links` — records which agent opened
//! a PR for a given plan/task. Read queries live in `agent_queries`
//! (they share the pool; stores are cheap to construct).

use crate::error::{DurabilityError, Result};
use chrono::{DateTime, Utc};
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

/// One `plan_pr_links` row.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanPrLink {
    /// Row id (UUIDv4).
    pub id: String,
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
    /// Row creation time.
    pub created_at: DateTime<Utc>,
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

    /// List links for a given PR (repo slug + number), newest first.
    pub async fn list_by_pr(
        &self,
        repo_slug: &str,
        pr_number: i64,
        limit: i64,
    ) -> Result<Vec<PlanPrLink>> {
        let limit = limit.clamp(1, 100);
        let rows = sqlx::query_as::<_, PlanPrLinkRow>(
            "SELECT id, plan_id, task_id, pr_number, repo_slug, branch, agent_id, created_at \
             FROM plan_pr_links WHERE repo_slug = ? AND pr_number = ? \
             ORDER BY created_at DESC LIMIT ?",
        )
        .bind(repo_slug)
        .bind(pr_number)
        .bind(limit)
        .fetch_all(self.pool.inner())
        .await?;

        rows.into_iter().map(TryInto::try_into).collect()
    }
}

#[derive(sqlx::FromRow)]
struct PlanPrLinkRow {
    id: String,
    plan_id: String,
    task_id: Option<String>,
    pr_number: i64,
    repo_slug: String,
    branch: Option<String>,
    agent_id: Option<String>,
    created_at: String,
}

impl TryFrom<PlanPrLinkRow> for PlanPrLink {
    type Error = DurabilityError;

    fn try_from(r: PlanPrLinkRow) -> Result<Self> {
        let created_at = DateTime::parse_from_rfc3339(&r.created_at)
            .map(|d| d.with_timezone(&Utc))
            .map_err(|_| DurabilityError::NotFound {
                entity: "timestamp",
                id: r.created_at.clone(),
            })?;

        Ok(Self {
            id: r.id,
            plan_id: r.plan_id,
            task_id: r.task_id,
            pr_number: r.pr_number,
            repo_slug: r.repo_slug,
            branch: r.branch,
            agent_id: r.agent_id,
            created_at,
        })
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

        let links = store.list_by_pr("owner/repo", 42, 10).await.unwrap();
        assert_eq!(links.len(), 1);
        assert_eq!(links[0].plan_id, "plan-1");
        assert_eq!(links[0].agent_id.as_deref(), Some("agent-abc"));
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
