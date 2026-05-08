//! Evidence facade operations.

use crate::audit::{append_tx, EntityKind};
use crate::error::Result;
use crate::facade::Durability;
use crate::model::Evidence;
use chrono::Utc;
use serde_json::json;
use uuid::Uuid;

use crate::usage;
use crate::usage_telemetry;

impl Durability {
    /// Attach evidence to a task and write the audit row.
    pub async fn attach_evidence(
        &self,
        task_id: &str,
        kind: &str,
        payload: serde_json::Value,
        exit_code: Option<i64>,
    ) -> Result<Evidence> {
        // Confirm task exists.
        let task = self.tasks().get(task_id).await?;

        // Validate/parse protocol-level evidence payloads before persisting.
        let parsed_usage = if kind == "usage" {
            Some(usage::parse_usage_payload(&payload)?)
        } else {
            None
        };

        let evidence = Evidence {
            id: Uuid::new_v4().to_string(),
            task_id: task_id.to_string(),
            kind: kind.to_string(),
            payload,
            exit_code,
            created_at: Utc::now(),
        };

        let mut tx = self.pool().inner().begin().await?;
        sqlx::query(
            "INSERT INTO evidence (id, task_id, kind, payload, exit_code, created_at) \
             VALUES (?, ?, ?, ?, ?, ?)",
        )
        .bind(&evidence.id)
        .bind(&evidence.task_id)
        .bind(&evidence.kind)
        .bind(serde_json::to_string(&evidence.payload)?)
        .bind(evidence.exit_code)
        .bind(evidence.created_at.to_rfc3339())
        .execute(&mut *tx)
        .await?;
        if let Some(parsed) = parsed_usage.as_ref() {
            // Aggregate into the durable agent registry. Prefer the task owner,
            // but also update any registered agent currently pointing at this
            // task (covers `cvg agent spawn` which heartbeats `current_task_id`).
            let mut agent_ids: Vec<String> = Vec::new();
            if let Some(owner) = task.agent_id.as_deref() {
                agent_ids.push(owner.to_string());
            }
            let mut active: Vec<String> =
                sqlx::query_scalar("SELECT id FROM agents WHERE current_task_id = ?")
                    .bind(task_id)
                    .fetch_all(&mut *tx)
                    .await
                    .unwrap_or_default();
            agent_ids.append(&mut active);
            agent_ids.sort();
            agent_ids.dedup();

            for id in agent_ids {
                usage_telemetry::apply_usage_evidence_tx(&mut tx, Some(&id), parsed).await?;
            }
        }
        append_tx(
            &mut tx,
            EntityKind::Evidence,
            &evidence.id,
            "evidence.attached",
            &json!({
                "evidence_id": evidence.id,
                "task_id": task_id,
                "kind": kind,
                "exit_code": exit_code,
                "agent_id": task.agent_id,
            }),
            None,
        )
        .await?;
        tx.commit().await?;
        Ok(evidence)
    }

    /// Remove evidence by id and write the audit row. Returns the
    /// row that was deleted so callers can echo it back. The audit
    /// payload preserves enough context (`task_id`, `kind`) to make
    /// the deletion forensically reconstructible.
    pub async fn remove_evidence(&self, evidence_id: &str) -> Result<Evidence> {
        let evidence = self.evidence().get(evidence_id).await?;

        let mut tx = self.pool().inner().begin().await?;
        let res = sqlx::query("DELETE FROM evidence WHERE id = ?")
            .bind(&evidence.id)
            .execute(&mut *tx)
            .await?;
        if res.rows_affected() == 0 {
            return Err(crate::error::DurabilityError::NotFound {
                entity: "evidence",
                id: evidence_id.to_string(),
            });
        }
        append_tx(
            &mut tx,
            EntityKind::Evidence,
            &evidence.id,
            "evidence.removed",
            &json!({
                "evidence_id": evidence.id,
                "task_id": evidence.task_id,
                "kind": evidence.kind,
                "exit_code": evidence.exit_code,
            }),
            None,
        )
        .await?;
        tx.commit().await?;
        Ok(evidence)
    }
}
