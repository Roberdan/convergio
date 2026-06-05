use super::SearchResult;
use std::collections::HashMap;

pub(super) fn upsert(map: &mut HashMap<(String, String), SearchResult>, mut next: SearchResult) {
    let key = (next.kind.clone(), next.id.clone());
    match map.get_mut(&key) {
        Some(existing) => {
            if next.score > existing.score {
                existing.score = next.score;
            }
            for s in next.match_sources.drain(..) {
                if !existing.match_sources.contains(&s) {
                    existing.match_sources.push(s);
                }
            }
        }
        None => {
            map.insert(key, next);
        }
    }
}

pub(super) fn matches_ci<S, I>(needle: &str, haystack: I) -> bool
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    let n = needle.trim().to_ascii_lowercase();
    if n.is_empty() {
        return false;
    }
    haystack
        .into_iter()
        .any(|h| h.as_ref().to_ascii_lowercase().contains(&n))
}

pub(super) fn score_fields<S, I>(needle: &str, fields: I) -> f64
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    let n = needle.trim().to_ascii_lowercase();
    let mut best: f64 = 0.0;
    for f in fields {
        let f = f.as_ref().to_ascii_lowercase();
        if f == n {
            best = best.max(20.0);
            continue;
        }
        if f.starts_with(&n) {
            best = best.max(10.0);
            continue;
        }
        if f.contains(&n) {
            best = best.max(5.0);
        }
    }
    best
}

pub(super) fn href(kind: &str, id: &str) -> String {
    // UI contract: `/o/[type]/[id]`. `id` is percent-encoded as a single path segment.
    format!("/o/{kind}/{}", encode_path_segment(id))
}

fn encode_path_segment(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for b in value.as_bytes() {
        let c = *b;
        let ok = matches!(
            c,
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~'
        );
        if ok {
            out.push(c as char);
        } else {
            out.push('%');
            out.push_str(&format!("{c:02X}"));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::encode_path_segment;

    #[test]
    fn encode_path_segment_percent_encodes_slash() {
        assert_eq!(encode_path_segment("a/b"), "a%2Fb");
    }
}
