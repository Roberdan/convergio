# AGENTS.md — convergio-coherence

For repo-wide rules see [../../AGENTS.md](../../AGENTS.md).

## Responsibility

Documentation/code coherence verifiers for Convergio — the `cvg
coherence` suite. Pure local checks; no daemon dependency.

Two verifiers ship today:

- `coherence::CoherenceCommand::Check` — ADR frontmatter against the
  ADR index (`docs/adr/README.md`) and `workspace.members`; markdown
  body drift detector for `convergio-*` identifiers and repo-relative
  paths.
- `coherence::CoherenceCommand::Routes` — diff actual axum routes
  under `crates/convergio-server/src/routes/` against documented
  routes in `ARCHITECTURE.md` and `AGENTS.md`.

Public entry points: [`run`] and [`CoherenceCommand`].

## Boundaries

- No HTTP. No SQLite. No process spawning.
- Walks the repo with `walkdir`; reads `Cargo.toml` with `toml`; does
  not write any files.
- Verifiers must be agent-callable from any CLI/skill, not just
  `cvg`. The CLI hosts only a thin shim
  (`crates/convergio-cli/src/commands/coherence.rs`).
- All user-facing strings flow through `convergio-i18n` so verifier
  messages render in EN and IT.

## Invariants

- Add a new verifier as its own module + a new
  `CoherenceCommand` variant. Do not let any single file exceed the
  300-line cap (CONSTITUTION § 13).
- Any new `convergio-*` allowlist entry in [`body`] must cite the
  ADR or release artefact that justifies it.
- Tests are colocated in `#[cfg(test)] mod tests { ... }` blocks per
  module — no shared fixtures across files.

## ADRs

- `docs/adr/0040-split-coherence-into-its-own-crate.md` — extraction
  rationale (mirrors ADR-0013 for durability).
- `docs/adr/0014-code-graph-tier3-retrieval.md`,
  `docs/adr/0015-documentation-as-derived-state.md` — the wider
  Tier-2/3 retrieval context the verifiers fit inside.
