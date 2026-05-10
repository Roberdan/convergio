use crate::bridge::Bridge;
use crate::help;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
use convergio_api::{
    ActRequest, Action, AgentCode, HelpRequest, HelpTopic, HelpVerbosity, NextHint, SCHEMA_VERSION,
};
use serde_json::{json, Value};
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};
use tokio::{net::TcpListener, sync::Mutex};

struct StubDaemon {
    requests: AtomicUsize,
    last_transition: Mutex<Option<Value>>,
}

#[tokio::test]
async fn bridge_contract_rejects_mismatch_and_maps_gate_refusal() {
    let (url, daemon) = spawn_stub_daemon().await;
    let bridge = Bridge::new(url);

    assert_help_contract();

    let mismatch = bridge
        .dispatch(ActRequest {
            schema_version: "1".into(),
            action: Action::Status,
            params: json!({}),
        })
        .await;
    assert!(!mismatch.ok);
    assert_eq!(mismatch.code, AgentCode::SchemaVersionMismatch);
    assert_eq!(mismatch.next, Some(NextHint::RefreshHelp));
    assert_eq!(daemon.requests.load(Ordering::SeqCst), 0);

    let refused = bridge
        .dispatch(ActRequest {
            schema_version: SCHEMA_VERSION.into(),
            action: Action::SubmitTask,
            params: json!({"task_id": "task-1", "agent_id": "agent-1"}),
        })
        .await;
    assert!(!refused.ok);
    assert_eq!(refused.code, AgentCode::GateRefused);
    assert_eq!(refused.next, Some(NextHint::FixAddEvidenceRetrySubmit));
    assert_eq!(refused.data.as_ref().unwrap()["status"], 409);
    assert_eq!(
        refused.data.as_ref().unwrap()["path"],
        "/v1/tasks/task-1/transition"
    );
    assert_eq!(
        refused.data.as_ref().unwrap()["error"]["code"],
        "gate_refused"
    );

    let last_transition = daemon.last_transition.lock().await.clone().unwrap();
    assert_eq!(last_transition["task_id"], "task-1");
    assert_eq!(last_transition["body"]["target"], "submitted");
    assert_eq!(last_transition["body"]["agent_id"], "agent-1");

    let explained = bridge
        .dispatch(ActRequest {
            schema_version: SCHEMA_VERSION.into(),
            action: Action::ExplainLastRefusal,
            params: json!({"task_id": "task-1"}),
        })
        .await;
    assert!(explained.ok);
    assert_eq!(explained.data.as_ref().unwrap()["source"], "daemon_audit");
    assert_eq!(
        explained.data.as_ref().unwrap()["refusal"]["code"],
        "gate_refused"
    );
    assert_eq!(daemon.requests.load(Ordering::SeqCst), 2);
}

fn assert_help_contract() {
    let quickstart = help::response(&HelpRequest {
        topic: HelpTopic::Quickstart,
        action: None,
        verbosity: HelpVerbosity::Short,
    });
    assert_eq!(quickstart["schema_version"], SCHEMA_VERSION);
    assert_eq!(quickstart["tools"]["help"], "convergio.help");
    assert_eq!(quickstart["tools"]["act"], "convergio.act");

    let catalog = help::response(&HelpRequest {
        topic: HelpTopic::Actions,
        action: None,
        verbosity: HelpVerbosity::Schema,
    });
    let actions = catalog["actions"].as_array().unwrap();
    assert!(actions
        .iter()
        .any(|a| a["name"].as_str() == Some("validate_plan")));
    assert!(!actions
        .iter()
        .any(|a| a["name"].as_str() == Some("complete_task")));
}

async fn spawn_stub_daemon() -> (String, Arc<StubDaemon>) {
    let daemon = Arc::new(StubDaemon {
        requests: AtomicUsize::new(0),
        last_transition: Mutex::new(None),
    });
    let app = Router::new()
        .route("/v1/status", get(status))
        .route("/v1/tasks/:id/transition", post(refuse_transition))
        .route("/v1/audit/refusals/latest", get(latest_refusal))
        .with_state(daemon.clone());
    let listener = TcpListener::bind(("127.0.0.1", 0)).await.unwrap();
    let address = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    (format!("http://{address}"), daemon)
}

async fn status(State(daemon): State<Arc<StubDaemon>>) -> Json<Value> {
    daemon.requests.fetch_add(1, Ordering::SeqCst);
    Json(json!({"ok": true}))
}

async fn refuse_transition(
    State(daemon): State<Arc<StubDaemon>>,
    Path(task_id): Path<String>,
    Json(body): Json<Value>,
) -> (StatusCode, Json<Value>) {
    daemon.requests.fetch_add(1, Ordering::SeqCst);
    *daemon.last_transition.lock().await = Some(json!({
        "task_id": task_id,
        "body": body,
    }));
    (
        StatusCode::CONFLICT,
        Json(json!({
            "error": {
                "code": "gate_refused",
                "message": "gate refused by test daemon",
                "data": {"task_id": task_id, "gate": "no_debt"}
            }
        })),
    )
}

async fn latest_refusal(State(daemon): State<Arc<StubDaemon>>) -> Json<Value> {
    daemon.requests.fetch_add(1, Ordering::SeqCst);
    Json(json!({
        "task_id": "task-1",
        "code": "gate_refused",
        "message": "persisted refusal"
    }))
}
