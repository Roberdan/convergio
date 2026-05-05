//! Integration coverage for the P1-3 auto-ack behaviour of
//! `cvg session register-and-poll`.
//!
//! Boots a tiny axum-based fake daemon that records the `ack` calls,
//! drives `register_and_poll::run`, and asserts:
//!   - default mode: every unicast message gets `POST /v1/messages/:id/ack`
//!     once, broadcast (`plan:*`) messages do not.
//!   - `--no-auto-ack`: zero ack calls regardless of how many unicast
//!     messages were returned.

use axum::extract::{Path, Query, State};
use axum::response::Json;
use axum::routing::{get, post};
use axum::Router;
use convergio_cli_session::register_and_poll::{run, Args};
use convergio_cli_session::{Client, OutputMode};
use convergio_i18n::{Bundle, Locale};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use tokio::net::TcpListener;

/// Shared fixture state — every fake handler appends its observed
/// path so tests can assert the full call sequence after `run`.
#[derive(Default)]
struct Fixture {
    calls: Vec<String>,
    direct_messages: Vec<Value>,
    plan_messages: Vec<Value>,
}

type SharedFixture = Arc<Mutex<Fixture>>;

async fn boot(fixture: SharedFixture) -> String {
    let app = Router::new()
        .route("/v1/agent-registry/agents", post(register_handler))
        .route(
            "/v1/agent-registry/agents/:id/heartbeat",
            post(heartbeat_handler),
        )
        .route("/v1/plans", get(plans_handler))
        .route("/v1/plans/:plan_id/messages", get(messages_handler))
        .route("/v1/plans/:plan_id/messages", post(publish_handler))
        .route("/v1/messages/:id/ack", post(ack_handler))
        .with_state(fixture);

    let listener = TcpListener::bind(SocketAddr::from(([127, 0, 0, 1], 0)))
        .await
        .unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    format!("http://{addr}")
}

async fn register_handler(State(fx): State<SharedFixture>, Json(body): Json<Value>) -> Json<Value> {
    fx.lock().unwrap().calls.push("register".into());
    let id = body.get("id").and_then(Value::as_str).unwrap_or("?");
    Json(json!({"id": id, "kind": "claude", "host": "x"}))
}

async fn heartbeat_handler(
    State(fx): State<SharedFixture>,
    Path(_id): Path<String>,
    Json(_body): Json<Value>,
) -> Json<Value> {
    fx.lock().unwrap().calls.push("heartbeat".into());
    Json(json!({"status": "idle"}))
}

async fn plans_handler(State(_fx): State<SharedFixture>) -> Json<Value> {
    Json(json!([{"id": "plan-1", "title": "p", "status": "active"}]))
}

async fn messages_handler(
    State(fx): State<SharedFixture>,
    Path(_plan_id): Path<String>,
    Query(q): Query<HashMap<String, String>>,
) -> Json<Value> {
    let topic = q.get("topic").cloned().unwrap_or_default();
    let g = fx.lock().unwrap();
    if topic.starts_with("agent:") {
        Json(Value::Array(g.direct_messages.clone()))
    } else if topic.starts_with("plan:") {
        Json(Value::Array(g.plan_messages.clone()))
    } else {
        Json(Value::Array(vec![]))
    }
}

async fn publish_handler(
    State(fx): State<SharedFixture>,
    Path(_plan_id): Path<String>,
    Json(_body): Json<Value>,
) -> Json<Value> {
    fx.lock().unwrap().calls.push("publish".into());
    Json(json!({"id": "x", "seq": 1}))
}

async fn ack_handler(
    State(fx): State<SharedFixture>,
    Path(id): Path<String>,
    Json(_body): Json<Value>,
) -> Json<Value> {
    fx.lock().unwrap().calls.push(format!("ack:{id}"));
    Json(json!({"ok": true}))
}

fn make_args(no_auto_ack: bool) -> Args {
    Args {
        agent_id: Some("agent-x".into()),
        capabilities: vec![],
        kind: "claude".into(),
        host: Some("test-host".into()),
        quiet: true,
        no_auto_ack,
    }
}

fn unicast(id: &str, seq: i64) -> Value {
    json!({
        "id": id,
        "seq": seq,
        "plan_id": "plan-1",
        "topic": "agent:agent-x",
        "sender": "peer",
        "payload": {"hello": "world"},
        "consumed_at": null,
        "consumed_by": null,
        "created_at": "2026-05-04T00:00:00Z",
    })
}

fn announcement(id: &str, seq: i64) -> Value {
    json!({
        "id": id,
        "seq": seq,
        "plan_id": "plan-1",
        "topic": "plan:plan-1",
        "sender": "peer",
        "payload": {},
        "consumed_at": null,
        "consumed_by": null,
        "created_at": "2026-05-04T00:00:00Z",
    })
}

#[tokio::test]
async fn auto_ack_default_acks_each_unicast_message_once() {
    let fixture: SharedFixture = Arc::new(Mutex::new(Fixture {
        direct_messages: vec![unicast("msg-1", 35), unicast("msg-2", 36)],
        plan_messages: vec![announcement("ann-1", 37)],
        calls: vec![],
    }));
    let base = boot(fixture.clone()).await;
    let client = Client::new(base);
    let bundle = Bundle::new(Locale::En).unwrap();

    run(&client, &bundle, OutputMode::Plain, make_args(false))
        .await
        .unwrap();

    let calls = fixture.lock().unwrap().calls.clone();
    let acks: Vec<&String> = calls.iter().filter(|c| c.starts_with("ack:")).collect();
    assert_eq!(
        acks,
        vec!["ack:msg-1", "ack:msg-2"],
        "exactly two unicast acks; broadcasts are skipped"
    );
}

#[tokio::test]
async fn no_auto_ack_flag_skips_all_acks() {
    let fixture: SharedFixture = Arc::new(Mutex::new(Fixture {
        direct_messages: vec![unicast("msg-9", 99), unicast("msg-10", 100)],
        plan_messages: vec![],
        calls: vec![],
    }));
    let base = boot(fixture.clone()).await;
    let client = Client::new(base);
    let bundle = Bundle::new(Locale::En).unwrap();

    run(&client, &bundle, OutputMode::Plain, make_args(true))
        .await
        .unwrap();

    let calls = fixture.lock().unwrap().calls.clone();
    let acks: Vec<&String> = calls.iter().filter(|c| c.starts_with("ack:")).collect();
    assert!(
        acks.is_empty(),
        "--no-auto-ack must skip every ack ({:?})",
        acks
    );
}
