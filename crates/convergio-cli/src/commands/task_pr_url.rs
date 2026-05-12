//! Helpers extracted from `task.rs` so the command file keeps
//! headroom under the 300-line cap (T828d03c audit follow-up).

/// Parse a GitHub PR URL (`https://github.com/<owner>/<repo>/pull/<n>`)
/// into a `(repo_slug, pr_number)` pair. Returns `None` for anything
/// that does not match that exact shape — best-effort by design.
pub(super) fn parse_github_pr_url(url: &str) -> Option<(String, i64)> {
    let url = url.trim();
    let url = url
        .strip_prefix("https://github.com/")
        .or_else(|| url.strip_prefix("http://github.com/"))?;
    let mut parts = url.split('/');
    let owner = parts.next()?.trim();
    let repo = parts.next()?.trim();
    let kind = parts.next()?;
    if kind != "pull" {
        return None;
    }
    let pr_raw = parts.next()?;
    let pr_number_str = pr_raw.split(['?', '#']).next().unwrap_or("");
    let pr_number = pr_number_str.parse::<i64>().ok()?;
    if owner.is_empty() || repo.is_empty() {
        return None;
    }

    Some((format!("{owner}/{repo}"), pr_number))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_canonical_pr_url() {
        assert_eq!(
            parse_github_pr_url("https://github.com/foo/bar/pull/42"),
            Some(("foo/bar".to_string(), 42))
        );
    }

    #[test]
    fn rejects_non_pull_paths() {
        assert_eq!(
            parse_github_pr_url("https://github.com/foo/bar/issues/42"),
            None
        );
    }

    #[test]
    fn strips_trailing_query_or_fragment() {
        assert_eq!(
            parse_github_pr_url("https://github.com/foo/bar/pull/42?notification_referrer_id=x"),
            Some(("foo/bar".to_string(), 42))
        );
        assert_eq!(
            parse_github_pr_url("https://github.com/foo/bar/pull/42#discussion_r1"),
            Some(("foo/bar".to_string(), 42))
        );
    }
}
