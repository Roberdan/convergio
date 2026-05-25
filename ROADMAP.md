# Roadmap

Convergio is the **shovel for the long-tail of vertical AI
accelerators** — see [`docs/vision.md`](./docs/vision.md) and
[ADR-0016](./docs/adr/0016-long-tail-vertical-accelerators.md). The
*leash* (refusing agent work whose evidence does not match the
claim of done) is the safety belt of the shovel — the gate
pipeline, hash-chained audit, and OODA loop that keep the edge
sharp under industrial use.

For day-to-day status see [`STATUS.md`](./STATUS.md). For the
vision see [`docs/vision.md`](./docs/vision.md). For non-negotiables
see [`CONSTITUTION.md`](./CONSTITUTION.md).

This document organises work into **four waves**. Each wave has a
measurable success criterion and a single-sentence pitch. Older
release-train numbers (v0.1.x / v0.2.0 / v0.3 / v0.4) map onto the
waves; the wave is the unit we communicate, the version is the
unit we tag.

> **Current released line: v0.3.31** (2026-05). v0.2.x is closed
> with two follow-ups deferred (PR-stack i18n, file-trim cleanup).
> Wave 0a docs are in. Wave 0b (Claude Code adapter) is partially
> done — skill scaffolding is in the repo, hooks + registration
> are not. v0.3 also shipped a **fleet abstraction** that was not
> in the original 4-wave plan — see "Shipped in v0.3.x beyond the
> original plan" below. Run `cvg session resume` for live state.
>
> **For the v1.0 production-ready plan** (12 workstream, ~13
> weeks with 3 parallel agents) see
> [`docs/plans/v1.0-production-ready.md`](./docs/plans/v1.0-production-ready.md).

---

## Shipped — v0.1.x and v0.2.0

The runtime that v0.1.0 advertised, plus everything the
office-hours dogfood session proved was missing.

- [x] SQLite-backed daemon, localhost HTTP API, `cvg` CLI
- [x] Plans / tasks / evidence / audit_log
- [x] Hash-chained audit verification (`GET /v1/audit/verify`,
      ADR-0002)
- [x] Gate pipeline: PlanStatus, Evidence, NoDebt (7 languages),
      NoStub, ZeroWarnings, NoSecrets, WaveSequence
- [x] Persistent plan-scoped agent message bus (Layer 2)
- [x] Agent process spawn / heartbeat / watcher (Layer 3)
- [x] Reaper for stale `in_progress` tasks
- [x] Reference planner + Thor + executor tick (Layer 4)
- [x] CRDT actor/op store, workspace leases, patch proposals,
      merge arbiter (ADR-0006, ADR-0007)
- [x] Local capability registry + Ed25519 signed `install-file`
      (ADR-0008)
- [x] Local shell runner adapter proof
- [x] Setup + doctor + service + MCP bridge + adapter snippets
- [x] English / Italian i18n with coverage gate
- [x] PR template with `## Files touched` manifest
- [x] CONSTITUTION § 13 agent context budget + CI audit
- [x] CONSTITUTION § 15 worktree discipline
- [x] CONSTITUTION § 16 legibility score + CI audit
- [x] `cvg pr stack` (PR queue dashboard, read-only)
- [x] `cvg task create` with rich fields
- [x] `cvg plan create / list / get` + `cvg task list` honor
      `--output human|json|plain`
- [x] `examples/claude-skill-quickstart/` — runnable before/after
      demo (no LLM required)
- [x] Friction-log discipline (`docs/plans/v0.1.x-friction-log.md`)
- [x] Repo public on `Roberdan/convergio-local`, release-please
      workflow producing v0.1.x tags
- [x] **ADR-0011**: `done` is set only by Thor (BREAKING)
- [x] **ADR-0012**: OODA-aware validation — the spine of v0.3 work
- [x] **ADR-0010**: retire empty `convergio-worktree` crate
- [x] Code-graph crate `convergio-graph` (ADR-0014, drift v0)
- [x] **ADR-0015**: documentation as derived state
      (workspace-members + test-count auto-regen)

## Closed — v0.2.x → v0.3.31

These all landed; current released line is **v0.3.31**.

