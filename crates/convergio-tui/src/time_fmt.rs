//! Local-timezone time formatting helpers.
//!
//! The daemon stores every timestamp as RFC3339 UTC. The TUI is a
//! human-facing surface: it must render those instants in the
//! operator's local time, so a screenshot at 18:01 CEST does not
//! read `16:01`. This module is the single conversion seam.

use chrono::{DateTime, Local, TimeZone, Utc};

/// `HH:MM:SS` in the operator's local timezone.
pub fn clock_utc(dt: DateTime<Utc>) -> String {
    dt.with_timezone(&Local).format("%H:%M:%S").to_string()
}

/// Parse RFC3339 UTC and render as `HH:MM:SS` in local time.
/// Falls back to the raw 8-char prefix when the input is not parseable
/// (rare — keeps the pane from going blank on a daemon protocol drift).
pub fn parse_clock_local(raw: &str) -> String {
    parse_clock_in(raw, Local)
}

/// Parse RFC3339 UTC and render as `YYYY-MM-DD HH:MM` in local time.
pub fn parse_short_local(raw: &str) -> String {
    match DateTime::parse_from_rfc3339(raw) {
        Ok(dt) => dt
            .with_timezone(&Local)
            .format("%Y-%m-%d %H:%M")
            .to_string(),
        Err(_) => raw.get(..16).unwrap_or(raw).replace('T', " "),
    }
}

/// Timezone-parametric variant used by the tests so they don't depend
/// on the host clock.
pub fn parse_clock_in<Tz: TimeZone>(raw: &str, tz: Tz) -> String
where
    Tz::Offset: std::fmt::Display,
{
    match DateTime::parse_from_rfc3339(raw) {
        Ok(dt) => dt.with_timezone(&tz).format("%H:%M:%S").to_string(),
        Err(_) => raw.get(11..19).unwrap_or(raw).to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{FixedOffset, TimeZone};

    #[test]
    fn rfc3339_utc_converted_to_cest() {
        let cest = FixedOffset::east_opt(2 * 3600).unwrap();
        assert_eq!(parse_clock_in("2026-05-11T16:01:30Z", cest), "18:01:30");
    }

    #[test]
    fn rfc3339_utc_converted_to_pst() {
        let pst = FixedOffset::west_opt(8 * 3600).unwrap();
        assert_eq!(parse_clock_in("2026-05-11T16:01:30Z", pst), "08:01:30");
    }

    #[test]
    fn unparseable_input_falls_back_to_substring() {
        let tz = FixedOffset::east_opt(0).unwrap();
        // The RFC3339 substring slice gives a deterministic shape when
        // the input is a near-RFC3339 string; total garbage returns the
        // raw input so the pane never goes blank.
        assert_eq!(parse_clock_in("2026-05-11T08:09:10Z", tz), "08:09:10");
        assert_eq!(parse_clock_in("garbage", tz), "garbage");
    }

    #[test]
    fn short_local_drops_t_separator() {
        let dt = FixedOffset::east_opt(0)
            .unwrap()
            .with_ymd_and_hms(2026, 5, 11, 16, 1, 30)
            .unwrap()
            .with_timezone(&Utc);
        // Round-trip through the public function — guarantees no T in output.
        let s = parse_short_local(&dt.to_rfc3339());
        assert!(!s.contains('T'), "got {s:?}");
        assert_eq!(s.len(), 16);
    }

    #[test]
    fn clock_utc_renders_eight_chars() {
        let s = clock_utc(Utc.with_ymd_and_hms(2026, 5, 11, 16, 1, 30).unwrap());
        assert_eq!(s.len(), 8, "got {s:?}");
    }
}
