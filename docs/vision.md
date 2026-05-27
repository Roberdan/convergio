# Convergio — Vision

> *Convergio is not a house. It is a city, with its rules and its planning instruments.*

This file is the long-form rationale behind Convergio's direction. The
[`ROADMAP.md`](./ROADMAP.md) lists what ships when; the
[`CONSTITUTION.md`](./CONSTITUTION.md) lists what is non-negotiable; the
[`COMPLIANCE.md`](./COMPLIANCE.md) lists how the platform maps to EU
regulation; the [`docs/adr/`](./docs/adr/) folder records each major
decision. This document exists to answer a single question for every
reader, contributor, and agent:

**Why does Convergio exist, and what kind of artifact is it trying to be?**

---

## 0. One line

Convergio is the **open, local-first, EU-sovereign platform where AI
agents and humans converge on data both can trust** — the *good*
Palantir, built for places Palantir cannot go.

It is *one* artifact with two reinforcing layers:

- **The leash** (Layer 1) — server-side gates, hash-chained audit,
  the six sacred principles. Refuses unsafe work. Enforced today.
- **The ontology** (Layer 2) — typed schema registry, bitemporal
  store, provenance bundle, purpose registry, typed actions,
  scenario branching. The shared substrate AI and humans converge
  on. Status declared per primitive in [`COMPLIANCE.md`](./COMPLIANCE.md).

The leash makes the ontology trustable. The ontology makes the
leash productive. Removing either collapses the artifact into yet
another agent orchestrator. See
[ADR-0073](./docs/adr/0073-eu-sovereign-pivot.md) for the full
framing.

---

## 1. The bet

The world is filling up with people in love with their own AI solution to
their own narrow problem. The pace of change makes it impossible for any
single product to keep up. Two things follow from that observation.

**First**: the future belongs to whoever can build *specific, vertical, niche
solutions at scale* — the long tail Chris Anderson described in 2006, now
applied to AI-native software instead of books, films, and music. Amazon
KDP collapsed the marginal cost of publishing a book; Netflix collapsed the
marginal cost of distributing a film; Spotify collapsed the marginal cost
of a song. The same collapse is now happening to vertical software.

**Second**: the bottleneck is no longer *making* one solution — language
models do that adequately. The bottleneck is **making solutions reliably,
repeatedly, in parallel, with zero regression and zero lost work**.

Convergio is the *shovel* for that gold rush — the tool that lets a single
operator, or a small team, ship dozens of vertical accelerators (education,
research, healthcare compliance, public-sector workflows, accessibility
tooling, …) without each one rotting into a brittle prototype.

We are not betting on a model. We are betting on the *plumbing around the
model* — durability, gates, audit, multi-agent coordination, reusable
capability bundles. The plumbing is the long-tail multiplier.

---

## 2. The frame: urbanism, not architecture

Most AI infrastructure projects design *one building*. Convergio designs
*the urban code* — the rules, the standards, the modular units, the
infrastructure — that lets others build many buildings, none of which
collapse.

Two thinkers shape the frame.

**Jane Jacobs (1961, *The Death and Life of Great American Cities*)
shapes most of the frame.** Bottom-up emergence, eyes on the street,
mixed use, suspicion of master-plan thinking. Cities work because
ordinary people meet on sidewalks under transparent rules, not
because a planner has drawn perfect zones. Convergio is **strict on
a few non-negotiables and permissive everywhere else**. Eyes on the
street are the audit log. Mixed use is any agent, any vendor.
Bottom-up convergence is CRDT, not central authority.

**One technical concept from Le Corbusier (1948): the *Modulor*** —
a single, well-defined compositional unit that everything else
builds from. We adopt the Modulor as a structural rule (§ 4 below)
and we explicitly reject the rest of his master-plan posture. *Ville
Radieuse* was hostile to street-level innovation; long-tail vertical
accelerators die under that posture. Where Le Corbusier and Jacobs
disagree, Jacobs wins.

The result is Convergio as an *urban code* — a regulatory framework
with prefab structural elements — not a master plan. Builders pick
the building; we provide the standards, the inspection regime, the
registry of approved materials, and the cadastre.

