# PRD — Convergio Fleet: Cross-Repo Brain & Multi-Repo Orchestration

- **Status**: Draft v1.0
- **Date**: 2026-05-03
- **Author**: Roberto D'Angelo (with Claude)
- **Companion ADR**: ADR-0038 (`docs/adr/0038-fleet-retrieval-cross-repo-graph.md`)
- **Target release**: Convergio v3.x (F1) → v3.y (F2) → v4.0 (F3)
- **Audience**: Convergio core team, downstream-machine maintainers, Roberto (product owner)

---

## 1. Why this PRD exists

Convergio v3 is a leash for ONE repo's AI agent. Roberto operates a
**fleet** of 7+ repositories (Convergio engine, convergio-edu,
ui-framework (external), MirrorBuddy, MirrorHR_Set, VirtualBPM,
hve-core, WareHouse — heading toward 10-20). The current product
shape leaves him answering critical maintenance questions **manually
across N repos**:

- Can I remove this code/file/crate/ADR?
- Is documentation aligned with implementation?
- Is the context I'm passing to my agent correct and sufficient?
- What do my repos have in common — and could be hoisted into a
  shared library?
- How do I make a coordinated change across 4 repos without dropping
  the audit trail?

This PRD specifies a product evolution that turns Convergio from
"single-repo leash" into "**fleet brain + leash**" — a control plane
that ingests N heterogeneous repos (Rust, TypeScript, Python, Go),
reasons across them, and orchestrates plans/audits at fleet scope.

The mechanism is a combination of **multi-language code parsing**,
**semantic embeddings** layered on the existing structural graph, and
**fleet-scoped plan/audit primitives**.

---

## 2. Problem Statement

### 2.1. Who has this problem

| Persona | Description | How they hit the problem |
|---|---|---|
| **Roberto (P0)** | Solo founder operating a fleet of AI-built products with Convergio as the engine | Daily maintenance, deprecations, doc updates, refactor decisions. Can't see the fleet as one system today. |
| **Convergio agent (P1)** | An LLM agent doing work in a Convergio-managed task | Receives an incomplete or noisy context-pack, hallucinates, fails the gate. Loop is expensive. |
| **Downstream maintainers (P2)** | Teams maintaining a single Convergio-derived machine (e.g. convergio-edu) | Can't know when the upstream Convergio engine breaks their assumptions until production. |
| **Auditor / reviewer (P3)** | Human verifying a coordinated change across multiple repos | Reconstructs the change manually from N PRs, N audit chains, N evidence trails. |

### 2.2. The six concrete pains

#### P-1: "Can I remove this?"

Roberto inherits a directory or sees an old crate. He wants to delete
it. Today he has to:
- `git log` to see who last touched it
- grep across all fleet repos for references
- manually check ADRs that might still claim it
- guess whether dynamic dispatch / reflection / HTTP routes use it

**Frequency**: weekly. **Time cost**: 30-90 min per investigation.
**Failure mode**: removes something that was actually in use →
production incident.

#### P-2: "Is the doc still true?"

ADR-0014 says retrieval is substring-based. Tomorrow we ship semantic.
The ADR body still says substring. Nobody updates it. The next agent
that reads it makes wrong decisions.

README of convergio-edu says "7 domain agents". Code has 9. Nobody
updates the README.

**Frequency**: every release. **Time cost**: shipped-with-stale-docs
99% of the time. **Failure mode**: agents and humans make decisions
on outdated documentation.

#### P-3: "Is my agent's context complete?"

Roberto delegates a task to a sub-agent (Claude / Codex / Copilot)
with `cvg graph for-task <id>`. The agent does the work, but Roberto
has no visibility into:
- Was the context-pack the right size? (too small → hallucinations,
  too big → cost blowout)
- Did the agent end up reading files outside the pack? (pack
  incomplete)
- Were there relevant files in other fleet repos that should have
  been in the pack? (cross-repo blindness)

**Frequency**: every delegation, ~10x/day. **Time cost per failure**:
30-120 min remediation. **Failure mode**: the agent solves the wrong
problem with the wrong context.

#### P-4: "What do my repos have in common?"

Convergio (Rust) has a `Plan` with phases. convergio-edu (Python)
has a `Curriculum` with stages. MirrorHR has `OnboardingFlow` with
steps. They're the same shape doing the same thing in three
languages. **No tool tells him this** because AST parsers don't
cross language boundaries.

