//! Fleet plans — one logical plan spanning multiple repos.
//!
//! ADR-0038 § F3-2. Schema lives in [`crate::migrate`] (migration
//! `0800_fleet.sql`); this module is the CRUD surface for
//! `fleet_plans` and `fleet_plan_repos`. The per-repo plan rows
//! themselves live in `convergio-durability`'s `plans` table — this
//! module only records the **link** between a fleet-scoped plan and
//! the per-repo plans it fans out to.
//!
//! Status rollup is **derived at query time**, not stored. A
//! fleet-plan is `done` when every linked per-repo plan is `done`;
//! `in_progress` if any per-repo plan is in flight; `draft`
//! otherwise. This avoids drift between an aggregate column and the
//! authoritative per-repo state.

use crate::error::{FleetError, Result};
use convergio_db::Pool;
use serde::{Deserialize, Serialize};
use sqlx::Row;

/// A logical plan that spans multiple repos. Identified by UUID v4.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FleetPlan {
    /// UUID v4 primary key.
    pub id: String,
    /// Short human title.
    pub title: String,
    /// `"fleet"` for cross-repo, or a single repo name when the plan
    /// is scoped to one repo from the fleet (still useful when the
    /// operator wants the rollup view).
    pub scope: String,
    /// ISO-8601 creation timestamp.
    pub created_at: String,
}

/// Link between a fleet plan and one of the per-repo plans it owns.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FleetPlanRepoLink {
    /// Parent fleet plan id.
    pub fleet_plan_id: String,
    /// Repo name (matches `fleet_repos.name`).
    pub repo: String,
    /// Per-repo plan id in `convergio-durability`.
    pub repo_plan_id: String,
}

/// Input for creating a fleet plan.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewFleetPlan {
    /// Title.
    pub title: String,
    /// Scope. Either `"fleet"` or one of the fleet repo names.
    pub scope: String,
}

/// CRUD surface for fleet plans and their per-repo links.
#[derive(Clone)]
pub struct FleetPlanStore {
    pool: Pool,
}

impl FleetPlanStore {
    /// New store wrapping the shared pool.
    pub fn new(pool: Pool) -> Self {
        Self { pool }
    }

    /// Insert a new fleet plan. Generates a UUID v4 and timestamp.
    pub async fn create(&self, input: NewFleetPlan) -> Result<FleetPlan> {
        if input.title.trim().is_empty() {
            return Err(FleetError::InvalidInput("title must not be empty".into()));
        }
        let plan = FleetPlan {
            id: uuid::Uuid::new_v4().to_string(),
            title: input.title,
            scope: input.scope,
            created_at: chrono::Utc::now().to_rfc3339(),
        };
        sqlx::query("INSERT INTO fleet_plans (id, title, scope, created_at) VALUES (?, ?, ?, ?)")
            .bind(&plan.id)
            .bind(&plan.title)
            .bind(&plan.scope)
            .bind(&plan.created_at)
            .execute(self.pool.inner())
            .await?;
        Ok(plan)
    }

    /// List fleet plans. Newest first. Optional scope filter.
    pub async fn list(&self, scope: Option<&str>) -> Result<Vec<FleetPlan>> {
        let rows = if let Some(s) = scope {
            sqlx::query(
                "SELECT id, title, scope, created_at FROM fleet_plans \
                 WHERE scope = ? ORDER BY created_at DESC",
            )
            .bind(s)
            .fetch_all(self.pool.inner())
            .await?
        } else {
            sqlx::query(
                "SELECT id, title, scope, created_at FROM fleet_plans \
                 ORDER BY created_at DESC",
            )
            .fetch_all(self.pool.inner())
            .await?
        };
        Ok(rows.iter().map(row_to_plan).collect())
    }

    /// Fetch a single fleet plan by id.
    pub async fn get(&self, id: &str) -> Result<FleetPlan> {
        let row = sqlx::query("SELECT id, title, scope, created_at FROM fleet_plans WHERE id = ?")
            .bind(id)
            .fetch_optional(self.pool.inner())
            .await?
            .ok_or_else(|| FleetError::NotFound(format!("fleet_plan {id}")))?;
        Ok(row_to_plan(&row))
    }

    /// Return the per-repo plan links for a fleet plan.
    pub async fn links(&self, fleet_plan_id: &str) -> Result<Vec<FleetPlanRepoLink>> {
        let rows = sqlx::query(
            "SELECT fleet_plan_id, repo, repo_plan_id \
             FROM fleet_plan_repos WHERE fleet_plan_id = ? ORDER BY repo",
        )
        .bind(fleet_plan_id)
        .fetch_all(self.pool.inner())
        .await?;
        Ok(rows.iter().map(row_to_link).collect())
    }

