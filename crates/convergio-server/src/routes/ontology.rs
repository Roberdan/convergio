//! `/v1/ontology/*` — Ontology Runtime Core HTTP surface (ADR-0053).
//!
//! Endpoints:
//!
//! - `GET  /v1/ontology/types` — list latest revision of every
//!   registered `ObjectType` / `LinkType`.
//! - `GET  /v1/ontology/types/object/:name` — describe one object
//!   (newest version, properties inlined).
//! - `GET  /v1/ontology/types/link/:name` — describe one link.
//! - `GET  /v1/ontology/export/:format/object/:name?version=N` —
//!   deterministic export. `:format` is `jsonschema` or `shacl`.
//!
//! Bytes returned by the export endpoint are byte-identical to the
//! crate-level exporter (golden-tested in `convergio-ontology`).

use crate::AppState;
use axum::extract::{Path, Query, State};
use axum::http::header;
use axum::response::IntoResponse;
use axum::routing::get;
use axum::{Json, Router};
use convergio_ontology::{
    export_object_schema, export_object_shacl, Error as OntologyError, LinkTypeRecord,
    ObjectTypeRecord, PropertyTypeRecord,
};
use convergio_server_core::ApiError;
use serde::{Deserialize, Serialize};

/// Mount the ontology routes.
pub fn router() -> Router<AppState> {
    Router::new()
        .route("/v1/ontology/types", get(list_types))
        .route("/v1/ontology/types/object/:name", get(describe_object))
        .route("/v1/ontology/types/link/:name", get(describe_link))
        .route("/v1/ontology/export/:format/object/:name", get(export_object))
}

#[derive(Serialize)]
struct TypeRow {
    kind: &'static str,
    name: String,
    schema_version: i64,
    title: String,
    description: String,
    content_hash: String,
}

#[derive(Serialize)]
struct ListResponse {
    objects: Vec<TypeRow>,
    links: Vec<TypeRow>,
}

fn obj_row(r: &ObjectTypeRecord) -> TypeRow {
    TypeRow {
        kind: "object",
        name: r.name.clone(),
        schema_version: r.schema_version,
        title: r.title.clone(),
        description: r.description.clone(),
        content_hash: r.content_hash.clone(),
    }
}

fn link_row(r: &LinkTypeRecord) -> TypeRow {
    TypeRow {
        kind: "link",
        name: r.name.clone(),
        schema_version: r.schema_version,
        title: r.title.clone(),
        description: r.description.clone(),
        content_hash: r.content_hash.clone(),
    }
}

async fn list_types(State(state): State<AppState>) -> Result<Json<ListResponse>, ApiError> {
    let objects = state.ontology.list_objects().await?;
    let links = state.ontology.list_links().await?;
    Ok(Json(ListResponse {
        objects: objects.iter().map(obj_row).collect(),
        links: links.iter().map(link_row).collect(),
    }))
}

#[derive(Serialize)]
struct PropertyRow {
    name: String,
    schema_version: i64,
    datatype: String,
    required: bool,
    title: String,
    description: String,
    content_hash: String,
}

fn property_row(r: &PropertyTypeRecord) -> PropertyRow {
    PropertyRow {
        name: r.name.clone(),
        schema_version: r.schema_version,
        datatype: r.datatype.clone(),
        required: r.required,
        title: r.title.clone(),
        description: r.description.clone(),
        content_hash: r.content_hash.clone(),
    }
}

#[derive(Serialize)]
struct DescribeObject {
    name: String,
    schema_version: i64,
    title: String,
    description: String,
    breaking: bool,
    content_hash: String,
    properties: Vec<PropertyRow>,
}

#[derive(Serialize)]
struct DescribeLink {
    name: String,
    schema_version: i64,
    title: String,
    description: String,
    from_object: String,
    to_object: String,
    breaking: bool,
    content_hash: String,
}

#[derive(Deserialize)]
struct VersionQuery {
    version: Option<i64>,
}

async fn latest_object_version(
    state: &AppState,
    name: &str,
) -> Result<i64, ApiError> {
    let v = state
        .ontology
        .list_objects()
        .await?
        .into_iter()
        .find(|r| r.name == name)
        .map(|r| r.schema_version)
        .ok_or_else(|| OntologyError::NotFound {
            kind: "object",
            name: name.to_string(),
        })?;
    Ok(v)
}

async fn describe_object(
    State(state): State<AppState>,
    Path(name): Path<String>,
    Query(q): Query<VersionQuery>,
) -> Result<Json<DescribeObject>, ApiError> {
    let version = match q.version {
        Some(v) => v,
        None => latest_object_version(&state, &name).await?,
    };
    let object = state
        .ontology
        .get_object(&name, version)
        .await?
        .ok_or_else(|| OntologyError::NotFound {
            kind: "object",
            name: name.clone(),
        })?;
    let props = state.ontology.list_object_properties(&name, version).await?;
    Ok(Json(DescribeObject {
        name: object.name,
        schema_version: object.schema_version,
        title: object.title,
        description: object.description,
        breaking: object.breaking,
        content_hash: object.content_hash,
        properties: props.iter().map(property_row).collect(),
    }))
}

async fn describe_link(
    State(state): State<AppState>,
    Path(name): Path<String>,
    Query(q): Query<VersionQuery>,
) -> Result<Json<DescribeLink>, ApiError> {
    let version = match q.version {
        Some(v) => v,
        None => state
            .ontology
            .list_links()
            .await?
            .into_iter()
            .find(|r| r.name == name)
            .map(|r| r.schema_version)
            .ok_or_else(|| OntologyError::NotFound {
                kind: "link",
                name: name.clone(),
            })?,
    };
    let link = state
        .ontology
        .get_link(&name, version)
        .await?
        .ok_or_else(|| OntologyError::NotFound {
            kind: "link",
            name: name.clone(),
        })?;
    Ok(Json(DescribeLink {
        name: link.name,
        schema_version: link.schema_version,
        title: link.title,
        description: link.description,
        from_object: link.from_object,
        to_object: link.to_object,
        breaking: link.breaking,
        content_hash: link.content_hash,
    }))
}

async fn export_object(
    State(state): State<AppState>,
    Path((format, name)): Path<(String, String)>,
    Query(q): Query<VersionQuery>,
) -> Result<axum::response::Response, ApiError> {
    let version = match q.version {
        Some(v) => v,
        None => latest_object_version(&state, &name).await?,
    };
    let bytes = match format.as_str() {
        "jsonschema" => export_object_schema(&state.ontology, &name, version).await?,
        "shacl" => export_object_shacl(&state.ontology, &name, version).await?,
        other => {
            return Err(ApiError::BadRequest {
                code: "ontology_unknown_format",
                message: format!(
                    "unknown export format '{other}', expected one of: jsonschema, shacl"
                ),
            });
        }
    };
    Ok((
        [(header::CONTENT_TYPE, "application/json")],
        bytes,
    )
        .into_response())
}
