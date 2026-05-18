//! E2E test: boot an in-process daemon and run the full
//! `cvg coherence handshake` round-trip against it.
//!
//! Mirrors the boot pattern used in `crates/convergio-server/tests/`
//! so the verifier exercises the real axum router, the real
//! durability + bus stores, and a real SQLite tempdir — i.e. the
//! same surface the standalone `cvg` CLI hits at runtime.

use convergio_bus::Bus;
use convergio_coherence::handshake::{run_check, PhaseOutcome};
use convergio_db::Pool;
use convergio_durability::{init, Durability};
use convergio_lifecycle::Supervisor;
use convergio_server::{router, AppState};
use serde_json::Value;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use tempfile::tempdir;
use tokio::net::TcpListener;

async fn boot() -> (String, tempfile::TempDir) {
    let dir = tempdir().expect("tempdir");
    let db_path = dir.path().join("state.db");
    let pool = Pool::connect(&format!("sqlite://{}", db_path.display()))
        .await
        .expect("connect pool");
    init(&pool).await.expect("init durability");
    convergio_bus::init(&pool).await.expect("init bus");
    convergio_lifecycle::init(&pool)
        .await
        .expect("init lifecycle");
    let state = AppState {
        durability: Arc::new(Durability::new(pool.clone())),
        bus: Arc::new(Bus::new(pool.clone())),
        supervisor: Arc::new(Supervisor::new(pool.clone())),
        graph: Arc::new(convergio_graph::Store::new(pool.clone())),
        embed: Arc::new(convergio_embed::EmbedStore::new(pool.clone())),
        embedder: Arc::new(convergio_embed::embedder::testing::DeterministicTestEmbedder::new(8)),
        fleet: Arc::new(convergio_fleet::FleetStore::new(pool.clone())),
        fleet_plans: Arc::new(convergio_fleet::FleetPlanStore::new(pool.clone())),
        audit_verify_cache: Arc::new(std::sync::Mutex::new(None)),
    };
    let listener = TcpListener::bind(SocketAddr::from(([127, 0, 0, 1], 0)))
        .await
        .expect("bind");
    let addr = listener.local_addr().expect("local_addr");
    tokio::spawn(async move {
        axum::serve(listener, router(state)).await.expect("serve");
    });
    (format!("http://{addr}"), dir)
}

