---
id: 0004
status: accepted
date: 2026-04-27
topics: [foundation, principles]
related_adrs: []
touches_crates: []
last_validated: 2026-04-30
---

# 0004. Three sacred principles: zero tolerance, security first, accessibility first

- Status: accepted
- Date: 2026-04-27
- Deciders: Roberto, Claude (sessione 5)
- Tags: foundation, principles

## Context and Problem Statement

Sessions 1–4 built the durability layer + bus + lifecycle + a basic
Layer 4. The pitch drifted between "durability layer",
"audit-grade local runtime", "agent leash". A founder-level question
forced a choice: **what is Convergio actually about?**

Roberto's framing — agents are not trustworthy, they lie about what
they did, leave technical debt, skip pieces of the plan — landed on
three principles that the product must enforce, not just talk about.

## Decision Drivers

- The product must be **opinionated**. A daemon that lets the agent
  do anything is just storage; a daemon that refuses substandard work
  is a leash.
- We are not building "agent governance dashboards". We are building
  the runtime that says no when the agent tries to ship junk.
- Roberto's life work (FightTheStroke) makes accessibility a
  non-negotiable. Our product reflects that.
- LLM-specific security threats (prompt injection, deceptive
  alignment) are real and will only get more important. Including
  them now positions us ahead of compliance vendors.

## Considered Options

1. **No explicit principles, build features and let them speak**.
   Already tried — pitch drifted four times in four sessions.
2. **Three principles as marketing taglines**, not enforced in code.
   Compliance vendors do this. Cheap, wrong.
3. **Three principles enshrined in the Constitution AND enforced via
   server-side gates**. Each principle has at least one current or
   planned `Gate` implementation. The principle is not a slogan, it's
   a `409 gate_refused`.

## Decision Outcome

Chosen: **3 — principles enshrined and enforced**.

### The three principles (full text in CONSTITUTION.md § Sacred principles)

| # | Principle | Enforcement |
|---|-----------|-------------|
| P1 | Zero tolerance for technical debt, errors, warnings (any language) | `NoDebtGate`, `ZeroWarningsGate` |
| P2 | Security first, including LLM security | `NoSecretsGate`, `PromptInjectionGate` (phase 1), `DepsAuditGate` (planned) |
| P3 | Accessibility first | `A11yGate` (phase 1), CLI `--format` modes (planned) |

### Operational consequences

1. Adding a feature that violates a principle is **not** a trade-off
   discussion — it's a rejected proposal. If the team disagrees,
   amend the Constitution first.
2. New gates land as code, not as docs. A principle without a gate
   is a wish.
3. Defaults are strict. Override is possible per deployment but
   writes a row to the audit chain so the auditor sees "team X
   disabled gate Y on date Z".
4. Pitch and README are aligned to these three. No more drift to
   generic "durability layer" language.

### Positive consequences

- Pitch becomes one sentence: "the first runtime that imposes
  quality, security and accessibility on AI-agent output, server
  side, before it ships."
- New contributors know what to optimize for: not "more features",
  but "stronger gates".
- Differentiation vs LangGraph / CrewAI / Temporal is concrete:
  they orchestrate, we **refuse**.

### Negative consequences

- Strict defaults mean LLMs will hit gate refusals often. Workflows
  must include a fix-loop where the agent reads the error message and
  retries. Higher token cost, longer wall-clock — accepted.
- Some legitimate-looking debt (e.g. `TODO(#42)`) is rejected. Issue
  tracker is the right place for debt, not the codebase. No
  exceptions in the default rule set.

## Links

- Code: `crates/convergio-durability/src/gates/no_debt_gate.rs`,
  `zero_warnings_gate.rs`
- Tests: `crates/convergio-durability/tests/no_debt_gate.rs`,
  `no_debt_gate_multilang.rs`, `zero_warnings_gate.rs`
- Constitution: [CONSTITUTION.md](../../CONSTITUTION.md) § Sacred principles
- Related: ADR-0001 (four-layer architecture), ADR-0002 (audit hash chain)
