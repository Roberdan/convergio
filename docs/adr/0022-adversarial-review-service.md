---
id: 0022
status: proposed
date: 2026-05-01
topics: [vision, governance, review, capability-bundles]
related_adrs: [0008, 0012, 0016, 0017, 0018, 0019, 0020]
touches_crates: [convergio-mcp, convergio-cli, convergio-durability]
last_validated: 2026-05-01
---

# 0022. Adversarial review as a municipal ombudsman service

- Status: proposed
- Date: 2026-05-01
- Deciders: Roberdan
- Tags: vision, governance, review

## Context and Problem Statement

The Wave 0 documentation set (this very PR) was reviewed by an
independent agent acting as outside voice before being committed.
The review surfaced 24 findings across six categories — internal
contradictions, unsustainable promises, political/social risks,
broken metaphors, roadmap gaps, technical errors. Eleven were
fixed before merge; six were deferred with explicit notes; the
rest were accepted as-is with rationale.

Without that step, the doc set would have shipped with at least
two political-risk items (gstack maintainer framing,
Microsoft-alignment without disclaimer) that would have been
embarrassing or worse on first public read. The adversarial
review *worked*. It was also entirely manual, ad-hoc, and
non-reproducible.

This ADR turns that one-off practice into a municipal service.

In urbanism the analogue is the **ombudsman**
or public advocate — an independent figure inside the
municipality whose job is to challenge the municipality *on behalf of the
city*, before decisions become irreversible. Not the public
prosecutor (Thor); not the building inspector (gates); a third
party whose function is structural skepticism. Every meaningful
urban authority has one. So should Convergio.

## Decision Drivers

- **Reproducibility.** The next strategic doc set (Wave 1
  smart-Thor design, Wave 2 capability blocks, Wave 3 vertical
  accelerator) needs the same review or it ships with the same
  blind spots. A pattern that lives only in this session's chat
  log is not a pattern.
- **Cost asymmetry.** A fix-before-merge review costs ~30
  minutes of agent time. A fix-after-public-PR cleanup costs
  hours of human reputation management. The cost ratio is
  10–100×; structuring the cheaper path is rational.
- **Modulor compatibility.** An adversarial review *is* a task
  that produces evidence, runs through gates, and lands in the
  audit chain. It composes. It does not require a new primitive
  outside the ADR-0018 Modulor.
- **Multi-vendor neutrality.** The reviewer should not be
  hard-coded to one model or one vendor. ADR-0020 (model
  evaluation) gives us a way to pick the best reviewer for the
  task; we use that.

## Considered Options

### Option A — Keep it informal

The author manually spawns an outside-voice agent before each
strategic PR. Costs: works once, dies the next time the author
is in a hurry. Documented but unenforced.

### Option B — Mandatory external service (Codex CLI / OpenAI)

The PR template requires a Codex CLI run (gstack `/codex
challenge` mode) to be attached as a comment. Costs: requires
every contributor to install Codex CLI, tying the review surface
to a specific external dependency. Breaks if the author is
offline or the vendor account is suspended.

### Option C — Convergio capability with vendor pluggability (chosen)

The adversarial review becomes a Convergio capability with a
documented contract:

- A new MCP action `governance.adversarial_review` takes a doc
  set + prompt template + budget, spawns a runner adapter
  selected by ADR-0020 model evaluation, collects findings as
  evidence, attaches them to a task in the originating plan,
  and lets the gate pipeline + Thor decide whether to accept
  the PR.
- The capability bundle namespace is `governance.*` — a new
  zone in the ADR-0018 city plan, alongside `azure.*`,
  `auth.*`, `ui.*`, `a11y.*`, `payments.*`. First-party,
  Ed25519-signed.
- A versioned prompt template lives in
  `docs/templates/adversarial-challenge.md` so the prompt
  itself is reviewable, version-controlled, and improvable.
- Two implementation strategies coexist:
  - **Today** (no new code beyond the prompt template):
    operator runs the adversarial review manually using
    `thinking.gstack` (`/codex challenge` via ADR-0019) or any
    spawned independent agent.
  - **Wave 2** (after runner adapter and model evaluation
    ship): the capability bundle automates the review,
    multi-vendor, audited.

Both strategies use the same prompt template so the qualitative
output is comparable across years.

## Decision Outcome

Chosen option: **Option C**, because it gives Convergio a
constitutional reflex (every strategic ADR / vision / PRD goes
through adversarial review) without locking the reflex to a
specific vendor and without delaying the practice until Wave 2
ships.

### Implementation status (2026-05-04)

Stays `proposed`. No `governance.adversarial_review` MCP action, no
`governance.*` capability namespace, and no
`docs/templates/adversarial-challenge.md` template ship today. The
practice is followed manually for strategic doc sets (per the worked
example) but the automation that would graduate this ADR depends on
the runner-adapter + capability + model-evaluation surfaces from
ADR-0019/0020/0008. The ombudsman frame is real; the automated
service is future work.

