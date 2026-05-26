pub(super) fn has_color_only_signal(strings: &[String]) -> bool {
    for s in strings {
        for line in s.lines() {
            let (stripped, had_ansi) = strip_ansi(line);
            if had_ansi && stripped.trim().is_empty() {
                return true;
            }
        }
    }
    false
}

fn strip_ansi(line: &str) -> (String, bool) {
    let bytes = line.as_bytes();
    let mut out: Vec<u8> = Vec::with_capacity(bytes.len());
    let mut i = 0usize;
    let mut had = false;

    while i < bytes.len() {
        if bytes[i] == 0x1b && i + 1 < bytes.len() && bytes[i + 1] == b'[' {
            had = true;
            i += 2;
            while i < bytes.len() {
                let b = bytes[i];
                i += 1;
                if (0x40..=0x7E).contains(&b) {
                    break;
                }
            }
            continue;
        }
        out.push(bytes[i]);
        i += 1;
    }

    (String::from_utf8_lossy(&out).to_string(), had)
}
