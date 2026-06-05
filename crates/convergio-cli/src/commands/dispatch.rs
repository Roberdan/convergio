//! `cvg dispatch` — run one executor tick.

use super::Client;
use anyhow::Result;
use clap::ValueEnum;
use serde_json::{json, Value};

/// Dispatch executor mode.
#[derive(Clone, Copy, Debug, ValueEnum)]
pub enum ExecutorMode {
    /// Run the daemon's configured executor.
    Default,
    /// Tracker-only dry dispatch: do not spawn workers.
    None,
}

impl ExecutorMode {
    pub(crate) fn as_wire(self) -> &'static str {
        match self {
            Self::Default => "default",
            Self::None => "none",
        }
    }
}

/// Run the command.
pub async fn run(client: &Client, no_dispatch: bool, executor: ExecutorMode) -> Result<()> {
    let body: Value = client
        .post(
            "/v1/dispatch",
            &json!({"no_dispatch": no_dispatch, "executor": executor.as_wire()}),
        )
        .await?;
    println!("{}", serde_json::to_string_pretty(&body)?);
    Ok(())
}
