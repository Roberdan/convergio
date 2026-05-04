---
id: 0020
status: proposed
date: 2026-05-01
topics: [vision, dispatch, evaluation, multi-vendor, cost]
related_adrs: [0002, 0009, 0012, 0016, 0018]
touches_crates: [convergio-durability, convergio-executor, convergio-mcp]
last_validated: 2026-05-01
---

# 0020. Model evaluation framework — the municipality's procurement office

- Status: proposed
- Date: 2026-05-01
- Deciders: Roberdan
- Tags: vision, dispatch, multi-vendor

## Context and Problem Statement

ROADMAP Wave 2 commits to multi-vendor routing (T4.04) — the
ability for `cvg dispatch` to pick a runner adapter (Claude
Agent SDK, Copilot, OpenAI, local shell, future vendors) per
task. The roadmap names "cost / latency / capability profiles"
as the routing inputs but does not specify *how those profiles
are derived, kept current, or trusted*.

Without that specification, multi-vendor routing collapses into
one of three failure modes:

1. **Vibes-based routing.** "Claude is good at code reviews,
   GPT is good at planning" — claims the operator made once
   and never re-checked. The model that was best in March may
   not be best in April; the framework that made the call
   cannot tell.
2. **Static config.** A YAML file says model A handles
   task-type X. The file rots. Quality, latency, cost all
   drift; the file does not. Every accelerator inherits the
   rot.
3. **Per-vendor lock-in.** The team picks one vendor and
   hard-codes it. Fast to ship, fatal to the long-tail thesis
   (ADR-0016) — long-tail solutions live or die on cost
   curves, and freezing a vendor freezes the cost curve.

In a real city, this exact problem is solved by the
**procurement office** of the municipality: tenders, vendor
ratings, SLA monitoring, periodic re-evaluation, structured
substitution when a vendor underperforms. Convergio needs the
same service.

## Decision Drivers

- **Long-tail economics.** ADR-0016 commits Convergio to driving
  marginal cost of *creation* and *coordination* to near-zero.
  Picking the wrong model for a task type adds 2-10× to the cost
  of a vertical accelerator over its lifetime. A measured choice
  beats a guessed choice by an order of magnitude.
- **Audit-driven memory.** ADR-0002 already gives us a tamper-
  evident record of every gate refusal and pass. Aggregating it
  per `(task_type, model, prompt_template)` is a query, not a new
  data source. The municipality already has the data; it lacks the
  reporting drawer.
- **Outcome > Output (ADR-0012).** The OODA validation loop is
  the natural anchor: a "good model" is one whose evidence
  passes Thor on the first attempt, with low cost and latency.
  We already measure pass rate per agent identity; we just
  haven't been measuring it per *model*.
- **Multi-vendor neutrality.** The point of the municipality is that
  it does not pick winners. The evaluation framework must be
  vendor-agnostic by construction: any registered runner adapter
  can be benchmarked against any task type.

## Considered Options

### Option A — Static YAML profiles per vendor

Each vendor adapter ships a static `model-profile.toml` declaring
its claimed capabilities, costs, and latencies. `cvg dispatch`
reads them at boot. Costs: vendors over-claim, no continuous
verification, ages badly. Same problem the framework was supposed
to solve.

### Option B — Per-call live A/B testing

Every `cvg dispatch` call runs N candidate models in parallel,
picks the best by some live metric. Costs: pays N× the API cost
for every dispatch, lighting money on fire. Acceptable only as a
calibration mode, not as production routing.

### Option C — Audit-derived continuous benchmark, plus periodic
calibration runs (chosen)

The framework consists of three parts:

1. A **task-type taxonomy** (what kinds of work agents do —
   `generate-test`, `review-code`, `write-docs`, `refactor`,
   `plan`, `summarise`, …) defined in a lightweight schema and
   carried on every task as `task.taxonomy_kind`.
2. A **continuous evaluation pipeline** that, on each Thor
   validation, attributes the verdict (Pass / Fail with reason /
   Cost / Latency) to the `(model, prompt_template, taxonomy_kind)`
   tuple. Aggregated nightly into `model_evaluations` view. No
   extra runs; existing work *is* the benchmark.
3. A **periodic calibration suite** (`cvg eval calibrate`)
   that runs a small fixed test set against every registered
   adapter quarterly to surface regressions before they show up
   in real work. Calibration is opt-in per environment because
   it costs real money.

The dispatch decision becomes a query against `model_evaluations`:
"for taxonomy_kind X, give me the model with the best
Cost-of-Pass over the last 30 days, weighted by latency budget B
and quality floor Q." Vendor agnostic, audit-grounded,
continuously up to date.

## Decision Outcome

Chosen option: **Option C**, because it reuses the audit chain
as ground truth (no new authoritative data source), pays no extra
inference cost on the happy path, and surfaces drift in
production rather than at calibration boundaries.

### Implementation status (2026-05-04)

Stays `proposed`. No `model_evaluations` or `task_taxonomy` table
exists, no `eval.record` / `eval.recommend` / `eval.report` /
`eval.calibrate` MCP actions are wired, and `cvg eval` is not a CLI
verb. The ADR depends on multi-vendor routing (T4.04) and smart Thor
(T3.02), neither of which has shipped. The framework is the intended
shape; nothing about it is implemented yet.

### Architecture sketch

#### New schema (Wave 2)

