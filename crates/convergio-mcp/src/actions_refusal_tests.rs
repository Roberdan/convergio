//! Regression tests for `ExplainLastRefusal` param handling.
//!
//! Audit (`docs/reviews/crate-audits/convergio-mcp.md`): the dispatch
//! match arm discarded `request.params`, so a caller-supplied `task_id`
//! filter was silently replaced by whichever refusal happened to be in
//! bridge-local memory. These tests pin the documented behaviour from
//! `help.rs` (`Action::ExplainLastRefusal => {"task_id": "uuid?"}`).
use crate::bridge::Bridge;
use axum::{
    extract::{Query, State},
    routing::get,
    Json, Router,
};
use convergio_api::{ActRequest, Action, SCHEMA_VERSION};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};
use tokio::net::TcpListener;
use tokio::sync::Mutex;

struct Stub {
    last_query: Mutex<HashMap<String, String>>,
    calls: AtomicUsize,
}

#[tokio::test]
async fn explain_last_refusal_forwards_caller_task_id_over_bridge_memory() {
    let (url, stub) = spawn_stub_daemon().await;
    let bridge = Bridge::new(url);

    *bridge.last_refusal.lock().await = Some(json!({
        "path": "/v1/tasks/task-1/transition",
        "status": 409,
        "error": {"code": "gate_refused", "data": {"task_id": "task-1"}},
    }));

    let response = bridge
        .dispatch(ActRequest {
            schema_version: SCHEMA_VERSION.into(),
            action: Action::ExplainLastRefusal,
            params: json!({"task_id": "task-2"}),
        })
        .await;

    assert!(response.ok, "expected ok response: {response:?}");
    let recorded = stub.last_query.lock().await.clone();
    assert_eq!(
        recorded.get("task_id").map(String::as_str),
        Some("task-2"),
        "caller task_id must beat bridge-local memory; got query={recorded:?}",
    );
    assert_eq!(
        stub.calls.load(Ordering::SeqCst),
        1,
        "exactly one daemon call expected",
    );
}

#[tokio::test]
async fn explain_last_refusal_falls_back_to_bridge_memory_task_id() {
    let (url, stub) = spawn_stub_daemon().await;
    let bridge = Bridge::new(url);

    *bridge.last_refusal.lock().await = Some(json!({
        "path": "/v1/tasks/task-7/transition",
        "status": 409,
        "error": {"code": "gate_refused", "data": {"task_id": "task-7"}},
    }));

    let response = bridge
        .dispatch(ActRequest {
            schema_version: SCHEMA_VERSION.into(),
            action: Action::ExplainLastRefusal,
            params: json!({}),
        })
        .await;

    assert!(response.ok, "expected ok response: {response:?}");
    let recorded = stub.last_query.lock().await.clone();
    assert_eq!(
        recorded.get("task_id").map(String::as_str),
        Some("task-7"),
        "memory task_id used when caller omits one; got query={recorded:?}",
    );
}

async fn spawn_stub_daemon() -> (String, Arc<Stub>) {
    let stub = Arc::new(Stub {
        last_query: Mutex::new(HashMap::new()),
        calls: AtomicUsize::new(0),
    });
    let app = Router::new()
        .route("/v1/audit/refusals/latest", get(latest_refusal))
        .with_state(stub.clone());
    let listener = TcpListener::bind(("127.0.0.1", 0)).await.unwrap();
    let address = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    (format!("http://{address}"), stub)
}

async fn latest_refusal(
    State(stub): State<Arc<Stub>>,
    Query(params): Query<HashMap<String, String>>,
) -> Json<Value> {
    stub.calls.fetch_add(1, Ordering::SeqCst);
    *stub.last_query.lock().await = params.clone();
    let task_id = params.get("task_id").cloned().unwrap_or_default();
    Json(json!({
        "task_id": task_id,
        "code": "gate_refused",
        "message": "persisted refusal",
    }))
}
