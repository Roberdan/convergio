//! Convert a validated [`DraftOntology`] into ontology record structs.
//!
//! The records reuse the runtime's authoritative hashing: each record's
//! `content_hash` is computed over the same canonical semantic shape the
//! store hashes on `upsert`, so a draft exported here lines up byte-for-
//! byte with the same ontology later imported into the registry. All
//! drafts land at `schema_version = 1`, `breaking = false`.

use chrono::{DateTime, Utc};
use convergio_ontology::{
    content_hash, LinkTypeRecord, ObjectTypeRecord, OwnerKind, PropertyTypeRecord,
};
use serde_json::{json, Value};

use crate::draft::DraftOntology;
use crate::draft_names::normalize_datatype;
use crate::error::{AuthorError, Result};

/// The three record families produced from a draft.
pub struct OntologyRecords {
    /// Object types.
    pub objects: Vec<ObjectTypeRecord>,
    /// Link types.
    pub links: Vec<LinkTypeRecord>,
    /// Property types, keyed implicitly by `owner_name`.
    pub properties: Vec<PropertyTypeRecord>,
}

fn empty_body() -> Value {
    json!({})
}

/// Build records from a draft, stamping every row with `created_at`.
/// Assumes the draft already passed validation (datatypes normalise).
pub fn build_records(draft: &DraftOntology, created_at: DateTime<Utc>) -> Result<OntologyRecords> {
    let body = empty_body();

    let mut objects = Vec::with_capacity(draft.objects.len());
    for o in &draft.objects {
        let semantic = json!({
            "kind": "object", "name": o.name, "schema_version": 1,
            "breaking": false, "title": o.title, "description": o.description,
            "body": body,
        });
        objects.push(ObjectTypeRecord {
            name: o.name.clone(),
            schema_version: 1,
            breaking: false,
            title: o.title.clone(),
            description: o.description.clone(),
            body: body.clone(),
            content_hash: content_hash(&semantic)
                .map_err(|e| AuthorError::Records(e.to_string()))?,
            created_at,
            audit_seq: None,
        });
    }

    let mut properties = Vec::with_capacity(draft.properties.len());
    for p in &draft.properties {
        let datatype = normalize_datatype(&p.datatype)
            .ok_or_else(|| AuthorError::Records(format!("unknown datatype '{}'", p.datatype)))?;
        let semantic = json!({
            "kind": "property", "name": p.name, "schema_version": 1,
            "breaking": false, "title": p.title, "description": p.description,
            "owner_kind": "object", "owner_name": p.owner, "datatype": datatype,
            "required": p.required, "body": body,
        });
        properties.push(PropertyTypeRecord {
            name: p.name.clone(),
            schema_version: 1,
            breaking: false,
            title: p.title.clone(),
            description: p.description.clone(),
            owner_kind: OwnerKind::Object,
            owner_name: p.owner.clone(),
            datatype: datatype.to_string(),
            required: p.required,
            body: body.clone(),
            content_hash: content_hash(&semantic)
                .map_err(|e| AuthorError::Records(e.to_string()))?,
            created_at,
            audit_seq: None,
        });
    }

    let mut links = Vec::with_capacity(draft.links.len());
    for l in &draft.links {
        let semantic = json!({
            "kind": "link", "name": l.name, "schema_version": 1,
            "breaking": false, "title": l.title, "description": l.description,
            "from": l.from, "to": l.to, "body": body,
        });
        links.push(LinkTypeRecord {
            name: l.name.clone(),
            schema_version: 1,
            breaking: false,
            title: l.title.clone(),
            description: l.description.clone(),
            from_object: l.from.clone(),
            to_object: l.to.clone(),
            body: body.clone(),
            content_hash: content_hash(&semantic)
                .map_err(|e| AuthorError::Records(e.to_string()))?,
            created_at,
            audit_seq: None,
        });
    }

    Ok(OntologyRecords {
        objects,
        links,
        properties,
    })
}

impl OntologyRecords {
    /// Properties owned by the given object name, in document order.
    pub fn properties_of<'a>(&'a self, object: &str) -> Vec<&'a PropertyTypeRecord> {
        self.properties
            .iter()
            .filter(|p| p.owner_name == object)
            .collect()
    }
}
