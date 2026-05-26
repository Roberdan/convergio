use super::helpers;
use super::Ops;
use crate::engine::{EngineEvent, EngineTickOutcome, WorkflowEngine};
use crate::model::{OpsWorkflowInstance, OpsWorkflowInstanceStatus};
use crate::state::WorkflowInstanceState;
use chrono::Utc;
use convergio_durability::audit::{append_tx, EntityKind};
use convergio_durability::error::{DurabilityError, Result};
use serde_json::json;
use uuid::Uuid;

impl Ops {
    /// Start a new workflow instance pinned to a workflow id and (optional) version.
    pub async fn start_instance(
        &self,
        workflow_id: &str,
        workflow_version: Option<i64>,
        context: serde_json::Value,
        agent_id: Option<&str>,
    ) -> Result<OpsWorkflowInstance> {
        let wf = if let Some(v) = workflow_version {
            self.workflows().get_version(workflow_id, v).await?
        } else {
            self.workflows().get_current(workflow_id).await?
        };

        let engine = WorkflowEngine::new(wf.spec.clone());
        let instance_id = Uuid::new_v4().to_string();
        let now = Utc::now();
        let state = engine.start(instance_id.clone(), context);

        let mut tx = self.pool.inner().begin().await?;
        sqlx::query(
            "INSERT INTO ops_workflow_instances \
             (instance_id, workflow_id, workflow_version, status, state_json, valid_from, valid_to, system_from, system_to, created_by_agent, created_at) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(&instance_id)
        .bind(&wf.workflow_id)
        .bind(wf.version)
        .bind(OpsWorkflowInstanceStatus::Running.as_str())
        .bind(serde_json::to_string(&state)?)
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
            EntityKind::OpsWorkflowInstance,
            &instance_id,
            "ops_instance.started",
            &json!({
                "instance_id": instance_id,
                "workflow_id": wf.workflow_id,
                "workflow_version": wf.version,
                "agent_id": agent_id,
            }),
            agent_id,
        )
        .await?;

        tx.commit().await?;

        Ok(OpsWorkflowInstance {
            instance_id,
            workflow_id: wf.workflow_id,
            workflow_version: wf.version,
            status: OpsWorkflowInstanceStatus::Running,
            state,
            valid_from: now,
            valid_to: None,
            system_from: now,
            system_to: None,
            created_at: now,
            created_by_agent: agent_id.map(str::to_string),
        })
    }

    /// Tick an instance (advances cursors and emits/creates work items).
    pub async fn tick_instance(
        &self,
        instance_id: &str,
        agent_id: Option<&str>,
    ) -> Result<OpsWorkflowInstance> {
        let now = Utc::now();
        let current = self.instances().get_current(instance_id).await?;
        let wf = self
            .workflows()
            .get_version(&current.workflow_id, current.workflow_version)
            .await?;
        let engine = WorkflowEngine::new(wf.spec);

        let mut status = current.status;
        let mut out = engine.tick(current.state, now);

        if out
            .events
            .iter()
            .any(|e| matches!(e, EngineEvent::WorkItemFailed { .. }))
        {
            out.state.compensation_target_status = Some("failed".into());
            status = OpsWorkflowInstanceStatus::Compensating;
            let comp = engine.begin_compensation(out.state, now);
            out = EngineTickOutcome {
                state: comp.state,
                events: helpers::merge_events(out.events, comp.events),
            };
        }

        if matches!(status, OpsWorkflowInstanceStatus::Running)
            && out
                .events
                .iter()
                .any(|e| matches!(e, EngineEvent::Completed))
        {
            status = OpsWorkflowInstanceStatus::Completed;
        }

        if matches!(status, OpsWorkflowInstanceStatus::Compensating)
            && out
                .events
                .iter()
                .any(|e| matches!(e, EngineEvent::Completed))
        {
            status = helpers::parse_terminal_target(&out.state.compensation_target_status)?;
            out.state.compensation_target_status = None;
        }

        let audit_payload = json!({
            "instance_id": instance_id,
            "workflow_id": current.workflow_id,
            "workflow_version": current.workflow_version,
            "from_status": current.status.as_str(),
            "to_status": status.as_str(),
            "events": out.events,
            "agent_id": agent_id,
        });

        helpers::persist_instance_snapshot(helpers::InstanceSnapshot {
            pool: &self.pool,
            instance_id,
            workflow_id: &current.workflow_id,
            workflow_version: current.workflow_version,
            status,
            state: &out.state,
            now,
            transition: "ops_instance.ticked",
            audit_payload: &audit_payload,
            agent_id,
        })
        .await?;

        self.instances().get_snapshot(instance_id, now, now).await
    }

    /// Mark one work item completed (or failed) and persist a new instance snapshot.
    pub async fn complete_work_item(
        &self,
        instance_id: &str,
        work_item_id: &str,
        success: bool,
        agent_id: Option<&str>,
    ) -> Result<OpsWorkflowInstance> {
        let now = Utc::now();
        let current = self.instances().get_current(instance_id).await?;
        let wf = self
            .workflows()
            .get_version(&current.workflow_id, current.workflow_version)
            .await?;
        let engine = WorkflowEngine::new(wf.spec);

        let state = engine.complete_work_item(current.state, work_item_id, success);

        let audit_payload = json!({
            "instance_id": instance_id,
            "work_item_id": work_item_id,
            "success": success,
            "agent_id": agent_id,
        });

        helpers::persist_instance_snapshot(helpers::InstanceSnapshot {
            pool: &self.pool,
            instance_id,
            workflow_id: &current.workflow_id,
            workflow_version: current.workflow_version,
            status: current.status,
            state: &state,
            now,
            transition: "ops_instance.work_item_completed",
            audit_payload: &audit_payload,
            agent_id,
        })
        .await?;

        self.instances().get_snapshot(instance_id, now, now).await
    }

    /// Cancel an instance by entering compensation mode.
    pub async fn cancel_instance(
        &self,
        instance_id: &str,
        agent_id: Option<&str>,
    ) -> Result<OpsWorkflowInstance> {
        let now = Utc::now();
        let current = self.instances().get_current(instance_id).await?;

        if matches!(
            current.status,
            OpsWorkflowInstanceStatus::Completed
                | OpsWorkflowInstanceStatus::Failed
                | OpsWorkflowInstanceStatus::Cancelled
        ) {
            return Err(DurabilityError::InvalidOpsWorkflowInstance {
                reason: format!("instance is terminal ({})", current.status.as_str()),
            });
        }

        let wf = self
            .workflows()
            .get_version(&current.workflow_id, current.workflow_version)
            .await?;
        let engine = WorkflowEngine::new(wf.spec);

        let mut state: WorkflowInstanceState = current.state;
        state.cursors.clear();
        state.compensation_target_status = Some("cancelled".into());
        let comp = engine.begin_compensation(state, now);

        let audit_payload = json!({
            "instance_id": instance_id,
            "workflow_id": current.workflow_id,
            "workflow_version": current.workflow_version,
            "from_status": current.status.as_str(),
            "to_status": "compensating",
            "events": comp.events,
            "agent_id": agent_id,
        });

        helpers::persist_instance_snapshot(helpers::InstanceSnapshot {
            pool: &self.pool,
            instance_id,
            workflow_id: &current.workflow_id,
            workflow_version: current.workflow_version,
            status: OpsWorkflowInstanceStatus::Compensating,
            state: &comp.state,
            now,
            transition: "ops_instance.cancelled",
            audit_payload: &audit_payload,
            agent_id,
        })
        .await?;

        self.instances().get_snapshot(instance_id, now, now).await
    }
}