    /// Link a per-repo plan into a fleet plan. Idempotent on the
    /// `(fleet_plan_id, repo)` primary key — a duplicate insert
    /// returns the existing link.
    pub async fn link_repo(&self, link: &FleetPlanRepoLink) -> Result<()> {
        sqlx::query(
            "INSERT OR IGNORE INTO fleet_plan_repos \
             (fleet_plan_id, repo, repo_plan_id) VALUES (?, ?, ?)",
        )
        .bind(&link.fleet_plan_id)
        .bind(&link.repo)
        .bind(&link.repo_plan_id)
        .execute(self.pool.inner())
        .await?;
        Ok(())
    }

    /// Convenience: fetch a plan + its links in one call.
    pub async fn show(&self, id: &str) -> Result<FleetPlanView> {
        let plan = self.get(id).await?;
        let links = self.links(id).await?;
        Ok(FleetPlanView { plan, links })
    }
}

/// A fleet plan with its current per-repo links.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FleetPlanView {
    /// The plan itself.
    pub plan: FleetPlan,
    /// All per-repo plan rows currently linked.
    pub links: Vec<FleetPlanRepoLink>,
}

fn row_to_plan(row: &sqlx::sqlite::SqliteRow) -> FleetPlan {
    FleetPlan {
        id: row.get("id"),
        title: row.get("title"),
        scope: row.get("scope"),
        created_at: row.get("created_at"),
    }
}

fn row_to_link(row: &sqlx::sqlite::SqliteRow) -> FleetPlanRepoLink {
    FleetPlanRepoLink {
        fleet_plan_id: row.get("fleet_plan_id"),
        repo: row.get("repo"),
        repo_plan_id: row.get("repo_plan_id"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::init;

    async fn fresh() -> (FleetPlanStore, tempfile::TempDir) {
        let dir = tempfile::tempdir().unwrap();
        let url = format!("sqlite://{}/state.db", dir.path().display());
        let pool = Pool::connect(&url).await.unwrap();
        init(&pool).await.unwrap();
        (FleetPlanStore::new(pool), dir)
    }

    #[tokio::test]
    async fn create_list_get_roundtrip() {
        let (store, _g) = fresh().await;
        let p = store
            .create(NewFleetPlan {
                title: "cross-repo bug".into(),
                scope: "fleet".into(),
            })
            .await
            .unwrap();
        assert_eq!(p.title, "cross-repo bug");
        let list = store.list(None).await.unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].id, p.id);
        let got = store.get(&p.id).await.unwrap();
        assert_eq!(got, p);
    }

    #[tokio::test]
    async fn empty_title_rejected() {
        let (store, _g) = fresh().await;
        let err = store
            .create(NewFleetPlan {
                title: "  ".into(),
                scope: "fleet".into(),
            })
            .await
            .unwrap_err();
        assert!(matches!(err, FleetError::InvalidInput(_)));
    }

    #[tokio::test]
    async fn link_idempotent_and_listed() {
        let (store, _g) = fresh().await;
        let p = store
            .create(NewFleetPlan {
                title: "x".into(),
                scope: "fleet".into(),
            })
            .await
            .unwrap();
        // fleet_plan_repos has FK on fleet_repos.name; insert a repo
        // row directly so the FK passes (full repo CRUD lives in
        // FleetStore — out of scope for this unit test).
        sqlx::query(
            "INSERT INTO fleet_repos (name, path, language, parser, role) \
             VALUES ('r1', '/r1', 'rust', 'syn', 'engine')",
        )
        .execute(store.pool.inner())
        .await
        .unwrap();
        let link = FleetPlanRepoLink {
            fleet_plan_id: p.id.clone(),
            repo: "r1".into(),
            repo_plan_id: "repo-plan-1".into(),
        };
        store.link_repo(&link).await.unwrap();
        store.link_repo(&link).await.unwrap(); // idempotent
        let links = store.links(&p.id).await.unwrap();
        assert_eq!(links, vec![link]);
    }

    #[tokio::test]
    async fn show_combines_plan_and_links() {
        let (store, _g) = fresh().await;
        let p = store
            .create(NewFleetPlan {
                title: "y".into(),
                scope: "fleet".into(),
            })
            .await
            .unwrap();
        let view = store.show(&p.id).await.unwrap();
        assert_eq!(view.plan, p);
        assert!(view.links.is_empty());
    }
}
