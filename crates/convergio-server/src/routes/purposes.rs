//! `/v1/purposes` — Purpose Registry HTTP surface (ADR-0054 §B).
//!
//! - `POST /v1/purposes` — register a new immutable purpose.
//! - `GET  /v1/purposes` — list every registered purpose.
//!
//! This is the *registry* of declared "why" (purpose limitation), distinct
//! from the request-level `x-purpose-id` binding middleware in
//! `crate::purpose`. Registered purposes are immutable (the table refuses
//! UPDATE/DELETE); re-declaring a label is a `400 purpose_already_exists`.

use crate::AppState;
use axum::extract::State;
use axum::routing::post;
use axum::{Json, Router};
use convergio_ontology::{Error as OntologyError, PurposeRecord};
use convergio_server_core::ApiError;
use serde::{Deserialize, Serialize};

/// Mount the purpose-registry routes.
pub fn router() -> Router<AppState> {
    Router::new().route("/v1/purposes", post(register).get(list))
}

/// Request body for `POST /v1/purposes`.
#[derive(Deserialize)]
struct RegisterBody {
    label: String,
    #[serde(default)]
    description: String,
    #[serde(default)]
    declared_by_plan: Option<String>,
}

/// JSON view of a registered purpose.
#[derive(Serialize)]
struct PurposeView {
    id: String,
    label: String,
    description: String,
    declared_by_plan: Option<String>,
    effective_from: String,
}

impl From<PurposeRecord> for PurposeView {
    fn from(p: PurposeRecord) -> Self {
        Self {
            id: p.id,
            label: p.label,
            description: p.description,
            declared_by_plan: p.declared_by_plan,
            effective_from: p.effective_from,
        }
    }
}

async fn register(
    State(state): State<AppState>,
    Json(body): Json<RegisterBody>,
) -> Result<Json<PurposeView>, ApiError> {
    match state
        .ontology
        .purposes()
        .register(
            &body.label,
            &body.description,
            body.declared_by_plan.as_deref(),
        )
        .await
    {
        Ok(p) => Ok(Json(p.into())),
        Err(OntologyError::PurposeAlreadyExists { label }) => Err(ApiError::BadRequest {
            code: "purpose_already_exists",
            message: format!("purpose `{label}` already declared (immutable)"),
        }),
        Err(OntologyError::InvalidEntry { reason }) => Err(ApiError::BadRequest {
            code: "invalid_purpose",
            message: reason,
        }),
        Err(e) => Err(e.into()),
    }
}

async fn list(State(state): State<AppState>) -> Result<Json<Vec<PurposeView>>, ApiError> {
    let rows = state.ontology.purposes().list().await?;
    Ok(Json(rows.into_iter().map(PurposeView::from).collect()))
}
