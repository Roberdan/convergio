//! Cross-language fixture integration tests (F2-11).
//!
//! Uses three mini-repos under `tests/fixtures/fleet/` (plan-fsm-rs,
//! plan-fsm-ts, plan-fsm-py) as ground-truth for cluster and
//! duplicate-detection assertions.  Embeddings are synthetic: crafted
//! so all FSM module pairs have cosine > 0.95 ("duplicates") while
//! unrelated CLI-runner nodes remain orthogonal (no edges produced).

use convergio_embed::EmbedStore;
use convergio_fleet::{
    config::{RepoEntry, RepoRole},
    find_duplicates, find_patterns, init, run_similarity_batch, FleetStore,
};
use convergio_graph::{Node, NodeKind, Store as GraphStore};
use tempfile::NamedTempFile;

const FIXTURE_ROOT: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../tests/fixtures/fleet");

const RS_FSM: &str = "rs-plan-fsm-module";
const TS_FSM: &str = "ts-plan-fsm-module";
const PY_FSM: &str = "py-plan-fsm-module";
const RS_CLI: &str = "rs-cli-runner-item";
const TS_CLI: &str = "ts-cli-runner-item";
const PY_CLI: &str = "py-cli-runner-item";

// FSM vectors cluster tightly (cosine > 0.95 across all three pairs).
// CLI vectors are mutually orthogonal — no edge crosses the 0.85 threshold.
const RS_FSM_VEC: [f32; 4] = [1.0, 0.0, 0.0, 0.0];
const TS_FSM_VEC: [f32; 4] = [0.999, 0.0447, 0.0, 0.0];
const PY_FSM_VEC: [f32; 4] = [0.999, -0.0447, 0.0, 0.0];
const RS_CLI_VEC: [f32; 4] = [0.0, 1.0, 0.0, 0.0];
const TS_CLI_VEC: [f32; 4] = [0.0, 0.0, 1.0, 0.0];
const PY_CLI_VEC: [f32; 4] = [0.0, 0.0, 0.0, 1.0];

