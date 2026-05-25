//! Gate pipeline.
//!
//! A gate is a precondition that must hold before a state transition is
//! persisted. Gates run server-side in a fixed order (see
//! [`crate::Durability::transition_task`]):
//!
//! ```text
//! plan_status → evidence → crdt_conflict → no_debt → no_stub
//! → wire_check → no_secrets → prompt_injection → a11y → zero_warnings
//! → wave_sequence → pr_link
//! ```
//!
//! Adding a gate:
//!
//! 1. Implement the [`Gate`] trait in a new file under `gates/`.
//! 2. Register it in [`default_pipeline`].
//! 3. Document the rationale in an ADR.

mod a11y_gate;
mod crdt_conflict_gate;
mod evidence_gate;
mod no_debt_gate;
mod no_secrets_gate;
mod no_stub_gate;
mod plan_coherence_gate;
mod plan_status_gate;
mod pr_link_gate;
mod prompt_injection_gate;
mod wave_sequence_gate;
mod wire_check_gate;
mod zero_warnings_gate;

pub use a11y_gate::A11yGate;
pub use crdt_conflict_gate::CrdtConflictGate;
pub use evidence_gate::EvidenceGate;
pub use no_debt_gate::{DebtRule, NoDebtGate};
pub use no_secrets_gate::{NoSecretsGate, SecretRule};
pub use no_stub_gate::{NoStubGate, StubRule};
pub use plan_coherence_gate::PlanCoherenceGate;
pub use plan_status_gate::PlanStatusGate;
pub use pr_link_gate::PrLinkGate;
pub use prompt_injection_gate::{InjectionRule, PromptInjectionGate};
pub use wave_sequence_gate::WaveSequenceGate;
pub use wire_check_gate::WireCheckGate;
pub use zero_warnings_gate::ZeroWarningsGate;

pub use convergio_api::GatePrecondition;

use crate::error::Result;
use crate::model::{Task, TaskStatus};
use convergio_db::Pool;
use std::sync::Arc;

/// Context handed to every gate.
#[derive(Clone)]
pub struct GateContext {
    /// DB pool.
    pub pool: Pool,
    /// Task before the proposed transition.
    pub task: Task,
    /// Status the caller wants to move to.
    pub target_status: TaskStatus,
    /// Agent claiming the transition (if any).
    pub agent_id: Option<String>,
}

/// One gate.
#[async_trait::async_trait]
pub trait Gate: Send + Sync {
    /// Stable name (used in error messages and ADRs).
    fn name(&self) -> &'static str;
    /// Returns `Ok(())` to allow, `Err(GateRefused { ... })` to block.
    async fn check(&self, ctx: &GateContext) -> Result<()>;
    /// Declarative precondition (P3-2). Default implementation
    /// returns just the gate name; gates with meaningful inputs
    /// override this.
    fn describe(&self) -> GatePrecondition {
        GatePrecondition {
            gate: self.name().to_string(),
            ..GatePrecondition::default()
        }
    }
}

/// Erased pipeline.
pub type Pipeline = Vec<Arc<dyn Gate>>;

/// Default pipeline. Order is meaningful — see module docs.
///
/// Order rationale:
/// 1. `PlanStatusGate` first (cheap, refuses if the plan is dead).
/// 2. `EvidenceGate` second (refuses if required kinds missing).
/// 3. `CrdtConflictGate` — unresolved metadata conflicts block completion.
/// 4. `NoDebtGate` (P1) — debt markers in payloads.
/// 5. `NoStubGate` (P4) — scaffolding markers in payloads.
/// 6. `WireCheckGate` (P4) — structural verification of claimed
///    routes / CLI paths against the workspace tree (after the
///    cheap regex, before the rest).
/// 7. `NoSecretsGate` (P2) — common credential leaks in payloads.
/// 8. `PromptInjectionGate` (P2) — LLM prompt-injection patterns in
///    evidence payload strings; runs right after secrets so payload
///    scans are co-located.
/// 9. `A11yGate` phase 1 (P3) — built-in accessibility checks on
///    markdown / CLI evidence payloads.
/// 10. `ZeroWarningsGate` (P1) — build/lint/test signal must be clean.
/// 11. `WaveSequenceGate` (queries dependencies in the same plan).
/// 12. `PrLinkGate` last (only fires on `done`, cheap when it does
///     fire — single `COUNT(*)` against `plan_pr_links`).
pub fn default_pipeline() -> Pipeline {
    vec![
        Arc::new(PlanStatusGate),
        Arc::new(PlanCoherenceGate::new()),
        Arc::new(EvidenceGate),
        Arc::new(CrdtConflictGate),
        Arc::new(NoDebtGate::default()),
        Arc::new(NoStubGate::default()),
        Arc::new(WireCheckGate),
        Arc::new(NoSecretsGate::default()),
        Arc::new(PromptInjectionGate::default()),
        Arc::new(A11yGate::default()),
        Arc::new(ZeroWarningsGate),
        Arc::new(WaveSequenceGate),
        Arc::new(PrLinkGate),
    ]
}

/// Run every gate in `pipeline` against `ctx`, short-circuiting on the
/// first refusal.
pub async fn run(pipeline: &Pipeline, ctx: &GateContext) -> Result<()> {
    for gate in pipeline {
        gate.check(ctx).await?;
    }
    Ok(())
}

/// Collect the declarative precondition for every gate in a pipeline.
/// Used by `GET /v1/gates/preconditions` to expose the catalog
/// without forcing callers to deserialize the trait object set
/// (P3-2 — Palantir-inspired declarative gate preconditions).
pub fn describe_pipeline(pipeline: &Pipeline) -> Vec<GatePrecondition> {
    pipeline.iter().map(|g| g.describe()).collect()
}
