//! Thin re-export so `cvg coherence` keeps working. The real surface
//! lives in [`convergio_coherence`] (extracted to honour
//! CONSTITUTION § 13 per ADR-0040).
//!
//! Why a shim and not a direct re-export of the whole module: the
//! CLI defines its own [`super::OutputMode`] (shared across every
//! subcommand) which is distinct from
//! [`convergio_coherence::OutputMode`]. The shim converts between
//! the two so the dispatcher in `main.rs` keeps the same call shape.

use super::OutputMode;
use anyhow::Result;
use convergio_i18n::Bundle;

pub use convergio_coherence::CoherenceCommand;

/// Dispatch a `cvg coherence ...` invocation to the verifier crate.
pub async fn run(bundle: &Bundle, output: OutputMode, cmd: CoherenceCommand) -> Result<()> {
    convergio_coherence::run(bundle, to_coherence_output(output), cmd).await
}

fn to_coherence_output(o: OutputMode) -> convergio_coherence::OutputMode {
    match o {
        OutputMode::Human => convergio_coherence::OutputMode::Human,
        OutputMode::Json => convergio_coherence::OutputMode::Json,
        OutputMode::Plain => convergio_coherence::OutputMode::Plain,
    }
}
