//! MCP `llm.call` forwarding test (W5, ADR-0058). Split from
//! `e2e_tests.rs` to keep that file under the 300-line Rust cap.

use crate::bridge::Bridge;
use axum::{extract::State, routing::post, Json, Router};
use convergio_api::{ActRequest, Action, SCHEMA_VERSION};
use serde_json::{json, Value};
use std::sync::{Arc, Mutex};
use tokio::net::TcpListener;

#[tokio::test]
async fn llm_call_action_round_trips_to_gateway_path() {
    // W5/ADR-0058 acceptance: the `llm.call` action must reach
    // `POST /v1/llm-gateway/call` with the request body intact.
    #[derive(Default)]
    struct Calls {
        log: Mutex<Vec<(String, String, Value)>>,
    }
    let calls = Arc::new(Calls::default());
    let state = calls.clone();
    let app = Router::new()
        .route(
            "/v1/llm-gateway/call",
            post(
                |State(s): State<Arc<Calls>>, Json(body): Json<Value>| async move {
                    s.log.lock().unwrap().push((
                        "POST".into(),
                        "/v1/llm-gateway/call".into(),
                        body,
                    ));
                    Json(json!({
                        "result": {"output_text": "hello"},
                        "provenance": {},
                        "meta": {"provider_id": "stub", "cache_hit": false},
                        "egress": {"injection_flagged": false}
                    }))
                },
            ),
        )
        .with_state(state);
    let listener = TcpListener::bind(("127.0.0.1", 0)).await.unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
    let bridge = Bridge::new(url);

    let call = bridge
        .dispatch(ActRequest {
            schema_version: SCHEMA_VERSION.into(),
            action: Action::LlmCall,
            params: json!({
                "purpose": "demo",
                "model_id": "test-model",
                "prompt": "ping",
                "max_output_tokens": 16
            }),
        })
        .await;
    assert!(call.ok, "llm.call: {call:?}");
    assert_eq!(
        call.data.as_ref().unwrap()["result"]["output_text"],
        "hello"
    );

    let log = calls.log.lock().unwrap().clone();
    assert_eq!(log.len(), 1, "{log:?}");
    assert_eq!(log[0].0, "POST");
    assert_eq!(log[0].1, "/v1/llm-gateway/call");
    assert_eq!(log[0].2["purpose"], "demo");
    assert_eq!(log[0].2["model_id"], "test-model");
    assert_eq!(log[0].2["prompt"], "ping");
    assert_eq!(log[0].2["max_output_tokens"], 16);
}
