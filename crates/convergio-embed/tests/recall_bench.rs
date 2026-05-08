//! Recall benchmark — substring vs semantic vs RRF-fused retrieval
//! on the golden fixture set (ADR-0038 § 7 / golden methodology).
//!
//! Three retrievers are evaluated against the same corpus + query:
//! - **substring** — token-overlap count (lower-cased) over each
//!   file's first N lines. This is the baseline `convergio-graph`
//!   approximates with its static-score path; reimplemented here so
//!   the bench stays self-contained.
//! - **semantic** — `convergio_embed::semantic_search`.
//! - **hybrid** — `rrf_fuse(substring_top_k, semantic_top_k)` with
//!   `k = 60`.
//!
//! Reports recall@10 per branch and the lift hybrid/substring.
//!
//! Run with:
//!   cargo test -p convergio-embed --test recall_bench --release \
//!       -- --ignored --nocapture
//!
//! For real-model numbers, build with `--features fastembed` and set
//! `CONVERGIO_BENCH_MODEL=multilingual-e5-small`.

#![allow(clippy::expect_used)]

use convergio_db::Pool;
use convergio_embed::embedder::testing::DeterministicTestEmbedder;
use convergio_embed::{
    collect_files, ingest, rrf_fuse, semantic_search, Embedder, IngestNode, DEFAULT_RRF_K,
    SOURCE_EXTENSIONS,
};
use serde::Deserialize;
use std::path::{Path, PathBuf};
use tempfile::tempdir;

#[derive(Debug, Deserialize)]
struct Fixture {
    task_id: String,
    title: String,
    task_body: String,
    expected_files: Vec<String>,
}

fn fixture_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("tests/fixtures/retrieval-golden/convergio")
}

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("workspace root resolves")
}

fn load_fixtures() -> Vec<Fixture> {
    let dir = fixture_root();
    if !dir.is_dir() {
        return Vec::new();
    }
    let mut out = Vec::new();
    for entry in std::fs::read_dir(&dir).expect("read fixture dir") {
        let entry = entry.expect("dir entry");
        if entry.path().extension().and_then(|e| e.to_str()) != Some("json") {
            continue;
        }
        let raw = std::fs::read_to_string(entry.path()).expect("read fixture");
        let fx: Fixture = serde_json::from_str(&raw).expect("parse fixture");
        out.push(fx);
    }
    out.sort_by(|a, b| a.task_id.cmp(&b.task_id));
    out
}

/// Token-overlap baseline. Approximates `convergio-graph`'s
/// substring static-score path: lower-case query, split into tokens
/// of length ≥ 3, count matches in each file's source, return top-K
/// node ids by descending count (stable tie-break by node_id).
fn substring_rank(corpus: &[IngestNode], query: &str, k: usize) -> Vec<String> {
    let tokens: Vec<String> = query
        .split(|c: char| !c.is_alphanumeric())
        .filter(|t| t.len() >= 3)
        .map(|t| t.to_ascii_lowercase())
        .collect();
    let mut scored: Vec<(&str, usize)> = corpus
        .iter()
        .map(|n| {
            let body = n.source.to_ascii_lowercase();
            let count = tokens.iter().filter(|t| body.contains(t.as_str())).count();
            (n.node_id.as_str(), count)
        })
        .filter(|(_, c)| *c > 0)
        .collect();
    scored.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(b.0)));
    scored
        .iter()
        .take(k)
        .map(|(p, _)| (*p).to_string())
        .collect()
}

fn recall_at_k(retrieved: &[String], expected: &[String], k: usize) -> f64 {
    if expected.is_empty() {
        return 0.0;
    }
    let topk: std::collections::HashSet<&String> = retrieved.iter().take(k).collect();
    let hit = expected.iter().filter(|e| topk.contains(*e)).count();
    hit as f64 / expected.len() as f64
}

