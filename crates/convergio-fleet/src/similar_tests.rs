use crate::config::{RepoEntry, RepoRole};
use crate::migrate::init;
use crate::store::FleetStore;

async fn test_store() -> (FleetStore, tempfile::NamedTempFile) {
    let tmp = tempfile::NamedTempFile::new().unwrap();
    let url = format!("sqlite://{}", tmp.path().display());
    let pool = convergio_db::Pool::connect(&url).await.unwrap();
    init(&pool).await.unwrap();
    (FleetStore::new(pool), tmp)
}

fn repo(name: &str) -> RepoEntry {
    RepoEntry {
        name: name.to_owned(),
        path: format!("/repos/{name}"),
        language: "rust".to_owned(),
        parser: "syn".to_owned(),
        role: RepoRole::Engine,
        derives_from: None,
    }
}

#[tokio::test]
async fn upsert_and_count() {
    let (store, _tmp) = test_store().await;
    store.add_repo(&repo("alpha")).await.unwrap();
    store.add_repo(&repo("beta")).await.unwrap();
    store
        .upsert_similar_edge("alpha", "src/lib.rs", "beta", "src/lib.rs", 0.90)
        .await
        .unwrap();
    assert_eq!(store.count_similar_edges(None).await.unwrap(), 1);
    assert_eq!(
        store.count_similar_edges(Some("similar_to")).await.unwrap(),
        1
    );
    assert_eq!(
        store.count_similar_edges(Some("duplicates")).await.unwrap(),
        0
    );
}

#[tokio::test]
async fn upsert_idempotent_updates_score() {
    let (store, _tmp) = test_store().await;
    store
        .upsert_similar_edge("a", "n1", "b", "n2", 0.86)
        .await
        .unwrap();
    store
        .upsert_similar_edge("a", "n1", "b", "n2", 0.96)
        .await
        .unwrap();
    let edges = store.list_similar_edges(10).await.unwrap();
    assert_eq!(edges.len(), 1);
    assert!(edges[0].score >= 0.95, "score should be updated to 0.96");
    assert_eq!(edges[0].kind, "duplicates");
    assert_eq!(edges[0].weight, 960);
}

#[tokio::test]
async fn clear_removes_all_edges() {
    let (store, _tmp) = test_store().await;
    store
        .upsert_similar_edge("a", "n1", "b", "n2", 0.88)
        .await
        .unwrap();
    store.clear_similar_edges().await.unwrap();
    assert_eq!(store.count_similar_edges(None).await.unwrap(), 0);
}

#[tokio::test]
async fn weight_computed_correctly() {
    let (store, _tmp) = test_store().await;
    store
        .upsert_similar_edge("x", "n1", "y", "n2", 0.875)
        .await
        .unwrap();
    let edges = store.list_similar_edges(1).await.unwrap();
    assert_eq!(edges[0].weight, 875);
}

#[tokio::test]
async fn upsert_below_similar_to_threshold_is_noop() {
    // Regression: `upsert_similar_edge` used to store any score below
    // DUPLICATES_THRESHOLD as `similar_to`, even when the score was
    // below the documented 0.85 `SIMILAR_TO_THRESHOLD`. Callers should
    // not be able to slip sub-threshold edges into the table.
    let (store, _tmp) = test_store().await;
    store
        .upsert_similar_edge("a", "n1", "b", "n2", 0.10)
        .await
        .unwrap();
    store
        .upsert_similar_edge("a", "n1", "b", "n2", 0.84)
        .await
        .unwrap();
    assert_eq!(
        store.count_similar_edges(None).await.unwrap(),
        0,
        "scores below SIMILAR_TO_THRESHOLD must not produce edges"
    );
}

#[tokio::test]
async fn classified_upsert_respects_explicit_kind() {
    let (store, _tmp) = test_store().await;
    // Score would normally give "duplicates" (≥0.95), but we force "similar_to".
    store
        .upsert_similar_edge_classified("x", "n1", "y", "n2", 0.97, "similar_to")
        .await
        .unwrap();
    let edges = store.list_similar_edges(1).await.unwrap();
    assert_eq!(edges[0].kind, "similar_to");
}
