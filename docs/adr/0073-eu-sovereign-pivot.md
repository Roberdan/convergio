# ADR-0073 — EU-sovereign pivot: open ontology platform, AI-native, local-first

- Status: **accepted**
- Date: 2026-05-27
- Deciders: Roberto D'Angelo (project lead)
- Related: ADR-0004 (three sacred principles), ADR-0016 (long-tail
  vertical accelerators), ADR-0018 (urbanism — Jacobs + Modulor),
  ADR-0051 (ontology runtime core), ADR-0064 (bitemporal-store /
  lineage), ADR-0065 (provenance bundle + purpose registry)
- Supersedes: nothing — extends ADR-0016 thesis

## Context

ADR-0016 framed Convergio as the "shovel" for a long tail of vertical
AI accelerators, with ADR-0018 borrowing Jane Jacobs (urban code) and
the Modulor (proportional discipline) as the governance frame.

Two facts have since hardened:

1. The **ontology series** ADR-0051..0072 landed or is in flight: a
   typed schema registry (ADR-0051), bitemporal store with lineage
   (ADR-0064), provenance bundle + purpose registry (ADR-0065),
   typed actions, entity resolution, scenario branching, connector
   SDK, LLM gateway primitive. Read together these primitives
   describe a **platform where AI agents and humans converge on data
   both can trust** — not just a leash.
2. The EU regulatory perimeter (AI Act in force 2026-08, GDPR
   purpose-limitation enforcement, NIS2 tamper-evident logging,
   DORA, the EU Data Act) is creating a procurement segment where
   the dominant US platform (Palantir Foundry) is politically and
   reputationally blocked: public administration, public health,
   universities, civic infrastructure, NGOs.

The shovel thesis is correct but undersells the artifact.

## Decision drivers

- **D1 — Regulatory perimeter.** EU buyers need *provable* data
  governance, not slideware. Hash-chained audit (Layer 1) + purpose
  registry (ADR-0065) + bitemporal lineage (ADR-0064) map directly to
  GDPR Art 5/15/17/30, AI Act Art 12 (logging) and Art 14 (oversight),
  NIS2 Art 21 (auditability), DORA Art 17 (incident reconstruction).
- **D2 — Reputational alpha.** Open-source, AGPL-compatible, local-first
  software with no SaaS dependency is the *opposite* of the
  procurement risk Palantir carries. There is a market segment where
  "not Palantir" is a feature.
- **D3 — Sovereignty by construction, not by checkbox.** Single-user,
  SQLite-on-disk, `127.0.0.1` bind, no remote control plane, no
  vendor lock-in on the model side. The same property that makes
  Convergio a good *leash* makes it a credible *sovereign data
  platform*.
- **D4 — Already-built primitives.** The ontology series is not new
  scope; it is acknowledging what the codebase already ships or is
  about to ship.
- **D5 — Long-tail still holds.** Vertical accelerators (education,
  health compliance, public-sector workflows) remain the demand
  side. The pivot strengthens that thesis: each accelerator gets a
  typed ontology, provenance, time-travel and governance for free.

## Considered options

### Option A — Stay leash-only
Keep the README/vision framing on "make machines prove it" only.

- Pro: smallest doc surface, no positioning risk.
- Con: undersells what the codebase actually does; fails to qualify
  for the EU procurement segment; loses the differentiator vs every
  other "agent orchestrator" project.

### Option B — Full Palantir-clone now
Replace all leash language with "open Foundry alternative".

- Pro: clearest market signal.
- Con: overclaims; bitemporal/provenance/purpose are spec'd or
  partial, not enforced end-to-end yet; would put us in the position
  the project explicitly refuses (ADR § sacred-4: no scaffolding only).

### Option C — Layered framing (selected)
**Leash is the safety belt; ontology is the product.** Same artifact,
two layers. Honest status on each principle (enforced / partial /
planned).

