use super::*;
use crate::config::{RepoEntry, RepoRole};
use crate::migrate::init;

async fn test_store() -> (FleetStore, tempfile::NamedTempFile) {
    let tmp = tempfile::NamedTempFile::new().unwrap();
    let url = format!("sqlite://{}", tmp.path().display());
    let pool = convergio_db::Pool::connect(&url).await.unwrap();
    init(&pool).await.unwrap();
    // graph_nodes table required for node metadata lookups
    convergio_graph::Store::new(pool.clone())
        .migrate()
        .await
        .unwrap();
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
async fn empty_returns_empty() {
    let (store, _tmp) = test_store().await;
    let pairs = find_duplicates(&store, 0.95, None, false).await.unwrap();
    assert!(pairs.is_empty());
}

#[tokio::test]
async fn duplicates_edge_returned() {
    let (store, _tmp) = test_store().await;
    store.add_repo(&repo("alpha")).await.unwrap();
    store.add_repo(&repo("beta")).await.unwrap();
    store
        .upsert_similar_edge_classified("alpha", "n1", "beta", "n2", 0.97, "duplicates")
        .await
        .unwrap();
    let pairs = find_duplicates(&store, 0.95, None, false).await.unwrap();
    assert_eq!(pairs.len(), 1);
    assert_eq!(pairs[0].repo_a, "alpha");
    assert_eq!(pairs[0].repo_b, "beta");
    assert!((pairs[0].score - 0.97).abs() < 1e-4);
}

#[tokio::test]
async fn threshold_filters_low_score() {
    let (store, _tmp) = test_store().await;
    store
        .upsert_similar_edge_classified("a", "n1", "b", "n2", 0.96, "duplicates")
        .await
        .unwrap();
    store
        .upsert_similar_edge_classified("a", "n3", "b", "n4", 0.97, "duplicates")
        .await
        .unwrap();
    let pairs = find_duplicates(&store, 0.965, None, false).await.unwrap();
    assert_eq!(pairs.len(), 1);
    assert!(pairs[0].score >= 0.965);
}

#[tokio::test]
async fn repo_pair_filter_works() {
    let (store, _tmp) = test_store().await;
    store
        .upsert_similar_edge_classified("alpha", "n1", "beta", "n2", 0.97, "duplicates")
        .await
        .unwrap();
    store
        .upsert_similar_edge_classified("alpha", "n3", "gamma", "n4", 0.98, "duplicates")
        .await
        .unwrap();
    let pairs = find_duplicates(&store, 0.95, Some(("alpha", "beta")), false)
        .await
        .unwrap();
    assert_eq!(pairs.len(), 1);
    assert!(pairs.iter().all(|p| {
        (p.repo_a == "alpha" && p.repo_b == "beta") || (p.repo_a == "beta" && p.repo_b == "alpha")
    }));
}

#[tokio::test]
async fn undirected_repo_pair_both_directions() {
    let (store, _tmp) = test_store().await;
    store
        .upsert_similar_edge_classified("beta", "n1", "alpha", "n2", 0.97, "duplicates")
        .await
        .unwrap();
    let pairs = find_duplicates(&store, 0.95, Some(("alpha", "beta")), false)
        .await
        .unwrap();
    assert_eq!(pairs.len(), 1);
}

#[tokio::test]
async fn similar_to_edges_excluded() {
    let (store, _tmp) = test_store().await;
    store
        .upsert_similar_edge_classified("a", "n1", "b", "n2", 0.88, "similar_to")
        .await
        .unwrap();
    let pairs = find_duplicates(&store, 0.85, None, false).await.unwrap();
    assert!(pairs.is_empty());
}

#[test]
fn diff_preview_identical_names() {
    let lines = build_preview("foo", "module", Some("a.rs"), "foo", "module", Some("a.rs"));
    assert_eq!(lines.len(), 1);
    assert!(lines[0].contains("identical"));
}

#[test]
fn diff_preview_name_differs() {
    let lines = build_preview("foo", "module", Some("a.rs"), "bar", "module", Some("a.rs"));
    assert!(lines.iter().any(|l| l.contains("foo") && l.contains("bar")));
}

#[test]
fn diff_preview_truncated_to_three() {
    let lines = build_preview("foo", "module", Some("a.rs"), "bar", "item", Some("b.rs"));
    assert!(lines.len() <= 3);
}
