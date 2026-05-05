//! `GET /v1/telemetry/series` — rolling 7-day time-series query.
//!
//! Returns 1-minute resolution data-points for a single metric over a
//! configurable window (default 24 h, max 7 d).

use crate::app::AppState;
use crate::error::ApiError;
use axum::extract::{Query, State};
use axum::routing::get;
use axum::{Json, Router};
use convergio_durability::TelemetryPoint;
use serde::{Deserialize, Serialize};

/// Mount `/v1/telemetry/series`.
pub fn router() -> Router<AppState> {
    Router::new().route("/v1/telemetry/series", get(series))
}

#[derive(Deserialize)]
struct SeriesQuery {
    /// Metric name, e.g. `agents_active_24h`.  Required.
    metric: String,
    /// Look-back window in days (1–7, default 1).
    #[serde(default = "default_window")]
    window_days: u32,
}

fn default_window() -> u32 {
    1
}

#[derive(Serialize)]
struct SeriesResponse {
    metric: String,
    window_days: u32,
    points: Vec<TelemetryPoint>,
}

async fn series(
    State(state): State<AppState>,
    Query(q): Query<SeriesQuery>,
) -> Result<Json<SeriesResponse>, ApiError> {
    if q.window_days == 0 || q.window_days > 7 {
        return Err(ApiError::BadRequest {
            code: "invalid_window",
            message: "window_days must be between 1 and 7".into(),
        });
    }
    let points = state
        .durability
        .query_telemetry_series(&q.metric, q.window_days)
        .await?;
    Ok(Json(SeriesResponse {
        metric: q.metric,
        window_days: q.window_days,
        points,
    }))
}
