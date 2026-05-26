---
id: 0017
status: proposed
date: 2026-05-01
topics: [vision, microsoft-alignment, principles, gates]
related_adrs: [0004, 0005, 0011, 0012, 0016]
touches_crates: [convergio-durability]
last_validated: 2026-05-01
---

# 0017. Convergio aligns with ISE Engineering Fundamentals + hve-core as the runtime enforcer

- Status: proposed
- Date: 2026-05-01
- Deciders: Roberdan
- Tags: vision, microsoft-alignment

> **Disclaimer**. Convergio is a personal open-source project. It is
> not a Microsoft product, not affiliated with Microsoft, and not
> endorsed by Microsoft. The ISE Engineering Fundamentals Playbook is
> used under CC BY 4.0; `microsoft/hve-core` is used under MIT.
> Citations and the alignment described in this ADR reflect the
> author's reading of public documentation, not any internal Microsoft
> position. Any change to this disclaimer is itself an ADR change.

## Context and Problem Statement

Two Microsoft-affiliated bodies of work are adjacent to Convergio:

1. **ISE Engineering Fundamentals Playbook**
   ([microsoft.github.io/code-with-engineering-playbook/ISE](https://microsoft.github.io/code-with-engineering-playbook/ISE/))
   — a CC BY 4.0 prescriptive checklist of engineering practices
   used by Industry Solutions Engineering on customer-facing
   projects. Covers Working Agreements, Definition of Done/Ready,
   Source Control, Code Reviews, Automated Testing, CI/CD, Design,
   Developer Experience, Documentation, Observability, Security,
   and 14 Non-Functional Requirements (Accessibility, Availability,
   Capacity, Compliance, Data Integrity, DR/Continuity,
   *Internationalization & Localization*, Interoperability,
   Maintainability, Performance, Portability, Reliability,
   Scalability, Usability + Privacy).
2. **`microsoft/hve-core`** — a Microsoft-internal/public collection
   of Copilot agents, instructions, prompts, and skills consumed
   via a VS Code extension. Includes
   ISE Engineering Fundamentals (external) with code-level
   principles: DRY, Simplicity First, Surgical Changes, Approach
   Proportionality. Pipeline is RPI (Research → Plan → Implement →
   Review), strictly sequential and human-driven via context-clear.

Both are valuable. Both stop short of run-time enforcement. ISE is
*prescriptive checklists* a team should follow; hve-core is *prompts
the agent should think with*. Neither runs the test, refuses the
PR with HTTP 409, or signs the audit row.

Convergio v0.1.x ships exactly that missing layer: gates that refuse
evidence violating the principles, an audit chain that proves the
refusal, and an OODA loop that lets the agent and validator
converge or escalate. There is a clean bridge to be drawn.

The author also works inside the same organisation that owns ISE
and hve-core. Convergio's vision (ADR-0016) requires that the
position of "complementary runtime enforcer" is documented
explicitly, both for the public framing and for internal political
clarity. "Not duplicating", "not competing", "not bypassing" must
be statable in one paragraph.

## Decision Drivers

- **Honesty about overlap.** Convergio's five sacred principles are
  not novel — most map onto ISE NFRs. Pretending otherwise invites
  the legitimate question "why does this exist instead of Microsoft
  internal tooling?"
- **Honesty about the gap.** ISE checklists and hve-core prompts do
  not refuse work at runtime. An agent that knows the checklist can
  still violate it. Run-time enforcement is the missing layer, not
  a duplicated one.
- **Closing the P3 a11y honesty gap.** Convergio CONSTITUTION § 3
  promises Accessibility-first; ADR-0004 marks `A11yGate` as
  *planned*, not implemented. ISE's NFR Accessibility section is
  prescriptive; this ADR commits Convergio to *enforcing* that
  prescription via a real gate, finally.
- **Strategic positioning.** "Microsoft Inner Source compatible
  runtime enforcer" is a defensible niche that does not require
  Convergio to compete with model providers, vendor SDK teams, or
  Copilot itself.

## Considered Options

### Option A — Stay silent on alignment

Continue to define principles in CONSTITUTION.md without referring
out to ISE or hve-core. Costs: every Microsoft-internal reviewer
asks the alignment question; the answer lives in private docs only.
Loses the chance to position Convergio as ISE-compatible runtime
infrastructure.

### Option B — Adopt ISE NFR list verbatim

Replace the five sacred principles with the 14 ISE NFR. Costs: the
five sacred principles are intentionally **opinionated** — they
declare which non-negotiables Convergio enforces with HTTP 409, not
the broader hygienic checklist a team should follow. Adopting the
14 verbatim weakens the gate semantics ("everything is a principle"
means nothing is a principle).

### Option C — Document the mapping; commit to gate parity where it matters (chosen)

Keep the five sacred principles as the gate-level non-negotiables.
Document the mapping to ISE NFRs explicitly. Commit to closing the
P3 a11y honesty gap with a real `A11yGate` (planned for v0.3,
ROADMAP Wave 1). Position hve-core as the *prompt source* for an
agent runner adapter and link `ISE Engineering Fundamentals` (DRY
/ Simplicity / Surgical / Proportionality) as advisory taste rules
that may be enforced via an optional `EngineeringFundamentalsGate`
per-domain.

## Decision Outcome

Chosen option: **Option C**, because it preserves Convergio's
opinionated gate semantics while making the alignment with adjacent
Microsoft work explicit and useful.

### Implementation status (2026-05-04)

Stays `proposed`. This ADR is a positioning thesis — the alignment
table maps the five sacred principles against ISE NFRs and hve-core
concepts; the only mechanism it commits to (a real `A11yGate` for the
P3 honesty gap, an optional `EngineeringFundamentalsGate`) is tracked
under ROADMAP Wave 1 / Wave 2 and should graduate this ADR when those
gates ship. The frame is real; the buildings inside it are tracked
elsewhere.

### Mapping the five sacred principles ↔ ISE NFR + hve-core

| Convergio | ISE Playbook | hve-core | Convergio gate | Status |
|---|---|---|---|---|
| **P1** Zero-debt / zero-warnings | Code Reviews § Linters/Code Analyzers; Source Control § main-shippable | `ISE Engineering Fundamentals` Simplicity First, Surgical Changes | `NoDebtGate` (7 langs), `ZeroWarningsGate` | enforced |
| **P2** Security-first | Security chapter + NFR Privacy/Compliance | (no direct equivalent) | `NoSecretsGate`; `PromptInjectionGate` v0.3+ | partial |
| **P3** Accessibility-first | NFR Accessibility | (no direct equivalent) | `A11yGate` — **planned, not yet enforced** | honesty gap |
| **P4** No scaffolding only | DevEx § F5-to-run; CI/CD § main-shippable | Surgical Changes | `NoStubGate`; `WireCheckGate` v0.3+ | partial |
| **P5** Internationalization-first | NFR Internationalization & Localization | (no direct equivalent) | `convergio-i18n` Fluent + coverage gate | enforced |

Two ISE NFRs Convergio explicitly does **not** turn into gates:
*Performance* and *Scalability*. These are project-specific tunables;
making them gates would force every Convergio-built accelerator into
the same performance envelope. They remain the responsibility of the
accelerator's domain-specific gates.

### Closing the P3 a11y honesty gap

This ADR commits Convergio to implementing `A11yGate` as part of
ROADMAP Wave 1 (v0.3 timeframe). The minimum viable enforcement is:

- For evidence kinds that touch UI (`html_output`, `screenshot`,
  `component_render`): scan for axe-core violations of severity
  `serious` or `critical`, refuse if any present.
- For evidence kinds that touch CLI (`cli_output`, `tui_render`):
  refuse if output cannot be parsed without ANSI colours
  (i.e. requires colour to convey meaning).
- For evidence kinds that touch documentation: enforce alt-text on
  images, semantic heading structure, no colour-only emphasis.

The capability block `a11y-axe` (ROADMAP Wave 2) ships axe-core as
the standard scanner; the gate is its first consumer.

### hve-core integration model

hve-core is repositioned (in Convergio's view) as a **prompt
catalogue for runner adapters**. When Convergio's runner adapter
for Copilot ships (ROADMAP Wave 2-3, dependent on
T4.04 multi-vendor routing), it consumes the agent definitions,
instructions, and skills in `microsoft/hve-core` as the *prompt
configuration* for the spawned agent. The Convergio side enforces
runtime; hve-core supplies the prompt it enforces against.

The `ISE Engineering Fundamentals` taste rules (DRY, Simplicity
First, Surgical Changes, Approach Proportionality) become an
optional `EngineeringFundamentalsGate` that capability bundle
authors may enable for accelerators where they apply. They do not
become sacred principles — they are taste, not safety.

### Public framing in docs/vision.md and README

`docs/vision.md` § 6 ("Three layers, three functions, one machine")
already names this alignment. README.md hero copy is updated to
include a one-line pointer:

> *Convergio is the runtime enforcer of the principles ISE
> Engineering Fundamentals describes in checklists and hve-core
> transmits via Copilot prompts.*

This sentence is the elevator pitch for Microsoft-internal
audiences. It is accurate without overstating.

### What this decision does NOT do

- It does not adopt the 14 ISE NFRs as Convergio principles.
- It does not vendor hve-core into this repo. The integration is by
  reference (agent runner adapter, capability block consumption),
  not by copy.
- It does not require Microsoft sign-off. ISE Playbook is CC BY 4.0;
  hve-core is MIT (per its repo). Citation and alignment are
  permitted without coordination.
- It does not commit to upstreaming Convergio work to ISE or
  hve-core. Their stack (PowerShell + VS Code extension + Markdown)
  is incompatible with ours (Rust + HTTP daemon + SQLite).

## Consequences

### Positive

- A single short paragraph (the elevator pitch above) places
  Convergio cleanly in the Microsoft tooling landscape.
- The P3 a11y honesty gap gets a concrete close-out plan instead of
  a perennial "planned" status.
- Capability bundle authors get a known catalogue of taste rules
  (`ISE Engineering Fundamentals`) they can opt into, without
  Convergio becoming opinionated about taste.
- Copilot becomes a first-class runner alongside Claude and OpenAI
  via the runner adapter pattern (ROADMAP Wave 2-3).

### Negative

- Convergio inherits some of ISE's reputation. If ISE checklists
  are perceived as enterprise-heavy, alignment to them carries
  that perception. Mitigation: the elevator pitch makes clear
  Convergio is the *runtime* layer, not the checklist itself.
- A11y gate work is real engineering (axe-core integration, CLI
  ANSI parsing, evidence-kind detection). v0.3 ROADMAP Wave 1 has
  to absorb this scope or push P5-only commitment.

### Neutral

- The mapping table in this ADR will need updating if ISE Playbook
  evolves. Mitigation: the table cites the playbook URL and links
  to specific sections; an annual review is added to the operations
  cadence.

## Validation

This ADR is validated when:

1. The mapping table appears in `docs/vision.md` § 6 ("The five
   sacred principles, restated").
2. ROADMAP Wave 1 has an explicit `A11yGate` deliverable with
   axe-core dependency.
3. README.md hero copy includes the alignment elevator pitch.
4. A new contributor reading
   [microsoft.github.io/code-with-engineering-playbook/ISE](https://microsoft.github.io/code-with-engineering-playbook/ISE/)
   and Convergio CONSTITUTION.md back-to-back can identify what
   Convergio adds (runtime enforcement + audit) vs what ISE
   describes (checklist + working agreements) without further
   explanation.
