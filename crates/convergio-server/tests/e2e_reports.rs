//! E2E test for the report engine HTTP surface.

mod common;

use base64::Engine as _;
use common::{boot, client};
use serde_json::{json, Value};

fn decode_b64(s: &str) -> Vec<u8> {
    base64::engine::general_purpose::STANDARD
        .decode(s)
        .expect("base64 decode")
}

#[tokio::test]
async fn reports_templates_and_render_round_trip() {
    let (base, pool, _dir) = boot().await;
    let http = client();

    // Sanity: built-in ReportTemplate ObjectType is registered.
    let types: Value = http
        .get(format!("{base}/v1/ontology/types"))
        .send()
        .await
        .expect("list object types")
        .json()
        .await
        .expect("types json");
    assert!(
        types["objects"]
            .as_array()
            .expect("objects array")
            .iter()
            .any(|t| { t.get("name") == Some(&Value::String("cvg.report_template.v1".into())) }),
        "expected built-in report template type in ontology list"
    );

    // Register an ObjectType used to validate render parameters.
    convergio_ontology::Store::new(pool.clone())
        .upsert_object(
            "demo.report_params.v1",
            1,
            false,
            "DemoReportParams",
            "Params for demo report",
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
        .expect("upsert object type");

    // Creating a template with an unknown params ObjectType must 404.
    let missing = http
        .post(format!("{base}/v1/reports/templates"))
        .json(&json!({
            "id": "demo.missing",
            "title": "Missing",
            "description": "Missing",
            "template_html": "Hello",
            "params_object_type_id": "no.such.type"
        }))
        .send()
        .await
        .expect("create template");
    assert_eq!(missing.status(), 404);

    // Create a template that supports all formats.
    let created: Value = http
        .post(format!("{base}/v1/reports/templates"))
        .json(&json!({
            "id": "demo.invoice",
            "title": "Invoice",
            "description": "Demo invoice",
            "template_html": "<h1>Invoice for {{ name }}</h1><p>Amount: {{ amount }}</p>",
            "template_typst": "= Invoice for {{ name }}\nAmount: {{ amount }}",
            "template_docx": "Invoice for {{ name }}\nAmount: {{ amount }}",
            "params_object_type_id": "demo.report_params.v1"
        }))
        .send()
        .await
        .expect("create template")
        .json()
        .await
        .expect("template json");
    assert_eq!(created["id"], "demo.invoice");

    // List + get.
    let list: Vec<Value> = http
        .get(format!("{base}/v1/reports/templates"))
        .send()
        .await
        .expect("list templates")
        .json()
        .await
        .expect("list json");
    assert!(
        list.iter()
            .any(|t| t.get("id") == Some(&Value::String("demo.invoice".into()))),
        "created template should be present in list"
    );

    let got: Value = http
        .get(format!("{base}/v1/reports/templates/demo.invoice"))
        .send()
        .await
        .expect("get template")
        .json()
        .await
        .expect("get json");
    assert_eq!(got["params_object_type_id"], "demo.report_params.v1");

    let provenance = json!({
        "plan_id": "P-demo",
        "task_id": "T-demo",
        "agent_id": "A-demo"
    });

    // HTML render includes embedded manifest and original content.
    let html: Value = http
        .post(format!("{base}/v1/reports/render"))
        .json(&json!({
            "template_id": "demo.invoice",
            "format": "html",
            "params": {"name": "Alice", "amount": 12.5},
            "provenance": provenance,
        }))
        .send()
        .await
        .expect("render html")
        .json()
        .await
        .expect("render json");
    assert_eq!(html["ok"], true);
    assert!(html["mime_type"].as_str().unwrap().starts_with("text/html"));
    assert_eq!(
        html.pointer("/manifest/provenance/plan_id").unwrap(),
        "P-demo"
    );
    let html_bytes = decode_b64(html["bytes_base64"].as_str().expect("bytes_base64"));
    let html_str = std::str::from_utf8(&html_bytes).expect("html utf8");
    assert!(html_str.contains("Invoice for Alice"));
    assert!(html_str.contains("convergio-report-manifest"));

    // Schema validation must refuse mismatched params as 422.
    let bad = http
        .post(format!("{base}/v1/reports/render"))
        .json(&json!({
            "template_id": "demo.invoice",
            "format": "html",
            "params": {"name": 123, "amount": 12.5},
            "provenance": {
                "plan_id": "P-demo",
                "task_id": "T-demo",
                "agent_id": "A-demo"
            }
        }))
        .send()
        .await
        .expect("render bad");
    assert_eq!(bad.status(), 422);

    // PDF render returns %PDF bytes.
    let pdf: Value = http
        .post(format!("{base}/v1/reports/render"))
        .json(&json!({
            "template_id": "demo.invoice",
            "format": "pdf",
            "params": {"name": "Alice", "amount": 12.5},
            "provenance": {
                "plan_id": "P-demo",
                "task_id": "T-demo",
                "agent_id": "A-demo"
            }
        }))
        .send()
        .await
        .expect("render pdf")
        .json()
        .await
        .expect("pdf json");
    let pdf_bytes = decode_b64(pdf["bytes_base64"].as_str().expect("pdf bytes"));
    assert!(pdf_bytes.starts_with(b"%PDF"));

    // DOCX render should produce a non-trivial zip payload.
    let docx: Value = http
        .post(format!("{base}/v1/reports/render"))
        .json(&json!({
            "template_id": "demo.invoice",
            "format": "docx",
            "params": {"name": "Alice", "amount": 12.5},
            "provenance": {
                "plan_id": "P-demo",
                "task_id": "T-demo",
                "agent_id": "A-demo"
            }
        }))
        .send()
        .await
        .expect("render docx")
        .json()
        .await
        .expect("docx json");
    let docx_bytes = decode_b64(docx["bytes_base64"].as_str().expect("docx bytes"));
    assert!(docx_bytes.len() > 1000);
    assert!(docx_bytes.starts_with(b"PK"), "docx is a zip container");
}
