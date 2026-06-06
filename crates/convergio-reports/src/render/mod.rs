//! Report rendering implementation.

mod docx;
mod html;
mod pdf;
mod qr;
mod util;

use crate::error::{ReportError, Result};
use crate::types::{RenderFormat, RenderReportRequest, ReportManifest, ReportTemplate};
use base64::Engine as _;
use chrono::Utc;
use convergio_ontology::Store as OntologyStore;

use self::docx::render_docx;
use self::html::append_provenance_html;
use self::pdf::render_pdf_lopdf;
use self::qr::qr_png;
use self::util::{render_jinja, sha256_hex, validate_params};

async fn latest_object_version(ontology: &OntologyStore, name: &str) -> Result<i64> {
    ontology
        .list_objects()
        .await
        .map_err(|e| ReportError::InvalidInput(format!("params ObjectType lookup failed: {e}")))?
        .into_iter()
        .find(|r| r.name == name)
        .map(|r| r.schema_version)
        .ok_or_else(|| ReportError::NotFound(name.to_string()))
}

/// Rendered output bytes plus its manifest.
#[derive(Debug, Clone)]
pub struct RenderedReport {
    /// Output MIME type.
    pub mime_type: &'static str,
    /// Rendered bytes.
    pub bytes: Vec<u8>,
    /// Embedded manifest.
    pub manifest: ReportManifest,
}

/// Render a report from a stored template.
///
/// This function:
/// 1) Validates `params` against the referenced ontology `ObjectType` JSON schema.
/// 2) Renders the requested output format.
/// 3) Appends provenance QR + JSON manifest to the output.
pub async fn render_report(
    templates: &crate::store::ReportTemplateStore,
    ontology: &OntologyStore,
    req: &RenderReportRequest,
) -> Result<RenderedReport> {
    let template: ReportTemplate = templates.get(&req.template_id).await?;

    let version = latest_object_version(ontology, &template.params_object_type_id).await?;
    let obj_type = ontology
        .get_object(&template.params_object_type_id, version)
        .await
        .map_err(|e| ReportError::InvalidInput(format!("params ObjectType lookup failed: {e}")))?
        .ok_or_else(|| ReportError::NotFound(template.params_object_type_id.clone()))?;
    validate_params(&obj_type.body, &req.params)?;

    let params_bytes = serde_json::to_vec(&req.params)
        .map_err(|e| ReportError::InvalidInput(format!("params not serializable: {e}")))?;
    let params_sha256 = sha256_hex(&params_bytes);

    let report_id = uuid::Uuid::new_v4().to_string();
    let rendered_at = Utc::now().to_rfc3339();

    let manifest = ReportManifest {
        schema_version: "1".into(),
        report_id: report_id.clone(),
        template_id: template.id.clone(),
        rendered_at: rendered_at.clone(),
        provenance: req.provenance.clone(),
        params_object_type_id: template.params_object_type_id.clone(),
        params_sha256,
    };

    let manifest_json = serde_json::to_string_pretty(&manifest)
        .map_err(|e| ReportError::InvalidInput(format!("manifest not serializable: {e}")))?;
    let manifest_sha256 = sha256_hex(manifest_json.as_bytes());

    let qr_payload = serde_json::json!({
        "schema": "cvg_report_qr_v1",
        "report_id": report_id,
        "template_id": template.id,
        "rendered_at": rendered_at,
        "manifest_sha256": manifest_sha256.clone(),
    })
    .to_string();

    let qr_png_bytes = qr_png(&qr_payload)?;

    let (mime_type, bytes) = match req.format {
        RenderFormat::Html => {
            let body = template
                .template_html
                .ok_or_else(|| ReportError::InvalidInput("template_html missing".into()))?;
            let rendered = render_jinja(&body, &req.params)?;
            let qr_data_uri = format!(
                "data:image/png;base64,{}",
                base64::engine::general_purpose::STANDARD.encode(&qr_png_bytes)
            );
            let manifest_b64 =
                base64::engine::general_purpose::STANDARD.encode(manifest_json.as_bytes());
            (
                "text/html; charset=utf-8",
                append_provenance_html(
                    &rendered,
                    &qr_data_uri,
                    &manifest_json,
                    &manifest_b64,
                    &manifest_sha256,
                )
                .into_bytes(),
            )
        }
        RenderFormat::Pdf => {
            let body = template
                .template_typst
                .ok_or_else(|| ReportError::InvalidInput("template_typst missing".into()))?;
            let rendered = render_jinja(&body, &req.params)?;
            (
                "application/pdf",
                render_pdf_lopdf(&rendered, &qr_png_bytes, &manifest_json)?,
            )
        }
        RenderFormat::Docx => {
            let body = template
                .template_docx
                .ok_or_else(|| ReportError::InvalidInput("template_docx missing".into()))?;
            let rendered = render_jinja(&body, &req.params)?;
            (
                "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
                render_docx(&rendered, &qr_png_bytes, &manifest_json)?,
            )
        }
    };

    Ok(RenderedReport {
        mime_type,
        bytes,
        manifest,
    })
}
