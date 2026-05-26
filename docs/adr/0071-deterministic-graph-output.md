---
id: 0060
status: proposed
date: 2026-05-25
topics: [ontology, diff, visualization, output-format]
related_adrs: [0043, 0051, 0053, 0056]
touches_crates: [convergio-ontology, convergio-cli]
last_validated: 2026-05-25
---

# 0060. Deterministic Diff / Mermaid / Graphviz output format

- Status: proposed
- Date: 2026-05-25
- Tags: ontology, diff, visualization

## Context

ADR-0051 introduces `cvg ontology diff <from> <to>` for schema
versions; ADR-0053 introduces `cvg ontology lineage <id>`;
ADR-0056 introduces `cvg ontology branch diff <name>`. All
three produce graph-shaped output. The default `--output
human|json|plain` contract (ADR-0043) is right for tabular
data but awkward for graphs.

We need a small, stable, deterministic graph-output format set
so that:

- PR descriptions can embed diagrams (Mermaid renders inline
  on GitHub).
- External rendering pipelines can consume the output
  (Graphviz `dot`).
- CI can diff diagrams byte-for-byte across runs.

## Decision

Extend the `--output` contract for the three graph-producing
commands with two additional formats:

1. **`--output mermaid`** — emits a Mermaid `flowchart`
   (or `gantt` for bitemporal lineage where appropriate)
   block, with deterministic node ordering (sorted by stable
   identifier) and deterministic edge ordering.
2. **`--output dot`** — emits Graphviz `digraph { ... }`
   with the same determinism guarantees. Quoted node labels;
   no embedded comments that vary per run.

### Determinism contract

- Identical inputs MUST produce byte-identical output across
  runs and across machines. A golden test per command
  enforces this.
- No timestamps in the rendered graph (the operation
  timestamp is metadata, not graph content).
- Hash references are abbreviated to 7 characters (Git
  convention) in `mermaid` for legibility; `dot` retains
  full hashes.

### Commands covered

| Command | Formats added |
|---|---|
| `cvg ontology diff` | mermaid, dot |
| `cvg ontology lineage` | mermaid, dot |
| `cvg ontology branch diff` | mermaid, dot |

### Non-goals

- No SVG / PNG rendering. We emit text; external tools render.
- No interactive output. This is for documents and CI.
- No new format outside Mermaid / Graphviz. PlantUML, D2, etc.
  are out of scope — adding more is a follow-up.

## Decision Drivers

- ADR-0043 ID/payload consistency: extending the existing
  `--output` axis costs less than a new flag.
- Diagrams in PR descriptions are a force-multiplier for
  reviewer comprehension of bitemporal and lineage changes.
- Determinism is a P1 invariant for any byte-diffable output.

## Considered Options

1. **External post-processor (jq + custom template).**
   Rejected — every accelerator would re-implement it.
2. **Embed a rendering library.** Rejected — only emit text;
   keep dependencies thin.
3. **This proposal — two text formats, deterministic.**
   Accepted.

## Compliance Anchors

- P1 zero-debt: determinism enforced by golden tests.
- ADR-0043: the `--output` contract stays the surface.

## Rollout

- Folded into plan *[core] Ontology Runtime W1: Runtime Core*
  as one additional task: "Mermaid + dot output for ontology
  diff/lineage/branch-diff (ADR-0060)".
- Test fixtures live under
  `crates/convergio-ontology/tests/golden/diff/`.

## Consequences

- PRs touching the ontology can embed a Mermaid block in the
  description; reviewers see the structure without local
  tooling.
- CI gains three more golden tests (one per command).
- A future ADR may add a `--output d2` format if demand
  surfaces.

## References

- ADR-0043 API ID and payload consistency
- ADR-0051 ontology runtime core
- ADR-0053 bitemporal + lineage
- ADR-0056 scenario branching
