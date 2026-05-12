# AGENTS.md — convergio-durability

This crate is **Layer 1**, the load-bearing core of Convergio. The
durability + audit story sells the product. Be careful here.

For repo-wide rules see the root [AGENTS.md](../../AGENTS.md).
This file only documents what diverges.

## Do-not-touch without explicit approval

| Path | Why |
|------|-----|
| `src/audit/hash.rs` | The hash function and `GENESIS_HASH` are the security boundary. Changing either invalidates every audit log in the wild. |
| `src/audit/canonical.rs` | Canonicalization rules are part of the hash input. A subtle change (e.g. number formatting) will break old chains while passing tests. |
| `src/audit/log.rs` (`AuditLog::append` order of operations) | The atomicity of "read tail → compute hash → insert" is what gives us tamper-evidence. Any change must keep the operation effectively serial. |
| `migrations/0001_init.sql` | Once shipped, never edit. Add a new migration. |

If you must touch these files:

1. Open an ADR explaining why.
2. Run `cargo test -p convergio-durability --test audit_tamper`. All
   6 tests must stay green.
3. Add new tests proving the new invariant.

## Adding a new gate

1. New file under `src/gates/<name>_gate.rs`.
2. Implement the `Gate` trait.
3. Register it in `gates::default_pipeline()` (mind the order — see
   the module doc).
4. Open an ADR if the gate is non-obvious.
5. Add a test under `tests/gates.rs`.

## Adding a new domain entity / table

1. New migration in `migrations/` with a free version in the
   1–100 range (see [ADR-0003](../../docs/adr/0003-migration-coexistence.md)).
2. New module under `src/store/`.
3. New variant in `audit::EntityKind` if the entity should be audited.
4. Re-export the new type from `lib.rs` if the HTTP layer needs it.

## File-size cap

300-line rule applies. The `src/audit/` and `src/gates/` directories
are split this way deliberately — keep them split, don't fold files
back together.

## Testing conventions

- Audit-related changes → extend `tests/audit_tamper.rs`.
- Gate changes → extend `tests/gates.rs`.
- Reaper changes → extend `tests/reaper.rs`.
- Cross-store / facade changes → consider an HTTP-level test in
  `convergio-server/tests/e2e_*.rs`.

## Known invariants

- Every state-changing method on [`Durability`] writes **exactly one**
  audit row, with one documented exception: `register_agent()` writes
  `agent.registered` on every call and additionally writes
  `agent.session_started` when the agent is new or its previous
  heartbeat is older than the dedup window
  (`SESSION_STARTED_DEDUP_MINUTES`, currently 30 min). The
  `agent.session_started` row is the signal counted by
  `/v1/status.telemetry.sessions_started_24h`.
- `audit_log.seq` is monotonic and gap-free (sqlx-side; the verifier
  does not currently check for gaps but the chain hashing makes gaps
  self-detecting since `prev_hash` would not match).
- `Pool` is `Clone` and cheap to clone — share the same one across
  stores; do not connect a second time.
- Future CRDT and workspace-visible state must enter through audited
  helpers. Do not add "temporary" direct writes around the facade.

## Crate stats

The block below is rewritten by `cvg docs regenerate` (ADR-0015) —
do not edit between the markers.

<!-- BEGIN AUTO:crate_stats -->
**`convergio-durability` stats:** 59 `*.rs` files / 173 public items / 8903 lines (under `src/`).

Files approaching the 300-line cap:
- `src/audit/action.rs` (296 lines)
- `src/facade_transitions.rs` (294 lines)
- `src/store/agent_queries.rs` (294 lines)
- `src/store/tasks.rs` (288 lines)
- `src/store/workspace_patch.rs` (270 lines)
- `src/store/plan_pr_links.rs` (263 lines)
- `src/store/workspace.rs` (259 lines)
- `src/audit/log.rs` (258 lines)
- `src/facade.rs` (256 lines)
- `src/store/agents.rs` (253 lines)
- `src/store/crdt_merge.rs` (252 lines)
- `src/model.rs` (250 lines)
<!-- END AUTO -->
