//! `GET /v1/fleet/rot` — semantic dead-code candidates (ADR-0038, F3-5).
//!
//! Returns advisory rot candidates ranked by descending confidence.
//! Confidence weighting follows the owning repo's role.

use crate::app::AppState;
use crate::error::ApiError;
use axum::extract::{Query, State};
use axum::Json;
use convergio_fleet::{find_rot, DEFAULT_ROT_THRESHOLD};
use serde::Deserialize;
use serde_json::{json, Value};

/// Query parameters for `GET /v1/fleet/rot`.
#[derive(Debug, Deserialize)]
pub(super) struct RotQuery {
    /// Cosine ceiling — nodes below this value are surfaced (default 0.3).
    #[serde(default = "default_threshold")]
    threshold: f32,
    /// Restrict scan to a single repo.
    #[serde(default)]
    repo: Option<String>,
    /// Force-include this node ID, returning richer reasoning.
    #[serde(default)]
    explain: Option<String>,
}

fn default_threshold() -> f32 {
    DEFAULT_ROT_THRESHOLD
}

/// `GET /v1/fleet/rot` — list semantic dead-code candidates.
pub(super) async fn rot(
    State(state): State<AppState>,
    Query(q): Query<RotQuery>,
) -> Result<Json<Value>, ApiError> {
    if !q.threshold.is_finite() || !(0.0..=1.0).contains(&q.threshold) {
        return Err(ApiError::BadRequest {
            code: "invalid_threshold",
            message: format!("threshold must be in [0,1], got {}", q.threshold),
        });
    }
    let model = state.embedder.model_id();
    let candidates = find_rot(
        &state.fleet,
        &state.embed,
        model,
        q.threshold,
        q.repo.as_deref(),
        q.explain.as_deref(),
    )
    .await
    .map_err(|e| ApiError::Internal(format!("rot scan failed: {e}")))?;

    let total = candidates.len();
    Ok(Json(json!({
        "candidates": candidates,
        "total": total,
        "threshold": q.threshold,
        "model": model,
    })))
}
