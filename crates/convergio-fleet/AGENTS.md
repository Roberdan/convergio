# AGENTS.md — convergio-fleet

For repo-wide rules see [../../AGENTS.md](../../AGENTS.md). For the
decision behind this crate see
[../../docs/adr/0038-fleet-retrieval-cross-repo-graph.md](../../docs/adr/0038-fleet-retrieval-cross-repo-graph.md).

This crate owns the **fleet abstraction layer** for Convergio v4 (F2
scope): the `fleet.toml` config schema, the `fleet_repos` / `fleet_plans`
/ `fleet_plan_repos` database tables, and the cross-repo orchestration
primitives. It does **not** bundle graph building or embeddings — those
live in `convergio-graph` and `convergio-embed` respectively.

## Invariants

- **Config is schema only.** `config.rs` contains typed structs that
  mirror `fleet.toml`. It never reads from disk itself — callers
  do `toml::from_str(&std::fs::read_to_string(...)?)?`.
- **`FleetStore` is the single write path for `fleet_repos`.**
  No other crate inserts into or updates fleet tables directly.
- **Migration range 800-899** is exclusively reserved for
  `convergio-fleet` (ADR-0003). The next migration (`0801_…`) must
  not skip numbers.
- **`set_ignore_missing(true)` in the migrator** allows this crate to
  share the `_sqlx_migrations` table with sibling crates (same pattern
  as `convergio-embed`).
- **No business logic in `config.rs`.** Defaults live in serde
  `default = "fn_name"` annotations; no methods compute derived state
  on `FleetConfig`. Derivation belongs in callers (`convergio-server`,
  `convergio-cli`).
- **Fleet plans are stubs in F2.** `fleet_plans` and
  `fleet_plan_repos` tables are created but the full orchestration
  API ships in F3. Do not add half-finished orchestration paths —
  land them complete or defer to F3.
- **No `unwrap()` / `expect()` in `src/`** (tests exempted). Use `?`
  throughout.

## Module layout

| File | Owns |
|------|------|
| `config.rs`  | [`FleetConfig`], [`RepoEntry`], [`RepoRole`], defaults |
| `store.rs`   | [`FleetStore`] — CRUD over `fleet_repos` |
| `migrate.rs` | Migration runner (range 800-899, ADR-0003) |
| `error.rs`   | [`FleetError`] |

## Tests

- Per-module `#[cfg(test)] mod tests` in `config.rs` (serde
  roundtrips) and `store.rs` (real tempdir SQLite via `convergio-db`).
- `tests/fleet_e2e.rs` exercises init + store against a tempdir pool.
- No mocked DB — follow the existing integration-test discipline.

## F2 deliverables for this crate

- [x] `fleet.toml` schema (§ 5.6) — `config.rs`
- [x] Migration `0800_fleet.sql` — `fleet_repos`, `fleet_plans`,
  `fleet_plan_repos`
- [x] `FleetStore::add_repo`, `list_repos`, `get_repo`,
  `mark_built`, `set_enabled`, `remove_repo`
- [ ] HTTP routes `/v1/fleet/repos` — land in `convergio-server`
- [ ] CLI `cvg fleet add/ls` — land in `convergio-cli`
- [ ] Cross-repo similarity edges — F2 later tasks

## Crate stats

The block below is rewritten by `cvg docs regenerate` (ADR-0015) —
do not edit between the markers.

<!-- BEGIN AUTO:crate_stats -->
**`convergio-fleet` stats:** 14 `*.rs` files / 36 public items / 2397 lines (under `src/`).

Files approaching the 300-line cap:
- `src/batch.rs` (287 lines)
- `src/similar.rs` (269 lines)
- `src/patterns.rs` (262 lines)
- `src/store.rs` (262 lines)
<!-- END AUTO -->