- [x] **T2.04** auto-close plan task on PR merge
      (`cvg pr sync <plan>` — `crates/convergio-cli-pr/src/pr_sync.rs`)
- [x] **T1.17** Tier-2 retrieval: YAML frontmatter on every ADR
      + `cvg coherence check` (`crates/convergio-coherence/`)
- [ ] `cvg pr stack` localised EN/IT and validating manifest
      against the real diff (paused branch
      `fix/cvg-pr-stack-i18n-and-manifest-validation`)
- [ ] **T3.08** trim 12 Rust files in 250–300 LOC range to leave
      headroom under the 300-line cap

## Shipped in v0.3.x beyond the original plan

Work that was not on the original 4-wave plan but landed under
v0.3 because the dogfood loop demanded it. ADR coverage in
parentheses.

- [x] **Fleet abstraction** — one daemon orchestrating multiple
      repos: cross-repo plans + fan-out, Thor fleet validation,
      derived fleet audit-chain walk, semantic dead-code
      (`cvg fleet rot`), semantic doc-vs-code drift
      (`cvg fleet doc-drift`), MCP fleet actions (ADR-0038,
      ADR-0041, ADR-0049 retrospective)
- [x] **`convergio-embed`** — embeddings store + pluggable
      embedder trait (ONNX/fastembed feature) for Tier-3 retrieval
      (ADR-0038, F1)
- [x] **`convergio-parse-multi`** — Tree-sitter adapters for
      TypeScript + Python feeding the cross-repo graph (ADR-0038,
      F2)
- [x] **`convergio-coherence`** — `cvg coherence check / routes /
      adrs / agents / fleet / handshake / plan-execution` suite
      (ADR-0040)
- [x] **`convergio-tui`** — `cvg dash` four-pane terminal
      dashboard (Plans, Tasks, Agents, PRs) (ADR-0029)
- [x] **`convergio-server-core` + `convergio-cli-*` splits** — PR,
      session, plan-run extracted from monolith CLI to stay under
      the 300-LOC cap (ADR-0041 et al.)
- [x] **Vendor-CLI runners**: `claude`, `copilot` built-in;
      `qwen`, `codex`, `gemini` via `~/.convergio/runners.toml`
      with no recompile (ADR-0028, ADR-0032, ADR-0035)
- [x] **Permission profiles** `standard` / `read_only` / `sandbox`
      (ADR-0033) + per-task `runner_kind` / `profile` /
      `max_budget_usd` (ADR-0034)
- [x] **Compensating actions** — `GET /v1/audit/events/:seq/compensate`,
      `cvg audit compensate` (ADR-0048)
- [x] **Generated action surface** — `GET /v1/api/actions` +
      `cvg actions list` byte-identical with MCP discovery
      (ADR-0047)
- [x] **Gate preconditions introspection** — `GET /v1/gates/preconditions`
      + `cvg gates show` (P3-2)

---

## Wave 0 — Urban code articulated + first multi-agent client

> **Pitch**: name what we already built, then prove it works with
> two coordinated Claude Code sessions in one repo.

Wave 0 ships in two PRs. Wave 0a is pure documentation
(constitutional, can land immediately). Wave 0b is the first
multi-agent client implementation (PRD-001), which depends on a
small bus schema migration and lands separately.

### Wave 0a — narrative + constitutional update (in flight, Q2 2026)

A pure documentation PR. No new daemon code. Lands first.

- [x] [`docs/vision.md`](./docs/vision.md) rewritten with long-tail
      thesis, urbanism frame, Modulor, three layers
- [x] [ADR-0016](./docs/adr/0016-long-tail-vertical-accelerators.md)
      — long-tail accelerators
- [x] [ADR-0017](./docs/adr/0017-ise-hve-alignment.md) — ISE +
      hve-core alignment
- [x] [ADR-0018](./docs/adr/0018-urbanism-over-architecture.md) —
      urbanism over architecture (Le Corbusier + Jane Jacobs)
- [x] [ADR-0019](./docs/adr/0019-thinking-stack-gstack-vendored.md)
      — gstack as a Convergio capability
- [x] [ADR-0020](./docs/adr/0020-model-evaluation-framework.md) —
      model evaluation as the municipality's procurement office
