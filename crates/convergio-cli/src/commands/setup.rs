//! `cvg setup` — initialize local user configuration.

use anyhow::{Context, Result};
use clap::{Subcommand, ValueEnum};
use convergio_i18n::Bundle;
use std::fs;
use std::path::PathBuf;

const DEFAULT_URL: &str = "http://127.0.0.1:8420";
const CONFIG_MARKER: &str = "# Convergio v3 local configuration";

/// Setup subcommands.
#[derive(Subcommand)]
pub enum SetupCommand {
    /// Generate local configuration.
    Init {
        /// Overwrite an existing config file.
        #[arg(long)]
        force: bool,
    },
    /// Generate adapter snippets for an agent host.
    Agent {
        /// Agent host to configure.
        host: AgentHost,
        /// Overwrite existing snippets for this host.
        #[arg(long)]
        force: bool,
    },
    /// Bootstrap the operator's fleet: detect ~/GitHub/convergio*
    /// repos, register them, run fleet build (P0-2).
    Fleet,
    /// Verify install correctness (ADR-0044): daemon up, version match,
    /// MCP registered, fleet bootstrapped, embed non-empty, loops running,
    /// registry active. Exits non-zero on any FAIL check.
    SelfCheck,
}

/// Supported agent hosts for generated snippets.
#[derive(Clone, Copy, Debug, PartialEq, Eq, ValueEnum)]
pub enum AgentHost {
    /// Claude Desktop / Claude Code compatible MCP config.
    Claude,
    /// Opus overnight wrapper (shell adapter that runs `cvg agent spawn`).
    OpusOvernight,
    /// GitHub Copilot local IDE integrations.
    CopilotLocal,
    /// GitHub Copilot cloud agent repository hint.
    CopilotCloud,
    /// Cursor.
    Cursor,
    /// Cline.
    Cline,
    /// Continue.
    Continue,
    /// Qwen or qwen-code shell-style agent.
    Qwen,
    /// Generic shell agent.
    Shell,
}

impl AgentHost {
    /// Stable host slug used in URLs, paths, and the registry `kind`
    /// field. Stable across releases.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Claude => "claude",
            Self::OpusOvernight => "opus-overnight",
            Self::CopilotLocal => "copilot-local",
            Self::CopilotCloud => "copilot-cloud",
            Self::Cursor => "cursor",
            Self::Cline => "cline",
            Self::Continue => "continue",
            Self::Qwen => "qwen",
            Self::Shell => "shell",
        }
    }
}

/// Run setup. With no subcommand, runs `init`.
pub async fn run(
    client: &super::Client,
    bundle: &Bundle,
    output: super::OutputMode,
    cmd: Option<SetupCommand>,
) -> Result<()> {
    let command = cmd.unwrap_or(SetupCommand::Init { force: false });
    match command {
        SetupCommand::Init { force } => init(bundle, force),
        SetupCommand::Agent { host, force } => agent(bundle, host, force),
        SetupCommand::Fleet => super::setup_fleet::run(client, output).await,
        SetupCommand::SelfCheck => super::setup_self_check::run(client, bundle, output).await,
    }
}

fn init(bundle: &Bundle, force: bool) -> Result<()> {
    let home = convergio_home()?;
    let adapters = home.join("adapters");
    fs::create_dir_all(&adapters).with_context(|| format!("create {}", adapters.display()))?;

    let config = home.join("config.toml");
    if config.exists() && !force && is_current_config(&config)? {
        println!(
            "{}",
            bundle.t(
                "setup-config-exists",
                &[("path", &config.display().to_string())]
            )
        );
        backfill_repo_path(bundle, &config)?;
    } else {
        if config.exists() {
            let backup = home.join("config.toml.v2.bak");
            fs::copy(&config, &backup)
                .with_context(|| format!("backup {} to {}", config.display(), backup.display()))?;
            println!(
                "{}",
                bundle.t(
                    "setup-config-backed-up",
                    &[("path", &backup.display().to_string())]
                )
            );
        }
        fs::write(&config, default_config())
            .with_context(|| format!("write {}", config.display()))?;
        println!(
            "{}",
            bundle.t(
                "setup-config-created",
                &[("path", &config.display().to_string())]
            )
        );
    }

    println!(
        "{}",
        bundle.t("setup-complete", &[("path", &home.display().to_string())])
    );
    println!("{}", bundle.t("setup-next-start", &[]));
    println!("{}", bundle.t("setup-next-doctor", &[]));
    Ok(())
}

