//! Markdown AUTO-block rewriter — split out of [`super::docs`] to
//! honour the 300-line per-file cap.
//!
//! Rules:
//! - Markers must appear at column 0 of a line outside ``` fences.
//! - `<!-- BEGIN AUTO:<name> -->` opens a block; `<!-- END AUTO -->`
//!   closes it.
//! - Inline-code (`` `...` ``) and fenced-code (``` ``` ```) example
//!   markers are ignored.

use anyhow::{anyhow, Context, Result};
use std::path::Path;

pub(super) const BEGIN: &str = "<!-- BEGIN AUTO:";
pub(super) const END: &str = "<!-- END AUTO -->";

/// Function pointer table: marker name → generator that produces the
/// fresh body. `file_path` is the markdown file currently being
/// rewritten — generators that need crate-local context (e.g.
/// `crate_stats`) walk up from there.
pub(super) trait GeneratorLookup {
    fn run(&self, name: &str, file_path: &Path, root: &Path) -> Result<String>;
}

/// Walk `input`, replace every well-formed AUTO block with the
/// generator output for its name. Returns the rewritten string.
pub(super) fn rewrite<G: GeneratorLookup>(
    input: &str,
    registry: &G,
    file_path: &Path,
    root: &Path,
) -> Result<String> {
    let live = live_ranges(input);
    let mut out = String::with_capacity(input.len());
    let mut cursor = 0;
    while let Some(rel) = next_marker(&input[cursor..], &live, cursor) {
        let abs_begin = cursor + rel;
        out.push_str(&input[cursor..abs_begin]);
        let after_marker = abs_begin + BEGIN.len();
        let header_close = input[after_marker..]
            .find(" -->")
            .ok_or_else(|| anyhow!("unterminated BEGIN AUTO marker at byte {abs_begin}"))?;
        let name = input[after_marker..after_marker + header_close].trim();
        let header_end = after_marker + header_close + " -->".len();
        let body_end = input[header_end..]
            .find(END)
            .ok_or_else(|| anyhow!("unterminated AUTO block for '{name}' at byte {abs_begin}"))?;
        let abs_body_end = header_end + body_end;
        let regenerated = registry
            .run(name, file_path, root)
            .with_context(|| format!("generator '{name}'"))?;
        out.push_str(BEGIN);
        out.push_str(name);
        out.push_str(" -->\n");
        out.push_str(&regenerated);
        if !regenerated.ends_with('\n') {
            out.push('\n');
        }
        out.push_str(END);
        cursor = abs_body_end + END.len();
    }
    out.push_str(&input[cursor..]);
    Ok(out)
}

/// Byte ranges in `input` that are NOT inside a fenced code block.
fn live_ranges(input: &str) -> Vec<(usize, usize)> {
    let mut ranges: Vec<(usize, usize)> = Vec::new();
    let mut in_fence = false;
    let mut last_live_start = 0usize;
    let mut byte_cursor = 0usize;
    for line in input.split_inclusive('\n') {
        let trimmed = line.trim_start();
        if trimmed.starts_with("```") {
            if in_fence {
                in_fence = false;
                last_live_start = byte_cursor + line.len();
            } else {
                in_fence = true;
                if byte_cursor > last_live_start {
                    ranges.push((last_live_start, byte_cursor));
                }
            }
        }
        byte_cursor += line.len();
    }
    if !in_fence && byte_cursor > last_live_start {
        ranges.push((last_live_start, byte_cursor));
    }
    ranges
}

/// Find the next BEGIN marker that starts at column 0 of a line
/// inside a live range. Returns offset relative to `slice`.
fn next_marker(slice: &str, live: &[(usize, usize)], slice_offset: usize) -> Option<usize> {
    let bytes = slice.as_bytes();
    let mut search_from = 0;
    while let Some(rel) = slice[search_from..].find(BEGIN) {
        let local_pos = search_from + rel;
        let abs = slice_offset + local_pos;
        let at_line_start = local_pos == 0 || bytes[local_pos - 1] == b'\n';
        let in_live = live.iter().any(|(start, end)| abs >= *start && abs < *end);
        if at_line_start && in_live {
            return Some(local_pos);
        }
        search_from = local_pos + BEGIN.len();
    }
    None
}

