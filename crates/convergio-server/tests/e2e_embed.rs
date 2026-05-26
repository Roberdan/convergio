//! HTTP end-to-end test for `convergio-embed` (ADR-0038 F1-α).
//!
//! Boots the full daemon router with a tempdir SQLite, seeds the
//! [`EmbedStore`] directly, then hits `/v1/embed/stats` over HTTP and
//! verifies the response shape.
mod common;

use convergio_bus::Bus;
use convergio_db::Pool;
use convergio_durability::{init, Durability};
use convergio_embed::embedder::testing::DeterministicTestEmbedder;
use convergio_embed::{EmbedStore, Embedder, SourceText};
use convergio_lifecycle::Supervisor;
use convergio_server::{router, AppState};
use serde_json::Value;
use std::net::SocketAddr;
use std::sync::Arc;
use tempfile::tempdir;
use tokio::net::TcpListener;

async fn boot() -> (String, Arc<EmbedStore>, tempfile::TempDir) {
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
    let embed = Arc::new(EmbedStore::new(pool.clone()));

    let state = AppState {
        durability: Arc::new(Durability::new(pool.clone())),
        bus: Arc::new(Bus::new(pool.clone())),
        fleet: Arc::new(convergio_fleet::FleetStore::new(pool.clone())),
        fleet_plans: Arc::new(convergio_fleet::FleetPlanStore::new(pool.clone())),
        supervisor: Arc::new(Supervisor::new(pool)),
        graph,
        embed: embed.clone(),
        embedder: Arc::new(DeterministicTestEmbedder::new(384)),
        audit_verify_cache: Arc::new(std::sync::Mutex::new(None)),
    };
    let app = router(state);

    let listener = TcpListener::bind(SocketAddr::from(([127, 0, 0, 1], 0)))
        .await
        .expect("bind");
    let addr = listener.local_addr().expect("local addr");
    tokio::spawn(async move {
        axum::serve(listener, app).await.expect("serve");
    });
    (format!("http://{addr}"), embed, dir)
}

#[tokio::test]
async fn embed_stats_returns_zero_on_fresh_db() {
    let (base, _embed, _dir) = boot().await;
    let client = common::client();
    let body: Value = client
        .get(format!("{base}/v1/embed/stats"))
        .send()
        .await
        .expect("send")
        .json()
        .await
        .expect("json");
    assert_eq!(body["ok"], true);
    assert_eq!(body["count"], 0);
    assert_eq!(body["repo"], Value::Null);
}

#[tokio::test]
async fn embed_stats_reflects_seeded_rows() {
    let (base, embed, _dir) = boot().await;

    let embedder = DeterministicTestEmbedder::new(8);
    for (repo, n) in [("convergio", 3_u32), ("convergio-edu", 2)] {
        for i in 0..n {
            let id = format!("{repo}-{i}");
            let v = embedder.embed(&id).expect("embed");
            let h = SourceText::new(&id).source_hash;
            embed
                .upsert(repo, &id, embedder.model_id(), &v, &h)
                .await
                .expect("upsert");
        }
    }

    let client = common::client();
    let total: Value = client
        .get(format!("{base}/v1/embed/stats"))
        .send()
        .await
        .expect("send")
        .json()
        .await
        .expect("json");
    assert_eq!(total["count"], 5);

    let scoped: Value = client
        .get(format!("{base}/v1/embed/stats"))
        .query(&[("repo", "convergio")])
        .send()
        .await
        .expect("send")
        .json()
        .await
        .expect("json");
    assert_eq!(scoped["count"], 3);
    assert_eq!(scoped["repo"], "convergio");
}

#[tokio::test]
async fn embed_warm_returns_model_and_dim() {
    let (base, _embed, _dir) = boot().await;
    let client = common::client();
    let body: Value = client
        .post(format!("{base}/v1/embed/warm"))
        .send()
        .await
        .expect("send")
        .json()
        .await
        .expect("json");
    assert_eq!(body["ok"], true);
    assert_eq!(body["model"], "deterministic-test-d384");
    assert_eq!(body["dim"], 384);
}

#[tokio::test]
async fn embed_build_walks_directory_and_ingests() {
    let (base, embed, _dir) = boot().await;
    let corpus = tempdir().expect("corpus dir");
    std::fs::create_dir_all(corpus.path().join("src")).expect("mk src");
    std::fs::write(
        corpus.path().join("src/lib.rs"),
        "//! crate doc\npub fn answer() -> u8 { 42 }\n",
    )
    .expect("write rs");
    std::fs::write(corpus.path().join("README.md"), "# convergio\n").expect("write md");

    let client = common::client();
    let body: Value = client
        .post(format!("{base}/v1/embed/build"))
        .json(&serde_json::json!({
            "repo": "convergio",
            "root": corpus.path().to_string_lossy(),
        }))
        .send()
        .await
        .expect("send")
        .json()
        .await
        .expect("json");
    assert_eq!(body["ok"], true);
    assert_eq!(body["report"]["embedded"], 2);
    // Re-running the same build is idempotent under source_hash.
    let again: Value = client
        .post(format!("{base}/v1/embed/build"))
        .json(&serde_json::json!({
            "repo": "convergio",
            "root": corpus.path().to_string_lossy(),
        }))
        .send()
        .await
        .expect("send")
        .json()
        .await
        .expect("json");
    assert_eq!(again["report"]["embedded"], 0);
    assert_eq!(again["report"]["skipped_unchanged"], 2);
    // Store reflects the persisted rows.
    assert_eq!(embed.count(Some("convergio")).await.expect("count"), 2);
}

#[tokio::test]
async fn embed_for_task_finds_seeded_match() {
    let (base, _embed, _dir) = boot().await;
    let corpus = tempdir().expect("corpus dir");
    std::fs::create_dir_all(corpus.path().join("src")).expect("mk src");
    std::fs::write(corpus.path().join("src/auth.rs"), "fn login() {}\n").expect("write");
    std::fs::write(corpus.path().join("src/payments.rs"), "fn checkout() {}\n").expect("write");

    let client = common::client();
    // Build first.
    let _: Value = client
        .post(format!("{base}/v1/embed/build"))
        .json(&serde_json::json!({
            "repo": "convergio",
            "root": corpus.path().to_string_lossy(),
        }))
        .send()
        .await
        .expect("send")
        .json()
        .await
        .expect("json");

    // Querying with a seeded source string returns its node first
    // with cosine ≈ 1 (deterministic embedder is hash-based, so the
    // exact text yields the exact stored vector).
    let body: Value = client
        .post(format!("{base}/v1/embed/for-task"))
        .json(&serde_json::json!({
            "query": "fn login() {}",
            "top_k": 5,
        }))
        .send()
        .await
        .expect("send")
        .json()
        .await
        .expect("json");
    assert_eq!(body["ok"], true);
    let hits = body["hits"].as_array().expect("hits array");
    assert_eq!(hits[0]["node_id"], "src/auth.rs");
    assert_eq!(hits[0]["match_source"], "semantic");
}
