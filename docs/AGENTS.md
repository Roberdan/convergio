# AGENTS.md — docs

For repo-wide rules see [../AGENTS.md](../AGENTS.md).

This folder is **product memory**. Documentation must not claim
behavior that is not implemented or explicitly marked as future
work. Optimise for AI-agent consumption: imperative, lean, no
narrative bloat. The audit chain remembers everything; the docs
need only carry what the next agent must read to act.

## Tier-1 retrieval

Cold-start agents load these in order. Total budget ≤ ~5k tokens
before drilling deeper:

1. [`INDEX.md`](./INDEX.md) — auto-generated file map (Tier-1 entry).
2. [`agent-resume-packet.md`](./agent-resume-packet.md) — timeless protocol.
3. [`agent-protocol.md`](./agent-protocol.md) — MCP loop.
4. [`multi-agent-operating-model.md`](./multi-agent-operating-model.md) — swarm rules.

Drill into a single ADR, plan, or spec **only when the task demands it**.

## Rules

- Keep vision, ADR, roadmap, and user docs consistent.
- Mark future behavior as future; do not phrase it as shipped.
- When adding an ADR, update `docs/adr/README.md` (auto-regen via
  `cvg docs regenerate`, ADR-0015).
- Prefer one focused doc over scattering the same concept in many files.
- If an implementation changes the user workflow, update the relevant
  doc in the same PR.
- Follow [`agent-instruction-guidelines.md`](./agent-instruction-guidelines.md)
  for agent-optimized Markdown and prompt files.
- Do **not** cite live values that rot (version numbers, PR numbers,
  finding IDs) in timeless protocol docs. Run `cvg session resume`
  for live state instead.
- Long historical artefacts (friction logs, fresh-eyes test results,
  triage passes, public-readiness records) live under `plans/` and
  are excluded from Tier-1 retrieval via `.claudeignore` /
  `.cursorignore`. They are read on demand.

## Folder map

| Folder | Purpose |
|--------|---------|
| `adr/` | architecture decision records (MADR), monotonic numbering |
| `agents/` | per-host setup pointers (`cvg setup agent <host>`) |
| `plans/` | durable engineering plans, friction logs, post-hoc triage |
| `prd/` | product requirement docs |
| `reviews/` | adversarial / pre-PR review records |
| `spec/` | original specs and design docs (snapshots, not updated) |
| `templates/` | reusable templates (e.g. adversarial-challenge) |

## Load-bearing surface

- [`vision.md`](./vision.md) — product direction (read on demand).
- [`multi-agent-operating-model.md`](./multi-agent-operating-model.md) — how swarms use Convergio.
- [`agent-instruction-guidelines.md`](./agent-instruction-guidelines.md) — format rules.
- [`agent-protocol.md`](./agent-protocol.md) — MCP/tool loop.
- [`agent-resume-packet.md`](./agent-resume-packet.md) — cold-start packet.
- [`setup.md`](./setup.md) / [`release.md`](./release.md) — operator docs.
- [`wip-commit-template.md`](./wip-commit-template.md) — pause/handoff protocol.
- [`adr/`](./adr/) — decisions that constrain implementation.
