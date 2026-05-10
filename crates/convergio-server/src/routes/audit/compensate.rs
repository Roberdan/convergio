//! `GET /v1/audit/events/:seq/compensate` — apply a compensating action.

use crate::app::AppState;
use crate::error::ApiError;
use axum::extract::{Path, Query, State};
use axum::Json;
use convergio_durability::audit::Action as AuditAction;
use convergio_durability::DurabilityError;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
pub(super) struct CompensateQuery {
    /// When true, apply the computed compensating action.
    /// Defaults to false (dry-run).
    #[serde(default)]
    pub apply: bool,
}

#[derive(Debug, Serialize)]
pub(super) struct CompensateResponse {
    /// Audit sequence number that was compensated.
    pub source_seq: i64,
    /// Original audit transition kind.
    pub source_transition: String,
    /// Compensating action that would be (or was) applied.
    pub compensating_action: AuditAction,
    /// Whether the compensating action was applied.
    pub applied: bool,
}

pub(super) async fn compensate(
    State(state): State<AppState>,
    Path(seq): Path<i64>,
    Query(q): Query<CompensateQuery>,
) -> Result<Json<CompensateResponse>, ApiError> {
    let entry =
        state
            .durability
            .audit()
            .get(seq)
            .await?
            .ok_or_else(|| DurabilityError::NotFound {
                entity: "audit_event",
                id: seq.to_string(),
            })?;

    let source_transition = entry.transition.clone();
    let action = AuditAction::try_from_entry(&entry).map_err(|msg| ApiError::BadRequest {
        code: "audit_action_parse",
        message: msg,
    })?;

    let Some(comp) = action.compensate() else {
        let why = action
            .compensate_rationale()
            .unwrap_or("no compensating action exists for this audit transition");
        return Err(ApiError::Validation {
            code: "compensation_unavailable",
            message: why.to_string(),
        });
    };

    if q.apply {
        apply_compensation(&state, seq, &comp).await?;
    }

    Ok(Json(CompensateResponse {
        source_seq: seq,
        source_transition,
        compensating_action: comp,
        applied: q.apply,
    }))
}

async fn apply_compensation(
    state: &AppState,
    source_seq: i64,
    action: &AuditAction,
) -> Result<(), ApiError> {
    match action {
        AuditAction::AgentRetire { agent_id } => {
            state.durability.retire_agent(agent_id).await?;
        }
        AuditAction::AgentReregister { agent_id } => {
            state.durability.reregister_agent(agent_id).await?;
        }
        AuditAction::PlanRename {
            plan_id,
            to,
            agent_id,
            ..
        } => {
            state
                .durability
                .rename_plan(plan_id, to, agent_id.as_deref())
                .await?;
        }
        AuditAction::TaskTransition {
            task_id,
            to,
            agent_id,
            ..
        } => {
            state
                .durability
                .transition_task(task_id, *to, agent_id.as_deref())
                .await?;
        }
        AuditAction::TaskReopen {
            task_id,
            to,
            reason,
            source_seq: reopen_source,
            agent_id,
        } => {
            let reason = reason.as_deref().unwrap_or("compensating audit event");
            let source_seq = reopen_source.or(Some(source_seq));
            state
                .durability
                .reopen_task_from_done(task_id, *to, reason, source_seq, agent_id.as_deref())
                .await?;
        }
        AuditAction::EvidenceRemove { evidence_id } => {
            state.durability.remove_evidence(evidence_id).await?;
        }
        _ => {
            return Err(ApiError::Validation {
                code: "compensation_unsupported",
                message: "compensating action is not directly applicable".to_string(),
            });
        }
    }
    Ok(())
}
