//! Big pixel-block CONVERGIO header — 4 rows, custom solid-block
//! font, sourced from `convergio_brand::wordmark_big`.
//!
//! This is the "wide" tier of the dashboard header. The font is
//! intentionally chunky (`█▀▄` only, no thin line-art) to match
//! the brand kit's `wordmark-pixel.png` density. Narrow shells
//! fall back to the existing 2-row half-block banner; smaller still
//! to a single styled line.

use convergio_brand::wordmark_big;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

/// Height the big banner reserves: 4 wordmark rows + 1 spacer/stats.
pub const HEIGHT: u16 = 5;

/// Width budget the big tier expects: wordmark + 2-col gutter +
/// 16-col stats column. When the available width is below this
/// the caller falls back to the compact tiers. With the new
/// 67-col wordmark, big tier kicks in at 85 cols — most modern
/// terminals (≥ 100 cols) get it by default.
pub const MIN_WIDTH: u16 = wordmark_big::WIDTH + 2 + 16;

/// Render the big banner into `area`. Wordmark on the left,
/// right-aligned stats column on the right (htop / k9s convention).
///
/// `phase` is a `[0.0, 1.0)` cursor that shifts the gradient origin
/// across the wordmark: pass a fresh value per frame to make the
/// brand colours "breathe" (left-to-right neon sweep). Pass `0.0`
/// for a static render — all snapshot tests do this.
pub fn render(f: &mut Frame, area: Rect, stats: &[String], phase: f32) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Length(wordmark_big::WIDTH), Constraint::Min(0)])
        .split(area);
    f.render_widget(Paragraph::new(banner_lines(area.height, phase)), chunks[0]);
    f.render_widget(stats_paragraph(stats, chunks[1].height), chunks[1]);
}

/// Wall-clock-driven phase for the gradient sweep. Cycles every
/// ~6 seconds so the breathing is unmissable but not nauseating.
pub fn phase_now() -> f32 {
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs_f32())
        .unwrap_or(0.0);
    let cycle = 6.0_f32;
    (secs % cycle) / cycle
}

fn banner_lines(available_rows: u16, phase: f32) -> Vec<Line<'static>> {
    let mut lines = Vec::with_capacity(wordmark_big::ROWS.len() + 1);
    for (row_idx, row) in wordmark_big::ROWS.iter().enumerate() {
        lines.push(line_with_gradient(row, row_idx, phase));
    }
    while lines.len() < available_rows as usize {
        lines.push(Line::raw(""));
    }
    lines
}

fn line_with_gradient(row: &str, row_idx: usize, phase: f32) -> Line<'static> {
    // Per-character horizontal gradient. The vertical row index
    // shifts the gradient slightly so the banner has a soft sheen
    // top-to-bottom in addition to the L→R brand gradient. `phase`
    // walks the gradient origin so the colour breathes across the
    // wordmark over time.
    let total_cols = wordmark_big::WIDTH as usize;
    let v_shift = row_idx as f32 / (wordmark_big::ROWS.len().saturating_sub(1).max(1)) as f32;
    // Even rows full brightness, odd rows ~85% — gives a subtle
    // LED-matrix scanline effect that matches the raster look of
    // the brand kit's `wordmark-pixel.png`.
    let scan = if row_idx % 2 == 0 { 1.0 } else { 0.85 };
    let mut spans = Vec::with_capacity(row.chars().count());
    for (idx, ch) in row.chars().enumerate() {
        if ch == ' ' {
            spans.push(Span::raw(" "));
            continue;
        }
        let h = if total_cols <= 1 {
            0.0
        } else {
            idx as f32 / (total_cols - 1) as f32
        };
        // Mix horizontal + vertical + phase. `triangle_wave(phase)`
        // bounces back and forth in `[0, 1]` so the gradient sweeps
        // L→R then back R→L without a hard reset.
        let phase_t = triangle_wave(phase);
        let t = ((h + phase_t * 0.5) % 1.0 * 0.85 + v_shift * 0.15).clamp(0.0, 1.0);
        let rgb = convergio_brand::Rgb::lerp(convergio_brand::MAGENTA, convergio_brand::CYAN, t);
        let (r, g, b) = (
            (rgb.r as f32 * scan) as u8,
            (rgb.g as f32 * scan) as u8,
            (rgb.b as f32 * scan) as u8,
        );
        spans.push(Span::styled(
            ch.to_string(),
            Style::default()
                .fg(Color::Rgb(r, g, b))
                .add_modifier(Modifier::BOLD),
        ));
    }
    Line::from(spans)
}

