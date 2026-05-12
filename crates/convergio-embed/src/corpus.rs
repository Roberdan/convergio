//! Build [`IngestNode`] inputs from a directory tree.
//!
//! Walks `root` recursively, picks files matching the requested
//! extensions, reads up to `max_lines` of each, and turns them into
//! [`IngestNode`]s keyed by their repo-relative path. The result is
//! handed to [`crate::ingest`] for the actual embedding pass.
//!
//! ADR-0038 § 5.4 calls for embedding "first 200 LOC" of files —
//! `max_lines` is the local knob enforcing that.

use crate::ingest::IngestNode;
use std::path::Path;

/// File-extension filter (lowercase, without the leading dot).
pub type ExtensionFilter<'a> = &'a [&'a str];

/// Tally of paths the corpus walk had to skip, returned by
/// [`collect_files_report`] so orchestrators can surface coverage
/// loss instead of silently shrinking the corpus.
///
/// Convergio's zero-tolerance rule (CONSTITUTION § Sacred principles)
/// forbids dropping filesystem errors on the floor. The `walk_errors`
/// and `unreadable` counters exist so a caller — or a future audit
/// gate — can refuse the run when the loss is too high.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CorpusReport {
    /// Errors raised by `walkdir` while iterating the tree
    /// (broken symlinks, `EACCES` on a directory, etc.).
    pub walk_errors: usize,
    /// Files whose extension matched but whose contents could not be
    /// decoded as UTF-8 or could not be read (`EACCES`, etc.).
    pub unreadable: usize,
    /// Files whose first `max_lines` were empty after trim, so they
    /// were dropped before embedding.
    pub skipped_empty: usize,
    /// Number of [`IngestNode`]s the walk produced.
    pub collected: usize,
}

/// Collect [`IngestNode`]s from a directory tree.
///
/// Convenience wrapper around [`collect_files_report`] for callers
/// that do not need the per-skip counters. Equivalent to
/// `collect_files_report(...).0`.
pub fn collect_files(
    repo: &str,
    root: &Path,
    include_extensions: ExtensionFilter<'_>,
    max_lines: usize,
) -> Vec<IngestNode> {
    collect_files_report(repo, root, include_extensions, max_lines).0
}

/// Collect [`IngestNode`]s from a directory tree and report skips.
///
/// Behaves like [`collect_files`] but never drops a filesystem error
/// silently: walk errors and unreadable matching files are counted
/// in the returned [`CorpusReport`] (and logged at `warn`). Empty
/// files dropped after truncation are also counted so coverage loss
/// is visible.
pub fn collect_files_report(
    repo: &str,
    root: &Path,
    include_extensions: ExtensionFilter<'_>,
    max_lines: usize,
) -> (Vec<IngestNode>, CorpusReport) {
    let mut out = Vec::new();
    let mut report = CorpusReport::default();
    for entry in walkdir::WalkDir::new(root).follow_links(false) {
        let entry = match entry {
            Ok(e) => e,
            Err(err) => {
                report.walk_errors += 1;
                tracing::warn!(error = %err, "corpus walk error");
                continue;
            }
        };
        if !entry.file_type().is_file() {
            continue;
        }
        if !has_allowed_extension(entry.path(), include_extensions) {
            continue;
        }
        let content = match std::fs::read_to_string(entry.path()) {
            Ok(c) => c,
            Err(err) => {
                report.unreadable += 1;
                tracing::warn!(
                    path = %entry.path().display(),
                    error = %err,
                    "corpus file unreadable"
                );
                continue;
            }
        };
        let truncated = take_first_lines(&content, max_lines);
        if truncated.trim().is_empty() {
            report.skipped_empty += 1;
            continue;
        }
        let Ok(rel) = entry.path().strip_prefix(root) else {
            continue;
        };
        let node_id = repo_relative_id(rel);
        out.push(IngestNode {
            repo: repo.to_string(),
            node_id,
            source: truncated,
        });
    }
    report.collected = out.len();
    (out, report)
}

fn has_allowed_extension(path: &Path, allow: ExtensionFilter<'_>) -> bool {
    if allow.is_empty() {
        return true;
    }
    let Some(ext) = path.extension().and_then(|e| e.to_str()) else {
        return false;
    };
    let lower = ext.to_ascii_lowercase();
    allow.iter().any(|a| a.eq_ignore_ascii_case(&lower))
}

fn take_first_lines(content: &str, max_lines: usize) -> String {
    if max_lines == 0 {
        return String::new();
    }
    content
        .lines()
        .take(max_lines)
        .collect::<Vec<_>>()
        .join("\n")
}

fn repo_relative_id(rel: &Path) -> String {
    let mut parts: Vec<String> = Vec::new();
    for part in rel.components() {
        if let std::path::Component::Normal(p) = part {
            parts.push(p.to_string_lossy().to_string());
        }
    }
    parts.join("/")
}

/// Common file extensions for source-code corpora used by Convergio's
/// own ingest. Callers may pass their own list to [`collect_files`].
pub const SOURCE_EXTENSIONS: ExtensionFilter<'static> =
    &["rs", "md", "sql", "toml", "ftl", "yaml", "yml"];

