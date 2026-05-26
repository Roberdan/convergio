use crate::model::OpsWorkflow;
use crate::spec::WorkflowSpec;
use chrono::{DateTime, Utc};
use convergio_db::Pool;
use convergio_durability::error::{DurabilityError, Result};

/// Read access to `ops_workflows`.
#[derive(Clone)]
pub struct OpsWorkflowStore {
    pool: Pool,
}

impl OpsWorkflowStore {
    /// Wrap a pool.
    pub fn new(pool: Pool) -> Self {
        Self { pool }
    }

    /// Get the current system-time row for the given workflow id.
    pub async fn get_current(&self, workflow_id: &str) -> Result<OpsWorkflow> {
        let row = sqlx::query_as::<_, WorkflowRow>(
            "SELECT workflow_id, workflow_key, version, spec_json, valid_from, valid_to, \
                    system_from, system_to, created_by_agent, created_at \
             FROM ops_workflows \
             WHERE workflow_id = ? AND system_to IS NULL \
             ORDER BY system_from DESC \
             LIMIT 1",
        )
        .bind(workflow_id)
        .fetch_optional(self.pool.inner())
        .await?;

        let row = row.ok_or_else(|| DurabilityError::NotFound {
            entity: "ops_workflow",
            id: workflow_id.to_string(),
        })?;
        row.try_into()
    }

    /// Get the current system-time row for the given workflow key.
    pub async fn get_current_by_key(&self, workflow_key: &str) -> Result<OpsWorkflow> {
        let row = sqlx::query_as::<_, WorkflowRow>(
            "SELECT workflow_id, workflow_key, version, spec_json, valid_from, valid_to, \
                    system_from, system_to, created_by_agent, created_at \
             FROM ops_workflows \
             WHERE workflow_key = ? AND system_to IS NULL \
             ORDER BY system_from DESC \
             LIMIT 1",
        )
        .bind(workflow_key)
        .fetch_optional(self.pool.inner())
        .await?;

        let row = row.ok_or_else(|| DurabilityError::NotFound {
            entity: "ops_workflow_key",
            id: workflow_key.to_string(),
        })?;
        row.try_into()
    }

    /// Fetch a workflow spec by version, regardless of system-time closure.
    pub async fn get_version(&self, workflow_id: &str, version: i64) -> Result<OpsWorkflow> {
        let row = sqlx::query_as::<_, WorkflowRow>(
            "SELECT workflow_id, workflow_key, version, spec_json, valid_from, valid_to, \
                    system_from, system_to, created_by_agent, created_at \
             FROM ops_workflows \
             WHERE workflow_id = ? AND version = ? \
             ORDER BY system_from DESC \
             LIMIT 1",
        )
        .bind(workflow_id)
        .bind(version)
        .fetch_optional(self.pool.inner())
        .await?;

        let row = row.ok_or_else(|| DurabilityError::NotFound {
            entity: "ops_workflow_version",
            id: format!("{}:{}", workflow_id, version),
        })?;
        row.try_into()
    }

    /// Get bitemporal snapshot.
    pub async fn get_snapshot(
        &self,
        workflow_id: &str,
        as_of: DateTime<Utc>,
        valid_at: DateTime<Utc>,
    ) -> Result<OpsWorkflow> {
        let row = sqlx::query_as::<_, WorkflowRow>(
            "SELECT workflow_id, workflow_key, version, spec_json, valid_from, valid_to, \
                    system_from, system_to, created_by_agent, created_at \
             FROM ops_workflows \
             WHERE workflow_id = ? \
               AND system_from <= ? \
               AND (system_to IS NULL OR system_to > ?) \
               AND valid_from <= ? \
               AND (valid_to IS NULL OR valid_to > ?) \
             ORDER BY system_from DESC \
             LIMIT 1",
        )
        .bind(workflow_id)
        .bind(as_of.to_rfc3339())
        .bind(as_of.to_rfc3339())
        .bind(valid_at.to_rfc3339())
        .bind(valid_at.to_rfc3339())
        .fetch_optional(self.pool.inner())
        .await?;

        let row = row.ok_or_else(|| DurabilityError::NotFound {
            entity: "ops_workflow",
            id: workflow_id.to_string(),
        })?;
        row.try_into()
    }
}

#[derive(sqlx::FromRow)]
struct WorkflowRow {
    workflow_id: String,
    workflow_key: String,
    version: i64,
    spec_json: String,
    valid_from: String,
    valid_to: Option<String>,
    system_from: String,
    system_to: Option<String>,
    created_by_agent: Option<String>,
    created_at: String,
}

impl TryFrom<WorkflowRow> for OpsWorkflow {
    type Error = DurabilityError;

    fn try_from(r: WorkflowRow) -> Result<Self> {
        Ok(OpsWorkflow {
            workflow_id: r.workflow_id,
            workflow_key: r.workflow_key,
            version: r.version,
            spec: serde_json::from_str::<WorkflowSpec>(&r.spec_json)?,
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
