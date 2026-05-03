---
id: 0038
status: proposed
date: 2026-05-03
topics: [layer-1, retrieval, graph, context, fleet, multi-repo, semantic, embeddings]
related_adrs: [0003, 0004, 0014, 0015, 0029, 0030, 0034]
touches_crates: [convergio-graph, convergio-db, convergio-server, convergio-cli, convergio-durability, convergio-api]
introduces_crates: [convergio-fleet, convergio-parse-multi, convergio-embed]
last_validated: 2026-05-03
implemented_in: []
authors: [Roberto D'Angelo]
---

# 0038. Fleet retrieval & cross-repo graph (semantic + multi-language)

- Status: **proposed** (RFC, awaiting validation via F1 prototype)
- Date: 2026-05-03
- Deciders: Roberto D'Angelo, Convergio core
- Tags: layer-1, retrieval, graph, fleet, multi-repo, semantic
- Supersedes: extends ADR-0014 (Tier-3 single-repo retrieval)
- Companion: see PRD-Fleet (`PRD-fleet-retrieval-cross-repo-graph.md`) for product framing, user stories, success metrics

---

## TL;DR

Convergio v3 today is a **single-repo leash**: one daemon, one workspace,
one SQLite database, a Tier-3 syntactic graph (ADR-0014) that helps the
agent retrieve context for a task scoped to that one repo.

This ADR proposes evolving Convergio into a **fleet brain**: one logical
control plane that ingests N repos (heterogeneous languages: Rust,
TypeScript, Python, Go), maintains a code+docs graph that crosses repo
boundaries, and exposes retrieval, drift detection, deduplication
analysis, and cross-repo plan orchestration as first-class verbs.

The mechanism that makes cross-language and cross-repo work is a
**semantic embedding layer** added behind a feature flag, on top of the
existing structural graph. Substring matching stays as the precision
oracle; embeddings provide the recall oracle; hybrid ranking fuses them.

This is **Convergio v4 in shape**, not a single feature. We propose
landing it in three phases (F1 → F2 → F3) with explicit go/no-go gates
between phases. F1 is a 2-3 week prototype that produces measurable
recall data; only if the data is favourable do we commit to F2/F3.

---

## 1. Context and Problem Statement

ADR-0014 introduced Tier-3 retrieval for one repo. The graph is
syntactic-only (`syn` walker), persisted in SQLite, queryable via
`cvg graph for-task <id>` to produce a context-pack scoped to a task.
Match strategy is intentionally simple — substring + static score
(crate=10, module=3, item=1) — with the explicit note in
`crates/convergio-graph/src/query.rs` that "anything more sophisticated
(TF-IDF, embeddings, type-resolved call sites) is future work".

Six maintenance problems remain unsolved by the current tier-3 design,
and they are **the actual pain Roberto described** while operating
Convergio + its downstream "machines" (convergio-edu,
convergio-ui-framework, MirrorBuddy, MirrorHR, VirtualBPM, hve-core,
WareHouse — at least 7 repos today, planned 10-20):

### 1.1. Removability uncertainty

> *"Can I remove this crate, this file, this ADR, this feature?"*

Today: dead-code detection = nodes with no incoming `uses` edge. False
negatives are everywhere — code reachable only via reflection, MCP
actions, HTTP routes registered as strings, dynamic dispatch, scheduled
jobs. The graph cannot see these reachability paths.

We need a confidence signal that combines structural reachability with
**semantic relevance**: an item is a strong removal candidate only when
all three are true:

1. No incoming structural edges (`uses`, `mentions`, `claims`)
2. Low semantic similarity to active ADRs/plans/timeline of the last 90
   days
3. No surface-area exposure (not in `convergio-api` schema, not behind
   an HTTP route, not exported via `pub use` from a published crate)

### 1.2. Doc/code semantic drift

ADR-0014's drift detection (`drift.rs`) compares ADR-claimed crates to
actual `uses` edges. This is **structural** drift only.

The drift that hurts more is **semantic**: an ADR body says X, the code
that ADR claims now does Y. Frontmatter `touches_crates` is still
correct, structural drift = 0, but the prose is lying. README of
convergio-edu says "7 domain agents", code has 9 — no edge captures the
mismatch because README cardinality claims aren't modelled as edges.

### 1.3. Context-pack sufficiency

`cvg graph for-task` produces a context-pack. Nobody measures whether
the pack was **enough**. Two failure modes:

- Agent reads files outside the pack during execution (pack incomplete)
- Agent succeeds but the pack contained irrelevant files (pack noisy)

Without instrumentation we ship blind. Substring retrieval cannot
self-diagnose: by definition it doesn't know what it didn't find.

### 1.4. Cross-repo commonality discovery

Roberto operates a fleet. Convergio is Rust. convergio-edu is
TypeScript+Python. VirtualBPM, MirrorHR, WareHouse have their own
stacks. **No AST-based parser tells you "the `Plan` struct in Rust and
the `Curriculum` class in Python are the same shape doing the same
thing"**. AST cross-language is meaningless because the languages don't
share a tree shape.

Embedding-based similarity over (name + docstring + first-N lines)
**does** work cross-language because it lives in semantic space, not
syntactic space.

### 1.5. Fleet-wide optimisation

Once cross-repo similarity is detectable, the next-order question is:
*"What pattern is duplicated across 4+ repos and should be hoisted into
a shared library?"* Today this analysis is impossible without manual
inventory. With cluster detection over cross-repo similarity edges it
becomes a `cvg fleet patterns` command.

### 1.6. Multi-repo orchestration

Convergio's plan/task/audit/gate machinery is scoped to a single repo's
SQLite database. A change like "rollout i18n across all UI machines"
cannot be expressed today as one plan with N task fanout. Each repo
gets its own plan, audit chain is fragmented, no fleet-level evidence
verification, no fleet-level rollback.

---

## 2. Decision Drivers

These are **non-negotiable** constraints derived from the Convergio
Constitution and ADRs already accepted:

| # | Driver | Source |
|---|---|---|
| D1 | **Zero tolerance for tech debt** | CONSTITUTION § Sacred principles, ADR-0004 |
| D2 | **Local-first, single-user, SQLite-only** | ADR-0014, root AGENTS.md |
| D3 | **Rust-only daemon, no scripts** | AGENTS.md root rule, ADR-0014 |
| D4 | **Migrations per crate, range-allocated** | ADR-0003 |
| D5 | **Sub-second on warm cache** | ADR-0014 §Decision Drivers |
| D6 | **Backward compatible — no break of existing `cvg graph` surface** | implied by Convergio Community License v1.3 stability promise |
| D7 | **Test discipline: 524 tests must stay green** | root AGENTS.md |
| D8 | **300-line file cap** | root code style table |
| D9 | **i18n IT/EN day one** | CONSTITUTION P5, ADR-0007 |
| D10 | **Opt-in tech: feature flags for anything probabilistic** | derived from D1 (probabilistic ≠ debt only if optional and tested) |
| D11 | **Audit chain integrity preserved** | ADR-0001 |

---

## 3. Considered Options

### Option A — Status quo: single-repo, structural-only

Keep ADR-0014 as is. Document the limitations. Tell users to maintain
fleet manually with external tooling.

**Pros**: zero engineering cost, zero risk to existing tests.

**Cons**: leaves all 6 problems in §1 unsolved. Convergio remains a
single-repo tool while Roberto's actual workflow is multi-repo. The
product framing in README ("the leash for AI agents") generalises
naturally to fleet but the implementation does not. Strategic dead end.

### Option B — Multi-repo graph, no embeddings

Add a `repo` dimension to the existing graph. Build cross-repo edges
only via explicit declarations (e.g. `convergio-edu/convergio.yaml`
declares `derives_from: convergio`). Keep substring matching.

**Pros**: low risk, no new tech, no probabilistic component, ~2 weeks
of work.

**Cons**: solves §1.6 (multi-repo orchestration) partially but does
not touch §1.1 (semantic dead-code), §1.2 (semantic drift), §1.3
(recall measurement), §1.4 (cross-language commonality), §1.5
(pattern detection). The cross-language gap is intractable without
embeddings. Half a solution leaves the user where they are today on
the highest-pain problems.

### Option C — Embeddings only, single-repo

Add semantic search to the existing single-repo graph. Skip the fleet
abstraction.

**Pros**: validates the embedding tech in isolation. Smaller scope.

**Cons**: misaligned with the actual demand signal. Roberto's pain is
fleet-scoped; staying single-repo means embeddings would be a
nice-to-have rather than load-bearing. Risks landing tech that solves
the wrong problem and being judged on the wrong axis ("did substring
already work fine?" — yes, it did, because the demand signal isn't
single-repo recall).

### Option D — Full fleet brain: multi-repo graph + embeddings + cross-repo plans (this ADR)

Three orthogonal capabilities, layered:

1. **Multi-repo graph** — extend `convergio-graph` with `repo`
   dimension and `tree-sitter`-based language-pluggable parsing
2. **Semantic layer** — new `convergio-embed` crate, `sqlite-vec`
   extension, `fastembed-rs` for inference, multilingual
   embeddings (BGE-M3 small) for Roberto's IT/EN constraint
3. **Fleet abstraction** — new `convergio-fleet` crate, `fleet.toml`
   config, `cvg fleet *` command surface, cross-repo plan/audit
   primitives

Each layer is independently feature-flagged. Each ships in its own
phase. Each has its own go/no-go gate.

**Pros**: solves all 6 problems in §1. Aligns Convergio's product
framing with Roberto's actual workflow. Validation path is incremental
and each phase produces measurable value.

**Cons**: largest engineering investment of the four options. Probab-
ilistic component (embeddings) requires test discipline beyond
existing assertion-based tests. Multi-language parsing introduces a
non-Rust dependency surface (tree-sitter grammars).

### Option E — Use gbrain (gstack) as the fleet brain

External alternative: rely on gbrain (Garry Tan's gstack memory
backend) for fleet-wide retrieval. Convergio stays single-repo;
gbrain handles the cross-repo concerns.

**Pros**: zero engineering cost on Convergio side. Leverages an
existing tool.

**Cons**: violates D2 (local-first, single tool). Violates D3
(introduces JS/Bun runtime as a hard dependency). gbrain is
**transcript-aware not code-aware** — it ingests sessions but does not
parse code. It cannot answer "this struct duplicates that class across
repos" because it has no AST awareness. It is the wrong shape of tool
for the problem. Useful as developer memory; useless as code fleet
brain.

---

## 4. Decision Outcome

**Chosen: Option D (Full fleet brain), staged in three phases with
go/no-go gates.**

Justification:

- **Aligned with demand**: Roberto's stated pain is fleet-scoped. Any
  solution that doesn't cross repo boundaries is mis-targeted.
- **Constitution-compatible**: every probabilistic component is
  feature-flagged (D10). The substring matcher remains as the default
  retrieval path (D6). New crates respect the 300-line cap (D8) and
  migration-range allocation (D4).
- **Incremental value**: each phase produces measurable user value
  before the next phase commits. F1 alone improves single-repo recall;
  F2 alone enables cross-repo discovery; F3 alone enables fleet
  orchestration.
- **Reversible at each gate**: if F1 measures recall improvement <15%,
  we abandon embeddings entirely with sunk cost = 2-3 weeks. F2 and F3
  similarly gated.

---

## 5. Architecture

### 5.1. Crate topology after F3

```
crates/
├── convergio-db/             [unchanged] sqlx pool, migrations
├── convergio-graph/          [extended] +repo dimension, +tree-sitter pluggable parser
├── convergio-embed/          [NEW] fastembed-rs + sqlite-vec, opt-in via feature
├── convergio-parse-multi/    [NEW] tree-sitter wrappers for ts/py/go/etc
├── convergio-fleet/          [NEW] fleet.toml, cross-repo plan/audit/build
├── convergio-durability/     [extended] cross-repo plans (Plan.scope = repo|fleet)
├── convergio-server/         [extended] /v1/fleet/* routes
├── convergio-cli/            [extended] cvg fleet *
├── convergio-api/            [extended] FleetAction schema additions
└── convergio-mcp/            [extended] expose fleet actions
```

Migration ranges (per ADR-0003):
- `convergio-graph`: 600-699 (existing)
- `convergio-embed`: **700-799 (newly allocated)**
- `convergio-fleet`: **800-899 (newly allocated)**

### 5.2. Data model changes

#### 5.2.1. `Node` gains a `repo` field

```rust
pub struct Node {
    pub id: String,           // hash now includes `repo`
    pub repo: String,         // NEW: e.g. "convergio", "convergio-edu"
    pub kind: NodeKind,
    pub name: String,
    pub file_path: Option<String>,
    pub crate_name: String,   // remains; meaningless when repo's lang != Rust
    pub item_kind: Option<&'static str>,
    pub span: Option<(u32, u32)>,
    pub language: Language,   // NEW: Rust | TypeScript | Python | Go | Markdown | Other
}
```

`compute_id` adds `repo` to the hash inputs. Existing rows are
re-parsed with `repo = "convergio"` once and the index rebuilt
(one-shot migration, idempotent).

#### 5.2.2. `EdgeKind` adds cross-repo relationships

| New kind | Source | Target | Generated by |
|---|---|---|---|
| `similar_to` | any node | any node (different repo) | embed batch job, cosine ≥ 0.85 |
| `duplicates` | any node | any node (different repo) | embed + structural shape match, cosine ≥ 0.95 |
| `derives_from` | repo | repo | manual via `convergio.yaml` |
| `consumes_api` | repo | api-version | manual + auto from package.json/Cargo.toml |
| `mirrors_pattern` | cluster | cluster | cluster detection output |

#### 5.2.3. New tables

```sql
-- Migration 0700_embeddings.sql (convergio-embed)
CREATE TABLE graph_node_embeddings (
    repo         TEXT NOT NULL,
    node_id      TEXT NOT NULL,
    model        TEXT NOT NULL,        -- e.g. "bge-m3-small-int8"
    dim          INTEGER NOT NULL,     -- e.g. 384
    vec          BLOB NOT NULL,        -- raw float32[dim] or int8 quantised
    embedded_at  TEXT NOT NULL,
    source_hash  TEXT NOT NULL,        -- hash of the embedded text; re-embed when changes
    PRIMARY KEY (repo, node_id, model)
);

-- sqlite-vec virtual table (loaded as extension)
CREATE VIRTUAL TABLE graph_vec_index USING vec0(
    embedding float[384]
);

-- Migration 0800_fleet.sql (convergio-fleet)
CREATE TABLE fleet_repos (
    name           TEXT PRIMARY KEY,
    path           TEXT NOT NULL,
    language       TEXT NOT NULL,
    parser         TEXT NOT NULL,
    last_built_at  TEXT,
    enabled        INTEGER NOT NULL DEFAULT 1
);

CREATE TABLE fleet_plans (
    id          TEXT PRIMARY KEY,        -- UUID
    title       TEXT NOT NULL,
    scope       TEXT NOT NULL,            -- "fleet" | repo name
    created_at  TEXT NOT NULL,
    -- audit chain hash links to convergio-durability.plans rows
    -- via fleet_plan_repos.repo_plan_id
);

CREATE TABLE fleet_plan_repos (
    fleet_plan_id   TEXT NOT NULL REFERENCES fleet_plans(id),
    repo            TEXT NOT NULL,
    repo_plan_id    TEXT NOT NULL,        -- the per-repo plan in convergio-durability
    PRIMARY KEY (fleet_plan_id, repo)
);
```

### 5.3. Parsing strategy per language

| Language | Parser | Crate |
|---|---|---|
| Rust | `syn` (existing) | `convergio-graph::parse` |
| TypeScript / JavaScript | `tree-sitter-typescript` | `convergio-parse-multi::ts` |
| Python | `tree-sitter-python` | `convergio-parse-multi::py` |
| Go | `tree-sitter-go` | `convergio-parse-multi::go` |
| Markdown (ADR/README/doc) | existing `doc_link.rs` | `convergio-graph::doc_link` |
| Other | `tree-sitter-` (per request) | `convergio-parse-multi::generic` |

Each parser produces the same `(Vec<Node>, Vec<Edge>)` interface. The
caller does not know the language. Universal node kinds across
languages: `module`, `item` (with `item_kind` set per language:
`function`, `class`, `interface`, `type`, etc.).

### 5.4. Embedding pipeline

```
ingestion:                  retrieval:
  parse(repo) → Nodes         query_text
       │                          │
       ▼                          ▼
  select embeddable           embed(query_text)
  (crates, modules,                │
   documented items,                ▼
   ADRs, README, docs)         vec0 KNN over
       │                       graph_vec_index
       ▼                          │
  build_text(node):                ▼
   name + docstring +          structural matches
   first 200 LOC               (substring + static score)
       │                          │
       ▼                          ▼
  embed() →                  RRF fusion
  bge-m3-small-int8          (recall ∪ precision)
       │                          │
       ▼                          ▼
  upsert into                 ContextPack with
  graph_node_embeddings       provenance per match
       +                       (semantic | structural | both)
  insert into vec0
```

**Selective embedding** — we do **not** embed every parsed node. We
embed:
- All `Crate`-kind nodes
- All `Module`-kind nodes (use `//!` doc + first 200 LOC of `lib.rs`/
  `mod.rs`/`__init__.py`/`index.ts`)
- `Item` nodes that have a docstring (skip undocumented private items)
- All `Adr` and `Doc` nodes (chunked at 512 tokens for long bodies)

For a 20-repo fleet at ~10K embeddable units mean, this is ~200K
embeddings × 384 dim × 4 byte = ~300MB on disk. Acceptable for
local-first.

**Model choice — BGE-M3-small (multilingual)**:
- Apache 2.0 licensed
- 384-dim output
- Multilingual (≥ 100 languages) — satisfies CONSTITUTION P5 (IT/EN)
- ONNX format ~120MB
- CPU inference ~5-15ms per text on M-series Mac
- `fastembed-rs` (Apache 2.0) provides a maintained Rust wrapper

**Quantisation** — int8 by default (~75% size reduction, ~2% recall
loss). Float32 mode available behind `embed.precision = "f32"` for
benchmarking.

**Re-embed trigger** — when a node's `source_hash` (SHA-256 of the
embedded text) changes, re-embed. Otherwise skip. Mtime alone is
insufficient (formatter touches don't change semantics). Hash alone
catches semantic-significant edits.

### 5.5. Hybrid retrieval ranking

```
score(node, query) = α · structural(node, query)
                   + (1 - α) · cosine(embed(node), embed(query))
```

Default `α = 0.5` (equal weight). Knob: `[retrieval] alpha = 0.5` in
fleet config. CLI override `--alpha 0.7`.

**Reciprocal Rank Fusion (RRF)** as alternative when scores aren't
directly comparable:

```
RRF(node) = 1 / (k + rank_structural(node))
          + 1 / (k + rank_semantic(node))
```

with `k = 60` (standard). RRF is the default for `cvg fleet for-task`;
linear fusion only when both scores are normalised.

**Provenance metadata** — every node in the returned `ContextPack`
carries `match_source: structural | semantic | both` so the user (and
audit log) can see why a file was included.

### 5.6. Fleet config

`~/.convergio/v3/fleet.toml`:

```toml
[fleet]
name = "roberdan-fleet"
default_branch = "main"

[retrieval]
alpha = 0.5
embed_model = "bge-m3-small-int8"
top_k = 25

[[repo]]
name = "convergio"
path = "/Users/Roberdan/GitHub/convergio"
language = "rust"
parser = "syn"
role = "engine"

[[repo]]
name = "convergio-edu"
path = "/Users/Roberdan/GitHub/convergio-edu"
language = "typescript+python"
parser = "tree-sitter"
role = "downstream"
derives_from = "convergio"

[[repo]]
name = "convergio-ui-framework"
path = "/Users/Roberdan/GitHub/convergio-ui-framework"
language = "typescript"
parser = "tree-sitter"
role = "library"
```

Roles (`engine | library | downstream | sandbox`) inform default
retrieval weighting and dead-code thresholds. An `engine` repo's
unused exports are scored differently than a `sandbox`'s.

### 5.7. CLI surface (additions)

```
cvg fleet add <path>                   add a repo to fleet config
cvg fleet ls                           list fleet repos with last-build status
cvg fleet build [--repo <name>]        build/refresh graph for repo or all
cvg fleet stats                        node/edge counts per repo, embedding coverage

cvg fleet for-task <id> [--repo <r>]   cross-repo context-pack
                       [--gap-check]   include semantic expansions over substring
                       [--alpha 0.5]   override fusion weight

cvg fleet rot [--threshold 0.3]        dead-code candidates with semantic confidence
cvg fleet doc-drift                    semantic drift between docs and code
cvg fleet patterns [--min-repos 3]     cross-repo pattern clusters
cvg fleet duplicates [--cosine 0.95]   near-exact cross-repo dupes (hoist candidates)

cvg fleet plan create "..."            fleet-scoped plan with per-repo task fanout
cvg fleet plan ls
cvg fleet validate <plan-id>           run validators across all touched repos
```

Existing `cvg graph *` commands remain unchanged and operate on the
current repo (single-repo back-compat per D6).

### 5.8. HTTP API additions

```
POST   /v1/fleet/repos                  add repo to fleet
GET    /v1/fleet/repos                  list
POST   /v1/fleet/build                  trigger build (idempotent)
GET    /v1/fleet/stats
GET    /v1/fleet/for-task/:id
GET    /v1/fleet/rot
GET    /v1/fleet/doc-drift
GET    /v1/fleet/patterns
GET    /v1/fleet/duplicates
POST   /v1/fleet/plans
GET    /v1/fleet/plans/:id
POST   /v1/fleet/plans/:id/validate
```

All routes follow existing `axum 0.7` `:id` convention (per root
AGENTS.md).

### 5.9. Audit chain compatibility

A fleet plan's `id` is hash-linked into both `fleet_plans` and the
per-repo `plans` rows it fans out to. The audit chain remains intact
per repo (ADR-0001 unchanged); fleet-level integrity verifies that
`fleet_plan_repos.repo_plan_id` exists in each touched repo's plan
table and that every repo plan's evidence rows are present.

`cvg audit verify --fleet` walks the fleet chain in addition to the
per-repo chain.

---

## 6. Implementation Phases

### F1 — Single-repo embedding prototype (2-3 weeks)

**Scope**:
- Implement `convergio-embed` crate with `fastembed-rs` + `sqlite-vec`
- Embed Convergio's own repo only (50 crates, ~3K embeddable units)
- Add `cvg graph for-task --semantic` flag, default off
- Build a golden set: 30 historical tasks with hand-curated
  expected-pack files (recall@10 ground truth)
- Measure: substring-only recall@10 vs hybrid recall@10

**Go/no-go gate**:
- Hybrid recall@10 improves by **≥ 15%** absolute over substring-only
- p95 query latency stays **< 1s** on the 3K-node graph
- Embedding storage **< 50MB**
- Re-embed on incremental rebuild **< 30s** for typical 5-file change

**No-go path**: abandon embeddings entirely. Cost: 2-3 weeks of work.
Document findings in a follow-up ADR. Revert feature flag, keep
substring-only.

**Deliverables**:
- `crates/convergio-embed/` (≤ 800 LOC)
- Golden set: `tests/fixtures/retrieval-golden/` (30 tasks)
- Bench harness: `crates/convergio-embed/benches/recall.rs`
- Migration `0700_embeddings.sql`
- Updated `cvg graph for-task` with `--semantic` and `--gap-check`
  flags
- One new ADR (0038) documenting the recall measurement methodology

### F2 — Multi-repo opening (4-6 weeks, gated by F1)

**Scope**:
- Implement `convergio-parse-multi` with TypeScript and Python
  tree-sitter parsers (the two languages Roberto's fleet uses today
  beyond Rust)
- Implement `convergio-fleet` with `fleet.toml`, `cvg fleet add/ls/
  build`, multi-repo graph schema (Node.repo + Node.language)
- Backfill: re-parse Convergio with `repo = "convergio"`, add convergio-
  edu and convergio-ui-framework
- Cross-repo similarity batch: nightly or on-demand `cvg fleet build
  --refresh-similarity` that computes `similar_to` and `duplicates`
  edges
- Implement `cvg fleet patterns` and `cvg fleet duplicates`

**Go/no-go gate**:
- Find **≥ 3 real cross-repo patterns** between Convergio +
  convergio-edu + convergio-ui-framework (e.g. plan/curriculum FSM,
  auth, request-id propagation, i18n bundle loading)
- False positive rate on `duplicates` (cosine ≥ 0.95) **< 20%**
  measured on 50 sampled pairs reviewed by Roberto
- All 524 existing tests still green; ≥ 60 new tests added

**No-go path**: keep F1, drop fleet/multi-repo. Cost: 4-6 weeks. Likely
indicator that fleet abstraction needs more thought (e.g. workspace
graph is intra-repo concern only).

**Deliverables**:
- `crates/convergio-parse-multi/` (≤ 600 LOC)
- `crates/convergio-fleet/` (≤ 1200 LOC)
- Migration `0800_fleet.sql`
- TypeScript and Python parser fixtures
- ADR-0036 "Multi-language parsing via tree-sitter"
- ADR-0037 "Fleet config schema and lifecycle"

### F3 — Fleet-grade orchestration (6-10 weeks, gated by F2)

**Scope**:
- Cross-repo plans (`fleet_plans` table, `cvg fleet plan create`)
- Fleet-scoped audit verify (`cvg audit verify --fleet`)
- `cvg fleet rot` (semantic dead-code) with role-aware thresholds
- `cvg fleet doc-drift` (snapshot embedding at commit, re-compare on
  query)
- MCP fleet actions (`convergio.fleet.for_task`, `convergio.fleet.
  validate`)
- `cvg fleet validate` runs gate pipeline across all touched repos
  with per-repo verdict + fleet-level rollup

**Go/no-go gate**:
- Roberto operates a real cross-repo task end-to-end on his fleet
  (e.g. "i18n rollout", "logging-format unification") via
  `cvg fleet plan` and the audit chain verifies green at fleet level
- Daily fleet rebuild (incremental) completes **< 5 min** for the
  first 5 repos

**No-go path**: ship F1+F2 as v3.x feature, document F3 as future
work. Cost: 6-10 weeks. Likely indicator that audit chain federation
needs a deeper redesign (separate ADR).

**Deliverables**:
- Fleet plan schema migration `0801_fleet_plans.sql`
- Cross-repo audit verifier
- ADR-0038 "Fleet plan/audit federation model"
- New PRD addendum on fleet-level evidence verification
- Documentation: `docs/spec/fleet-orchestration.md`

---

## 7. Test Strategy

### 7.1. Deterministic tests (existing pattern)

All structural code (parser, graph schema, fleet config, plan
fan-out, audit chain extension) follows the existing `#[test]` /
`#[tokio::test]` pattern. **No regression** in the 524 current tests
is acceptable.

### 7.2. Probabilistic tests (new pattern)

For embedding-driven features we use **golden-set + threshold
assertions**, not exact equality:

```rust
#[tokio::test]
async fn recall_at_10_meets_threshold() -> Result<()> {
    let golden = load_golden_set("tests/fixtures/retrieval-golden")?;
    let mut total_recall = 0.0;
    for task in &golden {
        let pack = pool.fleet_for_task(&task.id, alpha = 0.5).await?;
        let recall = recall_at_k(&pack.files, &task.expected_files, 10);
        total_recall += recall;
    }
    let avg = total_recall / golden.len() as f64;
    assert!(avg >= 0.85, "recall@10 = {avg} below threshold 0.85");
    Ok(())
}
```

### 7.3. CI cost control

Embedding tests run on every PR but use a **fixed model cache**
(`~/.cache/convergio/embed-models/`) and a 50-task subset of the
golden set. Full 30-task validation runs nightly on main only.

### 7.4. Cross-language fixture set

A new `tests/fixtures/fleet/` directory contains 3 mini-repos
(Rust + TypeScript + Python) with deliberate cross-language pattern
duplicates, used to assert `cvg fleet patterns` correctness.

---

## 8. Migration & Rollback

### 8.1. Forward migration

Adding the `repo` column to `graph_nodes` is backward-compatible:
- Migration `0601_repo_dimension.sql` adds nullable column
- Backfill updates existing rows with `repo = 'convergio'`
- Migration `0602_repo_not_null.sql` adds NOT NULL constraint
- All on first daemon start after upgrade; idempotent

Embeddings table is purely additive (migration `0700`); no risk to
existing data.

Fleet tables are additive (migration `0800`); no risk.

### 8.2. Rollback

Each phase is independently revertible:

- **F1 rollback**: drop `graph_node_embeddings` table, remove
  `convergio-embed` from workspace, remove `--semantic` flag. The
  `repo` column on `graph_nodes` (added in F2) is independent.
- **F2 rollback**: drop fleet tables, remove `convergio-fleet` and
  `convergio-parse-multi` from workspace, remove `--repo` filter on
  `cvg graph` commands. Embeddings (F1) survive.
- **F3 rollback**: drop fleet plan tables, remove cross-repo audit
  verifier. Multi-repo graph (F2) survives.

No data loss in any rollback path because feature data lives in
dedicated tables.

---

## 9. Risks & Mitigations

| # | Risk | Likelihood | Impact | Mitigation |
|---|---|---|---|---|
| R1 | Embedding model produces nonsensical similarities for code | Medium | High | Golden set with hand-graded fixtures gates merge; threshold recall@10 ≥ 0.85 enforced in CI |
| R2 | Embedding storage grows unboundedly | Low | Medium | Selective embedding (§5.4); int8 quantisation; cap at 50K embeddings per repo with eviction policy on least-recently-queried |
| R3 | Cross-language false-positive duplicates flood `cvg fleet duplicates` | Medium | Medium | Cosine threshold + structural shape match (same arity, same kind). Manual review queue for borderline cases |
| R4 | Tree-sitter grammar drift breaks parser | Low | Medium | Pin grammar versions in Cargo.toml; CI fixture for each language asserts known shapes still parse |
| R5 | Multilingual model performs worse on technical content | Medium | Medium | Benchmark BGE-M3-small vs alternatives (jina-embeddings-v3, mE5-small) during F1; switch model is one config change |
| R6 | sqlite-vec extension breaks on macOS/Linux variant | Low | High | Vendor pre-built binaries per platform; CI matrix covers macOS arm64 + linux x86_64 + linux arm64 |
| R7 | Audit chain federation introduces non-determinism | Low | Critical | Each repo's chain stays canonical (ADR-0001 unchanged); fleet-level chain is **derived view**, not authoritative |
| R8 | F3 audit federation needs a separate, larger redesign | Medium | Medium | F3 has its own go/no-go gate; if blocked, ship F1+F2 and defer F3 |
| R9 | Probabilistic tests become flaky in CI | Medium | High | Fixed model + fixed seed + deterministic int8 quantisation; recall@10 has hard floor not exact match |
| R10 | Roberto's fleet outgrows local SQLite (>100 repos) | Low (today) | Medium | Out of scope for this ADR; postgres backend is a future ADR if/when |

---

## 10. Performance Budget

| Operation | Budget | Notes |
|---|---|---|
| `cvg fleet build` cold (3 repos, ~30K nodes) | ≤ 5 min | parser + embed pass |
| `cvg fleet build --incremental` (5 files) | ≤ 30 s | only re-embed changed nodes |
| `cvg fleet for-task` warm cache | ≤ 1 s p95 | structural + vec0 KNN over ≤ 200K vecs |
| `cvg fleet for-task --gap-check` | ≤ 2 s p95 | additional embedding pass on query |
| `cvg fleet patterns` | ≤ 30 s | clustering pass over similar_to edges |
| `cvg fleet rot` | ≤ 10 s | join over edges + embeddings |
| `cvg fleet doc-drift` | ≤ 15 s | re-compare snapshot embeddings vs current |
| `cvg fleet duplicates` | ≤ 20 s | KNN brute-force across repos |
| Daemon memory overhead (idle) | + 250 MB | model loaded at first query, stays warm |

---

## 11. Compatibility & Versioning

- `convergio-api` schema is **additive only** in F2/F3. Existing
  agent contracts unchanged.
- `convergio-mcp` exposes new `convergio.fleet.*` actions; existing
  `convergio.help` and `convergio.act` unchanged.
- The Convergio Community License v1.3 covers the new crates with no
  modification needed.
- F1 ships as Convergio v3.x (minor); F2 ships as v3.y; F3 ships as
  Convergio v4.0 because cross-repo plans materially change the data
  model contract.

---

## 12. Open Questions

These need answers before F2 starts (not blocking F1):

- **Q1**: Does the fleet have one daemon (one `state.db` per fleet)
  or one daemon per repo (federated)? § 5.9 leans single, but
  multi-machine flotta evolution may push federated. Decision deferred
  to ADR-0037.
- **Q2**: How do we name the `repo` for forks/worktrees? Slug from
  remote URL? From path basename? Mixed?
- **Q3**: What happens when a fleet repo moves on disk? Symlink
  resolution? Re-add via `cvg fleet add`?
- **Q4**: How does `cvg fleet doc-drift` snapshot embeddings at
  commit time without a git hook? Background job on push? On every
  daemon start?
- **Q5**: Does fleet-scoped audit need its own merkle root, or is the
  derived-view sufficient (R7)?

These do not block F1. They block F2 design lock.

---

## 13. References

- ADR-0001 — Hash-chained audit log
- ADR-0003 — Migration range allocation per crate
- ADR-0004 — Three sacred principles
- ADR-0014 — Code-graph layer for Tier-3 context retrieval
- ADR-0015 — Auto-regenerated docs sections
- ADR-0029 — TUI dashboard crate separation
- ADR-0030 — Crate versioning policy
- CONSTITUTION.md — Sacred principles (§ Zero tolerance, § Local-first,
  § P5 i18n)
- README — Convergio Edu architecture (heterogeneous fleet evidence)
- gstack memory.md — gbrain reference (Option E rejected rationale)
- `crates/convergio-graph/src/query.rs` — explicit "future work"
  comment on embeddings as Tier-3 evolution
- `crates/convergio-graph/migrations/0600_graph_nodes_edges.sql` —
  current schema baseline
- BGE-M3 paper (Chen et al. 2024) — multilingual embedding model
- `fastembed-rs` — Apache 2.0, Rust ONNX wrapper
- `sqlite-vec` (Alex Garcia, MIT) — vector extension for SQLite
- `tree-sitter` — language-agnostic incremental parser

---

## 14. Decision Log

| Date | Author | Decision |
|---|---|---|
| 2026-05-03 | Roberto + claude | ADR drafted as proposed |
| TBD | Roberto | Approve F1 scope and budget |
| TBD | F1 retrospective | Go/no-go for F2 |
| TBD | F2 retrospective | Go/no-go for F3 |

---

*End of ADR-0038*