---

## 3. Convergio is the municipality

The metaphor sharpens once you stop calling Convergio "the urban code"
and start calling it **the municipality** — the city hall,
the registry office that any builder of any building must walk through.

In a real city you do not call a developer to coordinate with sewers,
or a roof tiler to register your address. You walk into the municipal
office and the city's services are already there: the civil registry
records people and entities, the cadastre records who owns what, the building
codes constrain how you may build, the urban services (roads, water,
power, emergency response) are already laid in the ground, the urban
planning office decides where new neighbourhoods grow.

Convergio is the local municipality for a city of agents, humans, models,
and vertical accelerators. Local — single-machine, single-user,
SQLite-only — but federated to the rest of the world by shared
standards (ISE Playbook, hve-core, MCP, capability signing).

| In a real city | In Convergio | ADR / file |
|---|---|---|
| Municipality (city hall) | the daemon | ADR-0001 |
| Civil registry | agent registry | ADR-0009 |
| Cadastre | hash-chained audit log | ADR-0002 |
| Building codes | five sacred principles + Modulor | ADR-0004, 0018 |
| Building permits (technical rules + permit to build) | the ADRs + gate pipeline (HTTP 409) | docs/adr/, ADR-0004 |
| Construction lease | workspace leases | ADR-0007 |
| Building inspector | Thor validator | ADR-0011, 0012 |
| Streets, sewers, power grid | agent message bus, HTTP, MCP, runner adapters | Layer 2, Layer 0–3 |
| Certified building materials | capability bundles, Ed25519 signed | ADR-0008 |
| Zoning (general master plan) | capability namespaces (`azure.*`, `auth.*`, `ui.*`, `a11y.*`, `payments.*`) | ADR-0018 |
| External standards (ISO / national codes) | ISE Playbook + hve-core | ADR-0017 |
| Architects | gstack thinking layer + agent runners | ADR-0019 |
| Procurement office (model selection) | model evaluation framework | ADR-0020 |
| Strategic programming (multi-year goals + KPIs) | OKR on plans (objective + key results) | ADR-0021 |
| Public ombudsman (challenge before approval) | adversarial-review service | ADR-0022 |
| Buildings | vertical accelerators (`convergio-edu`, …) | ROADMAP Wave 3 |
| Squares and convergence points | MCP help surface, bus topics, plan-scoped messages | ADR-0009 |
| Design Week (recurring showcase event) | accelerator demo at a stable cadence | ROADMAP Wave 3+ |

The name itself is the manifesto. **Convergio marks a point where
entities arriving from different directions meet:** agents, humans,
codes, models, vendors. The public square does not choose what the
parties say to each other — it sets the rules so they can meet
without hurting each other. Everything else is urbanism.

---

## 4. The Modulor

> **The Modulor of Convergio is the tuple `(task, evidence, gate, audit-row)`.**

Le Corbusier's *Modulor* (1948) was a system of human-scale
proportions that let any architect produce a coherent building. Our
Modulor does the analogous job: any builder of any accelerator works
in the same atomic unit, so pieces compose across the city.

Everything composes from this unit:

- A **skill** is *N* tasks.
- A **wave** is *M* tasks that ship together.
- A **plan** is a DAG of tasks.
- A **vertical accelerator** (e.g. `convergio-edu`) is a plan template
  parameterised by domain, plus a curated set of capability blocks, plus
  domain-specific gates.
- A **city** is the population of accelerators built on the same municipality.

The Modulor is not a metaphor. It is the literal data shape:

| Field | Storage | Why it matters |
|---|---|---|
| `task` | `tasks` table, ADR-0001 | atomic unit of agreed-upon work |
| `evidence` | `evidence` table | what the agent claims it did, in machine-readable form |
| `gate` | `gates/*.rs`, ADR-0004 | the inspection regime — refuses with HTTP 409 if non-negotiables are violated |
| `audit_row` | `audit_log` table, hash-chained, ADR-0002 | tamper-evident memory of every state change |

If you want to add behaviour to Convergio that does not decompose into
this shape, ask first whether the municipality can absorb it, or whether
you are designing a building inside the registry office.

