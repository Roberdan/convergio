//! `/v1/graph/*` — Tier-3 code-graph endpoints (ADR-0014).
//!
//! Trilogy complete in v0.2: `build`, `stats`, `refresh`, `for-task`,
//! `drift`, and `cluster`.

use crate::app::AppState;
use crate::error::ApiError;
use axum::extract::{Path as AxumPath, Query, State};
use axum::routing::{get, post};
use axum::{Json, Router};
use convergio_embed::{linear_blend_fuse, rrf_fuse, semantic_search, DEFAULT_RRF_K};
use convergio_graph::{
    BuildReport, ClusterReport, ContextPack, DriftReport, StructuredContextMetadata,
    DEFAULT_NODE_LIMIT, DEFAULT_TOKEN_BUDGET,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::path::PathBuf;

/// Mount the graph routes.
pub fn router() -> Router<AppState> {
    Router::new()
        .route("/v1/graph/build", post(build))
        .route("/v1/graph/stats", get(stats))
        .route("/v1/graph/refresh", post(refresh))
        .route("/v1/graph/for-task/:id", get(for_task))
        .route("/v1/graph/drift", get(drift))
        .route("/v1/graph/cluster/:crate_name", get(cluster))
}

#[derive(Debug, Deserialize, Default)]
struct BuildRequest {
    /// Workspace manifest dir. Defaults to the daemon's cwd.
    #[serde(default)]
    manifest_dir: Option<String>,
    /// Force re-parse even if mtime is unchanged.
    #[serde(default)]
    force: bool,
}

#[derive(Debug, Serialize)]
struct StatsResponse {
    nodes: usize,
    edges: usize,
}

async fn build(
    State(state): State<AppState>,
    Json(req): Json<BuildRequest>,
) -> Result<Json<BuildReport>, ApiError> {
    let manifest = req
        .manifest_dir
        .map(PathBuf::from)
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));
    let report = convergio_graph::build(&manifest, &state.graph, req.force)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;
    Ok(Json(report))
}

async fn stats(State(state): State<AppState>) -> Result<Json<StatsResponse>, ApiError> {
    let nodes = state
        .graph
        .count_nodes()
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;
    let edges = state
        .graph
        .count_edges()
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;
    Ok(Json(StatsResponse { nodes, edges }))
}

#[derive(Debug, Deserialize, Default)]
struct ForTaskQuery {
    /// Override the default node-count cap.
    #[serde(default)]
    node_limit: Option<usize>,
    /// Override the default token budget.
    #[serde(default)]
    token_budget: Option<u64>,
    /// Primary crate scope.
    #[serde(default, rename = "crate")]
    primary_crate: Option<String>,
    /// Comma-separated related crates.
    #[serde(default)]
    related_crates: Option<String>,
    /// Comma-separated required ADR ids or paths.
    #[serde(default)]
    adr_required: Option<String>,
    /// Comma-separated required documentation paths.
    #[serde(default)]
    docs_required: Option<String>,
    /// Named validation profile.
    #[serde(default)]
    validation_profile: Option<String>,
    /// Set to `1` (or any truthy value) to fuse substring + semantic
    /// retrieval via RRF (ADR-0038 § 5.5). When unset the response
    /// shape is unchanged from F1-α (`ContextPack` JSON).
    #[serde(default)]
    semantic: Option<String>,
    /// Top-K semantic neighbors to feed into RRF when `semantic=1`.
    /// Capped at 100 server-side.
    #[serde(default)]
    semantic_top_k: Option<usize>,
    /// Structural weight for linear-blend fusion (`0.0`–`1.0`).
    /// When set, overrides RRF with a rank-normalised linear blend:
    /// `score = alpha × structural + (1 − alpha) × semantic`.
    /// `1.0` = pure structural; `0.0` = pure semantic.
    /// Ignored unless `semantic=1`.
    #[serde(default)]
    alpha: Option<f64>,
}

fn parse_truthy(s: &str) -> bool {
    matches!(
        s.trim().to_ascii_lowercase().as_str(),
        "1" | "true" | "yes" | "on"
    )
}

/// `GET /v1/graph/for-task/:id` — context-pack for the named task.
///
/// When `?semantic=1` the response also carries an `hybrid_hits`
/// section (ADR-0038 § 5.5): RRF fusion of the substring matches
/// against `convergio-embed::semantic_search`, joined on file path.
async fn for_task(
    State(state): State<AppState>,
    AxumPath(id): AxumPath<String>,
    Query(q): Query<ForTaskQuery>,
) -> Result<Json<Value>, ApiError> {
    let task = state.durability.tasks().get(&id).await?;
    let text = format!(
        "{}\n{}",
        task.title,
        task.description.as_deref().unwrap_or("")
    );
    let metadata =
        StructuredContextMetadata::from_task_text(&text).merged_with(StructuredContextMetadata {
            primary_crate: q.primary_crate,
            related_crates: split_query_list(q.related_crates),
            adr_required: split_query_list(q.adr_required),
            docs_required: split_query_list(q.docs_required),
            validation_profile: q.validation_profile,
        });
    let pack = convergio_graph::for_task_text_with_metadata(
        &state.graph,
        &task.id,
        &text,
        metadata,
        q.node_limit.unwrap_or(DEFAULT_NODE_LIMIT),
        q.token_budget.unwrap_or(DEFAULT_TOKEN_BUDGET),
    )
    .await
    .map_err(|e| ApiError::Internal(e.to_string()))?;

    let mut body = serde_json::to_value(&pack)
        .map_err(|e| ApiError::Internal(format!("serialise pack: {e}")))?;

    if q.semantic.as_deref().is_some_and(parse_truthy) {
        let hybrid = build_hybrid_section(&state, &pack, &text, q.semantic_top_k, q.alpha).await?;
        if let Value::Object(ref mut map) = body {
            map.insert("hybrid".to_string(), hybrid);
        }
    }
    Ok(Json(body))
}

