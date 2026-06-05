//! ADR-0002 audit tests.
use convergio_db::Pool;
use convergio_durability::audit::{canonical_json, EntityKind};
use convergio_durability::{init, Durability, NewPlan};
use serde_json::json;
use tempfile::tempdir;
use tokio::task::JoinSet;

async fn fresh_dur() -> (Durability, tempfile::TempDir) {
    let dir = tempdir().unwrap();
    let pool = Pool::connect(&format!("sqlite://{}/state.db", dir.path().display()))
        .await
        .unwrap();
    init(&pool).await.unwrap();
    (Durability::new(pool), dir)
}

async fn fresh_dur_with_some_history() -> (Durability, tempfile::TempDir) {
    let (dur, dir) = fresh_dur().await;
    for i in 0..3 {
        dur.create_plan(NewPlan {
            title: format!("plan {i}"),
            description: None,
            project: None,
        })
        .await
        .unwrap();
    }
    (dur, dir)
}

async fn broken_after(sql: &str, value: String, seq: i64) -> Option<i64> {
    let (dur, _dir) = fresh_dur_with_some_history().await;
    sqlx::query(sql)
        .bind(value)
        .bind(seq)
        .execute(dur.pool().inner())
        .await
        .unwrap();
    let report = dur.audit().verify(None, None).await.unwrap();
    assert!(!report.ok);
    report.broken_at
}

async fn drain(mut jobs: JoinSet<()>) {
    while let Some(result) = jobs.join_next().await {
        result.unwrap();
    }
}

#[tokio::test]
async fn clean_chain_verifies() {
    let (dur, _dir) = fresh_dur_with_some_history().await;
    assert!(dur.audit().verify(None, None).await.unwrap().ok);
}

#[tokio::test]
async fn audit_mutations_break_chain() {
    let broken_at = broken_after(
        "UPDATE audit_log SET payload = ? WHERE seq = ?",
        r#"{"hacked":true}"#.to_string(),
        2,
    )
    .await;
    assert_eq!(broken_at, Some(2));

    let broken_at = broken_after(
        "UPDATE audit_log SET hash = ? WHERE seq = ?",
        "deadbeef".repeat(8),
        1,
    )
    .await;
    assert!(matches!(broken_at, Some(1) | Some(2)));

    let broken_at = broken_after(
        "UPDATE audit_log SET prev_hash = ? WHERE seq = ?",
        "0".repeat(64),
        2,
    )
    .await;
    assert_eq!(broken_at, Some(2));

    let (dur, _dir) = fresh_dur_with_some_history().await;
    sqlx::query("DELETE FROM audit_log WHERE seq = ?")
        .bind(2_i64)
        .execute(dur.pool().inner())
        .await
        .unwrap();
    let report = dur.audit().verify(None, None).await.unwrap();
    assert!(!report.ok, "deleted row must be detected");
    assert_eq!(report.broken_at, Some(3));
}

#[tokio::test]
async fn ranged_verify_catches_tamper_inside_range() {
    let (dur, _dir) = fresh_dur_with_some_history().await;
    sqlx::query("UPDATE audit_log SET payload = ? WHERE seq = ?")
        .bind(r#"{"x":1}"#)
        .bind(2_i64)
        .execute(dur.pool().inner())
        .await
        .unwrap();
    let r = dur.audit().verify(Some(1), Some(2)).await.unwrap();
    assert!(!r.ok);
    assert_eq!(r.broken_at, Some(2));
    let r = dur.audit().verify(Some(3), None).await.unwrap();
    assert!(r.ok, "ranged verify intentionally misses earlier tamper");
}

#[tokio::test]
async fn concurrent_writes_keep_a_contiguous_chain() {
    let (dur, _dir) = fresh_dur().await;
    let mut jobs = JoinSet::new();
    for i in 0..20 {
        let dur = dur.clone();
        jobs.spawn(async move {
            dur.create_plan(NewPlan {
                title: format!("plan {i}"),
                description: None,
                project: None,
            })
            .await
            .unwrap();
        });
    }
    drain(jobs).await;
    let report = dur.audit().verify(None, None).await.unwrap();
    assert!(report.ok);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 6)]
async fn direct_audit_appends_under_stress_remain_gap_free() {
    let (dur, _dir) = fresh_dur().await;
    let mut jobs = JoinSet::new();
    for worker in 0..24 {
        let audit = dur.audit();
        jobs.spawn(async move {
            for iteration in 0..5 {
                audit
                    .append(
                        EntityKind::Plan,
                        &format!("entity-{worker}-{iteration}"),
                        "audit.stress",
                        &json!({"iteration": iteration, "worker": worker}),
                        Some(format!("agent-{worker}").as_str()),
                    )
                    .await
                    .unwrap();
            }
        });
    }
    drain(jobs).await;
    let report = dur.audit().verify(None, None).await.unwrap();
    assert!(report.ok && report.checked == 120, "{report:?}");
}

#[test]
fn canonical_json_covers_numeric_edge_cases() {
    let payload = json!({
        "float_integer": 1.0,
        "large_exponent": 1.23e45,
        "max_i64": i64::MAX,
        "max_u64": u64::MAX,
        "min_i64": i64::MIN,
        "negative_zero": -0.0,
        "small_exponent": 1e-6,
    });
    assert_eq!(
        canonical_json(&payload).unwrap(),
        r#"{"float_integer":1.0,"large_exponent":1.23e+45,"max_i64":9223372036854775807,"max_u64":18446744073709551615,"min_i64":-9223372036854775808,"negative_zero":-0.0,"small_exponent":1e-6}"#
    );
    assert_eq!(canonical_json(&json!({"n": 1})).unwrap(), r#"{"n":1}"#);
    assert_eq!(canonical_json(&json!({"n": 1.0})).unwrap(), r#"{"n":1.0}"#);
}

#[tokio::test]
async fn append_with_provenance_writes_chained_bundle() {
    let (dur, _dir) = fresh_dur().await;
    let (entry, prov) = dur
        .audit()
        .append_with_provenance(
            EntityKind::Plan,
            "plan-1",
            "plan.created",
            &json!({"title": "EU"}),
            Some("agent-1"),
        )
        .await
        .unwrap();

    assert_eq!(entry.seq, 1);
    assert_eq!(prov.seq, 2);
    assert_eq!(prov.transition, "provenance.bundle_emitted");
    assert!(prov.payload.contains("wasGeneratedBy"));
    assert!(dur.audit().verify(None, None).await.unwrap().ok);
}
