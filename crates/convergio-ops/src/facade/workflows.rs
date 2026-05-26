use super::Ops;
use crate::model::OpsWorkflow;
use crate::spec::WorkflowSpec;
use chrono::Utc;
use convergio_durability::audit::{append_tx, EntityKind};
use convergio_durability::error::{DurabilityError, Result};
use serde_json::json;
use uuid::Uuid;

impl Ops {
    /// Create a new workflow definition (version 1) with bitemporal posture.
    pub async fn create_workflow(
        &self,
        workflow_key: &str,
        spec: &WorkflowSpec,
        agent_id: Option<&str>,
    ) -> Result<OpsWorkflow> {
        if workflow_key.trim().is_empty() {
            return Err(DurabilityError::InvalidOpsWorkflow {
                reason: "workflow_key must be non-empty".into(),
            });
        }
        spec.validate()
            .map_err(|reason| DurabilityError::InvalidOpsWorkflow { reason })?;

        let now = Utc::now();
        let workflow_id = Uuid::new_v4().to_string();
        let version = 1_i64;

        let mut tx = self.pool.inner().begin().await?;
        sqlx::query(
            "INSERT INTO ops_workflows \
             (workflow_id, workflow_key, version, spec_json, valid_from, valid_to, system_from, system_to, created_by_agent, created_at) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(&workflow_id)
        .bind(workflow_key)
        .bind(version)
        .bind(serde_json::to_string(spec)?)
        .bind(now.to_rfc3339())
        .bind(Option::<String>::None)
        .bind(now.to_rfc3339())
        .bind(Option::<String>::None)
        .bind(agent_id)
        .bind(now.to_rfc3339())
        .execute(&mut *tx)
        .await?;

        append_tx(
            &mut tx,
            EntityKind::OpsWorkflow,
            &workflow_id,
            "ops_workflow.created",
            &json!({
                "workflow_id": workflow_id,
                "workflow_key": workflow_key,
                "version": version,
                "agent_id": agent_id,
            }),
            agent_id,
        )
        .await?;

        tx.commit().await?;
        self.workflows().get_snapshot(&workflow_id, now, now).await
    }

    /// Append the next workflow version (closes current system-time row).
    pub async fn append_workflow_version(
        &self,
        workflow_id: &str,
        spec: &WorkflowSpec,
        agent_id: Option<&str>,
    ) -> Result<OpsWorkflow> {
        spec.validate()
            .map_err(|reason| DurabilityError::InvalidOpsWorkflow { reason })?;

        let current = self.workflows().get_current(workflow_id).await?;
        let now = Utc::now();
        let next_version = current.version + 1;

        let mut tx = self.pool.inner().begin().await?;
        sqlx::query(
            "UPDATE ops_workflows SET system_to = ? WHERE workflow_id = ? AND system_to IS NULL",
        )
        .bind(now.to_rfc3339())
        .bind(workflow_id)
        .execute(&mut *tx)
        .await?;

        sqlx::query(
            "INSERT INTO ops_workflows \
             (workflow_id, workflow_key, version, spec_json, valid_from, valid_to, system_from, system_to, created_by_agent, created_at) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(workflow_id)
        .bind(&current.workflow_key)
        .bind(next_version)
        .bind(serde_json::to_string(spec)?)
        .bind(now.to_rfc3339())
        .bind(Option::<String>::None)
        .bind(now.to_rfc3339())
        .bind(Option::<String>::None)
        .bind(agent_id)
        .bind(now.to_rfc3339())
        .execute(&mut *tx)
        .await?;

        append_tx(
            &mut tx,
            EntityKind::OpsWorkflow,
            workflow_id,
            "ops_workflow.version_appended",
            &json!({
                "workflow_id": workflow_id,
                "workflow_key": current.workflow_key,
                "from_version": current.version,
                "to_version": next_version,
                "agent_id": agent_id,
            }),
            agent_id,
        )
        .await?;

        tx.commit().await?;
        self.workflows().get_snapshot(workflow_id, now, now).await
    }
}
