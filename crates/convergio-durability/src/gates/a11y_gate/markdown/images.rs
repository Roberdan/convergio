pub(super) fn has_missing_alt(text: &str) -> bool {
    // Markdown image syntax: ![alt](url)
    let mut i = 0usize;
    while let Some(pos) = text[i..].find("![") {
        let start = i + pos + 2;
        if let Some(end_bracket) = text[start..].find(']') {
            let alt = &text[start..start + end_bracket];
            let after = &text[start + end_bracket + 1..];
            if after.trim_start().starts_with('(') && alt.trim().is_empty() {
                return true;
            }
            i = start + end_bracket + 1;
        } else {
            break;
        }
    }

    html_img_missing_alt(text)
}

fn html_img_missing_alt(text: &str) -> bool {
    let lower = text.to_ascii_lowercase();
    let mut i = 0usize;
    while let Some(pos) = lower[i..].find("<img") {
        let start = i + pos;
        let end = lower[start..]
            .find('>')
            .map(|e| start + e + 1)
            .unwrap_or(lower.len());
        let tag = &lower[start..end];

        if !tag.contains("alt=") {
            return true;
        }
        if alt_value_is_blank(tag) {
            return true;
        }

        i = end;
    }
    false
}

fn alt_value_is_blank(lower_tag: &str) -> bool {
    let pos = match lower_tag.find("alt=") {
        Some(p) => p + 4,
        None => return true,
    };

    let mut rest = &lower_tag[pos..];
    rest = rest.trim_start_matches(|c: char| c.is_ascii_whitespace());
    if rest.is_empty() {
        return true;
    }

    let bytes = rest.as_bytes();
    let first = bytes[0];
    if first == b'"' || first == b'\'' {
        let quote = first as char;
        let inner = &rest[1..];
        match inner.find(quote) {
            Some(end) => inner[..end].trim().is_empty(),
            None => true,
        }
    } else {
        let token = rest
            .split(|c: char| c.is_ascii_whitespace() || c == '>')
            .next()
            .unwrap_or("");
        token.trim().is_empty()
    }
}
