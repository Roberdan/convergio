//! HTTP error type — maps domain errors to status codes.

use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use convergio_bus::BusError;
use convergio_durability::DurabilityError;
use convergio_lifecycle::LifecycleError;
use serde_json::json;

/// API-facing error.
pub enum ApiError {
    /// Client supplied invalid request parameters.
    BadRequest {
        /// Stable error code.
        code: &'static str,
        /// Human-readable message.
        message: String,
    },
    /// Layer 1 error.
    Durability(DurabilityError),
    /// Layer 2 error.
    Bus(BusError),
    /// Layer 3 error.
    Lifecycle(LifecycleError),
    /// Tier-3 graph layer error (ADR-0014).
    Graph(convergio_graph::GraphError),
    /// Internal server error (catch-all with stable code).
    Internal(String),
}

impl From<convergio_graph::GraphError> for ApiError {
    fn from(e: convergio_graph::GraphError) -> Self {
        Self::Graph(e)
    }
}

impl From<DurabilityError> for ApiError {
    fn from(e: DurabilityError) -> Self {
        Self::Durability(e)
    }
}

impl From<BusError> for ApiError {
    fn from(e: BusError) -> Self {
        Self::Bus(e)
    }
}

impl From<LifecycleError> for ApiError {
    fn from(e: LifecycleError) -> Self {
        Self::Lifecycle(e)
    }
}

impl From<convergio_planner::PlannerError> for ApiError {
    fn from(e: convergio_planner::PlannerError) -> Self {
        match e {
            convergio_planner::PlannerError::Durability(d) => Self::Durability(d),
            convergio_planner::PlannerError::EmptyMission => {
                Self::Durability(DurabilityError::NotFound {
                    entity: "mission",
                    id: "empty".into(),
                })
            }
            convergio_planner::PlannerError::OpusSpawn(msg) => Self::BadRequest {
                code: "planner_opus_spawn",
                message: msg,
            },
            convergio_planner::PlannerError::OpusExited { status, stderr } => Self::BadRequest {
                code: "planner_opus_exited",
                message: format!("opus exited with status {status}: {stderr}"),
            },
            convergio_planner::PlannerError::OpusOutputInvalid(msg) => Self::BadRequest {
                code: "planner_opus_output_invalid",
                message: msg,
            },
        }
    }
}

impl From<convergio_thor::ThorError> for ApiError {
    fn from(e: convergio_thor::ThorError) -> Self {
        match e {
            convergio_thor::ThorError::Durability(d) => Self::Durability(d),
            convergio_thor::ThorError::PlanNotFound(id) => {
                Self::Durability(DurabilityError::NotFound { entity: "plan", id })
            }
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, code, message) = match &self {
            ApiError::BadRequest { code, message } => {
                (StatusCode::BAD_REQUEST, *code, message.clone())
            }
            ApiError::Durability(e) => match e {
                DurabilityError::NotFound { .. } => {
                    (StatusCode::NOT_FOUND, "not_found", e.to_string())
                }
                DurabilityError::GateRefused { gate, reason } => (
                    StatusCode::CONFLICT,
                    "gate_refused",
                    format!("{gate}: {reason}"),
                ),
                DurabilityError::DoneNotByThor => {
                    (StatusCode::FORBIDDEN, "done_not_by_thor", e.to_string())
                }
                DurabilityError::NotSubmitted { .. } => {
                    (StatusCode::CONFLICT, "not_submitted", e.to_string())
                }
                DurabilityError::NotFailed { .. } => {
                    (StatusCode::CONFLICT, "not_failed", e.to_string())
                }
                DurabilityError::PostHocReasonMissing => (
                    StatusCode::UNPROCESSABLE_ENTITY,
                    "post_hoc_reason_missing",
                    e.to_string(),
                ),
                DurabilityError::AlreadyDone { .. } => {
                    (StatusCode::CONFLICT, "already_done", e.to_string())
                }
                DurabilityError::PlanTitleEmpty => (
                    StatusCode::UNPROCESSABLE_ENTITY,
                    "plan_title_empty",
                    e.to_string(),
                ),
                DurabilityError::IllegalPlanTransition { .. } => (
                    StatusCode::CONFLICT,
                    "illegal_plan_transition",
                    e.to_string(),
                ),
                DurabilityError::WorkspaceLeaseConflict { .. } => (
                    StatusCode::CONFLICT,
                    "workspace_lease_conflict",
                    e.to_string(),
                ),
                DurabilityError::InvalidWorkspaceLease { .. } => (
                    StatusCode::UNPROCESSABLE_ENTITY,
                    "invalid_workspace_lease",
                    e.to_string(),
                ),
                DurabilityError::InvalidAgent { .. } => (
                    StatusCode::UNPROCESSABLE_ENTITY,
                    "invalid_agent",
                    e.to_string(),
                ),
                DurabilityError::InvalidCapability { .. } => (
                    StatusCode::UNPROCESSABLE_ENTITY,
                    "invalid_capability",
                    e.to_string(),
                ),
                DurabilityError::WorkspacePatchRefused { .. } => (
                    StatusCode::CONFLICT,
                    "workspace_patch_refused",
                    e.to_string(),
                ),
                DurabilityError::WorkspaceMergeRefused { .. } => (
                    StatusCode::CONFLICT,
                    "workspace_merge_refused",
                    e.to_string(),
                ),
                DurabilityError::AuditChainBroken { .. } => (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "audit_broken",
                    e.to_string(),
                ),
                _ => (StatusCode::INTERNAL_SERVER_ERROR, "internal", e.to_string()),
            },
            ApiError::Bus(e) => match e {
                BusError::NotFound(_) => (StatusCode::NOT_FOUND, "not_found", e.to_string()),
                BusError::InvalidTimestamp { .. } => (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "invalid_timestamp",
                    e.to_string(),
                ),
                _ => (StatusCode::INTERNAL_SERVER_ERROR, "internal", e.to_string()),
            },
            ApiError::Lifecycle(e) => match e {
                LifecycleError::NotFound(_) => (StatusCode::NOT_FOUND, "not_found", e.to_string()),
                LifecycleError::SpawnFailed(_) => (
                    StatusCode::UNPROCESSABLE_ENTITY,
                    "spawn_failed",
                    e.to_string(),
                ),
                LifecycleError::SpawnTimedOut { .. } => (
                    StatusCode::UNPROCESSABLE_ENTITY,
                    "spawn_timed_out",
                    e.to_string(),
                ),
                LifecycleError::InvalidTimestamp { .. } => (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "lifecycle_data_error",
                    e.to_string(),
                ),
                _ => (StatusCode::INTERNAL_SERVER_ERROR, "internal", e.to_string()),
            },
            ApiError::Graph(e) => (StatusCode::INTERNAL_SERVER_ERROR, "graph", e.to_string()),
            ApiError::Internal(msg) => (StatusCode::INTERNAL_SERVER_ERROR, "internal", msg.clone()),
        };

        let body = json!({
            "error": {
                "code": code,
                "message": message,
            }
        });
        (status, Json(body)).into_response()
    }
}
