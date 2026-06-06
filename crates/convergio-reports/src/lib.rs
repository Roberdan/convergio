//! # convergio-reports
//!
//! Report engine: persists `ReportTemplate` definitions and renders reports as:
//!
//! - HTML (templated)
//! - PDF (lopdf)
//! - DOCX
//!
//! Every rendered report appends a provenance section containing:
//!
//! - A QR code (compact payload)
//! - A canonical JSON manifest embedded in the document

#![forbid(unsafe_code)]
#![deny(missing_docs)]

pub mod builtins;
pub mod error;
pub mod migrate;
pub mod render;
pub mod store;
pub mod types;

pub use builtins::{report_template_object_type, REPORT_TEMPLATE_OBJECT_TYPE_ID};
pub use error::{ReportError, Result};
pub use migrate::init;
pub use render::{render_report, RenderedReport};
pub use store::ReportTemplateStore;
pub use types::{
    NewReportTemplate, ProvenanceInput, RenderFormat, RenderReportRequest, ReportManifest,
    ReportTemplate,
};
