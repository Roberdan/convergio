//! Public API types for templates and rendering.

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Persisted report template.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReportTemplate {
    /// Stable template identifier (PRIMARY KEY).
    pub id: String,
    /// Human title.
    pub title: String,
    /// Human description.
    pub description: String,
    /// HTML body template (minijinja).
    pub template_html: Option<String>,
    /// PDF body template (rendered as plain text into the PDF output).
    ///
    /// Field name is kept as `template_typst` for backward compatibility.
    pub template_typst: Option<String>,
    /// DOCX plain-text template.
    pub template_docx: Option<String>,
    /// Ontology `ObjectType` id used to validate render parameters.
    pub params_object_type_id: String,
    /// ISO-8601 creation timestamp.
    pub created_at: String,
    /// ISO-8601 last update timestamp.
    pub updated_at: String,
}

/// New template registration payload.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewReportTemplate {
    /// Stable id.
    pub id: String,
    /// Title.
    pub title: String,
    /// Description.
    pub description: String,
    /// HTML template body.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub template_html: Option<String>,
    /// PDF template body (rendered as plain text into the PDF output).
    ///
    /// Field name is kept as `template_typst` for backward compatibility.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub template_typst: Option<String>,
    /// DOCX template body (plain text).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub template_docx: Option<String>,
    /// Ontology object type id for parameter validation.
    pub params_object_type_id: String,
}

/// Supported output formats.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RenderFormat {
    /// HTML output.
    Html,
    /// PDF output.
    Pdf,
    /// DOCX output.
    Docx,
}

/// Provenance inputs that bind the rendered report to Convergio runtime state.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProvenanceInput {
    /// Plan id.
    pub plan_id: String,
    /// Task id.
    pub task_id: String,
    /// Agent id.
    pub agent_id: String,
}

/// Render request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RenderReportRequest {
    /// Report template id.
    pub template_id: String,
    /// Desired output format.
    pub format: RenderFormat,
    /// Parameters passed into the template.
    #[serde(default)]
    pub params: Value,
    /// Provenance binding (plan/task/agent).
    pub provenance: ProvenanceInput,
}

/// Canonical report manifest embedded in every output.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReportManifest {
    /// Manifest schema version.
    pub schema_version: String,
    /// Unique report render id.
    pub report_id: String,
    /// Template id.
    pub template_id: String,
    /// Render timestamp (RFC3339).
    pub rendered_at: String,
    /// Provenance binding.
    pub provenance: ProvenanceInput,
    /// ObjectType used for params validation.
    pub params_object_type_id: String,
    /// SHA-256 of the request params JSON.
    pub params_sha256: String,
}
