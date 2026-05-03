//! ASCII banner — the wordmark and the hexagonal-C lockup that
//! ships in the brand kit, rendered for an 80-column terminal.
//!
//! Two surfaces:
//!
//! - [`wordmark`] — the `CONVERGIO` text with a magenta→cyan
//!   gradient. Used by the CLI splash and the daemon boot header.
//! - [`lockup`] — the hexagonal "C" mark stacked above the wordmark,
//!   used by `cvg about` and the start-of-session brief.
//!
//! Both honour [`Theme`] and degrade to plain ASCII when colour is
//! disabled.

use crate::claim::{CLAIM, SUBLINE};
use crate::gradient;
use crate::theme::Theme;

/// Render the bare wordmark `CONVERGIO` with the brand gradient.
pub fn wordmark(theme: Theme) -> String {
    gradient::brand("CONVERGIO", theme)
}

/// Render the full lockup: hexagonal "C" mark, wordmark, claim,
/// subline. Suitable as the body of `cvg about` or the daemon boot
/// banner. Lines are ASCII-only when [`Theme::Mono`].
pub fn lockup(theme: Theme) -> String {
    let mark = hexagonal_c(theme);
    let wm = wordmark(theme);
    let claim = format_claim(theme);
    let subline = format_subline(theme);
    let mut out = String::with_capacity(mark.len() + wm.len() + 200);
    out.push_str(&mark);
    out.push('\n');
    out.push_str(&format!("        {wm}\n"));
    out.push('\n');
    out.push_str(&format!("        {claim}\n"));
    out.push_str(&format!("        {subline}\n"));
    out
}

fn format_claim(theme: Theme) -> String {
    if theme.allows_color() {
        gradient::brand(CLAIM, theme)
    } else {
        CLAIM.to_string()
    }
}

fn format_subline(theme: Theme) -> String {
    // Subline always renders in plain text — the gradient on a long
    // sentence becomes hard to read, and we want the subline to be
    // skimmable.
    if theme.allows_color() {
        format!("\x1b[2m{SUBLINE}\x1b[0m")
    } else {
        SUBLINE.to_string()
    }
}

/// Six-line hexagonal "C" mark. Sized to sit comfortably above the
/// wordmark on a 24-row terminal. The hex is drawn with box-drawing
/// glyphs and tinted with the brand gradient when colour is on.
fn hexagonal_c(theme: Theme) -> String {
    const ROWS: [&str; 6] = [
        "        ▄▄████████▄▄",
        "      ▄██▀        ▀██▄",
        "    ▐█▌    ▄▄▄▄      ▐█▌",
        "    ▐█▌    ▀▀▀▀      ▐█▌",
        "      ▀██▄        ▄██▀",
        "        ▀▀████████▀▀",
    ];
    if !theme.allows_color() {
        return ROWS.join("\n");
    }
    ROWS.iter()
        .enumerate()
        .map(|(i, row)| {
            let t = i as f32 / (ROWS.len() - 1) as f32;
            let rgb = crate::palette::Rgb::lerp(crate::palette::MAGENTA, crate::palette::CYAN, t);
            format!("{}{}{}", gradient::fg_escape(rgb), row, gradient::RESET)
        })
        .collect::<Vec<_>>()
        .join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wordmark_mono_is_plain_text() {
        assert_eq!(wordmark(Theme::Mono), "CONVERGIO");
    }

    #[test]
    fn lockup_contains_claim_and_subline() {
        let s = lockup(Theme::Mono);
        assert!(s.contains(CLAIM));
        assert!(s.contains(SUBLINE));
    }

    #[test]
    fn lockup_color_emits_escapes() {
        let s = lockup(Theme::Color);
        assert!(s.contains("\x1b["));
        // CLAIM is rendered through the gradient one char at a time,
        // so it does not appear as a contiguous substring. Stripping
        // escapes yields the readable text.
        assert!(strip_ansi(&s).contains(CLAIM));
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
    fn lockup_mono_has_no_escape_sequences() {
        let s = lockup(Theme::Mono);
        assert!(!s.contains('\x1b'), "mono lockup leaked an escape sequence");
    }

    #[test]
    fn lockup_has_six_hex_rows_plus_wordmark_and_claim() {
        let s = lockup(Theme::Mono);
        // 6 hex rows + 1 blank + 1 wordmark + 1 blank + 1 claim + 1
        // subline = 11 lines, possibly with a trailing newline.
        assert!(s.lines().count() >= 10);
    }
}