---

## 5. Planning: how the city grows

A city is not a snapshot. It is a plan in time. Urban planning
distinguishes four levels of planning that map cleanly onto Convergio
and protect it from a common failure mode of agent-driven projects:
*shipping buildings without a plan, then losing the city to unauthorized
construction*.

| Planning level | In urban planning | In Convergio | Lives in |
|---|---|---|---|
| **Strategy** | strategic plan | this document | `docs/vision.md` |
| **General regulation** | general zoning plan | the four-wave roadmap | `ROADMAP.md` |
| **Technical norm** | technical implementation rules | the ADRs | `docs/adr/` |
| **Operational plan** | detailed operational plan | a `plan` in the daemon | SQLite + `cvg plan create` |

Every vertical accelerator must traverse all four. **Strategy**
explains *why* it exists. **Regulation** places it in the city's
growth plan. **Norms** fix its constraints (which gates apply, which
capabilities are required, which evidence kinds are mandatory).
**Operational plan** is the DAG of tasks the agents actually execute,
materialised in the daemon, gated, audited, validated.

Skipping a level is what produces *unauthorized construction* — the AI-era
equivalent of buildings that look fine until inspection day. In our
field today, unauthorized construction is the norm: thousands of demos shipped
without a plan they fit into, without norms that constrain them,
without operational tracking that proves they did what they claimed.
Convergio refuses to host unauthorized construction. The municipality does not stamp
permits for buildings outside the plan.

**Design Weeks** of real cities (Milan, Paris, London, Tokyo) are
a useful loose analogue: temporary events on permanent
infrastructure. The streets are the same the day before and the
day after; the city becomes a stage without rewriting itself. We
borrow the *temporary-event-on-permanent-infrastructure* pattern,
not the marketing connotation: an accelerator demo is a
demonstrable composition of stable primitives, not a one-week
festival. Wave 3 (`convergio-edu` reborn) is the first such
demonstration. Whether subsequent demonstrations land at a regular
cadence is a community-effort question we do not pre-commit to.

The four marginal costs of § 9 below are the *services* the municipality
must drive to near-zero so that more builders can build in less time.
Closing them is the real work of the next twelve months.

---

## 6. The five sacred principles, restated

The non-negotiables in [`CONSTITUTION.md`](./CONSTITUTION.md) are not
slogans. They are the reason cities are habitable.

| # | Principle | What it means in code | Status |
|---|---|---|---|
| **P1** | Zero tolerance for technical debt | `NoDebtGate` + `ZeroWarningsGate` refuse evidence containing `TODO`, `FIXME`, `unwrap()`, `console.log`, ignored tests, `as any`, etc. across 7 languages | enforced |
| **P2** | Security first | `NoSecretsGate` enforces no secrets in evidence; LLM-specific threats (prompt injection) tracked in v0.3+ | partial |
| **P3** | Accessibility first | UI accessible, CLI usable without colour/animation, evidence must demonstrate a11y when UI is touched | **planned, not yet enforced — honesty gap** |
| **P4** | No scaffolding only | `NoStubGate` refuses self-admitted stubs; `WireCheckGate` (real wiring proofs) v0.3+ | partial |
| **P5** | Internationalization first | Fluent bundles EN+IT shipped together, coverage gate in `convergio-i18n` | enforced |

These are the *building codes*. Anyone can build any vertical. Nobody
can ship a vertical that violates these. The audit chain proves it.

The honesty gap on P3 is open and tracked. Closing it is one of the
first deliverables in [`ROADMAP.md`](./ROADMAP.md) v0.3.

---

## 7. The OODA loop is the operating principle

ADR-0012 ("OODA-aware validation") is the spine of Convergio's run-time
behaviour. Every task that wants to reach `done` traverses an
Observe-Orient-Decide-Act loop with three actors:

- **Agent** acts (writes code, attaches evidence, claims `submitted`)
- **Thor** observes (reads evidence + project context + past refusals
  via the learnings store) and orients on real-world pipeline signal
  (cargo test, lint, ADR coherence)
