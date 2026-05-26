# DPIA - Ontology Platform (Ontology + Typed Actions + Bitemporal)

- Doc: `docs/security/dpia.md`
- Status: Draft (requires DPO review)
- Last updated: 2026-05-26
- Scope: Convergio v3 core platform (local-first daemon) as used by vertical accelerators that may process personal data.

## Executive summary (EN)
Convergio's Ontology Platform introduces a graph of linked objects and an action framework that can project changes onto that graph. The planned bitemporal model (valid-time + system-time) and scenario branching increase auditability and safety for regulated domains, but they also amplify privacy risks if personal data is stored in the ontology, lineage, diffs, or logs.

This DPIA focuses on four priority risks:
1) re-identification through linked objects;
2) bitemporal/history leakage after erasure;
3) branch leakage (scenario branches and operational branching);
4) agent/action privilege escalation.

## Sintesi (IT)
La piattaforma di ontologia di Convergio introduce un grafo di oggetti collegati e un framework di azioni che puo proiettare modifiche sul grafo. Il modello bitemporale pianificato (valid-time + system-time) e il branching di scenario migliorano audit e sicurezza, ma aumentano i rischi privacy se dati personali finiscono in ontologia, lineage, diff o log.

Questa DPIA copre quattro rischi prioritari: re-identificazione via collegamenti, leakage bitemporale post-cancellazione, leakage da branch, escalation di privilegi agente/azioni.

## Scope and assumptions
### In scope
- Ontology objects and their links/lineage (ADR-0053 proposed; related ADR-0054, ADR-0051).
- Typed Actions framework and action registry/effects projection (ADR-0052 proposed; ADR-0047/0048 shipped).
- Bitemporal storage: valid_* and system_* axes; append-only mutation pattern (ADR-0053 proposed).
- Scenario branching as CRDT overlay and diff/merge primitives (ADR-0056 proposed).
- Agent runners and action execution including least-privilege permission profiles (ADR-0033 accepted).

### Out of scope (handled by vertical accelerators / operators)
- Domain-specific data taxonomy (exact personal-data fields), legal basis selection, and DPIAs for specific verticals.
- Cross-border transfers (core is local-first; verticals may add integrations).
- End-user UI/UX for rights requests.

### Implementation status note
This doc intentionally distinguishes:
- Implemented controls (current code/ops posture)
- Proposed controls (design requirements before shipping proposed ADRs)

## System description (high level)
Convergio is a local-first daemon with:
- SQLite state storage and a hash-chained audit log (ADR-0002).
- A gates pipeline refusing unsafe transitions (HTTP 409 + stable refusal reasons).
- Vendor-CLI agent runners with named permission profiles to reduce tool privilege (ADR-0033).
- An action registry (actions.json) and compensating actions surface (ADR-0047/0048).

The Ontology Platform work (planned) adds:
- Typed actions admitting/refusing based on preconditions and emitting typed effects (ADR-0052).
- Bitemporal ontology storage and lineage queries (ADR-0053).
- Scenario branches for safe 'what-if' simulation and controlled merge back to mainline (ADR-0056).

## Personal data and processing overview
Because Convergio is a platform, it may process personal data depending on the vertical accelerator. For DPIA purposes, assume the ontology can include:
- Identifiers (internal IDs, external IDs, pseudonyms), relationship edges, and attributes.
- Evidence payloads and action inputs that may contain personal data.

Key privacy-relevant surfaces:
- Graph topology (links) can be identifying even if node attributes are minimized.
- Append-only temporal history may preserve prior states.
- Branch overlays and diffs can retain snapshots.
- Actions can create/transform data at speed if mis-scoped.

## Baseline controls (Implemented)
- Least-privilege runner permission profiles (ADR-0033 accepted): avoid allow-all by default; explicit Sandbox escape hatch only.
- Audit chain (ADR-0002): tamper-evident logging; useful for accountability but a risk if personal data is logged.
- Local-first default: data remains on the operator's machine unless a vertical adds outbound connectors.

## Controls required before shipping proposed ontology/actions/bitemporal/branching
These are design requirements (must be implemented/validated before enabling the corresponding proposed ADR features in production verticals):
- Data classification on ontology properties (PII/sensitive/non-PII) with enforcement in action admission and export.
- Explicit retention/erasure strategy for personal data stored in ontology/history/branch overlays.
- Query/export redaction policies (including lineage/diffs).
- Capability/action scoping and approvals for high-risk actions.

## Risk register (priority)

| ID | Risk | Scenario | Impact | Likelihood | Required mitigations (summary) | Residual risk / notes |
|---:|------|----------|--------|------------|--------------------------------|-----------------------|
| R1 | Re-identification via linked objects | Combining multiple linked objects (even pseudonymized) allows re-identifying a person; lineage + graph queries amplify. | High | Medium | Property classification; access control + purpose binding for graph queries; redaction/k-anonymity/DP at export; minimize linkage; audit queries. | Residual Medium pending vertical controls + DPO sign-off. |
| R2 | Bitemporal leakage post-erasure | Append-only bitemporal history retains prior personal data after an erasure request; as_of queries could reveal erased states. | High | Medium | Define erasure semantics for bitemporal: hard delete where lawful; crypto-shredding (per-subject keys) or redaction tombstones; enforce retention windows; document exceptions (legal holds). | Residual High until an erasure model is implemented and tested end-to-end. |
| R3 | Branch leakage (scenario + ops) | Scenario branches retain snapshots/diffs containing personal data beyond intended scope; operational branches/worktrees/PRs leak data into history/artifacts. | High | Medium | Branch TTL + reaper (ADR-0056); caps on active branches; encrypt/segregate branch storage; restrict diff export; operator hygiene: no PII in git, evidence redaction, review gates. | Residual Medium; operational leakage depends on operator/vertical discipline. |
| R4 | Agent-action escalation | An agent triggers privileged actions or uses overly broad capabilities to exfiltrate/alter personal data; action registry misuse. | High | Medium | Least-privilege runner profiles (implemented); capability allowlists per plan/task; precondition gate (ADR-0052); approvals for irreversible/high-risk actions; comprehensive audit with alerting. | Residual Medium; requires strong scoping + monitoring. |

## DPO review checklist
- Confirm whether planned ontology/lineage exports qualify as high risk processing in the target vertical(s).
- Approve the erasure model for bitemporal history (R2) and define lawful exceptions.
- Validate re-identification mitigations for graph/linkage (R1), including export defaults.
- Validate capability governance and approval workflow for high-risk actions (R4).

## References
- ADR-0033 Vendor-CLI runners use least-privilege permission profiles (accepted)
- ADR-0052 Typed Actions Framework over the Ontology (proposed)
- ADR-0053 Bitemporal Store + Lineage over Ontology Objects (proposed)
- ADR-0056 Scenario Branching (Workshop primitives) (proposed)
