---
status: accepted
date: 2026-05-26
deciders: Roberdan
---

# ADR-0064 — A11yGate phase 2 local-stub via external `axe` binary

## Context

W11 of `docs/plans/v1.0-production-ready.md` calls for full WCAG
coverage in `A11yGate` via axe-core. The original plan slot it behind
the remote capability registry (W9 follow-up) so that the gate could
fetch a vendored Node axe-core bundle on demand. The remote registry
is **not yet on main** — only its local-search predecessor (ADR-0061)
shipped. Blocking W11 on a not-yet-existing subsystem leaves the
phase-1 built-in subset (`a11y_gate.rs`) as the entire accessibility
story, which is below what CONSTITUTION § Sacred principle #3
requires for v1.0.

## Decision

Ship a **local-stub** wrapper crate `convergio-a11y-axe` that:

1. Reads `CONVERGIO_A11Y_AXE_BIN` for an absolute binary path.
2. When unset or stale, returns `AxeStatus::NotConfigured` — the
   caller (eventually `A11yGate`) falls back to phase-1 checks.
3. When set, spawns the binary with HTML on stdin, parses a JSON
   report from stdout, and returns `AxeStatus::Ok(AxeReport)` or
   `AxeStatus::Error(String)`.

The wrapper is a leaf crate: no `convergio-*` deps, no panics, no
implicit shell-out. The actual axe-core bundle stays out-of-tree.

## Consequences

**Positive:**

- Unblocks W11 partial without inventing a Node toolchain dependency.
- Keeps `convergio-durability` (currently 15 477 / 15 500 LOC) untouched.
- Provides a stable contract that the future remote registry slice
  can adopt by simply setting `CONVERGIO_A11Y_AXE_BIN` to a
  registry-resolved path.

**Negative:**

- Until `A11yGate` is wired to call this crate, W11 still ships zero
  user-facing axe-core checks. That wire-up is the next slice; it
  will likely need a separate ADR if it grows beyond ~30 LOC inside
  `convergio-durability`.
- Operators must opt in explicitly. Documented in
  `crates/convergio-a11y-axe/AGENTS.md`.

## Alternatives considered

- **Inline axe-core via Deno/Node embed**: rejected — adds a heavy
  runtime dependency and a security surface for a leaf check.
- **Wait for W9 remote registry**: rejected — no ETA on main and
  W11 is part of the v1.0 acceptance set.
- **Skip W11 entirely**: rejected — accessibility is a sacred
  principle, not an optional feature.

## Links

- W11 row in `docs/plans/v1.0-production-ready.md`.
- ADR-0051 (A11yGate phase 1).
- ADR-0061 (capability search local).
