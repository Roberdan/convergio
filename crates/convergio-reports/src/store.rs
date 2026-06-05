//! Persistent store for report templates.

use crate::error::{ReportError, Result};
use crate::types::{NewReportTemplate, ReportTemplate};
use chrono::Utc;
use convergio_db::Pool;
use sqlx::Row;

fn row_to_template(row: &sqlx::sqlite::SqliteRow) -> ReportTemplate {
    ReportTemplate {
        id: row.get("id"),
        title: row.get("title"),
        description: row.get("description"),
        template_html: row.get("template_html"),
        template_typst: row.get("template_typst"),
        template_docx: row.get("template_docx"),
        params_object_type_id: row.get("params_object_type_id"),
        created_at: row.get("created_at"),
        updated_at: row.get("updated_at"),
    }
}

/// Provides CRUD operations over `report_templates`.
#[derive(Clone)]
pub struct ReportTemplateStore {
    pool: Pool,
}

impl ReportTemplateStore {
    /// Create a new store bound to the given pool.
    pub fn new(pool: Pool) -> Self {
        Self { pool }
    }

    /// Create a new report template.
    pub async fn create(&self, new: &NewReportTemplate) -> Result<ReportTemplate> {
        if new.id.trim().is_empty() {
            return Err(ReportError::InvalidInput("id cannot be empty".into()));
        }
        if new.params_object_type_id.trim().is_empty() {
            return Err(ReportError::InvalidInput(
                "params_object_type_id cannot be empty".into(),
            ));
        }
        if new.template_html.is_none()
            && new.template_typst.is_none()
            && new.template_docx.is_none()
        {
            return Err(ReportError::InvalidInput(
                "at least one of template_html/template_typst/template_docx must be provided"
                    .into(),
            ));
        }

        let now = Utc::now().to_rfc3339();

        let exists: i64 = sqlx::query("SELECT COUNT(*) FROM report_templates WHERE id = ?")
            .bind(&new.id)
            .fetch_one(self.pool.inner())
            .await?
            .get(0);

        if exists > 0 {
            return Err(ReportError::InvalidInput(format!(
                "template '{}' already exists",
                new.id
            )));
        }

        sqlx::query(
            "INSERT INTO report_templates \
             (id, title, description, template_html, template_typst, template_docx, params_object_type_id, created_at, updated_at) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(&new.id)
        .bind(&new.title)
        .bind(&new.description)
        .bind(&new.template_html)
        .bind(&new.template_typst)
        .bind(&new.template_docx)
        .bind(&new.params_object_type_id)
        .bind(&now)
        .bind(&now)
        .execute(self.pool.inner())
        .await?;

        self.get(&new.id).await
    }

    /// Fetch one template.
    pub async fn get(&self, id: &str) -> Result<ReportTemplate> {
        let row = sqlx::query("SELECT * FROM report_templates WHERE id = ?")
            .bind(id)
            .fetch_optional(self.pool.inner())
            .await?;

        let Some(row) = row else {
            return Err(ReportError::NotFound(format!(
                "report template '{}' not found",
                id
            )));
        };

        Ok(row_to_template(&row))
    }

    /// List all templates.
    pub async fn list(&self) -> Result<Vec<ReportTemplate>> {
        let rows = sqlx::query("SELECT * FROM report_templates ORDER BY id")
            .fetch_all(self.pool.inner())
            .await?;

        Ok(rows.into_iter().map(|r| row_to_template(&r)).collect())
    }
}
