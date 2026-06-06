//! Integration test: JSON Schema param validation against an ontology `ObjectType`.

use convergio_db::Pool;
use convergio_ontology::Store as OntologyStore;
use convergio_reports::{
    render_report, NewReportTemplate, ProvenanceInput, RenderFormat, RenderReportRequest,
    ReportTemplateStore,
};
use serde_json::json;
use tempfile::tempdir;

#[tokio::test]
async fn rejects_params_not_matching_object_type_schema() {
    let dir = tempdir().expect("tempdir");
    let db_url = format!("sqlite://{}/state.db?mode=rwc", dir.path().display());
    let pool = Pool::connect(&db_url).await.expect("connect db");

    convergio_ontology::init(&pool)
        .await
        .expect("ontology init");
    convergio_reports::init(&pool).await.expect("reports init");

    let ontology = OntologyStore::new(pool.clone());
    let templates = ReportTemplateStore::new(pool);

    ontology
        .upsert_object(
            "demo.strict",
            1,
            false,
            "Strict",
            "Strict schema",
            json!({
                "$schema": "http://json-schema.org/draft-07/schema#",
                "type": "object",
                "properties": {"name": {"type": "string"}},
                "required": ["name"],
                "additionalProperties": false
            }),
            None,
        )
        .await
        .expect("register object type");

    templates
        .create(&NewReportTemplate {
            id: "demo.t".into(),
            title: "T".into(),
            description: "T".into(),
            template_html: Some("Hello {{ name }}".into()),
            template_typst: None,
            template_docx: None,
            params_object_type_id: "demo.strict".into(),
        })
        .await
        .expect("create template");

    let err = render_report(
        &templates,
        &ontology,
        &RenderReportRequest {
            template_id: "demo.t".into(),
            format: RenderFormat::Html,
            params: json!({"name": 123}),
            provenance: ProvenanceInput {
                plan_id: "P".into(),
                task_id: "T".into(),
                agent_id: "A".into(),
            },
        },
    )
    .await
    .expect_err("should fail");

    assert!(err.to_string().contains("validation"));
}
