//! Unit tests for [`crate::fleet`].
//!
//! Split out of `fleet.rs` to honour the 300-line per-file cap
//! (CONSTITUTION § 13). Fixtures synthesize a tempdir-backed
//! `fleet.toml` plus the matching `tests/fixtures/retrieval-golden/`
//! tree so each finding bucket is exercised end-to-end.

#![cfg(test)]

use crate::fleet::build_report;
use std::io::Write;
use std::path::Path;
use tempfile::tempdir;

fn write(path: &Path, body: &str) {
    let mut f = std::fs::File::create(path).unwrap();
    f.write_all(body.as_bytes()).unwrap();
}

#[test]
fn missing_fleet_toml_reports_one_row() {
    let dir = tempdir().unwrap();
    let report = build_report(&dir.path().join("nope.toml")).unwrap();
    assert_eq!(report.rows.len(), 1);
    assert_eq!(report.rows[0].kind, "missing_fleet_toml");
}

#[test]
fn dangling_derives_from_is_flagged() {
    let dir = tempdir().unwrap();
    let toml = dir.path().join("fleet.toml");
    let repo_dir = dir.path().join("a");
    std::fs::create_dir_all(repo_dir.join("tests/fixtures/retrieval-golden/a")).unwrap();
    write(
        &toml,
        &format!(
            r#"
[fleet]
name = "test"

[[repo]]
name = "a"
path = "{}"
language = "rust"
parser = "syn"
role = "downstream"
derives_from = "ghost"
"#,
            repo_dir.display()
        ),
    );
    let report = build_report(&toml).unwrap();
    assert!(report
        .rows
        .iter()
        .any(|r| r.kind == "dangling_derives_from"));
}

#[test]
fn multiple_engine_roots_is_flagged() {
    let dir = tempdir().unwrap();
    let toml = dir.path().join("fleet.toml");
    let a = dir.path().join("a");
    let b = dir.path().join("b");
    std::fs::create_dir_all(a.join("tests/fixtures/retrieval-golden/a")).unwrap();
    std::fs::create_dir_all(b.join("tests/fixtures/retrieval-golden/b")).unwrap();
    write(
        &toml,
        &format!(
            r#"
[fleet]
name = "test"

[[repo]]
name = "a"
path = "{}"
language = "rust"
parser = "syn"
role = "engine"

[[repo]]
name = "b"
path = "{}"
language = "rust"
parser = "syn"
role = "engine"
"#,
            a.display(),
            b.display()
        ),
    );
    let report = build_report(&toml).unwrap();
    assert!(report
        .rows
        .iter()
        .any(|r| r.kind == "multiple_engine_roots"));
}

#[test]
fn missing_retrieval_golden_is_flagged() {
    let dir = tempdir().unwrap();
    let toml = dir.path().join("fleet.toml");
    let repo_dir = dir.path().join("solo");
    std::fs::create_dir_all(&repo_dir).unwrap();
    write(
        &toml,
        &format!(
            r#"
[fleet]
name = "test"

[[repo]]
name = "solo"
path = "{}"
language = "rust"
parser = "syn"
role = "downstream"
"#,
            repo_dir.display()
        ),
    );
    let report = build_report(&toml).unwrap();
    assert!(report
        .rows
        .iter()
        .any(|r| r.kind == "missing_retrieval_golden"));
}