/// Default per-file truncation when building the corpus (ADR-0038).
pub const DEFAULT_MAX_LINES: usize = 200;

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    fn build_corpus_dir() -> tempfile::TempDir {
        let dir = tempdir().expect("tempdir");
        fs::create_dir_all(dir.path().join("src")).expect("mkdir src");
        fs::write(
            dir.path().join("src/lib.rs"),
            "//! crate doc\npub fn answer() -> u8 { 42 }\n",
        )
        .expect("write rs");
        fs::write(dir.path().join("README.md"), "# convergio\nDo the thing.\n").expect("write md");
        // Ignored extension.
        fs::write(dir.path().join("notes.txt"), "do not embed me\n").expect("write txt");
        // Empty file.
        fs::write(dir.path().join("empty.rs"), "   \n  \n").expect("write empty");
        dir
    }

    #[test]
    fn picks_up_allowed_extensions_and_skips_others() {
        let dir = build_corpus_dir();
        let nodes = collect_files("convergio", dir.path(), SOURCE_EXTENSIONS, 200);
        let ids: Vec<&str> = nodes.iter().map(|n| n.node_id.as_str()).collect();
        assert!(ids.contains(&"src/lib.rs"));
        assert!(ids.contains(&"README.md"));
        assert!(!ids.iter().any(|id| id.ends_with("notes.txt")));
        // empty.rs has only whitespace → dropped after trim.
        assert!(!ids.iter().any(|id| id.ends_with("empty.rs")));
    }

    #[test]
    fn truncates_to_max_lines() {
        let dir = tempdir().expect("tempdir");
        let path = dir.path().join("a.rs");
        let body: String = (0..500).map(|i| format!("// line {i}\n")).collect();
        fs::write(&path, &body).expect("write");
        let nodes = collect_files("convergio", dir.path(), SOURCE_EXTENSIONS, 10);
        assert_eq!(nodes.len(), 1);
        assert_eq!(nodes[0].source.lines().count(), 10);
    }

    #[test]
    fn empty_extension_filter_accepts_all_files() {
        let dir = build_corpus_dir();
        let nodes = collect_files("convergio", dir.path(), &[], 200);
        // notes.txt has content → present when filter is empty.
        assert!(nodes.iter().any(|n| n.node_id == "notes.txt"));
    }

    #[test]
    fn node_id_uses_forward_slashes() {
        let dir = build_corpus_dir();
        let nodes = collect_files("convergio", dir.path(), SOURCE_EXTENSIONS, 200);
        for n in nodes {
            assert!(
                !n.node_id.contains('\\'),
                "node_id must use '/': {}",
                n.node_id
            );
        }
    }

    /// Regression: filesystem traversal must not silently swallow
    /// errors. `collect_files_report` should surface walk failures
    /// and unreadable-file counts, otherwise corpus coverage can
    /// shrink without signal (audit finding LOW · corpus.rs:38/46).
    #[test]
    fn report_surfaces_unreadable_and_empty_skips() {
        let dir = tempdir().expect("tempdir");
        // A normal source file — embedded.
        fs::write(dir.path().join("good.rs"), "fn answer() -> u8 { 42 }\n").expect("write good.rs");
        // A file with the right extension but invalid UTF-8 —
        // `std::fs::read_to_string` will refuse it. Today this is
        // dropped on the floor; the report must count it.
        fs::write(dir.path().join("bad.rs"), [0xFFu8, 0xFE, 0xFD, 0xFC]).expect("write bad.rs");
        // A whitespace-only matching file — currently dropped after
        // trim. The report must count it as `skipped_empty`.
        fs::write(dir.path().join("empty.rs"), "   \n\n").expect("write empty.rs");

        let (nodes, report) = collect_files_report("convergio", dir.path(), SOURCE_EXTENSIONS, 200);
        assert_eq!(nodes.len(), 1, "only good.rs survives");
        assert_eq!(report.collected, 1);
        assert_eq!(
            report.unreadable, 1,
            "bad.rs is invalid UTF-8 and must be counted as unreadable, not silently dropped"
        );
        assert_eq!(
            report.skipped_empty, 1,
            "empty.rs must be counted as skipped_empty"
        );
    }

    /// Regression: when `walkdir` itself errors on an entry (e.g.
    /// dangling symlink), the count must be visible in the report
    /// (audit finding LOW · corpus.rs:38).
    #[cfg(unix)]
    #[test]
    fn report_surfaces_walk_errors_for_dangling_symlinks() {
        use std::os::unix::fs::symlink;
        let dir = tempdir().expect("tempdir");
        // Real file so the walk has something to count as collected.
        fs::write(dir.path().join("real.rs"), "fn ok() {}\n").expect("write real.rs");
        // Dangling symlink in a subdir we cannot traverse into:
        // a symlink whose target does not exist makes `walkdir`
        // raise an error for the entry itself when `follow_links`
        // is false only if we try to descend; instead create a
        // subdirectory we make unreadable so `walkdir` yields an
        // error iterating it.
        let locked = dir.path().join("locked");
        fs::create_dir(&locked).expect("mkdir locked");
        fs::write(locked.join("hidden.rs"), "fn h() {}\n").expect("write hidden.rs");
        // Strip read+exec from the directory so walkdir cannot list it.
        let mut perm = fs::metadata(&locked).expect("meta").permissions();
        std::os::unix::fs::PermissionsExt::set_mode(&mut perm, 0o000);
        fs::set_permissions(&locked, perm).expect("chmod 000");
        // Also point a symlink at a missing target — handled by walkdir.
        symlink(
            dir.path().join("does-not-exist.rs"),
            dir.path().join("link.rs"),
        )
        .expect("symlink");

        let (_nodes, report) =
            collect_files_report("convergio", dir.path(), SOURCE_EXTENSIONS, 200);

        // Restore perms so tempdir cleanup works regardless of assertions.
        let mut perm = fs::metadata(&locked).expect("meta").permissions();
        std::os::unix::fs::PermissionsExt::set_mode(&mut perm, 0o755);
        let _ = fs::set_permissions(&locked, perm);

        assert!(
            report.walk_errors >= 1,
            "expected at least one walk error from the locked subdir, got {}",
            report.walk_errors
        );
    }
}
