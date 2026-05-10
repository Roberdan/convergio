//! `GET /v1/api/actions` — discoverable action type registry
//! (P3-1, Palantir-inspired Action types).
//!
//! Returns the same `ActionMetadata` list `convergio_api::actions_registry()`
//! produces. The MCP bridge and any external tool can consume this
//! without re-implementing the catalog or shelling out to `cvg`.

use crate::app::AppState;
use axum::body::Bytes;
use axum::http::header;
use axum::response::IntoResponse;
use axum::routing::get;
use axum::Router;
use convergio_api::actions_json_bytes;

/// Mount the actions registry route.
pub fn router() -> Router<AppState> {
    Router::new().route("/v1/api/actions", get(actions))
}

async fn actions() -> impl IntoResponse {
    (
        [(header::CONTENT_TYPE, "application/json")],
        Bytes::from_static(actions_json_bytes()),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::to_bytes;

    #[tokio::test]
    async fn actions_response_has_known_shape() {
        let response = actions().await.into_response();
        assert_eq!(
            response.headers().get(header::CONTENT_TYPE).unwrap(),
            "application/json"
        );

        let bytes = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("body bytes");
        let json: serde_json::Value = serde_json::from_slice(&bytes).expect("valid JSON");

        assert_eq!(json["schema_version"], convergio_api::SCHEMA_VERSION);
        let actions = json["actions"].as_array().expect("actions array");
        assert!(!actions.is_empty());

        for a in actions {
            assert!(!a["name"].as_str().unwrap_or_default().is_empty());
            assert!(!a["capability"].as_str().unwrap_or_default().is_empty());
            assert!(!a["summary"].as_str().unwrap_or_default().is_empty());
        }
    }
}