**Frequency**: continuous. **Time cost**: nobody knows because
nobody can see it. **Hidden cost**: 3 implementations to maintain,
3 places to fix bugs, divergent behaviour.

#### P-5: "How do I optimise the whole fleet?"

Once duplications are visible (P-4), the next question is: extract a
shared library? Standardise the schema? Pick the best implementation
and propagate? Today the answer is "manually inventory N repos and
hope I don't miss one."

**Frequency**: ~quarterly when Roberto has time. **Time cost**: a
full day per audit, brittle.

#### P-6: "Coordinated cross-repo change"

"Roll out i18n across all UI-bearing machines." Today this is N
parallel plans, N audit chains, N evidence reviews, N rollback
windows. No fleet-level "did we ship this everywhere?"

**Frequency**: ~monthly. **Time cost**: 2-5 days of coordination.
**Failure mode**: shipped on 4 of 5 machines, the 5th lags by
weeks because nobody noticed.

---

## 3. Goals & Non-Goals

### 3.1. Goals (what success looks like)

- **G1**: Roberto can ask "what can I remove from convergio-edu?"
  and get a ranked list with confidence scores within 30 seconds.
  **Metric**: at least 70% of suggested removals are accepted on
  manual review.
- **G2**: Roberto can ask "are the docs still aligned?" and get a
  list of doc/code drift candidates with semantic explanation.
  **Metric**: false positive rate < 25%.
- **G3**: When Roberto delegates a task, the agent's context-pack
  has a documented recall measurement. **Metric**: average recall@10
  ≥ 0.85 on the golden set.
- **G4**: Roberto can ask "what patterns exist across the fleet?"
  and get cross-repo cluster suggestions. **Metric**: discovers ≥ 3
  real cross-language patterns in the F2 prototype fleet.
- **G5**: Roberto can create one fleet-scoped plan that spans 3+
  repos, each with its own validators, with one rollup audit verdict.
  **Metric**: end-to-end fleet plan executes successfully on F3.
- **G6**: All of the above is **opt-in**, **deterministic in tests**,
  and respects the Convergio Constitution (zero tech debt, local-
  first, no scripts, i18n IT/EN).

### 3.2. Non-goals (explicit)

- **NG1**: We are NOT building a SaaS. Convergio remains local-first,
  one machine, one daemon (or one per developer machine).
- **NG2**: We are NOT replacing existing single-repo `cvg graph`
  commands. They remain unchanged. Fleet is additive.
- **NG3**: We are NOT supporting fleets > 100 repos in this iteration.
  SQLite scales to that point but the UX of `cvg fleet ls` doesn't.
  Future ADR if needed.
- **NG4**: We are NOT building a hosted embedding service.
  Embeddings run locally via fastembed-rs + ONNX models on disk.
- **NG5**: We are NOT solving non-code artefacts (Figma, Notion,
  Linear, Slack). Out of scope for v3.x.
- **NG6**: We are NOT replacing gbrain (gstack). Different tool,
  different audience. Convergio Fleet is **code-aware**; gbrain is
  **session-aware**. They can coexist on the same machine.
- **NG7**: We are NOT making retrieval mandatory. Substring-only
  remains the default for backward compat.

---

## 4. User Stories

### Epic 1 — Maintenance hygiene at fleet scale

**US-1.1** As Roberto, when I run `cvg fleet rot`, I see a ranked list
of code/files/crates/ADRs that are likely safe to remove, with
confidence scores and evidence (no incoming edges + low semantic
similarity to active plans + no API surface exposure).

**Acceptance criteria**:
- Output is a sortable list, default sort by confidence desc
- Each row shows: repo, kind, name, confidence (0-1), reason summary
- A `--explain <id>` flag shows the full evidence (incoming edges, 5
  nearest semantic matches, API exposure check)
- Output has a `--format json` mode for scripting

**US-1.2** As Roberto, when I run `cvg fleet doc-drift`, I see ADRs
and READMEs whose semantic content has drifted from the code they
claim, with a natural-language summary of the drift.

**Acceptance criteria**:
- Each row: doc path, claimed crate(s), drift score (0-1), one-line
  summary of what changed
- `--since <git-rev>` to scope drift to recent changes
- LLM-summarisation is opt-in (one inference call per drift row);
  default produces structural diff only

### Epic 2 — Better agent context

