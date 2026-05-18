//! End-to-end test for [`convergio_fleet::find_doc_drift`] (ADR-0038, F3-6).
//!
//! Seeds an ADR + linked code with identical embeddings, snapshots
//! the alignment, then perturbs the code embedding far enough to
//! cross the 0.2 default drift threshold. Verifies the ADR is
//! returned with positive delta.

use convergio_embed::EmbedStore;
use convergio_fleet::config::{RepoEntry, RepoRole};
use convergio_fleet::{find_doc_drift, snapshot_doc_alignment, FleetStore};
use convergio_graph::{Edge, EdgeKind, Node, NodeKind, Store as GraphStore};

const MODEL: &str = "test-model";

async fn boot() -> (FleetStore, EmbedStore, GraphStore, tempfile::NamedTempFile) {
    let tmp = tempfile::NamedTempFile::new().unwrap();
    let url = format!("sqlite://{}", tmp.path().display());
    let pool = convergio_db::Pool::connect(&url).await.unwrap();
    convergio_fleet::init(&pool).await.unwrap();
    convergio_embed::init(&pool).await.unwrap();
    let graph = GraphStore::new(pool.clone());
    graph.migrate().await.unwrap();
    let fleet = FleetStore::new(pool.clone());
    let embed = EmbedStore::new(pool);
    (fleet, embed, graph, tmp)
}

fn adr(id: &str, repo: &str) -> Node {
    Node {
        id: id.to_owned(),
        kind: NodeKind::Adr,
        name: format!("ADR-{id}"),
        file_path: Some(format!("docs/adr/{id}.md")),
        crate_name: "__docs__".to_owned(),
        repo: repo.to_owned(),
        item_kind: None,
        span: None,
    }
}

fn code(id: &str, repo: &str) -> Node {
    Node {
        id: id.to_owned(),
        kind: NodeKind::Item,
        name: id.to_owned(),
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

#[tokio::test]
async fn doc_drift_finds_seeded_drift() {
    let (fleet, embed, graph, _t) = boot().await;
    fleet.add_repo(&repo("eng")).await.unwrap();

    graph.upsert_node(&adr("0042", "eng")).await.unwrap();
    graph.upsert_node(&code("fn_target", "eng")).await.unwrap();
    graph
        .upsert_edge(&Edge {
            src: "0042".into(),
            dst: "fn_target".into(),
            kind: EdgeKind::Mentions,
            weight: 1,
        })
        .await
        .unwrap();

    // Phase 1: snapshot with aligned embeddings (cosine = 1.0).
    embed
        .upsert("eng", "0042", MODEL, &[1.0, 0.0], "h")
        .await
        .unwrap();
    embed
        .upsert("eng", "fn_target", MODEL, &[1.0, 0.0], "h")
        .await
        .unwrap();
    let snap = snapshot_doc_alignment(&fleet, &embed, MODEL).await.unwrap();
    assert_eq!(snap.nodes_snapshotted, 1);

    // Phase 2: rewrite the code embedding to be orthogonal — drift = 1.0.
    embed
        .upsert("eng", "fn_target", MODEL, &[0.0, 1.0], "h")
        .await
        .unwrap();

    let out = find_doc_drift(&fleet, &embed, MODEL, 0.2, None)
        .await
        .unwrap();
    assert_eq!(out.len(), 1, "expected one drift row, got: {out:?}");
    let row = &out[0];
    assert_eq!(row.node_id, "0042");
    assert_eq!(row.repo, "eng");
    assert!(
        row.delta > 0.5,
        "delta must reflect the orthogonal mutation, got: {}",
        row.delta
    );
    assert_eq!(row.linked_count, 1);
    assert!((row.snapshot_score - 1.0).abs() < 0.05);
    assert!(row.current_score.abs() < 0.05);
}
