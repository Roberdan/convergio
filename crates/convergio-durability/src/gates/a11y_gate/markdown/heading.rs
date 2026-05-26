/// Detects an H1->H3 (or any other skipped-level) sequence. We allow the
/// document to start at any level; only forward jumps of more than one
/// inside the document are flagged.
pub(super) fn has_heading_skip(text: &str) -> bool {
    let mut last: Option<usize> = None;
    for line in text.lines() {
        let mut level = 0usize;
        for c in line.chars() {
            if c == '#' && level < 6 {
                level += 1;
            } else {
                break;
            }
        }
        if level == 0 {
            continue;
        }
        if !line[level..].starts_with(' ') {
            continue;
        }
        if let Some(prev) = last {
            if level > prev + 1 {
                return true;
            }
        }
        last = Some(level);
    }
    false
}
