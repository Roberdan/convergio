# AGENTS.md — convergio-ontology

For repo-wide rules see [../../AGENTS.md](../../AGENTS.md).

This crate is the **Ontology Runtime Core** (ADR-0053). It owns the
platform-side schema registry of typed domain objects, links, and
properties, and the deterministic exporters that publish those
schemas as JSON-Schema and SHACL.

## Responsibility

- Define the `ObjectType` / `LinkType` / `PropertyType` shape and
  the schema-evolution rules (additive vs breaking, content hash,
  semver `schema_version`).
- Provide deterministic, byte-identical export to JSON-Schema and
  SHACL — same posture as `actions.json` (ADR-0047) and graph
  output (ADR-0060).
- Expose the `cvg ontology` CLI surface and the MCP `ontology.*`
  actions (registered in `convergio-api/actions.json` per
  ADR-0047).

## Boundaries

- **No domain content.** This crate is a registrar, not a
  librarian. Convergio ships zero built-in `ObjectType` instances;
  the concrete shapes come from verticals (`convergio-edu`,
  `convergio-research`, ...).
- **Persistence is owned by this crate.** SQLite tables live in
  `migrations/1000_*.sql` (range 1000-1099 per ADR-0003). This
  crate consumes the shared pool from `convergio-db`; per
  ADR-0003 each layer crate owns its own migration range, so we
  do not stash schema in `convergio-db` itself.
- **No runtime IO outside the daemon process.** The library
  compiles into the server; CLI flows go through the daemon HTTP
  surface like every other capability (ADR-0001, ADR-0043).
- Dependency on `convergio-graph` is allowed only for drift
  detection between schema versions (ADR-0053 § Decision Drivers).
  Do not reach into graph internals for anything else.

## Invariants

- Every exporter is covered by a golden test. Drift between the
  rendered output and the golden fixture is a CI failure, not a
  warning.
- Schema evolution requiring `breaking = true` MUST reference a
  migration plan id in the daemon — enforced at registration time,
  not just at export time.
- The crate respects the 300-line/file cap (CONSTITUTION) and the
  workspace `missing_docs = "warn"` lint (every `pub` item carries
  a `///` block).
- The CLI surface follows ADR-0043: `--output human|json|plain`
  on every subcommand; the graph-shaped subcommands additionally
  honour ADR-0060 (`--output mermaid|dot`, deterministic ordering).

## Tests

```bash
cargo test -p convergio-ontology
RUSTFLAGS="-Dwarnings" cargo clippy -p convergio-ontology --all-targets -- -D warnings
```

## Crate stats

The block below is rewritten by `cvg docs regenerate` (ADR-0015) —
do not edit between the markers.

<!-- BEGIN AUTO:crate_stats -->
**`convergio-ontology` stats:** 9 `*.rs` files / 23 public items / 1372 lines (under `src/`).

Files approaching the 300-line cap:
- `src/shacl.rs` (263 lines)
<!-- END AUTO -->
