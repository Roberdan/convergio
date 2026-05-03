//! Big pixel-block CONVERGIO wordmark — 6 rows tall, ~71 columns
//! wide, designed to dominate the top of an 80×24 terminal the way
//! the brand kit's `wordmark-pixel.png` dominates a marketing surface.
//!
//! The glyph data is the well-known *ANSI Shadow* figlet font.
//! Each row is laid out for a left-to-right magenta→cyan gradient
//! so it matches the rest of the brand kit byte-for-byte.

/// The six raw rows of the wordmark, before colouring. Width is
/// stable at 73 columns (Unicode display width 1 per char with this
/// font). Render with [`crate::gradient::render`] using
/// [`crate::MAGENTA`] → [`crate::CYAN`] for the canonical look.
pub const ROWS: [&str; 6] = [
    " ██████╗ ██████╗ ███╗   ██╗██╗   ██╗███████╗██████╗  ██████╗ ██╗ ██████╗ ",
    "██╔════╝██╔═══██╗████╗  ██║██║   ██║██╔════╝██╔══██╗██╔════╝ ██║██╔═══██╗",
    "██║     ██║   ██║██╔██╗ ██║██║   ██║█████╗  ██████╔╝██║  ███╗██║██║   ██║",
    "██║     ██║   ██║██║╚██╗██║╚██╗ ██╔╝██╔══╝  ██╔══██╗██║   ██║██║██║   ██║",
    "╚██████╗╚██████╔╝██║ ╚████║ ╚████╔╝ ███████╗██║  ██║╚██████╔╝██║╚██████╔╝",
    " ╚═════╝ ╚═════╝ ╚═╝  ╚═══╝  ╚═══╝  ╚══════╝╚═╝  ╚═╝ ╚═════╝ ╚═╝ ╚═════╝ ",
];

/// Number of rows the big wordmark occupies. Stable constant for
/// callers that need to budget vertical space.
pub const HEIGHT: u16 = 6;

/// Display width of every row in columns. Unicode display width is
/// 1 for every glyph in this font.
pub const WIDTH: u16 = 73;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn six_rows() {
        assert_eq!(ROWS.len(), 6);
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
    fn rows_are_pure_block_drawing() {
        // ANSI Shadow uses only space + a small set of box-drawing
        // chars. Catch accidental whitespace contamination.
        for row in ROWS {
            for c in row.chars() {
                assert!(
                    matches!(c, ' ' | '█' | '╗' | '╔' | '═' | '║' | '╝' | '╚'),
                    "unexpected glyph {c:?} in big wordmark"
                );
            }
        }
    }
}
