//! CLI smoke tests — exercise the binary directly so we catch
//! regressions in the clap definitions / wiring without booting a
//! daemon.

use assert_cmd::Command;
use predicates::prelude::*;

fn cvg() -> Command {
    Command::cargo_bin("cvg").expect("cvg binary built")
}

#[test]
fn help_lists_known_subcommands() {
    let mut a = cvg().arg("--help").assert().success();
    for name in [
        "health",
        "status",
        "setup",
        "doctor",
        "plan",
        "task",
        "evidence",
        "crdt",
        "workspace",
        "mcp",
        "service",
        "demo",
        "audit",
        "actions",
        "gates",
    ] {
        a = a.stdout(predicate::str::contains(name));
    }
}

#[test]
fn subcommand_help_table() {
    // (subcommand, snippets that must appear in --help).
    for (sub, snippets) in [
        ("setup", &["init", "agent"][..]),
        ("doctor", &["--json"][..]),
        ("plan", &["create", "list", "get"][..]),
        ("audit", &["verify", "compensate"][..]),
        ("actions", &["list"][..]),
        ("gates", &["show"][..]),
        ("crdt", &["conflicts"][..]),
        ("workspace", &["leases"][..]),
        ("mcp", &["tail"][..]),
        ("service", &["install", "start", "status", "uninstall"][..]),
        (
            "task",
            &["create", "list", "get", "transition", "heartbeat"][..],
        ),
        ("evidence", &["add", "list"][..]),
    ] {
        let mut a = cvg().args([sub, "--help"]).assert().success();
        for s in snippets {
            a = a.stdout(predicate::str::contains(*s));
        }
    }
}

#[test]
fn plan_create_help_lists_project() {
    cvg()
        .args(["plan", "create", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("--project"));
}

#[test]
fn version_reports_cargo_pkg_version() {
    cvg()
        .arg("--version")
        .assert()
        .success()
        .stdout(predicate::str::contains(env!("CARGO_PKG_VERSION")));
}

#[test]
fn unreachable_daemon_table() {
    let url = ["--url", "http://127.0.0.1:1"];
    cvg()
        .args(url)
        .args(["--output", "json", "health"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("Could not reach daemon"));
    cvg()
        .args(url)
        .args(["--output", "plain", "doctor"])
        .assert()
        .failure()
        .stdout(predicate::str::contains("fail"));
    cvg()
        .args(["--lang", "en"])
        .args(url)
        .arg("health")
        .assert()
        .failure()
        .stderr(predicate::str::contains("Could not reach daemon"));
    cvg()
        .args(url)
        .args(["doctor", "--json"])
        .assert()
        .failure()
        .stdout(predicate::str::contains("\"ok\": false"))
        .stdout(predicate::str::contains("\"name\": \"daemon\""));
}

#[test]
fn plan_create_accepts_global_output_modes() {
    for mode in ["human", "json", "plain"] {
        cvg()
            .args([
                "--url",
                "http://127.0.0.1:1",
                "--output",
                mode,
                "plan",
                "create",
                "x",
            ])
            .assert()
            .failure();
    }
}

#[test]
fn mcp_tail_without_log_is_clear() {
    let home = tempfile::tempdir().expect("temp home");
    cvg()
        .env("HOME", home.path())
        .args(["mcp", "tail"])
        .assert()
        .success()
        .stdout(predicate::str::contains("No MCP log"));
}

#[test]
fn unknown_subcommand_fails_with_error() {
    cvg()
        .arg("nope")
        .assert()
        .failure()
        .stderr(predicate::str::contains("unrecognized").or(predicate::str::contains("invalid")));
}

#[test]
fn doctor_json_with_stale_pid_keeps_stderr_clean() {
    let home = tempfile::tempdir().expect("temp home");
    let config_dir = home.path().join(".convergio");
    std::fs::create_dir_all(&config_dir).expect("config dir");
    std::fs::write(config_dir.join("daemon.pid"), "999999").expect("pid file");
    cvg()
        .env("HOME", home.path())
        .args(["--url", "http://127.0.0.1:1", "doctor", "--json"])
        .assert()
        .failure()
        .stdout(predicate::str::contains("stale pid"))
        .stderr(
            predicate::str::contains("doctor found failing checks")
                .and(predicate::str::contains("kill:").not()),
        );
}

#[test]
fn setup_creates_config_and_adapters() {
    let home = tempfile::tempdir().expect("temp home");
    cvg()
        .env("HOME", home.path())
        .arg("setup")
        .assert()
        .success()
        .stdout(predicate::str::contains("Setup complete"));
    assert!(home.path().join(".convergio/config.toml").is_file());
    assert!(home.path().join(".convergio/adapters").is_dir());

    cvg()
        .env("HOME", home.path())
        .args(["setup", "agent", "claude"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Adapter snippets created"));
    let dir = home.path().join(".convergio/adapters/claude");
    assert!(dir.join("mcp.json").is_file());
    assert!(dir.join("prompt.txt").is_file());
    assert!(dir.join("README.txt").is_file());
}

#[test]
fn lang_flag_localizes_and_is_global_after_subcommand() {
    let url = "http://127.0.0.1:1";
    for args in [
        vec!["--lang", "it", "--url", url, "health"],
        vec!["health", "--lang", "it", "--url", url],
    ] {
        cvg()
            .args(args)
            .assert()
            .failure()
            .stderr(predicate::str::contains("Impossibile raggiungere"));
    }
}