#[tokio::test]
#[ignore = "slow corpus walk + ingest; opt-in via --ignored"]
async fn recall_bench_substring_vs_hybrid() {
    let fixtures = load_fixtures();
    if fixtures.is_empty() {
        eprintln!("no fixtures under {}", fixture_root().display());
        return;
    }

    let dir = tempdir().expect("tempdir");
    let url = format!(
        "sqlite://{}?mode=rwc",
        dir.path().join("state.db").display()
    );
    let pool = Pool::connect(&url).await.expect("connect");
    convergio_embed::init(&pool).await.expect("migrate");
    let store = convergio_embed::EmbedStore::new(pool);
    let embedder = make_embedder();

    let workspace = workspace_root();
    let nodes: Vec<IngestNode> = collect_files("convergio", &workspace, SOURCE_EXTENSIONS, 200);
    eprintln!(
        "corpus: {} files under {}",
        nodes.len(),
        workspace.display()
    );
    let started_ingest = std::time::Instant::now();
    let report = ingest(&store, embedder.as_ref(), nodes.clone())
        .await
        .expect("ingest");
    eprintln!(
        "ingest: {report:?} in {:.1}s",
        started_ingest.elapsed().as_secs_f64()
    );

    let mut substring_total = 0.0;
    let mut semantic_total = 0.0;
    let mut hybrid_total = 0.0;
    let mut substring_ms_total = 0u128;
    let mut semantic_ms_total = 0u128;
    for fx in &fixtures {
        let query = format!("{}\n{}", fx.title, fx.task_body);

        let s = std::time::Instant::now();
        let substring_hits = substring_rank(&nodes, &query, 25);
        substring_ms_total += s.elapsed().as_millis();

        let s = std::time::Instant::now();
        let semantic = match semantic_search(&store, embedder.as_ref(), &query, 25).await {
            Ok(v) => v,
            Err(convergio_embed::EmbedError::EmbedderFailed(msg)) => {
                eprintln!(
                    "warning: semantic degraded to structural-only (model={}) : {msg}",
                    embedder.model_id()
                );
                Vec::new()
            }
            Err(e) => panic!("semantic_search failed: {e}"),
        };
        semantic_ms_total += s.elapsed().as_millis();
        let semantic_ids: Vec<String> = semantic.iter().map(|n| n.node_id.clone()).collect();

        let hybrid = rrf_fuse(&substring_hits, &semantic_ids, DEFAULT_RRF_K);
        let hybrid_ids: Vec<String> = hybrid.iter().map(|h| h.id.clone()).collect();

        let r_sub = recall_at_k(&substring_hits, &fx.expected_files, 10);
        let r_sem = recall_at_k(&semantic_ids, &fx.expected_files, 10);
        let r_hyb = recall_at_k(&hybrid_ids, &fx.expected_files, 10);
        eprintln!(
            "  fixture {:<40} substring={r_sub:.3} semantic={r_sem:.3} hybrid={r_hyb:.3}",
            fx.task_id
        );
        substring_total += r_sub;
        semantic_total += r_sem;
        hybrid_total += r_hyb;
    }
    let n = fixtures.len() as f64;
    let sub = substring_total / n;
    let sem = semantic_total / n;
    let hyb = hybrid_total / n;
    eprintln!(
        "F1-bench: fixtures={} model={} corpus={} | recall@10 substring={sub:.3} semantic={sem:.3} hybrid={hyb:.3} | lift hybrid-vs-substring={:+.3} | latency_ms substring_total={substring_ms_total} semantic_total={semantic_ms_total}",
        fixtures.len(),
        embedder.model_id(),
        nodes.len(),
        hyb - sub
    );
}

fn make_embedder() -> Box<dyn Embedder> {
    let model = std::env::var("CONVERGIO_BENCH_MODEL").unwrap_or_default();
    match model.as_str() {
        #[cfg(feature = "fastembed")]
        "multilingual-e5-small" => {
            let cache =
                std::path::PathBuf::from(std::env::var("HOME").unwrap_or_else(|_| ".".into()))
                    .join(".convergio/v3/models");
            Box::new(convergio_embed::MultilingualE5Embedder::new(cache))
        }
        #[cfg(feature = "fastembed")]
        "bge-m3" => {
            let cache =
                std::path::PathBuf::from(std::env::var("HOME").unwrap_or_else(|_| ".".into()))
                    .join(".convergio/v3/models");
            Box::new(convergio_embed::BgeM3Embedder::new(cache))
        }
        #[cfg(feature = "fastembed")]
        "bge-m3-small" | "bge-m3-small-int8" => {
            let cache =
                std::path::PathBuf::from(std::env::var("HOME").unwrap_or_else(|_| ".".into()))
                    .join(".convergio/v3/models");
            Box::new(convergio_embed::MultilingualE5Embedder::new(cache))
        }
        _ => Box::new(DeterministicTestEmbedder::new(384)),
    }
}
