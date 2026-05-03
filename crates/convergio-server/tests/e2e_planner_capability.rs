//! E2E proof for the first planner capability action.

use convergio_bus::Bus;
use convergio_db::Pool;
use convergio_durability::{
    capability_signature_payload, init, CapabilitySignatureRequest, Durability,
    TrustedCapabilityKey,
};
use convergio_lifecycle::Supervisor;
use convergio_server::{router, AppState};
use ed25519_dalek::{Signer, SigningKey};
use flate2::write::GzEncoder;
use flate2::Compression;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::io::Write;
use std::net::SocketAddr;
use std::sync::Arc;
use tar::{Builder, Header};
use tempfile::{tempdir, TempDir};
use tokio::net::TcpListener;

async fn boot() -> (String, TempDir) {
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

fn planner_package(dir: &TempDir) -> (String, String, Vec<TrustedCapabilityKey>) {
    let manifest = "name = \"planner\"\nversion = \"0.1.0\"\nactions = [\"planner.solve\"]\n";
    let path = dir.path().join("planner.tar.gz");
    let file = std::fs::File::create(&path).unwrap();
    let encoder = GzEncoder::new(file, Compression::default());
    let mut tar = Builder::new(encoder);
    append(&mut tar, "manifest.toml", manifest.as_bytes());
    append(&mut tar, "bin/convergio-cap-planner", b"#!/bin/sh\n");
    tar.finish().unwrap();
    tar.into_inner().unwrap().finish().unwrap();

    let bytes = std::fs::read(&path).unwrap();
    let manifest_toml: toml::Value = toml::from_str(manifest).unwrap();
    let signing_key = SigningKey::from_bytes(&[11; 32]);
    let mut request = CapabilitySignatureRequest {
        name: "planner".into(),
        version: "0.1.0".into(),
        checksum: format!("sha256:{}", hex::encode(Sha256::digest(bytes))),
        manifest: serde_json::to_value(manifest_toml).unwrap(),
        signature: String::new(),
        trusted_keys: vec![TrustedCapabilityKey {
            key_id: "test-root".into(),
            public_key: hex::encode(signing_key.verifying_key().to_bytes()),
        }],
    };
    let payload = capability_signature_payload(&request).unwrap();
    request.signature = hex::encode(signing_key.sign(&payload).to_bytes());
    (
        path.display().to_string(),
        request.signature,
        request.trusted_keys,
    )
}

fn append<W: Write>(tar: &mut Builder<W>, path: &str, bytes: &[u8]) {
    let mut header = Header::new_gnu();
    header.set_size(bytes.len() as u64);
    header.set_mode(0o644);
    header.set_cksum();
    tar.append_data(&mut header, path, bytes).unwrap();
}

#[tokio::test]
async fn installed_planner_capability_can_solve_a_plan() {
    let (base, _server_dir) = boot().await;
    let home = tempdir().unwrap();
    std::env::set_var("HOME", home.path());
    let package_dir = tempdir().unwrap();
    let (package_path, signature, trusted_keys) = planner_package(&package_dir);
    let client = reqwest::Client::new();

    let missing = client
        .post(format!("{base}/v1/capabilities/planner/solve"))
        .json(&json!({"mission": "should require install"}))
        .send()
        .await
        .unwrap();
    assert_eq!(missing.status(), 404);

    let installed: Value = client
        .post(format!("{base}/v1/capabilities/install-file"))
        .json(&json!({
            "package_path": package_path,
            "signature": signature,
            "trusted_keys": trusted_keys,
        }))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(installed["name"], "planner");
    assert_eq!(installed["manifest"]["actions"][0], "planner.solve");

    let solved: Value = client
        .post(format!("{base}/v1/capabilities/planner/solve"))
        .json(&json!({"mission": "ship one planner capability proof"}))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert!(solved["plan_id"].as_str().is_some());
}
