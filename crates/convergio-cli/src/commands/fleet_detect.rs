//! Fleet repo local metadata detection.

use std::path::Path;

pub(crate) fn read_derives_from(repo_root: &Path) -> Option<String> {
    let content = std::fs::read_to_string(repo_root.join("convergio.yaml")).ok()?;
    for line in content.lines() {
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix("derives_from:") {
            let value = rest.trim().trim_matches('"').trim_matches('\'').to_owned();
            if !value.is_empty() {
                return Some(value);
            }
        }
    }
    None
}

pub(crate) fn detect_language(repo_root: &Path) -> String {
    if repo_root.join("Cargo.toml").exists() {
        "rust".to_owned()
    } else if repo_root.join("package.json").exists() {
        "typescript".to_owned()
    } else if repo_root.join("pyproject.toml").exists() || repo_root.join("setup.py").exists() {
        "python".to_owned()
    } else {
        "unknown".to_owned()
    }
}

pub(crate) fn default_parser(language: &str) -> String {
    if language == "rust" {
        "syn".to_owned()
    } else {
        "tree-sitter".to_owned()
    }
}
