//! End-to-end tests for convergio-fleet: init + FleetStore against a
//! real tempdir SQLite pool.

use convergio_fleet::{
    config::{RepoEntry, RepoRole},
    init, FleetStore,
};

async fn setup() -> (FleetStore, tempfile::NamedTempFile) {
    let tmp = tempfile::NamedTempFile::new().unwrap();
    let url = format!("sqlite://{}", tmp.path().display());
    let pool = convergio_db::Pool::connect(&url).await.unwrap();
    init(&pool).await.unwrap();
    (FleetStore::new(pool), tmp)
}

fn make_entry(name: &str) -> RepoEntry {
    RepoEntry {
        name: name.to_owned(),
        path: format!("/repos/{name}"),
        language: "rust".to_owned(),
        parser: "syn".to_owned(),
        role: RepoRole::Downstream,
        derives_from: None,
    }
}

#[tokio::test]
async fn init_is_idempotent() {
    let tmp = tempfile::NamedTempFile::new().unwrap();
    let url = format!("sqlite://{}", tmp.path().display());
    let pool = convergio_db::Pool::connect(&url).await.unwrap();
    // Run twice — must not error.
    init(&pool).await.unwrap();
    init(&pool).await.unwrap();
}

#[tokio::test]
async fn add_list_get_roundtrip() {
    let (store, _tmp) = setup().await;
    store.add_repo(&make_entry("alpha")).await.unwrap();
    store.add_repo(&make_entry("beta")).await.unwrap();

    let all = store.list_repos().await.unwrap();
    assert_eq!(all.len(), 2);

    let r = store.get_repo("alpha").await.unwrap();
    assert_eq!(r.language, "rust");
    assert!(r.enabled);
    assert!(r.last_built_at.is_none());
}

#[tokio::test]
async fn mark_built_and_disable() {
    let (store, _tmp) = setup().await;
    store.add_repo(&make_entry("engine")).await.unwrap();

    store.mark_built("engine").await.unwrap();
    let r = store.get_repo("engine").await.unwrap();
    assert!(r.last_built_at.is_some());

    store.set_enabled("engine", false).await.unwrap();
    let r = store.get_repo("engine").await.unwrap();
    assert!(!r.enabled);
}

#[tokio::test]
async fn remove_then_list_empty() {
    let (store, _tmp) = setup().await;
    store.add_repo(&make_entry("tmp-repo")).await.unwrap();
    store.remove_repo("tmp-repo").await.unwrap();
    let all = store.list_repos().await.unwrap();
    assert!(all.is_empty());
}
