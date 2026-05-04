use super::*;
use crate::config::{RepoEntry, RepoRole};
use crate::migrate::init;
use crate::store::FleetStore;

async fn setup() -> (FleetStore, tempfile::NamedTempFile) {
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
async fn empty_edges_returns_no_clusters() {
    let (store, _tmp) = setup().await;
    let clusters = find_patterns(&store, 2).await.unwrap();
    assert!(clusters.is_empty());
}

#[tokio::test]
async fn single_repo_pair_filtered_by_min_repos() {
    let (store, _tmp) = setup().await;
    store.add_repo(&repo("alpha")).await.unwrap();
    store.add_repo(&repo("beta")).await.unwrap();
    store
        .upsert_similar_edge("alpha", "n1", "beta", "n2", 0.90)
        .await
        .unwrap();
    let zero = find_patterns(&store, 3).await.unwrap();
    assert!(zero.is_empty(), "cluster spans only 2 repos, min_repos=3");
    let one = find_patterns(&store, 2).await.unwrap();
    assert_eq!(one.len(), 1);
    assert_eq!(one[0].members.len(), 2);
}

#[tokio::test]
async fn confidence_is_average_edge_score() {
    let (store, _tmp) = setup().await;
    store.add_repo(&repo("a")).await.unwrap();
    store.add_repo(&repo("b")).await.unwrap();
    store
        .upsert_similar_edge("a", "n1", "b", "n2", 0.90)
        .await
        .unwrap();
    let clusters = find_patterns(&store, 2).await.unwrap();
    assert_eq!(clusters.len(), 1);
    let diff = (clusters[0].confidence - 0.90).abs();
    assert!(
        diff < 0.01,
        "expected ~0.90, got {}",
        clusters[0].confidence
    );
}

#[tokio::test]
async fn multi_edge_cluster_groups_correctly() {
    let (store, _tmp) = setup().await;
    store.add_repo(&repo("x")).await.unwrap();
    store.add_repo(&repo("y")).await.unwrap();
    store.add_repo(&repo("z")).await.unwrap();
    store
        .upsert_similar_edge("x", "n1", "y", "n2", 0.88)
        .await
        .unwrap();
    store
        .upsert_similar_edge("y", "n2", "z", "n3", 0.92)
        .await
        .unwrap();
    let clusters = find_patterns(&store, 2).await.unwrap();
    assert_eq!(
        clusters.len(),
        1,
        "three nodes should merge into one cluster"
    );
    assert_eq!(clusters[0].members.len(), 3);
}

#[test]
fn stable_id_is_deterministic() {
    let nodes = vec![
        ("alpha".to_owned(), "n1".to_owned()),
        ("beta".to_owned(), "n2".to_owned()),
    ];
    let id1 = stable_id("alpha\x00n1", &nodes);
    let id2 = stable_id("alpha\x00n1", &nodes);
    assert_eq!(id1, id2);
    assert_eq!(id1.len(), 16);
}

#[test]
fn hoist_target_uses_common_word() {
    let members = vec![
        ClusterMember {
            repo: "a".into(),
            name: "state_machine".into(),
            kind: "module".into(),
        },
        ClusterMember {
            repo: "b".into(),
            name: "state_machine".into(),
            kind: "module".into(),
        },
    ];
    let target = derive_hoist_target(&members);
    assert!(
        target.contains("state") || target.contains("machine"),
        "got: {target}"
    );
}
