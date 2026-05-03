//! `/v1/embed/*` — Tier-3 semantic retrieval (ADR-0038, F1-γ).
//!
//! Routes:
//! - `GET  /v1/embed/stats` — store inventory probe (F1-α)
//! - `POST /v1/embed/warm` — load the embedder, embed a sentinel,
//!   report `{model, dim, ms}`. Useful as a health probe and to
//!   trigger the model download out of the request hot path.
//! - `POST /v1/embed/build` — walk a directory, embed eligible files,
//!   upsert into the store. Idempotent via `source_hash`.
//! - `POST /v1/embed/for-task` — semantic-only nearest neighbours
//!   for the supplied query text.

use crate::app::AppState;
use crate::error::ApiError;
use axum::extract::{Query, State};
use axum::routing::{get, post};
use axum::{Json, Router};
use convergio_embed::{
    collect_files, ingest, semantic_search, Neighbor, DEFAULT_MAX_LINES, SOURCE_EXTENSIONS,
};
use serde::Deserialize;
use serde_json::{json, Value};
use std::path::PathBuf;

/// Mount the embed routes onto the daemon router.
pub fn router() -> Router<AppState> {
    Router::new()
        .route("/v1/embed/stats", get(stats))
        .route("/v1/embed/warm", post(warm))
        .route("/v1/embed/build", post(build))
        .route("/v1/embed/for-task", post(for_task))
}

#[derive(Debug, Deserialize)]
struct StatsQuery {
    repo: Option<String>,
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
    Ok(Json(json!({"ok": true, "count": count, "repo": q.repo})))
}

async fn warm(State(state): State<AppState>) -> Result<Json<Value>, ApiError> {
    let started = std::time::Instant::now();
    let v = state
        .embedder
        .embed("convergio")
        .map_err(|e| ApiError::Internal(format!("warm failed: {e}")))?;
    let ms = started.elapsed().as_millis() as u64;
    Ok(Json(json!({
        "ok": true,
        "model": state.embedder.model_id(),
        "dim": v.len(),
        "ms": ms,
    })))
}

#[derive(Debug, Deserialize)]
struct BuildBody {
    /// Repo identifier written into `graph_node_embeddings.repo`.
    /// Defaults to `"convergio"` when omitted.
    #[serde(default)]
    repo: Option<String>,
    /// Directory to walk. Required.
    root: PathBuf,
    /// Override the default file-extension allowlist.
    #[serde(default)]
    extensions: Option<Vec<String>>,
    /// Override the default per-file truncation (200 lines).
    #[serde(default)]
    max_lines: Option<usize>,
}

async fn build(
    State(state): State<AppState>,
    Json(body): Json<BuildBody>,
) -> Result<Json<Value>, ApiError> {
    let repo = body.repo.unwrap_or_else(|| "convergio".to_string());
    if !body.root.is_dir() {
        return Err(ApiError::BadRequest {
            code: "embed_build_root_invalid",
            message: format!("root is not a directory: {}", body.root.display()),
        });
    }
    let max_lines = body.max_lines.unwrap_or(DEFAULT_MAX_LINES);
    let owned_exts: Vec<String>;
    let ext_refs: Vec<&str>;
    let exts: &[&str] = match &body.extensions {
        Some(list) => {
            owned_exts = list.clone();
            ext_refs = owned_exts.iter().map(String::as_str).collect();
            &ext_refs
        }
        None => SOURCE_EXTENSIONS,
    };

    let started = std::time::Instant::now();
    let nodes = collect_files(&repo, &body.root, exts, max_lines);
    let report = ingest(&state.embed, state.embedder.as_ref(), nodes)
        .await
        .map_err(|e| ApiError::Internal(format!("ingest failed: {e}")))?;
    let ms = started.elapsed().as_millis() as u64;
    Ok(Json(json!({
        "ok": true,
        "repo": repo,
        "root": body.root.display().to_string(),
        "model": state.embedder.model_id(),
        "report": {
            "considered": report.considered,
            "embedded": report.embedded,
            "skipped_unchanged": report.skipped_unchanged,
            "failed": report.failed,
        },
        "ms": ms,
    })))
}

#[derive(Debug, Deserialize)]
struct ForTaskBody {
    /// Free-text query (typically the task body).
    query: String,
    /// Top-K neighbours to return. Defaults to 25.
    #[serde(default)]
    top_k: Option<usize>,
}

async fn for_task(
    State(state): State<AppState>,
    Json(body): Json<ForTaskBody>,
) -> Result<Json<Value>, ApiError> {
    let limit = body.top_k.unwrap_or(25).clamp(1, 100);
    let started = std::time::Instant::now();
    let hits = semantic_search(&state.embed, state.embedder.as_ref(), &body.query, limit)
        .await
        .map_err(|e| ApiError::Internal(format!("semantic search failed: {e}")))?;
    let ms = started.elapsed().as_millis() as u64;
    Ok(Json(json!({
        "ok": true,
        "model": state.embedder.model_id(),
        "ms": ms,
        "hits": hits.iter().map(neighbor_json).collect::<Vec<_>>(),
    })))
}

fn neighbor_json(n: &Neighbor) -> Value {
    json!({
        "repo": n.repo,
        "node_id": n.node_id,
        "score": n.score,
        "match_source": "semantic",
    })
}
