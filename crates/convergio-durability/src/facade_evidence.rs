//! Evidence facade helpers.

use crate::audit::{append_tx, EntityKind};
use crate::error::Result;
use crate::facade::Durability;
use crate::model::Evidence;
use crate::usage_evidence::parse_usage;
use chrono::Utc;
use serde_json::json;
use uuid::Uuid;

impl Durability {
    /// Attach evidence to a task and write the audit row.
    ///
    /// Special case: `kind == "usage"` validates the payload schema and
    /// accumulates token + cost rollups into `tasks` and cached totals in
    /// `plans` / `agents`, all in the same transaction as the evidence insert.
    pub async fn attach_evidence(
        &self,
        task_id: &str,
        kind: &str,
        payload: serde_json::Value,
        exit_code: Option<i64>,
    ) -> Result<Evidence> {
        // Confirm task exists.
        self.tasks().get(task_id).await?;

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

        if kind == "usage" {
            let usage = parse_usage(evidence.payload.clone())?;
            let total_tokens = usage.total_tokens();

            // Task-level accumulator.
            sqlx::query(
                "UPDATE tasks SET tokens = tokens + ?, cost_usd = cost_usd + ? WHERE id = ?",
            )
            .bind(total_tokens)
            .bind(usage.cost_usd)
            .bind(task_id)
            .execute(&mut *tx)
            .await?;

            // Plan/agent cached totals.
            let (plan_id, agent_id): (String, Option<String>) =
                sqlx::query_as("SELECT plan_id, agent_id FROM tasks WHERE id = ? LIMIT 1")
                    .bind(task_id)
                    .fetch_one(&mut *tx)
                    .await?;

            sqlx::query(
                "UPDATE plans SET \
                     tokens = COALESCE((SELECT SUM(tokens) FROM tasks WHERE plan_id = ?), 0), \
                     cost_usd = COALESCE((SELECT SUM(cost_usd) FROM tasks WHERE plan_id = ?), 0) \
                 WHERE id = ?",
            )
            .bind(&plan_id)
            .bind(&plan_id)
            .bind(&plan_id)
            .execute(&mut *tx)
            .await?;

            if let Some(agent_id) = agent_id {
                sqlx::query(
                    "UPDATE agents SET \
                         tokens = COALESCE((SELECT SUM(tokens) FROM tasks WHERE agent_id = ?), 0), \
                         cost_usd = COALESCE((SELECT SUM(cost_usd) FROM tasks WHERE agent_id = ?), 0) \
                     WHERE id = ?",
                )
                .bind(&agent_id)
                .bind(&agent_id)
                .bind(&agent_id)
                .execute(&mut *tx)
                .await?;
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
