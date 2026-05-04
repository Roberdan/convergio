//! Thin re-export so `cvg pr ...` keeps working. The real surface
//! lives in [`convergio_cli_pr`] (extracted to honour CONSTITUTION
//! § Agent context budget; same pattern as ADR-0041 for `cvg
//! session`).
//!
//! The CLI defines its own [`super::OutputMode`] / `Client` (shared
//! by every subcommand). The shim converts between those and the
//! mirrored types in `convergio_cli_pr` so the dispatcher in
//! `main.rs` keeps the same call shape.

use super::{Client as CliClient, OutputMode};
use anyhow::Result;
use convergio_i18n::Bundle;

pub use convergio_cli_pr::PrCommand;

/// Dispatch a `cvg pr ...` invocation to the pr crate.
pub async fn run(
    client: &CliClient,
    bundle: &Bundle,
    output: OutputMode,
    cmd: PrCommand,
) -> Result<()> {
    let pr_client = convergio_cli_pr::Client::new(client.base().to_string());
    convergio_cli_pr::run(&pr_client, bundle, to_pr_output(output), cmd).await
}

fn to_pr_output(o: OutputMode) -> convergio_cli_pr::OutputMode {
    match o {
        OutputMode::Human => convergio_cli_pr::OutputMode::Human,
        OutputMode::Json => convergio_cli_pr::OutputMode::Json,
        OutputMode::Plain => convergio_cli_pr::OutputMode::Plain,
    }
}