```sql
CREATE TABLE model_evaluations (
  id BLOB PRIMARY KEY,
  model_id TEXT NOT NULL,           -- e.g. "claude-opus-4-7"
  vendor_id TEXT NOT NULL,          -- e.g. "anthropic", "openai", "github-copilot"
  prompt_template_hash TEXT,        -- nullable; only for deterministic templates
  taxonomy_kind TEXT NOT NULL,      -- e.g. "generate-test"
  task_id BLOB NOT NULL REFERENCES tasks(id),
  outcome TEXT NOT NULL,            -- 'pass' | 'fail' | 'amend' | 'escalated'
  cost_usd_micros INTEGER,          -- nullable when self-hosted
  latency_ms INTEGER,
  evidence_size_bytes INTEGER,
  refusal_reason TEXT,              -- nullable
  observed_at TEXT NOT NULL
);

CREATE INDEX model_evaluations_kind ON model_evaluations(taxonomy_kind, observed_at DESC);
CREATE INDEX model_evaluations_model ON model_evaluations(model_id, observed_at DESC);

CREATE TABLE task_taxonomy (
  task_id BLOB PRIMARY KEY REFERENCES tasks(id),
  kind TEXT NOT NULL                 -- 'generate-test'|'review-code'|...
);
```

`tasks.taxonomy_kind` is populated either by the planner (when
`solve` decomposes a mission) or by the agent on `claim_task` if
not set. Closed taxonomy with extension via small ADR.

#### New MCP actions (Wave 2)

- `eval.record` — internal, called by Thor on validate verdict
- `eval.recommend` — given a `taxonomy_kind` + budget constraints
  (max cost, max latency, min quality floor), return ranked
  adapters
- `eval.report` — per-adapter trend report (last 30 / 90 days)
- `eval.calibrate` — run the calibration suite (slow, costs
  money, opt-in per environment)

#### Integration with `cvg dispatch` (T4.04)

The dispatcher consults `eval.recommend` for the task's
taxonomy_kind and the operator-configured budget. The chosen
adapter is recorded on the spawned `agent_processes` row so
post-hoc analysis can attribute outcomes back to the choice.

#### Integration with smart Thor (T3.02)

When Thor validates, it emits an `eval.record` row alongside the
existing audit row. No new data is produced; the existing pass /
fail signal is just attributed to a `(model, kind)` tuple.

### Failure modes the framework explicitly handles

- **Cold start**: a brand-new adapter has no historical data.
  The recommender falls back to the adapter's self-declared
  profile (option A as a *bootstrap*, not the *final state*),
  surfaces a `cold_start` flag in the recommendation so the
  operator knows the call is uncalibrated, and over-weights the
  first 50 task outcomes to converge fast.

- **Dogfood reality check on time-to-useful**: a single-user
  dogfood repo produces ~5–20 tasks/month per `taxonomy_kind`.
  Reaching the 50-outcome calibration threshold for *every*
  combination of `(adapter, kind)` realistically takes months
  of normal use, not days. The framework is *operational* in
  Wave 2, but *materially useful* (recommendations data-driven
  rather than self-declared) only after several months of audit
  accumulation. ADR is honest about that timeline; ROADMAP
  Wave 2 success criteria measure framework operability, not
  steady-state calibration.
- **Adversarial gaming**: an adapter that wins the benchmark by
  cherry-picking easy task kinds. Mitigation: the recommender
  groups by *taxonomy_kind*, not by vendor; an adapter that
  excels at `summarise` does not get used for `refactor`.
- **Cost obscurity**: not all adapters report cost the same way.
  Mitigation: `cost_usd_micros` is nullable; recommendations
  flag adapters with unknown cost rather than silently treating
  them as free.

## What this decision does not do

- It does not introduce LLM-as-judge evaluation. Quality is
  measured by gate Pass/Fail signal (objective) and by Thor's
  amendment requests (objective), not by another LLM rating
  the output.
- It does not centralise vendor billing. Cost is observed per
  task and aggregated; the operator still pays each vendor
  directly.
- It does not block on multi-vendor adapters. The framework is
  useful with a single adapter (it tracks one vendor's drift
  over time) but only valuable with two or more.
- It does not predict the future. It reports the past with
  confidence intervals; the operator chooses how to weight
  recency.

## Consequences

### Positive

- The procurement-office service of the municipality becomes
  operational. Vendor selection is data-driven from day one of
  Wave 2.
- Long-tail accelerator authors get vendor recommendations they
  can trust without having to run their own benchmarks.
- The audit chain pays a second dividend (the first was tamper
  evidence; the second is procurement memory) without a new
  authoritative data source.
- Cold-start handling means a vendor onboarding into Convergio
  reaches calibrated routing within ~50 task outcomes, not
  weeks.

### Negative

- Schema churn (`model_evaluations`, `task_taxonomy`). Wave 2
  has to absorb a non-trivial migration.
- The taxonomy is opinionated. Tasks that don't fit a known
  `kind` need a fallback path (`generic`); too many `generic`
  tasks make the recommender useless.
- Calibration runs cost real money. Operators who don't run
  them get marginally worse recommendations during quiet
  weeks.

### Neutral

- This ADR depends on T4.04 (multi-vendor routing) and on Wave 1
  smart Thor (T3.02). It cannot be implemented before either.
- Cost-of-Pass becomes a key metric for Convergio public
  reporting. We commit to publishing it for our own dogfood
  use, anonymised by vendor if required.

## Validation

This ADR is validated when:

1. After Wave 2 ships, `cvg eval recommend --kind generate-test`
   returns a ranked list with at least two adapters, each with
   non-zero observed sample size.
2. A demonstrably worse adapter (artificially degraded for the
   demo) drops in rank within 100 task outcomes, without manual
   intervention.
3. `cvg eval report --days 30` produces a per-vendor trend
   chart for the dogfood `convergio-local` repo's own work.
4. The 'long-tail accelerator author' (Wave 3 demo) makes one
   vendor decision based on `eval.recommend` rather than a
   guess, and the choice is reproducible by reading the audit
   chain.
