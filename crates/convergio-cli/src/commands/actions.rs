use super::{Client, OutputMode};
use anyhow::Result;
use clap::Subcommand;
use convergio_i18n::Bundle;
use serde::{Deserialize, Serialize};

#[derive(Subcommand)]
pub enum ActionsCommand {
    List {
        #[arg(long)]
        capability: Option<String>,
    },
}

#[derive(Deserialize, Serialize)]
struct ActionsRegistry {
    schema_version: String,
    #[serde(default)]
    actions: Vec<ActionRow>,
}

#[derive(Deserialize, Serialize)]
struct ActionRow {
    name: String,
    capability: String,
    summary: String,
}

pub async fn run(
    client: &Client,
    bundle: &Bundle,
    output: OutputMode,
    sub: ActionsCommand,
) -> Result<()> {
    let ActionsCommand::List { capability } = sub;
    let mut doc: ActionsRegistry = client.get("/v1/api/actions").await?;
    if let Some(cap) = capability.as_deref() {
        doc.actions.retain(|a| a.capability == cap);
    }

    match output {
        OutputMode::Human => {
            if doc.actions.is_empty() {
                println!("{}", bundle.t("actions-list-empty", &[]));
                return Ok(());
            }
            println!(
                "{}",
                bundle.t_n("actions-list-header", doc.actions.len() as i64)
            );
            for a in &doc.actions {
                println!(
                    "{}",
                    bundle.t(
                        "actions-list-line",
                        &[
                            ("capability", &a.capability),
                            ("name", &a.name),
                            ("summary", &a.summary),
                        ],
                    )
                );
            }
        }
        OutputMode::Json | OutputMode::Plain => println!("{}", serde_json::to_string_pretty(&doc)?),
    }

    Ok(())
}
