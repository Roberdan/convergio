---
id: 0048
status: proposed
date: 2026-05-10
topics: [layer-1, audit]
related_adrs: [0002, 0011, 0026]
touches_crates: [convergio-durability, convergio-server]
last_validated: 2026-05-10
---

# 0048. Add compensating action types

- Status: proposed
- Date: 2026-05-10
- Tags: layer-1, audit

## Context and Problem Statement

Convergio’s audit log is append-only and hash-chained (ADR-0002), so we can always *see* what happened, but today we cannot mechanically *undo* certain known-safe side effects.

Operators need a standard way to ask the daemon: “given audit event `seq=N`, what is the compensating action, and please apply it while recording the application as a new audit row?”

## Decision Drivers

- Keep the audit log as the source of truth for “what happened” (ADR-0002).
- Prefer explicit, typed, narrow undo operations over ad-hoc DB writes.
- Preserve Layer 1 invariants: each state-changing call writes exactly one audit row.
- Make non-invertible actions explicit (return `None` with rationale), not silent.

## Considered Options

1. **Free-form undo via `audit.append`** — append an “undo happened” marker, but do not mutate state.
2. **Typed compensating actions derived from audit rows** — infer a small set of actions and apply their inverses through the durability facade.
3. **Generic DB-level inverse** — attempt to roll back any audit row by replaying older snapshots.

## Decision Outcome

Chosen option: **Option 2**, because it keeps compensation safe and explicit, reuses existing durability invariants, and does not pretend that every action has a clean inverse.

### What ships

- A typed `convergio_durability::audit::Action` inferred from selected daemon-owned audit transitions.
- `Action::compensate() -> Option<Action>` returning a mechanical inverse when available.
- Non-invertible actions return `None` and expose a short rationale (e.g. creation has no delete surface; destructive removals lose data).
- `GET /v1/audit/events/:seq/compensate` computes the compensating action (dry-run by default). Pass `?apply=true` to apply it; the applied operation is recorded as the normal fresh audit row emitted by the underlying durability method.

### Explicit non-inverses (examples)

- `plan.created`: plan deletion is not a supported daemon action.
- `task.created`: task deletion is not a supported daemon action.
- `evidence.removed`: removal is destructive; restoring would require persisting the original evidence payload in the audit row.

### Positive consequences

- Operators get a standard “boomerang” mechanism to recover from known-safe mistakes.
- Compensation is auditable: the undo itself is a first-class audit event.
- The system does not claim reversibility where it does not exist.

### Negative consequences

- `/v1/audit/events/:seq/compensate` is a side-effecting `GET`, which is semantically unusual. We accept this for now because it is explicitly an operator/audit tool and is not intended for web caching.

## Links

- Related ADRs: [0002](./0002-audit-hash-chain.md), [0011](./0011-thor-only-done.md), [0026](./0026-plan-wave-milestone-vocabulary.md)
