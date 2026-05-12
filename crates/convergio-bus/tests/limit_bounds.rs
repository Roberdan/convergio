//! Reproduces audit findings F1–F3 + F9 (medium severity, security
//! constitution): every public read API on the bus accepts a
//! caller-supplied `limit` and binds it directly into SQLite. Negative
//! and zero limits become unbounded reads (`LIMIT -1` returns all rows
//! in SQLite) and huge limits skip any defence-in-depth cap at the
//! crate boundary.
//!
//! The HTTP layer (`convergio-server`) already clamps to 1..=100, but
//! the bus is also reachable from in-process callers (executor, Layer 4
//! helpers, future MCP tools). The crate boundary must enforce the
//! same invariant.

use convergio_bus::{init, Bus, NewMessage, NewSystemMessage};
use convergio_db::Pool;
use serde_json::json;
use tempfile::tempdir;

/// Documented per-call cap enforced at the crate boundary.
const EXPECTED_MAX_LIMIT: usize = 1000;

async fn fresh_bus() -> (Bus, tempfile::TempDir) {
    let dir = tempdir().unwrap();
    let url = format!("sqlite://{}/state.db", dir.path().display());
    let pool = Pool::connect(&url).await.unwrap();
    init(&pool).await.unwrap();
    (Bus::new(pool), dir)
}

async fn seed_plan(bus: &Bus, n: usize) {
    for i in 0..n {
        bus.publish(NewMessage {
            plan_id: "plan-1".into(),
            topic: "events".into(),
            sender: None,
            payload: json!({ "i": i }),
        })
        .await
        .unwrap();
    }
}

async fn seed_system(bus: &Bus, n: usize) {
    for i in 0..n {
        bus.publish_system(NewSystemMessage {
            topic: "system.presence".into(),
            sender: Some("agent-x".into()),
            payload: json!({ "i": i }),
        })
        .await
        .unwrap();
    }
}

#[tokio::test]
async fn poll_rejects_non_positive_limit() {
    let (bus, _dir) = fresh_bus().await;
    seed_plan(&bus, 3).await;

    assert!(
        bus.poll("plan-1", "events", 0, 0).await.is_err(),
        "limit=0 must be rejected"
    );
    assert!(
        bus.poll("plan-1", "events", 0, -1).await.is_err(),
        "limit=-1 must be rejected (currently returns ALL rows in SQLite)"
    );
}

#[tokio::test]
async fn poll_caps_oversized_limit() {
    let (bus, _dir) = fresh_bus().await;
    seed_plan(&bus, EXPECTED_MAX_LIMIT + 100).await;

    let rows = bus.poll("plan-1", "events", 0, i64::MAX).await.unwrap();
    assert!(
        rows.len() <= EXPECTED_MAX_LIMIT,
        "poll must cap large limits (got {} > {EXPECTED_MAX_LIMIT})",
        rows.len()
    );
}

#[tokio::test]
async fn poll_filtered_rejects_non_positive_limit() {
    let (bus, _dir) = fresh_bus().await;
    seed_plan(&bus, 1).await;

    assert!(bus
        .poll_filtered("plan-1", "events", 0, 0, None)
        .await
        .is_err());
    assert!(bus
        .poll_filtered("plan-1", "events", 0, -5, Some("agent-x"))
        .await
        .is_err());
}

#[tokio::test]
async fn tail_rejects_non_positive_limit() {
    let (bus, _dir) = fresh_bus().await;
    seed_plan(&bus, 1).await;

    assert!(bus.tail("plan-1", None, 0, 0).await.is_err());
    assert!(bus.tail("plan-1", Some("events"), 0, -1).await.is_err());
}

#[tokio::test]
async fn tail_caps_oversized_limit() {
    let (bus, _dir) = fresh_bus().await;
    seed_plan(&bus, EXPECTED_MAX_LIMIT + 100).await;

    let rows = bus.tail("plan-1", None, 0, i64::MAX).await.unwrap();
    assert!(
        rows.len() <= EXPECTED_MAX_LIMIT,
        "tail must cap large limits (got {} > {EXPECTED_MAX_LIMIT})",
        rows.len()
    );
}

#[tokio::test]
async fn poll_system_rejects_non_positive_limit() {
    let (bus, _dir) = fresh_bus().await;
    seed_system(&bus, 1).await;

    assert!(bus.poll_system("system.presence", 0, 0).await.is_err());
    assert!(bus.poll_system("system.presence", 0, -10).await.is_err());
}

#[tokio::test]
async fn poll_system_caps_oversized_limit() {
    let (bus, _dir) = fresh_bus().await;
    seed_system(&bus, EXPECTED_MAX_LIMIT + 100).await;

    let rows = bus
        .poll_system("system.presence", 0, i64::MAX)
        .await
        .unwrap();
    assert!(
        rows.len() <= EXPECTED_MAX_LIMIT,
        "poll_system must cap large limits (got {} > {EXPECTED_MAX_LIMIT})",
        rows.len()
    );
}
