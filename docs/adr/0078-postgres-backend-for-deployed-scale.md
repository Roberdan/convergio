---
adr: "0078"
title: "Add a PostgreSQL backend for deployed multi-user scale"
status: proposed
date: 2026-06-06
deciders: ["Roberto D'Angelo"]
consulted: []
informed: ["all contributors"]
supersedes: []
superseded-by: []
related: ["0001", "0003", "0073", "0079", "0080"]
---

# ADR-0078 — Add a PostgreSQL backend for deployed multi-user scale

- **Status**: proposed
- **Date**: 2026-06-06
- **Deciders**: Roberto D'Angelo
- **Related**: ADR-0001 (four-layer architecture), ADR-0003 (per-crate
  migrations), ADR-0073 (EU-sovereign pivot), ADR-0079 (Azure
  single-tenant deployment), ADR-0080 (ontology-author)

## Context and Problem Statement

Convergio is SQLite-only by design: single-user, local-first, one
file under `~/.convergio/`. This is correct for the operator-on-a-laptop
runtime and is part of the sovereignty story (no server to run).

The end-to-end platform goal (build real verticals such as a university
Student Information System, ADR-0080) breaks that assumption: a deployed
vertical serves many concurrent human users (registrars, faculty,
students), holds millions of ontology object rows, and must survive
process restarts on a managed host. SQLite with a single writer and
file-locking is the wrong substrate there. We need a real RDBMS **without
abandoning** the local-first SQLite mode that defines the single-operator
product.

## Decision Drivers

- Concurrency: many simultaneous writers (enrollment, grading) — SQLite
  serializes writers.
- Scale: ontology object/link/property tables grow to millions of rows
  per tenant.
- Operability on Azure (ADR-0079): managed Postgres (Azure Database for
  PostgreSQL Flexible Server) gives backups, HA, point-in-time restore.
- Preserve local-first: `cvg` on a laptop must keep working with zero
  external services.
- Minimise blast radius: the audit hash-chain (ADR-0002) and migrations
  (ADR-0003) must behave identically on both backends.

## Considered Options

1. **Stay SQLite-only** — cap the product at single-user; verticals run
   degraded or not at all.
2. **Replace SQLite with Postgres everywhere** — drop local-first;
   every operator must run a database.
3. **Dual-backend behind the existing `sqlx` layer** — SQLite is the
   default local runtime; Postgres is an opt-in backend selected by the
   `db` URL, enabled for deployed verticals. *(chosen)*

## Decision Outcome

Chosen option: **Option 3 — dual backend behind `sqlx`**. This ADR
**amends, but does not delete, the "SQLite-only" rule**: SQLite remains
the canonical local-first default; PostgreSQL becomes a supported,
opt-in backend for deployed single-tenant verticals.

Implementation contract:

- `convergio-db` exposes a backend-agnostic pool; the backend is chosen
  from the `db` URL scheme (`sqlite://…` vs `postgres://…`).
- Migrations (ADR-0003) ship in both SQLite and Postgres dialects per
  crate, validated by the same migration runner. No raw SQL that only
  one engine accepts in shared code paths.
- The audit hash-chain, gates, and ontology stores must pass the **same
  test suite** against both backends (CI matrix: `sqlite` + `postgres`).
- No connection to a remote control plane is introduced. Postgres is the
  operator's own database in the operator's own deployment (ADR-0079) —
  the sovereignty rule (P6) is about *control*, not about *which file
  format stores the bytes*.
- Local-first parity test: `cvg` against `sqlite://` must pass the full
  suite with zero external services running.

### Positive consequences

- Verticals can serve real institutional load.
- Managed Postgres on Azure EU gives HA/backup/PITR for free.
- One code path, two backends — no fork.

### Negative consequences

- Every migration is now written twice (SQLite + Postgres dialect).
- CI must run a Postgres matrix leg → slower CI.
- Some SQLite-isms (e.g. dynamic typing, `INSERT OR REPLACE`) must be
  rewritten portably.

## Pros and Cons of the Options

### Option 1 — stay SQLite-only
- 👍 Zero work, maximal simplicity.
- 👎 Kills the platform thesis; verticals cannot be deployed for real.

### Option 2 — Postgres everywhere
- 👍 One backend to maintain.
- 👎 Destroys local-first / "no server to run" — a P6 regression.

### Option 3 — dual backend (chosen)
- 👍 Keeps both audiences; selection is a URL scheme.
- 👎 Double-dialect migrations and a CI matrix cost.

## Links

- Related ADRs: ADR-0001, ADR-0002, ADR-0003, ADR-0073, ADR-0079, ADR-0080
- Constitution: P6 (sovereignty by construction) — clarified as a
  control property, not a storage-format property.
