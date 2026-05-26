pub(super) fn has_color_only_emphasis(text: &str) -> bool {
    let lower = text.to_ascii_lowercase();
    let mut i = 0usize;
    while let Some(pos) = lower[i..].find("<font") {
        let start = i + pos;
        let end = lower[start..]
            .find('>')
            .map(|e| start + e)
            .unwrap_or(lower.len());
        if lower[start..end].contains("color=") {
            return true;
        }
        i = start + 5;
    }
    false
}

pub(super) fn has_low_contrast_inline_style(text: &str) -> bool {
    let lower = text.to_ascii_lowercase();
    let bytes = text.as_bytes();
    let mut i = 0usize;

    while let Some(pos) = lower[i..].find("style=") {
        let mut j = i + pos + 6;
        while j < bytes.len() && bytes[j].is_ascii_whitespace() {
            j += 1;
        }
        if j >= bytes.len() {
            break;
        }

        let quote = bytes[j];
        if quote != b'"' && quote != b'\'' {
            i = j + 1;
            continue;
        }
        j += 1;

        let end = text[j..]
            .find(quote as char)
            .map(|e| j + e)
            .unwrap_or(text.len());
        let style = &text[j..end];
        if style_has_low_contrast(style) {
            return true;
        }
        i = end + 1;
    }

    false
}

fn style_has_low_contrast(style: &str) -> bool {
    let mut fg: Option<Rgb> = None;
    let mut bg: Option<Rgb> = None;

    for decl in style.split(';') {
        let Some((k, v)) = decl.split_once(':') else {
            continue;
        };
        let key = k.trim().to_ascii_lowercase();
        if key == "color" {
            fg = parse_hex_color_from_value(v);
        } else if key == "background-color" {
            bg = parse_hex_color_from_value(v);
        }
    }

    match (fg, bg) {
        (Some(fg), Some(bg)) => contrast_ratio(fg, bg) < 4.5,
        _ => false,
    }
}

fn parse_hex_color_from_value(value: &str) -> Option<Rgb> {
    let v = value.trim();
    let hash = v.find('#')?;
    parse_hex_color(&v[hash..])
}

#[derive(Copy, Clone)]
struct Rgb {
    r: u8,
    g: u8,
    b: u8,
}

fn parse_hex_color(hex: &str) -> Option<Rgb> {
    let hex = hex.strip_prefix('#')?;
    match hex.len() {
        3 => {
            let r = u8::from_str_radix(&hex[0..1].repeat(2), 16).ok()?;
            let g = u8::from_str_radix(&hex[1..2].repeat(2), 16).ok()?;
            let b = u8::from_str_radix(&hex[2..3].repeat(2), 16).ok()?;
            Some(Rgb { r, g, b })
        }
        6 => {
            let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
            let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
            let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
            Some(Rgb { r, g, b })
        }
        _ => None,
    }
}

fn contrast_ratio(a: Rgb, b: Rgb) -> f64 {
    let la = relative_luminance(a);
    let lb = relative_luminance(b);
    let (l1, l2) = if la >= lb { (la, lb) } else { (lb, la) };
    (l1 + 0.05) / (l2 + 0.05)
}

fn relative_luminance(c: Rgb) -> f64 {
    0.2126 * srgb_to_linear(c.r) + 0.7152 * srgb_to_linear(c.g) + 0.0722 * srgb_to_linear(c.b)
}

fn srgb_to_linear(v: u8) -> f64 {
    let s = f64::from(v) / 255.0;
    if s <= 0.03928 {
        s / 12.92
    } else {
        ((s + 0.055) / 1.055).powf(2.4)
    }
}
