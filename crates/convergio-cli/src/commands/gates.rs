use super::{Client, OutputMode};
use anyhow::Result;
use clap::Subcommand;
use convergio_api::GatePreconditionsCatalog;
use convergio_i18n::Bundle;

#[derive(Subcommand)]
pub enum GatesCommand {
    Show {
        #[arg(long)]
        gate: Option<String>,
    },
}

pub async fn run(
    client: &Client,
    bundle: &Bundle,
    output: OutputMode,
    sub: GatesCommand,
) -> Result<()> {
    let GatesCommand::Show { gate } = sub;
    let mut catalog: GatePreconditionsCatalog = client.get("/v1/gates/preconditions").await?;
    if let Some(name) = gate.as_deref() {
        catalog.preconditions.retain(|p| p.gate == name);
    }

    match output {
        OutputMode::Human => {
            if catalog.preconditions.is_empty() {
                println!("{}", bundle.t("gates-list-empty", &[]));
                return Ok(());
            }
            println!(
                "{}",
                bundle.t_n("gates-list-header", catalog.preconditions.len() as i64)
            );
            for p in &catalog.preconditions {
                let reads = join_or_dash(&p.reads_evidence_kinds);
                let active = join_or_dash(&p.active_target_status);
                let refusals = join_or_dash(&p.refusal_reasons);
                println!(
                    "{}",
                    bundle.t(
                        "gates-list-line",
                        &[
                            ("gate", &p.gate),
                            ("reads", &reads),
                            ("active", &active),
                            ("refusals", &refusals),
                            (
                                "evidence_required",
                                if p.enforces_task_evidence_required {
                                    "true"
                                } else {
                                    "false"
                                },
                            ),
                        ],
                    )
                );
            }
        }
        OutputMode::Json | OutputMode::Plain => {
            println!("{}", serde_json::to_string_pretty(&catalog)?)
        }
    }
    Ok(())
}

fn join_or_dash(items: &[String]) -> String {
    if items.is_empty() {
        "-".to_string()
    } else {
        items.join(",")
    }
}
