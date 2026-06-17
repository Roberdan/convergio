//! `cvg capability` — local capability registry diagnostics.

use super::capability_install::run_install_capability;
use super::capability_types::*;
use super::{Client, OutputMode};
use anyhow::Result;
use clap::Subcommand;
use convergio_i18n::Bundle;
use serde_json::Value;
use std::path::PathBuf;

/// Capability subcommands.
#[derive(Subcommand)]
pub enum CapabilityCommand {
    /// List local capability registry rows.
    List,
    /// Install a signed local capability `.tar.gz` package.
    InstallFile {
        /// Path to the local capability package.
        package: PathBuf,
        /// Detached Ed25519 signature as hex.
        #[arg(long)]
        signature: String,
        /// Trusted key as key_id:hex_public_key. Repeat to pin more roots.
        #[arg(long = "trusted-key")]
        trusted_keys: Vec<String>,
    },
    /// Disable an installed capability.
    Disable {
        /// Capability name.
        name: String,
    },
    /// Substring search across locally-installed capabilities (W9
    /// local-only slice; remote registry is a follow-up — ADR-0057).
    Search {
        /// Lowercased substring matched against name / version / status.
        query: String,
    },
    /// Remove a disabled capability from disk and registry.
    Remove {
        /// Capability name.
        name: String,
    },
    /// Verify a detached Ed25519 capability package signature.
    VerifySignature {
        /// Capability name.
        #[arg(long)]
        name: String,
        /// Capability version.
        #[arg(long)]
        version: String,
        /// Package checksum in sha256:<hex> form.
        #[arg(long)]
        checksum: String,
        /// Path to manifest JSON.
        #[arg(long)]
        manifest: PathBuf,
        /// Detached Ed25519 signature as hex.
        #[arg(long)]
        signature: String,
        /// Trusted key as key_id:hex_public_key. Repeat to pin more roots.
        #[arg(long = "trusted-key")]
        trusted_keys: Vec<String>,
    },
    /// Manage the local trust store (ADR-0072).
    Trust {
        #[command(subcommand)]
        sub: super::capability_trust::TrustCommand,
    },
    /// Print install hint for a named capability (e.g. `a11y-axe`).
    ///
    /// Advisory only — prints the install command and env-var wiring;
    /// does not shell out automatically.
    Install {
        /// Capability name, e.g. `a11y-axe`.
        name: String,
    },
}

/// Run a capability registry subcommand.
pub async fn run(
    client: &Client,
    bundle: &Bundle,
    output: OutputMode,
    sub: CapabilityCommand,
) -> Result<()> {
    match sub {
        CapabilityCommand::List => list(client, bundle, output).await,
        CapabilityCommand::InstallFile {
            package,
            signature,
            trusted_keys,
        } => install_file(client, bundle, output, package, signature, trusted_keys).await,
        CapabilityCommand::Disable { name } => disable(client, bundle, output, &name).await,
        CapabilityCommand::Search { query } => search(client, output, &query).await,
        CapabilityCommand::Remove { name } => remove(client, output, &name).await,
        CapabilityCommand::VerifySignature {
            name,
            version,
            checksum,
            manifest,
            signature,
            trusted_keys,
        } => {
            verify_signature(
                client,
                bundle,
                output,
                VerifyArgs {
                    name,
                    version,
                    checksum,
                    manifest,
                    signature,
                    trusted_keys,
                },
            )
            .await
        }
        CapabilityCommand::Trust { sub } => super::capability_trust::run(output, sub).await,
        CapabilityCommand::Install { name } => Ok(run_install_capability(&name)?),
    }
}

async fn disable(client: &Client, bundle: &Bundle, output: OutputMode, name: &str) -> Result<()> {
    let cap: Capability = client
        .post(
            &format!("/v1/capabilities/{name}/disable"),
            &serde_json::json!({}),
        )
        .await?;
    match output {
        OutputMode::Json => println!("{}", serde_json::to_string_pretty(&cap)?),
        OutputMode::Plain => println!("disabled={}@{}", cap.name, cap.version),
        OutputMode::Human => println!(
            "{}",
            bundle.t(
                "capability-disabled",
                &[("name", &cap.name), ("version", &cap.version)],
            )
        ),
    }
    Ok(())
}

