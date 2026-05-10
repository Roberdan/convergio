use crate::audit::{append_tx, EntityKind};
use crate::error::{DurabilityError, Result};
use crate::model::Evidence;
use crate::Durability;
use chrono::Utc;
use serde::Deserialize;
use serde_json::{json, Value};
use uuid::Uuid;

#[derive(Debug, Clone, Deserialize)]
struct UsageEvidencePayload {
    input_tokens: u64,
    output_tokens: u64,
    model: String,
    #[serde(default)]
    cost_usd: Option<f64>,
}

impl UsageEvidencePayload {
    fn validate(&self) -> Result<()> {
        if self.model.trim().is_empty() {
            return Err(DurabilityError::InvalidEvidence {
                reason: "usage evidence requires non-empty model".into(),
            });
        }
        if let Some(cost) = self.cost_usd {
            if !cost.is_finite() || cost < 0.0 {
                return Err(DurabilityError::InvalidEvidence {
                    reason: "usage evidence cost_usd must be a finite non-negative number".into(),
                });
            }
        }
        Ok(())
    }

    fn as_json(&self) -> Value {
        json!({
            "input_tokens": self.input_tokens,
            "output_tokens": self.output_tokens,
            "model": self.model.as_str(),
            "cost_usd": self.cost_usd,
        })
    }
}

fn parse_usage_payload(payload: &Value) -> Result<UsageEvidencePayload> {
    let usage: UsageEvidencePayload =
        serde_json::from_value(payload.clone()).map_err(|e| DurabilityError::InvalidEvidence {
            reason: format!("usage evidence payload invalid: {e}"),
        })?;
    usage.validate()?;
    Ok(usage)
}

impl Durability {
    /// Attach evidence to a task (audited).
    pub async fn attach_evidence(
        &self,
        task_id: &str,
        kind: &str,
        payload: serde_json::Value,
        exit_code: Option<i64>,
    ) -> Result<Evidence> {
        let task = self.tasks().get(task_id).await?;
        let usage = if kind == "usage" {
            Some(parse_usage_payload(&payload)?)
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
        let (agent_id, agent_usage_totals) = if let (Some(agent_id), Some(usage)) =
            (task.agent_id.as_deref(), usage.as_ref())
        {
            let totals = update_agent_usage_totals_tx(&mut tx, agent_id, task_id, usage).await?;
            (Some(agent_id.to_string()), totals)
        } else {
            (task.agent_id.clone(), None)
        };
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
                "agent_id": agent_id,
                "usage": usage.as_ref().map(UsageEvidencePayload::as_json),
                "agent_usage_totals": agent_usage_totals,
            }),
            None,
        )
        .await?;
        tx.commit().await?;
        Ok(evidence)
    }

    /// Remove one evidence row by id (audited).
    pub async fn remove_evidence(&self, evidence_id: &str) -> Result<Evidence> {
        let evidence = self.evidence().get(evidence_id).await?;
        let mut tx = self.pool().inner().begin().await?;
        let res = sqlx::query("DELETE FROM evidence WHERE id = ?")
            .bind(&evidence.id)
            .execute(&mut *tx)
            .await?;
        if res.rows_affected() == 0 {
            return Err(DurabilityError::NotFound {
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

async fn update_agent_usage_totals_tx(
    tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
    agent_id: &str,
    task_id: &str,
    usage: &UsageEvidencePayload,
) -> Result<Option<Value>> {
    let row: Option<(String,)> = sqlx::query_as("SELECT metadata FROM agents WHERE id = ? LIMIT 1")
        .bind(agent_id)
        .fetch_optional(&mut **tx)
        .await?;
    let Some((raw,)) = row else {
        return Ok(None);
    };
    let mut meta: Value = serde_json::from_str(&raw).unwrap_or_else(|_| json!({}));
    if !meta.is_object() {
        meta = json!({});
    }
    let now = Utc::now().to_rfc3339();
    let input_total = meta
        .get("usage")
        .and_then(|v| v.get("input_tokens"))
        .and_then(Value::as_u64)
        .unwrap_or(0)
        .saturating_add(usage.input_tokens);
    let output_total = meta
        .get("usage")
        .and_then(|v| v.get("output_tokens"))
        .and_then(Value::as_u64)
        .unwrap_or(0)
        .saturating_add(usage.output_tokens);
    let mut cost_total = meta
        .get("usage")
        .and_then(|v| v.get("cost_usd"))
        .and_then(Value::as_f64)
        .unwrap_or(0.0);
    if let Some(cost) = usage.cost_usd {
        cost_total += cost;
    }
    meta["usage"] = json!({
        "input_tokens": input_total,
        "output_tokens": output_total,
        "cost_usd": cost_total,
        "last_model": usage.model.as_str(),
        "last_task_id": task_id,
        "updated_at": now,
    });
    let meta_json = serde_json::to_string(&meta)?;
    let rows = sqlx::query("UPDATE agents SET metadata = ?, updated_at = ? WHERE id = ?")
        .bind(meta_json)
        .bind(&now)
        .bind(agent_id)
        .execute(&mut **tx)
        .await?
        .rows_affected();
    if rows == 0 {
        return Ok(None);
    }
    Ok(Some(meta["usage"].clone()))
}
