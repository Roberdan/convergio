---
id: 0018
status: proposed
date: 2026-05-01
topics: [vision, philosophy, capability-bundles, gates]
related_adrs: [0001, 0002, 0006, 0007, 0008, 0011, 0012, 0014, 0016]
touches_crates: []
last_validated: 2026-05-01
---

# 0018. Urbanism over architecture: Convergio is an urban code, not a master plan

- Status: proposed
- Date: 2026-05-01
- Deciders: Roberdan
- Tags: vision, philosophy

## Context and Problem Statement

ADR-0016 names the long-tail thesis: Convergio is the shovel for
many vertical AI accelerators. That commitment immediately raises a
design question: how does Convergio scale to many accelerators
without becoming opinionated about each of them, and without
collapsing into either a god-monolith or a permissive free-for-all?

The candidate metaphor is **urbanism**.

Convergio is not a house, not even a particularly good one. It is
a **city** — with its rules (CONSTITUTION), its planning instruments
(ADRs), its inspection regime (gates), its cadastre (audit chain),
its modular materials registry (capability bundles), and its
coordination protocols (CRDT + workspace leases).

Two thinkers shape what kind of urbanism we mean.

**Le Corbusier** (1924-1933, *Ville Radieuse*; 1948, *Modulor*).
Master plan, modularity, zoning by function, shared infrastructure,
written codes. These are the technical primitives of urbanism. We
adopt them.

**Jane Jacobs** (1961, *The Death and Life of Great American
Cities*). Bottom-up emergence, eyes on the street, mixed use,
suspicion of master-plan thinking. She demolished Le Corbusier's
*Ville Radieuse* as anti-city. We take her warnings seriously.

The decision is to be **strict on a few non-negotiables and
permissive everywhere else** — Le Corbusier for the building
codes, Jacobs for the street life. Without naming this explicitly,
Convergio risks drifting either toward *Ville Radieuse*
totalitarianism (gates everywhere, no room for vertical innovation)
or toward Jacobs anarchy (no enforcement, every accelerator a
brittle prototype). Both extremes destroy the long-tail thesis.

## Decision Drivers

- **Convergio v0.1.x already shipped urbanism primitives** without
  naming them. CRDT (ADR-0006) is bottom-up convergence — Jacobs.
  Workspace leases (ADR-0007) are building permits — Le Corbusier.
  Hash-chained audit (ADR-0002) is transparent cadastre — both.
  Capability bundles (ADR-0008) with Ed25519 signing are a
  materials registry — Le Corbusier modularity with Jacobs trust.
  The decision is to *name* what we already built, not to build
  new things.
- **The long-tail thesis (ADR-0016) requires Lego, not buildings.**
  If Convergio designs each accelerator, throughput collapses to
  one author. If Convergio designs the urban code, throughput
  scales to as many authors as adopt the standards.
- **The risk of master-plan drift is real.** Five sacred principles
  + 14 ISE NFRs + custom gates per domain + plan templates + …
  it is easy to slide toward "Convergio knows better than the
  builder". Jacobs is the antibody to that drift, codified.
- **The Modulor concept genuinely helps.** Le Corbusier's *Modulor*
  was a system of human-scale proportions that let any architect
  produce a coherent building. Convergio's Modulor —
  `(task, evidence, gate, audit_row)` — does the analogous job:
  any builder of an accelerator works in the same atomic unit, so
  pieces compose across the city.

## Considered Options

### Option A — Pure Le Corbusier (master-planned city)

Convergio defines the canonical plan template, the canonical agent
roles, the canonical capability composition for each major vertical
(education, research, healthcare, …). Builders pick a vertical and
follow the canonical template. Costs: Jacobs was right. Master
plans hostile to street-level innovation produce dead cities. The
long-tail thesis dies the moment the urban code becomes the
solution.

### Option B — Pure Jacobs (no plan, no codes)

Convergio is a permissive bus. Builders register agents, submit
evidence, and the daemon trusts them. Costs: ships the same
"agents claim done before they are done" problem v0.1.x exists to
solve. The five sacred principles become aspirational. The leash
turns into a length of string.

### Option C — Le Corbusier for the codes, Jacobs for the street (chosen)

