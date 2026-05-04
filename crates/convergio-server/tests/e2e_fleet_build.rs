//! HTTP end-to-end tests for `POST /v1/fleet/build` (ADR-0038, F2-7).
//!
//! Boots the full daemon, registers a real directory as a fleet repo,
//! calls the build endpoint, and verifies the response shape and DB state.

use convergio_bus::Bus;
use convergio_db::Pool;
use convergio_durability::{init, Durability};
use convergio_embed::embedder::testing::DeterministicTestEmbedder;
use convergio_embed::EmbedStore;
use convergio_fleet::FleetStore;
use convergio_lifecycle::Supervisor;
use convergio_server::{router, AppState};
use serde_json::Value;
use std::net::SocketAddr;
use std::sync::Arc;
use tempfile::tempdir;
use tokio::net::TcpListener;

async fn boot() -> (String, Arc<FleetStore>, Arc<EmbedStore>, tempfile::TempDir) {
    let dir = tempdir().expect("tempdir");
    let url = format!(
        "sqlite://{}?mode=rwc",
        dir.path().join("state.db").display()
    );
    let pool = Pool::connect(&url).await.expect("connect");

    init(&pool).await.expect("durability init");
    convergio_bus::init(&pool).await.expect("bus init");
    convergio_lifecycle::init(&pool)
        .await
        .expect("lifecycle init");
    let graph = Arc::new(convergio_graph::Store::new(pool.clone()));
    graph.migrate().await.expect("graph migrate");
    convergio_embed::init(&pool).await.expect("embed init");
    convergio_fleet::init(&pool).await.expect("fleet init");

    let embed = Arc::new(EmbedStore::new(pool.clone()));
    let fleet = Arc::new(FleetStore::new(pool.clone()));

    let state = AppState {
        durability: Arc::new(Durability::new(pool.clone())),
        bus: Arc::new(Bus::new(pool.clone())),
        fleet: fleet.clone(),
        supervisor: Arc::new(Supervisor::new(pool)),
        graph,
        embed: embed.clone(),
        embedder: Arc::new(DeterministicTestEmbedder::new(8)),
    };
    let app = router(state);

    let listener = TcpListener::bind(SocketAddr::from(([127, 0, 0, 1], 0)))
        .await
        .expect("bind");
    let addr = listener.local_addr().expect("local addr");
    tokio::spawn(async move {
        axum::serve(listener, app).await.expect("serve");
    });
    (format!("http://{addr}"), fleet, embed, dir)
}

/// Registers a real directory as a fleet repo via the HTTP API.
async fn register_repo(base: &str, name: &str, path: &str, language: &str) {
    let client = reqwest::Client::new();
    let resp = client
        .post(format!("{base}/v1/fleet/repos"))
        .json(&serde_json::json!({
            "name": name,
            "path": path,
            "language": language,
            "parser": "tree-sitter",
        }))
        .send()
        .await
        .expect("register send");
    assert!(
        resp.status().is_success(),
        "register failed: {}",
        resp.status()
    );
}

#[tokio::test]
async fn build_empty_fleet_returns_ok_with_zero_counts() {
    let (base, _fleet, _embed, _dir) = boot().await;
    let client = reqwest::Client::new();
    let body: Value = client
        .post(format!("{base}/v1/fleet/build"))
        .json(&serde_json::json!({}))
        .send()
        .await
        .expect("send")
        .json()
        .await
        .expect("json");
    assert_eq!(body["ok"], true);
    assert_eq!(body["repos_processed"], 0);
    assert_eq!(body["similar_edges_written"], 0);
}

