//! End-to-end test for [`convergio_fleet::find_rot`] (ADR-0038, F3-5).
//!
//! Boots a tempdir SQLite, migrates the fleet + embed + graph stores,
//! seeds a representative graph (one unreachable + orthogonal item,
//! one used pair), and verifies the unreachable item ranks first.
//! Lives here rather than under `convergio-server/tests/` so the
//! server crate stays under its per-crate context budget cap; the
//! HTTP route is a thin wrapper that does no extra logic.

use convergio_embed::EmbedStore;
use convergio_fleet::config::{RepoEntry, RepoRole};
use convergio_fleet::{find_rot, FleetStore};
use convergio_graph::{Edge, EdgeKind, Node, NodeKind, Store as GraphStore};

const MODEL: &str = "test-model";

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

#[tokio::test]
async fn fleet_rot_ranks_unreachable_with_low_cosine() {
    let tmp = tempfile::NamedTempFile::new().unwrap();
    let url = format!("sqlite://{}", tmp.path().display());
    let pool = convergio_db::Pool::connect(&url).await.unwrap();
    convergio_fleet::init(&pool).await.unwrap();
    convergio_embed::init(&pool).await.unwrap();
    let graph = GraphStore::new(pool.clone());
    graph.migrate().await.unwrap();
    let fleet = FleetStore::new(pool.clone());
    let embed = EmbedStore::new(pool);

    fleet
        .add_repo(&RepoEntry {
            name: "engine".into(),
            path: "/repos/engine".into(),
            language: "rust".into(),
            parser: "syn".into(),
            role: RepoRole::Engine,
            derives_from: None,
        })
        .await
        .unwrap();

    for n in [
        item("dead", "dead_fn", "engine"),
        item("alive_a", "alive_a", "engine"),
        item("alive_b", "alive_b", "engine"),
    ] {
        graph.upsert_node(&n).await.unwrap();
    }
    graph
        .upsert_edge(&Edge {
            src: "alive_a".into(),
            dst: "alive_b".into(),
            kind: EdgeKind::Uses,
            weight: 1,
        })
        .await
        .unwrap();

    let vecs: [(&str, &[f32]); 3] = [
        ("dead", &[1.0, 0.0, 0.0, 0.0]),
        ("alive_a", &[0.0, 1.0, 0.0, 0.0]),
        ("alive_b", &[0.0, 0.99, 0.14, 0.0]),
    ];
    for (id, v) in vecs {
        embed.upsert("engine", id, MODEL, v, "h").await.unwrap();
    }

    let out = find_rot(&fleet, &embed, MODEL, 0.3, None, None)
        .await
        .unwrap();
    assert!(!out.is_empty(), "expected dead candidate, got: {out:?}");
    assert_eq!(out[0].node_id, "dead");
    assert_eq!(out[0].inbound_uses, 0);
    assert!(out[0].best_similar_score < 0.3);
    assert_eq!(out[0].role, "engine");
    assert!(out[0].confidence > 0.0);
}

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

// Codex P1 regression (PR #380 review): a node with no embedding row
// must not appear as a strong rot candidate. Without similarity data
// we cannot tell isolation from "simply unscored". `--explain` still
// surfaces it with a "no embedding stored" reason.
#[tokio::test]
async fn missing_embedding_is_not_flagged() {
    let (fleet, embed, graph, _t) = boot().await;
    fleet.add_repo(&repo("engine")).await.unwrap();
    graph
        .upsert_node(&item("unembedded", "u", "engine"))
        .await
        .unwrap();

    let out = find_rot(&fleet, &embed, MODEL, 0.3, None, None)
        .await
        .unwrap();
    assert!(
        out.iter().all(|c| c.node_id != "unembedded"),
        "unembedded node must not be flagged: {out:?}"
    );

    let explained = find_rot(&fleet, &embed, MODEL, 0.3, None, Some("unembedded"))
        .await
        .unwrap();
    assert_eq!(explained.len(), 1);
    assert!(explained[0]
        .reasons
        .iter()
        .any(|r| r.contains("no embedding stored")));
}

// Codex P1 regression (PR #380 review): the best-similarity cache key
// must include repo. If two repos share a node_id (path-like IDs,
// non-hash-derived), scores must not cross-contaminate.
#[tokio::test]
async fn same_node_id_across_repos_keeps_separate_scores() {
    let (fleet, embed, graph, _t) = boot().await;
    fleet.add_repo(&repo("a")).await.unwrap();
    fleet.add_repo(&repo("b")).await.unwrap();
    graph
        .upsert_node(&item("a-shared", "f", "a"))
        .await
        .unwrap();
    graph
        .upsert_node(&item("b-shared", "f", "b"))
        .await
        .unwrap();
    // Distinct repo, identical short id — must not collide in the cache.
    embed
        .upsert("a", "shared", MODEL, &[1.0, 0.0], "h")
        .await
        .unwrap();
    embed
        .upsert("b", "shared", MODEL, &[1.0, 0.0], "h")
        .await
        .unwrap();

    let out = find_rot(&fleet, &embed, MODEL, 0.3, None, None)
        .await
        .unwrap();
    assert!(
        out.is_empty(),
        "no graph node has matching embedding rows: {out:?}"
    );
}
