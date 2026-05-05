//! Thin re-export so `cvg plan run` keeps working. The orchestrator
//! lives in [`convergio_cli_plan_run`] (extracted to honour
//! CONSTITUTION § Agent context budget; same pattern as ADR-0040 for
//! `cvg pr` and ADR-0041 for `cvg session`).

use super::{Client as CliClient, OutputMode};
use anyhow::Result;
use convergio_i18n::Bundle;

pub(super) async fn run(
    client: &CliClient,
    bundle: &Bundle,
    output: OutputMode,
    id: &str,
    agent_id: Option<&str>,
    max_parallel: u8,
) -> Result<()> {
    let plan_client = convergio_cli_plan_run::Client::new(client.base().to_string());
    convergio_cli_plan_run::run(
        &plan_client,
        bundle,
        to_plan_run_output(output),
        id,
        agent_id,
        max_parallel,
    )
    .await
}

fn to_plan_run_output(o: OutputMode) -> convergio_cli_plan_run::OutputMode {
    match o {
        OutputMode::Human => convergio_cli_plan_run::OutputMode::Human,
        OutputMode::Json => convergio_cli_plan_run::OutputMode::Json,
        OutputMode::Plain => convergio_cli_plan_run::OutputMode::Plain,
    }
}
