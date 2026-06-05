//! W3C PROV-JSON provenance bundles for Convergio audit events.
//!
//! The crate stays leaf-only: it serializes standards-shaped bundles but
//! does not know about SQLite, HTTP, or Convergio's audit store.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// A W3C PROV-JSON bundle.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProvBundle {
    /// PROV namespace declarations.
    #[serde(rename = "prefix", default, skip_serializing_if = "BTreeMap::is_empty")]
    pub prefixes: BTreeMap<String, String>,
    /// PROV `Activity` nodes.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub activity: Vec<Activity>,
    /// PROV `Agent` nodes.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub agent: Vec<Agent>,
    /// PROV `Entity` nodes.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub entity: Vec<Entity>,
    /// PROV `wasGeneratedBy` relations.
    #[serde(
        rename = "wasGeneratedBy",
        default,
        skip_serializing_if = "Vec::is_empty"
    )]
    pub was_generated_by: Vec<WasGeneratedBy>,
    /// PROV `wasAssociatedWith` relations.
    #[serde(
        rename = "wasAssociatedWith",
        default,
        skip_serializing_if = "Vec::is_empty"
    )]
    pub was_associated_with: Vec<WasAssociatedWith>,
    /// PROV `used` relations.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub used: Vec<Used>,
}

/// A PROV `Activity`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Activity {
    /// Opaque globally unique identifier.
    pub id: String,
    /// Activity kind, e.g. `audit.task.done`.
    pub kind: String,
    /// Start instant.
    pub started_at: DateTime<Utc>,
    /// End instant, when known.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ended_at: Option<DateTime<Utc>>,
}

/// A PROV `Agent`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Agent {
    /// Opaque identifier.
    pub id: String,
    /// Free-form display label.
    pub label: String,
}

/// A PROV `Entity`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Entity {
    /// Opaque identifier.
    pub id: String,
    /// Entity kind.
    pub kind: String,
}

/// A PROV `wasGeneratedBy` relation.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WasGeneratedBy {
    /// Relation identifier.
    pub id: String,
    /// Generated entity id.
    pub entity: String,
    /// Generating activity id.
    pub activity: String,
}

/// A PROV `wasAssociatedWith` relation.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WasAssociatedWith {
    /// Relation identifier.
    pub id: String,
    /// Activity id.
    pub activity: String,
    /// Responsible agent id.
    pub agent: String,
}

/// A PROV `used` relation.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Used {
    /// Relation identifier.
    pub id: String,
    /// Activity id.
    pub activity: String,
    /// Used entity id.
    pub entity: String,
}

/// Error type for provenance serialization.
#[derive(Debug, thiserror::Error)]
pub enum ProvenanceError {
    /// JSON serialization failed.
    #[error("prov-json serialization failed: {0}")]
    Json(#[from] serde_json::Error),
    /// Required identifier was empty.
    #[error("empty provenance identifier: {0}")]
    EmptyIdentifier(&'static str),
}

/// Emit a minimal but complete PROV-JSON bundle for one activity,
/// responsible agent, and generated entity.
pub fn emit_bundle(
    activity: Activity,
    agent: Agent,
    entity: Entity,
) -> Result<ProvBundle, ProvenanceError> {
    validate_id("activity.id", &activity.id)?;
    validate_id("agent.id", &agent.id)?;
    validate_id("entity.id", &entity.id)?;
    let mut prefixes = BTreeMap::new();
    prefixes.insert("prov".into(), "http://www.w3.org/ns/prov#".into());
    prefixes.insert(
        "cvg".into(),
        "https://github.com/Roberdan/convergio#".into(),
    );
    Ok(ProvBundle {
        prefixes,
        was_generated_by: vec![WasGeneratedBy {
            id: format!("wgb:{}:{}", entity.id, activity.id),
            entity: entity.id.clone(),
            activity: activity.id.clone(),
        }],
        was_associated_with: vec![WasAssociatedWith {
            id: format!("waw:{}:{}", activity.id, agent.id),
            activity: activity.id.clone(),
            agent: agent.id.clone(),
        }],
        activity: vec![activity],
        agent: vec![agent],
        entity: vec![entity],
        used: Vec::new(),
    })
}

/// Serialize a bundle to deterministic JSON bytes.
pub fn to_prov_json(bundle: &ProvBundle) -> Result<Vec<u8>, ProvenanceError> {
    Ok(serde_json::to_vec(bundle)?)
}

fn validate_id(label: &'static str, value: &str) -> Result<(), ProvenanceError> {
    if value.trim().is_empty() {
        Err(ProvenanceError::EmptyIdentifier(label))
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn emit_bundle_serializes_relations() {
        let now = Utc::now();
        let bundle = emit_bundle(
            Activity {
                id: "act-1".into(),
                kind: "audit.task.done".into(),
                started_at: now,
                ended_at: Some(now),
            },
            Agent {
                id: "agent-1".into(),
                label: "copilot".into(),
            },
            Entity {
                id: "audit-42".into(),
                kind: "audit.entry".into(),
            },
        )
        .unwrap();
        assert_eq!(bundle.was_generated_by[0].entity, "audit-42");
        let json = String::from_utf8(to_prov_json(&bundle).unwrap()).unwrap();
        assert!(json.contains("wasGeneratedBy"));
        assert!(json.contains("wasAssociatedWith"));
    }
}
