//! SQLite-backed store for ontology object instances, link events, and property events.

use crate::error::{Error, Result};
use chrono::Utc;
use convergio_db::Pool;
use sqlx::Row;
use uuid::Uuid;

/// Insert-only operation for `object_links`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LinkOp {
    /// Assert that a link exists.
    Add,
    /// Retract a previously asserted link.
    Remove,
}

impl LinkOp {
    fn as_str(self) -> &'static str {
        match self {
            Self::Add => "add",
            Self::Remove => "remove",
        }
    }
}

/// Insert-only operation for `object_properties`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PropertyOp {
    /// Set a property value.
    Set,
    /// Remove a property value.
    Unset,
}

impl PropertyOp {
    fn as_str(self) -> &'static str {
        match self {
            Self::Set => "set",
            Self::Unset => "unset",
        }
    }
}

/// Row from `object_instances`.
#[derive(Debug, Clone)]
pub struct ObjectInstance {
    /// UUID v4.
    pub id: String,
    /// Tenant scope (today: `plans.id`).
    pub tenant_id: String,
    /// Object type identifier.
    pub r#type: String,
    /// Creation timestamp (RFC3339).
    pub created_at: String,
}

/// Row from `object_links`.
#[derive(Debug, Clone)]
pub struct ObjectLinkEvent {
    /// UUID v4.
    pub id: String,
    /// Tenant scope.
    pub tenant_id: String,
    /// Source object id.
    pub from_id: String,
    /// Destination object id.
    pub to_id: String,
    /// Link type identifier.
    pub link_type: String,
    /// Operation.
    pub op: String,
    /// Creation timestamp (RFC3339).
    pub created_at: String,
}

/// Row from `object_properties`.
#[derive(Debug, Clone)]
pub struct ObjectPropertyEvent {
    /// UUID v4.
    pub id: String,
    /// Tenant scope.
    pub tenant_id: String,
    /// Owning object id.
    pub object_id: String,
    /// Property type identifier.
    pub property_type: String,
    /// JSON-encoded value payload.
    pub value_json: String,
    /// Operation.
    pub op: String,
    /// Creation timestamp (RFC3339).
    pub created_at: String,
}

/// Store facade over ontology object storage tables.
#[derive(Clone)]
pub struct OntologyStore {
    pool: Pool,
}

impl OntologyStore {
    /// Create a new store bound to the given pool.
    pub fn new(pool: Pool) -> Self {
        Self { pool }
    }

    fn now() -> String {
        Utc::now().to_rfc3339()
    }

    /// Create an object instance under `tenant_id`.
    ///
    /// If the target `ObjectType` is flagged `requires_purpose` (ADR-0082),
    /// `purpose` must be `Some(label)` referencing a **registered** purpose
    /// (`cvg purpose register`); otherwise the write is refused with
    /// [`Error::PurposeRequired`] / [`Error::PurposeMismatch`]. Types that
    /// are not flagged ignore `purpose` (opt-in, zero churn).
    pub async fn create_instance(
        &self,
        tenant_id: &str,
        object_type: &str,
        purpose: Option<&str>,
    ) -> Result<ObjectInstance> {
        self.admit_purpose(object_type, purpose).await?;
        let id = Uuid::new_v4().to_string();
        let created_at = Self::now();
        sqlx::query(
            "INSERT INTO object_instances (id, tenant_id, type, created_at) VALUES (?, ?, ?, ?)",
        )
        .bind(&id)
        .bind(tenant_id)
        .bind(object_type)
        .bind(&created_at)
        .execute(self.pool.inner())
        .await?;

        Ok(ObjectInstance {
            id,
            tenant_id: tenant_id.to_owned(),
            r#type: object_type.to_owned(),
            created_at,
        })
    }

    /// Purpose-limitation admission for a write to `object_type` (ADR-0082).
    /// No-op unless the type's latest schema sets `requires_purpose`.
    async fn admit_purpose(&self, object_type: &str, purpose: Option<&str>) -> Result<()> {
        if !self.type_requires_purpose(object_type).await? {
            return Ok(());
        }
        match purpose {
            None => Err(Error::PurposeRequired {
                object_type: object_type.to_owned(),
            }),
            Some(label) if self.purpose_is_registered(label).await? => Ok(()),
            Some(label) => Err(Error::PurposeMismatch {
                object_type: object_type.to_owned(),
                purpose: label.to_owned(),
            }),
        }
    }

