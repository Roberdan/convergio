//! Big pixel-block CONVERGIO wordmark — 4 rows × 67 cols, fully
//! solid blocks (`█▀▄`), no thin line-art glyphs.
//!
//! Designed in-house (not a figlet font) to match the chunky LED-
//! display look of the brand kit's `wordmark-pixel.png`. The
//! previous ANSI Shadow font (commit `6ae8176`, replaced here)
//! used box-drawing chars (`╗╔═║`) which read as outlined rather
//! than solid — fine for architecture diagrams, wrong for a brand
//! mark that should feel **dense**.
//!
//! Each letter occupies 7 cols (3 cols for `I`) and 4 terminal
//! rows. A single space separates letters. Render with the brand
//! magenta→cyan gradient via `convergio_brand::gradient`.

/// The four raw rows of the wordmark, before colouring. Width is
/// stable at 67 columns (Unicode display width 1 per glyph; only
/// `space`, `█`, `▀`, `▄` appear). Render top-to-bottom.
pub const ROWS: [&str; 4] = [
    "▄█▀▀▀█▄ ▄█▀▀▀█▄ ██   ██ ██   ██ ██▀▀▀▀▀ ██▀▀▀█▄ ▄█▀▀▀█▄ ▀█▀ ▄█▀▀▀█▄",
    "██      ██   ██ ███▄ ██ ▀█   █▀ ██▄▄▄▄  ██▄▄▄█▀ ██  ▄▄▄  █  ██   ██",
    "██      ██   ██ ██ ▀███  ██ ██  ██▀▀▀▀  ██▀█▄   ██   ██  █  ██   ██",
    "▀█▄▄▄█▀ ▀█▄▄▄█▀ ██   ██   ▀█▀   ██▄▄▄▄▄ ██  ▀█▄ ▀█▄▄▄█▀ ▄█▄ ▀█▄▄▄█▀",
];

/// Number of rows the big wordmark occupies. Stable constant for
/// callers that need to budget vertical space.
pub const HEIGHT: u16 = 4;

/// Display width of every row in columns. Unicode display width is
/// 1 for every glyph in this font.
pub const WIDTH: u16 = 67;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn four_rows() {
        assert_eq!(ROWS.len(), 4);
        assert_eq!(HEIGHT, ROWS.len() as u16);
    }

    #[test]
    fn every_row_is_same_width() {
        let w = ROWS[0].chars().count();
        for (i, row) in ROWS.iter().enumerate() {
            assert_eq!(
                row.chars().count(),
                w,
                "row {i} differs in char count: {row:?}"
            );
        }
        assert_eq!(WIDTH as usize, w);
    }

    #[test]
    fn rows_are_solid_block_only() {
        // Custom font uses ONLY space + solid blocks + half blocks.
        // Catches accidental contamination by line-drawing chars,
        // emojis, or whitespace lookalikes.
        for row in ROWS {
            for c in row.chars() {
                assert!(
                    matches!(c, ' ' | '█' | '▀' | '▄'),
                    "unexpected glyph {c:?} in solid-block wordmark — \
                     this font is full-block only"
                );
            }
        }
    }
}
