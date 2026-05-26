mod heading;
mod images;
mod links;
mod style;

pub(super) fn check(kind: &str, text: &str, violations: &mut Vec<String>) {
    if heading::has_heading_skip(text) {
        violations.push(format!("{kind}#md_heading_skip"));
    }
    if images::has_missing_alt(text) {
        violations.push(format!("{kind}#md_image_missing_alt"));
    }
    if links::has_nondescriptive_link(text) {
        violations.push(format!("{kind}#md_link_nondescriptive"));
    }
    if style::has_color_only_emphasis(text) {
        violations.push(format!("{kind}#md_color_only_emphasis"));
    }
    if style::has_low_contrast_inline_style(text) {
        violations.push(format!("{kind}#md_color_contrast_low"));
    }
}
