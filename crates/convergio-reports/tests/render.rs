//! Integration test: render HTML/PDF/DOCX outputs and verify provenance embedding.

use convergio_db::Pool;
use convergio_ontology::Store as OntologyStore;
use convergio_reports::{
    render_report, NewReportTemplate, ProvenanceInput, RenderFormat, RenderReportRequest,
    ReportTemplateStore,
};
use serde_json::json;
use tempfile::tempdir;

#[tokio::test]
async fn renders_html_pdf_docx_with_manifest() {
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
            "demo.report_params",
            1,
            false,
            "Demo report params",
            "Params for demo reports",
            json!({
                "$schema": "http://json-schema.org/draft-07/schema#",
                "type": "object",
                "properties": {
                    "name": {"type": "string"},
                    "amount": {"type": "number"}
                },
                "required": ["name", "amount"],
                "additionalProperties": false
            }),
            None,
        )
        .await
        .expect("register object type");

    templates
        .create(&NewReportTemplate {
            id: "demo.invoice".into(),
            title: "Invoice".into(),
            description: "Demo invoice".into(),
            template_html: Some(
                "<h1>Invoice for {{ name }}</h1><p>Amount: {{ amount }}</p>".into(),
            ),
            template_typst: Some("= Invoice for {{ name }}\nAmount: {{ amount }}".into()),
            template_docx: Some("Invoice for {{ name }}\nAmount: {{ amount }}".into()),
            params_object_type_id: "demo.report_params".into(),
        })
        .await
        .expect("create template");

    let req = RenderReportRequest {
        template_id: "demo.invoice".into(),
        format: RenderFormat::Html,
        params: json!({"name": "Alice", "amount": 12.5}),
        provenance: ProvenanceInput {
            plan_id: "P1".into(),
            task_id: "T1".into(),
            agent_id: "A1".into(),
        },
    };

    let html = render_report(&templates, &ontology, &req)
        .await
        .expect("render html");
    let html_str = std::str::from_utf8(&html.bytes).expect("utf8");
    assert!(html_str.contains("convergio-report-manifest"));
    assert!(html_str.contains("data-base64="));
    assert!(html_str.contains("Invoice for Alice"));

    let pdf = render_report(
        &templates,
        &ontology,
        &RenderReportRequest {
            format: RenderFormat::Pdf,
            ..req.clone()
        },
    )
    .await
    .expect("render pdf");
    assert!(pdf.bytes.starts_with(b"%PDF"));

    let docx = render_report(
        &templates,
        &ontology,
        &RenderReportRequest {
            format: RenderFormat::Docx,
            ..req
        },
    )
    .await
    .expect("render docx");
    assert!(docx.bytes.len() > 1000);
    assert_eq!(
        docx.mime_type,
        "application/vnd.openxmlformats-officedocument.wordprocessingml.document"
    );
}
