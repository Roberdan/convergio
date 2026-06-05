use crate::engine::EngineEvent;
use crate::model::OpsWorkflowInstanceStatus;
use crate::state::WorkflowInstanceState;
use chrono::{DateTime, Utc};
use convergio_db::Pool;
use convergio_durability::audit::{append_tx, EntityKind};
use convergio_durability::error::{DurabilityError, Result};

pub(super) fn merge_events(mut a: Vec<EngineEvent>, b: Vec<EngineEvent>) -> Vec<EngineEvent> {
    a.extend(b);
    a
}

pub(super) fn parse_terminal_target(tag: &Option<String>) -> Result<OpsWorkflowInstanceStatus> {
    match tag.as_deref() {
        Some("failed") => Ok(OpsWorkflowInstanceStatus::Failed),
        Some("cancelled") => Ok(OpsWorkflowInstanceStatus::Cancelled),
        Some(other) => Err(DurabilityError::InvalidOpsWorkflowInstance {
            reason: format!("unknown compensation target '{other}'"),
        }),
        None => Ok(OpsWorkflowInstanceStatus::Failed),
    }
}

pub(super) struct InstanceSnapshot<'a, P> {
    pub(super) pool: &'a Pool,
    pub(super) instance_id: &'a str,
    pub(super) workflow_id: &'a str,
    pub(super) workflow_version: i64,
    pub(super) status: OpsWorkflowInstanceStatus,
    pub(super) state: &'a WorkflowInstanceState,
    pub(super) now: DateTime<Utc>,
    pub(super) transition: &'a str,
    pub(super) audit_payload: &'a P,
    pub(super) agent_id: Option<&'a str>,
}

pub(super) async fn persist_instance_snapshot<P: serde::Serialize>(
    args: InstanceSnapshot<'_, P>,
) -> Result<()> {
    let mut tx = args.pool.inner().begin().await?;
    sqlx::query(
        "UPDATE ops_workflow_instances SET system_to = ? WHERE instance_id = ? AND system_to IS NULL",
    )
    .bind(args.now.to_rfc3339())
    .bind(args.instance_id)
    .execute(&mut *tx)
    .await?;

    sqlx::query(
        "INSERT INTO ops_workflow_instances \
         (instance_id, workflow_id, workflow_version, status, state_json, valid_from, valid_to, system_from, system_to, created_by_agent, created_at) \
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(args.instance_id)
    .bind(args.workflow_id)
    .bind(args.workflow_version)
    .bind(args.status.as_str())
    .bind(serde_json::to_string(args.state)?)
    .bind(args.now.to_rfc3339())
    .bind(Option::<String>::None)
    .bind(args.now.to_rfc3339())
    .bind(Option::<String>::None)
    .bind(args.agent_id)
    .bind(args.now.to_rfc3339())
    .execute(&mut *tx)
    .await?;

    append_tx(
        &mut tx,
        EntityKind::OpsWorkflowInstance,
        args.instance_id,
        args.transition,
        args.audit_payload,
        args.agent_id,
    )
    .await?;

    tx.commit().await?;
    Ok(())
}
