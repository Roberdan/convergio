---
id: 0002
status: accepted
date: 2026-04-26
topics: [layer-1, security, compliance]
related_adrs: []
touches_crates: []
last_validated: 2026-05-02
---

# 0002. Hash-chain the audit log for tamper-evidence

- Status: accepted
- Date: 2026-04-26
- Deciders: Roberto
- Tags: layer-1, security, compliance

## Context and Problem Statement

Customers in regulated AI (healthcare, finance) need an audit trail that:

1. Records every state transition with WHO did WHAT and WHEN.
2. Is **tamper-evident**: a malicious or buggy operator that mutates a row
   should be detectable by an external auditor.
3. Is **cheap**: not a blockchain, not Merkle proofs, not multi-party signing.

We want this property in the OSS core so local runs are auditable
without external infrastructure.

## Decision Drivers

- Compliance posture (HIPAA, SOC 2, FDA 21 CFR Part 11) needs an audit
  log that cannot be silently modified.
- Verification must be a cron job, not a workflow engine.
- We don't have a central key infrastructure and don't want one in MVP.

## Considered Options

1. **Plain audit table** (no chain) — fine for "what happened" queries,
   useless against tampering.
2. **Per-row signature** — every row signed with a server key. Strong but
   requires key rotation, doesn't detect deletions in the middle.
3. **Hash-chained log** — each row's `hash = sha256(prev_hash || canonical_json(payload))`.
   Detects tampering and deletions. No keys.
4. **Merkle tree + checkpoint** — overkill for MVP, deferred.

## Decision Outcome

Chosen option: **3 — Hash-chained log**. Single column `hash` on every
`audit_log` row, chained from a fixed genesis (`0x00..0`).

### Verification protocol

```
GET /v1/audit/verify[?from=<id>&to=<id>]
```

Recomputes hashes server-side and returns `{ ok: bool, broken_at_id: ?id }`.
External cron calls this hourly and alarms on `ok == false`.

### Canonical JSON

To avoid false positives from formatting drift, the payload is canonicalized
before hashing: keys sorted lexicographically, no whitespace, numbers in
shortest form.

Hardening tests added on 2026-05-02 clarify that "shortest form" means
the current Rust `serde_json::Number::to_string()` byte spelling, not RFC
8785/JCS normalization. Integer and float representations remain
distinct (`1` vs `1.0`), negative zero remains `-0.0`, and positive
exponents include the serializer's plus sign (for example, `1.23e+45`).
Changing these spellings is an audit hash semantic change and requires a
new ADR.

### Positive consequences

- Tamper-evidence with O(N) verification, no keys, no infrastructure.
- Works without external services.
- Easy to communicate ("hash chain like Git").

### Negative consequences

- Verification is O(N) — for very large audit logs we may need
  checkpointing. Tracked: future ADR.
- A row added between insert and chain-update is a bug surface.
  Mitigation: insert and chain-update happen in one DB transaction.

## Custom kinds

Originally only daemon-internal transitions wrote audit rows
(`task.in_progress`, `plan.created`, `evidence.attached`, etc.). The
P2-2 retrospective (2026-05-04 retro item A8) found that agent CLI
subcommands wanting to emit operational signals — `session.pre_stop`
checks, `cvg coherence` scans, retro boomerangs — had no path: they
fell back to `tracing::info!` and lost the hash-chain signal.

`POST /v1/audit/append` (P2-2) extends this ADR with **agent-emitted
custom rows**. The route is just another append: it goes through the
same `AuditLog::append` writer, hashes the same way, and verifies
through `GET /v1/audit/verify` exactly like daemon-written rows. No
new chain, no new schema.

The contract:

| Field | Rule |
|---|---|
| `kind` | Must match `^[a-z][a-z0-9_]*\.[a-z0-9_]+(\.[a-z0-9_]+)*$`. Examples: `myapp.session.pre_stop.check.1`, `cvg.coherence.scan`. |
| `kind` | Must NOT start with daemon-reserved prefixes (`task.`, `plan.`, `evidence.`, `crdt.`, `workspace.`, `capability.`) and must NOT be a reserved name (`agent.session_started`, `agent.retired`, `agent.retired_stale`). 422 `kind_reserved`. |
| `entity_kind` | Closed enum: `agent | task | plan | evidence | free`. `free` is the catch-all for non-Convergio entities. |
| `entity_id` | Required, non-empty. Opaque to the daemon. |
| `agent_id` | Optional. Use the registered agent identity when known. |
| `payload` | Must be a JSON object (scalars and arrays rejected with 422 `payload_not_object`) so downstream tools stay machine-readable. |

The `convergio.act audit_append` action wraps this route; agents
should prefer it over raw HTTP.

## Links

- Spec: [docs/spec/v3-durability-layer.md](../spec/v3-durability-layer.md) § "Layer 1 — Durability Core"
- Constitution: [CONSTITUTION.md](../../CONSTITUTION.md) § 7
- P2-2 retro: 2026-05-04 retrospective, A8 (agent-emitted audit rows)
