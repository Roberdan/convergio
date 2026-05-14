//! Regression tests for `Bridge::daemon_response` JSON decoding.
//!
//! Audit (`docs/reviews/crate-audits/convergio-mcp.md` L3):
//! Invalid daemon JSON used to be collapsed to `{}`, so a malformed
//! daemon response masqueraded as a successful empty payload. The
//! bridge MUST surface a protocol-mapping error instead.
use crate::bridge::Bridge;
use axum::{
    http::{header, StatusCode},
    response::IntoResponse,
    routing::get,
    Router,
};
use convergio_api::AgentCode;
use tokio::net::TcpListener;

#[tokio::test]
async fn daemon_invalid_json_surfaces_error_instead_of_empty_success() {
    let url = spawn_garbage_daemon().await;
    let bridge = Bridge::new(url);

    let response = bridge.get("/v1/anything").await;

    assert!(
        !response.ok,
        "expected error for invalid daemon JSON, got: {response:?}",
    );
    assert_eq!(response.code, AgentCode::Error);
    assert!(
        response.message.to_lowercase().contains("json"),
        "expected message to mention JSON; got {:?}",
        response.message,
    );
}

async fn spawn_garbage_daemon() -> String {
    async fn garbage() -> impl IntoResponse {
        (
            StatusCode::OK,
            [(header::CONTENT_TYPE, "application/json")],
            "<<not json at all>>",
        )
    }
    let app = Router::new().route("/v1/anything", get(garbage));
    let listener = TcpListener::bind(("127.0.0.1", 0)).await.unwrap();
    let address = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    format!("http://{address}")
}

#[tokio::test]
async fn daemon_plaintext_404_maps_to_not_found_not_error() {
    // Framework-default 404 bodies are plain text. The early-return
    // on JSON decode failure must still honour status == NOT_FOUND
    // so callers can distinguish "endpoint missing" from "daemon
    // returned a malformed payload".
    let url = spawn_plaintext_404_daemon().await;
    let bridge = Bridge::new(url);
    let response = bridge.get("/v1/missing").await;
    assert!(!response.ok);
    assert_eq!(response.code, AgentCode::NotFound);
}

async fn spawn_plaintext_404_daemon() -> String {
    async fn not_found() -> impl IntoResponse {
        (
            StatusCode::NOT_FOUND,
            [(header::CONTENT_TYPE, "text/plain; charset=utf-8")],
            "not found",
        )
    }
    let app = Router::new().fallback(not_found);
    let listener = TcpListener::bind(("127.0.0.1", 0)).await.unwrap();
    let address = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    format!("http://{address}")
}