#[tokio::test]
async fn handshake_full_round_trip_succeeds() {
    let (base, _dir) = boot().await;
    let report = run_check(&base, 10).await.expect("run_check");
    assert!(
        report.success,
        "handshake should succeed against fresh daemon, phases: {:?}",
        report
            .phases
            .iter()
            .map(|p| (p.n, p.outcome, p.detail.clone()))
            .collect::<Vec<_>>()
    );
    assert_eq!(report.phases.len(), 6);
    for p in &report.phases {
        assert_eq!(
            p.outcome,
            PhaseOutcome::Ok,
            "phase {} ({}) should be Ok, was {:?}: {}",
            p.n,
            p.label,
            p.outcome,
            p.detail
        );
    }
    assert!(
        report.total_elapsed_ms < report.timeout_ms * 6,
        "total elapsed should be well under 6×timeout"
    );
    assert!(
        report.agent_ids.0.starts_with("handshake-A-"),
        "agent A id format"
    );
    assert!(
        report.agent_ids.1.starts_with("handshake-B-"),
        "agent B id format"
    );
    assert!(!report.plan_id.is_empty(), "plan_id is recorded");

    // Stronger assertions than "success": the two published messages
    // must be acked (so they no longer re-surface on poll), and the
    // audit chain must verify.
    let http = reqwest::Client::new();

    let tail: Value = http
        .get(format!(
            "{base}/v1/plans/{}/messages/tail?topic=coordination/handshake&limit=10",
            report.plan_id
        ))
        .send()
        .await
        .expect("tail request")
        .json()
        .await
        .expect("tail json");
    let arr = tail.as_array().expect("tail is array");
    assert_eq!(arr.len(), 2, "handshake publishes ping + pong");

    let ping = &arr[0];
    let pong = &arr[1];
    let ping_id = ping.get("id").and_then(Value::as_str).expect("ping id");

    assert_eq!(
        ping.get("sender").and_then(Value::as_str),
        Some(report.agent_ids.0.as_str()),
        "ping sender must be agent A"
    );
    assert_eq!(
        pong.get("sender").and_then(Value::as_str),
        Some(report.agent_ids.1.as_str()),
        "pong sender must be agent B"
    );
    assert_eq!(
        ping.pointer("/payload/type").and_then(Value::as_str),
        Some("ping"),
        "ping payload type"
    );
    assert_eq!(
        pong.pointer("/payload/type").and_then(Value::as_str),
        Some("pong"),
        "pong payload type"
    );
    assert_eq!(
        pong.pointer("/payload/replying_to").and_then(Value::as_str),
        Some(ping_id),
        "pong must reply to ping id"
    );

    for (label, msg) in [("ping", ping), ("pong", pong)] {
        assert!(
            msg.get("consumed_at")
                .map(|v| !v.is_null())
                .unwrap_or(false),
            "{label} must be acked (consumed_at non-null): {msg}"
        );
    }
    assert_eq!(
        ping.get("consumed_by").and_then(Value::as_str),
        Some(report.agent_ids.1.as_str()),
        "B must ack ping"
    );
    assert_eq!(
        pong.get("consumed_by").and_then(Value::as_str),
        Some(report.agent_ids.0.as_str()),
        "A must ack pong"
    );

    let poll: Value = http
        .get(format!(
            "{base}/v1/plans/{}/messages?topic=coordination/handshake&limit=10",
            report.plan_id
        ))
        .send()
        .await
        .expect("poll request")
        .json()
        .await
        .expect("poll json");
    assert!(
        poll.as_array().expect("poll is array").is_empty(),
        "acked messages must not re-surface on poll"
    );

    let audit: Value = http
        .get(format!("{base}/v1/audit/verify"))
        .send()
        .await
        .expect("audit request")
        .json()
        .await
        .expect("audit json");
    assert_eq!(audit["ok"], true, "audit chain should verify");
}

#[tokio::test]
async fn handshake_against_dead_daemon_reports_bootstrap_failure() {
    // Bind a port and immediately drop the listener so the address
    // is closed; nothing should be answering at this URL.
    let listener = TcpListener::bind(SocketAddr::from(([127, 0, 0, 1], 0)))
        .await
        .expect("bind");
    let addr = listener.local_addr().expect("addr");
    drop(listener);
    let base = format!("http://{addr}");
    let report = run_check(&base, 1).await.expect("run_check");
    assert!(!report.success, "expected failure against dead daemon");
    // Bootstrap (phase 0) plus six padded phases, all but the
    // first marked Skipped.
    let bootstrap = &report.phases[0];
    assert_eq!(bootstrap.outcome, PhaseOutcome::Failed);
    assert!(
        bootstrap.detail.contains("plan create failed"),
        "bootstrap detail names the broken seam: {}",
        bootstrap.detail
    );
    let padded: Vec<_> = report
        .phases
        .iter()
        .filter(|p| p.outcome == PhaseOutcome::Skipped)
        .collect();
    assert_eq!(padded.len(), 6, "every downstream phase is padded");
}

#[tokio::test]
async fn handshake_with_short_timeout_finishes_fast() {
    // Timeout 1s should still be enough for an in-process daemon —
    // this regression guards against accidental sleeps in the
    // verifier itself.
    let (base, _dir) = boot().await;
    let report = run_check(&base, 1).await.expect("run_check");
    assert!(report.success, "1s window should be plenty in-process");
    assert!(
        report.total_elapsed_ms < 1500,
        "in-process round-trip well under 1.5s, was {}ms",
        report.total_elapsed_ms
    );
    // Sanity-check on the configured timeout snapshot.
    assert_eq!(report.timeout_ms, Duration::from_secs(1).as_millis());
}
