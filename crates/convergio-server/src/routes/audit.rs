//! `/v1/audit/verify` — recompute the chain.

use crate::app::AppState;
use crate::error::ApiError;
use axum::extract::{Query, State};
use axum::routing::get;
use axum::{Json, Router};
use convergio_durability::audit::{AuditEntry, VerifyReport};
use serde::Deserialize;

/// Mount audit routes.
pub fn router() -> Router<AppState> {
    Router::new()
        .route("/v1/audit/verify", get(verify))
        .route("/v1/audit/refusals/latest", get(latest_refusal))
        .route("/v1/audit/events", get(events))
}

#[derive(Deserialize)]
struct VerifyQuery {
    #[serde(default)]
    from: Option<i64>,
    #[serde(default)]
    to: Option<i64>,
}

#[derive(Deserialize)]
struct RefusalQuery {
    #[serde(default)]
    task_id: Option<String>,
}

#[derive(Deserialize)]
struct EventsQuery {
    /// Cursor: only return entries with `seq > after_seq`. Defaults
    /// to `0` so the first call returns the start of the log.
    #[serde(default)]
    after_seq: i64,
    /// Page size, clamped server-side to `[1, 1000]`. Defaults to
    /// 100 — comfortable for live tail UIs.
    #[serde(default = "default_events_limit")]
    limit: i64,
}

fn default_events_limit() -> i64 {
    100
}

async fn verify(
    State(state): State<AppState>,
    Query(q): Query<VerifyQuery>,
) -> Result<Json<VerifyReport>, ApiError> {
    let report = state.durability.audit().verify(q.from, q.to).await?;
    Ok(Json(report))
}

async fn latest_refusal(
    State(state): State<AppState>,
    Query(q): Query<RefusalQuery>,
) -> Result<Json<Option<AuditEntry>>, ApiError> {
    let entry = state
        .durability
        .audit()
        .latest_refusal(q.task_id.as_deref())
        .await?;
    Ok(Json(entry))
}

async fn events(
    State(state): State<AppState>,
    Query(q): Query<EventsQuery>,
) -> Result<Json<Vec<AuditEntry>>, ApiError> {
    let entries = state
        .durability
        .audit()
        .list_since(q.after_seq, q.limit)
        .await?;
    Ok(Json(entries))
}
