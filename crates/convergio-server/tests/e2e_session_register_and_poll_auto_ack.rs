//! End-to-end coverage for the P1-3 auto-ack default of
//! `cvg session register-and-poll`.
//!
//! Boots the real daemon in-process, publishes two unicast messages
//! to `agent:auto-ack-test`, drives `register_and_poll::run`, and
//! asserts each message's `consumed_at` is non-null in
//! `GET /messages/tail` afterwards. Mirrors the shape of
//! `e2e_session_register_and_poll.rs`.

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
    let dir = tempdir().unwrap();
    let db_path = dir.path().join("state.db");
    let pool = Pool::connect(&format!("sqlite://{}", db_path.display()))
        .await
        .unwrap();
    init(&pool).await.unwrap();
    convergio_ops::init(&pool).await.unwrap();
    convergio_bus::init(&pool).await.unwrap();
    convergio_lifecycle::init(&pool).await.unwrap();
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
        .unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(listener, router(state)).await.unwrap();
    });
    (format!("http://{addr}"), dir)
}

#[tokio::test]
async fn unicast_messages_are_consumed_after_register_and_poll() {
    let (base, _dir) = boot().await;
    let http = reqwest::Client::new();

    // 1. Create a plan so unicast topics resolve.
    let plan: Value = http
        .post(format!("{base}/v1/plans"))
        .json(&json!({"title": "auto-ack plan"}))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let plan_id = plan["id"].as_str().unwrap();

    // 2. Publish two unicast messages to the future agent.
    let agent_id = "auto-ack-test";
    for n in 0..2 {
        http.post(format!("{base}/v1/plans/{plan_id}/messages"))
            .json(&json!({
                "topic": format!("agent:{agent_id}"),
                "sender": "peer",
                "payload": {"n": n},
            }))
            .send()
            .await
            .unwrap()
            .error_for_status()
            .unwrap();
    }

    // 3. Drive the real CLI handler in default mode.
    let client = Client::new(base.clone());
    let bundle = Bundle::new(Locale::En).unwrap();
    run(
        &client,
        &bundle,
        OutputMode::Plain,
        Args {
            agent_id: Some(agent_id.into()),
            capabilities: vec![],
            kind: "claude".into(),
            host: Some("test-host".into()),
            quiet: true,
            no_auto_ack: false,
        },
    )
    .await
    .unwrap();

    // 4. Inspect the tail (which surfaces every row regardless of
    //    consumed status) and assert both unicast rows are now acked.
    let tail: Value = http
        .get(format!(
            "{base}/v1/plans/{plan_id}/messages/tail?topic=agent:{agent_id}"
        ))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let arr = tail.as_array().expect("tail is array");
    assert_eq!(arr.len(), 2, "two unicast messages were published");
    for m in arr {
        assert!(
            m.get("consumed_at").map(|v| !v.is_null()).unwrap_or(false),
            "consumed_at must be non-null after auto-ack: {m}"
        );
        assert_eq!(
            m.get("consumed_by").and_then(Value::as_str),
            Some(agent_id),
            "consumed_by must be the polling agent id"
        );
    }

    // 5. A second poll returns nothing — the inbox is now empty.
    let second: Value = http
        .get(format!(
            "{base}/v1/plans/{plan_id}/messages?topic=agent:{agent_id}&limit=20"
        ))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert!(
        second.as_array().unwrap().is_empty(),
        "consumed messages must not re-surface"
    );
}

#[tokio::test]
async fn no_auto_ack_flag_leaves_consumed_at_null() {
    let (base, _dir) = boot().await;
    let http = reqwest::Client::new();
    let plan: Value = http
        .post(format!("{base}/v1/plans"))
        .json(&json!({"title": "no-ack plan"}))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let plan_id = plan["id"].as_str().unwrap();
    let agent_id = "no-ack-test";
    http.post(format!("{base}/v1/plans/{plan_id}/messages"))
        .json(&json!({
            "topic": format!("agent:{agent_id}"),
            "sender": "peer",
            "payload": {"n": 0},
        }))
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap();

    let client = Client::new(base.clone());
    let bundle = Bundle::new(Locale::En).unwrap();
    run(
        &client,
        &bundle,
        OutputMode::Plain,
        Args {
            agent_id: Some(agent_id.into()),
            capabilities: vec![],
            kind: "claude".into(),
            host: Some("test-host".into()),
            quiet: true,
            no_auto_ack: true,
        },
    )
    .await
    .unwrap();

    let tail: Value = http
        .get(format!(
            "{base}/v1/plans/{plan_id}/messages/tail?topic=agent:{agent_id}"
        ))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let arr = tail.as_array().unwrap();
    assert_eq!(arr.len(), 1);
    assert!(
        arr[0]
            .get("consumed_at")
            .map(Value::is_null)
            .unwrap_or(true),
        "--no-auto-ack must leave consumed_at null"
    );
}
