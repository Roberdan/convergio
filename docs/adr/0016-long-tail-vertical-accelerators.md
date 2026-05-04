---
id: 0016
status: proposed
date: 2026-05-01
topics: [vision, product-strategy, capability-bundles, roadmap]
related_adrs: [0004, 0008, 0012, 0017, 0018, 0019]
touches_crates: []
last_validated: 2026-05-01
---

# 0016. Convergio is the shovel for the long tail of vertical AI accelerators

- Status: proposed
- Date: 2026-05-01
- Deciders: Roberdan
- Tags: vision, product-strategy

## Context and Problem Statement

Convergio shipped v0.1.x as a local-first daemon that refuses agent
work whose evidence does not match the claim of done. That framing —
"the leash for AI agents" — is technically accurate but strategically
narrow. It describes one of Convergio's behaviours, not its purpose.

Two market observations pull the framing wider.

1. **Volume.** Anthropic, OpenAI, Microsoft, Google, and a long list
   of OSS projects have made it cheap to *generate* a vertical
   solution to a niche problem. Costs of generation are collapsing
   monthly. The gold rush of 2024-2025 is producing thousands of
   demos and hundreds of half-shipped niche products.
2. **Reliability.** Almost none of those demos cross the line into
   production-grade software. The gap between "an LLM produced this"
   and "a customer can rely on this" remains brutally wide. Audit
   trails, accessibility, internationalisation, security baselines,
   regression discipline, multi-agent coordination — none of these
   come for free from a model.

This is the long-tail moment for vertical AI accelerators. The
analogy is direct: Amazon KDP collapsed the marginal cost of
*publishing* a book; Netflix and Spotify collapsed the marginal
costs of distributing film and music; the value migrated from the
top of the curve to the long tail of niches that, in aggregate,
exceed the blockbuster head. Chris Anderson's 2006 thesis is now
applicable to vertical software, but **only if the marginal costs
of building reliable software collapse first**.

The bottleneck is no longer making one solution. It is making
solutions reliably, repeatedly, in parallel, with zero regression
and zero lost work.

Convergio v0.1.x already shipped the primitives that close most of
the gap: durability, gates, audit, CRDT, workspace leases,
capability registry, MCP bridge, code graph, OODA-aware validation.
What is missing is the **public framing** and the **last-mile work**
that turns these primitives into a long-tail accelerator engine.

## Decision Drivers

- **The primitives we already shipped are long-tail tooling**, not
  leash tooling. The capability registry (ADR-0008), CRDT (ADR-0006),
  workspace leases (ADR-0007), and OODA loop (ADR-0012) are not
  what one builds when the goal is "stop one agent". They are what
  one builds when the goal is "let many builders ship many things
  reliably". The framing should match the architecture.
- **Microsoft alignment.** The author works inside an organisation
  that already runs the *Engineering Fundamentals Playbook* (ISE)
  and `microsoft/hve-core`. Both stop short of run-time enforcement.
  Long-tail framing slots Convergio between them as the runtime
  enforcer (ADR-0017).
