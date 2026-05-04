//! Fleet-scope recall benchmark — extends the single-repo bench to cover
//! cross-repo fixtures from all subdirectories of `retrieval-golden/`.
//!
//! Run with:
//!   cargo test -p convergio-embed --test recall_bench_fleet \
//!       -- --ignored --nocapture

#![allow(clippy::expect_used)]

use serde::Deserialize;
use std::collections::{BTreeSet, HashSet};
use std::path::{Path, PathBuf};

#[derive(Debug, Deserialize, Clone)]
#[allow(dead_code)]
struct FleetBenchFixture {
    task_id: String,
    #[serde(default)]
    repo: String,
    title: String,
    task_body: String,
    expected_files: Vec<String>,
}

// ── path helpers ──────────────────────────────────────────────────────────────

fn golden_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("tests/fixtures/retrieval-golden")
}

fn distinct_repo_prefixes(paths: &[String]) -> Vec<String> {
    let mut seen = BTreeSet::new();
    for p in paths {
        if let Some((head, _)) = p.split_once('/') {
            if head.starts_with("convergio") {
                seen.insert(head.to_string());
            }
        }
    }
    seen.into_iter().collect()
}

fn is_cross_repo(fx: &FleetBenchFixture) -> bool {
    distinct_repo_prefixes(&fx.expected_files).len() >= 2
}

fn bench_recall_at_k(retrieved: &[String], expected: &[String], k: usize) -> f64 {
    if expected.is_empty() {
        return 0.0;
    }
    let top_k: HashSet<&String> = retrieved.iter().take(k).collect();
    let hits = expected.iter().filter(|e| top_k.contains(*e)).count();
    hits as f64 / expected.len() as f64
}

fn load_fixtures_from_dir(dir: &Path) -> Vec<FleetBenchFixture> {
    if !dir.is_dir() {
        return Vec::new();
    }
    let mut out = Vec::new();
    for entry in std::fs::read_dir(dir).expect("read dir").flatten() {
        if entry.path().extension().and_then(|e| e.to_str()) != Some("json") {
            continue;
        }
        let raw = std::fs::read_to_string(entry.path()).expect("read fixture");
        let fx: FleetBenchFixture = serde_json::from_str(&raw).expect("parse fixture");
        out.push(fx);
    }
    out.sort_by(|a, b| a.task_id.cmp(&b.task_id));
    out
}

fn load_all_fleet_fixtures(root: &Path) -> Vec<FleetBenchFixture> {
    if !root.is_dir() {
        return Vec::new();
    }
    let mut all = Vec::new();
    for entry in std::fs::read_dir(root).expect("read golden root").flatten() {
        if entry.path().is_dir() {
            all.extend(load_fixtures_from_dir(&entry.path()));
        }
    }
    all.sort_by(|a, b| a.task_id.cmp(&b.task_id));
    all
}

fn make_bench_fx(id: &str, expected_files: Vec<&str>) -> FleetBenchFixture {
    FleetBenchFixture {
        task_id: id.into(),
        repo: "convergio-edu".into(),
        title: "test".into(),
        task_body: "body".into(),
        expected_files: expected_files.iter().map(|s| s.to_string()).collect(),
    }
}

// ── unit tests ────────────────────────────────────────────────────────────────

#[test]
fn distinct_prefixes_empty_paths() {
    assert!(distinct_repo_prefixes(&[]).is_empty());
}

#[test]
fn distinct_prefixes_bare_paths_ignored() {
    let paths = vec!["crates/foo/src/lib.rs".to_string()];
    assert!(distinct_repo_prefixes(&paths).is_empty());
}

#[test]
fn distinct_prefixes_single_convergio_prefix() {
    let paths = vec!["convergio/crates/foo/src/lib.rs".to_string()];
    assert_eq!(distinct_repo_prefixes(&paths), vec!["convergio"]);
}

#[test]
fn distinct_prefixes_two_different_repos() {
    let paths = vec![
        "convergio/crates/embed/src/lib.rs".to_string(),
        "convergio-edu/src/lesson.ts".to_string(),
    ];
    assert_eq!(
        distinct_repo_prefixes(&paths),
        vec!["convergio", "convergio-edu"]
    );
}

#[test]
fn distinct_prefixes_deduplicates_same_repo() {
    let paths = vec![
        "convergio/crates/a/src/lib.rs".to_string(),
        "convergio/crates/b/src/lib.rs".to_string(),
    ];
    assert_eq!(distinct_repo_prefixes(&paths), vec!["convergio"]);
}

#[test]
fn is_cross_repo_false_for_bare_paths() {
    let fx = make_bench_fx("T-001", vec!["src/lesson.ts"]);
    assert!(!is_cross_repo(&fx));
}