async fn build_hybrid_section(
    state: &AppState,
    pack: &ContextPack,
    query_text: &str,
    semantic_top_k: Option<usize>,
    alpha: Option<f64>,
) -> Result<Value, ApiError> {
    let top_k = semantic_top_k.unwrap_or(25).clamp(1, 100);
    // Structural ranks come from matched_files, ordered by node_count
    // descending (the existing relevance heuristic). The RRF input
    // is a list of file-path ids in rank order.
    let structural_ids: Vec<&str> = pack.files.iter().map(|f| f.path.as_str()).collect();
    let semantic_neighbors = match semantic_search(
        &state.embed,
        state.embedder.as_ref(),
        query_text,
        top_k,
    )
    .await
    {
        Ok(v) => v,
        Err(convergio_embed::EmbedError::EmbedderFailed(msg)) => {
            tracing::warn!(error = %msg, "semantic unavailable; falling back to structural-only");
            Vec::new()
        }
        Err(e) => return Err(ApiError::Internal(format!("semantic search: {e}"))),
    };
    let semantic_ids: Vec<&str> = semantic_neighbors
        .iter()
        .map(|n| n.node_id.as_str())
        .collect();
    if let Some(a) = alpha {
        let a = a.clamp(0.0, 1.0);
        let hits = linear_blend_fuse(&structural_ids, &semantic_ids, a);
        Ok(json!({
            "fusion": "linear_blend",
            "alpha": a,
            "model": state.embedder.model_id(),
            "structural_count": pack.files.len(),
            "semantic_count": semantic_neighbors.len(),
            "hits": hits,
        }))
    } else {
        let hits = rrf_fuse(&structural_ids, &semantic_ids, DEFAULT_RRF_K);
        Ok(json!({
            "fusion": "rrf",
            "k": DEFAULT_RRF_K,
            "model": state.embedder.model_id(),
            "structural_count": pack.files.len(),
            "semantic_count": semantic_neighbors.len(),
            "hits": hits,
        }))
    }
}

fn split_query_list(value: Option<String>) -> Vec<String> {
    value
        .as_deref()
        .unwrap_or("")
        .split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(ToString::to_string)
        .collect()
}

/// `POST /v1/graph/refresh` — lefthook nudge after a commit.
/// Re-runs an incremental build against the daemon's cwd. Returns
/// the build report so a caller can verify what changed. Idempotent.
async fn refresh(State(state): State<AppState>) -> Result<Json<BuildReport>, ApiError> {
    let manifest = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let report = convergio_graph::build(&manifest, &state.graph, false)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;
    Ok(Json(report))
}

#[derive(Debug, Deserialize, Default)]
struct DriftQuery {
    /// Repo root for the git diff (defaults to daemon cwd).
    #[serde(default)]
    repo_root: Option<String>,
    /// Git ref to compare against (default `origin/main`).
    #[serde(default)]
    since: Option<String>,
    /// Optional ADR id to scope the declared set to a single ADR.
    #[serde(default)]
    adr: Option<String>,
}

/// `GET /v1/graph/drift` — ADR claims vs git diff (advisory).
async fn drift(
    State(state): State<AppState>,
    Query(q): Query<DriftQuery>,
) -> Result<Json<DriftReport>, ApiError> {
    let root = q
        .repo_root
        .map(PathBuf::from)
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));
    let since = q.since.as_deref().unwrap_or("origin/main");
    let report = convergio_graph::drift_since(&state.graph, &root, since, q.adr.as_deref())
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;
    Ok(Json(report))
}

#[derive(Debug, Deserialize, Default)]
struct ClusterQuery {
    /// Optional target line count. Communities exceeding this are
    /// flagged in `above_target`.
    #[serde(default)]
    target_loc: Option<u64>,
}

/// `GET /v1/graph/cluster/:crate_name` — community detection over the
/// per-crate file graph (label propagation). Suggests split seams.
async fn cluster(
    State(state): State<AppState>,
    AxumPath(crate_name): AxumPath<String>,
    Query(q): Query<ClusterQuery>,
) -> Result<Json<ClusterReport>, ApiError> {
    let report = convergio_graph::cluster_for_crate(&state.graph, &crate_name, q.target_loc)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;
    Ok(Json(report))
}
