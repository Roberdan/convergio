//! HTTP E2E tests for signed local capability install.
mod common;

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

fn package(dir: &TempDir) -> (String, String, Vec<TrustedCapabilityKey>) {
    let manifest = format!(
        "name = \"planner\"\nversion = \"0.1.0\"\nplatforms = [\"{}\"]\n",
        current_platform()
    );
    let path = dir.path().join("planner.tar.gz");
    let file = std::fs::File::create(&path).unwrap();
    let encoder = GzEncoder::new(file, Compression::default());
    let mut tar = Builder::new(encoder);
    append(&mut tar, "manifest.toml", manifest.as_bytes());
    append(&mut tar, "bin/planner", b"#!/bin/sh\n");
    tar.finish().unwrap();
    tar.into_inner().unwrap().finish().unwrap();

    let bytes = std::fs::read(&path).unwrap();
    let manifest_toml: toml::Value = toml::from_str(&manifest).unwrap();
    let manifest_json = serde_json::to_value(manifest_toml).unwrap();
    let signing_key = SigningKey::from_bytes(&[9; 32]);
    let mut request = CapabilitySignatureRequest {
        name: "planner".into(),
        version: "0.1.0".into(),
        checksum: format!("sha256:{}", hex::encode(Sha256::digest(bytes))),
        manifest: manifest_json,
        signature: String::new(),
        trusted_keys: vec![TrustedCapabilityKey {
            key_id: "test-root".into(),
            public_key: hex::encode(signing_key.verifying_key().to_bytes()),
        }],
    };
    let payload = capability_signature_payload(&request).unwrap();
    let signature = hex::encode(signing_key.sign(&payload).to_bytes());
    request.signature = signature.clone();
    (path.display().to_string(), signature, request.trusted_keys)
}

fn append<W: Write>(tar: &mut Builder<W>, path: &str, bytes: &[u8]) {
    let mut header = Header::new_gnu();
    header.set_size(bytes.len() as u64);
    header.set_mode(0o644);
    header.set_cksum();
    tar.append_data(&mut header, path, bytes).unwrap();
}

fn current_platform() -> String {
    let os = if std::env::consts::OS == "macos" {
        "darwin"
    } else {
        std::env::consts::OS
    };
    let arch = if std::env::consts::ARCH == "aarch64" {
        "arm64"
    } else {
        std::env::consts::ARCH
    };
    format!("{os}-{arch}")
}

#[tokio::test]
async fn install_file_requires_good_signature_and_installs_atomically() {
    let (base, _server_dir) = boot().await;
    let home = tempdir().unwrap();
    std::env::set_var("HOME", home.path());
    let package_dir = tempdir().unwrap();
    let (package_path, signature, trusted_keys) = package(&package_dir);
    let client = common::client();

    let mut bad_signature = hex::decode(&signature).unwrap();
    bad_signature[0] ^= 0xff;
    let refused = client
        .post(format!("{base}/v1/capabilities/install-file"))
        .json(&json!({
            "package_path": package_path,
            "signature": hex::encode(bad_signature),
            "trusted_keys": trusted_keys.clone(),
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(refused.status(), 422);

    let response = client
        .post(format!("{base}/v1/capabilities/install-file"))
        .json(&json!({
            "package_path": package_path,
            "signature": signature,
            "trusted_keys": trusted_keys,
        }))
        .send()
        .await
        .unwrap();
    let status = response.status();
    let installed: Value = response.json().await.unwrap();
    assert_eq!(status, 200, "{installed}");
    assert_eq!(installed["name"], "planner");
    assert_eq!(installed["status"], "installed");
    let root = home.path().join(".convergio/capabilities/planner");
    assert!(root.join("manifest.toml").is_file());

    let disabled: Value = client
        .post(format!("{base}/v1/capabilities/planner/disable"))
        .json(&json!({}))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(disabled["status"], "disabled");

    let removed: Value = client
        .delete(format!("{base}/v1/capabilities/planner"))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(removed["removed"], "planner");
    assert!(!root.exists());
}
