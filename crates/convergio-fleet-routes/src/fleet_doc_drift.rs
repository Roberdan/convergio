//! Doc-drift routes (ADR-0038, F3-6).
//!
//! - `GET  /v1/fleet/doc-drift`          — surface ADR/Doc nodes whose
//!   alignment with linked code has drifted beyond `threshold`.
//! - `POST /v1/fleet/doc-drift/snapshot` — recompute and persist the
//!   ADR↔code alignment snapshot for every doc node.

use axum::extract::{Query, State};
use axum::Json;
use convergio_fleet::{find_doc_drift, snapshot_doc_alignment, DEFAULT_DOC_DRIFT_THRESHOLD};
use convergio_server_core::ApiError;
use convergio_server_core::AppState;
use serde::Deserialize;

use crate::bitemporal::parse_bitemporal;
use serde_json::{json, Value};

#[derive(Debug, Deserialize)]
pub(super) struct DriftQuery {
    #[serde(default)]
    as_of: Option<String>,
    #[serde(default)]
    tx_as_of: Option<String>,
    #[serde(default = "default_threshold")]
    threshold: f32,
    #[serde(default)]
    repo: Option<String>,
}

fn default_threshold() -> f32 {
    DEFAULT_DOC_DRIFT_THRESHOLD
}

/// `GET /v1/fleet/doc-drift` — list ADR/Doc drift candidates.
pub(super) async fn drift(
    State(state): State<AppState>,
    Query(q): Query<DriftQuery>,
) -> Result<Json<Value>, ApiError> {
    let _ = parse_bitemporal(q.as_of.as_deref(), q.tx_as_of.as_deref())?;
    if !q.threshold.is_finite() || !(0.0..=2.0).contains(&q.threshold) {
        return Err(ApiError::BadRequest {
            code: "invalid_threshold",
            message: format!("threshold must be in [0,2], got {}", q.threshold),
        });
    }
    let model = state.embedder.model_id();
    let rows = find_doc_drift(
        &state.fleet,
        &state.embed,
        model,
        q.threshold,
        q.repo.as_deref(),
    )
    .await
    .map_err(|e| ApiError::Internal(format!("doc-drift query failed: {e}")))?;
    let total = rows.len();
    Ok(Json(json!({
        "candidates": rows,
        "total": total,
        "threshold": q.threshold,
        "model": model,
    })))
}

/// `POST /v1/fleet/doc-drift/snapshot` — refresh ADR↔code alignment snapshots.
pub(super) async fn snapshot(State(state): State<AppState>) -> Result<Json<Value>, ApiError> {
    let model = state.embedder.model_id();
    let report = snapshot_doc_alignment(&state.fleet, &state.embed, model)
        .await
        .map_err(|e| ApiError::Internal(format!("snapshot failed: {e}")))?;
    Ok(Json(json!({
        "ok": true,
        "model": model,
        "nodes_considered": report.nodes_considered,
        "nodes_snapshotted": report.nodes_snapshotted,
    })))
}
