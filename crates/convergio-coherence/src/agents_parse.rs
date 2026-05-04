//! Git-log parsing helpers for [`crate::agents_scan`].
//!
//! Split out to honour the 300-line per-file cap. The git log output
//! is a sentinel-delimited stream of merge-commit records that this
//! module turns into [`crate::agents_scan::MergedPr`] values.

use crate::agents_scan::MergedPr;
use chrono::DateTime;

/// Parse the sentinel-delimited git log into [`MergedPr`] records.
pub(crate) fn parse_git_log(log: &str) -> Vec<MergedPr> {
    let mut out = Vec::new();
    for chunk in log.split("RECORD\x1e").filter(|s| !s.is_empty()) {
        let fields: Vec<&str> = chunk.splitn(5, '\x1f').collect();
        if fields.len() < 4 {
            continue;
        }
        let sha = fields[0].trim().to_string();
        let Ok(epoch) = fields[1].trim().parse::<i64>() else {
            continue;
        };
        let merged_at = match DateTime::from_timestamp(epoch, 0) {
            Some(t) => t,
            None => continue,
        };
        let author_name = fields[2].trim().to_string();
        let subject = fields[3].trim().to_string();
        let body = fields.get(4).map(|s| s.trim()).unwrap_or("");
        let (number, branch) = parse_merge_subject(&subject);
        let title = parse_pr_title(&subject, body);
        let author = pick_author(&author_name, &branch, body);
        out.push(MergedPr {
            number,
            author,
            branch,
            title,
            sha: sha.clone(),
            merged_at,
            // Cheap fallback: a precise per-PR `git log --not --merges`
            // would cost an extra spawn per row. Worst-case effect is
            // a slightly tighter heartbeat window.
            first_commit_at: merged_at,
        });
    }
    out
}

/// `Merge pull request #138 from Roberdan/feat/foo` →
/// `("138", "feat/foo")`.
pub(crate) fn parse_merge_subject(subject: &str) -> (String, String) {
    let mut number = String::new();
    let mut branch = String::new();
    if let Some(rest) = subject.strip_prefix("Merge pull request #") {
        let mut it = rest.splitn(2, ' ');
        if let Some(n) = it.next() {
            if n.chars().all(|c| c.is_ascii_digit()) {
                number = n.to_string();
            }
        }
        if let Some(tail) = it.next() {
            if let Some(branch_part) = tail.strip_prefix("from ") {
                branch = branch_part
                    .split_once('/')
                    .map(|x| x.1)
                    .unwrap_or(branch_part)
                    .trim()
                    .to_string();
            }
        }
    }
    (number, branch)
}

/// PR title heuristic: first non-empty line of the merge body, or the
/// merge subject when there is no body.
fn parse_pr_title(subject: &str, body: &str) -> String {
    for line in body.lines() {
        let t = line.trim();
        if !t.is_empty() {
            return t.to_string();
        }
    }
    subject.to_string()
}

/// Pick an author login. Order:
///   1. `author_name` field (the human who clicked Merge)
///   2. `Co-Authored-By:` trailer in the body, lower-cased local-part
///
/// We never invent a value — empty string means "could not resolve".
fn pick_author(author_name: &str, _branch: &str, body: &str) -> String {
    if !author_name.is_empty() && !is_machine_author(author_name) {
        return author_name.to_string();
    }
    if let Some(login) = parse_coauthored_by(body) {
        return login;
    }
    author_name.to_string()
}

/// True for known bot / machine authors.
pub(crate) fn is_machine_author(name: &str) -> bool {
    let n = name.to_ascii_lowercase();
    n.contains("[bot]") || n == "github" || n == "github-actions"
}

/// Parse a `Co-Authored-By: Name <local@host>` trailer to its email
/// local-part.
pub(crate) fn parse_coauthored_by(body: &str) -> Option<String> {
    for line in body.lines() {
        let l = line.trim();
        let lower = l.to_ascii_lowercase();
        if let Some(rest) = lower.strip_prefix("co-authored-by:") {
            let rest = rest.trim();
            if let Some(start) = rest.find('<') {
                let email = &rest[start + 1..];
                if let Some(end) = email.find('@') {
                    return Some(email[..end].to_string());
                }
            }
            if let Some(first) = rest.split_whitespace().next() {
                return Some(first.to_string());
            }
        }
    }
    None
}

/// Detect a revision range like `origin/main~50..origin/main`.
pub(crate) fn parse_revision_range(since: &str) -> Option<&str> {
    if since.contains("..") {
        Some(since)
    } else {
        None
    }
}

/// Translate `7d` → `7 days ago`, `48h` → `48 hours ago`, otherwise
/// pass through (git accepts ISO dates and many natural forms).
pub(crate) fn normalise_since(since: &str) -> String {
    if let Some(n) = since.strip_suffix('d') {
        if n.chars().all(|c| c.is_ascii_digit()) {
            return format!("{n} days ago");
        }
    }
    if let Some(n) = since.strip_suffix('h') {
        if n.chars().all(|c| c.is_ascii_digit()) {
            return format!("{n} hours ago");
        }
    }
    since.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_subject_pr_number_and_branch() {
        let (n, b) =
            parse_merge_subject("Merge pull request #138 from Roberdan/feat/coherence-agents");
        assert_eq!(n, "138");
        assert_eq!(b, "feat/coherence-agents");
    }

    #[test]
    fn parse_subject_handles_non_pr_merge() {
        let (n, b) = parse_merge_subject("Merge branch 'main' into feat/x");
        assert!(n.is_empty());
        assert!(b.is_empty());
    }

    #[test]
    fn normalise_since_handles_d_h_passthrough() {
        assert_eq!(normalise_since("7d"), "7 days ago");
        assert_eq!(normalise_since("48h"), "48 hours ago");
        assert_eq!(normalise_since("2026-05-01"), "2026-05-01");
    }

    #[test]
    fn parse_revision_range_detects_dotdot() {
        assert!(parse_revision_range("origin/main~5..origin/main").is_some());
        assert!(parse_revision_range("7d").is_none());
    }

    #[test]
    fn parse_coauthored_by_extracts_email_local() {
        let body =
            "feat: bla\n\nCo-Authored-By: Claude Opus <noreply@anthropic.com>\nSigned-off-by: x\n";
        assert_eq!(parse_coauthored_by(body), Some("noreply".to_string()));
    }

    #[test]
    fn parse_log_one_record_round_trip() {
        let raw = format!(
            "RECORD\x1e{sha}\x1f{epoch}\x1f{author}\x1f{subject}\x1f{body}",
            sha = "deadbeefdeadbeefdeadbeefdeadbeefdeadbeef",
            epoch = 1_700_000_000,
            author = "Roberdan",
            subject = "Merge pull request #99 from Roberdan/feat/x",
            body = "feat(coherence): something\n"
        );
        let prs = parse_git_log(&raw);
        assert_eq!(prs.len(), 1);
        let pr = &prs[0];
        assert_eq!(pr.number, "99");
        assert_eq!(pr.branch, "feat/x");
        assert_eq!(pr.author, "Roberdan");
    }

    #[test]
    fn machine_authors_detected() {
        assert!(is_machine_author("dependabot[bot]"));
        assert!(is_machine_author("github-actions"));
        assert!(!is_machine_author("Roberdan"));
    }
}
