//! Source-scan helpers for [`crate::adrs`].
//!
//! Split out to honour the 300-line per-file cap.

use anyhow::{Context, Result};
use std::path::Path;

/// Parse `superseded by 0042` → `Some("0042")`. Lower-case input.
pub(super) fn parse_superseded_by(status_lc: &str) -> Option<String> {
    let rest = status_lc.strip_prefix("superseded by")?.trim();
    let token = rest
        .split_whitespace()
        .next()?
        .trim_matches(|c: char| !c.is_ascii_digit() && c != '0');
    if !token.is_empty() && token.chars().all(|c| c.is_ascii_digit()) {
        Some(format!("{:0>4}", token))
    } else {
        None
    }
}

/// Topic keywords from an ADR slug. `0006-crdt-storage` → `["crdt",
/// "storage"]`. Filters out short / common stopwords.
pub(super) fn slug_keywords(slug: &str) -> Vec<String> {
    const STOP: &[&str] = &[
        "and", "the", "for", "with", "into", "from", "over", "kind", "kinds", "field", "fields",
        "tier", "layer", "policy", "init",
    ];
    slug.split('-')
        .filter(|s| s.len() >= 4)
        .filter(|s| !s.chars().all(|c| c.is_ascii_digit()))
        .filter(|s| !STOP.contains(&s.to_ascii_lowercase().as_str()))
        .map(|s| s.to_ascii_lowercase())
        .collect()
}

/// Return all crate directory names under `<root>/crates/`.
pub(super) fn all_crates(root: &Path) -> Result<Vec<String>> {
    let crates_dir = root.join("crates");
    let mut out: Vec<String> = Vec::new();
    if !crates_dir.exists() {
        return Ok(out);
    }
    for e in std::fs::read_dir(&crates_dir)
        .with_context(|| format!("read_dir {}", crates_dir.display()))?
    {
        let e = e?;
        if e.path().is_dir() {
            if let Some(name) = e.file_name().to_str() {
                out.push(name.to_string());
            }
        }
    }
    Ok(out)
}

/// Walk a list of crate names under `root/crates/` and look for
/// either an `ADR-NNNN` mention or any topic keyword from `slug`.
/// Returns the first hit's display string (relative-ish path).
pub(super) fn find_evidence(
    id: &str,
    slug: &str,
    crates: &[String],
    root: &Path,
) -> Result<Option<String>> {
    let needle_adr = format!("ADR-{id}");
    let needle_adr_lc = format!("adr-{id}");
    let needle_adr_space = format!("ADR {id}");
    let keywords = slug_keywords(slug);
    for c in crates {
        let dir = root.join("crates").join(c).join("src");
        if !dir.exists() {
            continue;
        }
        if let Some(hit) = scan_dir(
            &dir,
            &needle_adr,
            &needle_adr_lc,
            &needle_adr_space,
            &keywords,
        )? {
            return Ok(Some(hit));
        }
    }
    Ok(None)
}

fn scan_dir(
    dir: &Path,
    n_adr: &str,
    n_adr_lc: &str,
    n_adr_space: &str,
    keywords: &[String],
) -> Result<Option<String>> {
    for entry in walkdir::WalkDir::new(dir)
        .follow_links(false)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        let p = entry.path();
        if !p.is_file() || p.extension().and_then(|s| s.to_str()) != Some("rs") {
            continue;
        }
        let body = std::fs::read_to_string(p).with_context(|| format!("read {}", p.display()))?;
        if body.contains(n_adr) || body.contains(n_adr_lc) || body.contains(n_adr_space) {
            return Ok(Some(format!("{}", p.display())));
        }
        for kw in keywords {
            if body_mentions_keyword(&body, kw) {
                return Ok(Some(format!("{} (matches '{kw}')", p.display())));
            }
        }
    }
    Ok(None)
}

fn body_mentions_keyword(body: &str, keyword: &str) -> bool {
    if keyword.len() < 4 {
        return false;
    }
    let needle = keyword.to_ascii_lowercase();
    let hay = body.to_ascii_lowercase();
    hay.contains(&needle)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_superseded_by_extracts_id() {
        assert_eq!(
            parse_superseded_by("superseded by 0042"),
            Some("0042".into())
        );
        assert_eq!(parse_superseded_by("accepted"), None);
    }

    #[test]
    fn slug_keywords_drops_stopwords_and_short_tokens() {
        let kws = slug_keywords("0006-crdt-storage-and-init");
        assert!(kws.contains(&"crdt".to_string()));
        assert!(kws.contains(&"storage".to_string()));
        assert!(!kws.iter().any(|s| s == "and"));
        assert!(!kws.iter().any(|s| s == "init"));
    }

    #[test]
    fn slug_keywords_drops_id_token() {
        let kws = slug_keywords("0099-foo-bar");
        assert!(!kws.iter().any(|s| s == "0099"));
    }

    #[test]
    fn body_mentions_keyword_short_skipped() {
        assert!(!body_mentions_keyword("foo bar baz", "foo"));
    }
}