async fn remove(client: &Client, output: OutputMode, name: &str) -> Result<()> {
    let body: Value = client.delete(&format!("/v1/capabilities/{name}")).await?;
    match output {
        OutputMode::Json => println!("{}", serde_json::to_string_pretty(&body)?),
        OutputMode::Plain | OutputMode::Human => println!("removed={name}"),
    }
    Ok(())
}

async fn install_file(
    client: &Client,
    bundle: &Bundle,
    output: OutputMode,
    package: PathBuf,
    signature: String,
    trusted_keys: Vec<String>,
) -> Result<()> {
    let body = InstallFileRequest {
        package_path: package.display().to_string(),
        signature,
        trusted_keys: parse_trusted_keys(trusted_keys)?,
    };
    let capability: Capability = client.post("/v1/capabilities/install-file", &body).await?;
    match output {
        OutputMode::Json => println!("{}", serde_json::to_string_pretty(&capability)?),
        OutputMode::Plain => println!(
            "installed={}@{} status={}",
            capability.name, capability.version, capability.status
        ),
        OutputMode::Human => println!(
            "{}",
            bundle.t(
                "capability-installed",
                &[
                    ("name", &capability.name),
                    ("version", &capability.version),
                    ("status", &capability.status),
                ],
            )
        ),
    }
    Ok(())
}

async fn list(client: &Client, bundle: &Bundle, output: OutputMode) -> Result<()> {
    let body: Value = client.get("/v1/capabilities").await?;
    match output {
        OutputMode::Json => println!("{}", serde_json::to_string_pretty(&body)?),
        OutputMode::Plain => {
            let caps: Vec<Capability> = serde_json::from_value(body)?;
            println!("capabilities={}", caps.len());
        }
        OutputMode::Human => {
            let caps: Vec<Capability> = serde_json::from_value(body)?;
            render_human(bundle, &caps);
        }
    }
    Ok(())
}

async fn search(client: &Client, output: OutputMode, query: &str) -> Result<()> {
    let body: Value = client.get("/v1/capabilities").await?;
    let caps: Vec<Capability> = serde_json::from_value(body)?;
    let needle = query.to_lowercase();
    let hits: Vec<&Capability> = caps
        .iter()
        .filter(|c| {
            c.name.to_lowercase().contains(&needle)
                || c.version.to_lowercase().contains(&needle)
                || c.status.to_lowercase().contains(&needle)
        })
        .collect();
    match output {
        OutputMode::Json => println!("{}", serde_json::to_string_pretty(&hits)?),
        OutputMode::Plain => {
            for c in &hits {
                println!("{}\t{}\t{}", c.name, c.version, c.status);
            }
        }
        OutputMode::Human => {
            if hits.is_empty() {
                println!("no local capabilities match `{query}`");
            } else {
                println!(
                    "matched {} of {} local capabilities:",
                    hits.len(),
                    caps.len()
                );
                for c in &hits {
                    println!("  - {}@{} ({})", c.name, c.version, c.status);
                }
            }
        }
    }
    Ok(())
}

async fn verify_signature(
    client: &Client,
    bundle: &Bundle,
    output: OutputMode,
    args: VerifyArgs,
) -> Result<()> {
    let manifest: Value = serde_json::from_str(&std::fs::read_to_string(args.manifest)?)?;
    let body = VerifyRequest {
        name: args.name,
        version: args.version,
        checksum: args.checksum,
        manifest,
        signature: args.signature,
        trusted_keys: parse_trusted_keys(args.trusted_keys)?,
    };
    let verified: SignatureVerification = client
        .post("/v1/capabilities/verify-signature", &body)
        .await?;
    match output {
        OutputMode::Json => println!("{}", serde_json::to_string_pretty(&verified)?),
        OutputMode::Plain => println!(
            "verified={}@{} key={}",
            verified.name, verified.version, verified.key_id
        ),
        OutputMode::Human => println!(
            "{}",
            bundle.t(
                "capability-signature-ok",
                &[
                    ("name", &verified.name),
                    ("version", &verified.version),
                    ("key", &verified.key_id),
                ],
            )
        ),
    }
    Ok(())
}

fn parse_trusted_keys(values: Vec<String>) -> Result<Vec<TrustedKey>> {
    super::capability_types::parse_trusted_keys(values)
}