async fn setup() -> (FleetStore, EmbedStore, GraphStore, NamedTempFile) {
    let tmp = NamedTempFile::new().unwrap();
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

fn fixture_repo(name: &str, language: &str, parser: &str) -> RepoEntry {
    RepoEntry {
        name: name.to_owned(),
        path: format!("{FIXTURE_ROOT}/{name}"),
        language: language.to_owned(),
        parser: parser.to_owned(),
        role: RepoRole::Downstream,
        derives_from: None,
    }
}

fn make_node(id: &str, kind: NodeKind, name: &str, repo: &str) -> Node {
    Node {
        id: id.to_owned(),
        kind,
        name: name.to_owned(),
        file_path: None,
        crate_name: repo.to_owned(),
        repo: repo.to_owned(),
        item_kind: None,
        span: None,
    }
}

async fn seed_three_repos(fleet: &FleetStore) {
    for (name, lang, parser) in [
        ("plan-fsm-rs", "rust", "syn"),
        ("plan-fsm-ts", "typescript", "swc"),
        ("plan-fsm-py", "python", "ast"),
    ] {
        fleet
            .add_repo(&fixture_repo(name, lang, parser))
            .await
            .unwrap();
    }
}

async fn seed_six_nodes(graph: &GraphStore) {
    for (id, kind, name, repo) in [
        (RS_FSM, NodeKind::Module, "plan_fsm", "plan-fsm-rs"),
        (TS_FSM, NodeKind::Module, "plan_fsm", "plan-fsm-ts"),
        (PY_FSM, NodeKind::Module, "plan_fsm", "plan-fsm-py"),
        (RS_CLI, NodeKind::Item, "cli_runner", "plan-fsm-rs"),
        (TS_CLI, NodeKind::Item, "cli_runner", "plan-fsm-ts"),
        (PY_CLI, NodeKind::Item, "cli_runner", "plan-fsm-py"),
    ] {
        graph
            .upsert_node(&make_node(id, kind, name, repo))
            .await
            .unwrap();
    }
}

async fn seed_six_embeddings(embed: &EmbedStore) {
    for (repo, id, vec) in [
        ("plan-fsm-rs", RS_FSM, RS_FSM_VEC.as_slice()),
        ("plan-fsm-ts", TS_FSM, TS_FSM_VEC.as_slice()),
        ("plan-fsm-py", PY_FSM, PY_FSM_VEC.as_slice()),
        ("plan-fsm-rs", RS_CLI, RS_CLI_VEC.as_slice()),
        ("plan-fsm-ts", TS_CLI, TS_CLI_VEC.as_slice()),
        ("plan-fsm-py", PY_CLI, PY_CLI_VEC.as_slice()),
    ] {
        embed
            .upsert(repo, id, "test-model", vec, "h")
            .await
            .unwrap();
    }
}

async fn seed_dup(fleet: &FleetStore, ra: &str, na: &str, rb: &str, nb: &str, score: f32) {
    fleet
        .upsert_similar_edge_classified(ra, na, rb, nb, score, "duplicates")
        .await
        .unwrap();
}

async fn seed_three_fsm_edges(fleet: &FleetStore) {
    seed_dup(fleet, "plan-fsm-rs", RS_FSM, "plan-fsm-ts", TS_FSM, 0.999).await;
    seed_dup(fleet, "plan-fsm-rs", RS_FSM, "plan-fsm-py", PY_FSM, 0.999).await;
    seed_dup(fleet, "plan-fsm-ts", TS_FSM, "plan-fsm-py", PY_FSM, 0.996).await;
}

#[tokio::test]
async fn fixture_dirs_exist() {
    let root = std::path::Path::new(FIXTURE_ROOT);
    assert!(
        root.join("plan-fsm-rs/src/lib.rs").exists(),
        "plan-fsm-rs fixture missing"
    );
    assert!(
        root.join("plan-fsm-ts/src/plan_fsm.ts").exists(),
        "plan-fsm-ts fixture missing"
    );
    assert!(
        root.join("plan-fsm-py/plan_fsm.py").exists(),
        "plan-fsm-py fixture missing"
    );
}

#[tokio::test]
async fn batch_produces_three_duplicate_pairs_for_fsm_nodes() {
    let (fleet, embed, graph, _tmp) = setup().await;

    seed_three_repos(&fleet).await;
    seed_six_nodes(&graph).await;
    seed_six_embeddings(&embed).await;

    let report = run_similarity_batch(&embed, &fleet, "test-model")
        .await
        .unwrap();

    assert_eq!(report.duplicates, 3, "expected 3 duplicate FSM pairs");
    assert_eq!(report.similar_to, 0, "CLI nodes must not form any edges");
    assert_eq!(
        fleet.count_similar_edges(Some("duplicates")).await.unwrap(),
        3
    );
    assert_eq!(
        fleet.count_similar_edges(Some("similar_to")).await.unwrap(),
        0
    );
}

#[tokio::test]
async fn patterns_clusters_fsm_across_three_languages() {
    let (fleet, _embed, _graph, _tmp) = setup().await;
    seed_three_fsm_edges(&fleet).await;

    let clusters = find_patterns(&fleet, 3).await.unwrap();

    assert_eq!(clusters.len(), 1, "expected exactly one FSM cluster");
    assert_eq!(
        clusters[0].members.len(),
        3,
        "cluster must span all three languages"
    );

    let repos: Vec<&str> = clusters[0]
        .members
        .iter()
        .map(|m| m.repo.as_str())
        .collect();
    for r in ["plan-fsm-rs", "plan-fsm-ts", "plan-fsm-py"] {
        assert!(repos.contains(&r), "{r} missing from cluster");
    }

    assert!(
        clusters[0].confidence > 0.95,
        "confidence should reflect high similarity, got {}",
        clusters[0].confidence
    );
}

#[tokio::test]
async fn duplicates_detects_all_three_language_pairs() {
    let (fleet, _embed, _graph, _tmp) = setup().await;
    seed_three_fsm_edges(&fleet).await;

    let pairs = find_duplicates(&fleet, 0.95, None, false).await.unwrap();
    assert_eq!(
        pairs.len(),
        3,
        "all three cross-language FSM pairs must be detected"
    );

    for p in &pairs {
        assert_ne!(p.repo_a, p.repo_b, "intra-repo edge must not appear");
        assert!(p.score > 0.95, "score below threshold: {}", p.score);
    }
}

#[tokio::test]
async fn two_repo_cluster_filtered_by_min_repos() {
    let (fleet, _embed, _graph, _tmp) = setup().await;
    seed_dup(&fleet, "plan-fsm-rs", RS_FSM, "plan-fsm-ts", TS_FSM, 0.999).await;

    let clusters = find_patterns(&fleet, 3).await.unwrap();
    assert!(
        clusters.is_empty(),
        "a 2-repo cluster must be filtered when min_repos=3"
    );
}
