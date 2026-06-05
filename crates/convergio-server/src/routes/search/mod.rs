//! `/api/ unified exploration search surface.search`
//!
//! Fuses a few local-first sources:
//! - structured: plans, tasks, capabilities, fleet repos, action registry
//! - semantic: embedding neighbors via `convergio-embed`
//! - operational: agent registry + running supervised processes

use crate::app::AppState;
use crate::error::ApiError;
use axum::extract::{Query, State};
use axum::routing::get;
use axum::{Json, Router};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

mod sources;
mod util;

/// Mount `/api/search`.
pub fn router() -> Router<AppState> {
    Router::new().route("/api/search", get(search))
}

#[derive(Debug, Deserialize)]
struct SearchQuery {
    /// Free-text query.
    q: Option<String>,
    /// Maximum number of results to return (default 25, max 100).
    limit: Option<usize>,
}

#[derive(Debug, Serialize)]
struct SearchResponse {
    query: String,
    limit: usize,
    degraded_semantic: bool,
    results: Vec<SearchResult>,
}

#[derive(Debug, Clone, Serialize)]
struct SearchResult {
    /// Result kind (`plan`, `task`, `capability`, `repo`, `action`, `agent`, `process`, `gh_run`, `gh_job`, `graph_node`, `doc`).
    #[serde(rename = "type")]
    kind: String,
    /// Stable identifier within the kind.
    id: String,
    /// Primary display label.
    title: String,
    /// Optional secondary label.
    #[serde(skip_serializing_if = "Option::is_none")]
    subtitle: Option<String>,
    /// UI routing hint.
    href: String,
    /// Normalised score (higher is better).
    score: f64,
    /// One or more match sources (`structured`, `semantic`, `operational`).
    match_sources: Vec<String>,
}

async fn search(
    State(state): State<AppState>,
    Query(q): Query<SearchQuery>,
) -> Result<Json<SearchResponse>, ApiError> {
    let query = q.q.unwrap_or_default();
    let query = query.trim().to_string();
    if query.is_empty() {
        return Err(ApiError::BadRequest {
            code: "search_query_empty",
            message: "missing query parameter 'q'".into(),
        });
    }
    let limit = q.limit.unwrap_or(25).clamp(1, 100);

    let mut merged: HashMap<(String, String), SearchResult> = HashMap::new();

    sources::collect_structured(&state, &query, &mut merged).await?;
    sources::collect_operational(&state, &query, &mut merged).await?;
    sources::collect_graph(&state, &query, &mut merged).await;
    let degraded_semantic = sources::collect_semantic(&state, &query, &mut merged).await?;

    let mut results: Vec<SearchResult> = merged.into_values().collect();
    results.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.kind.cmp(&b.kind))
            .then_with(|| a.title.cmp(&b.title))
            .then_with(|| a.id.cmp(&b.id))
    });
    results.truncate(limit);

    Ok(Json(SearchResponse {
        query,
        limit,
        degraded_semantic,
        results,
    }))
}
