//! Helpers that produce the canonical "semantic body" objects used
//! for hashing in [`crate::store`]. Split out of `store.rs` to keep
//! both files under the 300-line cap.

use chrono::{DateTime, Utc};
use convergio_db::Pool;
use serde_json::{json, Value};
use sqlx::Row;

use crate::error::{Error, Result};
use crate::model::{OwnerKind, TypeKind};

pub(crate) fn parse_ts(s: &str) -> DateTime<Utc> {
    DateTime::parse_from_rfc3339(s)
        .map(|d| d.with_timezone(&Utc))
        .unwrap_or_else(|_| Utc::now())
}

pub(crate) async fn check_existing(
    pool: &Pool,
    table: &str,
    kind: TypeKind,
    name: &str,
    schema_version: i64,
    hash: &str,
) -> Result<()> {
    let sql = format!("SELECT content_hash FROM {table} WHERE name = ? AND schema_version = ?");
    let row = sqlx::query(&sql)
        .bind(name)
        .bind(schema_version)
        .fetch_optional(pool.inner())
        .await?;
    if let Some(r) = row {
        let existing: String = r.try_get("content_hash")?;
        if existing != hash {
            return Err(Error::VersionConflict {
                kind: kind.as_static_str(),
                name: name.to_string(),
                version: schema_version,
            });
        }
    }
    Ok(())
}

pub(crate) fn object_semantic(
    name: &str,
    schema_version: i64,
    breaking: bool,
    title: &str,
    description: &str,
    body: &Value,
) -> Value {
    json!({
        "kind": "object",
        "name": name,
        "schema_version": schema_version,
        "breaking": breaking,
        "title": title,
        "description": description,
        "body": body,
    })
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn link_semantic(
    name: &str,
    schema_version: i64,
    breaking: bool,
    title: &str,
    description: &str,
    from_object: &str,
    to_object: &str,
    body: &Value,
) -> Value {
    json!({
        "kind": "link",
        "name": name,
        "schema_version": schema_version,
        "breaking": breaking,
        "title": title,
        "description": description,
        "from": from_object,
        "to": to_object,
        "body": body,
    })
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn property_semantic(
    name: &str,
    schema_version: i64,
    breaking: bool,
    title: &str,
    description: &str,
    owner_kind: OwnerKind,
    owner_name: &str,
    datatype: &str,
    required: bool,
    body: &Value,
) -> Value {
    json!({
        "kind": "property",
        "name": name,
        "schema_version": schema_version,
        "breaking": breaking,
        "title": title,
        "description": description,
        "owner_kind": owner_kind.as_db_str(),
        "owner_name": owner_name,
        "datatype": datatype,
        "required": required,
        "body": body,
    })
}
