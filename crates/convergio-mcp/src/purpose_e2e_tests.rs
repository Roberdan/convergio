//! Regression guard for the post-#443 purpose-binding fix.
//!
//! Verifies that `Bridge::new` builds a client that sends `x-purpose-id` on
//! every outbound request, so the server's purpose-enforcement middleware does
//! not return 400.

use crate::bridge::Bridge;
use axum::{
    extract::State,
    http::{HeaderMap, StatusCode},
    routing::get,
    Json, Router,
};
use convergio_api::{ActRequest, Action, SCHEMA_VERSION};
use serde_json::{json, Value};
use std::sync::Arc;
use tokio::{net::TcpListener, sync::Mutex};

type ReceivedHeader = Arc<Mutex<Option<String>>>;

/// Spawn a stub daemon that requires `x-purpose-id` on `/v1/status`.
///
/// Returns the URL and a shared cell that holds the header value received on
/// the first successful request.
async fn spawn_purpose_enforcing_stub() -> (String, ReceivedHeader) {
    let captured: ReceivedHeader = Arc::new(Mutex::new(None));
    let captured_clone = captured.clone();

    async fn status_with_header_check(
        State(captured): State<ReceivedHeader>,
        headers: HeaderMap,
    ) -> (StatusCode, Json<Value>) {
        let purpose = headers
            .get(convergio_api::PURPOSE_ID_HEADER)
            .and_then(|v| v.to_str().ok())
            .map(str::to_owned);
        if purpose.is_none() {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({"error": {"code": "purpose_id_missing"}})),
            );
        }
        *captured.lock().await = purpose;
        (StatusCode::OK, Json(json!({"ok": true})))
    }

    let app = Router::new()
        .route("/v1/status", get(status_with_header_check))
        .with_state(captured_clone);
    let listener = TcpListener::bind(("127.0.0.1", 0)).await.unwrap();
    let address = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    (format!("http://{address}"), captured)
}

/// Verify that `Bridge::new` sends the purpose-id header so the real daemon's
/// purpose-binding middleware does not reject the request with HTTP 400.
#[tokio::test]
async fn bridge_sends_purpose_id_header() {
    let (url, received_header) = spawn_purpose_enforcing_stub().await;
    let bridge = Bridge::new(url);

    let resp = bridge
        .dispatch(ActRequest {
            schema_version: SCHEMA_VERSION.into(),
            action: Action::Status,
            params: json!({}),
        })
        .await;

    // The stub returns ok:true only when the header is present.
    assert!(
        resp.ok,
        "bridge must send x-purpose-id; stub returned: {:?}",
        resp.message
    );
    let header_value = received_header.lock().await.clone();
    assert!(
        header_value.is_some(),
        "stub must have seen the x-purpose-id header"
    );
    // The header must look like a UUID (xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx).
    let raw = header_value.unwrap();
    assert_eq!(
        raw.len(),
        36,
        "x-purpose-id must be a 36-char UUID, got: {raw:?}"
    );
    assert_eq!(
        raw.chars().filter(|c| *c == '-').count(),
        4,
        "x-purpose-id must be a UUID with 4 hyphens, got: {raw:?}"
    );
}
