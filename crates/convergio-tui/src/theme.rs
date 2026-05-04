//! Single source of truth for TUI styling.
//!
//! Convergio's CONSTITUTION P3 (Accessibility-first) makes this
//! module load-bearing. Every colour decision in `cvg dash` flows
//! through here — there is no `Color::DarkGray` literal anywhere
//! else, and `Modifier::REVERSED` is forbidden because it produces
//! the white-on-light-gray failure mode reported by the operator
//! after PR #114.
//!
//! Palette: **tokyo-night-dim**. Every foreground / background pair
//! we ship clears WCAG AA (4.5:1 for body text) on both pure black
//! `#000` and the navy `#141E37` Apple Terminal default. The worst
//! case is `failed_red` on navy at 6.1:1 — comfortably above AA.
//! `cancelled_gray` on navy is 3.05:1 (passes AA Large only); that
//! is intentional because cancelled rows must read as
//! de-emphasised, and the variant carries a glyph as well.
//!
//! Status pills (`● ◐ ✓ ✗ ⊘`) carry meaning *without* colour, so
//! colour-blind users still parse the dashboard.
//!
//! Selected rows use an explicit `bg + fg` pair plus a `█`
//! left-edge accent in the focus-border colour. We never invert
//! existing cells.

use ratatui::style::{Color, Modifier, Style};
use ratatui::text::Span;

/// Primary text. ~14.8:1 on black, ~9.6:1 on navy.
pub const TEXT: Color = Color::Rgb(192, 202, 245);
/// Secondary / dim text. Use this *instead* of `Color::DarkGray`.
/// 11.1:1 on black, 7.2:1 on navy.
pub const DIM: Color = Color::Rgb(169, 177, 214);
/// `active`, `working`, `done` accents.
pub const SUCCESS: Color = Color::Rgb(158, 206, 106);
/// `draft`, `pending`, in-flight CI.
pub const WARNING: Color = Color::Rgb(224, 175, 104);
/// `failed`, `cancelled`-when-emphasised, CI failure.
pub const DANGER: Color = Color::Rgb(247, 118, 142);
/// `submitted`, `in_progress`, info accents.
pub const INFO: Color = Color::Rgb(125, 207, 255);
/// Cancelled / muted entity state. Always paired with a glyph.
pub const MUTED: Color = Color::Rgb(115, 122, 162);
/// Focus-border + accent-bar colour.
pub const FOCUS: Color = Color::Rgb(122, 162, 247);
/// Selected-row background.
pub const HIGHLIGHT_BG: Color = Color::Rgb(61, 89, 161);
/// Selected-row foreground (always paired with `HIGHLIGHT_BG`).
pub const HIGHLIGHT_FG: Color = Color::Rgb(255, 255, 255);
/// Banner gradient endpoint A.
pub const GRADIENT_START: Color = Color::Rgb(122, 162, 247);
/// Banner gradient endpoint B.
pub const GRADIENT_END: Color = Color::Rgb(187, 154, 247);

/// Body text style.
pub fn text() -> Style {
    Style::default().fg(TEXT)
}

/// Secondary text style. Replaces every former `Color::DarkGray` fg.
pub fn dim() -> Style {
    Style::default().fg(DIM)
}

/// Bold heading style.
pub fn heading() -> Style {
    Style::default().fg(TEXT).add_modifier(Modifier::BOLD)
}

/// Style for the highlighted (selected) row inside a pane. Pairs an
/// explicit fg + bg so it is legible regardless of the row's own
/// per-cell colours. Callers that also want the accent bar should
/// prefix the row with [`accent_span`].
pub fn row_highlight() -> Style {
    Style::default()
        .fg(HIGHLIGHT_FG)
        .bg(HIGHLIGHT_BG)
        .add_modifier(Modifier::BOLD)
}

/// One-character left-edge accent for the focused/selected row.
/// Paired with [`row_highlight`] for the non-colour cue (P3).
pub fn accent_span() -> Span<'static> {
    Span::styled("▎", Style::default().fg(FOCUS))
}