#[tokio::test]
async fn build_with_real_dir_embeds_files() {
    let (base, fleet, embed, _dir) = boot().await;
    let client = reqwest::Client::new();

    // Use the crate migrations dir as a small real directory with .sql files.
    // We register it under a custom extension set that includes "sql" — but since
    // the server defaults to SOURCE_EXTENSIONS which omits ".sql", the file count
    // may be zero. Instead, use a directory that has .rs files.
    let src_dir = env!("CARGO_MANIFEST_DIR");
    register_repo(&base, "server-src", src_dir, "rust").await;

    let body: Value = client
        .post(format!("{base}/v1/fleet/build"))
        .json(&serde_json::json!({}))
        .send()
        .await
        .expect("send")
        .json()
        .await
        .expect("json");

    assert_eq!(body["ok"], true);
    assert_eq!(body["repos_processed"], 1);
    assert_eq!(body["repos_skipped"], 0);

    let considered = body["embed"]["considered"].as_u64().unwrap_or(0);
    assert!(considered > 0, "expected some .rs files to be considered");

    // Mark should have been stamped.
    let repo = fleet.get_repo("server-src").await.expect("get_repo");
    assert!(repo.last_built_at.is_some(), "last_built_at should be set");

    // Embeddings should exist in the store.
    let count = embed.count(Some("server-src")).await.expect("count");
    assert!(count > 0, "expected embeddings in store for server-src");
}

#[tokio::test]
async fn build_is_idempotent() {
    let (base, _fleet, embed, _dir) = boot().await;
    let client = reqwest::Client::new();
    let src_dir = env!("CARGO_MANIFEST_DIR");
    register_repo(&base, "idempotent-repo", src_dir, "rust").await;

    let first: Value = client
        .post(format!("{base}/v1/fleet/build"))
        .json(&serde_json::json!({}))
        .send()
        .await
        .expect("first send")
        .json()
        .await
        .expect("first json");
    let first_count = embed.count(Some("idempotent-repo")).await.expect("count");

    let second: Value = client
        .post(format!("{base}/v1/fleet/build"))
        .json(&serde_json::json!({}))
        .send()
        .await
        .expect("second send")
        .json()
        .await
        .expect("second json");

    // Second run should skip all unchanged files.
    let second_embedded = second["embed"]["embedded"].as_u64().unwrap_or(0);
    let second_skipped = second["embed"]["skipped_unchanged"].as_u64().unwrap_or(0);
    assert_eq!(second_embedded, 0, "second build should embed nothing new");
    assert!(
        second_skipped > 0,
        "second build should skip all unchanged files"
    );
    // Total count must not grow.
    let second_count = embed.count(Some("idempotent-repo")).await.expect("count2");
    assert_eq!(first_count, second_count);
    let _ = first;
}

#[tokio::test]
async fn disabled_repo_is_skipped_by_build() {
    let (base, _fleet, embed, _dir) = boot().await;
    let client = reqwest::Client::new();
    let src_dir = env!("CARGO_MANIFEST_DIR");
    register_repo(&base, "disabled-repo", src_dir, "rust").await;

    // Disable the repo before building.
    client
        .patch(format!("{base}/v1/fleet/repos/disabled-repo"))
        .json(&serde_json::json!({ "enabled": false }))
        .send()
        .await
        .expect("disable send")
        .error_for_status()
        .expect("disable");

    let body: Value = client
        .post(format!("{base}/v1/fleet/build"))
        .json(&serde_json::json!({}))
        .send()
        .await
        .expect("send")
        .json()
        .await
        .expect("json");

    assert_eq!(body["repos_processed"], 0, "disabled repos must be skipped");
    let count = embed.count(Some("disabled-repo")).await.expect("count");
    assert_eq!(count, 0, "no embeddings should exist for disabled repo");
}

#[tokio::test]
async fn refresh_similarity_returns_edge_count() {
    let (base, fleet, _embed, _dir) = boot().await;
    let client = reqwest::Client::new();
    let src_dir = env!("CARGO_MANIFEST_DIR");
    register_repo(&base, "repo-a", src_dir, "rust").await;

    let body: Value = client
        .post(format!("{base}/v1/fleet/build"))
        .json(&serde_json::json!({ "refresh_similarity": true }))
        .send()
        .await
        .expect("send")
        .json()
        .await
        .expect("json");

    assert_eq!(body["ok"], true);
    // With a single repo, no cross-repo edges can exist.
    let edges = body["similar_edges_written"].as_u64().unwrap_or(99);
    assert_eq!(edges, 0, "single repo should produce no cross-repo edges");
    let stored = fleet.count_similar_edges(None).await.expect("count edges");
    assert_eq!(stored, 0);
}
