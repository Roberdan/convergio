# AGENTS.md — convergio-coherence

For repo-wide rules see [../../AGENTS.md](../../AGENTS.md).

## Responsibility

Documentation/code coherence verifiers for Convergio — the `cvg
coherence` suite. Most verifiers are pure local checks; four
exceptions (`Agents`, `Handshake`, `ClosePostHoc`, `PlanExecution`)
exercise the local daemon HTTP surface.

Eight verifiers ship today:

- `coherence::CoherenceCommand::Check` — ADR frontmatter against the
  ADR index (`docs/adr/README.md`) and `workspace.members`; markdown
  body drift detector for `convergio-*` identifiers and repo-relative
  paths.
- `coherence::CoherenceCommand::Routes` — diff actual axum routes
  under `crates/convergio-server/src/routes/` against documented
  routes in `ARCHITECTURE.md` and `AGENTS.md`.
- `coherence::CoherenceCommand::Adrs` — cross-check ADR `status:`
  frontmatter against implementation reality.
- `coherence::CoherenceCommand::Agents` — flag merged PRs whose author
  skipped the multi-agent protocol (no `agent_registry` entry,
  no heartbeat in window, no coordination messages). Daemon-backed;
  unreachable daemon downgrades the check to advisory.
- `coherence::CoherenceCommand::Fleet` — cross-repo schema check on
  `~/.convergio/v3/fleet.toml` (missing paths, dangling
  `derives_from`, multiple `engine` roots, missing
  retrieval-golden fixtures).
- `coherence::CoherenceCommand::ClosePostHoc` — walk the daemon audit
  chain and surface `task.closed_post_hoc` rows (volume per agent /
  plan / reason). Requires the daemon.
- `coherence::CoherenceCommand::Handshake` — 2-session E2E smoke test
  of the multi-agent loop (register → publish → poll → ack).
  Requires the daemon at `--daemon` (default
  `http://127.0.0.1:8420`).
- `coherence::CoherenceCommand::PlanExecution` — per-plan mechanism
  compliance score (ADR-0044). Calls the daemon for tasks,
  evidence, agent registry, and bus state.

Public entry points: [`run`] and [`CoherenceCommand`].

## Boundaries

- No SQLite. No process spawning.
- Local verifiers (`Check`, `Routes`, `Adrs`, `Fleet`): walk the repo
  with `walkdir`, read `Cargo.toml` with `toml`, never write files.
- Daemon-backed verifiers (`Agents`, `Handshake`, `ClosePostHoc`,
  `PlanExecution`): pure `reqwest` client against the daemon, no
  in-process state.
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
  module — no shared fixtures across files. Where the unit tests
  alone push the host module past the 300-line cap, split them into
  a sibling `*_tests.rs` module (e.g. `handshake_tests.rs`,
  `adrs_tests.rs`).
- The handshake verifier has its own E2E test under
  `crates/convergio-coherence/tests/e2e_handshake.rs` that boots
  the server in-process and runs the full round-trip.

## ADRs

- `docs/adr/0040-split-coherence-into-its-own-crate.md` — extraction
  rationale (mirrors ADR-0013 for durability).
- `docs/adr/0014-code-graph-tier3-retrieval.md`,
  `docs/adr/0015-documentation-as-derived-state.md` — the wider
  Tier-2/3 retrieval context the verifiers fit inside.
