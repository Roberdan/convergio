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

/// Collect [`IngestNode`]s from a directory tree.
///
/// Files whose content trims to empty are dropped (not embedded).
/// Symlinks and unreadable files are skipped silently — corpus build
/// is best-effort, the orchestrator uses [`IngestReport`] to surface
/// counts.
///
/// `node_id` is the repo-relative path with forward-slash separators,
/// stable across operating systems.
///
/// [`IngestReport`]: crate::IngestReport
pub fn collect_files(
    repo: &str,
    root: &Path,
    include_extensions: ExtensionFilter<'_>,
    max_lines: usize,
) -> Vec<IngestNode> {
    let mut out = Vec::new();
    for entry in walkdir::WalkDir::new(root)
        .follow_links(false)
        .into_iter()
        .filter_map(Result::ok)
    {
        if !entry.file_type().is_file() {
            continue;
        }
        if !has_allowed_extension(entry.path(), include_extensions) {
            continue;
        }
        let Ok(content) = std::fs::read_to_string(entry.path()) else {
            continue;
        };
        let truncated = take_first_lines(&content, max_lines);
        if truncated.trim().is_empty() {
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
    out
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
}
