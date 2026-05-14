//! `/v1/system-messages` — Layer 2 system-scoped bus topics (ADR-0025).
//!
//! `system.*` topics live outside any single plan; messages on them
//! carry `plan_id IS NULL` and represent presence and coordination
//! signals (`agent.attached`, `agent.heartbeat`, `agent.idle`,
//! `agent.detached`, …).
//!
//! Only the `system.*` topic family is accepted here. The bus
//! enforces the prefix in [`convergio_bus::Bus::publish_system`] and
//! [`convergio_bus::Bus::poll_system`]; if a non-system topic slips
//! through HTTP we surface the bus error as a 400.

use crate::app::AppState;
use crate::error::ApiError;
use crate::routes::limits::validate_message_limit;
use axum::extract::{Query, State};
use axum::routing::get;
use axum::{Json, Router};
use convergio_bus::{Message, NewSystemMessage};
use serde::Deserialize;

/// Mount the system-message routes.
pub fn router() -> Router<AppState> {
    Router::new().route("/v1/system-messages", get(poll).post(publish))
}

#[derive(Deserialize)]
struct PublishBody {
    topic: String,
    #[serde(default)]
    sender: Option<String>,
    payload: serde_json::Value,
}

#[derive(Deserialize)]
struct PollQuery {
    topic: String,
    #[serde(default)]
    cursor: Option<i64>,
    #[serde(default = "default_limit")]
    limit: i64,
}

fn default_limit() -> i64 {
    50
}

fn validate_limit(limit: i64) -> Result<i64, ApiError> {
    validate_message_limit(limit, "invalid_message_limit", "limit")
}

async fn publish(
    State(state): State<AppState>,
    Json(body): Json<PublishBody>,
) -> Result<Json<Message>, ApiError> {
    let m = state
        .bus
        .publish_system(NewSystemMessage {
            topic: body.topic,
            sender: body.sender,
            payload: body.payload,
        })
        .await?;
    Ok(Json(m))
}

async fn poll(
    State(state): State<AppState>,
    Query(q): Query<PollQuery>,
) -> Result<Json<Vec<Message>>, ApiError> {
    let cursor = q.cursor.unwrap_or(0);
    let limit = validate_limit(q.limit)?;
    let messages = state.bus.poll_system(&q.topic, cursor, limit).await?;
    Ok(Json(messages))
}
