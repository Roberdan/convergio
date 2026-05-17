//! Capability registry API E2E tests.

use convergio_bus::Bus;
use convergio_db::Pool;
use convergio_durability::{init, NewCapability};
use convergio_lifecycle::Supervisor;
use convergio_server::{router, AppState};
use serde_json::{json, Value};
use std::net::SocketAddr;
use std::sync::Arc;
use tempfile::tempdir;
use tokio::net::TcpListener;

#[tokio::test]
async fn capability_registry_lists_seeded_capabilities() {
    let dir = tempdir().unwrap();
    let db_path = dir.path().join("state.db");
    let pool = Pool::connect(&format!("sqlite://{}", db_path.display()))
        .await
        .unwrap();
    init(&pool).await.unwrap();
    convergio_bus::init(&pool).await.unwrap();
    convergio_lifecycle::init(&pool).await.unwrap();
    let dur = convergio_durability::Durability::new(pool.clone());
    dur.register_capability(NewCapability {
        name: "planner".into(),
        version: "0.1.0".into(),
        status: "disabled".into(),
        source: "local".into(),
        root_path: None,
        manifest: json!({"actions": ["planner.solve"]}),
        checksum: Some("sha256:abc".into()),
        signature: Some("sig".into()),
    })
    .await
    .unwrap();

    let state = AppState {
        durability: Arc::new(dur),
        bus: Arc::new(Bus::new(pool.clone())),
        supervisor: Arc::new(Supervisor::new(pool.clone())),
        graph: Arc::new(convergio_graph::Store::new(pool.clone())),
        embed: Arc::new(convergio_embed::EmbedStore::new(pool.clone())),
        embedder: Arc::new(convergio_embed::embedder::testing::DeterministicTestEmbedder::new(8)),
        fleet: Arc::new(convergio_fleet::FleetStore::new(pool.clone())),
        fleet_plans: Arc::new(convergio_fleet::FleetPlanStore::new(pool.clone())),
        audit_verify_cache: Arc::new(std::sync::Mutex::new(None)),
    };
    let listener = TcpListener::bind(SocketAddr::from(([127, 0, 0, 1], 0)))
        .await
        .unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(listener, router(state)).await.unwrap();
    });
    let base = format!("http://{addr}");
    let client = reqwest::Client::new();

    let caps: Value = client
        .get(format!("{base}/v1/capabilities"))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(caps.as_array().unwrap().len(), 1);
    assert_eq!(caps[0]["name"], "planner");

    let cap: Value = client
        .get(format!("{base}/v1/capabilities/planner"))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(cap["status"], "disabled");
}
