//! Script snippets + small fs helpers for `cvg setup`.
//!
//! Split from `setup.rs` to keep files under the 300-line cap.

use anyhow::{Context, Result};
use std::fs;
use std::path::Path;

pub(super) fn opus_overnight_run_sh() -> &'static str {
    "#!/usr/bin/env bash\n\
set -euo pipefail\n\
\n\
: \"${CONVERGIO_TASK_ID:?CONVERGIO_TASK_ID is required}\"\n\
: \"${CONVERGIO_AGENT_ID:=claude-opus-overnight-${USER:-unknown}-$$}\"\n\
\n\
DAEMON_URL=\"${CONVERGIO_DAEMON_URL:-${CONVERGIO_URL:-http://127.0.0.1:8420}}\"\n\
PROFILE=\"${CONVERGIO_PROFILE:-standard}\"\n\
\n\
ARGS=(--url \"${DAEMON_URL}\" agent spawn --task \"${CONVERGIO_TASK_ID}\" --runner \"claude:opus\" --agent-id \"${CONVERGIO_AGENT_ID}\" --profile \"${PROFILE}\")\n\
\n\
if [[ -n \"${CONVERGIO_MAX_BUDGET_USD:-}\" ]]; then\n\
  ARGS+=(--max-budget-usd \"${CONVERGIO_MAX_BUDGET_USD}\")\n\
fi\n\
if [[ -n \"${CONVERGIO_CWD:-}\" ]]; then\n\
  ARGS+=(--cwd \"${CONVERGIO_CWD}\")\n\
fi\n\
\n\
exec cvg \"${ARGS[@]}\"\n"
}

pub(super) fn make_executable(path: &Path) -> Result<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(path)
            .with_context(|| format!("stat {}", path.display()))?
            .permissions();
        perms.set_mode(0o755);
        fs::set_permissions(path, perms).with_context(|| format!("chmod {}", path.display()))?;
    }
    Ok(())
}