fn agent(bundle: &Bundle, host: AgentHost, force: bool) -> Result<()> {
    let home = convergio_home()?;
    let dir = home.join("adapters").join(host.as_str());
    fs::create_dir_all(&dir).with_context(|| format!("create {}", dir.display()))?;

    write_snippet(&dir.join("mcp.json"), &mcp_snippet(host), force)?;
    super::setup_agent_prompt::write_prompt(&dir.join("prompt.txt"), host, force)?;
    write_snippet(
        &dir.join("README.txt"),
        &super::setup_readme::readme_snippet(host),
        force,
    )?;

    if matches!(host, AgentHost::Claude) {
        let skill_dir = dir.join("skill-cvg-attach");
        fs::create_dir_all(&skill_dir)
            .with_context(|| format!("create {}", skill_dir.display()))?;
        write_snippet(&skill_dir.join("SKILL.md"), claude_skill_md(), force)?;
        write_snippet(&skill_dir.join("cvg-attach.sh"), claude_skill_sh(), force)?;
        write_snippet(&dir.join("settings.json"), claude_settings_json(), force)?;
    }

    if matches!(host, AgentHost::OpusOvernight) {
        let path = dir.join("run.sh");
        write_snippet(&path, super::setup_scripts::opus_overnight_run_sh(), force)?;
        super::setup_scripts::make_executable(&path)?;
    }

    println!(
        "{}",
        bundle.t(
            "setup-agent-created",
            &[
                ("host", host.as_str()),
                ("path", &dir.display().to_string())
            ]
        )
    );
    println!("{}", bundle.t("setup-agent-copy", &[]));
    if matches!(host, AgentHost::Claude) {
        println!(
            "{}",
            bundle.t(
                "setup-agent-claude-extras",
                &[("path", &dir.display().to_string())]
            )
        );
    }
    Ok(())
}

fn write_snippet(path: &std::path::Path, content: &str, force: bool) -> Result<()> {
    if path.exists() && !force {
        return Ok(());
    }
    fs::write(path, content).with_context(|| format!("write {}", path.display()))
}

fn convergio_home() -> Result<PathBuf> {
    let home = std::env::var("HOME").context("HOME is not set")?;
    Ok(PathBuf::from(home).join(".convergio"))
}

/// Patch a current-shape config that is missing the `repo_path`
/// field. Idempotent: silent when the field is already present;
/// silent when the workspace cannot be resolved (the operator can
/// always re-run from inside the repo or set
/// `CONVERGIO_REPO_DIR`).
fn backfill_repo_path(bundle: &Bundle, config: &std::path::Path) -> Result<()> {
    use super::setup_repo_path::{backfill, Outcome};
    let resolve = || {
        super::update_repo_root::resolve()
            .ok()
            .map(|p| p.display().to_string())
    };
    let outcome = backfill(config, resolve)
        .with_context(|| format!("backfill repo_path in {}", config.display()))?;
    if matches!(outcome, Outcome::Added) {
        println!(
            "{}",
            bundle.t(
                "setup-config-repo-path-added",
                &[("path", &config.display().to_string())]
            )
        );
    }
    Ok(())
}

fn default_config() -> String {
    let repo_line = match super::update_repo_root::resolve() {
        Ok(p) => format!("repo_path = \"{}\"\n", p.display()),
        Err(_) => String::new(),
    };
    format!(
        "{CONFIG_MARKER}\n\
         version = 1\n\
         url = \"{DEFAULT_URL}\"\n\
         db = \"sqlite://$HOME/.convergio/v3/state.db?mode=rwc\"\n\
         bind = \"127.0.0.1:8420\"\n\
         {repo_line}"
    )
}

fn mcp_snippet(host: AgentHost) -> String {
    let name = if matches!(host, AgentHost::CopilotCloud) {
        "convergio-local"
    } else {
        "convergio"
    };
    format!(
        "{{\n  \"mcpServers\": {{\n    \"{name}\": {{\n      \"type\": \"stdio\",\n      \"command\": \"convergio-mcp\",\n      \"args\": [\"--url\", \"{DEFAULT_URL}\"]\n    }}\n  }}\n}}\n"
    )
}

fn claude_skill_md() -> &'static str {
    include_str!("../../../../examples/skills/cvg-attach/SKILL.md")
}

fn claude_skill_sh() -> &'static str {
    include_str!("../../../../examples/skills/cvg-attach/cvg-attach.sh")
}

fn claude_settings_json() -> &'static str {
    "{\n  \"hooks\": {\n    \"SessionStart\": [\n      {\n        \"hooks\": [\n          {\n            \"type\": \"command\",\n            \"command\": \"bash ~/.claude/skills/cvg-attach/cvg-attach.sh\",\n            \"timeout\": 5,\n            \"async\": true\n          }\n        ]\n      }\n    ]\n  }\n}\n"
}

fn is_current_config(path: &std::path::Path) -> Result<bool> {
    let content = fs::read_to_string(path).with_context(|| format!("read {}", path.display()))?;
    Ok(content.contains(CONFIG_MARKER) && content.contains("version = 1"))
}
