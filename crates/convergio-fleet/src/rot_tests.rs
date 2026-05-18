use super::*;
use crate::config::{RepoEntry, RepoRole};
use crate::migrate::init;
use convergio_embed::EmbedStore;
use convergio_graph::{Node, NodeKind, Store as GraphStore};

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

fn item(id: &str, name: &str, repo: &str) -> Node {
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

fn repo(name: &str, role: RepoRole) -> RepoEntry {
    RepoEntry {
        name: name.to_owned(),
        path: format!("/repos/{name}"),
        language: "rust".to_owned(),
        parser: "syn".to_owned(),
        role,
        derives_from: None,
    }
}

async fn upsert_embedding(embed: &EmbedStore, repo_: &str, id: &str, v: &[f32]) {
    embed.upsert(repo_, id, MODEL, v, "h").await.unwrap();
}

#[test]
fn role_weights_are_ordered() {
    assert!(role_weight("engine") > role_weight("library"));
    assert!(role_weight("library") > role_weight("downstream"));
    assert!(role_weight("downstream") > role_weight("sandbox"));
    assert_eq!(role_weight("garbage"), 0.6);
}

#[test]
fn cosine_with_norm_handles_zero() {
    assert_eq!(cosine_with_norm(&[0.0, 0.0], &[1.0, 0.0], 0.0, 1.0), 0.0);
}

#[tokio::test]
async fn empty_graph_returns_empty() {
    let (fleet, embed, _g, _t) = setup().await;
    let out = find_rot(&fleet, &embed, MODEL, 0.3, None, None)
        .await
        .unwrap();
    assert!(out.is_empty());
}

#[tokio::test]
async fn unreachable_with_low_cosine_is_ranked() {
    let (fleet, embed, graph, _t) = setup().await;
    fleet
        .add_repo(&repo("engine", RepoRole::Engine))
        .await
        .unwrap();

    // dead: no inbound edges, orthogonal embedding
    graph
        .upsert_node(&item("dead", "dead_fn", "engine"))
        .await
        .unwrap();
    // alive: another item that "uses" itself + has a high cosine pair
    graph
        .upsert_node(&item("alive_a", "alive_a", "engine"))
        .await
        .unwrap();
    graph
        .upsert_node(&item("alive_b", "alive_b", "engine"))
        .await
        .unwrap();

    use convergio_graph::{Edge, EdgeKind};
    graph
        .upsert_edge(&Edge {
            src: "alive_a".into(),
            dst: "alive_b".into(),
            kind: EdgeKind::Uses,
            weight: 1,
        })
        .await
        .unwrap();

    upsert_embedding(&embed, "engine", "dead", &[1.0, 0.0, 0.0, 0.0]).await;
    upsert_embedding(&embed, "engine", "alive_a", &[0.0, 1.0, 0.0, 0.0]).await;
    upsert_embedding(&embed, "engine", "alive_b", &[0.0, 0.99, 0.14, 0.0]).await;

    let out = find_rot(&fleet, &embed, MODEL, 0.3, None, None)
        .await
        .unwrap();
    assert!(!out.is_empty(), "expected at least one rot candidate");
    assert_eq!(out[0].node_id, "dead", "dead_fn must rank first");
    assert_eq!(out[0].inbound_uses, 0);
    assert!(out[0].best_similar_score < 0.3);
    assert!(out[0].confidence > 0.0);
    assert_eq!(out[0].role, "engine");
}

#[tokio::test]
async fn threshold_filters_above() {
    let (fleet, embed, graph, _t) = setup().await;
    fleet
        .add_repo(&repo("engine", RepoRole::Engine))
        .await
        .unwrap();
    graph.upsert_node(&item("a", "a", "engine")).await.unwrap();
    graph.upsert_node(&item("b", "b", "engine")).await.unwrap();
    // identical vectors → cosine ≈ 1 (well above 0.3 threshold)
    upsert_embedding(&embed, "engine", "a", &[1.0, 0.0]).await;
    upsert_embedding(&embed, "engine", "b", &[1.0, 0.0]).await;

    let out = find_rot(&fleet, &embed, MODEL, 0.3, None, None)
        .await
        .unwrap();
    assert!(
        out.is_empty(),
        "both nodes share cosine ≈ 1, should be filtered out"
    );
}

#[tokio::test]
async fn inbound_uses_suppresses_candidate() {
    let (fleet, embed, graph, _t) = setup().await;
    fleet
        .add_repo(&repo("engine", RepoRole::Engine))
        .await
        .unwrap();
    graph
        .upsert_node(&item("used", "used", "engine"))
        .await
        .unwrap();
    graph
        .upsert_node(&item("caller", "caller", "engine"))
        .await
        .unwrap();

    use convergio_graph::{Edge, EdgeKind};
    graph
        .upsert_edge(&Edge {
            src: "caller".into(),
            dst: "used".into(),
            kind: EdgeKind::Uses,
            weight: 1,
        })
        .await
        .unwrap();

    upsert_embedding(&embed, "engine", "used", &[1.0, 0.0]).await;
    upsert_embedding(&embed, "engine", "caller", &[0.0, 1.0]).await;

    let out = find_rot(&fleet, &embed, MODEL, 0.3, None, None)
        .await
        .unwrap();
    assert!(
        out.iter().all(|c| c.node_id != "used"),
        "node with inbound uses must not appear: {out:?}"
    );
}

#[tokio::test]
async fn engine_role_outranks_sandbox() {
    let (fleet, embed, graph, _t) = setup().await;
    fleet
        .add_repo(&repo("eng", RepoRole::Engine))
        .await
        .unwrap();
    fleet
        .add_repo(&repo("sbx", RepoRole::Sandbox))
        .await
        .unwrap();
    graph
        .upsert_node(&item("eng-dead", "eng_dead", "eng"))
        .await
        .unwrap();
    graph
        .upsert_node(&item("sbx-dead", "sbx_dead", "sbx"))
        .await
        .unwrap();
    upsert_embedding(&embed, "eng", "eng-dead", &[1.0, 0.0]).await;
    upsert_embedding(&embed, "sbx", "sbx-dead", &[0.0, 1.0]).await;

    let out = find_rot(&fleet, &embed, MODEL, 0.3, None, None)
        .await
        .unwrap();
    assert_eq!(out.len(), 2);
    let eng = out.iter().find(|c| c.repo == "eng").unwrap();
    let sbx = out.iter().find(|c| c.repo == "sbx").unwrap();
    assert!(
        eng.confidence > sbx.confidence,
        "engine confidence {} must exceed sandbox confidence {}",
        eng.confidence,
        sbx.confidence
    );
}

#[tokio::test]
async fn explain_returns_node_even_if_not_flagged() {
    let (fleet, embed, graph, _t) = setup().await;
    fleet
        .add_repo(&repo("engine", RepoRole::Engine))
        .await
        .unwrap();
    graph
        .upsert_node(&item("alive", "alive", "engine"))
        .await
        .unwrap();
    graph
        .upsert_node(&item("twin", "twin", "engine"))
        .await
        .unwrap();
    use convergio_graph::{Edge, EdgeKind};
    graph
        .upsert_edge(&Edge {
            src: "twin".into(),
            dst: "alive".into(),
            kind: EdgeKind::Uses,
            weight: 1,
        })
        .await
        .unwrap();
    upsert_embedding(&embed, "engine", "alive", &[1.0, 0.0]).await;
    upsert_embedding(&embed, "engine", "twin", &[1.0, 0.0]).await;

    let out = find_rot(&fleet, &embed, MODEL, 0.3, None, Some("alive"))
        .await
        .unwrap();
    assert_eq!(out.len(), 1, "explain returns exactly the requested node");
    assert_eq!(out[0].node_id, "alive");
    assert_eq!(out[0].inbound_uses, 1);
    assert_eq!(out[0].confidence, 0.0, "inbound>0 ⇒ confidence pinned to 0");
    assert!(out[0].reasons.iter().any(|r| r.contains("explain")));
}

#[tokio::test]
async fn repo_filter_scopes_results() {
    let (fleet, embed, graph, _t) = setup().await;
    fleet.add_repo(&repo("a", RepoRole::Engine)).await.unwrap();
    fleet.add_repo(&repo("b", RepoRole::Engine)).await.unwrap();
    graph.upsert_node(&item("a-dead", "x", "a")).await.unwrap();
    graph.upsert_node(&item("b-dead", "y", "b")).await.unwrap();
    upsert_embedding(&embed, "a", "a-dead", &[1.0, 0.0]).await;
    upsert_embedding(&embed, "b", "b-dead", &[0.0, 1.0]).await;

    let out = find_rot(&fleet, &embed, MODEL, 0.3, Some("a"), None)
        .await
        .unwrap();
    assert_eq!(out.len(), 1);
    assert_eq!(out[0].repo, "a");
}
