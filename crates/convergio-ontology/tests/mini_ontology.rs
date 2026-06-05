//! Integration test for the mini-ontology demo fixture (W1 T7).
//!
//! Loads `docs/examples/mini-ontology.yaml`, registers every entry
//! against a fresh in-memory store, exports both JSON-Schema and
//! SHACL for one object, and asserts:
//!
//! 1. **Byte-identity across reruns.** Two consecutive export calls
//!    against the same `(name, schema_version)` MUST return
//!    identical bytes — same posture as `actions.json` (ADR-0047)
//!    and graph output (ADR-0060).
//! 2. **Byte-identity across store re-population.** Re-creating the
//!    store from the same YAML must produce the same export bytes.
//!    Together with (1) this rules out hidden order-dependence
//!    (HashMap iteration, monotonic counters baked into output,
//!    etc.).
//! 3. The demo YAML actually parses and registers without errors.
//!
//! This test is intentionally schema-shape-agnostic: it does not pin
//! the exact bytes (the per-format goldens in `jsonschema.rs` /
//! `shacl.rs` already do that). What it pins is the **loader path**:
//! YAML → Store → exporters must keep determinism.

use anyhow::{Context, Result};
use convergio_db::Pool;
use convergio_ontology::{export_object_schema, export_object_shacl, OwnerKind, Store};
use serde::Deserialize;
use serde_json::Value;
use std::path::PathBuf;

#[derive(Debug, Deserialize)]
struct MiniOntology {
    schema_version: i64,
    #[serde(default)]
    objects: Vec<ObjectEntry>,
    #[serde(default)]
    links: Vec<LinkEntry>,
    #[serde(default)]
    properties: Vec<PropertyEntry>,
}

#[derive(Debug, Deserialize)]
struct ObjectEntry {
    name: String,
    title: String,
    description: String,
    #[serde(default)]
    body: Value,
}

#[derive(Debug, Deserialize)]
struct LinkEntry {
    name: String,
    title: String,
    description: String,
    from_object: String,
    to_object: String,
    #[serde(default)]
    body: Value,
}

#[derive(Debug, Deserialize)]
struct PropertyEntry {
    name: String,
    title: String,
    description: String,
    owner_kind: String,
    owner_name: String,
    datatype: String,
    required: bool,
    #[serde(default)]
    body: Value,
}

fn fixture_path() -> PathBuf {
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    // Crate is at <repo>/crates/convergio-ontology — fixture is at
    // <repo>/docs/examples/mini-ontology.yaml.
    p.pop();
    p.pop();
    p.push("docs/examples/mini-ontology.yaml");
    p
}

fn parse_fixture() -> Result<MiniOntology> {
    let path = fixture_path();
    let raw = std::fs::read_to_string(&path)
        .with_context(|| format!("read demo fixture at {}", path.display()))?;
    let parsed: MiniOntology =
        serde_yaml::from_str(&raw).context("parse docs/examples/mini-ontology.yaml")?;
    Ok(parsed)
}

async fn load_fixture(store: &Store, fixture: &MiniOntology) -> Result<()> {
    let v = fixture.schema_version;
    for o in &fixture.objects {
        store
            .upsert_object(
                &o.name,
                v,
                false,
                &o.title,
                &o.description,
                o.body.clone(),
                None,
            )
            .await?;
    }
    for l in &fixture.links {
        store
            .upsert_link(
                &l.name,
                v,
                false,
                &l.title,
                &l.description,
                &l.from_object,
                &l.to_object,
                l.body.clone(),
                None,
            )
            .await?;
    }
    for p in &fixture.properties {
        let owner_kind = match p.owner_kind.as_str() {
            "object" => OwnerKind::Object,
            "link" => OwnerKind::Link,
            other => anyhow::bail!("unknown owner_kind in fixture: {other}"),
        };
        store
            .upsert_property(
                &p.name,
                v,
                false,
                &p.title,
                &p.description,
                owner_kind,
                &p.owner_name,
                &p.datatype,
                p.required,
                p.body.clone(),
                None,
            )
            .await?;
    }
    Ok(())
}

async fn fresh_store() -> Result<(tempfile::TempDir, Store)> {
    let dir = tempfile::tempdir()?;
    let url = format!("sqlite://{}?mode=rwc", dir.path().join("o.db").display());
    let pool = Pool::connect(&url).await?;
    let store = Store::new(pool);
    store.migrate().await?;
    Ok((dir, store))
}

#[tokio::test]
async fn fixture_parses_and_registers_cleanly() -> Result<()> {
    let fixture = parse_fixture()?;
    assert_eq!(fixture.schema_version, 1);
    assert!(!fixture.objects.is_empty());
    assert!(!fixture.properties.is_empty());

    let (_dir, store) = fresh_store().await?;
    load_fixture(&store, &fixture).await?;
    Ok(())
}

#[tokio::test]
async fn jsonschema_export_is_byte_identical_across_reruns() -> Result<()> {
    let fixture = parse_fixture()?;
    let (_dir, store) = fresh_store().await?;
    load_fixture(&store, &fixture).await?;

    let a = export_object_schema(&store, "Person", 1).await?;
    let b = export_object_schema(&store, "Person", 1).await?;
    assert_eq!(a, b, "JSON-Schema export must be byte-identical on rerun");
    Ok(())
}

#[tokio::test]
async fn shacl_export_is_byte_identical_across_reruns() -> Result<()> {
    let fixture = parse_fixture()?;
    let (_dir, store) = fresh_store().await?;
    load_fixture(&store, &fixture).await?;

    let a = export_object_shacl(&store, "Person", 1).await?;
    let b = export_object_shacl(&store, "Person", 1).await?;
    assert_eq!(a, b, "SHACL export must be byte-identical on rerun");
    Ok(())
}

#[tokio::test]
async fn exports_are_stable_across_store_repopulation() -> Result<()> {
    // Same YAML → two different SQLite files → identical export
    // bytes. This catches order-of-insertion bugs and any latent
    // dependence on monotonic counters / timestamps in the
    // exporter pipeline (ADR-0053 § Determinism, ADR-0060).
    let fixture = parse_fixture()?;

    let (_dir1, store1) = fresh_store().await?;
    load_fixture(&store1, &fixture).await?;
    let js1 = export_object_schema(&store1, "Person", 1).await?;
    let sh1 = export_object_shacl(&store1, "Person", 1).await?;

    let (_dir2, store2) = fresh_store().await?;
    load_fixture(&store2, &fixture).await?;
    let js2 = export_object_schema(&store2, "Person", 1).await?;
    let sh2 = export_object_shacl(&store2, "Person", 1).await?;

    assert_eq!(
        js1, js2,
        "JSON-Schema export must be stable across store repopulation",
    );
    assert_eq!(
        sh1, sh2,
        "SHACL export must be stable across store repopulation",
    );
    Ok(())
}

#[tokio::test]
async fn link_and_organisation_export_round_trip() -> Result<()> {
    // The fixture defines an Organisation peer + a WorksFor link, so
    // exercise the exporter against an object other than Person too.
    let fixture = parse_fixture()?;
    let (_dir, store) = fresh_store().await?;
    load_fixture(&store, &fixture).await?;

    let org_a = export_object_schema(&store, "Organisation", 1).await?;
    let org_b = export_object_schema(&store, "Organisation", 1).await?;
    assert_eq!(org_a, org_b);

    let org_shacl = export_object_shacl(&store, "Organisation", 1).await?;
    assert!(!org_shacl.is_empty());
    Ok(())
}
