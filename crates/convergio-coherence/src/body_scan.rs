//! Line-level scanners for [`crate::body`].
//!
//! Pure helpers (no I/O) that turn a markdown line into a list of
//! candidate identifiers / paths. Split out to honour the 300-line cap.

/// Iterate over the lines of `body` that are NOT inside a fenced
/// code block (``` … ```).
pub(super) fn live_lines(body: &str) -> impl Iterator<Item = &str> {
    let mut in_fence = false;
    body.lines().filter(move |line| {
        if line.trim_start().starts_with("```") {
            in_fence = !in_fence;
            return false;
        }
        !in_fence
    })
}

/// Find `convergio-foo` style identifiers in `line`. Strips inline
/// backticks so `` `convergio-cli` `` matches.
///
/// Skips identifiers that look like a directory component: anything
/// preceded by `/` or followed by `/`. These are paths the user
/// invents (e.g. `~/convergio-worktrees/<branch>/`), not Cargo
/// crate names.
pub(super) fn find_crate_idents(line: &str) -> Vec<String> {
    let stripped: String = strip_inline_code(line);
    let bytes = stripped.as_bytes();
    let mut out: Vec<String> = Vec::new();
    let needle = b"convergio-";
    let mut i = 0;
    while i + needle.len() <= bytes.len() {
        if &bytes[i..i + needle.len()] == needle {
            if i > 0 {
                let prev = bytes[i - 1];
                if prev.is_ascii_alphanumeric() || prev == b'-' || prev == b'_' || prev == b'/' {
                    i += 1;
                    continue;
                }
            }
            let mut j = i + needle.len();
            while j < bytes.len() {
                let c = bytes[j];
                if c.is_ascii_lowercase() || c.is_ascii_digit() || c == b'-' {
                    j += 1;
                } else {
                    break;
                }
            }
            if j < bytes.len() && bytes[j] == b'/' {
                i = j;
                continue;
            }
            let end = if bytes[j - 1] == b'-' { j - 1 } else { j };
            if end > i + needle.len() {
                let s = &stripped[i..end];
                out.push(s.to_string());
            }
            i = j;
        } else {
            i += 1;
        }
    }
    out
}

/// Find repo-relative path references. We anchor only on well-known
/// top-level dirs from the repo root. `tests/` is intentionally
/// excluded because crate-local AGENTS.md files use it relative to
/// the crate, not the repo.
pub(super) fn find_repo_paths(line: &str) -> Vec<String> {
    let stripped: String = strip_inline_code(line);
    let prefixes = ["crates/", "docs/", "scripts/", "examples/"];
    let bytes = stripped.as_bytes();
    let mut out: Vec<String> = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        let mut matched_prefix: Option<&str> = None;
        for p in &prefixes {
            if bytes.len() - i >= p.len() && &bytes[i..i + p.len()] == p.as_bytes() {
                if i > 0 {
                    let prev = bytes[i - 1];
                    if prev.is_ascii_alphanumeric() || prev == b'/' || prev == b'_' {
                        continue;
                    }
                }
                matched_prefix = Some(p);
                break;
            }
        }
        if matched_prefix.is_some() {
            let mut j = i;
            while j < bytes.len() {
                let c = bytes[j];
                // `*` and `?` are kept so glob patterns like
                // `tests/e2e_*.rs` survive intact and the
                // glob-skipping check downstream can recognise them.
                if c.is_ascii_alphanumeric() || matches!(c, b'/' | b'.' | b'_' | b'-' | b'*' | b'?')
                {
                    j += 1;
                } else {
                    break;
                }
            }
            let s = &stripped[i..j];
            if s.contains('/') && !s.ends_with('/') {
                out.push(s.to_string());
            }
            i = j;
        } else {
            i += 1;
        }
    }
    out
}

/// Strip inline-code backticks from a line so the text inside is
/// still scanned (we want to validate references even when wrapped
/// in code formatting), but the backticks themselves do not break
/// boundary checks.
fn strip_inline_code(line: &str) -> String {
    line.replace('`', "")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ignores_substring_matches() {
        let idents = find_crate_idents("aconvergio-foo bconvergio-bar");
        assert!(idents.is_empty());
    }

    #[test]
    fn ignores_directory_components() {
        // `~/convergio-worktrees/<branch>/` is a directory the user
        // invents, not a crate name.
        let idents = find_crate_idents("see ~/convergio-worktrees/main/ for help");
        assert!(idents.is_empty());
    }

    #[test]
    fn finds_paths_with_globs() {
        let paths = find_repo_paths("see crates/foo/tests/e2e_*.rs");
        assert!(paths.iter().any(|p| p.contains('*')));
    }
}
