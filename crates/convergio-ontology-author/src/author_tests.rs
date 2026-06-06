//! End-to-end pipeline tests using deterministic test doubles.

use std::path::PathBuf;

use chrono::{TimeZone, Utc};
use tempfile::tempdir;

use crate::author::{author, AuthorOptions};
use crate::ingest::testing::StubConverter;
use crate::intent::{AuthoringRequest, Intent};
use crate::propose::testing::StubProposer;

const VALID: &str = r#"{
  "name": "sis",
  "objects": [
    {"name": "Student", "title": "Student", "description": "A learner"},
    {"name": "Course", "title": "Course", "description": "A unit of study"},
    {"name": "Enrollment", "title": "Enrollment", "description": "A registration"}
  ],
  "properties": [
    {"name": "email", "owner": "Student", "datatype": "string", "required": true, "title": "Email"},
    {"name": "credits", "owner": "Course", "datatype": "integer", "required": false, "title": "Credits"}
  ],
  "links": [
    {"name": "enrolled_in", "from": "Student", "to": "Course", "title": "Enrolled in"}
  ]
}"#;

fn fixed_opts(out: PathBuf) -> AuthorOptions {
    AuthorOptions {
        out,
        max_attempts: 3,
        now: Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap(),
    }
}

#[test]
fn happy_path_emits_standard_artifacts() {
    let dir = tempdir().unwrap();
    let request = AuthoringRequest::from_intent(Intent {
        prompt: "model a student information system".into(),
        industry: "higher-education".into(),
        use_case: "sis".into(),
    });
    let converter = StubConverter::default();
    let proposer = StubProposer::new(vec![VALID.to_string()]);
    let opts = fixed_opts(dir.path().to_path_buf());

    let outcome = author(&request, &converter, &proposer, &opts).unwrap();
    assert_eq!(outcome.attempts, 1);
    assert_eq!(outcome.draft.objects.len(), 3);

    let owl = std::fs::read_to_string(&outcome.artifacts.owl).unwrap();
    assert!(owl.contains(":Student a owl:Class"));
    assert!(owl.contains(":enrolled_in a owl:ObjectProperty"));
    assert!(owl.contains("rdfs:range xsd:integer"));

    assert_eq!(outcome.artifacts.json_schema.len(), 3);
    assert_eq!(outcome.artifacts.shacl.len(), 3);
    let prov = std::fs::read_to_string(&outcome.artifacts.provenance).unwrap();
    assert!(prov.contains("ontology.author"));
}

#[test]
fn deterministic_across_runs() {
    let request = AuthoringRequest::from_intent(Intent {
        prompt: "x".into(),
        industry: "y".into(),
        use_case: "z".into(),
    });

    let run = || {
        let dir = tempdir().unwrap();
        let proposer = StubProposer::new(vec![VALID.to_string()]);
        let outcome = author(
            &request,
            &StubConverter::default(),
            &proposer,
            &fixed_opts(dir.path().to_path_buf()),
        )
        .unwrap();
        std::fs::read_to_string(&outcome.artifacts.owl).unwrap()
    };
    assert_eq!(run(), run());
}

#[test]
fn repair_loop_recovers_from_invalid_first_attempt() {
    // First response has a dangling link target; second is valid.
    let invalid = r#"{"name":"sis","objects":[{"name":"Student","title":"Student"}],
        "properties":[],"links":[{"name":"enrolled_in","from":"Student","to":"Ghost","title":"E"}]}"#;
    let dir = tempdir().unwrap();
    let request = AuthoringRequest::from_intent(Intent {
        prompt: "model".into(),
        industry: "edu".into(),
        use_case: "sis".into(),
    });
    let proposer = StubProposer::new(vec![invalid.to_string(), VALID.to_string()]);
    let outcome = author(
        &request,
        &StubConverter::default(),
        &proposer,
        &fixed_opts(dir.path().to_path_buf()),
    )
    .unwrap();

    assert_eq!(outcome.attempts, 2);
    let seen = proposer.seen.borrow();
    assert!(seen[1].contains("failed validation"));
    assert!(seen[1].contains("Ghost"));
}

#[test]
fn gives_up_after_attempt_budget() {
    let invalid = r#"{"name":"sis","objects":[],"properties":[],"links":[]}"#;
    let dir = tempdir().unwrap();
    let request = AuthoringRequest::from_intent(Intent {
        prompt: "model".into(),
        industry: "edu".into(),
        use_case: "sis".into(),
    });
    let proposer = StubProposer::new(vec![
        invalid.to_string(),
        invalid.to_string(),
        invalid.to_string(),
    ]);
    let mut opts = fixed_opts(dir.path().to_path_buf());
    opts.max_attempts = 2;
    let err = author(&request, &StubConverter::default(), &proposer, &opts).unwrap_err();
    assert!(matches!(
        err,
        crate::error::AuthorError::Unrepaired { attempts: 2, .. }
    ));
}

#[test]
fn empty_request_is_rejected() {
    let dir = tempdir().unwrap();
    let request = AuthoringRequest::default();
    let proposer = StubProposer::new(vec![VALID.to_string()]);
    let err = author(
        &request,
        &StubConverter::default(),
        &proposer,
        &fixed_opts(dir.path().to_path_buf()),
    )
    .unwrap_err();
    assert!(matches!(err, crate::error::AuthorError::EmptyRequest));
}
