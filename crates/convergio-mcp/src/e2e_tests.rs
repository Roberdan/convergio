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

#[tokio::test]
async fn fleet_plan_actions_round_trip_to_expected_paths() {
    // F3-7 acceptance: each of the three fleet actions must reach the
    // matching daemon HTTP path with the right verb and payload.
    use std::sync::Mutex as StdMutex;
    #[derive(Default)]
    struct Calls {
        log: StdMutex<Vec<(String, String, Value)>>,
    }
    let calls = Arc::new(Calls::default());
    let state = calls.clone();
    let app = Router::new()
        .route(
            "/v1/fleet/plans",
            post(
                |State(s): State<Arc<Calls>>, Json(body): Json<Value>| async move {
                    s.log
                        .lock()
                        .unwrap()
                        .push(("POST".into(), "/v1/fleet/plans".into(), body));
                    Json(json!({"id": "fp-1", "title": "x", "scope": "fleet"}))
                },
            ),
        )
        .route(
            "/v1/fleet/plans/:id",
            get(
                |State(s): State<Arc<Calls>>, Path(id): Path<String>| async move {
                    s.log.lock().unwrap().push((
                        "GET".into(),
                        format!("/v1/fleet/plans/{id}"),
                        json!({}),
                    ));
                    Json(json!({"plan": {"id": id}, "links": []}))
                },
            ),
        )
        .route(
            "/v1/fleet/plans/:id/validate",
            post(
                |State(s): State<Arc<Calls>>,
                 Path(id): Path<String>,
                 axum::extract::RawQuery(q),
                 Json(body): Json<Value>| async move {
                    s.log.lock().unwrap().push((
                        "POST".into(),
                        format!("/v1/fleet/plans/{id}/validate?{}", q.unwrap_or_default()),
                        body,
                    ));
                    Json(json!({"fleet_plan_id": id, "passing": true, "verdicts": []}))
                },
            ),
        )
        .with_state(state);
    let listener = TcpListener::bind(("127.0.0.1", 0)).await.unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
    let bridge = Bridge::new(url);

    let create = bridge
        .dispatch(ActRequest {
            schema_version: SCHEMA_VERSION.into(),
            action: Action::FleetPlanCreate,
            params: json!({"title": "fleet x", "scope": "fleet"}),
        })
        .await;
    assert!(create.ok, "create: {create:?}");

    let show = bridge
        .dispatch(ActRequest {
            schema_version: SCHEMA_VERSION.into(),
            action: Action::FleetPlanShow,
            params: json!({"fleet_plan_id": "fp-1"}),
        })
        .await;
    assert!(show.ok, "show: {show:?}");

    let validate = bridge
        .dispatch(ActRequest {
            schema_version: SCHEMA_VERSION.into(),
            action: Action::FleetPlanValidate,
            params: json!({"fleet_plan_id": "fp-1", "per_repo_timeout_secs": 30}),
        })
        .await;
    assert!(validate.ok, "validate: {validate:?}");

    let log = calls.log.lock().unwrap().clone();
    assert_eq!(log.len(), 3, "{log:?}");
    assert_eq!(log[0].0, "POST");
    assert_eq!(log[0].1, "/v1/fleet/plans");
    assert_eq!(log[0].2["title"], "fleet x");
    assert_eq!(log[1].0, "GET");
    assert_eq!(log[1].1, "/v1/fleet/plans/fp-1");
    assert_eq!(log[2].0, "POST");
    assert_eq!(
        log[2].1,
        "/v1/fleet/plans/fp-1/validate?per_repo_timeout_secs=30"
    );

    let bad = bridge
        .dispatch(ActRequest {
            schema_version: SCHEMA_VERSION.into(),
            action: Action::FleetPlanShow,
            params: json!({}),
        })
        .await;
    assert!(!bad.ok, "missing fleet_plan_id must be a typed error");
}
