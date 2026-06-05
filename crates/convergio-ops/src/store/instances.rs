use crate::model::{OpsWorkflowInstance, OpsWorkflowInstanceStatus};
use crate::state::WorkflowInstanceState;
use chrono::{DateTime, Utc};
use convergio_db::Pool;
use convergio_durability::error::{DurabilityError, Result};

/// Read access to `ops_workflow_instances`.
#[derive(Clone)]
pub struct OpsWorkflowInstanceStore {
    pool: Pool,
}

impl OpsWorkflowInstanceStore {
    /// Wrap a pool.
    pub fn new(pool: Pool) -> Self {
        Self { pool }
    }

    /// Get the current system-time row for the given instance id.
    pub async fn get_current(&self, instance_id: &str) -> Result<OpsWorkflowInstance> {
        let row = sqlx::query_as::<_, InstanceRow>(
            "SELECT instance_id, workflow_id, workflow_version, status, state_json, valid_from, valid_to, \
                    system_from, system_to, created_by_agent, created_at \
             FROM ops_workflow_instances \
             WHERE instance_id = ? AND system_to IS NULL \
             ORDER BY system_from DESC \
             LIMIT 1",
        )
        .bind(instance_id)
        .fetch_optional(self.pool.inner())
        .await?;

        let row = row.ok_or_else(|| DurabilityError::NotFound {
            entity: "ops_workflow_instance",
            id: instance_id.to_string(),
        })?;
        row.try_into()
    }

    /// Get bitemporal snapshot.
    pub async fn get_snapshot(
        &self,
        instance_id: &str,
        as_of: DateTime<Utc>,
        valid_at: DateTime<Utc>,
    ) -> Result<OpsWorkflowInstance> {
        let row = sqlx::query_as::<_, InstanceRow>(
            "SELECT instance_id, workflow_id, workflow_version, status, state_json, valid_from, valid_to, \
                    system_from, system_to, created_by_agent, created_at \
             FROM ops_workflow_instances \
             WHERE instance_id = ? \
               AND system_from <= ? \
               AND (system_to IS NULL OR system_to > ?) \
               AND valid_from <= ? \
               AND (valid_to IS NULL OR valid_to > ?) \
             ORDER BY system_from DESC \
             LIMIT 1",
        )
        .bind(instance_id)
        .bind(as_of.to_rfc3339())
        .bind(as_of.to_rfc3339())
        .bind(valid_at.to_rfc3339())
        .bind(valid_at.to_rfc3339())
        .fetch_optional(self.pool.inner())
        .await?;

        let row = row.ok_or_else(|| DurabilityError::NotFound {
            entity: "ops_workflow_instance",
            id: instance_id.to_string(),
        })?;
        row.try_into()
    }
}

#[derive(sqlx::FromRow)]
struct InstanceRow {
    instance_id: String,
    workflow_id: String,
    workflow_version: i64,
    status: String,
    state_json: String,
    valid_from: String,
    valid_to: Option<String>,
    system_from: String,
    system_to: Option<String>,
    created_by_agent: Option<String>,
    created_at: String,
}

impl TryFrom<InstanceRow> for OpsWorkflowInstance {
    type Error = DurabilityError;

    fn try_from(r: InstanceRow) -> Result<Self> {
        let status = OpsWorkflowInstanceStatus::parse(&r.status).ok_or_else(|| {
            DurabilityError::NotFound {
                entity: "ops_workflow_instance_status",
                id: format!("{}={}", r.instance_id, r.status),
            }
        })?;

        Ok(OpsWorkflowInstance {
            instance_id: r.instance_id,
            workflow_id: r.workflow_id,
            workflow_version: r.workflow_version,
            status,
            state: serde_json::from_str::<WorkflowInstanceState>(&r.state_json)?,
            valid_from: parse_ts(&r.valid_from)?,
            valid_to: r.valid_to.as_deref().map(parse_ts).transpose()?,
            system_from: parse_ts(&r.system_from)?,
            system_to: r.system_to.as_deref().map(parse_ts).transpose()?,
            created_at: parse_ts(&r.created_at)?,
            created_by_agent: r.created_by_agent,
        })
    }
}

fn parse_ts(s: &str) -> Result<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(s)
        .map(|d| d.with_timezone(&Utc))
        .map_err(|_| DurabilityError::NotFound {
            entity: "timestamp",
            id: s.to_string(),
        })
}
