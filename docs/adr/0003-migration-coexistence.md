---
id: 0003
status: accepted
date: 2026-04-27
topics: [layer-1, layer-2, layer-3, migrations]
related_adrs: []
touches_crates: []
last_validated: 2026-04-30
---

# 0003. Per-crate migrations on a shared `_sqlx_migrations` table

- Status: accepted
- Date: 2026-04-27
- Deciders: Roberto, Claude (sessione 2)
- Tags: layer-1, layer-2, layer-3, migrations

## Context and Problem Statement

`convergio-durability` (Layer 1) owns the schema for `plans`, `tasks`,
`evidence`, `agents`, `audit_log`. `convergio-bus` (Layer 2) owns
`agent_messages`. `convergio-lifecycle` (Layer 3) owns
`agent_processes`. They share a single local SQLite database file,
because the runtime is intentionally local-first.

`sqlx::migrate!` produces a `Migrator` that:

1. Embeds all `*.sql` files from the given directory.
2. On `run()`, expects every previously-applied row in the bookkeeping
   table `_sqlx_migrations` to have a corresponding file in the embedded
   set, otherwise returns `MigrateError::VersionMissing`.

If we point three separate migrators at the same `_sqlx_migrations`
table, the second one to run sees rows from the first crate and refuses
to start. We hit this in sessione 2 — the bus migrator boot panicked
with `VersionMissing(1)` because durability had already inserted a row
at version 1.

## Decision Drivers

- Each crate must own its tables (CONSTITUTION § 6, "Server-enforced
  gates only" implies "the crate that defines the gate also owns the
  table").
- We do not want a "migrations" crate that aggregates everyone's SQL —
  it inverts the dependency direction and makes adding a new layer
  require touching shared code.
- We want a single `_sqlx_migrations` table so `sqlx migrate info`
  shows a coherent picture.
- We will not write a custom migration runner; sqlx is good enough.

## Considered Options

1. **One centralized migrator** — a `convergio-migrations` crate that
   embeds every SQL file from every crate. Reverses the dependency
   graph (low → high), violates "each crate owns its tables".
2. **Custom bookkeeping table per crate** — `_sqlx_migrations_durability`,
   `_sqlx_migrations_bus`, etc. Possible (sqlx allows
   `set_table_name`) but produces noisy `sqlite_master` output and
   makes ad-hoc queries harder.
3. **Per-crate migrator with `set_ignore_missing(true)`** — each crate
   has its own version range and refuses to complain about rows it
   didn't write.

## Decision Outcome

Chosen option: **3**.

### Convention: per-crate version range

| Crate | Range | Examples |
|-------|-------|----------|
| `convergio-durability` | 1 — 100 | `0001_init.sql` |
| `convergio-bus` | 101 — 200 | `0101_bus_init.sql` |
| `convergio-lifecycle` | 201 — 300 | `0201_lifecycle_init.sql` |
| (future Layer 4 crate) | 301 — 400 | TBD |
| `convergio-ops` | 401 — 500 | `0401_ops_workflow_engine.sql` |
| (future general extensions) | 501 — 599 | TBD |
| `convergio-graph` | 600 — 699 | `0600_graph_nodes_edges.sql` (ADR-0014) |
| `convergio-embed` *(proposed, ADR-0038 F1)* | 700 — 799 | `0700_embeddings.sql` |
| `convergio-fleet` *(proposed, ADR-0038 F2/F3)* | 800 — 899 | `0800_fleet.sql`, `0801_fleet_plans.sql` |
| `convergio-parse-multi` *(proposed, ADR-0038 F2)* | 900 — 999 | reserved (parser is mostly stateless; range held for future fixture/cache tables) |

A migration's `version` is the integer prefix on the filename (sqlx
convention). New crates must pick the next free hundred. Ranges marked
*proposed* are reserved by an open ADR and become active only when the
corresponding crate lands; until then the slot is held to prevent
collisions with other in-flight work.

### Implementation

Every migrator looks like:

```rust
pub async fn init(pool: &Pool) -> Result<()> {
    let mut migrator = sqlx::migrate!("./migrations");
    migrator.set_ignore_missing(true);
    migrator.run(pool.inner()).await?;
    Ok(())
}
```

`set_ignore_missing(true)` instructs sqlx to skip the
`VersionMissing` check — each migrator only runs files it owns and
ignores rows it doesn't recognize.

### Positive consequences

- Dependency graph stays acyclic (lower layers don't know about higher
  layers' migrations).
- Adding a new crate is a self-contained change — pick the next free
  hundred, write `0NNN_init.sql`, copy-paste the `init` boilerplate.
- Single bookkeeping table keeps `sqlx migrate info` and ad-hoc
  queries simple.

### Negative consequences

- We rely on developers picking unique version numbers manually. A
  future CI check (`xtask check-versions`) could enforce this, but for
  the MVP we trust review.
- `set_ignore_missing(true)` weakens sqlx's safety net: if a
  developer deletes a migration file in their crate's range, sqlx will
  not warn. Mitigation: never delete migration files (CONSTITUTION-style
  rule, enforced by review).

## Links

- Implementation:
  - `crates/convergio-durability/src/migrate.rs`
  - `crates/convergio-bus/src/migrate.rs`
  - `crates/convergio-lifecycle/src/migrate.rs`
- Supersedes informal note in
  `crates/convergio-bus/src/migrate.rs` doc-comment.
- Related: ADR-0001 (four-layer architecture), CONSTITUTION § 1.