**US-2.1** As Roberto, when I run `cvg fleet for-task <id>` with
`--gap-check`, the returned context-pack is enriched with
semantically-relevant files that substring matching missed, each
labelled with `match_source: semantic`.

**Acceptance criteria**:
- Default: behaves like `cvg graph for-task` (single-repo, structural)
- `--repo <name>` scopes to one repo of the fleet
- `--gap-check` adds semantic expansion, max 5 additional files
- Every file in the pack has a `match_source: structural | semantic |
  both` field
- Total token estimate respects existing `--token-budget`

**US-2.2** As Roberto, when an agent finishes a task, the daemon logs
which files in the pack were actually opened by the agent and which
were not. Over time, this becomes a recall measurement that surfaces
in `cvg fleet stats`.

**Acceptance criteria**:
- Evidence rows include `read_files: [...]` for completed tasks
- `cvg fleet stats --recall` shows recall@10, recall@25, precision@10
  trends per agent runner
- A regression in retrieval quality surfaces in the dashboard

### Epic 3 — Cross-repo intelligence

**US-3.1** As Roberto, when I run `cvg fleet patterns`, I see
clusters of semantically-similar code/concepts that span 3+ repos,
each cluster summarised with a candidate name and a hoist
suggestion.

**Acceptance criteria**:
- Output groups by cluster; each cluster shows member nodes (repo +
  name + kind) and a confidence score
- Filter `--min-repos 3` enforces cross-repo presence
- LLM-suggested cluster names are opt-in; default uses centroid
  keyword extraction
