//! `GET /v1/embed/stats` — embedding store inventory probe.
//!
//! ADR-0035 § 5.7: minimal F1-α surface. Returns the count of stored
//! embeddings, optionally filtered by `?repo=<name>`. The richer
//! `/v1/embed/build`, `/v1/embed/for-task`, and similar routes land
//! in F1-β alongside the real model.

use crate::app::AppState;
use crate::error::ApiError;
use axum::extract::{Query, State};
use axum::routing::get;
use axum::{Json, Router};
use serde::Deserialize;
use serde_json::{json, Value};

/// Query parameters for `GET /v1/embed/stats`.
#[derive(Debug, Deserialize)]
struct StatsQuery {
    /// Optional repo filter; when omitted the count covers every
    /// repo currently registered in the store.
    repo: Option<String>,
}

/// Mount the embed routes onto the daemon router.
pub fn router() -> Router<AppState> {
    Router::new().route("/v1/embed/stats", get(stats))
}

async fn stats(
    State(state): State<AppState>,
    Query(q): Query<StatsQuery>,
) -> Result<Json<Value>, ApiError> {
    let count = state
        .embed
        .count(q.repo.as_deref())
        .await
        .map_err(|e| ApiError::Internal(format!("embed count failed: {e}")))?;
    Ok(Json(json!({
        "ok": true,
        "count": count,
        "repo": q.repo,
    })))
}
