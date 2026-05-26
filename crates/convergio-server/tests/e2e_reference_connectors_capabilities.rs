//! E2E test: reference SIS + Canvas connector capability payloads are installable.
//!
//! The Connector SDK runtime is not shipped yet (ADR-0057 is proposed), but
//! these reference payloads must remain **valid signed capability packages**
//! and must carry an ontology mapping with explicit per-field lawful basis +
//! DPA references.

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
use reqwest::header::{HeaderMap, HeaderValue};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::io::Write;
use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tar::{Builder, Header};
use tempfile::{tempdir, TempDir};
use tokio::net::TcpListener;

const TEST_PURPOSE_ID: &str = "00000000-0000-4000-8000-000000000445";

async fn boot() -> (String, TempDir) {
    let dir = tempdir().unwrap();
    let db_path = dir.path().join("state.db");
    let pool = Pool::connect(&format!("sqlite://{}", db_path.display()))
        .await
        .unwrap();
    init(&pool).await.unwrap();
    convergio_bus::init(&pool).await.unwrap();
    convergio_lifecycle::init(&pool).await.unwrap();
    convergio_ops::init(&pool).await.unwrap();
    let ontology = Arc::new(convergio_ontology::Store::new(pool.clone()));
    ontology.migrate().await.unwrap();
    convergio_reports::init(&pool).await.unwrap();

    let state = AppState {
        durability: Arc::new(Durability::new(pool.clone())),
        bus: Arc::new(Bus::new(pool.clone())),
        supervisor: Arc::new(Supervisor::new(pool.clone())),
        graph: Arc::new(convergio_graph::Store::new(pool.clone())),
        embed: Arc::new(convergio_embed::EmbedStore::new(pool.clone())),
        embedder: Arc::new(convergio_embed::embedder::testing::DeterministicTestEmbedder::new(8)),
        fleet: Arc::new(convergio_fleet::FleetStore::new(pool.clone())),
        fleet_plans: Arc::new(convergio_fleet::FleetPlanStore::new(pool.clone())),
        ops: Arc::new(convergio_ops::Ops::new(pool.clone())),
        ontology,
        reports: Arc::new(convergio_reports::ReportTemplateStore::new(pool.clone())),
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

#[derive(Debug, Clone)]
struct ReferenceConnector {
    dir_name: &'static str,
    expected_mapping_fields: usize,
}

fn repo_root() -> PathBuf {
    let crate_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    crate_dir
        .parent()
        .and_then(|p| p.parent())
        .map(Path::to_path_buf)
        .expect("repo root")
}

fn read_example(dir: &Path, file: &str) -> Vec<u8> {
    std::fs::read(dir.join(file)).unwrap_or_else(|_| panic!("missing {file}"))
}

fn assert_mapping_has_per_field_controls(source: &str, expected_fields: usize) {
    let field_count = source.matches("\n      - source:").count();
    let lawful_basis_count = source.matches("\n        lawful_basis:").count();
    let dpa_ref_count = source.matches("\n        dpa_reference:").count();

    assert_eq!(
        field_count, expected_fields,
        "unexpected field count; got={field_count} expected={expected_fields}"
    );
    assert_eq!(
        lawful_basis_count, expected_fields,
        "lawful_basis must be declared per field"
    );
    assert_eq!(
        dpa_ref_count, expected_fields,
        "dpa_reference must be declared per field"
    );
}

fn append<W: Write>(tar: &mut Builder<W>, path: &str, bytes: &[u8]) {
    let mut header = Header::new_gnu();
    header.set_size(bytes.len() as u64);
    header.set_mode(0o644);
    header.set_cksum();
    tar.append_data(&mut header, path, bytes).unwrap();
}

fn build_package(dir: &Path, out: &Path) {
    let file = std::fs::File::create(out).unwrap();
    let encoder = GzEncoder::new(file, Compression::default());
    let mut tar = Builder::new(encoder);

    for entry in [
        "manifest.toml",
        "connector.yaml",
        "ontology-mapping.yaml",
        "dpa-reference.md",
    ] {
        let bytes = read_example(dir, entry);
        append(&mut tar, entry, &bytes);
    }

    tar.finish().unwrap();
    tar.into_inner().unwrap().finish().unwrap();
}

fn sign(
    path: &Path,
    manifest_source: &str,
    signing_key: &SigningKey,
) -> (String, Vec<TrustedCapabilityKey>) {
    let bytes = std::fs::read(path).unwrap();
    let manifest_toml: toml::Value = toml::from_str(manifest_source).unwrap();
    let manifest_json = serde_json::to_value(&manifest_toml).unwrap();

    let mut request = CapabilitySignatureRequest {
        name: manifest_toml
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap()
            .to_string(),
        version: manifest_toml
            .get("version")
            .and_then(|v| v.as_str())
            .unwrap()
            .to_string(),
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

    (signature, request.trusted_keys)
}

#[tokio::test]
async fn reference_sis_and_canvas_connectors_are_signed_installable_capabilities() {
    let (base, _server_dir) = boot().await;

    let home = tempdir().unwrap();
    std::env::set_var("HOME", home.path());

    let root = repo_root().join("examples/ontology-platform/connectors");
    let connectors = [
        ReferenceConnector {
            dir_name: "connector-sis-ethos",
            expected_mapping_fields: 6,
        },
        ReferenceConnector {
            dir_name: "connector-canvas-rest-lti13",
            expected_mapping_fields: 10,
        },
    ];

    let signing_key = SigningKey::from_bytes(&[9; 32]);
    let mut headers = HeaderMap::new();
    headers.insert(
        convergio_api::PURPOSE_ID_HEADER,
        HeaderValue::from_static(TEST_PURPOSE_ID),
    );
    let client = reqwest::Client::builder()
        .default_headers(headers)
        .build()
        .unwrap();

    for c in connectors {
        let dir = root.join(c.dir_name);
        let manifest_source = String::from_utf8(read_example(&dir, "manifest.toml")).unwrap();
        let mapping_source =
            String::from_utf8(read_example(&dir, "ontology-mapping.yaml")).unwrap();

        assert_mapping_has_per_field_controls(&mapping_source, c.expected_mapping_fields);

        let package_dir = tempdir().unwrap();
        let package_path = package_dir.path().join(format!("{}.tar.gz", c.dir_name));
        build_package(&dir, &package_path);

        let (signature, trusted_keys) = sign(&package_path, &manifest_source, &signing_key);

        let response = client
            .post(format!("{base}/v1/capabilities/install-file"))
            .json(&json!({
                "package_path": package_path.display().to_string(),
                "signature": signature,
                "trusted_keys": trusted_keys,
            }))
            .send()
            .await
            .unwrap();

        let status = response.status();
        let installed: Value = response.json().await.unwrap();
        assert_eq!(status, 200, "{installed}");

        let name = installed["name"].as_str().unwrap();
        let cap_root = home.path().join(format!(".convergio/capabilities/{name}"));
        assert!(cap_root.join("manifest.toml").is_file());
        assert!(cap_root.join("connector.yaml").is_file());
        assert!(cap_root.join("ontology-mapping.yaml").is_file());
        assert!(cap_root.join("dpa-reference.md").is_file());

        let _disabled: Value = client
            .post(format!("{base}/v1/capabilities/{name}/disable"))
            .json(&json!({}))
            .send()
            .await
            .unwrap()
            .json()
            .await
            .unwrap();

        let removed: Value = client
            .delete(format!("{base}/v1/capabilities/{name}"))
            .send()
            .await
            .unwrap()
            .json()
            .await
            .unwrap();
        assert_eq!(removed["removed"], name);
        assert!(!cap_root.exists());
    }
}