- [x] [ADR-0021](./docs/adr/0021-okr-on-plans.md) — Plans are
      Objectives + Key Results
- [x] [ADR-0022](./docs/adr/0022-adversarial-review-service.md) —
      adversarial review as a municipal service
- [x] [PRD-001](./docs/prd/0001-claude-code-adapter.md) — written
      and reviewed (implementation in 0b)
- [x] README.md hero copy reflects long-tail + alignment elevator
      pitch + Microsoft disclaimer
- [x] CONSTITUTION.md § 17 — Modulor structural rule (ADR-0018)

#### Success criteria — Wave 0a

1. A reader of `README.md` + `docs/vision.md` can answer "is
   Convergio a leash or a shovel?" with "both — the leash is how
   the shovel keeps its edge".
2. Every doc shipped under this wave references the ADR or PRD it
   implements (no dangling marketing prose).
3. The doc set passes its own adversarial-review service
   (ADR-0022) before merge.

### Wave 0b — Claude Code adapter (PRD-001 implementation)

A code PR. Estimated 9-13 days (revised after adversarial review
of the original 4-day estimate). Includes a small bus schema
migration to allow `plan_id IS NULL` for the new
`system.session-events` topic.

- [~] Skill `/cvg-attach` — scaffolding present in
      `examples/skills/cvg-attach/` and `cvg-attach-cursor/`, but
      the four Claude Code hooks (SessionStart / PreToolUse /
      PostToolUse / Stop) and registration against
      `POST /v1/agent-registry/agents` are **not yet wired**
- [ ] Bus schema migration for system topic + small ADR
- [ ] `cvg status --agents` flag with EN/IT i18n (string is
      referenced in `setup_readme.rs` docs but the flag is not
      implemented in `status` command)
- [ ] E2E test exercising two ephemeral agent registrations
- [ ] `cvg setup claude-code` installer
- [ ] README section in `examples/skills/cvg-attach/` showing the
      live two-session demo

#### Success criteria — Wave 0b

1. Two Claude Code sessions in the same repo see each other in
   `cvg status --agents` within 30 seconds, with last heartbeat
   and held leases visible.
2. Audit chain verifies clean (`cvg audit verify`) after the
   end-to-end demo of two sessions claiming and releasing leases.
3. Killing one session releases its leases within 90 seconds and
   writes an `agent.retired` audit row.

---

## Wave 1 — Building codes complete + thinking-stack + plan templates

> **Pitch**: close the P3 honesty gap, make gstack a first-class
> Convergio capability, ship parameterised plan templates so a
> new vertical accelerator costs hours not weeks.

**Q2-Q3 2026.** Subsumes the ADR-0012 work formerly tagged v0.3.

### Smart Thor + outcome reliability (from ADR-0012)

- [x] **T3.02** smart Thor — `cvg validate` runs the project's
      actual pipeline (cargo fmt / clippy / test / doc-check /
      ADR coherence) before Pass
- [ ] **T3.03** agent ↔ Thor negotiation via
      `propose_plan_amendment`
- [ ] **T3.04** 3-strike escalation to the human operator
- [ ] **T3.06** wave-scoped validation (`cvg validate <plan>
      --wave N`)
- [ ] **T3.07** wave-aware planner that bundles tasks into
      coherent ship units
- [ ] **T2.05** split `convergio-durability` (ADR-0013) into
      audit+gates / plan-task-evidence / workspace+crdt+capability
- [ ] **T4.01** context packet: Thor receives project rules + past
      refusals + crate AGENTS.md + related ADRs as input
- [ ] **T4.02** learnings store: query view over the audit chain
- [ ] **T4.03** agent reputation aggregated from audit history
- [ ] **T4.05** durable agent sessions for long runs

### Building codes (close the honesty gaps)

- [ ] **A11yGate (P3) — phase 1 (built-in)**: closes the honesty
      gap declared since ADR-0004 with built-in checks that do
      *not* require external tooling — semantic heading
      structure on Markdown, alt-text on images, no
      colour-only emphasis on terminal output, ANSI-strippable
      CLI text. This phase ships in Wave 1 *without* depending
      on Wave 2's `a11y-axe` capability.
