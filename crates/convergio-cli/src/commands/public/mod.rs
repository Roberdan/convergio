//! `cvg public ...` — public transparency artefact generators.
//!
//! This module is intentionally file-IO only (no daemon calls): it consumes
//! a release-gate registry document produced elsewhere (W10) and emits a
//! static site tree that can be published as-is.

use super::OutputMode;
use anyhow::Result;
use clap::Subcommand;
use convergio_i18n::Bundle;

mod algorithms;
mod algorithms_render;
mod algorithms_schema;

/// Public transparency commands.
#[derive(Subcommand)]
pub(crate) enum PublicCommand {
    /// Public algorithm register (one page per AI Action).
    Algorithms {
        #[command(subcommand)]
        sub: algorithms::AlgorithmsCommand,
    },
}

pub(crate) async fn run(bundle: &Bundle, output: OutputMode, cmd: PublicCommand) -> Result<()> {
    match cmd {
        PublicCommand::Algorithms { sub } => algorithms::run(bundle, output, sub),
    }
}
