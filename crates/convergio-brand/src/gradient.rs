//! Truecolor gradient helpers.
//!
//! The brand wordmark fades from [`MAGENTA`](crate::MAGENTA) on the
//! left to [`CYAN`](crate::CYAN) on the right. This module renders
//! that gradient as a foreground-coloured string using ANSI
//! `\x1b[38;2;R;G;Bm` escapes — no terminfo lookup, no extra
//! dependency.

use crate::palette::{Rgb, CYAN, MAGENTA};
use crate::theme::Theme;

/// ANSI reset escape — clears foreground colour back to the
/// terminal default.
pub const RESET: &str = "\x1b[0m";

/// Format a single foreground escape for `rgb`.
pub fn fg_escape(rgb: Rgb) -> String {
    format!("\x1b[38;2;{};{};{}m", rgb.r, rgb.g, rgb.b)
}

/// Render `text` with a left-to-right gradient between `start` and
/// `end`. When `theme` does not allow colour, returns `text`
/// unchanged so non-TTY consumers see plain ASCII.
pub fn render(text: &str, start: Rgb, end: Rgb, theme: Theme) -> String {
    if !theme.allows_color() {
        return text.to_string();
    }
    let chars: Vec<char> = text.chars().collect();
    let len = chars.len();
    if len == 0 {
        return String::new();
    }
    let mut out = String::with_capacity(text.len() + len * 19 + RESET.len());
    for (i, c) in chars.into_iter().enumerate() {
        let t = if len == 1 {
            0.0
        } else {
            i as f32 / (len - 1) as f32
        };
        let rgb = Rgb::lerp(start, end, t);
        out.push_str(&fg_escape(rgb));
        out.push(c);
    }
    out.push_str(RESET);
    out
}

/// Convenience: brand magenta → cyan gradient.
pub fn brand(text: &str, theme: Theme) -> String {
    render(text, MAGENTA, CYAN, theme)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mono_theme_returns_plain_text() {
        assert_eq!(brand("CONVERGIO", Theme::Mono), "CONVERGIO");
    }

    #[test]
    fn high_contrast_returns_plain_text() {
        assert_eq!(brand("CONVERGIO", Theme::HighContrast), "CONVERGIO");
    }

    #[test]
    fn color_theme_emits_truecolor_escapes() {
        let s = brand("AB", Theme::Color);
        assert!(s.contains("\x1b[38;2;255;0;180mA"));
        assert!(s.contains('B'));
        assert!(s.ends_with(RESET));
    }

    #[test]
    fn empty_input_is_empty_output() {
        assert_eq!(brand("", Theme::Color), "");
    }

    #[test]
    fn single_char_does_not_divide_by_zero() {
        let s = brand("X", Theme::Color);
        assert!(s.contains('X'));
        assert!(s.ends_with(RESET));
    }

    #[test]
    fn fg_escape_format_is_stable() {
        assert_eq!(fg_escape(Rgb::new(1, 2, 3)), "\x1b[38;2;1;2;3m");
    }
}