/// Remove git conflict markers (`<<<<<<<` / `=======` / `>>>>>>>`) that
/// appear inside AUTO blocks. Content between markers is discarded — the
/// rewriter regenerates it. Markers outside AUTO blocks are left untouched.
pub(super) fn strip_auto_conflicts(content: &str, marker_size: u32) -> String {
    let cs = "<".repeat(marker_size as usize);
    let sep = "=".repeat(marker_size as usize);
    let ce = ">".repeat(marker_size as usize);
    enum St {
        Normal,
        InAuto,
        AcOurs,
        AcTheirs,
    }
    let mut st = St::Normal;
    let mut out = String::with_capacity(content.len());
    for line in content.split_inclusive('\n') {
        let t = line.trim_end_matches(['\n', '\r']);
        match st {
            St::Normal => {
                if t.starts_with(BEGIN) {
                    st = St::InAuto;
                }
                out.push_str(line);
            }
            St::InAuto => {
                if t == END {
                    st = St::Normal;
                    out.push_str(line);
                } else if t.starts_with(&cs) {
                    st = St::AcOurs;
                } else {
                    out.push_str(line);
                }
            }
            St::AcOurs => {
                if t.starts_with(&sep) {
                    st = St::AcTheirs;
                }
            }
            St::AcTheirs => {
                if t.starts_with(&ce) {
                    st = St::InAuto;
                }
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Dummy;
    impl GeneratorLookup for Dummy {
        fn run(&self, _name: &str, _file: &Path, _root: &Path) -> Result<String> {
            Ok("- one\n- two\n".into())
        }
    }

    struct Strict;
    impl GeneratorLookup for Strict {
        fn run(&self, name: &str, _file: &Path, _root: &Path) -> Result<String> {
            Err(anyhow!("unknown AUTO generator '{name}'"))
        }
    }

    #[test]
    fn replaces_block() {
        let input = "intro\n<!-- BEGIN AUTO:dummy -->\nold\n<!-- END AUTO -->\noutro\n";
        let out = rewrite(input, &Dummy, Path::new("test.md"), Path::new(".")).unwrap();
        assert!(out.contains("- one") && out.contains("- two"));
        assert!(!out.contains("old"));
        assert!(out.contains("intro") && out.contains("outro"));
    }

    #[test]
    fn passes_through_files_with_no_markers() {
        let input = "no markers here\njust text\n";
        let out = rewrite(input, &Dummy, Path::new("test.md"), Path::new(".")).unwrap();
        assert_eq!(out, input);
    }

    #[test]
    fn handles_multiple_blocks() {
        let input = "<!-- BEGIN AUTO:a -->\nx\n<!-- END AUTO -->\nmid\n<!-- BEGIN AUTO:b -->\ny\n<!-- END AUTO -->\n";
        let out = rewrite(input, &Dummy, Path::new("test.md"), Path::new(".")).unwrap();
        assert_eq!(out.matches("- one").count(), 2);
    }

    #[test]
    fn skips_markers_inside_fences() {
        let input = "before\n```markdown\n<!-- BEGIN AUTO:dummy -->\nfenced\n<!-- END AUTO -->\n```\nafter\n";
        let out = rewrite(input, &Strict, Path::new("test.md"), Path::new(".")).unwrap();
        // The strict generator would error on any real call, so a
        // clean pass means no marker was parsed inside the fence.
        assert_eq!(out, input);
    }

    #[test]
    fn skips_markers_inside_inline_code() {
        // Marker mid-line inside backticks must not match (column 0 rule).
        let input = "Use `<!-- BEGIN AUTO:dummy -->` for examples.\n";
        let out = rewrite(input, &Strict, Path::new("test.md"), Path::new(".")).unwrap();
        assert_eq!(out, input);
    }

    #[test]
    fn strip_auto_conflicts_resolves_inside_keeps_outside() {
        let inside = "<!-- BEGIN AUTO:x -->\n<<<<<<< ours\n-a\n=======\n-b\n>>>>>>> theirs\n<!-- END AUTO -->\n";
        let o = strip_auto_conflicts(inside, 7);
        assert!(
            !o.contains("<<<<<<<")
                && o.contains("<!-- BEGIN AUTO:x -->")
                && o.contains("<!-- END AUTO -->")
        );
        let outside = "<<<<<<< ours\nfoo\n=======\nbar\n>>>>>>> theirs\n";
        assert!(strip_auto_conflicts(outside, 7).contains("<<<<<<<"));
    }
}
