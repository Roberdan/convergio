//! `/v1/ontology/events*` — bitemporal as-of read surface (ADR-0053, W3).
//!
//! The `object_events` log is bitemporal (valid-time + transaction-time),
//! but the rest of the HTTP surface only ever exposes the current state.
//! These two endpoints finally let callers travel both axes:
//!
//! - `GET /v1/ontology/events/:object_id?as_of=&tx_as_of=` — the single
//!   as-of event for one object. `as_of` selects by **valid-time**,
//!   `tx_as_of` by **transaction-time**; with neither, the
//!   transaction-current row is returned. Returns 404 when no row matches.
//! - `GET /v1/ontology/events?as_of=&tx_as_of=` — the as-of snapshot
//!   across every object (list form of the same selection).
//!
//! Both timestamp query params are RFC3339; an unparseable value yields
//! [`ApiError::BadRequest`] with code `invalid_timestamp`. `as_of` wins
//! over `tx_as_of` when both are supplied.

use axum::extract::{Path, Query, State};
use axum::routing::get;
use axum::{Json, Router};
use chrono::{DateTime, Utc};
use convergio_ontology::{Error as OntologyError, ObjectEvent};
use convergio_server_core::{ApiError, AppState};
use serde::{Deserialize, Serialize};

/// Mount the bitemporal event read routes.
pub fn router() -> Router<AppState> {
    Router::new()
        .route("/v1/ontology/events", get(list_events))
        .route("/v1/ontology/events/:object_id", get(get_event))
}

/// Valid-time / transaction-time selectors shared by both endpoints.
#[derive(Deserialize)]
struct AsOfQuery {
    #[serde(default)]
    as_of: Option<String>,
    #[serde(default)]
    tx_as_of: Option<String>,
}

/// JSON projection of one [`ObjectEvent`] with RFC3339 timestamps.
#[derive(Serialize)]
struct EventRow {
    object_id: String,
    op: String,
    payload: serde_json::Value,
    valid_from: String,
    valid_to: Option<String>,
    tx_from: String,
    tx_to: Option<String>,
}

impl From<ObjectEvent> for EventRow {
    fn from(e: ObjectEvent) -> Self {
        EventRow {
            object_id: e.object_id,
            op: e.op,
            payload: e.payload,
            valid_from: e.valid_from.to_rfc3339(),
            valid_to: e.valid_to.map(|t| t.to_rfc3339()),
            tx_from: e.tx_from.to_rfc3339(),
            tx_to: e.tx_to.map(|t| t.to_rfc3339()),
        }
    }
}

fn parse_param(raw: &str) -> Result<DateTime<Utc>, ApiError> {
    DateTime::parse_from_rfc3339(raw)
        .map(|t| t.with_timezone(&Utc))
        .map_err(|e| ApiError::BadRequest {
            code: "invalid_timestamp",
            message: format!("'{raw}' is not an RFC3339 timestamp: {e}"),
        })
}

async fn get_event(
    State(state): State<AppState>,
    Path(object_id): Path<String>,
    Query(q): Query<AsOfQuery>,
) -> Result<Json<EventRow>, ApiError> {
    let store = state.ontology.object_events();
    let event = if let Some(raw) = q.as_of.as_deref() {
        store.get_valid_as_of(&object_id, parse_param(raw)?).await?
    } else if let Some(raw) = q.tx_as_of.as_deref() {
        store.get_tx_as_of(&object_id, parse_param(raw)?).await?
    } else {
        store.get_tx_current(&object_id).await?
    };
    event
        .map(|e| Json(EventRow::from(e)))
        .ok_or(ApiError::Ontology(OntologyError::NotFound {
            kind: "object_event",
            name: object_id,
        }))
}

async fn list_events(
    State(state): State<AppState>,
    Query(q): Query<AsOfQuery>,
) -> Result<Json<Vec<EventRow>>, ApiError> {
    let store = state.ontology.object_events();
    let events = if let Some(raw) = q.as_of.as_deref() {
        store.list_valid_as_of(parse_param(raw)?).await?
    } else if let Some(raw) = q.tx_as_of.as_deref() {
        store.list_tx_as_of(parse_param(raw)?).await?
    } else {
        store.list_tx_current().await?
    };
    Ok(Json(events.into_iter().map(EventRow::from).collect()))
}
