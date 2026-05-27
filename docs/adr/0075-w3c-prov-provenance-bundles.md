---
adr: "0075"
title: "W3C-PROV-JSON provenance bundles"
status: accepted
date: 2026-05-27
deciders: ["Roberto D'Angelo"]
consulted: []
informed: ["all contributors"]
supersedes: []
superseded-by: []
related: ["0073", "0051", "0002", "0074"]
---

# ADR-0075 — W3C-PROV-JSON provenance bundles

- **Status**: accepted
- **Date**: 2026-05-27
- **Supersedes**: —
- **Related**: ADR-0073 (EU-sovereign pivot), ADR-0051 (Ontology Runtime Core),
  ADR-0002 (audit chain), ADR-0074 (AGPL-3.0 relicense)

## Context

The EU-sovereign positioning (ADR-0073) and the GDPR-Art.-30 / AI-Act
Annex-IV deliverables in COMPLIANCE.md both require Convergio to
emit *machine-checkable* provenance for every ontology mutation:
who did it, when, on what evidence, under which declared purpose,
and which downstream entities were produced.

Today the audit chain (ADR-0002) tells us *what happened in the
daemon* (state transitions, evidence rows, gate refusals). It does
not, in a standards-shaped way, tell a regulator *why* a particular
ontology object now has its current value, or *which human/AI
agent* authorised the change. Regulators want PROV, not a custom
JSON shape.

## Decision

We will model Convergio's mutation history as
[W3C-PROV-JSON][prov-json] bundles, emitted from a new leaf crate
`convergio-provenance`. Every ontology write (and, in a later
phase, every audit row tagged `prov.relevant=true`) produces a
PROV bundle containing:

- one or more `Activity` nodes (the operation),
- one or more `Agent` nodes (human or AI actor, plus the daemon),
- one or more `Entity` nodes (the produced revision),
- `wasGeneratedBy`, `wasAssociatedWith`, and `used` relations.

[prov-json]: https://www.w3.org/TR/prov-json/

Bundles can be emitted for audit rows and chained as provenance audit
events by `convergio-durability`. HTTP lookup remains a later surface.

This ADR's scope: standardize on PROV-JSON, keep the serialization
crate leaf-only, ship working `emit_bundle()`/`to_prov_json()` APIs,
and wire audit rows into the hash chain via durability provenance events.

## Status of the implementation

| Area | Status |
|------|--------|
| Crate types + deterministic serde round-trip | **shipped** |
| `emit_bundle()` validation and relation construction | **shipped** |
| `to_prov_json()` serialization API | **shipped** |
| Workspace integration + cargo-deny allowance | **shipped** |
| Hook into `convergio-durability` audit chain | **shipped** |
| HTTP surface (`GET /v1/provenance/:seq`) | **planned** — follow-up |
| Hook into `convergio-ontology` upsert path | **planned** — follow-up |
| CLI surface (`cvg provenance show <seq>`) | **planned** — follow-up |
| Integration test | **shipped** |

## Alternatives considered

1. **Roll our own JSON shape.** Rejected: regulators ask for PROV
   by name. A custom shape forces every audit consumer to write a
   PROV adapter anyway.
2. **PROV-XML instead of PROV-JSON.** Rejected: JSON is the
   default audit format already used by Convergio; XML adds a
   second serialiser and an XSD dependency for zero downstream
   benefit.
3. **Embed PROV into the existing audit row body.** Rejected: the
   audit row body is hash-chained and write-once. PROV bundles
   may need to be re-emitted (e.g. after signing-key rotation),
   so they need their own table with FK to `audit_events.seq`.
4. **Defer the whole thing until ADR-0073 wave 3.** Rejected:
   downstream ontology consumers need the *type shape* now to
   start emitting; only the storage/signing layer is genuinely
   multi-day.

## Consequences

- **Working PROV emission**: call sites can produce W3C PROV-JSON and
  durability can append provenance bundle events to the existing hash chain.
- **CCL → AGPL alignment**: PROV bundles will be served over
  HTTP (eventually). The AGPL relicense (ADR-0074) is what makes
  shipping this implementation safe — any downstream SaaS that hosts
  the future endpoint must publish modifications.
- **Schema caution**: PROV-JSON is W3C-stable; HTTP/query persistence
  can be added later without changing the emitted bundle shape.

## References

- [W3C PROV-JSON][prov-json]
- ADR-0073 — EU-sovereign pivot
- ADR-0051 — Ontology Runtime Core
- ADR-0002 — Audit chain
- ADR-0074 — AGPL-3.0-or-later relicense
- COMPLIANCE.md § GDPR Art. 30 / AI Act Annex IV
