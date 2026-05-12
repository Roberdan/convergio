# AGENTS.md — convergio-cli-session

For repo-wide rules see [../../AGENTS.md](../../AGENTS.md).

## Responsibility

Session lifecycle commands for Convergio — the `cvg session` suite.

Primary subcommands:

- `session::SessionCommand::Resume` — print a cold-start brief
  (daemon health, audit chain, active plan, top pending tasks, open
  PRs, optional graph context-pack).
- `session::SessionCommand::PreStop` — end-of-session safety net
  (PRD-001 § Artefact 4): walks a registry of pre-detach checks and
  refuses to detach when findings are present unless `--force`.

Hook wiring subcommands (host SessionStart / PreToolUse):

- `session::SessionCommand::RegisterAndPoll` — register + heartbeat +
  poll each active plan inbox.
- `session::SessionCommand::HeartbeatSinceLastTurn` — best-effort,
  throttled heartbeat for “still alive” telemetry.

Public entry points: [`run`] and [`SessionCommand`].

## Boundaries

- HTTP only via the `Client` injected from the host CLI; no direct
  daemon URL hard-coding inside the crate.
- Shells out to `gh`, `git`, and `curl` for PR + worktree + daemon
  visibility (the pre-stop checks); every shell-out is conservative —
  failures collapse to `Pass`, never to a brick wall. The `curl`
  shell-out in `check_1_plan_pr_drift` lets a sync-trait `Check`
  hit the daemon without dragging in an async runtime; it disappears
  the day the `Check` trait widens to async and the injected
  `Client` can be used directly.
- All user-facing strings flow through `convergio-i18n` so output
  renders in EN and IT (`session-*` Fluent keys).
- The CLI hosts only a thin shim
  (`crates/convergio-cli/src/commands/session.rs`).

## Invariants

- Add a new pre-stop check as its own module under `checks/` and
  register it in `pre_stop::registry()`. Do not let any single
  file exceed the 300-line cap (CONSTITUTION § 13).
- Public `Client` and `OutputMode` types are re-exported from the
  CLI; downstream skills depend on this crate, not on the CLI.
- Fluent keys keep the `session-*` prefix — they are part of the
  observable surface.

## ADRs

- `docs/adr/0041-split-session-into-its-own-crate.md` — extraction
  rationale (mirrors ADR-0040 for coherence).
- `docs/adr/0040-split-coherence-into-its-own-crate.md`,
  `docs/adr/0013-durability-split.md` — sibling cross-cutting
  splits.
- `docs/adr/0014-code-graph-tier3-retrieval.md` — Tier-3 graph
  context-pack consumed by `session resume --task-id`.
