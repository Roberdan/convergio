//! `cvg capability install` — advisory install hints for named capabilities.
//!
//! This is an **informational** command: it prints the install hint and URL
//! for a named capability, then suggests the env-var wiring. It never
//! shell-outs automatically (security constraint: the gate system avoids
//! automated shell execution).

use anyhow::{bail, Result};
use convergio_a11y_axe::capability_descriptor;

/// Print the install hint for the named capability.
///
/// Currently only `a11y-axe` is supported. Unknown names return an error.
pub fn run_install_capability(name: &str) -> Result<()> {
    match name {
        "a11y-axe" => print_a11y_axe_hint(),
        other => {
            bail!("unknown capability `{other}`; only `a11y-axe` is supported by this command")
        }
    }
}

fn print_a11y_axe_hint() -> Result<()> {
    let descriptor: serde_json::Value = serde_json::from_str(capability_descriptor())
        .map_err(|e| anyhow::anyhow!("failed to parse a11y-axe descriptor: {e}"))?;

    let platform_key = current_platform();
    let version = descriptor["version"].as_str().unwrap_or("unknown");
    let env_var = descriptor["env_var"]
        .as_str()
        .unwrap_or("CONVERGIO_A11Y_AXE_BIN");
    let description = descriptor["description"].as_str().unwrap_or("");

    println!("capability: a11y-axe v{version}");
    println!("  {description}");
    println!();

    if let Some(platform) = descriptor["platforms"].get(&platform_key) {
        let hint = platform["install_hint"].as_str().unwrap_or("");
        let url = platform["url"].as_str().unwrap_or("");
        println!("platform: {platform_key}");
        println!("  url:          {url}");
        println!();
        println!("run:");
        println!("  {hint}");
        println!();
        println!("then wire it up:");
        println!("  export {env_var}=$(which axe)");
    } else {
        println!("platform `{platform_key}` is not listed in the descriptor.");
        println!("Install @axe-core/cli manually via npm, then set:");
        println!("  export {env_var}=$(which axe)");
    }

    Ok(())
}

/// Returns the platform key used in the capability descriptor, e.g.
/// `macos-aarch64` or `linux-x86_64`.
fn current_platform() -> String {
    let os = std::env::consts::OS;
    let arch = std::env::consts::ARCH;
    format!("{os}-{arch}")
}