- Each cluster has a "candidate hoist target" (e.g. "extract to
  fsm-core (external)")

**US-3.2** As Roberto, when I run `cvg fleet duplicates`, I see
near-exact code/concept duplicates across repos with cosine ≥ 0.95
and matching structural shape.

**Acceptance criteria**:
- Output: pairs (repo_a/node, repo_b/node, cosine, shape_match)
- `--cosine 0.9` to lower threshold; `--repo-pair convergio,convergio-
  edu` to scope to two repos
- Each pair has a "diff preview" (semantic delta in 1-3 lines)

### Epic 4 — Fleet plans & audit

**US-4.1** As Roberto, I can create one plan that spans 3+ repos,
each repo getting an auto-generated per-repo plan with the right
gate set, all linked under one fleet plan ID.

**Acceptance criteria**:
- `cvg fleet plan create "<title>" --repos convergio,convergio-edu,
  ui-framework (external)`
- Each repo's plan inherits fleet plan ID + has its own evidence
  rows + gate run
- `cvg fleet plan show <id>` shows fleet rollup: per-repo status,
  evidence counts, gate verdicts

**US-4.2** As Roberto, I can validate a fleet plan with one command
and get a per-repo rollup of gate results, with green status only if
all touched repos pass.

**Acceptance criteria**:
- `cvg fleet validate <fleet-plan-id>` runs the gate pipeline in
  each touched repo's daemon (or in-process if single-daemon mode)
- Returns 200 + green only if all repos pass; otherwise 409 with
  per-repo verdicts
- Audit chain integrity: each repo's chain stays canonical; fleet
  view is derived

---

## 5. Functional Requirements

### 5.1. Fleet management

| FR | Requirement |
|---|---|
| **FR-1.1** | User can register a repo via `cvg fleet add <path>` with auto-detected language and parser |
| **FR-1.2** | User can list registered repos via `cvg fleet ls` with last-build time, node count, embedding coverage |
| **FR-1.3** | User can build/refresh a repo's graph via `cvg fleet build [--repo <name>]`; default: all enabled repos |
| **FR-1.4** | User can disable a repo without removing it via `cvg fleet disable <name>` |
| **FR-1.5** | Repo path can be relative or absolute; daemon resolves at build time |
| **FR-1.6** | If a repo path doesn't exist on build, daemon emits a warning but continues with other repos |
| **FR-1.7** | Repo names must be globally unique across the fleet config |
| **FR-1.8** | `convergio.yaml` in a repo can declare `derives_from: <repo-name>` to auto-add edges |

### 5.2. Multi-language parsing

| FR | Requirement |
|---|---|
| **FR-2.1** | Rust parsing uses existing `syn` walker (unchanged) |
| **FR-2.2** | TypeScript parsing uses `tree-sitter-typescript` and produces same Node/Edge model |
| **FR-2.3** | Python parsing uses `tree-sitter-python` and produces same Node/Edge model |
| **FR-2.4** | Markdown/MDX parsing uses existing `doc_link.rs` extended with frontmatter parsing |
| **FR-2.5** | Per-language `item_kind` taxonomy is documented and stable: rust=`struct\|fn\|...`, ts=`function\|class\|interface\|...`, python=`function\|class\|method\|...` |
| **FR-2.6** | Parser failures emit a warning per file and skip; never crash the build |
| **FR-2.7** | Each language parser is in its own module under `convergio-parse-multi` and respects 300-line cap |

### 5.3. Embeddings

| FR | Requirement |
|---|---|
| **FR-3.1** | Embedding is opt-in via `convergio-embed` workspace feature flag |
| **FR-3.2** | Default model: BGE-M3-small-int8 (multilingual, 384-dim) |
| **FR-3.3** | Model is downloaded on first use to `~/.convergio/v3/models/` and cached |
| **FR-3.4** | Embeddings are stored in `graph_node_embeddings` keyed by `(repo, node_id, model)` |
| **FR-3.5** | sqlite-vec extension is loaded by `convergio-db::Pool` only when feature `embed` is on |
| **FR-3.6** | Selective embedding: crate, module, documented item, ADR, doc — not undocumented private items |
| **FR-3.7** | Re-embed trigger: source_hash change (not just mtime) |
| **FR-3.8** | Embedding storage budget: ≤ 500MB per fleet by default; warn at 80% |
| **FR-3.9** | Failed embeddings (model unavailable, OOM) downgrade gracefully to structural-only |

### 5.4. Retrieval

| FR | Requirement |
|---|---|
| **FR-4.1** | `cvg fleet for-task <id>` accepts `--alpha`, `--top-k`, `--token-budget`, `--gap-check`, `--repo`, `--format` |
| **FR-4.2** | Default ranking: RRF fusion of structural + semantic |
| **FR-4.3** | `--alpha N` switches to linear fusion with weight N |
| **FR-4.4** | Every returned node has `match_source` and `score_components` (structural, semantic) |
| **FR-4.5** | p95 latency < 1s on 200K-node fleet (warm cache) |
| **FR-4.6** | If embedding feature disabled, behaves identically to existing `cvg graph for-task` |

### 5.5. Maintenance commands

| FR | Requirement |
|---|---|
| **FR-5.1** | `cvg fleet rot` outputs ranked dead-code candidates |
| **FR-5.2** | `cvg fleet doc-drift` outputs ADRs/READMEs with stale semantic content |
| **FR-5.3** | `cvg fleet patterns` outputs cross-repo clusters |
| **FR-5.4** | `cvg fleet duplicates` outputs cross-repo near-duplicates |
| **FR-5.5** | All maintenance commands support `--format json` |
| **FR-5.6** | All maintenance commands support `--explain <id>` for evidence trail |

### 5.6. Fleet plans

| FR | Requirement |
|---|---|
| **FR-6.1** | `cvg fleet plan create` creates a fleet plan + per-repo plans linked by fleet_plan_id |
| **FR-6.2** | `cvg fleet plan ls` lists fleet plans with rollup status |
| **FR-6.3** | `cvg fleet plan show <id>` displays fleet rollup + per-repo details |
| **FR-6.4** | `cvg fleet validate <id>` runs gates across all touched repos |
| **FR-6.5** | Audit chain per repo remains canonical; fleet view is derived |
| **FR-6.6** | `cvg audit verify --fleet <id>` walks all touched chains and verifies fleet integrity |

---

## 6. Non-Functional Requirements

| NFR | Requirement | Threshold |
|---|---|---|
| **NFR-1** | Cold build of full fleet (3 repos, ~30K nodes) | ≤ 5 min |
| **NFR-2** | Incremental build (5 files changed) | ≤ 30 s |
| **NFR-3** | Query p95 (warm) | ≤ 1 s |
| **NFR-4** | Query p95 with `--gap-check` | ≤ 2 s |
| **NFR-5** | Daemon memory overhead (model loaded + idle) | ≤ +250 MB |
| **NFR-6** | Disk overhead (embeddings, 3 repos) | ≤ 100 MB |
| **NFR-7** | Test recall@10 floor | ≥ 0.85 |
| **NFR-8** | Test duplicate-detection precision | ≥ 0.80 |
| **NFR-9** | Existing test suite | Stay 100% green (524 baseline) |
| **NFR-10** | New file count must respect 300-line/file cap | 100% |
| **NFR-11** | All user-facing strings localised (IT + EN) per CONSTITUTION P5 | 100% |
| **NFR-12** | All probabilistic features behind feature flags | 100% |
| **NFR-13** | All changes preserve backward compat for `cvg graph *` commands | 100% |
| **NFR-14** | Network calls during retrieval: zero (local-first) | 0 |

---

## 7. UX

### 7.1. CLI command map

```
cvg fleet
├── add <path>                    register repo
├── ls                            list with status
├── disable <name> | enable <name>
├── build [--repo <name>] [--refresh-similarity]
├── stats [--recall]
├── for-task <id>                 cross-repo context-pack
│   [--repo <r>] [--gap-check] [--alpha 0.5] [--top-k 25]
│   [--token-budget 12000] [--format json|text]
├── rot [--threshold 0.3] [--repo <r>] [--explain <id>]
├── doc-drift [--since <rev>] [--explain <id>]
├── patterns [--min-repos 3] [--cosine 0.85]
├── duplicates [--cosine 0.95] [--repo-pair a,b]
├── plan
│   ├── create "<title>" --repos a,b,c [--from-template <name>]
│   ├── ls
│   ├── show <id>
│   └── add-task <plan-id> --repo <r> --task <task-id>
├── validate <plan-id>
└── audit verify --fleet <id>     fleet-level audit walk
```

### 7.2. Output samples

**`cvg fleet ls`**:

```
NAME                   PATH                                           LANG     ROLE        NODES    EMBED    LAST BUILD
convergio              ~/GitHub/convergio                             rust     engine      4823     ✓        2 min ago
convergio-edu          ~/GitHub/convergio-edu                         ts+py    downstream  6914     ✓        2 min ago
ui-framework (external) ~/GitHub/ui-framework (external)                ts       library     2102     ✓        2 min ago
MirrorHR_Set           ~/GitHub/MirrorHR_Set                          ts       downstream  3441     ✗        never
```

**`cvg fleet rot`**:

```
RANK  CONF   REPO            KIND     NAME                             REASON
1     0.94   convergio       fn       convergio_thor::legacy_validate   no incoming edges, cosine 0.12 to active ADRs, no API exposure
2     0.91   convergio-edu   class    LegacyAuthAdapter                 no incoming edges, cosine 0.18, deprecated in plan-2025-Q4
3     0.87   convergio       module   convergio-thor::v0                empty after recent refactor, cosine 0.15
4     0.81   convergio       adr      0007-old-provisioning             not referenced in 90d, cosine 0.22 to current
...
```

**`cvg fleet patterns`**:

```
CLUSTER  CONF   REPOS-COUNT   PATTERN-NAME                  HOIST CANDIDATE
P-01     0.93   4             "Plan/Workflow with phases"   extract → fsm-core (external)
P-02     0.88   3             "Auth via scrypt + JWT"       align on convergio-auth
P-03     0.85   3             "i18n bundle loader"          extract → convergio-i18n
```

### 7.3. Error UX

- All command failures emit a structured error with `code`, `message`,
  `remediation`. Example:
  ```
  ERROR FLT-013: tree-sitter-go grammar version mismatch
    expected: 0.21.0
    found:    0.20.5
  REMEDIATION: cargo update -p tree-sitter-go
  ```
- Embedding model download failure → graceful fallback to structural-
  only with a one-line warning per query
- Repo path missing → continue with other repos, summary at end

### 7.4. Localisation

All user-facing strings flow through `convergio-i18n` Fluent bundles.
IT and EN parity day one (per CONSTITUTION P5).

---

## 8. Privacy & Security

- **Local-first**: zero outbound network during retrieval. Embedding
  model is downloaded once from the project's release artefacts (or
  vendored). No remote inference.
