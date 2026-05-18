//! Fleet audit-verify route (ADR-0038 F3-4). Single-daemon shares
//! one chain — run verify once, replicate per linked repo. Pure read.

use crate::app::AppState;
use crate::error::ApiError;
use axum::extract::{Path, State};
use axum::routing::get;
use axum::{Json, Router};
use serde_json::{json, Value};

/// Mount the route.
pub fn router() -> Router<AppState> {
    Router::new().route("/v1/fleet/plans/:id/audit-verify", get(audit_verify))
}

async fn audit_verify(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<Value>, ApiError> {
    state.fleet_plans.get(&id).await?;
    let mut links = state.fleet_plans.links(&id).await?;
    links.sort_by(|a, b| a.repo.cmp(&b.repo));
    let chain = state.durability.audit().verify(None, None).await?;
    let mut verdicts = Vec::with_capacity(links.len());
    let mut passing = true;
    for l in links {
        if !chain.ok {
            passing = false;
        }
        let mut v = json!({
            "repo": l.repo,
            "repo_plan_id": l.repo_plan_id,
            "ok": chain.ok,
            "checked": chain.checked,
        });
        if let Some(b) = chain.broken_at {
            v["broken_at"] = b.into();
        }
        verdicts.push(v);
    }
    Ok(Json(
        json!({ "fleet_plan_id": id, "passing": passing, "verdicts": verdicts }),
    ))
}
