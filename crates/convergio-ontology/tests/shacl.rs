//! Integration tests for the SHACL / JSON-LD exporter.
//!
//! Golden snapshot lives in `tests/golden/`. Drift = CI failure.

use convergio_db::Pool;
use convergio_ontology::{export_object_shacl, OwnerKind, Store};
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
            json!({}),
            None,
        )
        .await?;
    store
        .upsert_property(
            "homepage",
            1,
            false,
            "",
            "",
            OwnerKind::Object,
            "Person",
            "iri",
            false,
            json!({}),
            None,
        )
        .await?;
    Ok((dir, store))
}

#[tokio::test]
async fn export_matches_golden_shacl() -> anyhow::Result<()> {
    let (_dir, store) = seeded_store().await?;
    let bytes = export_object_shacl(&store, "Person", 1).await?;
    let actual = std::str::from_utf8(&bytes)?;
    let golden = include_str!("golden/person_v1.shacl.jsonld");
    if actual != golden {
        eprintln!("=== actual ===\n{actual}\n=== golden ===\n{golden}");
        panic!("SHACL export drifted from golden fixture");
    }
    Ok(())
}

#[tokio::test]
async fn export_is_idempotent_across_reruns() -> anyhow::Result<()> {
    let (_dir, store) = seeded_store().await?;
    let a = export_object_shacl(&store, "Person", 1).await?;
    let b = export_object_shacl(&store, "Person", 1).await?;
    assert_eq!(a, b);
    Ok(())
}

#[tokio::test]
async fn export_missing_object_is_not_found() -> anyhow::Result<()> {
    let (_dir, store) = seeded_store().await?;
    assert!(export_object_shacl(&store, "Nope", 1).await.is_err());
    Ok(())
}
