//! Bulk import of an ontology draft into the registry.
//!
//! Closes the authoring loop (ADR-0080): a self-contained draft —
//! produced by `convergio-ontology-author` or any external tool that
//! speaks the same JSON shape — is registered through the daemon so it
//! becomes queryable and exportable like any other schema. Import is
//! additive and idempotent: every type lands at `schema_version = 1`
//! and re-importing the same draft is a no-op (the store dedupes on the
//! content hash). The importer never registers a breaking revision.

use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::BTreeSet;

use crate::error::{Error, Result};
use crate::model::OwnerKind;
use crate::store::Store;

/// A proposed object type in an import payload.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ImportObject {
    /// Machine name.
    pub name: String,
    /// Short human title.
    #[serde(default)]
    pub title: String,
    /// Longer description.
    #[serde(default)]
    pub description: String,
    /// Purpose-limitation flag (ADR-0082): instances of this type may only
    /// be created under a declared, registered purpose.
    #[serde(default)]
    pub requires_purpose: bool,
}

/// A proposed property in an import payload (owner is an object).
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ImportProperty {
    /// Machine name.
    pub name: String,
    /// Owning object name.
    pub owner: String,
    /// Datatype tag.
    pub datatype: String,
    /// Whether instances must carry the property.
    #[serde(default)]
    pub required: bool,
    /// Short human title.
    #[serde(default)]
    pub title: String,
    /// Longer description.
    #[serde(default)]
    pub description: String,
}

/// A proposed link in an import payload.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ImportLink {
    /// Machine name.
    pub name: String,
    /// Source object name.
    pub from: String,
    /// Target object name.
    pub to: String,
    /// Short human title.
    #[serde(default)]
    pub title: String,
    /// Longer description.
    #[serde(default)]
    pub description: String,
}

/// A complete import payload — the same shape an author tool emits as
/// `ontology.json`.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct ImportDraft {
    /// Ontology name (informational; types are registered by their own
    /// names).
    #[serde(default)]
    pub name: String,
    /// Object types.
    #[serde(default)]
    pub objects: Vec<ImportObject>,
    /// Property types.
    #[serde(default)]
    pub properties: Vec<ImportProperty>,
    /// Link types.
    #[serde(default)]
    pub links: Vec<ImportLink>,
}

/// Counts of what an import registered.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct ImportReport {
    /// Object types upserted.
    pub objects: usize,
    /// Property types upserted.
    pub properties: usize,
    /// Link types upserted.
    pub links: usize,
}

/// Register every type in `draft` into the store. Objects are written
/// first, then properties and links (which reference objects). Fails
/// closed if any property owner or link endpoint is not defined in the
/// same draft.
pub async fn import_draft(store: &Store, draft: &ImportDraft) -> Result<ImportReport> {
    let defined: BTreeSet<&str> = draft.objects.iter().map(|o| o.name.as_str()).collect();
    check_unique(draft)?;
    check_refs(draft, &defined)?;

    let body = json!({});
    let mut report = ImportReport::default();

    for o in &draft.objects {
        let obj_body = if o.requires_purpose {
            json!({ "requires_purpose": true })
        } else {
            body.clone()
        };
        store
            .upsert_object(&o.name, 1, false, &o.title, &o.description, obj_body, None)
            .await?;
        report.objects += 1;
    }
    for p in &draft.properties {
        store
            .upsert_property(
                &p.name,
                1,
                false,
                &p.title,
                &p.description,
                OwnerKind::Object,
                &p.owner,
                &p.datatype,
                p.required,
                body.clone(),
                None,
            )
            .await?;
        report.properties += 1;
    }
    for l in &draft.links {
        store
            .upsert_link(
                &l.name,
                1,
                false,
                &l.title,
                &l.description,
                &l.from,
                &l.to,
                body.clone(),
                None,
            )
            .await?;
        report.links += 1;
    }

    Ok(report)
}

fn check_refs(draft: &ImportDraft, defined: &BTreeSet<&str>) -> Result<()> {
    let mut missing: Vec<String> = Vec::new();
    for p in &draft.properties {
        if !defined.contains(p.owner.as_str()) {
            missing.push(format!("property '{}' owner '{}'", p.name, p.owner));
        }
    }
    for l in &draft.links {
        if !defined.contains(l.from.as_str()) {
            missing.push(format!("link '{}' from '{}'", l.name, l.from));
        }
        if !defined.contains(l.to.as_str()) {
            missing.push(format!("link '{}' to '{}'", l.name, l.to));
        }
    }
    if missing.is_empty() {
        Ok(())
    } else {
        Err(Error::ImportClosure {
            missing: missing.join("; "),
        })
    }
}

/// Reject a draft that names the same object, property or link twice.
/// Duplicates would otherwise upsert in order and the second write could
/// conflict *after* the first already landed, leaving a partial import.
fn check_unique(draft: &ImportDraft) -> Result<()> {
    let mut dupes: Vec<String> = Vec::new();
    let mut seen: BTreeSet<&str> = BTreeSet::new();
    for o in &draft.objects {
        if !seen.insert(o.name.as_str()) {
            dupes.push(format!("object '{}'", o.name));
        }
    }
    seen.clear();
    for p in &draft.properties {
        if !seen.insert(p.name.as_str()) {
            dupes.push(format!("property '{}'", p.name));
        }
    }
    seen.clear();
    for l in &draft.links {
        if !seen.insert(l.name.as_str()) {
            dupes.push(format!("link '{}'", l.name));
        }
    }
    if dupes.is_empty() {
        Ok(())
    } else {
        Err(Error::ImportClosure {
            missing: format!("duplicate type name(s): {}", dupes.join("; ")),
        })
    }
}