- [ ] **A11yGate (P3) — phase 2 (axe-core)**: in Wave 2,
      `a11y-axe` capability extends the gate to UI-touching
      evidence kinds (`html_output`, `screenshot`,
      `component_render`) by running axe-core. Wave 1 phase 1
      covers the constitutional commitment; Wave 2 phase 2
      hardens UI coverage.
- [x] **WireCheckGate (P4)** — **shipped** in v0.3.x. Refuses
      scaffolding via structured `wire_check` evidence row that
      verifies every claimed HTTP route and CLI subcommand
      actually exists in the workspace tree
      (`crates/convergio-durability/src/gates/wire_check_gate.rs`).
      A future `ClaimCheckGate` (F55-B) will force the opt-in
      `wire_check` row to be present.
- [ ] **PromptInjectionGate (P2)** — refuses evidence containing
      known prompt injection patterns; baseline list curated.

### Thinking-stack capability (ADR-0019)

- [ ] `convergio-thinking-bundles/gstack` repo created, signed
      bundles published
- [ ] `cvg capability sync thinking-stack-gstack` command
- [ ] MCP actions `thinking.plan_ceo`, `thinking.plan_eng`,
      `thinking.plan_design`, `thinking.plan_devex`,
      `thinking.autoplan`, `thinking.codex`, `thinking.office_hours`
- [ ] i18n + a11y overlay on skill output

### Plan templates

- [ ] `cvg plan create --template <name> --param k=v …` —
      parameterised templates with explicit gates and
      `evidence_required` per task
- [ ] First template: `vertical-accelerator-v1` (general scaffold
      for any domain accelerator)
- [ ] Template authoring guide in `docs/templates/`

### Strategic programming — OKR on plans (ADR-0021)

- [ ] **Schema**: `plans.objective NOT NULL`, `plan_key_results`
      table, `tasks.contributes_to_kr_id` (per-crate migration in
      `convergio-durability`)
- [ ] `PlanCoherenceGate` — refuses task `submitted` if the plan
      has no objective + at least one KR; warns on NULL
      `contributes_to_kr_id`
- [ ] `PlanOutcomeGate` — smart Thor refuses plan-level `done`
      until every KR is `achieved` or `missed_with_override`
- [ ] CLI: `cvg plan kr add | measure | list`,
      `cvg status --plan --okr`
- [ ] MCP: `set_plan_objective`, `add_key_result`,
      `update_key_result_value`, `list_plan_okrs`
- [ ] Retroactive OKR annotation on the Wave 0 dogfood plan

### Success criteria

1. P3 a11y is enforced: a task touching UI cannot reach `done`
   without a passing axe-core scan.
2. Smart Thor refuses a deliberately broken `cargo test` evidence
   that would have passed shape-only validation in v0.2.0.
3. `cvg capability sync thinking-stack-gstack` upgrades a stale
   bundle end to end, with audit row and signature verification.
4. A new accelerator can be scaffolded with one CLI command from
   the template + parameters, producing a structured plan ready
   for `cvg validate`.

---

## Wave 2 — First capability blocks ship (the Lego)

> **Pitch**: the first five materials in the registry, plus the
> runner adapters that let any LLM consume them.

**Q3 2026 — Q1 2027.** Originally scoped Q3 2026; the
adversarial review of this roadmap (ADR-0022 dogfood pass)
flagged single-quarter delivery as optimistic given that this
wave bundles 3 runner adapters + 5 capability blocks + remote
registry + model evaluation framework + OKR dashboard. The
revised window splits each capability block into its own PR and
allows quarter spillover. The end-of-wave milestone is
*"Wave 3 can start without missing dependencies"*, not a fixed
calendar date.

Each block is its own capability bundle, signed, namespaced,
testable, and lands as its own PR with its own tests.

### Runner adapters

- [ ] **`ai-claude`** — Claude Agent SDK runner adapter
      (registered as agent kind `claude-sdk`, spawn protocol per
      ADR-0009)
- [ ] **`ai-copilot`** — GitHub Copilot runner adapter, consumes
      `microsoft/hve-core` agent definitions as prompt source
      (ADR-0017 integration model)
- [ ] **`ai-openai`** — OpenAI / OpenAI-compatible runner adapter
- [ ] **T4.04** multi-vendor routing — `cvg dispatch` picks an
      adapter per task based on capability requirements + cost +
      latency profile

