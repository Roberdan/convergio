//! Integration tests for graph SQLite storage.

use chrono::Utc;
use convergio_db::Pool;
use convergio_graph::{Edge, EdgeKind, Node, NodeKind, Store};

async fn migrated_store() -> anyhow::Result<(tempfile::TempDir, Store)> {
    let dir = tempfile::tempdir()?;
    let url = format!("sqlite://{}?mode=rwc", dir.path().join("g.db").display());
    let pool = Pool::connect(&url).await?;
    let store = Store::new(pool);
    store.migrate().await?;
    Ok((dir, store))
}

#[tokio::test]
async fn migrate_creates_tables() -> anyhow::Result<()> {
    let (_dir, store) = migrated_store().await?;
    assert_eq!(store.count_nodes().await?, 0);
    Ok(())
}

#[tokio::test]
async fn upsert_then_count() -> anyhow::Result<()> {
    let (_dir, store) = migrated_store().await?;
    let n = Node {
        id: "abc".into(),
        kind: NodeKind::Crate,
        name: "test-crate".into(),
        file_path: None,
        crate_name: "test-crate".into(),
        repo: "test-repo".into(),
        item_kind: None,
        span: None,
    };
    store.upsert_node(&n).await?;
    assert_eq!(store.count_nodes().await?, 1);

    store.upsert_node(&n).await?;
    assert_eq!(store.count_nodes().await?, 1);
    Ok(())
}

#[tokio::test]
async fn upsert_file_replaces_previous() -> anyhow::Result<()> {
    let (_dir, store) = migrated_store().await?;
    let module = Node {
        id: "m1".into(),
        kind: NodeKind::Module,
        name: "lib".into(),
        file_path: Some("src/lib.rs".into()),
        crate_name: "x".into(),
        repo: "test-repo".into(),
        item_kind: None,
        span: None,
    };
    let item = Node {
        id: "i1".into(),
        kind: NodeKind::Item,
        name: "Foo".into(),
        file_path: Some("src/lib.rs".into()),
        crate_name: "x".into(),
        repo: "test-repo".into(),
        item_kind: Some("struct"),
        span: None,
    };
    store
        .upsert_file(
            "src/lib.rs",
            Utc::now(),
            &[module.clone(), item],
            &[Edge {
                src: "m1".into(),
                dst: "i1".into(),
                kind: EdgeKind::Declares,
                weight: 1,
            }],
        )
        .await?;
    assert_eq!(store.count_nodes().await?, 2);
    assert_eq!(store.count_edges().await?, 1);

    store
        .upsert_file("src/lib.rs", Utc::now(), &[module], &[])
        .await?;
    assert_eq!(store.count_nodes().await?, 1);
    assert_eq!(store.count_edges().await?, 0);
    Ok(())
}
