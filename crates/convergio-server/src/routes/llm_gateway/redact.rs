//! Outbound PII/secret redactor chain for the LLM gateway.
//!
//! Pure, offline, dependency-light. Each prompt token is classified and,
//! when it matches an email, phone number, or common secret/API-key
//! pattern, masked before the prompt leaves the daemon. Heuristic by
//! design — it favours masking obvious leaks over perfect recall.

use serde::Serialize;

const EMAIL_MASK: &str = "[REDACTED_EMAIL]";
const PHONE_MASK: &str = "[REDACTED_PHONE]";
const SECRET_MASK: &str = "[REDACTED_SECRET]";

/// Category of a single redaction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub(super) enum RedactionKind {
    /// An e-mail address.
    Email,
    /// A phone number.
    Phone,
    /// A secret, token, or API key.
    Secret,
}

/// Result of running the redactor chain over a prompt.
#[derive(Debug, Clone)]
pub(super) struct RedactionReport {
    /// The prompt with every detected leak masked.
    pub(super) redacted: String,
    /// One entry per masked token, in scan order.
    pub(super) findings: Vec<RedactionKind>,
}

/// Punctuation stripped from a token before classification and restored
/// around the mask afterwards.
const AFFIX: &[char] = &[
    '.', ',', ';', ':', '!', '?', '(', ')', '[', ']', '{', '}', '<', '>', '"', '\'', '`',
];

/// Run the redactor chain over `input`, masking emails, phone numbers and
/// common secrets while preserving the surrounding whitespace.
pub(super) fn redact_prompt(input: &str) -> RedactionReport {
    let mut redacted = String::with_capacity(input.len());
    let mut findings = Vec::new();
    let mut pending_secret = false;

    for token in tokens(input) {
        match token {
            Token::Space(s) => redacted.push_str(s),
            Token::Word(w) => {
                if pending_secret {
                    pending_secret = false;
                    redacted.push_str(SECRET_MASK);
                    findings.push(RedactionKind::Secret);
                    continue;
                }
                if w.eq_ignore_ascii_case("bearer") {
                    pending_secret = true;
                    redacted.push_str(w);
                    continue;
                }
                match mask_word(w) {
                    Some((masked, kind)) => {
                        redacted.push_str(&masked);
                        findings.push(kind);
                    }
                    None => redacted.push_str(w),
                }
            }
        }
    }

    RedactionReport { redacted, findings }
}

enum Token<'a> {
    Word(&'a str),
    Space(&'a str),
}

/// Split `input` into alternating word and whitespace runs without losing
/// any byte, so redaction is whitespace-preserving.
fn tokens(input: &str) -> Vec<Token<'_>> {
    let mut out = Vec::new();
    let mut start = 0usize;
    let mut in_ws: Option<bool> = None;
    for (idx, ch) in input.char_indices() {
        let ws = ch.is_whitespace();
        match in_ws {
            Some(prev) if prev == ws => {}
            Some(prev) => {
                let slice = &input[start..idx];
                out.push(if prev {
                    Token::Space(slice)
                } else {
                    Token::Word(slice)
                });
                start = idx;
                in_ws = Some(ws);
            }
            None => in_ws = Some(ws),
        }
    }
    if let Some(prev) = in_ws {
        let slice = &input[start..];
        out.push(if prev {
            Token::Space(slice)
        } else {
            Token::Word(slice)
        });
    }
    out
}

/// Classify one word, returning its masked form (preserving leading and
/// trailing punctuation) when it leaks something sensitive.
fn mask_word(word: &str) -> Option<(String, RedactionKind)> {
    let after_pre = word.trim_start_matches(AFFIX);
    let pre = &word[..word.len() - after_pre.len()];
    let core = after_pre.trim_end_matches(AFFIX);
    let suf = &after_pre[core.len()..];
    if core.is_empty() {
        return None;
    }

    if is_email(core) {
        return Some((format!("{pre}{EMAIL_MASK}{suf}"), RedactionKind::Email));
    }
    if let Some(masked_core) = secret_kv(core) {
        return Some((format!("{pre}{masked_core}{suf}"), RedactionKind::Secret));
    }
    if is_secret_token(core) {
        return Some((format!("{pre}{SECRET_MASK}{suf}"), RedactionKind::Secret));
    }
    if is_phone(core) {
        return Some((format!("{pre}{PHONE_MASK}{suf}"), RedactionKind::Phone));
    }
    None
}

fn is_email(s: &str) -> bool {
    let Some(at) = s.find('@') else { return false };
    let (local, domain) = (&s[..at], &s[at + 1..]);
    if local.is_empty() || domain.contains('@') {
        return false;
    }
    if !domain.contains('.') || domain.starts_with('.') || domain.ends_with('.') {
        return false;
    }
    local
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || "._%+-".contains(c))
        && domain
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || ".-".contains(c))
}

/// Detect `key=value` pairs whose key names a credential, masking the value.
fn secret_kv(s: &str) -> Option<String> {
    let (key, value) = s.split_once('=')?;
    if value.len() < 4 {
        return None;
    }
    let k = key.to_ascii_lowercase();
    let sensitive = [
        "apikey", "api_key", "key", "token", "secret", "password", "passwd",
    ]
    .iter()
    .any(|needle| k.contains(needle));
    sensitive.then(|| format!("{key}={SECRET_MASK}"))
}

fn is_secret_token(s: &str) -> bool {
    const PREFIXES: &[&str] = &[
        "sk-",
        "sk_live_",
        "sk_test_",
        "ghp_",
        "gho_",
        "ghs_",
        "ghu_",
        "github_pat_",
        "xoxb-",
        "xoxp-",
        "AKIA",
        "ASIA",
        "AIza",
    ];
    if PREFIXES.iter().any(|p| s.starts_with(p)) {
        return true;
    }
    if s.len() >= 32
        && s.chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | '+' | '/'))
    {
        let has_digit = s.chars().any(|c| c.is_ascii_digit());
        let has_alpha = s.chars().any(|c| c.is_ascii_alphabetic());
        return has_digit && has_alpha;
    }
    false
}

fn is_phone(s: &str) -> bool {
    let mut digits = 0usize;
    for c in s.chars() {
        if c.is_ascii_digit() {
            digits += 1;
        } else if !matches!(c, '+' | '-' | '(' | ')' | '.') {
            return false;
        }
    }
    (7..=15).contains(&digits)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn masks_email_phone_and_secrets() {
        let out = redact_prompt("contact john.doe@example.com, call +1-202-555-0173");
        assert_eq!(
            out.redacted,
            "contact [REDACTED_EMAIL], call [REDACTED_PHONE]"
        );
        assert_eq!(
            out.findings,
            vec![RedactionKind::Email, RedactionKind::Phone]
        );
    }

    #[test]
    fn masks_api_key_prefixes_and_bearer_tokens() {
        let out = redact_prompt("key sk-ABCDEF0123456789 and Bearer eyJhbGciOiJI");
        assert!(out.redacted.contains("[REDACTED_SECRET]"));
        assert_eq!(out.findings.len(), 2);
    }

    #[test]
    fn masks_key_value_secret() {
        let out = redact_prompt("api_key=supersecretvalue");
        assert_eq!(out.redacted, "api_key=[REDACTED_SECRET]");
        assert_eq!(out.findings, vec![RedactionKind::Secret]);
    }

    #[test]
    fn leaves_clean_prompt_untouched() {
        let out = redact_prompt("summarize the quarterly report");
        assert_eq!(out.redacted, "summarize the quarterly report");
        assert!(out.findings.is_empty());
    }
}