/// `[0, 1]` triangle wave: 0 → 1 over the first half of the cycle,
/// 1 → 0 over the second. Smooth bounce so the gradient sweep
/// reverses direction without a visible jump.
fn triangle_wave(phase: f32) -> f32 {
    let p = phase.rem_euclid(1.0);
    if p < 0.5 {
        p * 2.0
    } else {
        2.0 - p * 2.0
    }
}

fn stats_paragraph(stats: &[String], height: u16) -> Paragraph<'static> {
    let style = Style::default()
        .fg(Color::Rgb(169, 177, 214))
        .add_modifier(Modifier::BOLD);
    let mut lines: Vec<Line<'static>> = Vec::with_capacity(height as usize);
    if (height as usize) >= stats.len() {
        for s in stats {
            lines.push(Line::from(Span::styled(s.clone(), style)).right_aligned());
        }
    } else {
        let joined = stats.join("  ·  ");
        lines.push(Line::from(Span::styled(joined, style)).right_aligned());
    }
    while lines.len() < height as usize {
        lines.push(Line::raw(""));
    }
    Paragraph::new(lines)
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::backend::TestBackend;
    use ratatui::Terminal;

    fn sample_stats() -> Vec<String> {
        vec![
            "plans:32".into(),
            "tasks:99".into(),
            "agents:5".into(),
            "prs:7".into(),
            "v0.3.10".into(),
        ]
    }

    #[test]
    fn renders_pixel_block_glyphs() {
        let backend = TestBackend::new(MIN_WIDTH + 4, HEIGHT);
        let mut term = Terminal::new(backend).unwrap();
        let stats = sample_stats();
        term.draw(|f| render(f, f.area(), &stats, 0.0)).unwrap();
        let buf = term.backend().buffer();
        let dump = buf.content().iter().map(|c| c.symbol()).collect::<String>();
        assert!(
            dump.contains('█'),
            "expected pixel-block glyphs in big banner: {dump:?}"
        );
        assert!(dump.contains("plans:32"));
        assert!(dump.contains("v0.3.10"));
    }

    #[test]
    fn triangle_wave_endpoints_and_peak() {
        assert!((triangle_wave(0.0) - 0.0).abs() < 1e-6);
        assert!((triangle_wave(0.5) - 1.0).abs() < 1e-6);
        assert!((triangle_wave(1.0) - 0.0).abs() < 1e-6);
        assert!((triangle_wave(0.25) - 0.5).abs() < 1e-6);
    }

    #[test]
    fn phase_now_stays_in_unit_interval() {
        let p = phase_now();
        assert!((0.0..1.0).contains(&p), "phase {p} out of [0,1)");
    }

    #[test]
    fn min_width_accommodates_brand_wordmark() {
        // Re-bind through `let` so clippy does not flag the
        // assertion as constant — we want this to be a runtime check
        // (a future palette/font change must trip this test).
        let min = MIN_WIDTH;
        let wm = wordmark_big::WIDTH;
        assert!(min >= wm, "MIN_WIDTH must fit the brand wordmark");
    }

    #[test]
    fn height_is_six_plus_spacer() {
        assert_eq!(HEIGHT, wordmark_big::HEIGHT + 1);
    }
}
