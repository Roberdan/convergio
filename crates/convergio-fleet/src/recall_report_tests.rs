use super::*;

// ── helpers ───────────────────────────────────────────────────────────────────

fn make_fixture(expected_files: Vec<&str>) -> FleetFixture {
    FleetFixture {
        task_id: "T-test-001".to_string(),
        repo: "convergio-edu".to_string(),
        title: "test".to_string(),
        task_body: "test body".to_string(),
        expected_files: expected_files.iter().map(|s| s.to_string()).collect(),
        rationale: None,
        curator: None,
        curated_at: None,
        schema_version: 1,
    }
}

// ── FleetFixture::is_cross_repo / referenced_repos ───────────────────────────

#[test]
fn fixture_bare_paths_not_cross_repo() {
    let fx = make_fixture(vec!["crates/foo/src/lib.rs", "crates/bar/src/main.rs"]);
    assert!(!fx.is_cross_repo());
    assert!(fx.referenced_repos().is_empty());
}

#[test]
fn fixture_single_prefixed_repo_not_cross_repo() {
    let fx = make_fixture(vec!["convergio/crates/foo/src/lib.rs"]);
    assert!(!fx.is_cross_repo());
}

#[test]
fn fixture_cross_repo_two_repos() {
    let fx = make_fixture(vec![
        "convergio/crates/embed/src/lib.rs",
        "convergio-edu/src/lesson.ts",
    ]);
    assert!(fx.is_cross_repo());
}

#[test]
fn fixture_cross_repo_three_repos() {
    let fx = make_fixture(vec![
        "convergio/crates/embed/src/lib.rs",
        "convergio-edu/src/lesson.ts",
        "convergio-ui/src/index.ts",
    ]);
    assert!(fx.is_cross_repo());
    assert_eq!(fx.referenced_repos().len(), 3);
}

#[test]
fn fixture_referenced_repos_empty_when_bare() {
    let fx = make_fixture(vec!["crates/foo/src/lib.rs"]);
    assert!(fx.referenced_repos().is_empty());
}

#[test]
fn fixture_referenced_repos_deduplicates() {
    let fx = make_fixture(vec![
        "convergio/crates/a/src/lib.rs",
        "convergio/crates/b/src/lib.rs",
    ]);
    assert_eq!(fx.referenced_repos(), vec!["convergio"]);
}

// ── FleetRecallReport ─────────────────────────────────────────────────────────

#[test]
fn report_empty_fixtures() {
    let report = FleetRecallReport::from_fixtures(&[]);
    assert_eq!(
        report,
        FleetRecallReport {
            total: 0,
            cross_repo: 0,
            single_repo: 0
        }
    );
}

#[test]
fn report_all_single_repo() {
    let fixtures = vec![
        make_fixture(vec!["crates/a/src/lib.rs"]),
        make_fixture(vec!["crates/b/src/lib.rs"]),
    ];
    let report = FleetRecallReport::from_fixtures(&fixtures);
    assert_eq!(report.total, 2);
    assert_eq!(report.cross_repo, 0);
    assert_eq!(report.single_repo, 2);
}

#[test]
fn report_all_cross_repo() {
    let fixtures = vec![
        make_fixture(vec!["convergio/a.rs", "convergio-edu/b.ts"]),
        make_fixture(vec!["convergio/c.rs", "convergio-edu/d.ts"]),
    ];
    let report = FleetRecallReport::from_fixtures(&fixtures);
    assert_eq!(report.total, 2);
    assert_eq!(report.cross_repo, 2);
    assert_eq!(report.single_repo, 0);
}

#[test]
fn report_mixed_fixtures() {
    let fixtures = vec![
        make_fixture(vec!["convergio/a.rs", "convergio-edu/b.ts"]),
        make_fixture(vec!["crates/foo/src/lib.rs"]),
        make_fixture(vec!["convergio/c.rs", "convergio-edu/d.ts"]),
    ];
    let report = FleetRecallReport::from_fixtures(&fixtures);
    assert_eq!(report.total, 3);
    assert_eq!(report.cross_repo, 2);
    assert_eq!(report.single_repo, 1);
}

#[test]
fn report_total_invariant() {
    let fixtures = vec![
        make_fixture(vec!["convergio/a.rs", "convergio-edu/b.ts"]),
        make_fixture(vec!["bare/path.rs"]),
    ];
    let report = FleetRecallReport::from_fixtures(&fixtures);
    assert_eq!(report.total, report.cross_repo + report.single_repo);
}

// ── additional edge cases (F2-12) ────────────────────────────────────────────

#[test]
fn report_single_fixture_cross_repo() {
    let fixtures = vec![make_fixture(vec!["convergio/a.rs", "convergio-edu/b.ts"])];
    let report = FleetRecallReport::from_fixtures(&fixtures);
    assert_eq!(report.total, 1);
    assert_eq!(report.cross_repo, 1);
    assert_eq!(report.single_repo, 0);
}

#[test]
fn fixture_schema_version_defaults_to_one() {
    let fx = make_fixture(vec!["crates/foo/src/lib.rs"]);
    assert_eq!(fx.schema_version, 1);
}

#[test]
fn fixture_cross_repo_count_matches_report() {
    let fixtures: Vec<FleetFixture> = vec![
        make_fixture(vec!["convergio/a.rs", "convergio-edu/b.ts"]),
        make_fixture(vec!["convergio/c.rs"]),
        make_fixture(vec!["convergio-edu/d.ts", "convergio-ui/e.ts"]),
    ];
    let report = FleetRecallReport::from_fixtures(&fixtures);
    let manual_count = fixtures.iter().filter(|f| f.is_cross_repo()).count();
    assert_eq!(report.cross_repo, manual_count);
}
