//! `/v1/ontology/diff|lineage|branch-diff/*` — graph output surface
//! for the Ontology Runtime Core (ADR-0060, W1 T9).
//!
//! Endpoints:
//!
//! - `GET /v1/ontology/diff/object/:name?from=N&to=M&format=…`
//! - `GET /v1/ontology/lineage/object/:name?format=…`
//! - `GET /v1/ontology/branch-diff/object/:name?format=…` — always
//!   returns 501 in W1; the branching primitive itself ships in a
//!   later ADR (ADR-0059).
//!
//! Supported `format` values: `json` (default), `mermaid`, `dot`.
//! Output bytes for `mermaid` / `dot` are byte-identical to the
//! crate-level renderers (golden-tested in `convergio-ontology`).

use crate::AppState;
use axum::extract::{Path, Query, State};
use axum::http::header;
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use axum::Router;
use convergio_ontology::{
    diff_object, lineage_object, render_diff_dot, render_diff_mermaid, render_lineage_dot,
    render_lineage_mermaid, Error as OntologyError,
};
use convergio_server_core::ApiError;
use serde::Deserialize;

/// Mount the graph routes. Composed alongside the rest of the
/// ontology routes from `convergio-server::router::build`.
pub fn router() -> Router<AppState> {
    Router::new()
        .route("/v1/ontology/diff/object/:name", get(diff))
        .route("/v1/ontology/lineage/object/:name", get(lineage))
        .route("/v1/ontology/branch-diff/object/:name", get(branch_diff))
}

#[derive(Deserialize)]
struct DiffQuery {
    from: i64,
    to: i64,
    #[serde(default)]
    format: Option<String>,
}

#[derive(Deserialize)]
struct LineageQuery {
    #[serde(default)]
    format: Option<String>,
}

async fn diff(
    State(state): State<AppState>,
    Path(name): Path<String>,
    Query(q): Query<DiffQuery>,
) -> Result<Response, ApiError> {
    let d = diff_object(&state.ontology, &name, q.from, q.to).await?;
    Ok(render_response(q.format.as_deref(), || {
        (
            render_diff_mermaid(&d),
            render_diff_dot(&d),
            serde_json::to_vec_pretty(&d).unwrap_or_default(),
        )
    }))
}

async fn lineage(
    State(state): State<AppState>,
    Path(name): Path<String>,
    Query(q): Query<LineageQuery>,
) -> Result<Response, ApiError> {
    let l = lineage_object(&state.ontology, &name).await?;
    Ok(render_response(q.format.as_deref(), || {
        (
            render_lineage_mermaid(&l),
            render_lineage_dot(&l),
            serde_json::to_vec_pretty(&l).unwrap_or_default(),
        )
    }))
}

async fn branch_diff(
    State(_state): State<AppState>,
    Path(_name): Path<String>,
    Query(_q): Query<LineageQuery>,
) -> Result<Response, ApiError> {
    Err(ApiError::Ontology(OntologyError::NotImplemented {
        feature: "ontology branch-diff (ADR-0059)",
    }))
}

/// Build the HTTP response based on the `format` query param.
fn render_response<F>(format: Option<&str>, f: F) -> Response
where
    F: FnOnce() -> (String, String, Vec<u8>),
{
    let (mermaid, dot, json) = f();
    match format.unwrap_or("json") {
        "mermaid" => (
            [(header::CONTENT_TYPE, "text/vnd.mermaid; charset=utf-8")],
            mermaid,
        )
            .into_response(),
        "dot" => (
            [(header::CONTENT_TYPE, "text/vnd.graphviz; charset=utf-8")],
            dot,
        )
            .into_response(),
        _ => ([(header::CONTENT_TYPE, "application/json")], json).into_response(),
    }
}

#[allow(dead_code)]
fn _format_marker() {}
