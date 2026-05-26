use super::*;
use crate::commands::public::algorithms_schema::ReleaseGateRegistry;

#[test]
fn validate_slug_rejects_unsafe() {
    assert!(validate_slug("").is_err());
    assert!(validate_slug("..").is_err());
    assert!(validate_slug("a/b").is_err());
    assert!(validate_slug("UPPER").is_err());
    assert!(validate_slug("white space").is_err());
    assert!(validate_slug("ü").is_err());
}

#[test]
fn generate_writes_index_and_pages() {
    let tmp = tempfile::tempdir().unwrap();
    let out = tmp.path().join("out");
    let reg = tmp.path().join("registry.json");

    let json = r#"{
  "schema_version": "1",
  "tenant": "acme",
  "generated_at": "2026-05-26T00:00:00Z",
  "algorithms": [
    {
      "slug": "doc-summariser",
      "action": "summarise_documents",
      "title": {"en": "Document summariser", "it": "Riassuntore documenti"},
      "purpose": {"en": "Provide summaries", "it": "Fornire riassunti"},
      "lawful_basis": {"en": "Public task", "it": "Compito di interesse pubblico"},
      "data_categories": [
        {"en": "Free-form text", "it": "Testo libero"}
      ],
      "model": {"name": "gpt-5.2", "version": "2026-05", "provider": "Example"},
      "region": "EU",
      "oversight": {"en": "Human review", "it": "Revisione umana"},
      "risk_class": "low",
      "limitations": {"en": "May omit details", "it": "Può omettere dettagli"},
      "appeal_contact": {}
    }
  ]
}"#;
    std::fs::write(&reg, json).unwrap();

    let bundle = Bundle::new(convergio_i18n::Locale::En).unwrap();
    generate_algorithms(&bundle, OutputMode::Plain, &reg, &out, "acme").unwrap();

    let idx = out.join("acme").join("algorithms").join("index.html");
    let page = out
        .join("acme")
        .join("algorithms")
        .join("doc-summariser")
        .join("index.html");
    assert!(idx.exists());
    assert!(page.exists());

    let idx_body = std::fs::read_to_string(idx).unwrap();
    assert!(idx_body.contains("Algorithm Register"));
    assert!(idx_body.contains("doc-summariser"));

    let page_body = std::fs::read_to_string(page).unwrap();
    assert!(page_body.contains("summarise_documents"));
    // When fields are absent, we still render stable "none" placeholders.
    assert!(page_body.contains("none / nessuno"));
}

#[test]
fn validate_algorithms_rejects_duplicate_action() {
    let json = r#"{
  "schema_version": "1",
  "algorithms": [
    {
      "slug": "a",
      "action": "same",
      "title": {"en": "A", "it": "A"},
      "purpose": {"en": "A", "it": "A"},
      "lawful_basis": {"en": "A", "it": "A"},
      "data_categories": [],
      "model": {"name": "m"},
      "region": "EU",
      "oversight": {"en": "A", "it": "A"},
      "risk_class": "low",
      "limitations": {"en": "A", "it": "A"},
      "appeal_contact": {}
    },
    {
      "slug": "b",
      "action": "same",
      "title": {"en": "B", "it": "B"},
      "purpose": {"en": "B", "it": "B"},
      "lawful_basis": {"en": "B", "it": "B"},
      "data_categories": [],
      "model": {"name": "m"},
      "region": "EU",
      "oversight": {"en": "B", "it": "B"},
      "risk_class": "low",
      "limitations": {"en": "B", "it": "B"},
      "appeal_contact": {}
    }
  ]
}"#;

    let doc: ReleaseGateRegistry = serde_json::from_str(json).unwrap();
    let err = validate_algorithms(&doc.algorithms).unwrap_err();
    assert!(err.to_string().contains("duplicate AI Action"));
}
