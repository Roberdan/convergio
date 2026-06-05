//! `/v1/reports/*` — report templates and rendering.

use crate::app::AppState;
use crate::error::ApiError;
use axum::extract::{Path, State};
use axum::routing::{get, post};
use axum::{Json, Router};
use base64::Engine as _;
use convergio_reports::{
    render_report, NewReportTemplate, RenderReportRequest, RenderedReport, ReportError,
    ReportTemplate,
};
use serde::Serialize;

/// Mount report routes onto the daemon router.
pub fn router() -> Router<AppState> {
    Router::new()
        .route(
            "/v1/reports/templates",
            post(create_template).get(list_templates),
        )
        .route("/v1/reports/templates/:id", get(get_template))
        .route("/v1/reports/render", post(render))
}

async fn create_template(
    State(state): State<AppState>,
    Json(body): Json<NewReportTemplate>,
) -> Result<Json<ReportTemplate>, ApiError> {
    // Enforce ontology typing: referenced ObjectType must exist.
    let has_type = state
        .ontology
        .list_objects()
        .await?
        .into_iter()
        .any(|r| r.name == body.params_object_type_id);
    if !has_type {
        return Err(convergio_ontology::Error::NotFound {
            kind: "object",
            name: body.params_object_type_id.clone(),
        }
        .into());
    }
    Ok(Json(
        state.reports.create(&body).await.map_err(report_error)?,
    ))
}

async fn list_templates(
    State(state): State<AppState>,
) -> Result<Json<Vec<ReportTemplate>>, ApiError> {
    Ok(Json(state.reports.list().await.map_err(report_error)?))
}

async fn get_template(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<ReportTemplate>, ApiError> {
    Ok(Json(state.reports.get(&id).await.map_err(report_error)?))
}

#[derive(Debug, Serialize)]
struct RenderResponse {
    ok: bool,
    mime_type: String,
    bytes_base64: String,
    manifest: convergio_reports::ReportManifest,
}

async fn render(
    State(state): State<AppState>,
    Json(body): Json<RenderReportRequest>,
) -> Result<Json<RenderResponse>, ApiError> {
    let rendered: RenderedReport = render_report(&state.reports, &state.ontology, &body)
        .await
        .map_err(report_error)?;

    Ok(Json(RenderResponse {
        ok: true,
        mime_type: rendered.mime_type.to_string(),
        bytes_base64: base64::engine::general_purpose::STANDARD.encode(&rendered.bytes),
        manifest: rendered.manifest,
    }))
}

fn report_error(e: ReportError) -> ApiError {
    match e {
        ReportError::NotFound(_) => ApiError::BadRequest {
            code: "report_not_found",
            message: e.to_string(),
        },
        ReportError::InvalidInput(_) | ReportError::ParamValidation(_) => ApiError::Validation {
            code: "report_invalid",
            message: e.to_string(),
        },
        ReportError::Template(_)
        | ReportError::Pdf(_)
        | ReportError::Docx(_)
        | ReportError::Qr(_) => ApiError::Validation {
            code: "report_render_failed",
            message: e.to_string(),
        },
        ReportError::Db(_) | ReportError::Migrate(_) => ApiError::Internal(e.to_string()),
    }
}
