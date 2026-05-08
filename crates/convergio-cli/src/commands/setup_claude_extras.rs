//! Claude-specific optional extras for `cvg setup agent claude`.

use anyhow::{Context, Result};
use std::fs;
use std::path::Path;

/// Optional runner adapter: an operator may point `spawn_runner` at
/// `~/.convergio/adapters/opus-overnight/run.sh` for scheduled runs.
pub(crate) fn write_opus_overnight_adapter(home: &Path, force: bool) -> Result<()> {
    let overnight_dir = home.join("adapters").join("opus-overnight");
    fs::create_dir_all(&overnight_dir)
        .with_context(|| format!("create {}", overnight_dir.display()))?;
    super::setup::write_snippet(
        &overnight_dir.join("run.sh"),
        opus_overnight_run_sh(),
        force,
    )
}

fn opus_overnight_run_sh() -> &'static str {
    include_str!("../../../../examples/adapters/opus-overnight/run.sh")
}
