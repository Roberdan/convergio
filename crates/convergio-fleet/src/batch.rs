//! Cross-repo similarity batch job (ADR-0038, F2-8).
//!
//! [`run_similarity_batch`] reads all embeddings for a given model,
//! computes cosine similarity for every cross-repo pair, and upserts
//! edges into `fleet_similar_edges`.  It is idempotent: the table is
//! cleared before each run so stale edges do not accumulate.

use crate::error::Result;
use crate::similar::{DUPLICATES_THRESHOLD, SIMILAR_TO_THRESHOLD};
use crate::store::FleetStore;
use convergio_embed::EmbedStore;
use sqlx::Row;
use std::collections::HashMap;

/// Summary of one similarity batch run.
#[derive(Debug, Clone, Default)]
pub struct BatchReport {
    /// Cross-repo pairs evaluated.
    pub pairs_checked: u64,
    /// `similar_to` edges written.
    pub similar_to: u64,
    /// `duplicates` edges written.
    pub duplicates: u64,
}

/// Run the cross-repo similarity batch over all embeddings for `model`.
///
/// For each pair of embeddings from **distinct** repos:
/// - cosine ≥ [`SIMILAR_TO_THRESHOLD`] (0.85) → `similar_to` edge
/// - cosine ≥ [`DUPLICATES_THRESHOLD`] (0.95) **and** both nodes share
///   the same structural `kind` in `graph_nodes` → `duplicates` edge
///
/// Edges are persisted in `fleet_similar_edges` with
/// `weight = round(cosine × 1000)`.  The table is cleared first, so
/// re-runs are fully idempotent.
pub async fn run_similarity_batch(
    embed_store: &EmbedStore,
    fleet_store: &FleetStore,
    model: &str,
) -> Result<BatchReport> {
    fleet_store.clear_similar_edges().await?;

    let rows = embed_store.all_for_model(model).await?;
    if rows.is_empty() {
        return Ok(BatchReport::default());
    }

    let node_kinds = load_node_kinds(fleet_store, &rows).await?;

    let mut report = BatchReport::default();
    for (i, (repo_a, node_a, vec_a)) in rows.iter().enumerate() {
        for (repo_b, node_b, vec_b) in &rows[i + 1..] {
            if repo_a == repo_b {
                continue;
            }
            report.pairs_checked += 1;
            let score = cosine_sim(vec_a, vec_b);
            if score < SIMILAR_TO_THRESHOLD {
                continue;
            }
            let kind = classify(score, node_a, node_b, &node_kinds);
            fleet_store
                .upsert_similar_edge_classified(repo_a, node_a, repo_b, node_b, score, kind)
                .await?;
            if kind == "duplicates" {
                report.duplicates += 1;
            } else {
                report.similar_to += 1;
            }
        }
    }
    Ok(report)
}

/// Classify a cross-repo pair: `"duplicates"` when cosine ≥ threshold
/// and both nodes share the same structural kind; `"similar_to"` otherwise.
fn classify<'s>(
    score: f32,
    node_a: &str,
    node_b: &str,
    node_kinds: &HashMap<String, String>,
) -> &'s str {
    if score >= DUPLICATES_THRESHOLD {
        let ka = node_kinds.get(node_a);
        let kb = node_kinds.get(node_b);
        if ka.is_some() && ka == kb {
            return "duplicates";
        }
    }
    "similar_to"
}

/// Load `(id → kind)` from `graph_nodes` for every node in `rows`.
///
/// Queries the shared SQLite pool in chunks of 500 to stay well under
/// the SQLite variable limit.
async fn load_node_kinds(
    fleet_store: &FleetStore,
    rows: &[(String, String, Vec<f32>)],
) -> Result<HashMap<String, String>> {
    let ids: Vec<&str> = rows.iter().map(|(_, id, _)| id.as_str()).collect();
    let mut out = HashMap::with_capacity(ids.len());

    for chunk in ids.chunks(500) {
        let placeholders = chunk.iter().map(|_| "?").collect::<Vec<_>>().join(",");
        let sql = format!("SELECT id, kind FROM graph_nodes WHERE id IN ({placeholders})");
        let mut q = sqlx::query(&sql);
        for id in chunk {
            q = q.bind(*id);
        }
        let db_rows = q
            .fetch_all(fleet_store.pool().inner())
            .await
            .map_err(crate::error::FleetError::Db)?;
        for row in db_rows {
            let id: String = row.get("id");
            let kind: String = row.get("kind");
            out.insert(id, kind);
        }
    }
    Ok(out)
}

/// Cosine similarity between two vectors.  Returns 0.0 for zero vectors
/// or mismatched dimensions.
fn cosine_sim(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() || a.is_empty() {
        return 0.0;
    }
    let dot: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let na: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let nb: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
    if na == 0.0 || nb == 0.0 {
        return 0.0;
    }
    dot / (na * nb)
}

#[cfg(test)]
mod tests {
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
}
