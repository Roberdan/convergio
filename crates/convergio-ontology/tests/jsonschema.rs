//! Integration tests for the deterministic JSON-Schema export.
//!
//! Goldens live next to this file under `tests/golden/` and are
//! compared byte-for-byte. ADR-0053 § Determinism + ADR-0047
//! posture: a drift here is a CI failure, not a warning.

use convergio_db::Pool;
use convergio_ontology::{export_object_schema, OwnerKind, Store};
use serde_json::json;

async fn seeded_store() -> anyhow::Result<(tempfile::TempDir, Store)> {
    let dir = tempfile::tempdir()?;
    let url = format!("sqlite://{}?mode=rwc", dir.path().join("o.db").display());
    let pool = Pool::connect(&url).await?;
    let store = Store::new(pool);
    store.migrate().await?;

    store
        .upsert_object(
            "Person",
            1,
            false,
            "Person",
            "A natural person.",
            json!({}),
            None,
        )
        .await?;
    store
        .upsert_property(
            "email",
            1,
            false,
            "Email",
            "Primary email address.",
            OwnerKind::Object,
            "Person",
            "string",
            true,
            json!({"maxLength": 254}),
            None,
        )
        .await?;
    store
        .upsert_property(
            "age",
            1,
            false,
            "",
            "",
            OwnerKind::Object,
            "Person",
            "integer",
            false,
            json!({"minimum": 0}),
            None,
        )
        .await?;
    Ok((dir, store))
}

#[tokio::test]
async fn export_matches_golden_jsonschema() -> anyhow::Result<()> {
    let (_dir, store) = seeded_store().await?;
    let bytes = export_object_schema(&store, "Person", 1).await?;
    let actual = std::str::from_utf8(&bytes)?;
    let golden = include_str!("golden/person_v1.jsonschema.json");
    if actual != golden {
        eprintln!("=== actual ===\n{actual}\n=== golden ===\n{golden}");
        panic!("JSON-Schema export drifted from golden fixture");
    }
    Ok(())
}

#[tokio::test]
async fn export_is_idempotent_across_reruns() -> anyhow::Result<()> {
    let (_dir, store) = seeded_store().await?;
    let a = export_object_schema(&store, "Person", 1).await?;
    let b = export_object_schema(&store, "Person", 1).await?;
    assert_eq!(a, b, "exporter must produce byte-identical output");
    Ok(())
}

#[tokio::test]
async fn export_missing_object_is_not_found() -> anyhow::Result<()> {
    let (_dir, store) = seeded_store().await?;
    let err = export_object_schema(&store, "Nope", 1).await;
    assert!(err.is_err(), "missing object must error");
    Ok(())
}
