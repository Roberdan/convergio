use super::*;
use crate::config::{RepoEntry, RepoRole};
use crate::migrate::init;

async fn setup() -> (FleetStore, EmbedStore, tempfile::NamedTempFile) {
    let tmp = tempfile::NamedTempFile::new().unwrap();
    let url = format!("sqlite://{}", tmp.path().display());
    let pool = convergio_db::Pool::connect(&url).await.unwrap();
    init(&pool).await.unwrap();
    convergio_embed::init(&pool).await.unwrap();
    // graph_nodes table is needed for structural shape lookups
    convergio_graph::Store::new(pool.clone())
        .migrate()
        .await
        .unwrap();
    let fleet = FleetStore::new(pool.clone());
    let embed = EmbedStore::new(pool);
    (fleet, embed, tmp)
}

fn repo_entry(name: &str) -> RepoEntry {
    RepoEntry {
        name: name.to_owned(),
        path: format!("/tmp/{name}"),
        language: "rust".to_owned(),
        parser: "syn".to_owned(),
        role: RepoRole::Engine,
        derives_from: None,
    }
}

#[tokio::test]
async fn empty_store_returns_zero_report() {
    let (fleet, embed, _tmp) = setup().await;
    fleet.add_repo(&repo_entry("a")).await.unwrap();
    fleet.add_repo(&repo_entry("b")).await.unwrap();
    let r = run_similarity_batch(&embed, &fleet, "m").await.unwrap();
    assert_eq!(r.pairs_checked, 0);
    assert_eq!(r.similar_to, 0);
    assert_eq!(r.duplicates, 0);
}

#[tokio::test]
async fn cross_repo_above_threshold_emitted() {
    let (fleet, embed, _tmp) = setup().await;
    // dim=4, cosine ~ 0.997 (well above 0.85)
    let a: Vec<f32> = vec![1.0, 0.0, 0.0, 0.0];
    let b: Vec<f32> = vec![0.99, 0.14, 0.0, 0.0];
    embed.upsert("alpha", "n1", "m", &a, "ha").await.unwrap();
    embed.upsert("beta", "n2", "m", &b, "hb").await.unwrap();
    let r = run_similarity_batch(&embed, &fleet, "m").await.unwrap();
    assert_eq!(r.pairs_checked, 1);
    assert_eq!(r.similar_to + r.duplicates, 1);
    assert_eq!(fleet.count_similar_edges(None).await.unwrap(), 1);
}

#[tokio::test]
async fn same_repo_pairs_skipped() {
    let (fleet, embed, _tmp) = setup().await;
    let v: Vec<f32> = vec![1.0, 0.0, 0.0, 0.0];
    embed.upsert("alpha", "n1", "m", &v, "h1").await.unwrap();
    embed.upsert("alpha", "n2", "m", &v, "h2").await.unwrap();
    let r = run_similarity_batch(&embed, &fleet, "m").await.unwrap();
    assert_eq!(r.pairs_checked, 0);
    assert_eq!(fleet.count_similar_edges(None).await.unwrap(), 0);
}

#[tokio::test]
async fn below_threshold_not_emitted() {
    let (fleet, embed, _tmp) = setup().await;
    let a: Vec<f32> = vec![1.0, 0.0, 0.0, 0.0];
    let b: Vec<f32> = vec![0.0, 1.0, 0.0, 0.0]; // cosine = 0
    embed.upsert("alpha", "n1", "m", &a, "ha").await.unwrap();
    embed.upsert("beta", "n2", "m", &b, "hb").await.unwrap();
    let r = run_similarity_batch(&embed, &fleet, "m").await.unwrap();
    assert_eq!(r.similar_to, 0);
    assert_eq!(fleet.count_similar_edges(None).await.unwrap(), 0);
}

#[tokio::test]
async fn idempotent_on_rerun() {
    let (fleet, embed, _tmp) = setup().await;
    let a: Vec<f32> = vec![1.0, 0.0, 0.0, 0.0];
    let b: Vec<f32> = vec![0.99, 0.14, 0.0, 0.0];
    embed.upsert("alpha", "n1", "m", &a, "ha").await.unwrap();
    embed.upsert("beta", "n2", "m", &b, "hb").await.unwrap();
    run_similarity_batch(&embed, &fleet, "m").await.unwrap();
    run_similarity_batch(&embed, &fleet, "m").await.unwrap();
    assert_eq!(fleet.count_similar_edges(None).await.unwrap(), 1);
}

#[tokio::test]
async fn weight_stored_as_cosine_times_1000() {
    let (fleet, embed, _tmp) = setup().await;
    // Unit vector along x → unit vector along x: cosine = 1.0, weight = 1000
    let a: Vec<f32> = vec![1.0, 0.0, 0.0, 0.0];
    embed.upsert("r1", "n1", "m", &a, "h1").await.unwrap();
    embed.upsert("r2", "n2", "m", &a, "h2").await.unwrap();
    run_similarity_batch(&embed, &fleet, "m").await.unwrap();
    let edges = fleet.list_similar_edges(1).await.unwrap();
    assert_eq!(edges.len(), 1);
    assert_eq!(edges[0].weight, 1000);
}

#[test]
fn cosine_unit_vectors() {
    let a = vec![1.0f32, 0.0, 0.0];
    let b = vec![1.0f32, 0.0, 0.0];
    assert!((cosine_sim(&a, &b) - 1.0).abs() < 1e-6);
}

#[test]
fn cosine_orthogonal_vectors() {
    let a = vec![1.0f32, 0.0, 0.0];
    let b = vec![0.0f32, 1.0, 0.0];
    assert!((cosine_sim(&a, &b)).abs() < 1e-6);
}

#[test]
fn cosine_zero_vector_returns_zero() {
    let a = vec![0.0f32, 0.0, 0.0];
    let b = vec![1.0f32, 0.0, 0.0];
    assert_eq!(cosine_sim(&a, &b), 0.0);
}

#[test]
fn classify_duplicates_with_matching_kind() {
    let mut kinds = HashMap::new();
    kinds.insert("n1".to_owned(), "module".to_owned());
    kinds.insert("n2".to_owned(), "module".to_owned());
    assert_eq!(classify(0.97, "n1", "n2", &kinds), "duplicates");
}

#[test]
fn classify_similar_to_when_kinds_differ() {
    let mut kinds = HashMap::new();
    kinds.insert("n1".to_owned(), "module".to_owned());
    kinds.insert("n2".to_owned(), "item".to_owned());
    assert_eq!(classify(0.97, "n1", "n2", &kinds), "similar_to");
}

#[test]
fn classify_similar_to_when_kind_missing() {
    let kinds = HashMap::new();
    assert_eq!(classify(0.97, "n1", "n2", &kinds), "similar_to");
}
