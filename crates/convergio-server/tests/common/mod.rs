//! Shared E2E test bootstrap.
//!
//! Extracted from per-file `async fn boot()` duplication (P2-5 of
//! the 2026-05-04 retrospective fix plan, finding H9). Spinning up
//! the in-process daemon used to live in 30+ near-identical copies
//! across `tests/e2e_*.rs`; an `AppState` field addition meant
//! editing every one. The shared helper now lives here.
//!
//! Per Cargo convention (`mod common;` per consumer), each test file
//! that wants the helper declares `mod common;` and imports
//! `common::boot`. Existing tests with their own `boot()` keep
//! working byte-for-byte until they migrate.

use convergio_bus::Bus;
use convergio_db::Pool;
use convergio_durability::{init, Durability};
use convergio_lifecycle::Supervisor;
use convergio_server::{router, AppState};
use std::net::SocketAddr;
use std::sync::Arc;
use tempfile::{tempdir, TempDir};
use tokio::net::TcpListener;

/// Spin up an in-process Convergio daemon backed by a temp SQLite
/// pool. Returns the bound base URL (`http://127.0.0.1:N`), the
/// shared `Pool` for direct test-side mutations, and the `TempDir`
/// — keep it alive for the duration of the test or the file is
/// rm-rf'd by Drop.
#[allow(dead_code)]
pub async fn boot() -> (String, Pool, TempDir) {
    // E2E tests must not depend on operator env — these flags change
    // dispatch semantics and can make assertions flaky.
    std::env::remove_var("CONVERGIO_EXECUTOR_USE_RUNNER");
    std::env::remove_var("CONVERGIO_EXECUTOR_MAX_PARALLEL");

    let dir = tempdir().expect("tempdir");
    let db_path = dir.path().join("state.db");
    let url = format!("sqlite://{}", db_path.display());
    let pool = Pool::connect(&url).await.expect("pool connect");
    init(&pool).await.expect("durability init");
    convergio_bus::init(&pool).await.expect("bus init");
    convergio_lifecycle::init(&pool)
        .await
        .expect("lifecycle init");

    let state = AppState {
        durability: Arc::new(Durability::new(pool.clone())),
        bus: Arc::new(Bus::new(pool.clone())),
        supervisor: Arc::new(Supervisor::new(pool.clone())),
        graph: Arc::new(convergio_graph::Store::new(pool.clone())),
        embed: Arc::new(convergio_embed::EmbedStore::new(pool.clone())),
        embedder: Arc::new(convergio_embed::embedder::testing::DeterministicTestEmbedder::new(8)),
        fleet: Arc::new(convergio_fleet::FleetStore::new(pool.clone())),
        audit_verify_cache: Arc::new(std::sync::Mutex::new(None)),
    };
    let app = router(state);
    let listener = TcpListener::bind(SocketAddr::from(([127, 0, 0, 1], 0)))
        .await
        .expect("bind");
    let addr = listener.local_addr().expect("local_addr");
    tokio::spawn(async move {
        axum::serve(listener, app).await.ok();
    });
    (format!("http://{addr}"), pool, dir)
}