### First-party capability blocks

- [ ] **`azure.voice`** — Speech-to-Text + Text-to-Speech via
      Azure Cognitive Services, evidence kinds
      `audio_transcript`, `tts_render`
- [ ] **`auth.entra`** — Microsoft Entra ID OIDC flows, scoped
      tokens, evidence kinds `auth_flow_proof`,
      `token_introspection`
- [ ] **`ui.fluent`** — Fluent UI components accessible by
      default, evidence kinds `component_render`,
      `storybook_snapshot`
- [ ] **`a11y.axe`** — axe-core integration; first consumer of
      this block is the A11yGate (Wave 1 dependency)
- [ ] **`payments.stripe`** — Stripe Checkout + webhook
      validation, evidence kinds `webhook_signature_verified`,
      `payment_intent_created`

### Capability registry — distribution surface

- [ ] **Remote capability registry** — ADR-0008 graduates from
      first-party-local to first-party-remote. HTTPS endpoint,
      mirror discipline, key rotation policy.
- [ ] `cvg capability search <query>` — manifest text search
- [ ] `cvg capability install <name>@<version>` — pull from
      remote, verify signature, install locally
- [ ] Bundle reproducibility test in CI

### Model evaluation framework — the municipality's procurement office (ADR-0020)

- [ ] **Schema**: `model_evaluations` view + `task_taxonomy`
      table (closed taxonomy: `generate-test`, `review-code`,
      `write-docs`, `refactor`, `plan`, `summarise`, `generic`,
      extensible via small ADR)
- [ ] **Continuous evaluation pipeline**: every Thor verdict
      attributes Pass/Fail/cost/latency to
      `(model, prompt_template, taxonomy_kind)`
- [ ] **MCP actions**: `eval.record` (internal),
      `eval.recommend`, `eval.report`, `eval.calibrate`
- [ ] **`cvg dispatch` integration** (T4.04): the dispatcher
      calls `eval.recommend` with budget constraints and picks
      the runner adapter; the choice is recorded on the spawned
      `agent_processes` row
- [ ] **Cold-start handling**: bootstrap with adapter's
      self-declared profile, over-weight first 50 outcomes,
      `cold_start` flag in recommendations
- [ ] **`cvg eval calibrate`** — opt-in periodic suite that
      runs a fixed test set against every registered adapter
- [ ] **OKR integration**: Cost-of-Pass per accelerator becomes
      a defaultable KR for vertical accelerator templates

### OKR dashboard + drift detection

- [ ] `cvg status --plan <id> --okr` rich output (progress bars,
      KR trend, time-to-target estimate)
- [ ] **`kr.drift` audit row**: code-graph (ADR-0014) detects
      tasks claiming to advance KR X whose evidence does not
      move KR X's `current_value`. Advisory v1, gating v2.

### Success criteria

1. A vertical accelerator author composes `azure.voice` +
   `auth.entra` + `ui.fluent` + `a11y.axe` + `payments.stripe`
   into a working plan with no glue code beyond capability
   composition.
2. `cvg dispatch` runs the same plan with `ai-claude` or
   `ai-copilot` interchangeably; evidence is identical in shape.
3. `cvg capability install azure.voice@1.0` against the remote
   registry installs and verifies in under 10 seconds on a
   typical home connection.
4. Every block ships with E2E test coverage of its declared
   evidence kinds.

---

## Wave 3 — First vertical accelerator: `convergio-edu` reborn

> **Pitch**: the demo that proves the urban code works. End to
> end, in public.

**Wave 3 starts when Wave 2 closes.** Originally scoped Q3-Q4
2026; revised to a *condition* (Wave 2 dependency-complete)
rather than a calendar date, since Wave 3 requires every Wave 2
capability block plus runner adapters plus the OKR/eval
infrastructure from Wave 1. The realistic earliest start is
late Q4 2026; the realistic latest start without re-scoping is
Q2 2027. This is the pitch demo, not the product. Its job is
to prove that the long-tail thesis holds in practice.

### Deliverables

- [ ] Plan template `education-accelerator-v1` with parameters:
      `domain`, `target-age`, `primary-language`,
      `secondary-language`, `accessibility-profile` (default
      `dyslexia-friendly`)