    /// Read the `requires_purpose` flag from the type's latest schema body.
    /// Unknown types are treated as not requiring a purpose here (type
    /// existence is validated on the registration/import path).
    async fn type_requires_purpose(&self, object_type: &str) -> Result<bool> {
        let row: Option<(String,)> = sqlx::query_as(
            "SELECT body_json FROM ontology_object_types WHERE name = ? \
             ORDER BY schema_version DESC LIMIT 1",
        )
        .bind(object_type)
        .fetch_optional(self.pool.inner())
        .await?;
        match row {
            Some((body_json,)) => {
                let body: serde_json::Value = serde_json::from_str(&body_json)?;
                Ok(body
                    .get("requires_purpose")
                    .and_then(serde_json::Value::as_bool)
                    .unwrap_or(false))
            }
            None => Ok(false),
        }
    }

    /// Whether `label` exists in the purpose registry (ADR-0054 §B).
    async fn purpose_is_registered(&self, label: &str) -> Result<bool> {
        let row: Option<(i64,)> = sqlx::query_as("SELECT 1 FROM purposes WHERE label = ? LIMIT 1")
            .bind(label)
            .fetch_optional(self.pool.inner())
            .await?;
        Ok(row.is_some())
    }

    /// Append a link event to the `object_links` log.
    ///
    /// Refuses to link objects that are not both present under `tenant_id`.
    pub async fn append_link(
        &self,
        tenant_id: &str,
        from_id: &str,
        to_id: &str,
        link_type: &str,
        op: LinkOp,
    ) -> Result<ObjectLinkEvent> {
        self.ensure_instance_in_tenant(tenant_id, from_id).await?;
        self.ensure_instance_in_tenant(tenant_id, to_id).await?;

        let id = Uuid::new_v4().to_string();
        let created_at = Self::now();
        let op_s = op.as_str();
        sqlx::query(
            "INSERT INTO object_links (id, tenant_id, from_id, to_id, link_type, op, created_at)\
             VALUES (?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(&id)
        .bind(tenant_id)
        .bind(from_id)
        .bind(to_id)
        .bind(link_type)
        .bind(op_s)
        .bind(&created_at)
        .execute(self.pool.inner())
        .await?;

        Ok(ObjectLinkEvent {
            id,
            tenant_id: tenant_id.to_owned(),
            from_id: from_id.to_owned(),
            to_id: to_id.to_owned(),
            link_type: link_type.to_owned(),
            op: op_s.to_owned(),
            created_at,
        })
    }

    /// Append a property event to the `object_properties` log.
    pub async fn append_property(
        &self,
        tenant_id: &str,
        object_id: &str,
        property_type: &str,
        value_json: &str,
        op: PropertyOp,
    ) -> Result<ObjectPropertyEvent> {
        self.ensure_instance_in_tenant(tenant_id, object_id).await?;

        let id = Uuid::new_v4().to_string();
        let created_at = Self::now();
        let op_s = op.as_str();
        sqlx::query(
            "INSERT INTO object_properties (id, tenant_id, object_id, property_type, value_json, op, created_at)\
             VALUES (?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(&id)
        .bind(tenant_id)
        .bind(object_id)
        .bind(property_type)
        .bind(value_json)
        .bind(op_s)
        .bind(&created_at)
        .execute(self.pool.inner())
        .await?;

        Ok(ObjectPropertyEvent {
            id,
            tenant_id: tenant_id.to_owned(),
            object_id: object_id.to_owned(),
            property_type: property_type.to_owned(),
            value_json: value_json.to_owned(),
            op: op_s.to_owned(),
            created_at,
        })
    }

    async fn ensure_instance_in_tenant(&self, tenant_id: &str, id: &str) -> Result<()> {
        let exists: i64 =
            sqlx::query("SELECT COUNT(*) FROM object_instances WHERE tenant_id = ? AND id = ?")
                .bind(tenant_id)
                .bind(id)
                .fetch_one(self.pool.inner())
                .await?
                .get(0);
        if exists == 0 {
            return Err(Error::InstanceNotFound {
                tenant_id: tenant_id.to_owned(),
                id: id.to_owned(),
            });
        }
        Ok(())
    }
}
