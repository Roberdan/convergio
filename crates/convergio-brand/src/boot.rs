//! Boot animation orchestrator.
//!
//! Used by `cvg about` and `convergio-server start` to render the
//! brand boot sequence: glitch → boot log → wordmark → claim. The
//! sequence collapses to a one-shot static print when [`Theme`]
//! disallows animation, so CI logs stay clean and `NO_COLOR` users
//! still see the banner.
//!
//! The orchestrator is **sink-agnostic**: callers pass a
//! [`std::io::Write`] and a [`Sleeper`]. Tests use a buffer + a
//! no-op sleeper to assert exact output without sleeping.

use std::io::{self, Write};
use std::time::Duration;

use crate::banner;
use crate::claim::CLAIM;
use crate::gradient;
use crate::theme::Theme;

/// Indirection over `std::thread::sleep` so tests can drive the
/// animation without real time passing.
pub trait Sleeper {
    /// Block (or pretend to) for `dur`.
    fn sleep(&mut self, dur: Duration);
}

/// Production sleeper — blocks the calling thread for real.
pub struct RealSleeper;

impl Sleeper for RealSleeper {
    fn sleep(&mut self, dur: Duration) {
        std::thread::sleep(dur);
    }
}

/// No-op sleeper for tests.
#[derive(Default)]
pub struct NoSleep {
    /// Total duration the orchestrator asked us to sleep for.
    pub total: Duration,
}

impl Sleeper for NoSleep {
    fn sleep(&mut self, dur: Duration) {
        self.total += dur;
    }
}

/// Render the full boot sequence to `out` using `sleeper` for any
/// inter-frame pauses. Returns the underlying `io::Result`.
pub fn play<W: Write, S: Sleeper>(out: &mut W, sleeper: &mut S, theme: Theme) -> io::Result<()> {
    if !theme.allows_animation() {
        return print_static(out, theme);
    }
    let base = "CONVERGIO";
    for f in 0..3 {
        writeln!(
            out,
            "{}{}{}",
            gradient::fg_escape(crate::palette::MAGENTA),
            crate::glitch::frame(base, f),
            gradient::RESET
        )?;
        out.flush()?;
        sleeper.sleep(Duration::from_millis(80));
    }
    for line in BOOT_LOG {
        writeln!(
            out,
            "{}{}{}",
            gradient::fg_escape(crate::palette::CYAN),
            line,
            gradient::RESET
        )?;
        out.flush()?;
        sleeper.sleep(Duration::from_millis(180));
    }
    writeln!(
        out,
        "{}> convergence achieved.{}",
        gradient::fg_escape(crate::palette::MAGENTA),
        gradient::RESET
    )?;
    writeln!(out)?;
    writeln!(out, "{}", banner::wordmark(theme))?;
    writeln!(out, "{}", gradient::brand(CLAIM, theme))?;
    out.flush()?;
    Ok(())
}

fn print_static<W: Write>(out: &mut W, theme: Theme) -> io::Result<()> {
    writeln!(out, "{}", banner::wordmark(theme))?;
    writeln!(out, "{CLAIM}")?;
    out.flush()?;
    Ok(())
}

/// The four boot-log lines. Kept inline so the animation has no
/// runtime configuration and the brand stays consistent across
/// every surface that calls it.
const BOOT_LOG: &[&str] = &[
    "> booting convergio kernel...",
    "> loading gates...",
    "> verifying audit chain...",
    "> syncing nodes...",
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mono_theme_prints_static_banner_only() {
        let mut buf = Vec::new();
        let mut s = NoSleep::default();
        play(&mut buf, &mut s, Theme::Mono).expect("write to buffer");
        let out = String::from_utf8(buf).expect("utf8");
        assert!(out.contains("CONVERGIO"));
        assert!(out.contains(CLAIM));
        assert!(!out.contains('\x1b'));
        assert_eq!(s.total, Duration::ZERO);
    }

    #[test]
    fn color_theme_emits_glitch_then_boot_log() {
        let mut buf = Vec::new();
        let mut s = NoSleep::default();
        play(&mut buf, &mut s, Theme::Color).expect("write to buffer");
        let out = String::from_utf8(buf).expect("utf8");
        assert!(out.contains("> booting convergio kernel..."));
        assert!(out.contains("> convergence achieved."));
        // CLAIM and the wordmark go through a per-character gradient,
        // so they are not contiguous substrings — strip the escapes
        // to assert the readable text.
        let plain = strip_ansi(&out);
        assert!(plain.contains("CONVERGIO"));
        assert!(plain.contains(CLAIM));
        // 3 glitch frames * 80ms + 4 boot lines * 180ms
        assert_eq!(s.total, Duration::from_millis(80 * 3 + 180 * 4));
    }

    fn strip_ansi(s: &str) -> String {
        let mut out = String::with_capacity(s.len());
        let mut in_esc = false;
        for c in s.chars() {
            if c == '\x1b' {
                in_esc = true;
                continue;
            }
            if in_esc {
                if c == 'm' {
                    in_esc = false;
                }
                continue;
            }
            out.push(c);
        }
        out
    }

    #[test]
    fn high_contrast_theme_skips_animation() {
        let mut buf = Vec::new();
        let mut s = NoSleep::default();
        play(&mut buf, &mut s, Theme::HighContrast).expect("write to buffer");
        assert_eq!(s.total, Duration::ZERO);
        let out = String::from_utf8(buf).expect("utf8");
        assert!(out.contains("CONVERGIO"));
    }
}