- [ ] Capability composition: `azure.voice` + `auth.entra` +
      `ui.fluent` + `a11y.axe` + `payments.stripe` +
      `thinking.gstack`
- [ ] Domain-strengthened gates:
      - **A11yGate** runs at `serious`+ severity, must pass for
        every UI-touching task
      - **GDPRStudentGate** — refuses evidence that exposes minor
        student PII outside encrypted boundaries
      - **MultiLanguageGate** — refuses UI strings outside the
        Fluent bundles; primary + secondary languages mandatory
- [ ] Capability bundle `convergio-edu-v1.cap`, Ed25519 signed
- [ ] End-to-end demo: `cvg solve "build accessible educational
      app for dyslexic kids in EN+IT"` → plan generated → tasks
      executed in parallel → evidence collected → gates pass →
      validated by Thor → audit chain verifies → installable
      `.cap` on disk
- [ ] Public dogfood post + recorded demo

### Success criteria

1. The demo runs from a clean machine (single `cvg setup` call
   followed by the `cvg solve` line above) in under 30 minutes.
2. Every task in the generated plan has machine-readable evidence
   that passes all P1-P5 gates plus the three domain gates.
3. The audit chain over the demo run is non-trivial (50+ rows)
   and verifies clean end to end.
4. A second vertical accelerator (any domain — research / health
   / civic) can be authored using the same pattern within one
   week of the demo, by the same operator, without changes to
   Convergio core.

---

## v0.4+ — Multi-vendor swarm + AI-maintained knowledge

Post-Wave-3 work. The OODA loop becomes a swarm; the wiki grows
under the agents' hands.

- [ ] **T4.06** fresh-eyes legibility simulation (zero-shot agent
      comprehension test)
- [ ] **T4.07** local RAG over the corpus (SQLite FTS5 + optional
      embeddings)
- [ ] **T4.08** **LLM Wiki**: `docs/learnings/<topic>.md`
      AI-maintained knowledge base, fed by Thor refusals and
      compacted by a periodic agent
- [ ] **T3.09** Tolaria-style frontmatter compatibility +
      `cvg setup vault` to bridge a Tolaria vault into the
      `convergio.help` surface
- [ ] Capability blocks for adjacent zones: `data.*`, `obs.*`
- [ ] Second and third vertical accelerators (research,
      civic-services), authored *outside* this repo to prove the
      urban-code claim
- [ ] **Multi-language graph adapters (deferred, Rust-first
      until further notice)** — today the code-graph engine
      (ADR-0014) parses Rust only via `syn`. The daemon, plans,
      tasks, and audit chain are already project-scoped and work
      against any external repo path (`cvg graph build
      --manifest-dir <X>`). The missing piece is a
      `LanguageAdapter` trait with implementations for Python /
      TypeScript / Go / Swift, so a vertical solution authored
      outside this repo in a non-Rust stack can use Convergio as
      its leash. ADR-0016 candidate when this becomes blocking;
      until then, every vertical accelerator stays Rust to keep
      the dogfood loop tight.

---

## Explicitly out of scope

- hosted service
- remote multi-user deployment
- account / organisation model
- RBAC
- distributed mesh
- graphical UI
- billing
- agent marketplace (the capability registry is *not* a
  marketplace; it is a materials registry — see ADR-0018)

## Success criteria — overall

- A solo developer can install the daemon and CLI, run the
  quickstart, and see a gate refusal plus audit verification in
  minutes (Wave 0).
- A multi-agent team can work in parallel without directly
  mutating the canonical workspace or silently overwriting each
  other's state (Wave 0 + Wave 1).
- The legibility score (CONSTITUTION § 16) stays above 70/100;
  any drop is investigated as a regression.
- Every refusal Thor produces is non-falsifiable and fixable —
  the agent receives a structured pointer, not a blank "no"
  (Wave 1, T4.02 learnings store).
- The first vertical accelerator (`convergio-edu` reborn) ships
  end-to-end and is demonstrably reproducible by a second
  operator in under one week (Wave 3).
- The four marginal costs (creation, coordination, distribution,
  discovery — `docs/vision.md` § 7) are each closed by the wave
  that names them.
