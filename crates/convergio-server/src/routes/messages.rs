//! `/v1/plans/:plan_id/messages` and `/v1/messages/:id/ack` — Layer 2.
//!
//! Plus the human-facing read surfaces:
//! - `GET /v1/plans/:plan_id/messages/tail` — every message regardless
//!   of consumed status, optional `topic` filter.
//! - `GET /v1/plans/:plan_id/topics` — per-topic summaries.

use crate::app::AppState;
use crate::error::ApiError;
use crate::sse::{poll_stream, StreamEvent};
use axum::extract::{Path, Query, State};
use axum::response::IntoResponse;
use axum::routing::{get, post};
use axum::{Json, Router};
use convergio_bus::{Message, NewMessage, TopicSummary};
use serde::Deserialize;

const MAX_MESSAGE_LIMIT: i64 = 100;

/// Mount Layer 2 routes.
pub fn router() -> Router<AppState> {
    Router::new()
        .route("/v1/plans/:plan_id/messages", post(publish).get(poll))
        .route("/v1/plans/:plan_id/messages/tail", get(tail))
        .route("/v1/plans/:plan_id/messages/stream", get(stream))
        .route("/v1/plans/:plan_id/topics", get(topics))
        .route("/v1/messages/:id/ack", post(ack))
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
    /// Optional sender id to skip — useful for an agent that polls a
    /// topic it also publishes to and wants peer-only traffic
    /// (ADR-0024). System messages (`sender NULL`) are always
    /// returned.
    #[serde(default)]
    exclude_sender: Option<String>,
}

fn default_limit() -> i64 {
    50
}

fn validate_limit(limit: i64) -> Result<i64, ApiError> {
    if (1..=MAX_MESSAGE_LIMIT).contains(&limit) {
        Ok(limit)
    } else {
        Err(ApiError::BadRequest {
            code: "invalid_message_limit",
            message: format!("limit must be between 1 and {MAX_MESSAGE_LIMIT}"),
        })
    }
}

#[derive(Deserialize)]
struct AckBody {
    #[serde(default)]
    consumer: Option<String>,
}

async fn publish(
    State(state): State<AppState>,
    Path(plan_id): Path<String>,
    Json(body): Json<PublishBody>,
) -> Result<Json<Message>, ApiError> {
    let m = state
        .bus
        .publish(NewMessage {
            plan_id,
            topic: body.topic,
            sender: body.sender,
            payload: body.payload,
        })
        .await?;
    Ok(Json(m))
}

async fn poll(
    State(state): State<AppState>,
    Path(plan_id): Path<String>,
    Query(q): Query<PollQuery>,
) -> Result<Json<Vec<Message>>, ApiError> {
    let cursor = q.cursor.unwrap_or(0);
    let limit = validate_limit(q.limit)?;
    let messages = state
        .bus
        .poll_filtered(
            &plan_id,
            &q.topic,
            cursor,
            limit,
            q.exclude_sender.as_deref(),
        )
        .await?;
    Ok(Json(messages))
}

async fn ack(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(body): Json<AckBody>,
) -> Result<Json<serde_json::Value>, ApiError> {
    state.bus.ack(&id, body.consumer.as_deref()).await?;
    Ok(Json(serde_json::json!({"ok": true})))
}

#[derive(Deserialize)]
struct TailQuery {
    #[serde(default)]
    topic: Option<String>,
    #[serde(default)]
    cursor: Option<i64>,
    #[serde(default = "default_limit")]
    limit: i64,
}

async fn tail(
    State(state): State<AppState>,
    Path(plan_id): Path<String>,
    Query(q): Query<TailQuery>,
) -> Result<Json<Vec<Message>>, ApiError> {
    let cursor = q.cursor.unwrap_or(0);
    let limit = validate_limit(q.limit)?;
    let messages = state
        .bus
        .tail(&plan_id, q.topic.as_deref(), cursor, limit)
        .await?;
    Ok(Json(messages))
}

async fn topics(
    State(state): State<AppState>,
    Path(plan_id): Path<String>,
) -> Result<Json<Vec<TopicSummary>>, ApiError> {
    Ok(Json(state.bus.topics(&plan_id).await?))
}

#[derive(Deserialize)]
struct StreamQuery {
    /// Optional literal-match topic filter. Glob patterns
    /// (`coordination/*`, `agent:*`) are deferred to wave-2 — the
    /// current implementation only matches the exact string.
    #[serde(default)]
    topic: Option<String>,
    /// Cursor: only messages with `seq > since` are emitted.
    /// Defaults to the current per-plan tail so a fresh client only
    /// sees new messages from connect time forward.
    #[serde(default)]
    since: Option<i64>,
}

const MESSAGE_STREAM_BATCH_LIMIT: i64 = 100;

async fn stream(
    State(state): State<AppState>,
    Path(plan_id): Path<String>,
    Query(q): Query<StreamQuery>,
) -> Result<axum::response::Response, ApiError> {
    let initial_cursor = match q.since {
        Some(s) => s,
        None => {
            // Default: current per-plan tail. We snapshot the tail
            // by paging once with cursor=0 and a generous limit;
            // the next-cursor is the largest seq returned. For
            // empty plans this is 0.
            let snapshot = state
                .bus
                .tail(&plan_id, q.topic.as_deref(), 0, MESSAGE_STREAM_BATCH_LIMIT)
                .await?;
            // Walk forward in batches until we exhaust history,
            // so we don't deliver the backlog on connect.
            let mut last = snapshot.last().map(|m| m.seq).unwrap_or(0);
            let mut current = last;
            loop {
                let more = state
                    .bus
                    .tail(
                        &plan_id,
                        q.topic.as_deref(),
                        current,
                        MESSAGE_STREAM_BATCH_LIMIT,
                    )
                    .await?;
                if more.is_empty() {
                    break;
                }
                last = more.last().map(|m| m.seq).unwrap_or(last);
                current = last;
            }
            last
        }
    };
    let bus = state.bus.clone();
    let plan_id_owned = plan_id.clone();
    let topic_owned = q.topic.clone();

    let sse = poll_stream::<serde_json::Value, _, _>("bus", initial_cursor, move |cursor| {
        let bus = bus.clone();
        let plan_id = plan_id_owned.clone();
        let topic = topic_owned.clone();
        async move {
            let rows = bus
                .tail(
                    &plan_id,
                    topic.as_deref(),
                    cursor,
                    MESSAGE_STREAM_BATCH_LIMIT,
                )
                .await
                .map_err(|e| e.to_string())?;
            let mapped = rows
                .into_iter()
                .map(|m| StreamEvent {
                    seq: m.seq,
                    payload: serde_json::json!({
                        "seq": m.seq,
                        "plan_id": m.plan_id,
                        "topic": m.topic,
                        "sender": m.sender,
                        "payload": m.payload,
                        "created_at": m.created_at.to_rfc3339(),
                    }),
                })
                .collect::<Vec<_>>();
            Ok(mapped)
        }
    });
    Ok(sse.into_response())
}
