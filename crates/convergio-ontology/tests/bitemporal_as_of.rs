//! Bitemporal as-of read tests for the `object_events` log (ADR-0053, W3).
//!
//! Boots a tempdir SQLite via `convergio-db::Pool`, runs durability +
//! ontology migrations, appends events with distinct valid-time windows
//! across separate transactions, then asserts the as-of query methods
//! return the correct historical snapshot on both temporal axes.

use chrono::{Duration, Utc};
use convergio_db::Pool;
use convergio_ontology::{init, NewObjectEvent, ObjectEventsStore, Store};
use serde_json::json;

async fn boot() -> (Pool, tempfile::TempDir) {
    let dir = tempfile::tempdir().expect("tempdir");
    let url = format!(
        "sqlite://{}?mode=rwc",
        dir.path().join("state.db").display()
    );
    let pool = Pool::connect(&url).await.expect("connect");
    convergio_durability::init(&pool)
        .await
        .expect("durability migrations");
    init(&pool).await.expect("ontology migrations");
    Store::new(pool.clone())
        .migrate()
        .await
        .expect("ontology store migrate");
    (pool, dir)
}

#[tokio::test]
async fn get_valid_as_of_returns_window_owner() {
    let (pool, _dir) = boot().await;
    let store = ObjectEventsStore::new(pool.clone());

    let t0 = Utc::now();
    let t1 = t0 + Duration::days(10);
    let t2 = t0 + Duration::days(20);

    // Single transaction-current row whose valid-time window is [t1, t2).
    store
        .append_event(
            NewObjectEvent {
                object_id: "o1".to_owned(),
                op: "upsert".to_owned(),
                payload: json!({"v": "mid"}),
                valid_from: t1,
                valid_to: Some(t2),
            },
            Some("agent-1"),
        )
        .await
        .expect("append");

    // Before the window opens: nothing valid.
    assert!(store
        .get_valid_as_of("o1", t0 + Duration::days(5))
        .await
        .expect("query")
        .is_none());

    // Inside the window: the row.
    let inside = store
        .get_valid_as_of("o1", t0 + Duration::days(15))
        .await
        .expect("query")
        .expect("event in window");
    assert_eq!(inside.payload, json!({"v": "mid"}));

    // At the exclusive upper bound t2: excluded.
    assert!(store
        .get_valid_as_of("o1", t2)
        .await
        .expect("query")
        .is_none());
}

#[tokio::test]
async fn tx_as_of_reflects_what_was_known_then() {
    let (pool, _dir) = boot().await;
    let store = ObjectEventsStore::new(pool.clone());

    let valid = Utc::now();

    // First belief: v=1.
    store
        .append_event(
            NewObjectEvent {
                object_id: "o1".to_owned(),
                op: "upsert".to_owned(),
                payload: json!({"v": 1}),
                valid_from: valid,
                valid_to: None,
            },
            Some("agent-1"),
        )
        .await
        .expect("append v1");

    let between = Utc::now();
    // Ensure a strictly later tx_from for the correction.
    tokio::time::sleep(std::time::Duration::from_millis(5)).await;

    // Correction supersedes the belief: v=2.
    store
        .append_event(
            NewObjectEvent {
                object_id: "o1".to_owned(),
                op: "upsert".to_owned(),
                payload: json!({"v": 2}),
                valid_from: valid,
                valid_to: None,
            },
            Some("agent-1"),
        )
        .await
        .expect("append v2");

    // As known at `between`: the system still believed v=1.
    let past = store
        .get_tx_as_of("o1", between)
        .await
        .expect("query")
        .expect("event known at `between`");
    assert_eq!(past.payload, json!({"v": 1}));

    // As known now: v=2.
    let now = store
        .get_tx_as_of("o1", Utc::now())
        .await
        .expect("query")
        .expect("event known now");
    assert_eq!(now.payload, json!({"v": 2}));
}

#[tokio::test]
async fn list_variants_snapshot_all_objects() {
    let (pool, _dir) = boot().await;
    let store = ObjectEventsStore::new(pool.clone());

    let valid = Utc::now();

    for id in ["a", "b"] {
        store
            .append_event(
                NewObjectEvent {
                    object_id: id.to_owned(),
                    op: "upsert".to_owned(),
                    payload: json!({"id": id, "v": 1}),
                    valid_from: valid,
                    valid_to: None,
                },
                Some("agent-1"),
            )
            .await
            .expect("append");
    }

    let between = Utc::now();
    tokio::time::sleep(std::time::Duration::from_millis(5)).await;

    // Correct only "a" to v=2.
    store
        .append_event(
            NewObjectEvent {
                object_id: "a".to_owned(),
                op: "upsert".to_owned(),
                payload: json!({"id": "a", "v": 2}),
                valid_from: valid,
                valid_to: None,
            },
            Some("agent-1"),
        )
        .await
        .expect("append a v2");

    // valid-as-of: both objects valid now, deterministically ordered.
    let valid_snap = store
        .list_valid_as_of(Utc::now())
        .await
        .expect("valid snapshot");
    assert_eq!(valid_snap.len(), 2);
    assert_eq!(valid_snap[0].object_id, "a");
    assert_eq!(valid_snap[1].object_id, "b");
    assert_eq!(valid_snap[0].payload, json!({"id": "a", "v": 2}));

    // tx-as-of at `between`: "a" was still v=1.
    let tx_snap = store.list_tx_as_of(between).await.expect("tx snapshot");
    assert_eq!(tx_snap.len(), 2);
    assert_eq!(tx_snap[0].object_id, "a");
    assert_eq!(tx_snap[0].payload, json!({"id": "a", "v": 1}));

    // tx-as-of now: "a" is v=2.
    let tx_now = store
        .list_tx_as_of(Utc::now())
        .await
        .expect("tx snapshot now");
    assert_eq!(tx_now[0].payload, json!({"id": "a", "v": 2}));
}
