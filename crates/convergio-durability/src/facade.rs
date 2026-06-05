//! `Durability` — the public facade tying stores, gates and audit log
//! together so that callers (HTTP layer, CLI) only see one type.

use crate::audit::{append_tx, AuditLog, EntityKind};
use crate::error::Result;
use crate::gates::{self, Pipeline};
use crate::model::{NewPlan, NewTask, Plan, PlanStatus, Task, TaskStatus};
use crate::ontology_branch_store::OntologyBranchStore;
use crate::store::{
    CrdtStore, EvidenceStore, PlanPrLinksStore, PlanStore, TaskStore, WorkspaceStore, WorktreeStore,
};
use chrono::Utc;
use convergio_db::Pool;
use serde_json::json;
use uuid::Uuid;

/// Top-level Layer 1 handle (cheap to clone; wraps a shared pool).
#[derive(Clone)]
pub struct Durability {
    pool: Pool,
    pipeline: Pipeline,
}

impl Durability {
    /// Build with the [`gates::default_pipeline`].
    pub fn new(pool: Pool) -> Self {
        Self {
            pool,
            pipeline: gates::default_pipeline(),
        }
    }

    /// Underlying pool (for advanced callers that need raw access).
    pub fn pool(&self) -> &Pool {
        &self.pool
    }

    /// Gate pipeline (used by sibling facade modules — not part of the
    /// public API).
    pub(crate) fn pipeline(&self) -> &Pipeline {
        &self.pipeline
    }

    /// Plan store accessor.
    pub fn plans(&self) -> PlanStore {
        PlanStore::new(self.pool.clone())
    }

    /// Task store accessor.
    pub fn tasks(&self) -> TaskStore {
        TaskStore::new(self.pool.clone())
    }

    /// Evidence store accessor.
    pub fn evidence(&self) -> EvidenceStore {
        EvidenceStore::new(self.pool.clone())
    }

    /// CRDT actor/op store accessor.
    pub fn crdt(&self) -> CrdtStore {
        CrdtStore::new(self.pool.clone())
    }

    /// Workspace coordination store accessor.
    pub fn workspace(&self) -> WorkspaceStore {
        WorkspaceStore::new(self.pool.clone())
    }

    /// Worktree-to-task reverse lookup store (used by the executor
    /// dispatch guard to enumerate active holders in refusal
    /// messages).
    pub fn worktrees(&self) -> WorktreeStore {
        WorktreeStore::new(self.pool.clone())
    }

    /// Plan↔PR link store accessor (P2-3 / F47).
    pub fn plan_pr_links(&self) -> PlanPrLinksStore {
        PlanPrLinksStore::new(self.pool.clone())
    }

    /// Ontology branch + overlay entry store accessor.
    pub fn ontology(&self) -> OntologyBranchStore {
        OntologyBranchStore::new(self.pool.clone())
    }

    /// Audit log accessor.
    pub fn audit(&self) -> AuditLog {
        AuditLog::new(self.pool.clone())
    }

    /// Create a plan, assign the next project-group number atomically, and write the audit row.
    ///
    /// Uses `BEGIN IMMEDIATE` so concurrent writers do not race the
    /// `MAX(number)+1` → `INSERT` pair onto the same `(project, number)`.
    pub async fn create_plan(&self, input: NewPlan) -> Result<Plan> {
        let now = Utc::now();
        let mut tx = self.pool.inner().begin().await?;
        sqlx::query("ROLLBACK").execute(&mut *tx).await.ok();
        sqlx::query("BEGIN IMMEDIATE").execute(&mut *tx).await?;

        let number = PlanStore::next_number_in_tx(&mut tx, input.project.as_deref()).await?;

        let plan = Plan {
            id: Uuid::new_v4().to_string(),
            number,
            title: input.title,
            description: input.description,
            project: input.project,
            status: PlanStatus::Draft,
            created_at: now,
            updated_at: now,
            started_at: None,
            ended_at: None,
            duration_ms: None,
        };

        sqlx::query(
            "INSERT INTO plans \
             (id, number, title, description, project, status, created_at, updated_at) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(&plan.id)
        .bind(plan.number)
        .bind(&plan.title)
        .bind(&plan.description)
        .bind(&plan.project)
        .bind(plan.status.as_str())
        .bind(plan.created_at.to_rfc3339())
        .bind(plan.updated_at.to_rfc3339())
        .execute(&mut *tx)
        .await?;
        append_tx(
            &mut tx,
            EntityKind::Plan,
            &plan.id,
            "plan.created",
            &json!({
                "plan_id": plan.id,
                "number": plan.number,
                "title": plan.title,
                "project": plan.project,
            }),
            None,
        )
        .await?;
        tx.commit().await?;
        Ok(plan)
    }

    /// Atomically claim a `pending` task by promoting it to
    /// `in_progress` and recording `agent_id`. Returns `Ok(Some(task))`
    /// when this caller is the one who won the claim and
    /// `Ok(None)` when the row was already in another state (a
    /// concurrent caller won, or the task never was `pending`).
    ///
    /// The UPDATE is conditional on `status = 'pending'` and the
    /// audit row goes into the same transaction, so two ticks
    /// claiming the same task cannot both pass — the second one
    /// gets `rows_affected = 0` and returns `None`. This closes the
    /// 2026-05-11 audit's HIGH-severity duplicate-dispatch race in
    /// `convergio-executor` (`executor.rs:89/108`).
    pub async fn try_claim_pending(&self, task_id: &str, agent_id: &str) -> Result<Option<Task>> {
        let now = Utc::now().to_rfc3339();
        let mut tx = self.pool.inner().begin().await?;
        let rows_affected = sqlx::query(
            "UPDATE tasks SET status = 'in_progress', agent_id = ?, \
             started_at = COALESCE(started_at, ?), updated_at = ? \
             WHERE id = ? AND status = 'pending'",
        )
        .bind(agent_id)
        .bind(&now)
        .bind(&now)
        .bind(task_id)
        .execute(&mut *tx)
        .await?
        .rows_affected();
        if rows_affected == 0 {
            tx.rollback().await.ok();
            return Ok(None);
        }
        append_tx(
            &mut tx,
            EntityKind::Task,
            task_id,
            "task.in_progress",
            &json!({
                "task_id": task_id,
                "from": "pending",
                "to": "in_progress",
                "agent_id": agent_id,
                "claim": "atomic",
            }),
            Some(agent_id),
        )
        .await?;
        tx.commit().await?;
        let task = self.tasks().get(task_id).await?;
        Ok(Some(task))
    }

    /// Create a task and write the audit row.
    pub async fn create_task(&self, plan_id: &str, input: NewTask) -> Result<Task> {
        // Make sure the plan exists (yields NotFound if not).
        self.plans().get(plan_id).await?;
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

        let mut tx = self.pool.inner().begin().await?;
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
        .execute(&mut *tx)
        .await?;
        append_tx(
            &mut tx,
            EntityKind::Task,
            &task.id,
            "task.created",
            &json!({
                "task_id": task.id,
                "plan_id": task.plan_id,
                "wave": task.wave,
                "sequence": task.sequence,
                "title": task.title,
            }),
            None,
        )
        .await?;
        tx.commit().await?;
        Ok(task)
    }
}
