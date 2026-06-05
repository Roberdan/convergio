//! E2E smoke test: two concurrent-ish sessions must not cross-talk.
//!
//! Regression target: a second session should never consume unicast
//! `agent:<other>` messages when running `cvg session register-and-poll`.
//!
//! Boots the real daemon in-process, publishes one unicast message for
//! each agent, then runs the real CLI handler for B first and asserts
//! A's inbox is untouched.
mod common;

use convergio_bus::Bus;
use convergio_cli_session::register_and_poll::{run, Args};
use convergio_cli_session::{Client, OutputMode};
use convergio_db::Pool;
use convergio_durability::{init, Durability};
use convergio_i18n::{Bundle, Locale};
use convergio_lifecycle::Supervisor;
use convergio_server::{router, AppState};
use serde_json::{json, Value};
use std::net::SocketAddr;
use std::sync::Arc;
use tempfile::tempdir;
use tokio::net::TcpListener;

async fn boot() -> (String, tempfile::TempDir) {
    let dir = tempdir().expect("tempdir");
    let db_path = dir.path().join("state.db");
    let pool = Pool::connect(&format!("sqlite://{}", db_path.display()))
        .await
        .expect("connect pool");
    init(&pool).await.expect("init durability");
    convergio_ops::init(&pool).await.expect("ops init");
    convergio_bus::init(&pool).await.expect("init bus");
    convergio_lifecycle::init(&pool)
        .await
        .expect("init lifecycle");
    let state = AppState {
        durability: Arc::new(Durability::new(pool.clone())),
        ops: Arc::new(convergio_ops::Ops::new(pool.clone())),
        bus: Arc::new(Bus::new(pool.clone())),
        supervisor: Arc::new(Supervisor::new(pool.clone())),
        graph: Arc::new(convergio_graph::Store::new(pool.clone())),
        embed: Arc::new(convergio_embed::EmbedStore::new(pool.clone())),
        embedder: Arc::new(convergio_embed::embedder::testing::DeterministicTestEmbedder::new(8)),
        fleet: Arc::new(convergio_fleet::FleetStore::new(pool.clone())),
        fleet_plans: Arc::new(convergio_fleet::FleetPlanStore::new(pool.clone())),
        ontology: Arc::new(convergio_ontology::Store::new(pool.clone())),
        audit_verify_cache: Arc::new(std::sync::Mutex::new(None)),
    };
    let listener = TcpListener::bind(SocketAddr::from(([127, 0, 0, 1], 0)))
        .await
        .expect("bind");
    let addr = listener.local_addr().expect("local_addr");
    tokio::spawn(async move {
        axum::serve(listener, router(state)).await.expect("serve");
    });
    (format!("http://{addr}"), dir)
}

