use super::*;
use crate::config::{RepoEntry, RepoRole};
use crate::migrate::init;
use convergio_embed::EmbedStore;
use convergio_graph::{Edge, EdgeKind, Node, NodeKind, Store as GraphStore};

const MODEL: &str = "test-model";

async fn setup() -> (FleetStore, EmbedStore, GraphStore, tempfile::NamedTempFile) {
    let tmp = tempfile::NamedTempFile::new().unwrap();
    let url = format!("sqlite://{}", tmp.path().display());
    let pool = convergio_db::Pool::connect(&url).await.unwrap();
    init(&pool).await.unwrap();
    convergio_embed::init(&pool).await.unwrap();
    let graph = GraphStore::new(pool.clone());
    graph.migrate().await.unwrap();
    let fleet = FleetStore::new(pool.clone());
    let embed = EmbedStore::new(pool);
    (fleet, embed, graph, tmp)
}

fn doc(id: &str, name: &str, repo: &str) -> Node {
    Node {
        id: id.to_owned(),
        kind: NodeKind::Adr,
        name: name.to_owned(),
        file_path: Some(format!("{repo}/docs/{name}.md")),
        crate_name: "__docs__".to_owned(),
        repo: repo.to_owned(),
        item_kind: None,
        span: None,
    }
}

fn code(id: &str, name: &str, repo: &str) -> Node {
    Node {
        id: id.to_owned(),
        kind: NodeKind::Item,
        name: name.to_owned(),
        file_path: Some(format!("{repo}/src/lib.rs")),
        crate_name: repo.to_owned(),
        repo: repo.to_owned(),
        item_kind: Some("fn"),
        span: None,
    }
}

fn repo(name: &str) -> RepoEntry {
    RepoEntry {
        name: name.into(),
        path: format!("/repos/{name}"),
        language: "rust".into(),
        parser: "syn".into(),
        role: RepoRole::Engine,
        derives_from: None,
    }
}

async fn upsert_embed(embed: &EmbedStore, r: &str, id: &str, v: &[f32]) {
    embed.upsert(r, id, MODEL, v, "h").await.unwrap();
}

async fn mention(graph: &GraphStore, src: &str, dst: &str) {
    graph
        .upsert_edge(&Edge {
            src: src.into(),
            dst: dst.into(),
            kind: EdgeKind::Mentions,
            weight: 1,
        })
        .await
        .unwrap();
}

#[tokio::test]
async fn empty_returns_empty_drift() {
    let (fleet, embed, _g, _t) = setup().await;
    let out = find_doc_drift(&fleet, &embed, MODEL, 0.2, None)
        .await
        .unwrap();
    assert!(out.is_empty());
}

#[tokio::test]
async fn snapshot_persists_alignment() {
    let (fleet, embed, graph, _t) = setup().await;
    fleet.add_repo(&repo("eng")).await.unwrap();
    graph.upsert_node(&doc("adr", "0042", "eng")).await.unwrap();
    graph.upsert_node(&code("fn1", "foo", "eng")).await.unwrap();
    mention(&graph, "adr", "fn1").await;
    upsert_embed(&embed, "eng", "adr", &[1.0, 0.0]).await;
    upsert_embed(&embed, "eng", "fn1", &[1.0, 0.0]).await;

    let report = snapshot_doc_alignment(&fleet, &embed, MODEL).await.unwrap();
    assert_eq!(report.nodes_considered, 1);
    assert_eq!(report.nodes_snapshotted, 1);
}

#[tokio::test]
async fn drift_threshold_filters_below_cutoff() {
    let (fleet, embed, graph, _t) = setup().await;
    fleet.add_repo(&repo("eng")).await.unwrap();
    graph.upsert_node(&doc("adr", "0042", "eng")).await.unwrap();
    graph.upsert_node(&code("fn1", "foo", "eng")).await.unwrap();
    mention(&graph, "adr", "fn1").await;
    upsert_embed(&embed, "eng", "adr", &[1.0, 0.0]).await;
    upsert_embed(&embed, "eng", "fn1", &[1.0, 0.0]).await;
    snapshot_doc_alignment(&fleet, &embed, MODEL).await.unwrap();

    // Mutate code embedding by a tiny amount → delta below 0.2
    upsert_embed(&embed, "eng", "fn1", &[0.99, 0.14]).await;
    let out = find_doc_drift(&fleet, &embed, MODEL, 0.2, None)
        .await
        .unwrap();
    assert!(
        out.is_empty(),
        "delta below threshold should not surface: {out:?}"
    );
}

#[tokio::test]
async fn snapshot_idempotent() {
    let (fleet, embed, graph, _t) = setup().await;
    fleet.add_repo(&repo("eng")).await.unwrap();
    graph.upsert_node(&doc("adr", "0042", "eng")).await.unwrap();
    graph.upsert_node(&code("fn1", "foo", "eng")).await.unwrap();
    mention(&graph, "adr", "fn1").await;
    upsert_embed(&embed, "eng", "adr", &[1.0, 0.0]).await;
    upsert_embed(&embed, "eng", "fn1", &[1.0, 0.0]).await;

    let first = snapshot_doc_alignment(&fleet, &embed, MODEL).await.unwrap();
    let second = snapshot_doc_alignment(&fleet, &embed, MODEL).await.unwrap();
    assert_eq!(first.nodes_snapshotted, second.nodes_snapshotted);
}

#[tokio::test]
async fn missing_snapshot_returns_empty() {
    let (fleet, embed, graph, _t) = setup().await;
    fleet.add_repo(&repo("eng")).await.unwrap();
    graph.upsert_node(&doc("adr", "0042", "eng")).await.unwrap();
    upsert_embed(&embed, "eng", "adr", &[1.0, 0.0]).await;
    // no snapshot taken
    let out = find_doc_drift(&fleet, &embed, MODEL, 0.2, None)
        .await
        .unwrap();
    assert!(out.is_empty(), "no snapshot => no drift");
}
