# convergio-coherence

Documentation/code coherence verifiers for Convergio — the `cvg
coherence` suite (ADR-0040). Most verifiers are pure local checks
with no daemon dependency; three exceptions (`agents`, `handshake`,
`close-post-hoc`, `plan-execution`) call the local daemon at
`--daemon` (default `http://127.0.0.1:8420`).

Eight verifiers ship today:

- `cvg coherence check` — ADR frontmatter, workspace membership,
  index status, and markdown body drift.
- `cvg coherence routes` — diff actual axum routes under
  `crates/convergio-server/src/routes/` against the documented
  surface in `ARCHITECTURE.md` / `AGENTS.md`.
- `cvg coherence adrs` — cross-check ADR `status:` frontmatter
  against implementation reality.
- `cvg coherence agents` — flag merged PRs whose author skipped the
  multi-agent protocol (registry / heartbeat / bus). Daemon-backed
  (advisory if the daemon is unreachable).
- `cvg coherence fleet` — cross-repo schema check on
  `~/.convergio/v3/fleet.toml`.
- `cvg coherence close-post-hoc` — surface bypass-the-gate volume by
  walking the daemon audit chain for `task.closed_post_hoc` rows.
- `cvg coherence handshake` — 2-session E2E smoke test of the
  multi-agent loop against a running daemon.
- `cvg coherence plan-execution` — per-plan mechanism compliance
  score (ADR-0044); calls the daemon for tasks, evidence, registry,
  and bus state.

Render the API:

```bash
cargo doc --open -p convergio-coherence
```

The `convergio-cli` crate hosts a thin shim
(`crates/convergio-cli/src/commands/coherence.rs`) that converts
the CLI's `OutputMode` into this crate's and dispatches to
`convergio_coherence::run`.
