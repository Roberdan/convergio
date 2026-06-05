//! Shared helpers for the F2-13 measurement e2e test.
//!
//! Included by `e2e_f2_13_measure.rs` via `#[path = "..."] mod`.

#![allow(dead_code, missing_docs, clippy::expect_used, clippy::unwrap_used)]

use convergio_bus::Bus;
use convergio_db::Pool;
use convergio_durability::{init, Durability};
use convergio_embed::EmbedStore;
use convergio_fleet::FleetStore;
use convergio_lifecycle::Supervisor;
use convergio_server::{router, AppState};
use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tempfile::tempdir;
use tokio::net::TcpListener;

use reqwest::header::{HeaderMap, HeaderValue};

pub const TEST_PURPOSE_ID: &str = "00000000-0000-0000-0000-000000000001";

pub fn client_builder() -> reqwest::ClientBuilder {
    let mut headers = HeaderMap::new();
    headers.insert(
        convergio_api::PURPOSE_ID_HEADER,
        HeaderValue::from_static(TEST_PURPOSE_ID),
    );
    reqwest::Client::builder().default_headers(headers)
}

pub fn client() -> reqwest::Client {
    client_builder().build().expect("reqwest client")
}

#[cfg(feature = "fastembed")]
pub fn make_embedder() -> Arc<dyn convergio_embed::Embedder> {
    use convergio_embed::MultilingualE5Embedder;
    let cache = std::env::var("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("/tmp"))
        .join(".convergio")
        .join("v3")
        .join("models");
    std::fs::create_dir_all(&cache).ok();
    Arc::new(MultilingualE5Embedder::new(cache))
}

#[cfg(not(feature = "fastembed"))]
pub fn make_embedder() -> Arc<dyn convergio_embed::Embedder> {
    use convergio_embed::embedder::testing::DeterministicTestEmbedder;
    Arc::new(DeterministicTestEmbedder::new(384))
}

pub fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("workspace root")
}

pub fn fleet_fixture_root() -> PathBuf {
    workspace_root().join("tests/fixtures/fleet")
}

/// Resolve a repo path: tries the real repo first, falls back to fixture.
pub fn resolve_repo(real_path: &str, fixture_subdir: &str) -> (PathBuf, bool) {
    let real = PathBuf::from(real_path);
    if real.is_dir() {
        (real, true)
    } else {
        (fleet_fixture_root().join(fixture_subdir), false)
    }
}

pub async fn boot_with_embedder(
    embedder: Arc<dyn convergio_embed::Embedder>,
) -> (String, Arc<FleetStore>, Arc<EmbedStore>, tempfile::TempDir) {
    let dir = tempdir().expect("tempdir");
    let url = format!(
        "sqlite://{}?mode=rwc",
        dir.path().join("state.db").display()
    );
    let pool = Pool::connect(&url).await.expect("connect");

    init(&pool).await.expect("durability init");
    convergio_ops::init(&pool).await.expect("ops init");
    convergio_bus::init(&pool).await.expect("bus init");
    convergio_lifecycle::init(&pool)
        .await
        .expect("lifecycle init");
    let graph = Arc::new(convergio_graph::Store::new(pool.clone()));
    graph.migrate().await.expect("graph migrate");
    convergio_embed::init(&pool).await.expect("embed init");
    convergio_fleet::init(&pool).await.expect("fleet init");

    let embed = Arc::new(EmbedStore::new(pool.clone()));
    let fleet = Arc::new(FleetStore::new(pool.clone()));

    let state = AppState {
        durability: Arc::new(Durability::new(pool.clone())),
        ops: Arc::new(convergio_ops::Ops::new(pool.clone())),
        bus: Arc::new(Bus::new(pool.clone())),
        fleet: fleet.clone(),
        fleet_plans: Arc::new(convergio_fleet::FleetPlanStore::new(pool.clone())),
        ontology: Arc::new(convergio_ontology::Store::new(pool.clone())),
        supervisor: Arc::new(Supervisor::new(pool)),
        graph,
        embed: embed.clone(),
        embedder,
        audit_verify_cache: Arc::new(std::sync::Mutex::new(None)),
    };
    let app = router(state);

    let listener = TcpListener::bind(SocketAddr::from(([127, 0, 0, 1], 0)))
        .await
        .expect("bind");
    let addr = listener.local_addr().expect("local addr");
    tokio::spawn(async move {
        axum::serve(listener, app).await.expect("serve");
    });
    (format!("http://{addr}"), fleet, embed, dir)
}

pub async fn register_repo(base: &str, name: &str, path: &str, lang: &str) {
    let resp = client()
        .post(format!("{base}/v1/fleet/repos"))
        .json(&serde_json::json!({
            "name": name,
            "path": path,
            "language": lang,
            "parser": "tree-sitter",
        }))
        .send()
        .await
        .expect("send");
    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        panic!("register_repo {name} failed {status}: {body}");
    }
}

/// Classify a duplicate pair as TRUE POSITIVE when both nodes clearly
/// represent the same concept across repos. TP iff identical normalised
/// names, OR same kind AND ≥50% shared significant tokens (len ≥ 4).
/// Falls back to score ≥ 0.98 when names are too short to tokenise.
pub fn classify_pair_tp(
    name_a: &str,
    kind_a: &str,
    name_b: &str,
    kind_b: &str,
    score: f32,
) -> bool {
    let na = normalize(name_a);
    let nb = normalize(name_b);
    if na == nb {
        return true;
    }
    if kind_a != kind_b {
        return false;
    }
    let toks = |s: &str| -> Vec<String> {
        s.split(|c: char| !c.is_alphanumeric())
            .filter(|t| t.len() >= 4)
            .map(str::to_owned)
            .collect()
    };
    let ta = toks(&na);
    let tb = toks(&nb);
    if ta.is_empty() || tb.is_empty() {
        return score >= 0.98;
    }
    let shared = ta.iter().filter(|t| tb.contains(t)).count();
    let denom = ta.len().max(tb.len());
    (shared as f64 / denom as f64) >= 0.5
}

fn normalize(s: &str) -> String {
    s.to_ascii_lowercase()
        .replace(['-', '_'], " ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}
