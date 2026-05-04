# convergio-coherence

Documentation/code coherence verifiers for Convergio — the `cvg
coherence` suite (ADR-0040). Local-only, no daemon dependency.

Two verifiers ship today:

- `cvg coherence check` — ADR frontmatter, workspace membership,
  index status, and markdown body drift.
- `cvg coherence routes` — diff actual axum routes under
  `crates/convergio-server/src/routes/` against the documented
  surface in `ARCHITECTURE.md` / `AGENTS.md`.

Render the API:

```bash
cargo doc --open -p convergio-coherence
```

The `convergio-cli` crate hosts a thin shim
(`crates/convergio-cli/src/commands/coherence.rs`) that converts
the CLI's `OutputMode` into this crate's and dispatches to
`convergio_coherence::run`.
