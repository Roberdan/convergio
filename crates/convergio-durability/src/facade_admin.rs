//! Operator-driven administrative transitions on [`Durability`].
//!
//! Two methods live here, both backed by ADR-0026:
//!
//! - [`Durability::close_task_post_hoc`] — drains a task whose work
//!   already shipped outside the daemon's evidence flow (e.g. merged
//!   in `main` before the convention existed). Second exception to
//!   ADR-0011 alongside Thor's `complete_validated_tasks`. Mandatory
//!   `reason` is recorded in the audit row.
//! - [`Durability::rename_plan`] — updates a plan's title in place.
//!   The title is human-facing and changes for legitimate reasons
//!   (`Wave 0` → `W0` per ADR-0026). One audit row of kind
//!   `plan.renamed`.

use crate::audit::{append_tx, EntityKind};
use crate::error::{DurabilityError, Result};
use crate::facade::Durability;
use crate::model::{Plan, Task, TaskStatus};
use chrono::Utc;
use serde_json::json;

impl Durability {
    /// Close a task `post hoc`: move it directly to `done` because
    /// the operator confirms the work shipped outside the daemon's
    /// evidence flow. Writes one audit row of kind
    /// `task.closed_post_hoc` whose payload includes the reason and
    /// the previous status.
    ///
    /// This is the second escape valve from ADR-0011 (Thor-only-done)
    /// and is intentionally narrow: only an operator triage pass uses
    /// it, mediated through `cvg task close-post-hoc`.
    ///
    /// Errors:
    /// - [`DurabilityError::PostHocReasonMissing`] if `reason` is
    ///   empty or whitespace.
    /// - [`DurabilityError::AlreadyDone`] if the task is already
    ///   `done` (idempotency guard).
    pub async fn close_task_post_hoc(
        &self,
        task_id: &str,
        reason: &str,
        agent_id: Option<&str>,
    ) -> Result<Task> {
        let task = self.tasks().get(task_id).await?;
        if matches!(task.status, TaskStatus::Done) {
            return Err(DurabilityError::AlreadyDone {
                id: task_id.to_string(),
            });
        }

        let trimmed = reason.trim();
        if trimmed.is_empty() {
            return Err(DurabilityError::PostHocReasonMissing);
        }
        crate::post_hoc_reason::validate_post_hoc_reason(trimmed)?;
        let mut tx = self.pool().inner().begin().await?;
        let now_dt = Utc::now();
        let now = now_dt.to_rfc3339();
        // ADR-0031: post-hoc close is the second public path to `done`,
        // it must keep the timing cache consistent with the audit row.
        let duration_ms = task.started_at.map(|s| (now_dt - s).num_milliseconds());
        sqlx::query(
            "UPDATE tasks SET status = ?, updated_at = ?, ended_at = ?, duration_ms = ? \
             WHERE id = ?",
        )
        .bind(TaskStatus::Done.as_str())
        .bind(&now)
        .bind(&now)
        .bind(duration_ms)
        .bind(task_id)
        .execute(&mut *tx)
        .await?;
        append_tx(
            &mut tx,
            EntityKind::Task,
            task_id,
            "task.closed_post_hoc",
            &json!({
                "task_id": task_id,
                "from": task.status.as_str(),
                "to": "done",
                "reason": trimmed,
                "agent_id": agent_id,
            }),
            agent_id,
        )
        .await?;
        tx.commit().await?;
        self.tasks().get(task_id).await
    }

    /// Rename a plan in place. Writes one audit row of kind
    /// `plan.renamed` with the previous and new title.
    ///
    /// Errors with [`DurabilityError::PlanTitleEmpty`] if the new
    /// title is empty or whitespace.
    pub async fn rename_plan(
        &self,
        plan_id: &str,
        new_title: &str,
        agent_id: Option<&str>,
    ) -> Result<Plan> {
        let trimmed = new_title.trim();
        if trimmed.is_empty() {
            return Err(DurabilityError::PlanTitleEmpty);
        }
        let plan = self.plans().get(plan_id).await?;
        let mut tx = self.pool().inner().begin().await?;
        let now = Utc::now().to_rfc3339();
        sqlx::query("UPDATE plans SET title = ?, updated_at = ? WHERE id = ?")
            .bind(trimmed)
            .bind(&now)
            .bind(plan_id)
            .execute(&mut *tx)
            .await?;
        append_tx(
            &mut tx,
            EntityKind::Plan,
            plan_id,
            "plan.renamed",
            &json!({
                "plan_id": plan_id,
                "from": plan.title,
                "to": trimmed,
                "agent_id": agent_id,
            }),
            agent_id,
        )
        .await?;
        tx.commit().await?;
        self.plans().get(plan_id).await
    }
}
