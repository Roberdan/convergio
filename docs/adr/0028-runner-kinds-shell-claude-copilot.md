---
id: 0028
status: accepted
date: 2026-05-02
topics: [layer3, supervisor, runners, adapters]
related_adrs: [0009, 0027]
touches_crates: [convergio-server, convergio-mcp]
last_validated: 2026-05-02
---

# 0028. `spawn_runner` accepts `shell`, `claude`, and `copilot` kinds

- Status: accepted
- Date: 2026-05-02
- Deciders: Roberdan
- Tags: layer3, supervisor, runners, adapters

## Context and Problem Statement

Until v0.3.0 the `POST /v1/agents/spawn-runner` route (`convergio.act`
action `spawn_runner`) accepted only `kind="shell"`. Any other value
returned `InvalidAgent { reason: "only the local shell runner is
supported" }`. This was honest but limiting: it forbade the daemon
from acknowledging that a Claude Code CLI invocation or a GitHub
Copilot CLI invocation is *also* "a local process started by a
shell". The honest mechanism (run a command, capture the PID, watch
it exit) is identical for all three.

Meanwhile, `cvg setup agent <claude|copilot-local>` already writes
adapter scaffolding under `~/.convergio/adapters/<kind>/`
(`mcp.json`, `prompt.txt`, `README.txt`, plus a Claude Code skill
under `claude/`). The convention exists, but the runner refused to
admit it.

## Decision Drivers

- **Honest taxonomy.** A registered agent's `kind` is the runtime
  shape that produced it. `shell`, `claude`, `copilot` are different
  shapes with different prompts and different outputs even when the
  underlying spawn is the same.
- **No new supervisor surface.** Layer 3
  (`convergio-lifecycle::Supervisor::spawn`) already handles arbitrary
  `command + args + env`. The expansion is purely API-layer
  validation + a friendlier metadata label.
- **Forward compatibility.** Wave 0b.2 was scheduled to add
  full Claude/Copilot adapters with their own supervisors. Doing so
  would be a breaking change. This ADR is the additive precursor:
  accept the *kinds*, dispatch through the existing supervisor,
  defer the adapter-specific process supervision to a future ADR if
  it ever proves necessary.
- **No scaffolding (CONSTITUTION P4).** The daemon must not lie about
  what it does. Accepting `kind="claude"` and routing through the
  shell supervisor is *not* scaffolding because it works end-to-end
  the moment the operator wires `~/.convergio/adapters/claude/run.sh`
  to invoke Claude Code (or any other binary). Whether that wrapper
  exists is the operator's concern, not the daemon's.

## Considered Options

1. **Accept an allow-list of kinds, dispatch identically (chosen).**
   Validate at API boundary against
   `KNOWN_RUNNER_KINDS = ["shell", "claude", "copilot"]`. Every kind
   constructs the same `SpawnSpec`. The supervisor and the watcher
   loop are unchanged. The agent registry's `metadata.runner` label
   is set per-kind so observers can tell adapters apart.
2. **Reject everything except shell, build runner adapters separately.**
   Wave 0b.2 plan, full implementation. Estimated 200+ LOC + a new
   process supervision channel + new tests. Honest but expensive,
   and it asks for a breaking change to the supervisor's `SpawnSpec`.
   Deferred indefinitely.
3. **Accept any free-form kind string.** Permissive but allows typos
   to silently produce agents with `kind="cluade"` that no observer
   can group on. Rejected.

## Decision Outcome

Chosen option **(1): allow-list + identical dispatch**.

The change touches three files:

- `crates/convergio-server/src/routes/agents.rs` — `KNOWN_RUNNER_KINDS`
  const, validation against it, `runner_label(kind)` helper that
  feeds `agents.metadata.runner`.
- `crates/convergio-mcp/src/help.rs` — `spawn_runner` help schema
  documents the three kinds and the `~/.convergio/adapters/<kind>/`
  convention.
- `docs/adr/0028-runner-kinds-shell-claude-copilot.md...` — this file.

No supervisor change. No DB migration. No new dependency.

## Consequences

- **Positive.** A client can now call `convergio.act spawn_runner
  kind=claude command=~/.convergio/adapters/claude/run.sh ...` and
  get a registered agent + a tracked process + an in-progress task.
  The `~/.convergio/adapters/opus-overnight/run.sh` wrapper from the
  2026-05-01 overnight op now has a first-class `kind` to register
  under (`claude`).
- **Positive.** Observers (`cvg agent list`, `agent_processes` table)
  can group/filter by adapter kind without inferring it from the
  command path.
- **Negative.** The label vs runtime gap stays. A kind=claude agent
  whose command is `/bin/echo hi` is "a claude agent" by the label
  but not by behaviour. Operators are trusted not to mislabel — same
  trust the daemon already extends to free-form `command` values.
- **Negative.** A future Wave 0b.2 that introduces a *different*
  supervisor for one of these kinds will have to either (a) keep
  the API backwards compatible by routing on kind, or (b) write a
  new action. This ADR makes that decision easier, not harder.

## Validation

- `cargo fmt --all -- --check` clean.
- `RUSTFLAGS=-Dwarnings cargo clippy --workspace --all-targets -- -D warnings` clean.
- Existing E2E test
  `crates/convergio-server/tests/e2e_agents.rs::spawn_runner_registers_agent_claims_task_and_tracks_process`
  still green (kind=shell path).
- New E2E test
  `crates/convergio-server/tests/e2e_agents.rs::spawn_runner_accepts_claude_kind`
  proves a `kind=claude` invocation registers, spawns, transitions,
  and shows up in the registry with `metadata.runner=claude-shell-wrapper`.
- New E2E test asserts `kind="cluade"` (typo) is refused with a 4xx.

## Out of scope

- A real Claude/Copilot dispatcher loop that picks a `pending` task,
  builds the prompt, invokes the binary, and submits evidence.
  That's an adapter problem, not a supervisor problem; it lives in
  user-space (`~/.convergio/adapters/<kind>/run.sh`) until proven
  otherwise.
- Adding more kinds (`cursor`, `cline`, `qwen`). Trivial to add to
  `KNOWN_RUNNER_KINDS` when the matching adapter scaffolding is
  shipped by `cvg setup agent`.
- DB-level constraints on `agent_processes.kind`. The column stays
  unconstrained at SQLite layer; the API layer is the single source
  of truth for valid kinds.
