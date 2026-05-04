//! Integration tests for graph context-pack ranking.

use convergio_db::Pool;
use convergio_graph::{
    for_task_text, for_task_text_with_metadata, Node, NodeKind, Store, StructuredContextMetadata,
    DEFAULT_TOKEN_BUDGET,
};

async fn migrated_store() -> anyhow::Result<(tempfile::TempDir, Store)> {
    let dir = tempfile::tempdir()?;
    let url = format!("sqlite://{}?mode=rwc", dir.path().join("g.db").display());
    let pool = Pool::connect(&url).await?;
    let store = Store::new(pool);
    store.migrate().await?;
    Ok((dir, store))
}

fn node(id: &str, kind: NodeKind, name: &str, crate_name: &str, file_path: Option<&str>) -> Node {
    Node {
        id: id.into(),
        kind,
        name: name.into(),
        file_path: file_path.map(str::to_string),
        crate_name: crate_name.into(),
        repo: "test-repo".into(),
        item_kind: None,
        span: None,
    }
}

#[tokio::test]
async fn ranks_matches_by_kind_score_then_name() -> anyhow::Result<()> {
    let (_dir, store) = migrated_store().await?;
    for n in [
        node(
            "item",
            NodeKind::Item,
            "graph_item",
            "x",
            Some("src/lib.rs"),
        ),
        node(
            "module",
            NodeKind::Module,
            "graph_module",
            "x",
            Some("src/lib.rs"),
        ),
        node("crate", NodeKind::Crate, "graph_crate", "graph_crate", None),
    ] {
        store.upsert_node(&n).await?;
    }

    let pack = for_task_text(&store, "t1", "graph", 10, DEFAULT_TOKEN_BUDGET).await?;
    let names: Vec<_> = pack.matched_nodes.iter().map(|n| n.name.as_str()).collect();
    assert_eq!(names, vec!["graph_crate", "graph_module", "graph_item"]);
    Ok(())
}

#[tokio::test]
async fn token_budget_limits_returned_files() -> anyhow::Result<()> {
    let (_dir, store) = migrated_store().await?;
    for n in [
        node(
            "lib",
            NodeKind::Item,
            "budgetmatch_lib",
            "x",
            Some("src/lib.rs"),
        ),
        node(
            "model",
            NodeKind::Item,
            "budgetmatch_model",
            "x",
            Some("src/model.rs"),
        ),
    ] {
        store.upsert_node(&n).await?;
    }

    let pack = for_task_text(&store, "t1", "budgetmatch", 10, 1).await?;
    assert_eq!(pack.matched_nodes.len(), 2);
    assert!(pack.files.is_empty());
    assert!(pack.estimated_tokens <= 1);
    Ok(())
}

#[tokio::test]
async fn structured_metadata_scopes_node_matches() -> anyhow::Result<()> {
    let (_dir, store) = migrated_store().await?;
    for n in [
        node(
            "graph-crate",
            NodeKind::Crate,
            "convergio-graph",
            "convergio-graph",
            None,
        ),
        node(
            "graph-item",
            NodeKind::Item,
            "structured_context",
            "convergio-graph",
            Some("crates/convergio-graph/src/query.rs"),
        ),
        node(
            "cli-item",
            NodeKind::Item,
            "structured_context",
            "convergio-cli",
            Some("crates/convergio-cli/src/commands/graph.rs"),
        ),
    ] {
        store.upsert_node(&n).await?;
    }

    let metadata = StructuredContextMetadata {
        primary_crate: Some("convergio-graph".into()),
        validation_profile: Some("graph".into()),
        ..StructuredContextMetadata::default()
    };
    let pack =
        for_task_text_with_metadata(&store, "t1", "structured context", metadata, 10, 10_000)
            .await?;

    assert_eq!(
        pack.structured_metadata.primary_crate.as_deref(),
        Some("convergio-graph")
    );
    assert_eq!(
        pack.structured_metadata.validation_profile.as_deref(),
        Some("graph")
    );
    assert!(pack
        .matched_nodes
        .iter()
        .all(|n| n.crate_name == "convergio-graph"));
    assert!(pack.matched_nodes.iter().any(|n| n.kind == "crate"));
    Ok(())
}

#[tokio::test]
async fn task_text_metadata_is_parsed() -> anyhow::Result<()> {
    let (_dir, store) = migrated_store().await?;
    store
        .upsert_node(&node(
            "graph-crate",
            NodeKind::Crate,
            "convergio-graph",
            "convergio-graph",
            None,
        ))
        .await?;
    store
        .upsert_node(&node(
            "adr-0014",
            NodeKind::Adr,
            "0014",
            "docs",
            Some("docs/adr/0014-code-graph-tier3-retrieval.md"),
        ))
        .await?;

    let pack = for_task_text(
        &store,
        "t1",
        "Improve context packs\ncrate: convergio-graph\nadr_required: 0014\nvalidation_profile: graph",
        10,
        10_000,
    )
    .await?;
    assert_eq!(
        pack.structured_metadata.primary_crate.as_deref(),
        Some("convergio-graph")
    );
    assert_eq!(pack.structured_metadata.adr_required, vec!["0014"]);
    assert_eq!(
        pack.structured_metadata.validation_profile.as_deref(),
        Some("graph")
    );
    assert!(pack.related_adrs.iter().any(|a| a.adr_id == "0014"
        && a.via_crate == "metadata"
        && a.file_path == "docs/adr/0014-code-graph-tier3-retrieval.md"));
    Ok(())
}
