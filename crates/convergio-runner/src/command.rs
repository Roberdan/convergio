//! `PreparedCommand` — serialised view of the subprocess to spawn.
//!
//! Kept separate from `std::process::Command` so tests can assert on
//! the argv + stdin shape without invoking a real subprocess. The
//! caller (executor / `cvg agent spawn`) materialises a real
//! `Command` via [`PreparedCommand::into_std_command`].

use std::ffi::OsString;
use std::path::PathBuf;
use std::process::{Command, Stdio};

/// What a runner produces: argv, working dir, and the prompt that
/// will be piped on stdin (vendor CLIs read very long prompts off
/// stdin instead of argv to avoid OS-level argv truncation).
#[derive(Debug, Clone)]
pub struct PreparedCommand {
    /// Binary to invoke (`claude`, `copilot`).
    pub program: OsString,
    /// Argv after the binary.
    pub args: Vec<OsString>,
    /// Working directory the agent runs in. Always a Convergio
    /// worktree under `.claude/worktrees/<branch>/`.
    pub cwd: PathBuf,
    /// Extra environment variables for the vendor CLI process.
    pub env: Vec<(String, String)>,
    /// Prompt fed to the CLI on stdin.
    pub stdin_prompt: String,
}

impl PreparedCommand {
    /// Materialise as a `std::process::Command` configured with
    /// `Stdio::piped()` on stdin so the caller can write the prompt.
    pub fn into_std_command(self) -> (Command, String) {
        let mut cmd = Command::new(&self.program);
        cmd.args(&self.args);
        for (key, value) in &self.env {
            cmd.env(key, value);
        }
        cmd.current_dir(&self.cwd);
        cmd.stdin(Stdio::piped());
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());
        (cmd, self.stdin_prompt)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn into_std_command_preserves_program_and_args() {
        let pc = PreparedCommand {
            program: OsString::from("/usr/bin/echo"),
            args: vec![OsString::from("-n"), OsString::from("hello")],
            cwd: PathBuf::from("/tmp"),
            env: vec![("EXAMPLE".into(), "1".into())],
            stdin_prompt: "ignored by echo".into(),
        };
        let (cmd, prompt) = pc.into_std_command();
        let dbg = format!("{cmd:?}");
        assert!(dbg.contains("echo"));
        assert!(dbg.contains("hello"));
        assert_eq!(prompt, "ignored by echo");
    }
}