- Pro: matches reality of the codebase; gives EU buyers a credible
  compliance posture without lying; keeps the existing user (AI
  agent operators) addressed.
- Con: positioning text becomes denser; requires a public
  `COMPLIANCE.md` to back the claim.

## Decision

**Adopt Option C.** Convergio is positioned as an
**open, local-first, EU-sovereign platform where AI agents and
humans converge on data both can trust** — built on:

- *Layer 1 (leash):* hash-chained audit, gate pipeline, evidence,
  the five sacred principles (ADR-0004). Enforced today.
- *Layer 2 (ontology):* typed schema registry, bitemporal store,
  provenance bundle, purpose registry, typed actions, scenario
  branching, entity resolution. Status per primitive declared in
  `COMPLIANCE.md`.
- *Frame:* ontology = *what* is modelled (Palantir-like surface area);
  urbanism (ADR-0018) = *how* governance, oversight and modular reuse
  are organized. The two are orthogonal and reinforce each other.

A **sixth sacred principle — *Sovereignty by construction*** — is
added to `CONSTITUTION.md` to lock the local-first / no-remote-control
plane / no-vendor-lock guarantees as non-negotiable.

A **regulatory matrix** (`COMPLIANCE.md`) maps each EU obligation
(GDPR, AI Act, NIS2, DORA, EU Data Act, eIDAS) to the Convergio
primitive that implements or will implement it, with honest status.

The project will **relicense to AGPL-3.0-or-later + CLA** (tracked in
the EU-sovereign pivot plan) to clear the OSI-approval blocker for
EU public-administration procurement.

## Consequences

### Positive
- A coherent narrative ("AI + humans converge on trustable data")
  that maps to a real, fundable buyer segment.
- Existing primitives (audit, ontology, bitemporal, provenance,
  purpose) are explicitly accounted for, not lost in marketing.
- Honest status (`enforced` / `partial` / `planned`) preserves trust
  with technical buyers and contributors.
- Multi-vendor agent runners (Claude, Copilot, Codex, Gemini, Qwen)
  remain a feature, not a contradiction: model choice is the
  operator's, data sovereignty is the platform's.

### Negative
- Positioning text becomes denser; the 30-second pitch needs more
  care to stay honest.
- `COMPLIANCE.md` is now a load-bearing document — drift between it
  and the gate pipeline becomes a bug class. Mitigated by ADR-0015
  auto-regen pattern (apply to compliance status table).
- Relicense to AGPL is a breaking change for any downstream
  consumer that assumed the previous community license; mitigated
  by CLA and an explicit BREAKING changelog entry.

### Neutral
- Local-first, single-user, SQLite-only does **not** change.
- The five existing sacred principles do **not** change.
- The leash framing remains valid for the AI-agent-operator audience;
  it just stops being the *only* framing.

## What changes immediately

- `README.md`: new hero, updated *is / is not* table, **Compliance
  posture (honest status)** section.
- `docs/vision.md`: lead pivots to the layered framing.
- `CONSTITUTION.md`: § 6 *Sovereignty by construction* (status:
  partial; criteria for `enforced` listed).
- `COMPLIANCE.md`: regulatory matrix, honest per-primitive status.
- `docs/AGENTS.md`: canonical-doc registry adds COMPLIANCE.md and
  this ADR.

## What does *not* change

- 5 existing sacred principles (P1..P5) and their enforcement.
- Local-first / SQLite-only / `127.0.0.1` / no-SaaS architecture.
- Vendor-agnostic agent runner surface.
- The shovel thesis for long-tail vertical accelerators.

## Validation

- ADR-0015 coherence check: every other doc referencing leash-only
  framing updated in the same PR.
- `COMPLIANCE.md` cross-checked: every `enforced` claim points to a
  gate name in `crates/convergio-durability/src/gates/` or an
  audit-kind in the chain.
- Pivot plan (`cvg plan get 8e7936e6-bc05-47c9-ade6-28c3e968e39b`)
  references this ADR.
