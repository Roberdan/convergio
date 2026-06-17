//! `cvg purpose` — register and list immutable processing purposes
//! (ADR-0054 §B). A purpose declares WHY data may be accessed; it is the
//! upstream complement to the capability bucket's WHAT (ADR-0008). The CLI
//! stays a thin HTTP client; the daemon owns the registry and its
//! immutability rule. Every subcommand respects `--output human|json|plain`.

use super::{Client, OutputMode};
use anyhow::Result;
use clap::Subcommand;
use serde_json::{json, Value};

/// `cvg purpose` subcommand surface.
#[derive(Subcommand)]
pub enum PurposeCommand {
    /// Register a new immutable purpose. Re-declaring a label is refused.
    Register {
        /// Unique purpose label (e.g. `student-records`).
        label: String,
        /// Free-form description of the declared intent.
        #[arg(long, default_value = "")]
        description: String,
        /// Plan that declares this purpose (provenance only).
        #[arg(long)]
        plan: Option<String>,
    },
    /// List every registered purpose.
    List,
}

/// Dispatch a `cvg purpose` subcommand.
pub async fn run(client: &Client, output: OutputMode, sub: PurposeCommand) -> Result<()> {
    match sub {
        PurposeCommand::Register {
            label,
            description,
            plan,
        } => {
            let body = json!({
                "label": label,
                "description": description,
                "declared_by_plan": plan,
            });
            let p: Value = client.post("/v1/purposes", &body).await?;
            match output {
                OutputMode::Json => println!("{}", serde_json::to_string_pretty(&p)?),
                OutputMode::Plain => println!(
                    "{}\t{}",
                    p["id"].as_str().unwrap_or(""),
                    p["label"].as_str().unwrap_or("")
                ),
                OutputMode::Human => println!(
                    "registered purpose `{}` ({})",
                    p["label"].as_str().unwrap_or(""),
                    p["id"].as_str().unwrap_or("")
                ),
            }
        }
        PurposeCommand::List => {
            let rows: Value = client.get("/v1/purposes").await?;
            let empty = Vec::new();
            let list = rows.as_array().unwrap_or(&empty);
            match output {
                OutputMode::Json => println!("{}", serde_json::to_string_pretty(&rows)?),
                OutputMode::Plain => {
                    for p in list {
                        println!(
                            "{}\t{}",
                            p["id"].as_str().unwrap_or(""),
                            p["label"].as_str().unwrap_or("")
                        );
                    }
                }
                OutputMode::Human => {
                    if list.is_empty() {
                        println!("no purposes registered");
                    } else {
                        for p in list {
                            println!(
                                "- {} — {}",
                                p["label"].as_str().unwrap_or(""),
                                p["description"].as_str().unwrap_or("")
                            );
                        }
                    }
                }
            }
        }
    }
    Ok(())
}
