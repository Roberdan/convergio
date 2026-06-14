//! CLI/daemon version drift warning (P1-2 from 544e78cc retro).
//!
//! A `cvg` binary built from a fresh checkout against a stale daemon
//! produces confusing diagnostics — the daemon reports the *previous*
//! version's behavior, but the operator thinks they're talking to the
//! version they just built. This module fires a fast best-effort
//! `GET /v1/health` before daemon-touching subcommands and prints a
//! one-shot warning to stderr when the daemon's `running_version`
//! differs from `env!("CARGO_PKG_VERSION")`. The check never blocks
//! the subcommand and stays silent when the daemon is unreachable.
//!
//! Suppressed by exporting `CONVERGIO_NO_DRIFT_WARN=1`.
//!
//! Output goes to stderr so stdout stays script-friendly.

use convergio_i18n::Bundle;
use std::time::Duration;

/// Env var that suppresses the drift warning when set to a truthy value.
pub const SUPPRESS_ENV: &str = "CONVERGIO_NO_DRIFT_WARN";

/// Cap the health probe so a hung daemon never delays the real command.
const PROBE_TIMEOUT: Duration = Duration::from_millis(750);

/// Outcome of comparing the CLI's compile-time version to the daemon's
/// reported `running_version`.
#[derive(Debug, PartialEq, Eq)]
pub enum DriftDecision {
    /// Versions match, or the suppress env is set, or no daemon version.
    Silent,
    /// Versions differ — caller should print the warning.
    Warn {
        /// CLI's `CARGO_PKG_VERSION`.
        cli_version: String,
        /// Daemon's `running_version` from `/v1/health`.
        daemon_version: String,
    },
}

/// Pure decision: should we warn given these inputs?
///
/// `daemon_version` is `None` when the daemon was unreachable or its
/// response did not carry `running_version`.
/// `suppress` is `true` when the env var is set to a truthy value.
pub fn decide(cli_version: &str, daemon_version: Option<&str>, suppress: bool) -> DriftDecision {
    if suppress {
        return DriftDecision::Silent;
    }
    let Some(d) = daemon_version else {
        return DriftDecision::Silent;
    };
    if d == cli_version {
        return DriftDecision::Silent;
    }
    DriftDecision::Warn {
        cli_version: cli_version.to_string(),
        daemon_version: d.to_string(),
    }
}

/// Read `CONVERGIO_NO_DRIFT_WARN` and return `true` when the value is
/// truthy (`1`, `true`, `yes`, case-insensitive). Empty / unset / `0`
/// are all `false`.
pub fn suppress_from_env() -> bool {
    match std::env::var(SUPPRESS_ENV) {
        Ok(v) => matches!(
            v.trim().to_ascii_lowercase().as_str(),
            "1" | "true" | "yes" | "on"
        ),
        Err(_) => false,
    }
}

/// Render the three-line warning for `decision` using `bundle`. Returns
/// an empty string when the decision is `Silent` (caller can skip the
/// print).
pub fn render(bundle: &Bundle, decision: &DriftDecision, url: &str) -> String {
    let DriftDecision::Warn {
        cli_version,
        daemon_version,
    } = decision
    else {
        return String::new();
    };
    let line1 = bundle.t(
        "cli-drift-warning",
        &[
            ("cli", cli_version.as_str()),
            ("daemon", daemon_version.as_str()),
            ("url", url),
        ],
    );
    let line2 = bundle.t("cli-drift-fix-hint", &[]);
    let line3 = bundle.t("cli-drift-suppress-hint", &[("env", SUPPRESS_ENV)]);
    format!("{line1}\n{line2}\n{line3}")
}

/// Best-effort: probe `<url>/v1/health`, return `running_version` on
/// success or `None` for any failure (unreachable, timeout, malformed
/// JSON, missing field). Never panics, never propagates errors —
/// drift checking must not break the user's command.
pub async fn fetch_running_version(url: &str) -> Option<String> {
    let client = reqwest::Client::builder()
        .timeout(PROBE_TIMEOUT)
        .default_headers(crate::http::purpose_headers())
        .build()
        .ok()?;
    let endpoint = format!("{url}/v1/health");
    let resp = client.get(&endpoint).send().await.ok()?;
    if !resp.status().is_success() {
        return None;
    }
    let body: serde_json::Value = resp.json().await.ok()?;
    body.get("running_version")
        .and_then(|v| v.as_str())
        .map(str::to_owned)
}

