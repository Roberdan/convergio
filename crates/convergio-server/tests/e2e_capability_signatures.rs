//! HTTP E2E tests for capability signature verification.

use convergio_bus::Bus;
use convergio_db::Pool;
use convergio_durability::{
    capability_signature_payload, init, CapabilitySignatureRequest, Durability,
    TrustedCapabilityKey,
};
use convergio_lifecycle::Supervisor;
use convergio_server::{router, AppState};
use ed25519_dalek::{Signer, SigningKey};
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
    convergio_bus::init(&pool).await.unwrap();
    convergio_lifecycle::init(&pool).await.unwrap();
    let state = AppState {
        durability: Arc::new(Durability::new(pool.clone())),
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
    (format!("http://{addr}"), dir)
}

fn signed_request() -> CapabilitySignatureRequest {
    let signing_key = SigningKey::from_bytes(&[7; 32]);
    let mut request = CapabilitySignatureRequest {
        name: "planner".into(),
        version: "0.1.0".into(),
        checksum: format!("sha256:{}", "a".repeat(64)),
        manifest: json!({"actions": ["planner.solve"]}),
        signature: String::new(),
        trusted_keys: vec![TrustedCapabilityKey {
            key_id: "test-root".into(),
            public_key: hex::encode(signing_key.verifying_key().to_bytes()),
        }],
    };
    let payload = capability_signature_payload(&request).unwrap();
    request.signature = hex::encode(signing_key.sign(&payload).to_bytes());
    request
}

#[tokio::test]
async fn verify_signature_route_accepts_good_signature_and_refuses_bad_one() {
    let (base, _dir) = boot().await;
    let client = reqwest::Client::new();

    let verified: Value = client
        .post(format!("{base}/v1/capabilities/verify-signature"))
        .json(&signed_request())
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(verified["key_id"], "test-root");

    let mut bad = signed_request();
    let mut signature = hex::decode(&bad.signature).unwrap();
    signature[0] ^= 0xff;
    bad.signature = hex::encode(signature);
    let refused = client
        .post(format!("{base}/v1/capabilities/verify-signature"))
        .json(&bad)
        .send()
        .await
        .unwrap();
    assert_eq!(refused.status(), 422);
    let body: Value = refused.json().await.unwrap();
    assert_eq!(body["error"]["code"], "invalid_capability");

    let audit: Value = client
        .get(format!("{base}/v1/audit/verify"))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(audit["ok"], true);
}