The non-negotiables are Le Corbusier: explicit, written, mechanical,
enforced by gates with HTTP 409 and audit rows. Everything else is
Jacobs: builders pick their materials (capability bundles), their
accelerator pattern (plan templates are starting points, not
prescriptions), their domain gates (extend, do not replace, the
sacred principles). The Modulor is shared so the pieces compose.

## Decision Outcome

Chosen option: **Option C**, because it preserves the safety belt
v0.1.x already shipped while leaving the long-tail surface
permissive enough to support thousands of vertical accelerators.

### Implementation status (2026-05-04)

Stays `proposed`. This ADR is a design-philosophy decision — it names
the urbanism frame (Le Corbusier building codes + Jacobs street life,
Modulor as `(task, evidence, gate, audit_row)`, capability namespaces
as zoning) so future ADRs can be evaluated against it. The frame is
real; no code change ships under this ADR alone, by design (§
"Neutral").

### Convergio is the municipality

The metaphor sharpens once we stop calling Convergio "the urban
code" and start calling it **the municipality** — the local authority that
*emits* the urban code. The urban code is what the municipality publishes;
the municipality is the institution that publishes it. Both terms appear
in this ADR with these distinct roles: when we discuss the
*institution* (the daemon, the registry, the authority that stamps
permits) we say municipality; when we discuss the *artefact* (the rules,
the building codes, the namespacing strategy) we say urban code.

A full mapping from real-city services to Convergio primitives is
maintained in [`docs/vision.md`](../vision.md) § 3 ("Convergio is
the municipality"). The four-level planning hierarchy (strategy /
regulation / norms / operational plan) is documented in
`docs/vision.md` § 5 ("Planning: how the city grows") and protects
the project from *unauthorized construction* — buildings shipped without a plan
they fit into.

### The Convergio urban code

#### Building codes (Le Corbusier — strict)

- The five sacred principles (CONSTITUTION) — P1-P5, enforced by
  gates, refuse with HTTP 409, audited
- The Modulor — `(task, evidence, gate, audit_row)` — every
  composable unit of work has this shape. Anything that does not
  decompose into this shape is rejected at design review (ADR
  process)
- The OODA loop (ADR-0012) — every transition through `done`
  goes through Observe-Orient-Decide-Act
- The audit chain (ADR-0002) — tamper-evident, hash-chained, no
  silent rewrites
- Capability signing (ADR-0008) — Ed25519, no `--allow-unsigned`,
  ever

#### Street-level freedom (Jacobs — permissive)

- Capability bundles compose freely. There is no canonical
  composition for any vertical. `convergio-edu` is *one* possible
  composition, not *the* one
- Plan templates are starting points. Builders extend, prune,
  reorder, and parameterise them
- Domain-specific gates extend the sacred principles. They cannot
  weaken them, but they can strengthen them (e.g., a healthcare
  accelerator can require HIPAA-compliant evidence kinds)
- Runner adapters are pluggable. Claude, Copilot, OpenAI, local
  shell, and any future agent runtime are first-class citizens
  via the agent registry + spawn protocol
- The bus is multi-tenant per plan. Agents from different vendors
  collaborate without negotiating with the urban code

#### Zoning (named-namespace capabilities)

Capability bundles are namespaced and the namespaces *zone the
city*. The first-tier zoning:

| Namespace | Purpose | Examples |
|---|---|---|
| `azure.*` | Microsoft Azure cloud services | `azure-voice`, `azure-storage`, `azure-openai` |
| `auth.*` | Identity and access | `auth-entra`, `auth-oauth2`, `auth-passkeys` |
| `ui.*` | User interface frameworks | `ui-fluent`, `ui-radix`, `ui-kit-cli` |
| `a11y.*` | Accessibility tooling | `a11y-axe`, `a11y-screen-reader-tests` |
| `payments.*` | Payment + billing | `payments-stripe`, `payments-billing` |
| `data.*` | Data layer adapters | `data-sqlite`, `data-postgres`, `data-cosmos` |
| `obs.*` | Observability | `obs-otel`, `obs-app-insights` |
| `ai.*` | Model + agent runners | `ai-claude`, `ai-copilot`, `ai-openai` |

These are starter zones. New namespaces require an ADR (small
ADR — boilerplate forthcoming) so the city does not balkanise.
Within a namespace, capability authors compete; namespaces
themselves do not proliferate without coordination.

#### The Modulor in detail

