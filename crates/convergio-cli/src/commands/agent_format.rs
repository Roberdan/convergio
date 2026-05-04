//! Formatting helpers for `cvg agent list` / `show` / `retire-stale`.
//!
//! Pure functions — no I/O, no allocations beyond what the returned
//! `String` requires. Kept in its own module so the relative-time
//! formatter is unit-testable without standing up the full CLI
//! plumbing.

use chrono::{DateTime, Utc};

/// Format `ts` as a short relative phrase suitable for a TTY column.
///
/// Returns at most 6 characters: `30s`, `2m`, `1h`, `5d`. Each
/// unit caps at the next boundary (60s → `1m`, 60m → `1h`, 24h →
/// `1d`). Future timestamps clamp to `0s`.
pub fn relative(ts: &DateTime<Utc>, now: &DateTime<Utc>) -> String {
    let delta = now.signed_duration_since(*ts);
    let secs = delta.num_seconds();
    if secs <= 0 {
        return "0s".into();
    }
    if secs < 60 {
        return format!("{secs}s");
    }
    let mins = secs / 60;
    if mins < 60 {
        return format!("{mins}m");
    }
    let hours = mins / 60;
    if hours < 24 {
        return format!("{hours}h");
    }
    let days = hours / 24;
    format!("{days}d")
}

/// `relative(ts, now()) + " ago"` with `-` for `None`.
pub fn relative_ago_opt(ts: Option<&DateTime<Utc>>, now: &DateTime<Utc>) -> String {
    match ts {
        Some(t) => format!("{} ago", relative(t, now)),
        None => "-".into(),
    }
}

/// Truncate a string to `max` chars (Unicode-aware), appending an
/// ellipsis when truncated. `max` must be ≥ 1.
pub fn truncate(input: &str, max: usize) -> String {
    if max == 0 {
        return String::new();
    }
    let count = input.chars().count();
    if count <= max {
        return input.to_string();
    }
    let take = max.saturating_sub(1);
    let mut out: String = input.chars().take(take).collect();
    out.push('…');
    out
}

/// Colorize a status token for the human view. Uses ANSI codes
/// directly — keeps the helper dependency-free. JSON / plain
/// callers must skip this entirely.
pub fn color_status(status: &str) -> String {
    let code = match status {
        "working" => "32",               // green
        "idle" => "36",                  // cyan
        "unhealthy" => "33",             // yellow
        "terminated" | "retired" => "2", // dim
        _ => "0",
    };
    format!("\x1b[{code}m{status}\x1b[0m")
}

/// Indent + dim subagent rows in the human list (visual cue that
/// they are managed children of an orchestrator).
pub fn maybe_indent_id(id: &str, kind: &str) -> String {
    if kind == "subagent" {
        format!("  \x1b[2m{id}\x1b[0m")
    } else {
        id.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    fn at(secs_ago: i64) -> DateTime<Utc> {
        let now: DateTime<Utc> = Utc.with_ymd_and_hms(2026, 5, 4, 12, 0, 0).unwrap();
        now - chrono::Duration::seconds(secs_ago)
    }

    fn now_fixed() -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 5, 4, 12, 0, 0).unwrap()
    }

    #[test]
    fn relative_formats_each_bucket() {
        assert_eq!(relative(&at(30), &now_fixed()), "30s");
        assert_eq!(relative(&at(120), &now_fixed()), "2m");
        assert_eq!(relative(&at(3600), &now_fixed()), "1h");
        assert_eq!(relative(&at(60 * 60 * 24 * 5), &now_fixed()), "5d");
    }

    #[test]
    fn relative_clamps_future_to_zero() {
        let future = now_fixed() + chrono::Duration::seconds(99);
        assert_eq!(relative(&future, &now_fixed()), "0s");
    }

    #[test]
    fn relative_ago_opt_handles_none() {
        assert_eq!(relative_ago_opt(None, &now_fixed()), "-");
    }

    #[test]
    fn truncate_appends_ellipsis_only_when_needed() {
        assert_eq!(truncate("short", 10), "short");
        assert_eq!(truncate("abcdef", 4), "abc…");
        assert_eq!(truncate("", 4), "");
    }

    #[test]
    fn truncate_handles_unicode() {
        // 5 graphemes, ascii-counted differently.
        assert_eq!(truncate("café-bar", 5), "café…");
    }

    #[test]
    fn color_status_is_idempotent_in_shape() {
        let out = color_status("working");
        assert!(out.contains("working"));
        assert!(out.starts_with("\x1b["));
        assert!(out.ends_with("\x1b[0m"));
    }
}
