-- convergio-reports: report templates
-- Migration range 501-599 reserved by ADR-0003.

CREATE TABLE IF NOT EXISTS report_templates (
  id                   TEXT PRIMARY KEY,
  title                TEXT NOT NULL,
  description          TEXT NOT NULL,
  template_html        TEXT,
  template_typst       TEXT,
  template_docx        TEXT,
  params_object_type_id TEXT NOT NULL,
  created_at           TEXT NOT NULL,
  updated_at           TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_report_templates_params_object_type_id
  ON report_templates(params_object_type_id);
