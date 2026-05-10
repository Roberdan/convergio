//! `/v1/audit/verify` — recompute the chain.
//! `/v1/audit/append` — agent-emitted custom audit rows (P2-2, ADR-0002 § Custom kinds).
//! `/v1/audit/stream` — Server-Sent Events tail (P1.1).

mod append;
mod compensate;

use crate::app::AppState;
use crate::error::ApiError;
use crate::sse::{poll_stream, StreamEvent};
use axum::extract::{Query, State};
use axum::response::IntoResponse;
use axum::routing::{get, post};
use axum::{Json, Router};
use convergio_durability::audit::{AuditEntry, VerifyReport};
use serde::Deserialize;
use std::collections::HashSet;
use std::sync::Arc;

/// Mount audit routes.
pub fn router() -> Router<AppState> {
    Router::new()
        .route("/v1/audit/verify", get(verify))
        .route("/v1/audit/refusals/latest", get(latest_refusal))
        .route("/v1/audit/events", get(events))
        .route(
            "/v1/audit/events/:seq/compensate",
            get(compensate::compensate),
        )
        .route("/v1/audit/stream", get(stream))
        .route("/v1/audit/append", post(append::append))
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
    // Fast path: memoised full-chain verify keyed by tail seq.
    if q.from.is_none() && q.to.is_none() {
        let tail_seq = state
            .durability
            .audit()
            .tail()
            .await?
            .map(|e| e.seq)
            .unwrap_or(0);
        {
            let guard = state
                .audit_verify_cache
                .lock()
                .expect("audit_verify_cache poisoned");
            if let Some((cached_seq, ref report)) = *guard {
                if cached_seq == tail_seq {
                    return Ok(Json(report.clone()));
                }
            }
        }
        let report = state.durability.audit().verify(None, None).await?;
        *state
            .audit_verify_cache
            .lock()
            .expect("audit_verify_cache poisoned") = Some((tail_seq, report.clone()));
        return Ok(Json(report));
    }
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

#[derive(Deserialize)]
struct StreamQuery {
    /// Cursor: only events with `seq > since` are emitted. Defaults
    /// to the current chain tip so a fresh client only sees new
    /// events from connect time forward, NOT the full backlog.
    #[serde(default)]
    since: Option<i64>,
    /// Optional comma-separated list of audit `transition` kinds
    /// (e.g. `task.in_progress,plan.created`). When set, the server
    /// drops rows whose `transition` is not in the list.
    #[serde(default)]
    kinds: Option<String>,
}

const STREAM_BATCH_LIMIT: i64 = 100;

async fn stream(
    State(state): State<AppState>,
    Query(q): Query<StreamQuery>,
) -> Result<axum::response::Response, ApiError> {
    let initial_cursor = match q.since {
        Some(s) => s,
        None => {
            // Default: current tip — only new events from now.
            state
                .durability
                .audit()
                .tail()
                .await?
                .map(|e| e.seq)
                .unwrap_or(0)
        }
    };
    let kinds: Option<HashSet<String>> = q.kinds.map(|s| {
        s.split(',')
            .map(|p| p.trim().to_string())
            .filter(|p| !p.is_empty())
            .collect()
    });
    let kinds = Arc::new(kinds);
    let durability = state.durability.clone();

    let sse = poll_stream::<serde_json::Value, _, _>("audit", initial_cursor, move |cursor| {
        let durability = durability.clone();
        let kinds = kinds.clone();
        async move {
            let rows = durability
                .audit()
                .list_since(cursor, STREAM_BATCH_LIMIT)
                .await
                .map_err(|e| e.to_string())?;
            let filtered = rows
                .into_iter()
                .filter(|r| match kinds.as_ref() {
                    Some(set) => set.contains(&r.transition),
                    None => true,
                })
                .map(|r| StreamEvent {
                    seq: r.seq,
                    payload: serde_json::json!({
                        "seq": r.seq,
                        "kind": r.transition,
                        "entity_kind": r.entity_type,
                        "entity_id": r.entity_id,
                        "agent_id": r.agent_id,
                        "payload": serde_json::from_str::<serde_json::Value>(&r.payload)
                            .unwrap_or(serde_json::Value::String(r.payload.clone())),
                        "created_at": r.created_at,
                    }),
                })
                .collect::<Vec<_>>();
            Ok(filtered)
        }
    });
    Ok(sse.into_response())
}
