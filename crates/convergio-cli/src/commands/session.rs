//! Thin re-export so `cvg session ...` keeps working. The real
//! surface lives in [`convergio_cli_session`] (extracted to honour
//! CONSTITUTION § 13 per ADR-0041).
//!
//! Why a shim and not a direct re-export of the whole module: the
//! CLI defines its own [`super::OutputMode`] and `Client` (shared
//! across every subcommand) which are distinct from
//! [`convergio_cli_session::OutputMode`] and
//! [`convergio_cli_session::Client`]. The shim converts between the
//! two so the dispatcher in `main.rs` keeps the same call shape.

use super::{Client as CliClient, OutputMode};
use anyhow::Result;
use convergio_i18n::Bundle;

pub use convergio_cli_session::SessionCommand;

/// Dispatch a `cvg session ...` invocation to the session crate.
pub async fn run(
    client: &CliClient,
    bundle: &Bundle,
    output: OutputMode,
    cmd: SessionCommand,
) -> Result<()> {
    let session_client = convergio_cli_session::Client::new(client.base().to_string());
    convergio_cli_session::run(&session_client, bundle, to_session_output(output), cmd).await
}

fn to_session_output(o: OutputMode) -> convergio_cli_session::OutputMode {
    match o {
        OutputMode::Human => convergio_cli_session::OutputMode::Human,
        OutputMode::Json => convergio_cli_session::OutputMode::Json,
        OutputMode::Plain => convergio_cli_session::OutputMode::Plain,
    }
}
