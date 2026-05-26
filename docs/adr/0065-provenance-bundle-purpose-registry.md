---
id: 0054
status: proposed
date: 2026-05-25
topics: [ontology, provenance, purpose, capabilities, w3c-prov]
related_adrs: [0002, 0008, 0018, 0051, 0052, 0053]
touches_crates: [convergio-ontology, convergio-durability, convergio-api]
last_validated: 2026-05-25
---

# 0054. Provenance Bundle & Purpose Registry

- Status: proposed
- Date: 2026-05-25
- Tags: ontology, provenance, purpose, w3c-prov

## Context

Vertical accelerators handling regulated data — `convergio-edu`
is the first, but research-IP, clinical, public-sector will
follow — must answer two questions that the audit log alone
cannot answer in a portable, externally consumable way:

1. **"What is the full provenance of this object?"** —
   actions, inputs, agents, models, prompts, evidence rows,
   capability versions — in a form a third-party auditor can
   ingest.
2. **"Was every read/write of this data justified by a
   declared purpose?"** — the capability bucket says *what* an
   agent may do, not *why*.

This ADR adds two upstream primitives, both generic and
dominio-agnostic.

## Decision

### A. Provenance bundle (W3C PROV-compatible)

1. Every successful typed action (ADR-0052) emits a
   **PROV bundle** referencing:
   - `prov:Activity` — the action.
   - `prov:Agent` — the agent that issued it (plus model
     identifier, prompt hash if LLM-driven, capability bundle
     id from ADR-0008).
   - `prov:Entity` — the ontology object(s) created, mutated,
     linked.
   - `prov:wasInformedBy` — upstream evidence rows.
2. Bundles serialised as canonical JSON-LD; bundle hash
   referenced from `audit_log` (extends, does not replace,
   ADR-0002).
3. CLI / MCP surface: `cvg ontology provenance <object_id>`
   returns the bundle for the requested time window
   (composes with ADR-0053 bitemporal queries).

### B. Purpose registry

1. New table `purposes` with
   `(id, label, description, declared_by_plan, effective_from)`.
   A purpose is a free-form string registered via
   `cvg purpose register` and immutable thereafter.
2. Capability bundles (ADR-0008) gain an optional
   `purposes: [...]` field. A capability not bound to a
   declared purpose is treated as "ambient" and may be refused
   by a vertical-level gate.
3. Every action invocation records the **active purpose** (set
   per task, defaulting to the plan-declared purpose). The
   PROV bundle includes the purpose id.
4. Purpose-mismatch gate: when an action effect touches an
   `ObjectType` flagged `requires_purpose: true` in its schema
   (ADR-0051), the gate refuses unless the active purpose is
   in the declared purpose set of the calling capability.

## Decision Drivers

- Portability: PROV bundles are an open standard; a regulator
  asking for "show me everything you did with object X" gets a
  file, not a database dump.
- Purpose-binding is the upstream primitive every regulated
  vertical needs (GDPR Art. 5(1)(b), education-specific
  FERPA-equivalent, healthcare consent registries).
- Capability registry already exists (ADR-0008); we extend it
  rather than parallel-track.

## Considered Options

1. **Audit log only.** Rejected: not portable, not bundleable,
   no purpose dimension.
2. **OpenLineage instead of PROV.** Considered, but PROV's
   coverage of Agent/Activity/Entity is the better match for
   the ontology model; nothing prevents a vertical from
   emitting OpenLineage on top.
3. **Purpose-binding as a vertical concern only.** Rejected:
   the cost of retro-fitting purpose into every action
   invocation later is much higher than reserving the column now.

## Compliance Anchors

- P1 zero-debt.
- P2 local-first: bundles live in SQLite, exported on demand.
- ADR-0002 audit chain stays the canonical tamper-evident
  spine; PROV bundles are derived, not authoritative.

## Rollout

- W2 / part of plan *Ontology Platform W2: Typed Actions +
  Provenance*:
  - PROV bundle emitter + canonical JSON-LD serialiser +
    golden tests.
  - `purposes` table + CLI + MCP surface.
  - Capability bundle field extension (ADR-0008 amendment).
  - Purpose-mismatch gate wired into the gate pipeline.

## Consequences

- Verticals like `convergio-edu` can now produce a regulator-
  ready provenance file per request without bespoke code.
- Every capability bundle author has a new decision to make
  (purposes), backed by sensible "ambient" default.
- `audit_log` row size grows by ~one hash; bundle storage is
  separate.

## Alternatives left for verticals

- Vertical-specific purpose taxonomies (FERPA categories,
  healthcare consent kinds).
- Public export formats (e.g. eIDAS-signed PROV PDFs).

## References

- ADR-0002 audit hash chain
- ADR-0008 capability bundles
- ADR-0018 long-tail vertical accelerators
- ADR-0051 ontology runtime core
- ADR-0052 typed actions framework
- ADR-0053 bitemporal store + lineage
- W3C PROV-O 2013 recommendation
