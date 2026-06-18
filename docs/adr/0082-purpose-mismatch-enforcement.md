---
status: proposed
date: 2026-06-18
deciders: Roberdan
---

# ADR-0082 — Purpose-mismatch enforcement on the ontology write path

## Context

[ADR-0054](./0054-provenance-bundle-purpose-registry.md) §B introduced
two upstream primitives for purpose limitation (GDPR Art. 5(1)(b)):

1. A **purpose registry** — immutable, labelled declarations of *why*
   data may be processed (shipped: `convergio-ontology` `purposes`
   table + `PurposeStore` + `cvg purpose`, PR #489).
2. A **capability `purposes` field** — a capability bundle (ADR-0008)
   may declare which purposes it is bound to; an install whose declared
   purposes are not all registered is refused (shipped: PR #490).

What is still missing is the **enforcement**: nothing yet refuses a
data mutation performed *without* a declared purpose when the data is
purpose-sensitive. ADR-0054 §B.4 sketched a "purpose-mismatch gate"
keyed on *typed-action effects* touching an `ObjectType` flagged
`requires_purpose`. That framing has two problems we must resolve
before implementing:

- **The effects[] dependency.** Typed-action `effects[]` (W2) do not
  exist yet. Blocking purpose enforcement on the full action-effects
  machinery delays the single most important sovereignty guarantee.
- **A layer cycle.** The durability gate pipeline
  (`convergio-durability`) cannot depend on `convergio-ontology`
  (ontology already depends on durability). The purpose registry and
  the `requires_purpose` schema flag both live in `convergio-ontology`,
  so a *durability task-gate* cannot read them.

There are also **two distinct "purpose" concepts** that must not be
conflated:

- The request-level `x-purpose-id` middleware (`convergio-server`
  `purpose.rs`): a per-request UUID, an audit-binding mechanism.
- The **purpose registry** label (this ADR): a declared, named
  processing purpose. Enforcement here concerns the *registry label*,
  not the request UUID.

## Decision

Enforce purpose limitation at the **ontology object-write admission
point**, not in the durability task-gate pipeline, and key it on the
*concrete object write* rather than on abstract typed-action effects.

### 1. `ObjectType.requires_purpose`

Add an optional boolean `requires_purpose` (default `false`) to the
`ObjectType` schema (`convergio-ontology` `model.rs` + registry +
deterministic JSON-Schema/SHACL export). A type so flagged may only be
created/mutated under a declared purpose.

### 2. Active purpose on the write

An ontology write (create instance / set property / assert link)
carries an **active purpose** (a registry label) on the request:

- Explicit `purpose` field on the write payload, else
- the plan-declared purpose for the owning tenant/plan, else
- absent.

The active purpose is recorded in the PROV bundle of the write
(ADR-0054 §A), so the "why" is auditable.

### 3. Admission check (the "gate")

On a write whose target `ObjectType` has `requires_purpose = true`, the
admission check in `convergio-ontology` (invoked from the
`OntologyStore` write path, surfaced over HTTP by the ontology routes)
refuses unless **all** hold:

1. an active purpose is present;
2. that purpose is **registered** in the purpose registry;
3. **if** the write is performed under a capability that declares a
   non-empty `purposes` set, the active purpose is a member of that
   set (the literal "mismatch" case).

Refusal is a stable, typed error
(`Error::PurposeRequired` / `Error::PurposeMismatch`) mapped to
`400`/`409` by the server, with `purpose_required` /
`purpose_mismatch` codes. Types **without** the flag are unaffected
(opt-in, zero churn for existing verticals).

### 4. Why the object-write path, not durability gates

`convergio-ontology` already depends on `convergio-durability` and owns
both the registry and the `requires_purpose` flag, so the check sits
where all inputs are visible with **no new crate dependency and no
cycle**. The capability `purposes` set is read via the server layer
(which depends on both) and passed into the write call as the "calling
capability" context; the ontology crate itself stays free of a
capability dependency.

## Decision Drivers

- Ship the sovereignty guarantee **without** waiting for typed-action
  `effects[]`; the concrete write path is a sufficient and auditable
  enforcement point.
- Respect the crate layering (no durability→ontology edge).
- Opt-in per `ObjectType`, so existing verticals are unaffected.
- Compose with the already-shipped registry (PR #489) and capability
  field (PR #490).

## Considered Options

1. **Typed-action effects gate in durability (ADR-0054 §B.4 literal).**
   Rejected for now: requires `effects[]` (not built) and would force a
   durability→ontology dependency (cycle).
2. **Request `x-purpose-id` UUID as the purpose.** Rejected: the UUID
   is an audit-binding token, not a declared registry purpose; reusing
   it conflates the two concepts and gives no purpose-limitation value.
3. **Object-write admission keyed on `requires_purpose` (chosen).**
   Minimal, cycle-free, ships now, upgradeable to the effects-based
   form later once `effects[]` lands (the admission helper can then
   also be invoked from the typed-action path).

## Consequences

- New optional schema field changes the deterministic exporters →
  golden fixtures updated; absence keeps output byte-identical for
  existing types.
- Enforcement is only as strong as types are flagged; flagging is a
  vertical's job (Convergio ships zero `ObjectType` instances).
- A follow-up can wire the same admission helper into the typed-action
  path when `effects[]` ships, without changing this contract.

## Dependencies

- PR #489 — purpose registry (`PurposeStore`).
- PR #490 — capability `purposes` field + `Capability::declared_purposes()`.
- This ADR — `ObjectType.requires_purpose` flag (new).
- Tracks plan #14 (Ontology Runtime W2).
