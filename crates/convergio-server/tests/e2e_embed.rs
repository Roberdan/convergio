//! HTTP end-to-end test for `convergio-embed` (ADR-0038 F1-α).
//!
//! Boots the full daemon router with a tempdir SQLite, seeds the
//! [`EmbedStore`] directly, then hits `/v1/embed/stats` over HTTP and
//! verifies the response shape.

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
        supervisor: Arc::new(Supervisor::new(pool)),
        graph,
        embed: embed.clone(),
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
    let client = reqwest::Client::new();
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

    let client = reqwest::Client::new();
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