- **Honesty.** v0.1.x friction logs (F26 plan growth from 14→38
  tasks; F33 status as derived state) and ADR-0012 ("outcome >
  output") already contain the long-tail thesis in scattered form.
  Naming it lets the team make decisions consistently.
- **The bet.** If we are wrong, the worst outcome is a perfectly
  reasonable single-purpose audit daemon. If we are right, the
  same code becomes the urban code for a class of vertical AI
  accelerators. Asymmetric upside, low downside.

## Considered Options

### Option A — Stay with the leash framing

Keep the README and ROADMAP language as-is. Continue shipping the
primitives, but never name what they add up to. Lets the project
remain easy to explain in one sentence ("a daemon that refuses
agent work that does not pass the gates"). Costs: contributors and
adopters cannot tell the difference between Convergio and any other
guard-rail tool. The capability registry, CRDT, leases, and OODA
loop look like over-engineering instead of urbanism.

### Option B — Rebrand entirely

Rewrite README/ROADMAP/CONSTITUTION around the long-tail framing.
Drop the "leash" language. Costs: invalidates v0.1.x positioning,
breaks the user-facing voice that earned the first contributors,
and risks marketing-first/code-second drift.

### Option C — Layered framing (chosen)

Add a `VISION.md` (separate from `ROADMAP.md`) that articulates the
long-tail thesis, the urbanism frame, and the relationship to
gstack/hve-core/ISE. Keep the leash language in README as a *safety
belt of the shovel* — accurate, narrow, technical. Update ROADMAP
v0.4+ around capability blocks, plan templates, runner adapters,
and the first vertical demo (`convergio-edu` reborn). Materialise
the framing in three companion ADRs:

- **ADR-0017** ISE + hve-core alignment
- **ADR-0018** Urbanism over architecture (Le Corbusier + Jane Jacobs)
- **ADR-0019** gstack as a thinking-stack capability

## Decision Outcome

Chosen option: **Option C**, because it preserves the technical
honesty of v0.1.x while giving the project a coherent forward story
that matches the architecture already in the repo.

### Implementation status (2026-05-04)

Stays `proposed`. This ADR is a vision/framing decision — the long-tail
thesis is the frame Convergio reasons inside, not a mechanism that
ships. The deliverables it commits to (ROADMAP wave restructure,
companion ADRs 0017/0018/0019, vertical accelerator demos) are tracked
in those individual artefacts; the frame itself is "real, the
buildings inside it are tracked elsewhere".

### What this decision commits us to

- [`docs/vision.md`](../vision.md) is rewritten to articulate the
  long-tail thesis and becomes the canonical answer to "why does
  Convergio exist?". README continues to answer "what does
  Convergio do?". They reference each other.
- The five sacred principles (CONSTITUTION § Sacred principles) are
  reframed as *building codes* in VISION.md — not slogans, but the
  reason the city is habitable. Their text in CONSTITUTION.md is
  unchanged.
- ROADMAP.md gains a four-wave structure (Wave 0 narrative + adapter,
  Wave 1 close P3 a11y + thinking-stack capability + plan templates,
  Wave 2 first five capability blocks, Wave 3 first vertical
  accelerator). v0.3 (smart Thor / OODA from ADR-0012) becomes
  Wave 1 work, not parallel work.
- Capability bundles (ADR-0008) are repositioned from
  "downloadable extensions" to **the long-tail distribution
  primitive**. The Ed25519 signed install-file is the unit of
  distribution; namespacing (`azure.*`, `auth.*`, `ui.*`, `a11y.*`,
  `payments.*`) becomes the urban zoning.
- The "Modulor" — the tuple `(task, evidence, gate, audit_row)` —
  is named and documented in VISION.md as the atomic unit of
  composition.

### What this decision does not change

- The technical commitment to local-first, single-user, SQLite-only.
  Long-tail does not mean cloud-native.
- The OODA loop (ADR-0012) and Thor-only-done (ADR-0011). The
  validator remains the only path to `done`.
- The 5 sacred principles. They become explicit as building codes;
  they are not weakened.
- Backwards compatibility commitments. v0.2.x remains the last
  pre-vision-framing release; v0.3 ships under the new framing
  but with no breaking schema or API changes attributable to this
  ADR.

### Concrete deliverables tracked under this ADR

- [ ] `docs/vision.md` rewritten with long-tail thesis + urbanism
      frame
- [ ] `README.md` updated with one-sentence long-tail framing in
      the hero copy and a pointer to `docs/vision.md`
- [ ] ADR-0017 (ISE + hve-core alignment)
- [ ] ADR-0018 (Urbanism over architecture)
- [ ] ADR-0019 (gstack as thinking-stack capability)
- [ ] `ROADMAP.md` restructured into four waves
- [ ] `PRD-001` written for the Claude Code adapter (Wave 0
      multi-agent coordination)
- [ ] Convergio plan materialised via `cvg plan create` (dogfood)

## Consequences

### Positive

- The architecture and the framing finally agree. v0.1.x primitives
  read as long-tail tooling instead of over-engineering.
- Microsoft-internal stakeholders can place Convergio next to ISE
  Playbook and hve-core without confusion (ADR-0017).
- Contributors get a clear "belongs / does not belong" signal:
  capability blocks go in their own repos; thinking frameworks go
  in gstack; engineering taste goes in hve-core. Convergio stays
  small.
- The long-tail thesis gives the four marginal costs (creation,
  coordination, distribution, discovery) explicit work targets in
  the roadmap. Each closure unlocks measurable capacity.

### Negative

- The project carries two reading levels: short technical (README,
  CONSTITUTION) and long strategic (VISION). Maintenance discipline
  is required to keep them consistent.
- The framing raises expectations. Once VISION.md says "shovel for
  the long tail", every honesty gap (especially P3 a11y) becomes
  more visible. ADR-0017 commits to closing those.
- The "urbanism" language is high-concept. It must be shipped with
  concrete deliverables (Wave 2 capability blocks, Wave 3 vertical
  accelerator) within a quarter, or it reads as marketing.

### Neutral

- The "leash" language survives in README and CONSTITUTION as a
  technical description of what the gate pipeline does. It is no
  longer the *purpose* of the project. This is the same shift Amazon
  did when it stopped calling itself a bookstore: the description
  did not become wrong, it became insufficient.

## Validation

This ADR is validated when:

1. `docs/vision.md` exists with the long-tail framing and
   references this ADR.
2. ADR-0017, ADR-0018, ADR-0019 exist and are referenced from
   `docs/vision.md`.
3. ROADMAP.md is restructured into the four-wave layout.
4. A reader who has not seen this conversation can answer the
   question "is Convergio a leash or a shovel?" with "both — the
   leash is how the shovel keeps its edge" after reading
   `README.md` + `docs/vision.md` + this ADR.