#[test]
fn is_cross_repo_false_for_single_prefixed_repo() {
    let fx = make_bench_fx("T-002", vec!["convergio/crates/foo/src/lib.rs"]);
    assert!(!is_cross_repo(&fx));
}

#[test]
fn is_cross_repo_true_for_two_repos() {
    let fx = make_bench_fx(
        "T-003",
        vec![
            "convergio/crates/embed/src/lib.rs",
            "convergio-edu/src/lesson.ts",
        ],
    );
    assert!(is_cross_repo(&fx));
}

#[test]
fn bench_recall_at_k_empty_expected_zero() {
    assert_eq!(bench_recall_at_k(&["a".to_string()], &[], 10), 0.0);
}

#[test]
fn bench_recall_at_k_empty_retrieved_zero() {
    let expected = vec!["a".to_string()];
    assert_eq!(bench_recall_at_k(&[], &expected, 10), 0.0);
}

#[test]
fn bench_recall_at_k_perfect_recall() {
    let items = vec!["a".to_string(), "b".to_string()];
    assert!((bench_recall_at_k(&items, &items, 10) - 1.0).abs() < f64::EPSILON);
}

#[test]
fn bench_recall_at_k_half_recall() {
    let retrieved = vec!["a".to_string(), "x".to_string()];
    let expected = vec!["a".to_string(), "b".to_string()];
    assert!((bench_recall_at_k(&retrieved, &expected, 10) - 0.5).abs() < f64::EPSILON);
}

#[test]
fn bench_recall_at_k_cutoff_excludes_late_hits() {
    let retrieved = vec!["x".to_string(), "y".to_string(), "a".to_string()];
    let expected = vec!["a".to_string()];
    assert_eq!(bench_recall_at_k(&retrieved, &expected, 2), 0.0);
    assert!((bench_recall_at_k(&retrieved, &expected, 3) - 1.0).abs() < f64::EPSILON);
}

#[test]
fn load_from_nonexistent_paths_returns_empty() {
    assert!(load_fixtures_from_dir(Path::new("/tmp/nonexistent-golden-99999")).is_empty());
    assert!(load_all_fleet_fixtures(Path::new("/tmp/nonexistent-golden-root-99999")).is_empty());
}

#[test]
fn golden_root_path_is_deterministic() {
    assert_eq!(golden_root(), golden_root());
}

#[test]
fn fixture_subset_counts_consistent() {
    let root = golden_root();
    let all = load_all_fleet_fixtures(&root);
    let cross = all.iter().filter(|f| is_cross_repo(f)).count();
    let single = all.iter().filter(|f| !is_cross_repo(f)).count();
    assert!(cross <= all.len());
    assert!(single <= all.len());
    assert_eq!(cross + single, all.len());
}

#[test]
fn all_fixtures_have_nonempty_task_id() {
    let root = golden_root();
    for fx in load_all_fleet_fixtures(&root) {
        assert!(!fx.task_id.is_empty(), "empty task_id in fixture");
    }
}

#[test]
fn all_fixtures_have_nonempty_expected_files() {
    let root = golden_root();
    for fx in load_all_fleet_fixtures(&root) {
        assert!(
            !fx.expected_files.is_empty(),
            "empty expected_files in {}",
            fx.task_id
        );
    }
}

#[test]
fn fixtures_sorted_by_task_id() {
    let root = golden_root();
    let all = load_all_fleet_fixtures(&root);
    let ids: Vec<_> = all.iter().map(|f| &f.task_id).collect();
    let mut sorted = ids.clone();
    sorted.sort();
    assert_eq!(ids, sorted);
}

// ── fleet-scope bench (slow, opt-in) ──────────────────────────────────────────

#[tokio::test]
#[ignore = "slow fleet bench; opt-in via --ignored --nocapture"]
async fn recall_bench_fleet_scope() {
    let root = golden_root();
    let all = load_all_fleet_fixtures(&root);
    if all.is_empty() {
        eprintln!("no fixtures under {}", root.display());
        return;
    }
    let cross: Vec<_> = all.iter().filter(|f| is_cross_repo(f)).cloned().collect();
    eprintln!(
        "F2-bench: total={} cross_repo={} single_repo={}",
        all.len(),
        cross.len(),
        all.len() - cross.len()
    );
    for fx in &cross {
        eprintln!(
            "  cross-repo {:<50} repos={:?}",
            fx.task_id,
            distinct_repo_prefixes(&fx.expected_files)
        );
    }
    let avg = if cross.is_empty() {
        0.0
    } else {
        cross
            .iter()
            .map(|f| bench_recall_at_k(&[], &f.expected_files, 10))
            .sum::<f64>()
            / cross.len() as f64
    };
    eprintln!("F2-bench cross_repo recall@10 (empty retrieval baseline): {avg:.3}");
}
