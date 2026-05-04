use super::*;

// ── distinct_repo_prefixes ────────────────────────────────────────────────────

#[test]
fn distinct_repos_empty_vec_returns_empty() {
    assert!(distinct_repo_prefixes(&[]).is_empty());
}

#[test]
fn distinct_repos_single_no_prefix_returns_empty() {
    let paths = vec!["crates/foo/src/lib.rs".to_string()];
    assert!(distinct_repo_prefixes(&paths).is_empty());
}

#[test]
fn distinct_repos_single_with_convergio_prefix() {
    let paths = vec!["convergio/crates/foo/src/lib.rs".to_string()];
    assert_eq!(distinct_repo_prefixes(&paths), vec!["convergio"]);
}

#[test]
fn distinct_repos_two_same_prefix_deduplicates() {
    let paths = vec![
        "convergio/crates/a/src/lib.rs".to_string(),
        "convergio/crates/b/src/main.rs".to_string(),
    ];
    assert_eq!(distinct_repo_prefixes(&paths), vec!["convergio"]);
}

#[test]
fn distinct_repos_two_different_prefixes() {
    let paths = vec![
        "convergio/crates/foo/src/lib.rs".to_string(),
        "convergio-edu/src/lesson.ts".to_string(),
    ];
    let got = distinct_repo_prefixes(&paths);
    assert_eq!(got, vec!["convergio", "convergio-edu"]);
}

#[test]
fn distinct_repos_three_prefixes_sorted() {
    let paths = vec![
        "convergio-ui/src/index.ts".to_string(),
        "convergio/crates/foo/src/lib.rs".to_string(),
        "convergio-edu/src/lesson.ts".to_string(),
    ];
    let got = distinct_repo_prefixes(&paths);
    assert_eq!(got, vec!["convergio", "convergio-edu", "convergio-ui"]);
}

#[test]
fn distinct_repos_non_convergio_prefix_ignored() {
    let paths = vec!["myapp/src/main.rs".to_string(), "other/lib.ts".to_string()];
    assert!(distinct_repo_prefixes(&paths).is_empty());
}

#[test]
fn distinct_repos_mixed_prefixed_and_bare() {
    let paths = vec![
        "convergio/crates/foo/src/lib.rs".to_string(),
        "bare/path/file.rs".to_string(),
        "convergio-edu/src/app.ts".to_string(),
    ];
    let got = distinct_repo_prefixes(&paths);
    assert_eq!(got, vec!["convergio", "convergio-edu"]);
}

// ── repo_prefix ───────────────────────────────────────────────────────────────

#[test]
fn repo_prefix_convergio_returns_some() {
    assert_eq!(
        repo_prefix("convergio/crates/foo/src/lib.rs"),
        Some("convergio".to_string())
    );
}

#[test]
fn repo_prefix_convergio_edu_returns_some() {
    assert_eq!(
        repo_prefix("convergio-edu/src/lesson.ts"),
        Some("convergio-edu".to_string())
    );
}

#[test]
fn repo_prefix_no_slash_returns_none() {
    assert!(repo_prefix("convergio").is_none());
}

#[test]
fn repo_prefix_non_convergio_returns_none() {
    assert!(repo_prefix("myapp/src/main.rs").is_none());
}

#[test]
fn repo_prefix_empty_returns_none() {
    assert!(repo_prefix("").is_none());
}

// ── strip_repo_prefix ─────────────────────────────────────────────────────────

#[test]
fn strip_prefix_strips_convergio_prefix() {
    assert_eq!(
        strip_repo_prefix("convergio/crates/foo/src/lib.rs"),
        "crates/foo/src/lib.rs"
    );
}

#[test]
fn strip_prefix_strips_convergio_edu_prefix() {
    assert_eq!(
        strip_repo_prefix("convergio-edu/src/lesson.ts"),
        "src/lesson.ts"
    );
}

#[test]
fn strip_prefix_bare_path_unchanged() {
    assert_eq!(
        strip_repo_prefix("crates/foo/src/lib.rs"),
        "crates/foo/src/lib.rs"
    );
}

#[test]
fn strip_prefix_non_convergio_unchanged() {
    assert_eq!(strip_repo_prefix("myapp/src/main.rs"), "myapp/src/main.rs");
}

#[test]
fn strip_prefix_empty_unchanged() {
    assert_eq!(strip_repo_prefix(""), "");
}

// ── recall_at_k ───────────────────────────────────────────────────────────────

#[test]
fn recall_empty_expected_returns_zero() {
    let retrieved = vec!["a".to_string(), "b".to_string()];
    assert_eq!(recall_at_k(&retrieved, &[], 10), 0.0);
}

#[test]
fn recall_empty_retrieved_returns_zero() {
    let expected = vec!["a".to_string()];
    assert_eq!(recall_at_k(&[], &expected, 10), 0.0);
}

#[test]
fn recall_all_hit_at_k() {
    let retrieved = vec!["a".to_string(), "b".to_string(), "c".to_string()];
    let expected = vec!["a".to_string(), "b".to_string()];
    assert!((recall_at_k(&retrieved, &expected, 10) - 1.0).abs() < f64::EPSILON);
}

#[test]
fn recall_partial_hit() {
    let retrieved = vec!["a".to_string(), "x".to_string()];
    let expected = vec!["a".to_string(), "b".to_string()];
    assert!((recall_at_k(&retrieved, &expected, 10) - 0.5).abs() < f64::EPSILON);
}

#[test]
fn recall_k_truncates_retrieved() {
    let retrieved = vec!["a".to_string(), "b".to_string(), "c".to_string()];
    let expected = vec!["c".to_string()];
    // k=2 excludes "c" at index 2
    assert_eq!(recall_at_k(&retrieved, &expected, 2), 0.0);
}

#[test]
fn recall_k_zero_returns_zero() {
    let retrieved = vec!["a".to_string()];
    let expected = vec!["a".to_string()];
    assert_eq!(recall_at_k(&retrieved, &expected, 0), 0.0);
}

#[test]
fn recall_no_hit_returns_zero() {
    let retrieved = vec!["x".to_string(), "y".to_string()];
    let expected = vec!["a".to_string(), "b".to_string()];
    assert_eq!(recall_at_k(&retrieved, &expected, 10), 0.0);
}

#[test]
fn recall_exact_k_boundary() {
    let retrieved = vec!["a".to_string(), "b".to_string()];
    let expected = vec!["b".to_string()];
    // k=2 includes "b"
    assert!((recall_at_k(&retrieved, &expected, 2) - 1.0).abs() < f64::EPSILON);
    // k=1 excludes "b"
    assert_eq!(recall_at_k(&retrieved, &expected, 1), 0.0);
}
