//! `GET /v1/fleet/duplicates` — cross-repo near-exact duplicate pairs (ADR-0038, F2-10).

use axum::extract::{Query, State};
use axum::Json;
use convergio_fleet::find_duplicates;
use convergio_server_core::ApiError;
use convergio_server_core::AppState;
use serde::Deserialize;

use crate::bitemporal::parse_bitemporal;
use serde_json::{json, Value};

/// Query parameters for `GET /v1/fleet/duplicates`.
#[derive(Debug, Deserialize)]
pub(super) struct DuplicatesQuery {
    #[serde(default)]
    as_of: Option<String>,
    #[serde(default)]
    tx_as_of: Option<String>,
    /// Cosine similarity threshold (default 0.95).
    #[serde(default = "default_cosine")]
    cosine: f64,
    /// Restrict to one undirected repo pair: `"repo_a:repo_b"`.
    repo_pair: Option<String>,
    /// Include 1–3 line semantic diff preview per pair.
    #[serde(default)]
    diff_preview: bool,
}

fn default_cosine() -> f64 {
    0.95
}

/// `GET /v1/fleet/duplicates` — near-exact cross-repo duplicate pairs.
pub(super) async fn duplicates(
    State(state): State<AppState>,
    Query(q): Query<DuplicatesQuery>,
) -> Result<Json<Value>, ApiError> {
    let _ = parse_bitemporal(q.as_of.as_deref(), q.tx_as_of.as_deref())?;
    let repo_pair = q
        .repo_pair
        .as_deref()
        .map(parse_repo_pair)
        .transpose()
        .map_err(|msg| ApiError::BadRequest {
            code: "invalid_repo_pair",
            message: msg,
        })?;
    let rp = repo_pair.as_ref().map(|(a, b)| (a.as_str(), b.as_str()));
    let pairs = find_duplicates(&state.fleet, q.cosine as f32, rp, q.diff_preview)
        .await
        .map_err(|e| ApiError::Internal(format!("duplicates query failed: {e}")))?;
    let total = pairs.len();
    Ok(Json(json!({ "pairs": pairs, "total": total })))
}

/// Parse `"repo_a:repo_b"` into `(repo_a, repo_b)`.
fn parse_repo_pair(s: &str) -> Result<(String, String), String> {
    let mut parts = s.splitn(2, ':');
    let a = parts.next().unwrap_or("").trim();
    let b = parts.next().unwrap_or("").trim();
    if a.is_empty() || b.is_empty() {
        return Err(format!("repo_pair must be 'repo_a:repo_b', got: {s}"));
    }
    Ok((a.to_owned(), b.to_owned()))
}
