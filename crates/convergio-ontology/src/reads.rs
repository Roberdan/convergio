//! Read-side helpers for [`crate::store::Store`]. Split out of
//! `store.rs` to keep both files under the 300-line cap.

use sqlx::Row;

use crate::error::Result;
use crate::model::{LinkTypeRecord, ObjectTypeRecord, OwnerKind, PropertyTypeRecord};
use crate::semantic::parse_ts;
use crate::store::Store;

impl Store {
    /// Fetch one object-type revision.
    pub async fn get_object(
        &self,
        name: &str,
        schema_version: i64,
    ) -> Result<Option<ObjectTypeRecord>> {
        let row = sqlx::query("SELECT name, schema_version, content_hash, breaking, title, description, body_json, created_at, audit_seq FROM ontology_object_types WHERE name = ? AND schema_version = ?")
            .bind(name)
            .bind(schema_version)
            .fetch_optional(self.pool.inner())
            .await?;
        row.map(|r| {
            Ok(ObjectTypeRecord {
                name: r.try_get::<String, _>("name")?,
                schema_version: r.try_get::<i64, _>("schema_version")?,
                breaking: r.try_get::<i64, _>("breaking")? != 0,
                title: r.try_get::<String, _>("title")?,
                description: r.try_get::<String, _>("description")?,
                body: serde_json::from_str(&r.try_get::<String, _>("body_json")?)?,
                content_hash: r.try_get::<String, _>("content_hash")?,
                created_at: parse_ts(&r.try_get::<String, _>("created_at")?),
                audit_seq: r.try_get::<Option<i64>, _>("audit_seq")?,
            })
        })
        .transpose()
    }

    /// Fetch one link-type revision.
    pub async fn get_link(
        &self,
        name: &str,
        schema_version: i64,
    ) -> Result<Option<LinkTypeRecord>> {
        let row = sqlx::query("SELECT name, schema_version, content_hash, breaking, title, description, from_object, to_object, body_json, created_at, audit_seq FROM ontology_link_types WHERE name = ? AND schema_version = ?")
            .bind(name)
            .bind(schema_version)
            .fetch_optional(self.pool.inner())
            .await?;
        row.map(|r| {
            Ok(LinkTypeRecord {
                name: r.try_get::<String, _>("name")?,
                schema_version: r.try_get::<i64, _>("schema_version")?,
                breaking: r.try_get::<i64, _>("breaking")? != 0,
                title: r.try_get::<String, _>("title")?,
                description: r.try_get::<String, _>("description")?,
                from_object: r.try_get::<String, _>("from_object")?,
                to_object: r.try_get::<String, _>("to_object")?,
                body: serde_json::from_str(&r.try_get::<String, _>("body_json")?)?,
                content_hash: r.try_get::<String, _>("content_hash")?,
                created_at: parse_ts(&r.try_get::<String, _>("created_at")?),
                audit_seq: r.try_get::<Option<i64>, _>("audit_seq")?,
            })
        })
        .transpose()
    }

    /// Fetch one property-type revision.
    pub async fn get_property(
        &self,
        name: &str,
        schema_version: i64,
    ) -> Result<Option<PropertyTypeRecord>> {
        let row = sqlx::query("SELECT name, schema_version, content_hash, breaking, title, description, owner_kind, owner_name, datatype, required, body_json, created_at, audit_seq FROM ontology_property_types WHERE name = ? AND schema_version = ?")
            .bind(name)
            .bind(schema_version)
            .fetch_optional(self.pool.inner())
            .await?;
        row.map(|r| {
            let owner = r.try_get::<String, _>("owner_kind")?;
            Ok(PropertyTypeRecord {
                name: r.try_get::<String, _>("name")?,
                schema_version: r.try_get::<i64, _>("schema_version")?,
                breaking: r.try_get::<i64, _>("breaking")? != 0,
                title: r.try_get::<String, _>("title")?,
                description: r.try_get::<String, _>("description")?,
                owner_kind: OwnerKind::from_db_str(&owner).unwrap_or(OwnerKind::Object),
                owner_name: r.try_get::<String, _>("owner_name")?,
                datatype: r.try_get::<String, _>("datatype")?,
                required: r.try_get::<i64, _>("required")? != 0,
                body: serde_json::from_str(&r.try_get::<String, _>("body_json")?)?,
                content_hash: r.try_get::<String, _>("content_hash")?,
                created_at: parse_ts(&r.try_get::<String, _>("created_at")?),
                audit_seq: r.try_get::<Option<i64>, _>("audit_seq")?,
            })
        })
        .transpose()
    }
}
