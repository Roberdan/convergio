//! SQLite persistence for the ontology registry.
//!
//! Append-on-write: registering a new revision inserts a new row at
//! `(name, schema_version)`. Attempting to re-register the same
//! `(name, schema_version)` with a different `content_hash` is a
//! [`Error::VersionConflict`] — registry history is immutable per
//! ADR-0053.

use crate::error::{Error, Result};
use crate::hash::{canonical_bytes, content_hash};
use crate::model::{LinkTypeRecord, ObjectTypeRecord, OwnerKind, PropertyTypeRecord, TypeKind};
use crate::semantic::{check_existing, link_semantic, object_semantic, property_semantic};
use chrono::Utc;
use convergio_db::Pool;
use serde_json::Value;

/// Storage handle backed by the shared SQLite pool from
/// [`convergio_db`].
#[derive(Clone)]
pub struct Store {
    pub(crate) pool: Pool,
}

impl Store {
    /// Bind to the existing SQLite pool.
    pub fn new(pool: Pool) -> Self {
        Self { pool }
    }

    /// Access the immutable purpose registry (ADR-0054 §B), backed by the
    /// same SQLite pool. Cheap to call — `Pool` clones share the handle.
    pub fn purposes(&self) -> crate::purposes::PurposeStore {
        crate::purposes::PurposeStore::new(self.pool.clone())
    }

    /// Run pending migrations (range 1000-1099). Idempotent — safe
    /// to call on every daemon start. Coexists with sibling crates'
    /// migrators thanks to `set_ignore_missing(true)`, exactly the
    /// same pattern as [`convergio_graph::Store::migrate`].
    pub async fn migrate(&self) -> Result<()> {
        let mut migrator = sqlx::migrate!("./migrations");
        migrator.set_ignore_missing(true);
        migrator.run(self.pool.inner()).await?;
        Ok(())
    }

    /// Register (or re-confirm) an object type at the given version.
    /// Returns the canonical, hashed row as actually stored.
    #[allow(clippy::too_many_arguments)]
    pub async fn upsert_object(
        &self,
        name: &str,
        schema_version: i64,
        breaking: bool,
        title: &str,
        description: &str,
        body: Value,
        audit_seq: Option<i64>,
    ) -> Result<ObjectTypeRecord> {
        let semantic = object_semantic(name, schema_version, breaking, title, description, &body);
        let hash = content_hash(&semantic)?;
        let now = Utc::now();
        let body_json =
            String::from_utf8(canonical_bytes(&body)?).expect("canonical bytes are utf-8");
        check_existing(
            &self.pool,
            "ontology_object_types",
            TypeKind::Object,
            name,
            schema_version,
            &hash,
        )
        .await?;
        sqlx::query(
            "INSERT OR IGNORE INTO ontology_object_types (name, schema_version, content_hash, breaking, title, description, body_json, created_at, audit_seq) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(name)
        .bind(schema_version)
        .bind(&hash)
        .bind(breaking as i64)
        .bind(title)
        .bind(description)
        .bind(&body_json)
        .bind(now.to_rfc3339())
        .bind(audit_seq)
        .execute(self.pool.inner())
        .await?;
        self.get_object(name, schema_version)
            .await?
            .ok_or(Error::NotFound {
                kind: "object_type",
                name: name.to_string(),
            })
    }

    /// Register (or re-confirm) a link type at the given version.
    #[allow(clippy::too_many_arguments)]
    pub async fn upsert_link(
        &self,
        name: &str,
        schema_version: i64,
        breaking: bool,
        title: &str,
        description: &str,
        from_object: &str,
        to_object: &str,
        body: Value,
        audit_seq: Option<i64>,
    ) -> Result<LinkTypeRecord> {
        let semantic = link_semantic(
            name,
            schema_version,
            breaking,
            title,
            description,
            from_object,
            to_object,
            &body,
        );
        let hash = content_hash(&semantic)?;
        let now = Utc::now();
        let body_json =
            String::from_utf8(canonical_bytes(&body)?).expect("canonical bytes are utf-8");
        check_existing(
            &self.pool,
            "ontology_link_types",
            TypeKind::Link,
            name,
            schema_version,
            &hash,
        )
        .await?;
        sqlx::query(
            "INSERT OR IGNORE INTO ontology_link_types (name, schema_version, content_hash, breaking, title, description, from_object, to_object, body_json, created_at, audit_seq) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(name)
        .bind(schema_version)
        .bind(&hash)
        .bind(breaking as i64)
        .bind(title)
        .bind(description)
        .bind(from_object)
        .bind(to_object)
        .bind(&body_json)
        .bind(now.to_rfc3339())
        .bind(audit_seq)
        .execute(self.pool.inner())
        .await?;
        self.get_link(name, schema_version)
            .await?
            .ok_or(Error::NotFound {
                kind: "link_type",
                name: name.to_string(),
            })
    }

    /// Register (or re-confirm) a property type at the given version.
    #[allow(clippy::too_many_arguments)]
    pub async fn upsert_property(
        &self,
        name: &str,
        schema_version: i64,
        breaking: bool,
        title: &str,
        description: &str,
        owner_kind: OwnerKind,
        owner_name: &str,
        datatype: &str,
        required: bool,
        body: Value,
        audit_seq: Option<i64>,
    ) -> Result<PropertyTypeRecord> {
        let semantic = property_semantic(
            name,
            schema_version,
            breaking,
            title,
            description,
            owner_kind,
            owner_name,
            datatype,
            required,
            &body,
        );
        let hash = content_hash(&semantic)?;
        let now = Utc::now();
        let body_json =
            String::from_utf8(canonical_bytes(&body)?).expect("canonical bytes are utf-8");
        check_existing(
            &self.pool,
            "ontology_property_types",
            TypeKind::Property,
            name,
            schema_version,
            &hash,
        )
        .await?;
        sqlx::query(
            "INSERT OR IGNORE INTO ontology_property_types (name, schema_version, content_hash, breaking, title, description, owner_kind, owner_name, datatype, required, body_json, created_at, audit_seq) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(name)
        .bind(schema_version)
        .bind(&hash)
        .bind(breaking as i64)
        .bind(title)
        .bind(description)
        .bind(owner_kind.as_db_str())
        .bind(owner_name)
        .bind(datatype)
        .bind(required as i64)
        .bind(&body_json)
        .bind(now.to_rfc3339())
        .bind(audit_seq)
        .execute(self.pool.inner())
        .await?;
        self.get_property(name, schema_version)
            .await?
            .ok_or(Error::NotFound {
                kind: "property_type",
                name: name.to_string(),
            })
    }
}