Every Convergio operation reduces to manipulations of the Modulor:

```
task         "what should be done"          → tasks table
evidence     "what was done, machine-form"  → evidence table
gate         "is what was done acceptable"  → gates/*.rs, HTTP 409 if not
audit_row    "the fact that this happened"  → audit_log, hash-chained
```

Compositional rules:

- `task → 1..N evidence rows` (a task may attach multiple kinds of
  evidence)
- `evidence → 1..N gate runs` (every gate that applies fires)
- `gate run → 1 audit_row` (always audited, accept or refuse)
- `task transition → 1 audit_row` (always audited)

The Modulor is the unit at which the OODA loop operates. Thor
observes evidence (one Modulor instance), the agent and Thor
orient on the audit log (the city's memory), they decide on the
gate outcome, the human acts when escalation triggers. Outside
the Modulor, the OODA loop has no anchor.

### Why this is *not* Le Corbusier alone

Three explicit anti-Ville-Radieuse choices:

1. **No canonical accelerator.** Convergio does not ship "the
   one true education accelerator". It ships the urban code that
   makes many education accelerators possible.
2. **No mandatory composition.** Capability bundles are
   independently usable. A builder can compose `azure-voice` +
   `auth-entra` + custom UI without buying into a Convergio plan
   template at all.
3. **No top-down agent role assignment.** The bus is symmetric;
   any agent registered via the agent registry can claim any
   task it has the capabilities for. Roles emerge from CRDT and
   workspace leases, not from a central scheduler.

### Why this is *not* Jacobs alone

Three explicit anti-anarchy choices:

1. **The five sacred principles are non-negotiable.** They are not
   community guidelines; they are HTTP 409 refusals with audit
   rows. The street has codes.
2. **Thor-only-done (ADR-0011).** Agents propose; the validator
   disposes. There is no agent self-promotion, ever.
3. **Capability signing is mandatory (ADR-0008).** No
   `--allow-unsigned`. The materials registry is gated.

## Consequences

### Positive

- The repo gains a coherent design philosophy for cross-cutting
  decisions. New ADRs can be evaluated against "does this
  strengthen the Modulor / building codes / street life balance?"
- The capability bundle namespacing strategy (ADR-0008) gets a
  formal frame: namespaces *are* the zoning. Authoring a new
  namespace becomes a small-ADR decision, preventing
  balkanisation.
- The five sacred principles are explicitly **building codes**,
  not aspirations. Closing the P3 a11y honesty gap (ADR-0017)
  becomes a building-code violation, not a roadmap nice-to-have.
- The Modulor is a teaching tool. New contributors and agents
  can be onboarded in fifteen minutes by reading
  CONSTITUTION + this ADR + a single example task end-to-end.

### Negative

- "Urbanism" is a high-concept frame. Without concrete deliverables
  in Wave 2 + Wave 3 of the ROADMAP, it reads as marketing.
  Mitigation: ADR-0016 commits to those waves; this ADR depends
  on them shipping.
- The Le Corbusier / Jacobs language is culturally specific
  (Western 20th-century architecture). International
  contributors may need a glossary. Mitigation: this ADR + VISION
  are the glossary. They link to Wikipedia for both names.
- Some readers will object to citing Le Corbusier at all (his
  later work in Chandigarh and Brasília is critiqued for the same
  reasons Jacobs critiqued *Ville Radieuse*). The ADR responds to
  that objection by adopting only the Modulor and zoning concepts
  and explicitly rejecting his master-plan posture.

### Neutral

- The CRDT + workspace lease + capability primitives shipped
  before this ADR. The ADR retroactively names what they were
  for. No code change is required by this ADR alone — only doc
  updates and namespacing discipline going forward.

## Validation

This ADR is validated when:

1. `docs/vision.md` § 2 ("The frame: urbanism, not architecture")
   cites this ADR.
2. The Modulor definition appears verbatim in `docs/vision.md` § 4
   and in CONSTITUTION.md § 17 as a structural rule.
3. Capability bundle authoring docs (when ADR-0008 graduates from
   `proposed` to `accepted` for first-party bundles) require
   namespace declaration and reference this ADR.
4. A contributor proposing a feature can answer "does this go in
   the urban code, in a capability bundle, or in the builder's
   own accelerator?" by reading VISION + this ADR + ADR-0008.
