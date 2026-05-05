//! `GET /v1/api/actions` — discoverable action type registry
//! (P3-1, Palantir-inspired Action types).
//!
//! Returns the same `ActionMetadata` list `convergio_api::actions_registry()`
//! produces. The MCP bridge and any external tool can consume this
//! without re-implementing the catalog or shelling out to `cvg`.

use crate::app::AppState;
use axum::routing::get;
use axum::{Json, Router};
use convergio_api::{actions_registry, ActionMetadata, SCHEMA_VERSION};
use serde::Serialize;

#[derive(Serialize)]
struct ActionsResponse {
    schema_version: &'static str,
    actions: Vec<ActionMetadata>,
}

/// Mount the actions registry route.
pub fn router() -> Router<AppState> {
    Router::new().route("/v1/api/actions", get(actions))
}

async fn actions() -> Json<ActionsResponse> {
    Json(ActionsResponse {
        schema_version: SCHEMA_VERSION,
        actions: actions_registry(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn actions_response_has_known_shape() {
        let resp = actions().await;
        assert_eq!(resp.0.schema_version, SCHEMA_VERSION);
        assert!(!resp.0.actions.is_empty());
        // Every entry has a non-empty name + capability + summary.
        for a in &resp.0.actions {
            assert!(!a.name.is_empty(), "action name is empty");
            assert!(!a.capability.is_empty(), "{} capability empty", a.name);
            assert!(!a.summary.is_empty(), "{} summary empty", a.name);
        }
    }
}
