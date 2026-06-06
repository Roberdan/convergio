# convergio-ontology-author

LLM-assisted ontology authoring (ADR-0080). Turn **documents** and/or a
generic **intent** (a prompt + industry + use-case) into a domain
ontology expressed in **standard languages** usable by Convergio and by
external tools (Protégé, RDF validators, code generators).

This crate is a **leaf**: nothing else depends on it. It ships a PoC
binary and never writes to the ontology registry — it emits a reviewable
draft.

## What it produces

| Artifact | Format | Consumer |
|----------|--------|----------|
| `owl/<name>.ttl` | OWL 2 Turtle | Protégé, RDF tools |
| `jsonschema/<Obj>.schema.json` | JSON-Schema | validators, codegen |
| `shacl/<Obj>.shacl.jsonld` | SHACL JSON-LD | RDF validators |
| `ontology.json` | Convergio draft | later registry import |
| `provenance.json` | W3C PROV-JSON | audit |

## Pipeline (thesis, not a wrapper)

1. **Ingest** documents via `markitdown` (never LibreOffice).
2. **Prompt** the model, constrained to the `DraftOntology` JSON-Schema.
3. **Propose** by shelling out to the operator's vendor CLI (ADR-0032 —
   never a raw HTTP API).
4. **Validate**: RDF-safe names, datatype allowlist, link/property
   closure, uniqueness.
5. **Repair** loop: re-prompt with violations, up to a budget.
6. **Emit** standard artifacts + provenance for every source document.

The machine must *prove* its output: invalid drafts are never written.

## PoC usage

```bash
# From an intent alone:
cargo run -p convergio-ontology-author -- \
  --prompt "model a student information system" \
  --industry higher-education --use-case sis \
  --out ./ontology-out

# Grounded in standards documents:
cargo run -p convergio-ontology-author -- \
  --doc ./OneRoster.pdf --doc ./EDCI.pdf \
  --use-case sis --out ./ontology-out
```

The LLM step uses the `--proposer-bin` vendor CLI (default `claude`).
Tests use deterministic stubs, so the pipeline is fully covered without
a network or model.
