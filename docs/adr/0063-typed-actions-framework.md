---
id: 0063
status: proposed
date: 2026-05-25
topics: [ontology, actions, compensations, audit]
related_adrs: [0002, 0047, 0048, 0062]
touches_crates: [convergio-api, convergio-server, convergio-durability]
last_validated: 2026-05-25
---

# 0063. Typed Actions Framework over the Ontology

- Status: proposed
- Date: 2026-05-25
- Tags: ontology, actions, compensations

## Context

ADR-0047 introduced an action type registry (`actions.json`) and
ADR-0048 introduced compensating action types. Both treat actions
as flat envelopes whose effects live in the head of whoever
implements them. Once `ontology (future)` (ADR-0062) exists, the
daemon can — and must — describe what each action *does to the
ontology*: which `ObjectType` it creates, mutates, or links;
which pre-conditions must hold; which compensation undoes it.

Vertical accelerators that drive change with LLM-generated
proposals (the explicit `convergio-edu` use case) cannot rely on
runtime reflection alone; they need a typed contract for safe
write-back and reliable undo.

## Decision

Promote actions from "named callable" to **typed effect on the
ontology**:

1. **Action specification** carries:
   - `inputs`: typed parameters (referencing `PropertyType` from
     ADR-0062 where applicable).
   - `effects`: structured list of
     `{kind: create|update|link|unlink|delete, object_type,
       property_set}`.
   - `preconditions`: predicate expressions evaluated against the
     ontology before the action is admitted.
   - `compensation_ref`: the action that semantically reverses it
     (extends ADR-0048).
2. **Build-time export** extends `actions.json` with an
   `effects[]` array per action. Byte-identical re-export rule
   from ADR-0047 still holds.
3. **Runtime admission gate** — before any typed action runs, the
   daemon evaluates preconditions and refuses with HTTP 409 +
   stable reason when violated. Refusals are audited
   (hash-chained per ADR-0002).
4. **Effect projection** — successful actions emit an
   `effect_envelope` written to a dedicated `ontology_events`
   table. The audit row points at the envelope hash. This is the
   substrate that ADR-0064 (bitemporal store) and ADR-0065
   (provenance bundle) will project on.
5. **No domain-specific actions in core.** The framework is
   generic; concrete actions are registered by the vertical
   accelerator at install time, via the capability bundle
   surface (ADR-0008).

## Decision Drivers

- Modulor consistency: every action becomes a first-class
  citizen of `(task, evidence, gate, audit_row)` *plus*
  `(effect, object, link)`.
- Reversibility: every mutation has a documented undo path.
- Auditability: effects, not just intentions, are recorded.
- Verticals stay verticals: core ships zero business actions.

## Considered Options

1. **Keep ADR-0047 envelope, add ad-hoc effects metadata per
   vertical.** Rejected — re-introduces drift.
2. **Switch to an external workflow engine.** Rejected — violates
   P2 local-first; the daemon already owns the queue.
3. **This proposal.** Accepted.

## Compliance Anchors

- P1 zero-debt: typed effects close a long-standing class of
  "did this action actually do X?" questions.
- P2 local-first: stays inside the daemon.
- P5 audit invariants (ADR-0002): every effect is hash-chained.

## Rollout

- W2 (plan *Ontology Platform W2: Typed Actions + Provenance*):
  - Schema extension in `actions.json` + golden tests.
  - `ontology_events` table + migration.
  - Admission gate (precondition predicates) wired into the
    existing gate pipeline.
  - Compensation pointer enforced — every non-`read` action
    requires a `compensation_ref` or an explicit `irreversible`
    flag with a CONSTITUTION-grade rationale.

## Consequences

- `convergio-edu` Workshop scenarios become safe to simulate:
  every typed action is a known mutation with a known undo.
- Adversarial-review service (ADR-0022) gains a structured
  surface to challenge actions against ontology invariants.
- New CI gate: any action without an effect spec fails build.

## Alternatives left for verticals

- Domain-specific compensation strategies (e.g. financial
  reversal vs academic record amendment) live in the vertical.

## References

- ADR-0002 hash-chained audit
- ADR-0008 capability bundles
- ADR-0022 adversarial-review service
- ADR-0047 actions.json
- ADR-0048 compensating actions
- ADR-0062 ontology runtime core