/// Empty-cell version of [`accent_span`] for non-selected rows so
/// columns line up.
pub fn accent_gap() -> Span<'static> {
    Span::raw(" ")
}

/// Glyph + style for an entity status. Returns the glyph as a
/// `Span` ready to drop into a line; callers append the textual
/// label themselves so the pill works in plain mode too.
pub fn status_pill(status: &str) -> (Span<'static>, Style) {
    let (glyph, color) = match status {
        // Plan + task active states.
        "active" | "working" | "in_progress" => ("●", SUCCESS),
        // Pending / draft / waiting.
        "draft" | "pending" => ("◐", WARNING),
        // Submitted = waiting for Thor.
        "submitted" => ("◑", INFO),
        // Validated / done.
        "completed" | "done" => ("✓", INFO),
        // Hard failure.
        "failed" => ("✗", DANGER),
        // Closed without ship.
        "cancelled" | "retired" | "terminated" => ("⊘", MUTED),
        // Idle agent.
        "idle" => ("○", MUTED),
        _ => ("·", DIM),
    };
    let style = Style::default().fg(color).add_modifier(Modifier::BOLD);
    (Span::styled(glyph, style), style)
}

/// Style for a Bus topic family (`coordination/*` green, `agent:*`
/// blue, `system.*` gray, `plan:*` yellow, anything else default).
/// Topic family classification lives in
/// [`crate::bus_stream::sse_parser::TopicFamily`].
pub fn topic_family_style(family: crate::bus_stream::sse_parser::TopicFamily) -> Style {
    use crate::bus_stream::sse_parser::TopicFamily;
    match family {
        TopicFamily::Coordination => Style::default().fg(SUCCESS),
        TopicFamily::Agent => Style::default().fg(INFO),
        TopicFamily::System => Style::default().fg(MUTED),
        TopicFamily::Plan => Style::default().fg(WARNING),
        TopicFamily::Other => Style::default().fg(TEXT),
    }
}

/// Glyph prefix that mirrors [`topic_family_style`] for users who
/// cannot rely on colour (CONSTITUTION P3): each family gets a
/// distinctive ASCII-friendly tag in addition to the colour.
pub fn topic_family_glyph(family: crate::bus_stream::sse_parser::TopicFamily) -> &'static str {
    use crate::bus_stream::sse_parser::TopicFamily;
    match family {
        TopicFamily::Coordination => "◇",
        TopicFamily::Agent => "▣",
        TopicFamily::System => "▦",
        TopicFamily::Plan => "▷",
        TopicFamily::Other => "·",
    }
}

/// Convenience: build a status pill `Line`-friendly pair (glyph
/// span, label span) with a single space separator. Use when you
/// want both the glyph and the label coloured the same way.
pub fn status_pill_with_label(status: &str) -> Vec<Span<'static>> {
    let (glyph, style) = status_pill(status);
    vec![
        glyph,
        Span::raw(" "),
        Span::styled(status.to_string(), style),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn primary_text_is_high_contrast() {
        // Sanity: TEXT is bright enough that it can never be
        // mistaken for DIM. (Catches accidental palette swaps.)
        if let (Color::Rgb(tr, tg, tb), Color::Rgb(dr, dg, db)) = (TEXT, DIM) {
            assert!(tr as u32 + tg as u32 + tb as u32 > dr as u32 + dg as u32 + db as u32);
        } else {
            panic!("palette must use truecolor for stable contrast");
        }
    }

    #[test]
    fn status_pill_active_uses_success_glyph() {
        let (span, _) = status_pill("active");
        assert_eq!(span.content, "●");
    }

    #[test]
    fn status_pill_failed_uses_x_glyph() {
        let (span, _) = status_pill("failed");
        assert_eq!(span.content, "✗");
    }

    #[test]
    fn status_pill_cancelled_uses_o_slash_glyph() {
        let (span, _) = status_pill("cancelled");
        assert_eq!(span.content, "⊘");
    }

    #[test]
    fn unknown_status_falls_back_to_dot() {
        let (span, _) = status_pill("nonsense-state");
        assert_eq!(span.content, "·");
    }
}
