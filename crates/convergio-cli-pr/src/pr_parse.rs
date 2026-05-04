//! Pure parser helpers for the `cvg pr` subcommands. Kept in a sibling
//! module so `pr.rs` (which carries the orchestration code) stays under
//! the 300-line Rust cap.

use std::collections::BTreeSet;

/// What we extract from a PR body.
#[derive(Debug, Clone, Default)]
pub(crate) struct ParsedManifest {
    pub files: BTreeSet<String>,
    pub depends_on: BTreeSet<i64>,
}

/// Extract the `## Files touched` block (lines inside the first
/// fenced code block under that header) and any
/// `Depends on PR #N` / `<!-- Depends on PR #N -->` declarations.
pub(crate) fn parse_manifest(body: &str) -> ParsedManifest {
    let mut files = BTreeSet::new();
    let mut depends = BTreeSet::new();

    let mut in_files_block = false;
    let mut in_files_section = false;
    for raw in body.lines() {
        let line = raw.trim_end();
        if line.starts_with("## ") {
            in_files_section = line.contains("Files touched");
            in_files_block = false;
            continue;
        }
        if in_files_section && line.trim_start().starts_with("```") {
            in_files_block = !in_files_block;
            continue;
        }
        if in_files_block {
            let path = line.trim();
            if !path.is_empty() && !path.starts_with('<') && !path.starts_with('-') {
                files.insert(path.to_string());
            }
        }
        if line.contains("Depends on PR #") {
            for (idx, _) in line.match_indices("Depends on PR #") {
                let tail = &line[idx + "Depends on PR #".len()..];
                let n: String = tail.chars().take_while(|c| c.is_ascii_digit()).collect();
                if let Ok(num) = n.parse::<i64>() {
                    depends.insert(num);
                }
            }
        }
    }
    ParsedManifest {
        files,
        depends_on: depends,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE_BODY: &str = "## Problem
something broke.

## Files touched

```
crates/convergio-cli/src/commands/pr.rs
crates/convergio-cli/src/main.rs
```

<!-- Depends on PR #11 -->
";

    #[test]
    fn parse_manifest_extracts_files_and_dependencies() {
        let m = parse_manifest(SAMPLE_BODY);
        assert!(m.files.contains("crates/convergio-cli/src/commands/pr.rs"));
        assert!(m.files.contains("crates/convergio-cli/src/main.rs"));
        assert_eq!(m.files.len(), 2);
        assert!(m.depends_on.contains(&11));
    }

    #[test]
    fn parse_manifest_handles_no_manifest_block() {
        let m = parse_manifest("## Problem\n\n## Why\n\nReasons.\n");
        assert!(m.files.is_empty());
        assert!(m.depends_on.is_empty());
    }

    #[test]
    fn parse_manifest_picks_multiple_dependencies() {
        let body = "Body.\n<!-- Depends on PR #1 -->\n<!-- Depends on PR #42 -->\n";
        let m = parse_manifest(body);
        assert!(m.depends_on.contains(&1));
        assert!(m.depends_on.contains(&42));
    }
}
