---
id: 0068
status: proposed
date: 2026-05-25
topics: [ontology, connectors, ingest, federated-query]
related_adrs: [0008, 0018, 0051, 0052, 0054]
touches_crates: [convergio-api]
last_validated: 2026-05-25
---

# 0068. Connector SDK + Federated Query

- Status: proposed
- Date: 2026-05-25
- Tags: ontology, connectors, ingest

## Context

Every vertical accelerator needs to pull data from upstream
systems (SIS, LMS, CRM, ERP, public registers, files) and map
them to ontology objects. Today each one writes bespoke code.
The municipality (ADR-0018) should ship the *building code* for
connectors — not the connectors themselves.

## Decision

Introduce a **Connector SDK** as a Rust trait + a YAML
declaration format, plus a minimal **federated query** surface
that lets read-only ontology queries fan out to live upstream
systems without first materialising everything locally.

### A. Connector SDK

1. **Three modes**, declared per connector:
   - `pull`: scheduled batch pull (cron-like, owned by the
     daemon scheduler).
   - `push`: webhook / inbound HTTP receiver registered with
     the daemon.
   - `stream`: long-lived subscription (kept inside the daemon
     process; reconnect/backoff in the SDK).
2. **Schema mapping declaration** (YAML).
   - Source fields → ontology `PropertyType` references.
   - Per-field comparator hint for ER (ADR-0066).
   - Required `purpose` (ADR-0065) and `capability_bundle`
     (ADR-0008) references — connectors do not run without
     explicit binding.
3. **Idempotency contract.**
   - Every ingested record carries a stable `source_key`; the
     SDK helper writes through typed actions (ADR-0063) that
     are idempotent on `(connector_id, source_key)`.
4. **Failure surface.**
   - Connectors emit structured failures to the bus
     (Layer 2); the daemon surfaces them as `plan`-attached
     incidents, never as silent drops.
5. **Capability-bundle distribution.**
   - Connectors ship inside Ed25519-signed bundles (ADR-0008).
     Core provides exactly two reference connectors — `csv`
     and `http-json` — used in tests and docs. No vertical
     connectors ship in core.

### B. Federated query (read-only, opt-in per `ObjectType`)

1. An `ObjectType` may declare `federated_source:
   {connector_id, query_template}`.
2. A query against such an `ObjectType` may include
   `--federated` to fan-out to the source connector for
   non-materialised properties.
3. **Refusal rules**: federated queries are refused when the
   active purpose (ADR-0065) is missing or does not cover
   "live upstream read"; refusal is audited.
4. **No write-through on federated queries.** Writes always go
   through typed actions and connectors' push/pull paths.

## Decision Drivers

- Building code, not buildings: vertical accelerators ship
  YAML + capability bundles, not glue scripts.
- Federated query closes the "we need to inspect the source
  before pulling everything" gap that would otherwise force
  unsafe full mirroring.
- Reference connectors prove the surface and keep CI honest.

## Considered Options

1. **No SDK; verticals own connectors entirely.** Rejected —
   drift; we have already seen `convergio-edu` re-invent the
   shape twice.
2. **Embed a full ETL engine.** Rejected — violates P2
   local-first and the urbanism posture; better to compose
   with existing tools at the vertical layer.
3. **This proposal — minimal SDK + reference connectors.**
   Accepted.

## Compliance Anchors

- P1 zero-debt: failures are structured, never swallowed.
- P2 local-first: SDK is in-process; federated queries open
  only to declared endpoints inside a declared purpose.
- ADR-0008 signing: connectors must be signed bundles.
- ADR-0065 purpose-binding: every connector must declare its
  purposes.

## Rollout

- W4 (plan *Ontology Platform W4: Workshop + Connector SDK*):
  - Trait + YAML schema + validator.
  - Reference `csv` + `http-json` connectors with golden
    tests.
  - Federated-query path + refusal gate.
  - CLI: `cvg connector list|register|run|status`.
  - MCP: `connector.*` actions in `actions.json`.

## Consequences

- Vertical accelerators converge on one connector
  declaration shape, simplifying audit.
- New CI gate (later): connectors without purposes or without
  `source_key` mapping fail validation.

## Alternatives left for verticals

- Domain-specific connectors (SIS, LMS, EHR, ERP) live in the
  vertical's capability bundles.
- Premium / cloud connectors are out of scope for core
  (P2 local-first).

## References

- ADR-0008 capability bundles
- ADR-0018 long-tail vertical accelerators
- ADR-0062 ontology runtime core
- ADR-0063 typed actions framework
- ADR-0065 provenance bundle + purpose registry
- ADR-0066 entity resolution
