//! Pure parser helpers for `cvg pr sync`. Kept in a sibling module so
//! `pr_sync.rs` (which contains async I/O) stays under the 300-line
//! Rust cap.

const TRACKS_PREFIX: &str = "Tracks:";

/// Extract every UUID from any `Tracks:` line in the PR body. The line
/// form is `Tracks: <uuid>[, <uuid>]...`. UUIDs are validated by shape
/// (8-4-4-4-12 hex with dashes) so that arbitrary text after `Tracks:`
/// does not produce spurious task ids.
pub(crate) fn parse_tracks_lines(body: &str) -> Vec<String> {
    let mut out = Vec::new();
    for raw in body.lines() {
        let line = raw.trim();
        let Some(rest) = line.strip_prefix(TRACKS_PREFIX) else {
            continue;
        };
        for token in rest.split(|c: char| c == ',' || c.is_whitespace()) {
            let t = token.trim();
            if is_valid_uuid(t) {
                out.push(t.to_string());
            }
        }
    }
    out
}

pub(crate) fn is_valid_uuid(s: &str) -> bool {
    if s.len() != 36 {
        return false;
    }
    s.chars().enumerate().all(|(i, c)| {
        if matches!(i, 8 | 13 | 18 | 23) {
            c == '-'
        } else {
            c.is_ascii_hexdigit()
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const UUID_A: &str = "7e33309f-1457-4c8e-9eae-dba599a4a452";
    const UUID_B: &str = "7ec3fc92-e6b7-4cc5-96b6-659a572160be";

    #[test]
    fn parse_tracks_extracts_single_uuid() {
        let body = format!("## Summary\n\nbody.\n\nTracks: {UUID_A}\n");
        let ids = parse_tracks_lines(&body);
        assert_eq!(ids, vec![UUID_A.to_string()]);
    }

    #[test]
    fn parse_tracks_extracts_multiple_lines() {
        let body = format!("Tracks: {UUID_A}\nTracks: {UUID_B}\n");
        let ids = parse_tracks_lines(&body);
        assert_eq!(ids, vec![UUID_A.to_string(), UUID_B.to_string()]);
    }

    #[test]
    fn parse_tracks_extracts_comma_separated() {
        let body = format!("Tracks: {UUID_A}, {UUID_B}\n");
        let ids = parse_tracks_lines(&body);
        assert_eq!(ids, vec![UUID_A.to_string(), UUID_B.to_string()]);
    }

    #[test]
    fn parse_tracks_rejects_non_uuid_garbage() {
        let body = "Tracks: not-a-uuid 12345 short-string\n";
        assert!(parse_tracks_lines(body).is_empty());
    }

    #[test]
    fn parse_tracks_returns_empty_on_no_tracks_line() {
        let body = "## Summary\n\nNothing tracked here.\n## Files touched\n";
        assert!(parse_tracks_lines(body).is_empty());
    }

    #[test]
    fn parse_tracks_ignores_inline_prose() {
        let body = format!("Some context. Tracks: {UUID_A}\n");
        assert!(parse_tracks_lines(&body).is_empty());
    }

    #[test]
    fn is_valid_uuid_accepts_v4_shape() {
        assert!(is_valid_uuid(UUID_A));
    }

    #[test]
    fn is_valid_uuid_rejects_too_short() {
        assert!(!is_valid_uuid("7e33309f-1457"));
    }

    #[test]
    fn is_valid_uuid_rejects_missing_dashes() {
        assert!(!is_valid_uuid("7e33309f1457bcde7e33309f1457bcde0000"));
    }

    #[test]
    fn is_valid_uuid_rejects_non_hex_chars() {
        assert!(!is_valid_uuid("zzzzzzzz-1457-4c8e-9eae-dba599a4a452"));
    }
}
