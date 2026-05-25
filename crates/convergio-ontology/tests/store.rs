//! Integration tests for the ontology registry SQLite storage.
//!
//! Tempdir-backed sqlite pool per test; no shared global state.
//! Golden snapshots live alongside in `tests/golden/` and are
//! compared byte-for-byte to enforce the determinism contract
//! shared with ADR-0047 (`actions.json`) and ADR-0060 (graph
//! output).

use convergio_db::Pool;
use convergio_ontology::{OwnerKind, Store};
use serde_json::json;

async fn migrated_store() -> anyhow::Result<(tempfile::TempDir, Store)> {
    let dir = tempfile::tempdir()?;
    let url = format!("sqlite://{}?mode=rwc", dir.path().join("o.db").display());
    let pool = Pool::connect(&url).await?;
    let store = Store::new(pool);
    store.migrate().await?;
    Ok((dir, store))
}

#[tokio::test]
async fn migrate_is_idempotent() -> anyhow::Result<()> {
    let (_dir, store) = migrated_store().await?;
    store.migrate().await?;
    store.migrate().await?;
    Ok(())
}

#[tokio::test]
async fn upsert_object_and_read_back() -> anyhow::Result<()> {
    let (_dir, store) = migrated_store().await?;
    let rec = store
        .upsert_object(
            "Task",
            1,
            false,
            "Task",
            "A unit of work tracked by Convergio.",
            json!({"shape": "modulor", "fields": ["title", "status"]}),
            Some(42),
        )
        .await?;
    assert_eq!(rec.name, "Task");
    assert_eq!(rec.schema_version, 1);
    assert!(!rec.breaking);
    assert_eq!(rec.audit_seq, Some(42));
    assert_eq!(rec.content_hash.len(), 64);

    let again = store.get_object("Task", 1).await?.expect("present");
    assert_eq!(again, rec);
    Ok(())
}

#[tokio::test]
async fn reupsert_same_version_same_hash_is_idempotent() -> anyhow::Result<()> {
    let (_dir, store) = migrated_store().await?;
    let body = json!({"shape": "modulor"});
    let a = store
        .upsert_object("Task", 1, false, "Task", "desc", body.clone(), None)
        .await?;
    let b = store
        .upsert_object("Task", 1, false, "Task", "desc", body, None)
        .await?;
    assert_eq!(a.content_hash, b.content_hash);
    assert_eq!(a.created_at, b.created_at, "no second insert happened");
    Ok(())
}

#[tokio::test]
async fn reupsert_same_version_different_body_is_conflict() -> anyhow::Result<()> {
    let (_dir, store) = migrated_store().await?;
    store
        .upsert_object("Task", 1, false, "Task", "desc", json!({"a": 1}), None)
        .await?;
    let err = store
        .upsert_object("Task", 1, false, "Task", "desc", json!({"a": 2}), None)
        .await
        .unwrap_err();
    assert!(
        matches!(
            err,
            convergio_ontology::Error::VersionConflict { kind: "object", .. }
        ),
        "expected VersionConflict, got {err:?}"
    );
    Ok(())
}

#[tokio::test]
async fn upsert_link_and_property() -> anyhow::Result<()> {
    let (_dir, store) = migrated_store().await?;
    store
        .upsert_object("Task", 1, false, "Task", "", json!({}), None)
        .await?;
    store
        .upsert_object("Plan", 1, false, "Plan", "", json!({}), None)
        .await?;
    let link = store
        .upsert_link(
            "BelongsTo",
            1,
            false,
            "Belongs to",
            "Task belongs to plan",
            "Task",
            "Plan",
            json!({"cardinality": "many_to_one"}),
            None,
        )
        .await?;
    assert_eq!(link.from_object, "Task");
    assert_eq!(link.to_object, "Plan");
    assert_eq!(link.content_hash.len(), 64);

    let prop = store
        .upsert_property(
            "TaskTitle",
            1,
            false,
            "Title",
            "Human-readable task title",
            OwnerKind::Object,
            "Task",
            "string",
            true,
            json!({"max_length": 200}),
            None,
        )
        .await?;
    assert_eq!(prop.datatype, "string");
    assert!(prop.required);
    assert_eq!(prop.owner_kind, OwnerKind::Object);
    Ok(())
}

#[tokio::test]
async fn content_hash_is_stable_across_runs() -> anyhow::Result<()> {
    // The hash MUST be a pure function of the semantic payload.
    // Mutating audit_seq or whether the row was already persisted
    // must not change it. This is the property the W1 task 3/4
    // exporters will rely on.
    let (_dir, store) = migrated_store().await?;
    let body = json!({"a": 1, "b": [1, 2, 3]});
    let first = store
        .upsert_object("X", 1, false, "X", "d", body.clone(), None)
        .await?;
    let (_dir2, store2) = migrated_store().await?;
    let second = store2
        .upsert_object("X", 1, false, "X", "d", body, Some(999))
        .await?;
    assert_eq!(first.content_hash, second.content_hash);
    Ok(())
}

#[tokio::test]
async fn golden_object_content_hash() -> anyhow::Result<()> {
    // Lock the hash for a known semantic body. If this golden ever
    // changes, the determinism contract has shifted and every
    // downstream consumer (JSON-Schema export, SHACL export,
    // exported graph artefacts) needs a coordinated migration.
    let (_dir, store) = migrated_store().await?;
    let rec = store
        .upsert_object(
            "GoldenObject",
            1,
            false,
            "Golden",
            "Stable fixture body for the W1 task 2 golden test.",
            json!({"a": 1, "b": "two", "c": [true, false, null]}),
            None,
        )
        .await?;
    let expected = include_str!("golden/object_content_hash.txt").trim();
    assert_eq!(
        rec.content_hash, expected,
        "GoldenObject content_hash drift — coordinate with downstream exporters before bumping"
    );
    Ok(())
}
