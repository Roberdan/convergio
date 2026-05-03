//! Brand palette — single source of truth for Convergio's neon
//! identity.
//!
//! The kit ships two accents on a black ground. Both accents clear
//! WCAG AA on `#000` (worst case: magenta at 5.39:1) so they are
//! safe for body-adjacent text, not just decoration.

/// 8-bit-per-channel RGB triple. Used by [`crate::gradient`] and the
/// banner renderer to emit `\x1b[38;2;R;G;Bm` truecolor sequences.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rgb {
    /// Red channel (0..=255).
    pub r: u8,
    /// Green channel (0..=255).
    pub g: u8,
    /// Blue channel (0..=255).
    pub b: u8,
}

impl Rgb {
    /// Construct an [`Rgb`] from individual channels.
    pub const fn new(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b }
    }

    /// Linear interpolation between two colours. `t` is clamped to
    /// `[0.0, 1.0]`. Used by the banner gradient.
    pub fn lerp(a: Rgb, b: Rgb, t: f32) -> Rgb {
        let t = t.clamp(0.0, 1.0);
        let mix = |x: u8, y: u8| -> u8 {
            let xf = f32::from(x);
            let yf = f32::from(y);
            (xf + (yf - xf) * t).round() as u8
        };
        Rgb::new(mix(a.r, b.r), mix(a.g, b.g), mix(a.b, b.b))
    }
}

/// Brand magenta — `#FF00B4`.
pub const MAGENTA: Rgb = Rgb::new(255, 0, 180);

/// Brand cyan — `#00C8FF`.
pub const CYAN: Rgb = Rgb::new(0, 200, 255);

/// Brand ground — pure black `#000000`. The banner is designed
/// against this background and the gradient is calibrated for it.
pub const BLACK: Rgb = Rgb::new(0, 0, 0);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lerp_endpoints() {
        assert_eq!(Rgb::lerp(MAGENTA, CYAN, 0.0), MAGENTA);
        assert_eq!(Rgb::lerp(MAGENTA, CYAN, 1.0), CYAN);
    }

    #[test]
    fn lerp_midpoint_is_blend() {
        let mid = Rgb::lerp(MAGENTA, CYAN, 0.5);
        // Midpoint between (255,0,180) and (0,200,255).
        assert_eq!(mid, Rgb::new(128, 100, 218));
    }

    #[test]
    fn lerp_clamps_out_of_range() {
        assert_eq!(Rgb::lerp(MAGENTA, CYAN, -1.0), MAGENTA);
        assert_eq!(Rgb::lerp(MAGENTA, CYAN, 2.0), CYAN);
    }

    #[test]
    fn brand_colors_are_distinct() {
        assert_ne!(MAGENTA, CYAN);
        assert_ne!(MAGENTA, BLACK);
        assert_ne!(CYAN, BLACK);
    }
}
