//! Idempotent prompt bootstrap for `cvg setup agent <host>`.
//!
//! The adapter `prompt.txt` is user-modifiable and may already exist
//! from an earlier Convergio version. This helper patches an existing
//! file by injecting the Step 0 register-and-poll block when missing,
//! so operators can upgrade by re-running `cvg setup agent <host>`
//! without needing `--force`.

use super::setup::AgentHost;
use anyhow::{Context, Result};
use std::fs;
use std::path::Path;

const STEP0_MARKER: &str = "cvg session register-and-poll";

/// Write or patch `prompt.txt` for an adapter host.
///
/// - If `force` or the file is missing: write the full template.
/// - Otherwise: patch-in Step 0 register-and-poll if the marker is missing.
pub fn write_prompt(path: &Path, host: AgentHost, force: bool) -> Result<()> {
    if force || !path.exists() {
        fs::write(path, super::setup_prompts::prompt_snippet(host))
            .with_context(|| format!("write {}", path.display()))?;
        return Ok(());
    }

    let raw = fs::read_to_string(path).with_context(|| format!("read {}", path.display()))?;
    if raw.contains(STEP0_MARKER) {
        return Ok(());
    }

    let title = format!("# Convergio adapter — {}", host.as_str());
    let step0 = super::setup_prompts::step_zero_snippet(host);

    let patched = if let Some((first, rest)) = split_first_line(&raw) {
        if first.trim_end() == title {
            let rest = rest.trim_start_matches(['\n', '\r']);
            if rest.is_empty() {
                format!("{first}\n\n{step0}\n")
            } else {
                format!("{first}\n\n{step0}\n\n{rest}")
            }
        } else {
            format!("{title}\n\n{step0}\n\n{raw}")
        }
    } else {
        format!("{title}\n\n{step0}\n")
    };

    fs::write(path, patched).with_context(|| format!("patch {}", path.display()))
}

fn split_first_line(s: &str) -> Option<(&str, &str)> {
    let idx = s.find(['\n', '\r'])?;
    Some((&s[..idx], &s[idx..]))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn split_first_line_handles_simple_case() {
        let (a, b) = split_first_line("a\nb").expect("split");
        assert_eq!(a, "a");
        assert_eq!(b, "\nb");
    }
}