### What "strategic" means for trigger purposes

Adversarial review is required (advisory now, gating in Wave 2)
for:

- new ADRs at tier ≥ 2 (any ADR that touches CONSTITUTION,
  data schema, public API, or cross-crate contracts)
- changes to `docs/vision.md`, `ROADMAP.md`, `CONSTITUTION.md`
- new PRDs
- changes to capability namespacing (ADR-0018)

Not required for:

- code-only PRs that implement an already-reviewed PRD
- typo fixes, formatting changes, dependency bumps
- adding a new task to an existing plan

### The prompt template

Lives at `docs/templates/adversarial-challenge.md`,
version-controlled, reviewable. Initial template (v1):

```
You are an outside-voice reviewer for Convergio strategic
documents. The author has explicitly requested adversarial
challenge, not cheerleading.

Read the doc set listed below. For each of the six categories,
return at least one finding (or explicit "none found, here's
why"):

A. Internal contradictions (file:line citations required)
B. Unsustainable promises (claims the codebase cannot back today)
C. Political / social / legal risks
D. Metaphors that break under technical scrutiny
E. Roadmap gaps (dependencies, timeline realism, scope creep)
F. Technical errors (endpoints, schema, ADR refs that do not match
   the codebase)

Conclude with a verdict: "ship now" or "fix N items first" with
the items ranked.

Be brutal where needed. The author wants a real challenge.
```

The template evolves; every change is itself an ADR-tracked
decision (small ADR for prompt template revisions).

### MCP surface (Wave 2)

```
convergio.act { "type": "governance.adversarial_review",
                "params": { "files": [...],
                            "template_version": "v1",
                            "budget_usd": 0.50,
                            "min_findings_per_category": 1 } }
```

Returns a structured set of findings keyed by category +
file:line + severity, attached as `evidence_kind: doc-review` to
a task on the originating plan.

### How this dogfoods itself

This very ADR (ADR-0022) goes through adversarial review using
the manual / pre-Wave-2 path before its own merge. The findings
of that review land in the audit chain alongside the doc set,
proving the practice exists from the moment it is documented.

### Worked example — Wave 0a doc set

Wave 0a was reviewed in this session using the manual fallback.
Twenty-four findings, six categories, eleven fixes pre-merge.
The audit chain over the Wave 0a commit includes the review
findings as evidence rows, so future readers can trace what was
challenged, what was fixed, and what was deferred. This is the
test case for Wave 2 automation: the automated capability must
produce an output structurally compatible with what the manual
review produced for Wave 0a.

## What this decision does not do

- It does not require every PR to carry adversarial review —
  only strategic changes (defined above).
- It does not freeze the prompt template. Improvements are
  encouraged; every change is its own ADR.
- It does not pick a vendor. ADR-0020 picks the vendor; this
  ADR picks the *practice*.
- It does not replace human review. Adversarial review is
  *additional* to the existing PR review process, not a
  substitute.

## Consequences

### Positive

- The ombudsman service exists. Strategic decisions are
  challenged before they ship. This is the constitutional
  reflex Convergio's CONSTITUTION exists to enforce, applied to
  itself.
- New contributors get a reproducible quality bar: their
  strategic PR runs through the same review the project's own
  vision did.
- The audit chain accumulates a corpus of "what was challenged
  and what was fixed" that, by Wave 4, is itself trainable
  ground truth for future reviewers.

### Negative

- Operational cost. Pre-Wave-2 the operator runs the review
  manually; that is friction. Mitigation: the prompt template
  is short and the practice is required only on strategic
  changes.
- Prompt-template fragility. A poorly tuned template misses
  real risks or generates noise. Mitigation: every template
  revision is a small ADR; we version it like code.
- Risk of theatrical compliance. Reviews could be ritualised
  ("we ran it, fine") without acting on findings. Mitigation:
  Wave 2 gating makes acceptance verifiable; pre-Wave-2 the
  PR template requires explicit response to each finding.

### Neutral

- This ADR depends on ADR-0019 (thinking-stack capability) for
  the manual-fallback path and on ADR-0020 (model evaluation)
  for the Wave 2 automated path. It does not block on either.

## Validation

This ADR is validated when:

1. The Wave 0a commit includes both this ADR and a structured
   record of the adversarial review that was run against the
   rest of the Wave 0a doc set.
2. The next strategic doc set (Wave 1 design or Wave 2
   capability blocks) carries an adversarial-review evidence
   row in the originating plan.
3. By Wave 2 close, the manual fallback is no longer the
   default — the automated capability is the default and the
   manual path remains as documented backup.