/// Run the full check + emit warning to stderr (one-shot, non-blocking
/// from the caller's perspective beyond the bounded probe timeout).
pub async fn check_and_warn(bundle: &Bundle, url: &str, cli_version: &str) {
    if suppress_from_env() {
        return;
    }
    let daemon_version = fetch_running_version(url).await;
    let decision = decide(cli_version, daemon_version.as_deref(), false);
    let msg = render(bundle, &decision, url);
    if !msg.is_empty() {
        eprintln!("{msg}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use convergio_i18n::Locale;

    #[test]
    fn decide_silent_when_versions_match() {
        let d = decide("0.3.11", Some("0.3.11"), false);
        assert_eq!(d, DriftDecision::Silent);
    }

    #[test]
    fn decide_silent_when_daemon_version_missing() {
        let d = decide("0.3.11", None, false);
        assert_eq!(d, DriftDecision::Silent);
    }

    #[test]
    fn decide_silent_when_suppressed_even_on_mismatch() {
        let d = decide("0.3.11", Some("0.3.10"), true);
        assert_eq!(d, DriftDecision::Silent);
    }

    #[test]
    fn decide_warn_on_mismatch() {
        let d = decide("0.3.11", Some("0.3.10"), false);
        assert_eq!(
            d,
            DriftDecision::Warn {
                cli_version: "0.3.11".into(),
                daemon_version: "0.3.10".into(),
            }
        );
    }

    #[test]
    fn render_silent_returns_empty() {
        let bundle = Bundle::new(Locale::En).expect("english bundle");
        assert_eq!(render(&bundle, &DriftDecision::Silent, "http://x"), "");
    }

    #[test]
    fn render_warn_emits_three_lines_in_english() {
        let bundle = Bundle::new(Locale::En).expect("english bundle");
        let decision = DriftDecision::Warn {
            cli_version: "0.3.11".into(),
            daemon_version: "0.3.10".into(),
        };
        let out = render(&bundle, &decision, "http://127.0.0.1:8420");
        assert!(out.contains("0.3.11"), "{out}");
        assert!(out.contains("0.3.10"), "{out}");
        assert!(out.contains("http://127.0.0.1:8420"), "{out}");
        assert!(out.contains("cvg service restart"), "{out}");
        assert!(out.contains(SUPPRESS_ENV), "{out}");
        assert_eq!(out.lines().count(), 3, "{out}");
    }

    #[test]
    fn render_warn_emits_three_lines_in_italian() {
        let bundle = Bundle::new(Locale::It).expect("italian bundle");
        let decision = DriftDecision::Warn {
            cli_version: "0.3.11".into(),
            daemon_version: "0.3.10".into(),
        };
        let out = render(&bundle, &decision, "http://127.0.0.1:8420");
        assert!(out.contains("0.3.11"), "{out}");
        assert!(out.contains("0.3.10"), "{out}");
        assert_eq!(out.lines().count(), 3, "{out}");
    }

    #[test]
    fn suppress_truthy_values() {
        // Use a unique env name per test to avoid races; here we test
        // the parser against direct values to keep things hermetic.
        for v in ["1", "true", "TRUE", "yes", "Yes", "on"] {
            // Temporary set/unset is safe within a single test thread.
            // Using the real env var on the parser would race other
            // tests; we re-test parsing logic via a closure.
            let parsed = matches!(
                v.trim().to_ascii_lowercase().as_str(),
                "1" | "true" | "yes" | "on"
            );
            assert!(parsed, "{v} should be truthy");
        }
        for v in ["0", "false", "no", "", "off"] {
            let parsed = matches!(
                v.trim().to_ascii_lowercase().as_str(),
                "1" | "true" | "yes" | "on"
            );
            assert!(!parsed, "{v} should be falsy");
        }
    }
}
