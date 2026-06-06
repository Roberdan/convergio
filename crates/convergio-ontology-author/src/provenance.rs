//! W3C-PROV provenance for an authoring run (ADR-0075, ADR-0080).
//!
//! Records who/what produced the ontology: the generated ontology
//! [`Entity`], the model [`Agent`], the authoring [`Activity`], and a
//! [`Used`] relation to every source document (each itself an entity
//! identified by its content hash). Serialized to deterministic
//! PROV-JSON.

use chrono::{DateTime, Utc};
use convergio_provenance::{emit_bundle, to_prov_json, Activity, Agent, Entity, ProvBundle, Used};

use crate::error::{AuthorError, Result};
use crate::ingest::SourceDoc;

/// Build a PROV bundle for one authoring run.
pub fn build_bundle(
    ontology_name: &str,
    model_id: &str,
    docs: &[SourceDoc],
    at: DateTime<Utc>,
) -> Result<ProvBundle> {
    let activity_id = format!("cvg:ontology-author:{ontology_name}");
    let entity_id = format!("cvg:ontology:{ontology_name}");

    let activity = Activity {
        id: activity_id.clone(),
        kind: "ontology.author".to_string(),
        started_at: at,
        ended_at: Some(at),
    };
    let agent = Agent {
        id: format!("cvg:model:{model_id}"),
        label: model_id.to_string(),
    };
    let entity = Entity {
        id: entity_id,
        kind: "ontology.draft".to_string(),
    };

    let mut bundle = emit_bundle(activity, agent, entity).map_err(prov_err)?;

    for doc in docs {
        let src_id = format!("cvg:source:{}", doc.content_hash);
        bundle.entity.push(Entity {
            id: src_id.clone(),
            kind: "source.document".to_string(),
        });
        bundle.used.push(Used {
            id: format!("used:{activity_id}:{}", doc.content_hash),
            activity: activity_id.clone(),
            entity: src_id,
        });
    }

    Ok(bundle)
}

/// Serialize a bundle to PROV-JSON bytes.
pub fn bundle_json(bundle: &ProvBundle) -> Result<Vec<u8>> {
    to_prov_json(bundle).map_err(prov_err)
}

fn prov_err(e: convergio_provenance::ProvenanceError) -> AuthorError {
    AuthorError::Provenance(e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;
    use std::path::PathBuf;

    #[test]
    fn bundle_records_sources_as_used() {
        let docs = vec![SourceDoc::new(PathBuf::from("a.pdf"), "x".into())];
        let at = Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap();
        let b = build_bundle("sis", "stub:test", &docs, at).unwrap();
        assert_eq!(b.used.len(), 1);
        assert!(b.entity.iter().any(|e| e.kind == "source.document"));
        assert!(!bundle_json(&b).unwrap().is_empty());
    }
}