- **No secrets ingestion**: parser skips files matching standard
  secret patterns (`.env*`, `*.pem`, `id_rsa*`, etc.). Configurable
  exclude list per repo.
- **Audit trail integrity**: per-repo audit chain is unchanged;
  fleet-level views are derived. ADR-0001 invariant holds.
- **Data locality**: embedding storage is co-located with repo state
  (`~/.convergio/v3/state.db`). No exfiltration, no telemetry by
  default.
- **License hygiene**: BGE-M3 model is Apache 2.0; fastembed-rs is
  Apache 2.0; sqlite-vec is MIT; tree-sitter grammars are MIT/
  Apache. All compatible with Convergio Community License v1.3.

---

## 9. Rollout & Phases

### F1 — Single-repo embedding prototype (2-3 weeks)

**What ships**:
- `convergio-embed` crate
- `cvg graph for-task --semantic` and `--gap-check` flags
- Golden set + recall benchmark
- Migration `0700_embeddings.sql`

**Who tries it**: Roberto on Convergio repo only

**Go/no-go gate**:
- recall@10 improves ≥ 15% absolute
- p95 latency < 1s
- storage < 50MB
- incremental rebuild < 30s

### F2 — Multi-repo opening (4-6 weeks)

**What ships**:
- `convergio-parse-multi` (TS + Python)
- `convergio-fleet` (config, CLI, multi-repo build)
- Backfill of `convergio` + `convergio-edu` + `ui-framework (external)`
- `cvg fleet patterns`, `cvg fleet duplicates`
- Migration `0800_fleet.sql`

