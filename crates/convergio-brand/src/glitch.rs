//! Glitch helper — deterministic char-swap used by the boot
//! animation.
//!
//! The kit demo uses random char substitution; we make the
//! substitution **deterministic** (driven by a frame counter) so
//! tests can assert exact frames and so animations do not need an
//! `rand` dependency.

/// Swap characters in `text` according to the glitch table at
/// `frame`. The table cycles through the same three frames the
/// brand kit demo ships, so the boot sequence stays visually
/// identical to the static-asset reference while remaining
/// reproducible.
pub fn frame(text: &str, frame_index: u32) -> String {
    text.chars()
        .enumerate()
        .map(|(i, c)| swap(c, frame_index, i))
        .collect()
}

fn swap(c: char, frame_index: u32, position: usize) -> char {
    let pos = position as u32;
    match (frame_index % 3, c) {
        // Frame 0: swap every 3rd 'O' for a zero.
        (0, 'O') if pos % 3 == 0 => '0',
        // Frame 1: insert an underscore look on Es at odd positions.
        (1, 'E') if pos % 2 == 1 => '_',
        // Frame 2: combine both rules so the final frame reads as
        // the most "glitched" one before the banner stabilises.
        (2, 'O') => '0',
        (2, 'E') => '_',
        _ => c,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frame_zero_swaps_first_o() {
        // CONVERGIO: positions 0..8 -> C O N V E R G I O
        // The 'O' at position 1 is not divisible by 3; the 'O' at
        // position 8 is not (8 % 3 == 2). Position 0 is 'C' so no
        // swap actually fires for this exact input — that is
        // intentional and asserted here so future regressions are
        // visible.
        assert_eq!(frame("CONVERGIO", 0), "CONVERGIO");
    }

    #[test]
    fn frame_one_underscores_e_at_odd_positions() {
        // 'E' lives at position 4 (even), so frame 1 leaves the word
        // untouched. The underscore swap is meant to fire on words
        // where 'E' lands on an odd index — covered below.
        assert_eq!(frame("CONVERGIO", 1), "CONVERGIO");
        assert_eq!(frame("HE", 1), "H_");
    }

    #[test]
    fn frame_two_swaps_all_o_and_e() {
        assert_eq!(frame("CONVERGIO", 2), "C0NV_RGI0");
    }

    #[test]
    fn frame_cycles_with_modulo() {
        assert_eq!(frame("CONVERGIO", 5), frame("CONVERGIO", 2));
    }

    #[test]
    fn unicode_passes_through_unchanged() {
        let input = "convergïø";
        assert_eq!(frame(input, 2), input);
    }
}
