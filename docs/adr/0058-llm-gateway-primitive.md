---
id: 0058
status: proposed
date: 2026-05-25
topics: [ontology, llm, gateway, safety, p2]
related_adrs: [0008, 0018, 0020, 0022, 0050, 0051, 0052]
touches_crates: [convergio-api, convergio-server, convergio-ontology]
last_validated: 2026-05-25
---

# 0058. LLM Gateway primitive (typed, ontology-aware)

- Status: proposed
- Date: 2026-05-25
- Tags: ontology, llm, gateway, safety

## Context

The municipality already has two LLM-safety primitives:
ADR-0050 (`PromptInjectionGate` on evidence payloads) and
ADR-0020 (model evaluation framework). Vertical accelerators
that drive ontology mutations from LLM output need a third
piece: a **gateway** that sits between an agent and the
underlying provider, so that every LLM call is typed against
the ontology, redacted before egress, schema-validated on
return, and auditable end-to-end.

Today each accelerator wires its own provider client. Result:
no central place to apply redaction, no central place to refuse
on suspicious output, and inconsistent provenance bundles
(ADR-0054).

This ADR ships an **upstream primitive** only — the gateway
contract and a local pass-through implementation. Vertical-
specific policies (PII redaction libraries by language,
sector taxonomies, public algorithm register entries) stay in
the vertical.

## Decision

Introduce a **typed LLM gateway** as part of the daemon's API
surface, behind a stable contract:

1. **Typed request envelope.**
   - `prompt`, `model_ref`, `expected_output_schema` (a
     JSON-Schema id, optionally pointing at a `PropertyType`
     fragment from ADR-0051).
   - `active_purpose` (ADR-0054) — required, refused otherwise.
   - `capability_bundle_id` of the calling capability
     (ADR-0008).
2. **Egress pre-flight.**
   - Prompt body passes through the same `PromptInjectionGate`
     surface as ADR-0050 (refuse with stable reason).
   - Optional **redactor hook** chain — the core registers a
     no-op default; verticals plug their library (Presidio,
     custom regex packs, locale-specific PII).
3. **Provider pass-through.**
   - The daemon does not bundle a provider. It forwards over
     HTTP to a provider endpoint declared in the capability
     bundle (signed). Pass-through is the only built-in mode
     to honour P2 (no surprise outbound traffic).
4. **Return validation.**
   - Response is validated against `expected_output_schema`.
     Failures are refused (no silent coercion) with a stable
     reason and the raw response stored as evidence for
     debugging.
5. **Audit + provenance.**
   - Every gateway call writes an `audit_log` row and emits a
     PROV bundle (ADR-0054) referencing prompt hash, model
     identifier, capability bundle, active purpose, output
     schema id, and the resulting typed action (if any).
6. **No model registry in core.** Models are referenced by
   identifier; the model-evaluation framework (ADR-0020)
   stays the place where verticals score them.

## Decision Drivers

- One place to enforce redaction + injection gate + schema
  validation; cheaper than auditing N vertical clients.
- Typed output is the natural feeder for typed actions
  (ADR-0052) and PROV bundles (ADR-0054).
- Pass-through only — P2 keeps networking explicit; verticals
  decide whether to talk to a cloud provider, an on-prem
  model, or an air-gapped local one.

## Considered Options

1. **No core gateway; each vertical owns it.** Rejected —
   produces drift, prevents central refusal hooks.
2. **Core gateway bundles providers (OpenAI, Azure OAI,
   Anthropic, …).** Rejected — violates P2 (silent outbound
   surface) and locks the project into provider churn.
3. **This proposal — typed contract + pass-through + hooks.**
   Accepted.

## Compliance Anchors

- P1 zero-debt: schema-validation failures are structured
  refusals, not warnings.
- P2 local-first: the daemon itself never initiates a network
  call to a model unless a signed capability bundle declares
  the endpoint.
- ADR-0050 PromptInjectionGate composes with egress pre-flight.
- ADR-0054 purpose-binding: every gateway call is refused
  without an active purpose.

## Rollout

- W5 (plan *Ontology Platform W5: LLM Gateway primitive*):
  - Gateway contract + handler in `convergio-api`.
  - Redactor + return-validator hook chain.
  - PROV bundle integration.
  - `cvg llm call --schema ... --purpose ...` CLI for tests.
  - MCP action `llm.call` registered in `actions.json`.
  - Golden tests with a stub provider; CI must never call a
    real provider.

## Consequences

- Verticals retire bespoke clients; redaction libraries become
  registered hooks, not forks.
- New surface for adversarial-review (ADR-0022) to inspect
  prompt/response pairs against ontology invariants.
- A future ADR may add streaming and tool-calling support;
  this ADR keeps the surface to non-streaming JSON output.

## Alternatives left for verticals

- PII redaction libraries / regex packs / locale support.
- Public algorithm registers (`convergio-edu` ADR-0027 lives
  here).
- Provider selection and SLA monitoring.

## References

- ADR-0008 capability bundles
- ADR-0020 model evaluation framework
- ADR-0022 adversarial-review service
- ADR-0050 PromptInjectionGate
- ADR-0051 ontology runtime core
- ADR-0052 typed actions framework
- ADR-0054 provenance bundle + purpose registry
