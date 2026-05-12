//! Service-unit rendering helpers for `cvg service install`.
//!
//! Kept separate from `service.rs` so the orchestration file stays
//! well under the 300-line cap and the rendered launchd/systemd
//! output has its own focused test surface.

use std::path::Path;

/// Render the launchd plist for the user-level Convergio daemon.
pub(super) fn launchd_plist(convergio: &Path, home: &Path) -> String {
    let label = super::service::LABEL;
    let out = home.join(".convergio/convergio.log");
    let err = home.join(".convergio/convergio.err.log");
    let cargo_bin = home.join(".cargo/bin");
    // launchd starts processes with a minimal PATH (typically
    // /usr/bin:/bin) and an unstable cwd. Both bite `cvg graph build`,
    // which shells out to `cargo metadata`: that needs `cargo` on PATH
    // and a valid current_dir(). We extend PATH to include
    // `~/.cargo/bin` (the canonical install path) plus the usual
    // homebrew/system bins, and pin WorkingDirectory to $HOME so the
    // daemon always has a stable cwd. Closes friction-log F45.
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0"><dict>
  <key>Label</key><string>{label}</string>
  <key>ProgramArguments</key><array><string>{}</string><string>start</string></array>
  <key>RunAtLoad</key><true/>
  <key>KeepAlive</key><true/>
  <key>WorkingDirectory</key><string>{}</string>
  <key>EnvironmentVariables</key><dict>
    <key>PATH</key><string>{}:/opt/homebrew/bin:/usr/local/bin:/usr/bin:/bin</string>
    <key>HOME</key><string>{}</string>
  </dict>
  <key>StandardOutPath</key><string>{}</string>
  <key>StandardErrorPath</key><string>{}</string>
</dict></plist>
"#,
        convergio.display(),
        home.display(),
        cargo_bin.display(),
        home.display(),
        out.display(),
        err.display()
    )
}

/// Render the systemd user unit for the Convergio daemon.
pub(super) fn systemd_unit(convergio: &Path) -> String {
    format!(
        "[Unit]\nDescription=Convergio local daemon\n\n[Service]\nExecStart={} start\nRestart=on-failure\n\n[Install]\nWantedBy=default.target\n",
        convergio.display()
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn plist() -> String {
        launchd_plist(
            &PathBuf::from("/usr/local/bin/convergio"),
            &PathBuf::from("/Users/example"),
        )
    }

    #[test]
    fn launchd_plist_includes_cargo_bin_in_path_env() {
        let out = plist();
        assert!(
            out.contains("<key>PATH</key>"),
            "EnvironmentVariables.PATH missing — F45 fix would not apply"
        );
        assert!(
            out.contains("/Users/example/.cargo/bin"),
            "expected ~/.cargo/bin in PATH, got: {out}"
        );
    }

    #[test]
    fn launchd_plist_pins_working_directory_to_home() {
        let out = plist();
        assert!(out.contains("<key>WorkingDirectory</key><string>/Users/example</string>"));
    }

    #[test]
    fn launchd_plist_keeps_log_redirects() {
        let out = plist();
        assert!(out.contains("/Users/example/.convergio/convergio.log"));
        assert!(out.contains("/Users/example/.convergio/convergio.err.log"));
    }
}
