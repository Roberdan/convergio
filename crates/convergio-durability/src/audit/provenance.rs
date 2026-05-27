//! Provenance emission for audit rows.

use super::{AuditEntry, AuditLog, EntityKind};
use crate::error::Result;
use chrono::{DateTime, Utc};
use convergio_provenance::{emit_bundle, to_prov_json, Activity, Agent, Entity, ProvBundle};
use serde::Serialize;
use serde_json::json;

impl AuditLog {
    /// Append an ordinary audit entry, then append a chained provenance
    /// bundle event describing that entry.
    pub async fn append_with_provenance<P: Serialize>(
        &self,
        entity: EntityKind,
        entity_id: &str,
        transition: &str,
        payload: &P,
        agent_id: Option<&str>,
    ) -> Result<(AuditEntry, AuditEntry)> {
        let entry = self
            .append(entity, entity_id, transition, payload, agent_id)
            .await?;
        let bundle = bundle_for_entry(&entry)?;
        let prov_json = to_prov_json(&bundle)?;
        let payload = json!({
            "audit_seq": entry.seq,
            "audit_hash": entry.hash,
            "prov_json": serde_json::from_slice::<serde_json::Value>(&prov_json)?,
        });
        let provenance_entry = self
            .append(
                EntityKind::Free,
                &format!("audit:{}", entry.seq),
                "provenance.bundle_emitted",
                &payload,
                agent_id,
            )
            .await?;
        Ok((entry, provenance_entry))
    }
}

/// Build a PROV bundle from an audit entry without writing it.
pub fn bundle_for_entry(entry: &AuditEntry) -> Result<ProvBundle> {
    let created_at = entry
        .created_at
        .parse::<DateTime<Utc>>()
        .unwrap_or_else(|_| Utc::now());
    let agent_id = entry.agent_id.as_deref().unwrap_or("system");
    Ok(emit_bundle(
        Activity {
            id: format!("cvg:audit:activity:{}", entry.seq),
            kind: entry.transition.clone(),
            started_at: created_at,
            ended_at: Some(created_at),
        },
        Agent {
            id: format!("cvg:agent:{agent_id}"),
            label: agent_id.to_string(),
        },
        Entity {
            id: format!("cvg:audit:entry:{}", entry.seq),
            kind: entry.entity_type.clone(),
        },
    )?)
}
