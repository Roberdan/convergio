//! `/v1/gdpr/*` data-subject-right request routes.

use crate::app::AppState;
use crate::error::ApiError;
use axum::extract::State;
use axum::routing::post;
use axum::{Json, Router};
use convergio_durability::audit::EntityKind;
use convergio_gdpr::{
    handle_request_with_records, DataSubjectRecord, DataSubjectRequest, DataSubjectResponse,
    GdprError,
};
use serde::Deserialize;
use serde_json::json;

/// Request body for `POST /v1/gdpr/requests`.
#[derive(Debug, Deserialize)]
pub struct SubmitGdprRequest {
    /// GDPR request contract.
    #[serde(flatten)]
    pub request: DataSubjectRequest,
    /// Subject-scoped records to process. Operators may pass an empty
    /// list to record a request before records are collected.
    #[serde(default)]
    pub records: Vec<DataSubjectRecord>,
    /// Agent or operator anchoring the audit event.
    #[serde(default)]
    pub agent_id: Option<String>,
}

/// Mount GDPR routes.
pub fn router() -> Router<AppState> {
    Router::new().route("/v1/gdpr/requests", post(submit_request))
}

async fn submit_request(
    State(state): State<AppState>,
    Json(body): Json<SubmitGdprRequest>,
) -> Result<Json<DataSubjectResponse>, ApiError> {
    let mut response =
        handle_request_with_records(&body.request, &body.records).map_err(map_gdpr)?;
    let payload = json!({
        "subject": response.request.subject.0,
        "right": response.request.right,
        "received_at": response.request.received_at,
        "responded_at": response.responded_at,
        "record_count": body.records.len(),
        "status": "fulfilled",
    });
    let (entry, _) = state
        .durability
        .audit()
        .append_with_provenance(
            EntityKind::Free,
            &format!("gdpr:{}", response.request.subject.0),
            "gdpr.request.fulfilled",
            &payload,
            body.agent_id.as_deref(),
        )
        .await?;
    response.audit_seq = Some(entry.seq as u64);
    Ok(Json(response))
}

fn map_gdpr(error: GdprError) -> ApiError {
    match error {
        GdprError::EmptySubject => ApiError::Validation {
            code: "gdpr_subject_empty",
            message: error.to_string(),
        },
        GdprError::UnsupportedRight => ApiError::Validation {
            code: "gdpr_right_unsupported",
            message: error.to_string(),
        },
        GdprError::Serde(_) => ApiError::Internal(error.to_string()),
    }
}
