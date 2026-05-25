//! `PlanCoherenceGate` — refuses `task.submitted` when the parent plan
//! has no objective set. Implements W4 (ADR-0055).
//!
//! Opt-in for backward compatibility: enforcement is gated by the env
//! var `CONVERGIO_REQUIRE_PLAN_OBJECTIVE=1`. Production deployments
//! should set this; a follow-up PR will flip the default once every
//! shipped plan has been backfilled with an objective.

use super::{Gate, GateContext, GatePrecondition};
use crate::error::{DurabilityError, Result};
use crate::model::TaskStatus;
use crate::store::PlanObjectiveStore;

const ENABLE_ENV: &str = "CONVERGIO_REQUIRE_PLAN_OBJECTIVE";

/// Refuses task submissions on plans that have no OKR objective.
///
/// The gate fires only on the `Submitted` transition: existing plans
/// without an objective stay editable up until the agent tries to
/// hand work off for validation.
#[derive(Default)]
pub struct PlanCoherenceGate {}

impl PlanCoherenceGate {
    /// New gate with defaults.
    pub fn new() -> Self {
        Self::default()
    }

    fn enforce_enabled() -> bool {
        matches!(
            std::env::var(ENABLE_ENV).ok().as_deref(),
            Some("1") | Some("true") | Some("TRUE") | Some("yes")
        )
    }
}

#[async_trait::async_trait]
impl Gate for PlanCoherenceGate {
    fn name(&self) -> &'static str {
        "plan_coherence"
    }

    async fn check(&self, ctx: &GateContext) -> Result<()> {
        if !Self::enforce_enabled() {
            return Ok(());
        }
        if !matches!(ctx.target_status, TaskStatus::Submitted) {
            return Ok(());
        }
        let store = PlanObjectiveStore::new(ctx.pool.clone());
        if store.exists(&ctx.task.plan_id).await? {
            return Ok(());
        }
        Err(DurabilityError::GateRefused {
            gate: "plan_coherence",
            reason: format!("plan_missing_objective: plan_id={}", ctx.task.plan_id),
        })
    }

    fn describe(&self) -> GatePrecondition {
        GatePrecondition {
            gate: "plan_coherence".into(),
            reads_evidence_kinds: vec![],
            enforces_task_evidence_required: false,
            active_target_status: vec!["submitted".into()],
            refusal_reasons: vec!["plan_missing_objective".into()],
        }
    }
}
