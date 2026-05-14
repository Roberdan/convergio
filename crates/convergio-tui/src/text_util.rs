//! Small text utilities shared across panes.
//!
//! Extracted from the per-pane `fn short` / `fn truncate` copies
//! that drifted into five pane modules. Centralising them here
//! keeps the unicode-safety invariant in one place and saves the
//! "are these five copies actually identical?" reading tax.

/// Truncate `s` to at most `max` bytes, snapping back to the nearest
/// preceding char boundary so a multi-byte codepoint is never split.
/// Returns the original slice when it already fits.
pub fn truncate(s: &str, max: usize) -> &str {
    if s.len() <= max {
        return s;
    }
    let mut end = max;
    while !s.is_char_boundary(end) && end > 0 {
        end -= 1;
    }
    &s[..end]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn truncate_passthrough_when_under_max() {
        assert_eq!(truncate("abc", 5), "abc");
        assert_eq!(truncate("abc", 3), "abc");
    }

    #[test]
    fn truncate_snaps_to_char_boundary() {
        // 'è' is 2 bytes in UTF-8. Cutting at byte 4 lands inside the
        // codepoint, so the helper must back up to byte 3.
        let s = "abcèfgh";
        assert_eq!(truncate(s, 4), "abc");
    }

    #[test]
    fn truncate_zero_max_returns_empty() {
        assert_eq!(truncate("abc", 0), "");
        assert_eq!(truncate("", 0), "");
    }
}