**Who tries it**: Roberto on 3-repo fleet

**Go/no-go gate**:
- ≥ 3 real cross-repo patterns surfaced
- duplicate FP rate < 20%
- 60+ new tests added, all 524 existing tests still green

### F3 — Fleet-grade orchestration (6-10 weeks)

**What ships**:
- `cvg fleet plan create / show / ls / validate`
- `cvg audit verify --fleet`
- `cvg fleet rot`, `cvg fleet doc-drift`
- MCP fleet actions

**Who tries it**: Roberto, end-to-end real cross-repo task

**Go/no-go gate**:
- Real fleet plan executes end-to-end
- Fleet-level audit verifies green
- Daily incremental rebuild < 5 min for 5 repos

### Total timeline (honest)

- **Part-time** (Roberto + 1 agent): 4-6 months for F1+F2+F3
- **Full-time equivalent**: 8-12 weeks for F1+F2+F3
- **F1 alone produces value**: 2-3 weeks → measurable recall data

---

## 10. Success Metrics (post-launch)

### F1 metrics

| Metric | Target | Measured by |
|---|---|---|
| recall@10 (Convergio repo) | ≥ 0.85 | golden set in CI |
| Query p95 | < 1s | bench harness |
| Storage overhead | < 50MB | sqlite size delta |
| Adoption (Roberto's daily flow) | `--semantic` used in ≥ 50% queries | telemetry opt-in |

### F2 metrics

| Metric | Target | Measured by |
|---|---|---|
| Cross-repo patterns discovered | ≥ 3 | manual review by Roberto |
| Duplicate-detection precision | ≥ 0.80 | manual sample of 50 |
| Fleet build time | ≤ 5 min cold | bench |
| Test coverage delta | +60 tests | CI |

### F3 metrics

| Metric | Target | Measured by |
|---|---|---|
| Real fleet plan executed | ≥ 1 | smoke test |
| Fleet audit verification | passes green | smoke test |
| Daily-rebuild time | < 5 min | bench |
| Roberto's NPS on fleet workflow | qualitative interview | self-assessment |

### Long-term (6 months post-F3)

| Metric | Target |
|---|---|
| Code Roberto removed via `cvg fleet rot` | ≥ 5% of total LOC |
| Doc drift items closed | ≥ 80% within a sprint |
| Cross-repo plans / month | ≥ 2 |
| Hoist refactors derived from `cvg fleet patterns` | ≥ 1 / quarter |

---

## 11. Risks & Open Questions

(Mirror of ADR-0038 § 9 + § 12 — see ADR for full table.)

**Top risks**:
- R1: Embedding model produces poor similarities for code → mitigated
  by golden-set gate
- R7: Audit chain federation introduces non-determinism → mitigated
  by keeping per-repo chain canonical, fleet view derived
- R10: Fleet outgrows local SQLite at >100 repos → out of scope, future
  ADR

**Open questions** that need answer before F2:
- Single daemon vs federated daemons?
- Repo identity: slug from remote URL or path basename?
- Fleet-level audit: derived view or own merkle?

---

## 12. Open Design Decisions for Roberto

These are explicit choices Roberto needs to make. Each has a default
recommendation but the call is product-level.

| # | Decision | Default | Alternative | When to call |
|---|---|---|---|---|
| **D-1** | Embedding model | BGE-M3-small (multilingual, 384-dim) | jina-v3, mE5-small | F1 start |
| **D-2** | Quantisation | int8 default | float32 | F1 start |
| **D-3** | Single daemon vs federated | Single (simpler) | Per-repo daemon | F2 lock |
| **D-4** | Fleet config location | `~/.convergio/v3/fleet.toml` | per-repo `convergio.yaml` aggregation | F2 lock |
| **D-5** | Cluster naming | Centroid keyword extraction | LLM-generated names (one inference call) | F3 |
| **D-6** | Doc-drift snapshot trigger | On daemon start (lazy) | Git pre-push hook | F3 lock |
| **D-7** | First fleet repos | convergio + convergio-edu + ui-framework (external) | All 7 at once | F2 start |
| **D-8** | Re-embed strategy | source_hash change | mtime + hash | F1 start |
| **D-9** | API versioning | F1+F2 in v3.x; F3 in v4.0 | Hold all for v4.0 | before F1 ships |
| **D-10** | Telemetry on retrieval recall | Opt-in, local-only | Off | F2 |

---

## 13. Out of Scope (this PRD)

- Hosted SaaS Convergio
- Fleet > 100 repos
- Non-code artefacts (Figma, Notion, Linear)
- Replacing gbrain / gstack
- Web UI for fleet dashboard (TUI via `cvg dash --fleet` is a stretch
  goal in F3)
- Cross-fleet federation (multi-organisation)
- Real-time agent coordination across repos (today: plan-level
  orchestration is enough)

---

## 14. Open Questions (for product/founder review)

- **Q-A**: Does "fleet" extend to **non-Convergio-derived** repos
  (e.g. WareHouse, hve-core, navy-dark-vscode)? Or only to repos
  that declare `derives_from: convergio` in `convergio.yaml`?
  *Recommendation*: open to both. Engine-derived flag affects
  default thresholds, not eligibility.
- **Q-B**: Is the fleet-level audit hash a v4 feature, or a v3.y add?
  Materially changes the data model contract. *Recommendation*: v4.
- **Q-C**: Should `cvg fleet rot` integrate with GitHub PR creation
  (auto-PR with deletion diff)? Or is it advisory-only?
  *Recommendation*: advisory-only in F3; auto-PR is a v4.1 add.
- **Q-D**: Distribution of Convergio Fleet to downstream maintainers
  who don't use Convergio for their own repo (yet) — is the fleet
  a "buy-in moment"?
- **Q-E**: How does Convergio Fleet affect the
  pricing / sustainability story for the Convergio project? (License
  is Convergio Community License v1.3; commercial use rules apply.)

---

## 15. Approval

This PRD is **ready for review** when:
- Convergio core team has reviewed all 6 problems in §2.2 and
  confirms the framing matches their experience
- Roberto has answered or deferred all 10 D- decisions in §12
- F1 budget (2-3 weeks) is allocated

Approval sequence:
1. Roberto reviews and approves this PRD + ADR-0038 — *blocks F1*
2. F1 ships → recall data is published
3. Go/no-go meeting on F2 — *if no-go: PRD is closed, embeddings
   feature flag stays off forever*
4. F2 ships → cross-repo patterns demo
5. Go/no-go meeting on F3
6. F3 ships → Convergio v4.0 release

---

## 16. Appendix — Glossary

- **Fleet**: a collection of repos managed under one Convergio control
  plane.
- **Engine repo**: the Convergio repo itself, designated `role:
  engine` in fleet config.
- **Downstream repo**: a repo that declares `derives_from: <engine>`
  in `convergio.yaml`, e.g. convergio-edu.
- **Library repo**: a repo intended to be consumed by other fleet
  repos, e.g. ui-framework (external).
- **Sandbox repo**: opt-in fleet member with relaxed dead-code
  thresholds (work in progress, prototypes).
- **Structural retrieval**: existing substring-based + static-score
  matcher in `convergio-graph::query`.
- **Semantic retrieval**: cosine-similarity search over learned
  embeddings.
- **Hybrid retrieval**: fusion of structural and semantic via RRF or
  linear combination.
- **Recall@K**: of the K files returned, how many of the
  ground-truth-relevant files appear?
- **RRF (Reciprocal Rank Fusion)**: rank-aggregation method that
  combines two ranked lists by summing 1/(k+rank); standard k=60.
- **Doc drift (semantic)**: divergence between the meaning of a doc
  body and the meaning of the code it claims, measurable via
  embedding cosine over time.
- **Code rot**: code with low structural reachability AND low
  semantic relevance to active development.

---

*End of PRD-Fleet*
