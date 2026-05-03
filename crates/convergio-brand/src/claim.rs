//! Brand claim and product name.
//!
//! These strings are the heart of the brand and are **not**
//! translated — they are trade dress, not UI copy. Translatable
//! prose (help text, descriptions) lives in `convergio-i18n` and
//! references the claim by interpolation when needed.

/// The product name as it appears on every surface.
pub const PRODUCT_NAME: &str = "Convergio";

/// Convergio's one-line claim. The promise the product keeps:
/// the gate pipeline turns "agent says it works" into "agent
/// **proved** it works".
pub const CLAIM: &str = "Make machines prove it.";

/// Subline that expands [`CLAIM`] into a description of what
/// Convergio actually is and does.
pub const SUBLINE: &str = "The machine that builds machines — and proves they work.";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn claim_is_short_and_punchy() {
        assert!(
            CLAIM.len() < 40,
            "claim should fit on one line of any terminal"
        );
    }

    #[test]
    fn subline_uses_em_dash() {
        // Brand spec uses a true em dash, not a hyphen.
        assert!(SUBLINE.contains('—'));
    }

    #[test]
    fn product_name_capitalisation_is_stable() {
        assert_eq!(PRODUCT_NAME, "Convergio");
    }
}
