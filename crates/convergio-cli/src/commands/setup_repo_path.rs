//! Backfill `repo_path` into an existing `~/.convergio/config.toml`.
//!
//! Pre-existing configs (created before the `repo_path` field was
//! added) silently miss the field, which makes `cvg update` fail
//! with "could not locate the Convergio workspace" the moment the
//! operator runs it from outside the repo. `cvg setup init` already
//! writes `repo_path` when it generates a config from scratch — this
//! module makes the same `cvg setup init` invocation also patch
//! existing configs. Idempotent: if `repo_path` is present, no-op.

use std::fs;
use std::path::Path;

/// Outcome of a backfill attempt.
#[derive(Debug, PartialEq, Eq)]
pub enum Outcome {
    /// Config already had `repo_path` — nothing to do.
    AlreadyPresent,
    /// `repo_path` was missing and we appended it.
    Added,
    /// Config did not exist (init will create it from scratch).
    NoConfig,
    /// Workspace root could not be resolved; we left the config as-is.
    NoWorkspace,
}

/// Append `repo_path = "<root>"` to `config_path` when:
/// - the file exists,
/// - it does not already declare `repo_path`,
/// - we can resolve a workspace root (env / config / walk-up).
///
/// The resolver is injected so tests can drive every branch without
/// touching the global filesystem layout.
pub fn backfill<P, F>(config_path: P, resolve: F) -> std::io::Result<Outcome>
where
    P: AsRef<Path>,
    F: FnOnce() -> Option<String>,
{
    let path = config_path.as_ref();
    if !path.exists() {
        return Ok(Outcome::NoConfig);
    }
    let text = fs::read_to_string(path)?;
    if has_repo_path(&text) {
        return Ok(Outcome::AlreadyPresent);
    }
    let Some(root) = resolve() else {
        return Ok(Outcome::NoWorkspace);
    };
    let mut updated = text;
    if !updated.ends_with('\n') {
        updated.push('\n');
    }
    updated.push_str(&format!("repo_path = \"{root}\"\n"));
    fs::write(path, updated)?;
    Ok(Outcome::Added)
}

/// `true` if the config already declares a non-empty `repo_path`
/// field. Comments are ignored so a `# repo_path = "/foo"` does not
/// suppress the backfill.
pub fn has_repo_path(text: &str) -> bool {
    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('#') {
            continue;
        }
        let Some(rest) = trimmed.strip_prefix("repo_path") else {
            continue;
        };
        let rest = rest.trim_start();
        let Some(rest) = rest.strip_prefix('=') else {
            continue;
        };
        let value = rest.trim().trim_matches('"').trim_matches('\'');
        if !value.is_empty() {
            return true;
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn make_config(dir: &Path, body: &str) -> std::path::PathBuf {
        let p = dir.join("config.toml");
        fs::write(&p, body).expect("write fixture");
        p
    }

    #[test]
    fn already_present_is_noop() {
        let tmp = tempdir().unwrap();
        let p = make_config(tmp.path(), "version = 1\nrepo_path = \"/already/here\"\n");
        let out = backfill(&p, || panic!("resolver must not run")).unwrap();
        assert_eq!(out, Outcome::AlreadyPresent);
        let after = fs::read_to_string(&p).unwrap();
        assert!(after.contains("/already/here"));
    }

    #[test]
    fn missing_field_appends_resolved_root() {
        let tmp = tempdir().unwrap();
        let p = make_config(tmp.path(), "version = 1\nurl = \"http://x\"\n");
        let out = backfill(&p, || Some("/tmp/wk".to_string())).unwrap();
        assert_eq!(out, Outcome::Added);
        let after = fs::read_to_string(&p).unwrap();
        assert!(after.contains("repo_path = \"/tmp/wk\""));
    }

    #[test]
    fn missing_field_with_no_workspace_leaves_file_alone() {
        let tmp = tempdir().unwrap();
        let p = make_config(tmp.path(), "version = 1\nurl = \"http://x\"\n");
        let out = backfill(&p, || None).unwrap();
        assert_eq!(out, Outcome::NoWorkspace);
        let after = fs::read_to_string(&p).unwrap();
        assert!(!after.contains("repo_path"));
    }

    #[test]
    fn missing_config_returns_no_config() {
        let tmp = tempdir().unwrap();
        let p = tmp.path().join("does-not-exist.toml");
        let out = backfill(&p, || Some("/tmp/wk".into())).unwrap();
        assert_eq!(out, Outcome::NoConfig);
    }

    #[test]
    fn commented_repo_path_does_not_block_backfill() {
        let tmp = tempdir().unwrap();
        let p = make_config(tmp.path(), "version = 1\n# repo_path = \"/old/comment\"\n");
        let out = backfill(&p, || Some("/tmp/wk".into())).unwrap();
        assert_eq!(out, Outcome::Added);
        let after = fs::read_to_string(&p).unwrap();
        assert!(after.contains("repo_path = \"/tmp/wk\""));
    }

    #[test]
    fn empty_repo_path_value_triggers_backfill() {
        let tmp = tempdir().unwrap();
        let p = make_config(tmp.path(), "version = 1\nrepo_path = \"\"\n");
        let out = backfill(&p, || Some("/tmp/wk".into())).unwrap();
        assert_eq!(out, Outcome::Added);
    }

    #[test]
    fn appended_line_terminates_with_newline_when_file_did_not() {
        let tmp = tempdir().unwrap();
        let p = make_config(tmp.path(), "version = 1\nurl = \"http://x\"");
        let out = backfill(&p, || Some("/tmp/wk".into())).unwrap();
        assert_eq!(out, Outcome::Added);
        let after = fs::read_to_string(&p).unwrap();
        assert!(after.ends_with('\n'));
        assert!(after.contains("\nrepo_path = \"/tmp/wk\"\n"));
    }
}