- Together they **decide**: Pass / Fail / NeedAmendment
- The accepted decision triggers the next **act** — promotion to
  `done`, refusal with diagnostic, or human escalation after three
  failed rounds

This is the same model used by aviation, surgery, and air traffic
control: outcome > output. Convergio extends it with a hash-chained
audit row for every transition, so the loop is not just executed,
it is *proven* to have run.

The OODA loop is also the urbanism of the system: every building
goes through inspection (Thor), the inspector is informed by
historical patterns (learnings store), and after a bounded number
of disputed inspections the local authority (human) settles.
Nothing is left to dispute forever.

---

## 8. Three layers, three functions, one machine

Convergio is intentionally one of three layers in a complete
agent-driven SDLC. The other two already exist; we integrate, we do
not duplicate.

| Layer | What it is | Owner | What it does |
|---|---|---|---|
| **Think** | [gstack](https://github.com/garrytan/gstack) | community | strategic planning skills (`/plan-ceo`, `/plan-eng`, `/plan-design`, `/codex`, `/autoplan`); pure Markdown skills consumed by Claude Code, Cursor, Codex CLI |
| **Engineer** | hve-core (Microsoft, internal/public) + ISE Engineering Fundamentals Playbook ([microsoft.github.io/code-with-engineering-playbook/ISE](https://microsoft.github.io/code-with-engineering-playbook/ISE/)) | Microsoft | Copilot agent prompts, instructions, skills; engineering checklists, NFR taxonomy, working agreements |
| **Govern + execute** | Convergio | this repo | durability, gates, audit, multi-agent coordination, capability bundles, runner adapters |

The integration model:

- **Convergio vendors gstack as a thinking-stack capability** (ADR-0019),
  updatable via `cvg capability update gstack-thinking`. The skill
  surface is wrapped to honour Convergio's principles (e.g. P5 i18n
  output) while preserving gstack's `/plan-*` semantics.
- **Convergio aligns explicitly with the ISE Playbook + hve-core**
  (ADR-0017). The five sacred principles map onto a subset of the 14
  ISE NFR. Convergio is the *runtime enforcer* of the principles ISE
  describes in checklists and hve-core transmits via prompts. This is
  the angle that "complements without competing" with Microsoft work.
- **Convergio orchestrates execution** through Layer 4 (planner / Thor
  / executor) and runner adapters that talk to Claude Agent SDK,
  Copilot, OpenAI, or local shell — abstracting the model from the
  workflow.

`gstack` thinks. `hve-core` and the ISE Playbook teach the agent
how to engineer. Convergio enforces, audits, parallelises, ships.

---

## 9. The four marginal costs of the long tail

For Convergio to be the shovel of the long tail, it has to drive
*four* marginal costs to near-zero:

| Marginal cost | Tool | Status |
|---|---|---|
| **Creation** of a new vertical accelerator | parameterised plan templates + thinking-stack overlay + skill packs | partial — primitives present, templates absent |
| **Coordination** of multiple agents on one accelerator | CRDT (ADR-0006) + workspace leases (ADR-0007) + agent registry + bus | partial — primitives shipped, no client adapter wires it yet |
| **Distribution** of a finished accelerator | capability bundles, Ed25519 signed install-file (ADR-0008) | partial — local only; remote registry deferred |
| **Discovery** of the right accelerator | capability registry + manifest schema + `convergio.help` MCP surface | partial — registry shipped, search/recommendation absent |

The roadmap closes these in order. Each closure unlocks a tier of
long-tail capacity:

1. close *Coordination* → many agents can work on one accelerator
2. close *Creation* → one builder can produce many accelerators
3. close *Distribution* → one accelerator can reach many users
4. close *Discovery* → the right accelerator finds the right user

We are at the tail end of step 1. The narrative has not caught up
to the code.

---

## 10. Lego, not buildings

Vertical accelerators are not built from scratch. They compose from
*capability blocks* — signed, isolated, namespaced packages that the
Convergio capability registry (ADR-0008) installs and verifies.

The first wave of first-party capability blocks (planned, not yet
shipped):

| Block | Namespace | What it provides |
|---|---|---|
| `azure-voice` | `azure.*` | Speech-to-Text + Text-to-Speech via Azure Cognitive Services |
| `auth-entra` | `auth.*` | Microsoft Entra ID identity, OIDC flows, scoped tokens |
| `ui-fluent` | `ui.*` | Fluent UI components, accessible by default |
| `a11y-axe` | `a11y.*` | axe-core integration, gate-domain check |
| `payments-stripe` | `payments.*` | Stripe checkout, webhook validation |

A vertical accelerator is *the right composition* of these blocks
plus a plan template plus domain-strengthened gates. Building a new
accelerator should feel like assembling Lego, not pouring concrete.

The constraint: every block enforces the five sacred principles by
construction. There is no `azure-voice` block that ships an
inaccessible UI or hardcoded English.

---

## 11. Tone and tells

A reader should be able to tell, within thirty seconds, whether a
piece of work belongs in Convergio or somewhere else. The tells:

**Belongs**
- It is a non-negotiable safety belt around agent work
- It composes from `(task, evidence, gate, audit-row)`
- It scales the marginal cost of one of the four long-tail levers
- It documents itself in an ADR that maps to the urban code

**Does not belong**
- It is a single vertical solution (those go in their own repo as
  capability bundles consumed by Convergio)
- It is a thinking framework (those live in gstack and are imported
  via the thinking-stack capability)
- It is engineering taste at the prompt level (that lives in
  hve-core and the ISE Playbook)
- It bypasses the gate or audit chain to ship faster (that is
  exactly what the leash exists to refuse)

When in doubt, ask: *would a Le Corbusier drawing fit, or does it
belong on a Jacobs sidewalk?* If it belongs on the sidewalk, it
goes in a capability bundle, not in this repo.

---

## 12. The next decisions

The work that materialises this vision is tracked in
[`ROADMAP.md`](./ROADMAP.md) as four waves:

- **Wave 0** (now): the urban code (this file + ADR-0016/0017/0018/0019)
  and the first multi-agent coordination client (PRD-001 Claude Code
  adapter)
- **Wave 1**: close the P3 a11y honesty gap, ship the
  thinking-stack capability, ship parameterised plan templates
- **Wave 2**: ship the first five capability blocks (Azure Voice,
  Entra, Fluent, axe, Stripe)
- **Wave 3**: ship the first end-to-end vertical accelerator
  (`convergio-edu` reborn) demonstrating Lego composition end-to-end

Every wave has a measurable success criterion. None of them ships
unless the gate pipeline accepts the evidence. We dogfood the city
we are designing.

---

## 13. What we are not

To prevent scope drift:

- **Not a hosted platform.** Local-first, single-user, SQLite-only
  remains the architectural commitment. Multi-tenant features are
  out of scope.
- **Not a model.** We do not train, host, or distribute language
  models. We orchestrate them.
- **Not a UI framework.** `ui-fluent` is one capability block among
  many. Convergio itself ships only a CLI.
- **Not a planning tool.** gstack is the planning tool. Convergio
  consumes its output and makes it executable.
- **Not a single vertical.** `convergio-edu` is *the demo*, not
  *the product*. The product is the urban code.

---

## 14. Closing

Convergio is the bet that the next decade of vertical AI software is
built by *fewer people, faster, with higher quality, on shared
infrastructure that refuses to ship broken work*. The five sacred
principles are the building codes. The OODA loop is the inspection
regime. The capability registry is the materials registry. The
audit chain is the cadastre.

Humans and agents converge on this shared city. **Three of the five
building codes (P1 zero-debt, P5 i18n, partial P2 secrets) are
mechanical today: the gate refuses with HTTP 409 and the audit row
proves it. Two (P3 accessibility, P4 wire-check) are still
aspirational at runtime — they live in the constitution but the
gate that enforces them ships in v0.3 (Wave 1).** The bet is that
this gap closes within the calendar year, not that it is already
closed. We are honest about the building codes that are mechanical
*today* and the ones that are *promised by date X*. Both kinds are
in the constitution; only the first kind makes the city safe.

— last reviewed 2026-05-01, see ADR-0016 / 0017 / 0018 / 0019 /
0020 / 0021 for the decisions that operationalise this document.
