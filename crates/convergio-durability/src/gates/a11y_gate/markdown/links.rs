pub(super) fn has_nondescriptive_link(text: &str) -> bool {
    // Markdown links: [label](url)
    let bytes = text.as_bytes();
    let mut i = 0usize;
    while let Some(pos) = text[i..].find('[') {
        let start = i + pos;
        if start > 0 && bytes[start - 1] == b'!' {
            i = start + 1;
            continue;
        }

        let label_start = start + 1;
        if let Some(end_bracket) = text[label_start..].find(']') {
            let label_end = label_start + end_bracket;
            let label = &text[label_start..label_end];
            let after = &text[label_end + 1..];
            if after.trim_start().starts_with('(') && is_nondescriptive_link_text(label) {
                return true;
            }
        }

        i = start + 1;
    }

    html_link_has_nondescriptive_text(text)
}

fn html_link_has_nondescriptive_text(text: &str) -> bool {
    let lower = text.to_ascii_lowercase();
    let mut i = 0usize;
    while let Some(pos) = lower[i..].find("<a") {
        let start = i + pos;
        let open_end = match lower[start..].find('>') {
            Some(e) => start + e + 1,
            None => break,
        };
        let close_start = match lower[open_end..].find("</a>") {
            Some(p) => open_end + p,
            None => {
                i = open_end;
                continue;
            }
        };
        let inner = &text[open_end..close_start];
        let stripped = strip_html_tags(inner);
        if is_nondescriptive_link_text(&stripped) {
            return true;
        }
        i = close_start + 4;
    }
    false
}

fn strip_html_tags(s: &str) -> String {
    let mut out = String::new();
    let mut in_tag = false;
    for ch in s.chars() {
        match ch {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if !in_tag => out.push(ch),
            _ => {}
        }
    }
    out
}

fn is_nondescriptive_link_text(text: &str) -> bool {
    matches!(
        text.trim().to_ascii_lowercase().as_str(),
        "here" | "click here" | "click" | "link" | "this" | "read more" | "more"
    )
}
