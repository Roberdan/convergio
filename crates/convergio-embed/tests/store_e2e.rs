//! End-to-end test for the `convergio-embed` storage layer.
//!
//! Boots a tempdir SQLite via `convergio-db::Pool`, runs the migration,
//! exercises the full [`EmbedStore`] API, and verifies brute-force
//! cosine KNN returns the expected ordering.

use convergio_db::Pool;
use convergio_embed::embedder::testing::DeterministicTestEmbedder;
use convergio_embed::{init, EmbedStore, Embedder, SourceText};
use tempfile::tempdir;

async fn boot() -> (Pool, tempfile::TempDir) {
    let dir = tempdir().expect("tempdir");
    let url = format!(
        "sqlite://{}?mode=rwc",
        dir.path().join("state.db").display()
    );
    let pool = Pool::connect(&url).await.expect("connect");
    init(&pool).await.expect("init migration");
    (pool, dir)
}

#[tokio::test]
async fn upsert_get_roundtrip() {
    let (pool, _dir) = boot().await;
    let store = EmbedStore::new(pool);
    let embedder = DeterministicTestEmbedder::new(8);
    let text = SourceText::new("alpha");
    let v = embedder.embed(&text.text).expect("embed");

    store
        .upsert(
            "convergio",
            "node-1",
            embedder.model_id(),
            &v,
            &text.source_hash,
        )
        .await
        .expect("upsert");
    let got = store
        .get("convergio", "node-1", embedder.model_id())
        .await
        .expect("get")
        .expect("row exists");
    assert_eq!(got.dim, 8);
    assert_eq!(got.vec, v);
    assert_eq!(got.source_hash, text.source_hash);
}

#[tokio::test]
async fn needs_reembed_flips_on_hash_change() {
    let (pool, _dir) = boot().await;
    let store = EmbedStore::new(pool);
    let embedder = DeterministicTestEmbedder::new(8);
    let v1 = embedder.embed("first").expect("embed");
    let h1 = SourceText::new("first").source_hash;
    store
        .upsert("convergio", "node-x", embedder.model_id(), &v1, &h1)
        .await
        .expect("upsert");

    // Same hash → no re-embed.
    assert!(!store
        .needs_reembed("convergio", "node-x", embedder.model_id(), &h1)
        .await
        .expect("check"));

    // Different hash → re-embed.
    let h2 = SourceText::new("second").source_hash;
    assert!(store
        .needs_reembed("convergio", "node-x", embedder.model_id(), &h2)
        .await
        .expect("check"));

    // Missing row → re-embed.
    assert!(store
        .needs_reembed("convergio", "missing", embedder.model_id(), &h1)
        .await
        .expect("check"));
}

#[tokio::test]
async fn delete_removes_row() {
    let (pool, _dir) = boot().await;
    let store = EmbedStore::new(pool);
    let embedder = DeterministicTestEmbedder::new(8);
    let v = embedder.embed("x").expect("embed");
    let h = SourceText::new("x").source_hash;
    store
        .upsert("convergio", "n", embedder.model_id(), &v, &h)
        .await
        .expect("upsert");
    let deleted = store
        .delete("convergio", "n", embedder.model_id())
        .await
        .expect("delete");
    assert_eq!(deleted, 1);
    assert!(store
        .get("convergio", "n", embedder.model_id())
        .await
        .expect("get")
        .is_none());
}

#[tokio::test]
async fn count_filters_by_repo() {
    let (pool, _dir) = boot().await;
    let store = EmbedStore::new(pool);
    let embedder = DeterministicTestEmbedder::new(8);
    for i in 0..3 {
        let id = format!("a-{i}");
        let v = embedder.embed(&id).expect("embed");
        let h = SourceText::new(&id).source_hash;
        store
            .upsert("convergio", &id, embedder.model_id(), &v, &h)
            .await
            .expect("upsert");
    }
    for i in 0..2 {
        let id = format!("b-{i}");
        let v = embedder.embed(&id).expect("embed");
        let h = SourceText::new(&id).source_hash;
        store
            .upsert("convergio-edu", &id, embedder.model_id(), &v, &h)
            .await
            .expect("upsert");
    }
    assert_eq!(store.count(None).await.expect("count"), 5);
    assert_eq!(store.count(Some("convergio")).await.expect("count"), 3);
    assert_eq!(store.count(Some("convergio-edu")).await.expect("count"), 2);
}

#[tokio::test]
async fn nearest_brute_force_orders_by_cosine() {
    let (pool, _dir) = boot().await;
    let store = EmbedStore::new(pool);
    let embedder = DeterministicTestEmbedder::new(16);

    let inputs = ["alpha", "beta", "gamma", "delta", "epsilon"];
    for (i, s) in inputs.iter().enumerate() {
        let v = embedder.embed(s).expect("embed");
        let h = SourceText::new(*s).source_hash;
        store
            .upsert("convergio", &format!("n-{i}"), embedder.model_id(), &v, &h)
            .await
            .expect("upsert");
    }

    // Query with one of the seeded vectors — that one ranks first
    // with cosine ≈ 1.
    let query = embedder.embed("gamma").expect("embed");
    let hits = store
        .nearest_brute_force(&query, embedder.model_id(), 5)
        .await
        .expect("knn");
    assert_eq!(hits.len(), 5);
    assert_eq!(hits[0].node_id, "n-2", "gamma was index 2");
    assert!(
        (hits[0].score - 1.0).abs() < 1e-5,
        "top score should be ~1.0, was {}",
        hits[0].score
    );
}

#[tokio::test]
async fn nearest_brute_force_filters_by_model_id() {
    let (pool, _dir) = boot().await;
    let store = EmbedStore::new(pool);
    let small = DeterministicTestEmbedder::new(8);
    let big = DeterministicTestEmbedder::new(16);
    let v8 = small.embed("a").expect("embed");
    let v16 = big.embed("b").expect("embed");
    store
        .upsert("convergio", "n8", small.model_id(), &v8, "h1")
        .await
        .expect("upsert");
    store
        .upsert("convergio", "n16", big.model_id(), &v16, "h2")
        .await
        .expect("upsert");

    // Query against the 8-dim model id: only n8 should come back.
    let q8 = small.embed("a").expect("embed");
    let hits = store
        .nearest_brute_force(&q8, small.model_id(), 10)
        .await
        .expect("knn");
    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0].node_id, "n8");
}

#[tokio::test]
async fn nearest_brute_force_handles_empty_store() {
    let (pool, _dir) = boot().await;
    let store = EmbedStore::new(pool);
    let embedder = DeterministicTestEmbedder::new(8);
    let q = embedder.embed("anything").expect("embed");
    let hits = store
        .nearest_brute_force(&q, embedder.model_id(), 5)
        .await
        .expect("knn");
    assert!(hits.is_empty());
}