#[tokio::test]
async fn register_and_poll_two_sessions_do_not_cross_talk() {
    let (base, _dir) = boot().await;
    let http = common::client();

    // Create a plan so active_plans() returns something to poll.
    let plan: Value = http
        .post(format!("{base}/v1/plans"))
        .json(&json!({"title": "two-session plan"}))
        .send()
        .await
        .expect("plan create")
        .json()
        .await
        .expect("plan json");
    let plan_id = plan["id"].as_str().expect("plan id");

    // Publish one unicast message to each agent inbox.
    let agent_a = "cross-talk-A";
    let agent_b = "cross-talk-B";
    http.post(format!("{base}/v1/plans/{plan_id}/messages"))
        .json(&json!({
            "topic": format!("agent:{agent_a}"),
            "sender": "peer",
            "payload": {"to": agent_a},
        }))
        .send()
        .await
        .expect("publish A")
        .error_for_status()
        .expect("publish A status");
    http.post(format!("{base}/v1/plans/{plan_id}/messages"))
        .json(&json!({
            "topic": format!("agent:{agent_b}"),
            "sender": "peer",
            "payload": {"to": agent_b},
        }))
        .send()
        .await
        .expect("publish B")
        .error_for_status()
        .expect("publish B status");

    let bundle = Bundle::new(Locale::En).expect("bundle");

    // Run B first. It must NOT consume A's inbox.
    run(
        &Client::new(base.clone()),
        &bundle,
        OutputMode::Plain,
        Args {
            agent_id: Some(agent_b.into()),
            capabilities: vec![],
            kind: "claude".into(),
            host: Some("test-host".into()),
            quiet: true,
            no_auto_ack: false,
        },
    )
    .await
    .expect("register-and-poll B");

    let tail_a: Value = http
        .get(format!(
            "{base}/v1/plans/{plan_id}/messages/tail?topic=agent:{agent_a}&limit=20"
        ))
        .send()
        .await
        .expect("tail A")
        .json()
        .await
        .expect("tail A json");
    let arr_a = tail_a.as_array().expect("tail A array");
    assert_eq!(arr_a.len(), 1, "one message for A exists");
    assert!(
        arr_a[0]
            .get("consumed_at")
            .map(Value::is_null)
            .unwrap_or(true),
        "B must not consume A inbox"
    );

    let tail_b: Value = http
        .get(format!(
            "{base}/v1/plans/{plan_id}/messages/tail?topic=agent:{agent_b}&limit=20"
        ))
        .send()
        .await
        .expect("tail B")
        .json()
        .await
        .expect("tail B json");
    let arr_b = tail_b.as_array().expect("tail B array");
    assert_eq!(arr_b.len(), 1, "one message for B exists");
    assert_eq!(
        arr_b[0].get("consumed_by").and_then(Value::as_str),
        Some(agent_b),
        "B must consume its own inbox"
    );
    assert!(
        arr_b[0]
            .get("consumed_at")
            .map(|v| !v.is_null())
            .unwrap_or(false),
        "B consumed_at non-null"
    );

    // Now run A; its inbox should be consumed.
    run(
        &Client::new(base.clone()),
        &bundle,
        OutputMode::Plain,
        Args {
            agent_id: Some(agent_a.into()),
            capabilities: vec![],
            kind: "claude".into(),
            host: Some("test-host".into()),
            quiet: true,
            no_auto_ack: false,
        },
    )
    .await
    .expect("register-and-poll A");

    let tail_a2: Value = http
        .get(format!(
            "{base}/v1/plans/{plan_id}/messages/tail?topic=agent:{agent_a}&limit=20"
        ))
        .send()
        .await
        .expect("tail A2")
        .json()
        .await
        .expect("tail A2 json");
    let arr_a2 = tail_a2.as_array().expect("tail A2 array");
    assert_eq!(arr_a2.len(), 1);
    assert_eq!(
        arr_a2[0].get("consumed_by").and_then(Value::as_str),
        Some(agent_a),
        "A must consume its own inbox"
    );
    assert!(
        arr_a2[0]
            .get("consumed_at")
            .map(|v| !v.is_null())
            .unwrap_or(false),
        "A consumed_at non-null"
    );

    // Inbox polls should now be empty for both.
    let poll_a: Value = http
        .get(format!(
            "{base}/v1/plans/{plan_id}/messages?topic=agent:{agent_a}&limit=20"
        ))
        .send()
        .await
        .expect("poll A")
        .json()
        .await
        .expect("poll A json");
    assert!(poll_a.as_array().expect("poll A array").is_empty());

    let poll_b: Value = http
        .get(format!(
            "{base}/v1/plans/{plan_id}/messages?topic=agent:{agent_b}&limit=20"
        ))
        .send()
        .await
        .expect("poll B")
        .json()
        .await
        .expect("poll B json");
    assert!(poll_b.as_array().expect("poll B array").is_empty());

    // Audit chain remains intact.
    let audit: Value = http
        .get(format!("{base}/v1/audit/verify"))
        .send()
        .await
        .expect("audit")
        .json()
        .await
        .expect("audit json");
    assert_eq!(audit["ok"], true);
}
