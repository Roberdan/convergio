//! `PrLinkGate` — refuses `task.done` when no PR is linked to the task's plan.
//!
//! The full pre-merge contract (PR is merged on `origin` AND its
//! branch is deleted AND no orphan worktree remains) cannot run
//! inside a synchronous gate because gates do not have network access
//! and must not block transitions on remote latency. The operator-side
//! `~/.claude/hooks/pre-completion-gate.sh` + the lefthook
//! `post-merge` `fleet-cleanup` step handle the remote-state checks.
//!
//! What this gate *can* enforce in-process is the **strict prerequisite
//! that the agent ever recorded a PR link**: a `plan_pr_links` row
//! (introduced by F47, ADR pending) exists for the task's plan.
//! Without that row the daemon has no anchor to verify the PR ever
//! existed, and the audit chain ends with a `task.done` whose
//! provenance is "trust me bro".
//!
//! Refusal contract:
//!
//! - Only fires on transitions whose target is [`TaskStatus::Done`].
//! - Other transitions return `Ok(())` unconditionally.
//! - When refused, the gate emits a `pr_link_missing` refusal code so
//!   tooling can suggest `cvg pr link` / `cvg task transition --pr-url`.
//!
//! Driven by the 2026-05 insights audit ("Claude occasionally claims
//! 'done' before PRs are merged"). The exhaustive form is documented
//! in the OPTIMIZATIONS catalogue under section 7.

use super::{Gate, GateContext};
use crate::error::{DurabilityError, Result};
use crate::model::TaskStatus;
use convergio_api::GatePrecondition;

/// Refuses `task.done` when the task's plan has no `plan_pr_links` row.
pub struct PrLinkGate;

#[async_trait::async_trait]
impl Gate for PrLinkGate {
    fn name(&self) -> &'static str {
        "pr_link"
    }

    async fn check(&self, ctx: &GateContext) -> Result<()> {
        if ctx.target_status != TaskStatus::Done {
            return Ok(());
        }

        let (count,): (i64,) =
            sqlx::query_as("SELECT COUNT(*) FROM plan_pr_links WHERE plan_id = ?")
                .bind(&ctx.task.plan_id)
                .fetch_one(ctx.pool.inner())
                .await?;

        if count > 0 {
            return Ok(());
        }

        Err(DurabilityError::GateRefused {
            gate: "pr_link",
            reason: format!(
                "pr_link_missing: task plan {} has no plan_pr_links row; record the PR with \
                 `cvg pr link --plan {} --pr <num>` (or have the agent submit via \
                 `cvg task transition <id> done --pr-url ...`) before closing the task",
                ctx.task.plan_id, ctx.task.plan_id
            ),
        })
    }

    fn describe(&self) -> GatePrecondition {
        GatePrecondition {
            gate: self.name().to_string(),
            reads_evidence_kinds: vec![],
            enforces_task_evidence_required: false,
            active_target_status: vec!["done".to_string()],
            refusal_reasons: vec!["pr_link_missing".to_string()],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gates::GateContext;
    use crate::model::Task;
    use crate::{init, Durability, NewPlan};
    use convergio_db::Pool;
    use tempfile::TempDir;

    async fn fresh_pool() -> (Pool, TempDir) {
        let dir = TempDir::new().expect("tmp");
        let url = format!("sqlite://{}/state.db", dir.path().display());
        let pool: Pool = Pool::connect(&url).await.expect("pool");
        init(&pool).await.expect("migrate");
        (pool, dir)
    }

    async fn make_plan(dur: &Durability) -> String {
        dur.create_plan(NewPlan {
            title: "p".into(),
            description: None,
            project: None,
        })
        .await
        .expect("plan")
        .id
    }

    fn ctx(pool: Pool, task: Task, target: TaskStatus) -> GateContext {
        GateContext {
            pool,
            task,
            target_status: target,
            agent_id: None,
        }
    }

    fn task_for(plan_id: &str) -> Task {
        Task {
            id: "t-1".into(),
            plan_id: plan_id.to_string(),
            wave: 1,
            sequence: 1,
            title: "t".into(),
            description: None,
            status: TaskStatus::Submitted,
            agent_id: None,
            evidence_required: vec![],
            last_heartbeat_at: None,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            started_at: None,
            ended_at: None,
            duration_ms: None,
            runner_kind: None,
            profile: None,
            max_budget_usd: None,
        }
    }

    #[tokio::test]
    async fn allows_non_done_transitions() {
        let (pool, _dir) = fresh_pool().await;
        let dur = Durability::new(pool.clone());
        let plan_id = make_plan(&dur).await;
        let gate = PrLinkGate;
        let context = ctx(pool, task_for(&plan_id), TaskStatus::Submitted);
        assert!(gate.check(&context).await.is_ok());
    }

    #[tokio::test]
    async fn refuses_done_without_pr_link() {
        let (pool, _dir) = fresh_pool().await;
        let dur = Durability::new(pool.clone());
        let plan_id = make_plan(&dur).await;
        let gate = PrLinkGate;
        let context = ctx(pool, task_for(&plan_id), TaskStatus::Done);
        let err = gate.check(&context).await.expect_err("must refuse");
        assert!(format!("{err}").contains("pr_link_missing"));
    }
}
