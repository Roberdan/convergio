use super::{Client, OutputMode};
use anyhow::Result;
use clap::Subcommand;
use convergio_durability::audit::VerifyReport;
use convergio_i18n::Bundle;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Subcommand)]
pub enum AuditCommand {
    Verify {
        #[arg(long)]
        from: Option<i64>,
        #[arg(long)]
        to: Option<i64>,
    },
    Compensate {
        seq: i64,
        #[arg(long)]
        apply: bool,
    },
}

#[derive(Deserialize, Serialize)]
struct CompensateResponse {
    source_seq: i64,
    source_transition: String,
    compensating_action: Value,
    applied: bool,
}

pub async fn run(
    client: &Client,
    bundle: &Bundle,
    output: OutputMode,
    cmd: AuditCommand,
) -> Result<()> {
    match cmd {
        AuditCommand::Verify { from, to } => {
            let mut path = String::from("/v1/audit/verify?");
            if let Some(f) = from {
                path.push_str(&format!("from={f}&"));
            }
            if let Some(t) = to {
                path.push_str(&format!("to={t}&"));
            }
            let r: VerifyReport = client.get(&path).await?;
            match output {
                OutputMode::Json => println!("{}", serde_json::to_string_pretty(&r)?),
                OutputMode::Plain | OutputMode::Human => {
                    if r.ok {
                        let count = r.checked.to_string();
                        println!("{}", bundle.t("audit-clean", &[("count", &count)]));
                    } else {
                        let seq = r.broken_at.unwrap_or_default().to_string();
                        println!("{}", bundle.t("audit-broken", &[("seq", &seq)]));
                    }
                }
            }
            Ok(())
        }
        AuditCommand::Compensate { seq, apply } => {
            let path = if apply {
                format!("/v1/audit/events/{seq}/compensate?apply=true")
            } else {
                format!("/v1/audit/events/{seq}/compensate")
            };
            let resp: CompensateResponse = client.get(&path).await?;
            match output {
                OutputMode::Json => println!("{}", serde_json::to_string_pretty(&resp)?),
                OutputMode::Plain | OutputMode::Human => {
                    let action = serde_json::to_string_pretty(&resp.compensating_action)?;
                    let seq_s = resp.source_seq.to_string();
                    if resp.applied {
                        println!(
                            "{}",
                            bundle.t(
                                "audit-compensate-applied",
                                &[("seq", &seq_s), ("transition", &resp.source_transition)],
                            )
                        );
                    } else {
                        println!(
                            "{}",
                            bundle.t(
                                "audit-compensate-dry-run",
                                &[("seq", &seq_s), ("transition", &resp.source_transition)],
                            )
                        );
                        println!(
                            "{}",
                            bundle.t("audit-compensate-apply-hint", &[("seq", &seq_s)])
                        );
                    }
                    println!(
                        "{}",
                        bundle.t("audit-compensate-action", &[("action", &action)])
                    );
                }
            }
            Ok(())
        }
    }
}
