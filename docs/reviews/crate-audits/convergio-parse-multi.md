# Audit — `convergio-parse-multi`

The crate is small, focused, and crate-scoped clippy-clean.
The main risk is production parser initialization using `expect()`.
Docs mostly match behavior, with stale entry-point/test wording in `AGENTS.md`.

## Bugs / smells
| Severity | Location | Finding | Suggested action |
|----------|----------|---------|------------------|
| high | src/parse.rs:18 | Production parsing panics on grammar version mismatch via `expect()`. | Return a typed `ParseError` for grammar setup failures. |
| high | src/ts.rs:39 | Production TypeScript graph parsing panics on grammar version mismatch via `expect()`. | Return a typed `ParseError` for grammar setup failures. |
| high | src/py.rs:57 | Production Python graph parsing panics on grammar version mismatch via `expect()`. | Return a typed `ParseError` for grammar setup failures. |
| medium | src/parse.rs:28 | Syntax errors are only logged even though `ParseError::SyntaxError` exists. | Either return `SyntaxError` or document partial-parse semantics and remove the unused variant. |
| medium | src/ts.rs:49 | TypeScript syntax errors are only logged and still produce graph nodes. | Either return `SyntaxError` or document partial-parse semantics for graph callers. |
| medium | src/py.rs:67 | Python syntax errors are only logged and still produce graph nodes. | Either return `SyntaxError` or document partial-parse semantics for graph callers. |

## Code↔doc drift
| Severity | Location | Finding | Suggested action |
|----------|----------|---------|------------------|
| medium | crates/convergio-parse-multi/AGENTS.md:15 | "`parse()` is the single entry point" conflicts with public `parse_ts()` and `parse_py()` entry points. | Update the invariant to distinguish lightweight parsing from graph-compatible parsing. |
| low | crates/convergio-parse-multi/AGENTS.md:29 | Module layout omits the public `py.rs` and `ts.rs` parser modules. | Add `py.rs` and `ts.rs` to the module layout table. |
| low | crates/convergio-parse-multi/AGENTS.md:39 | Per-module test coverage claim is stale for `py.rs` and `ts.rs`, which rely on integration tests. | Reword the tests section to mention integration coverage for graph parsers. |

## Refactor / optimization
| Severity | Location | Finding | Suggested action |
|----------|----------|---------|------------------|
| low | src/py.rs:49 | `py.rs` is 291 lines and close to the 300-line cap. | Split method/docstring helpers into a small `py_extract.rs` module before adding behavior. |
| low | src/ts.rs:37 | Parser setup and graph module-node construction duplicate the Python parser shape. | Extract shared setup/module-node helpers if another language is added. |

## Constitution compliance
| Severity | Location | Finding | Suggested action |
|----------|----------|---------|------------------|
| high | src/parse.rs:18 | P1 zero-tolerance is violated by a production panic path in parser setup. | Replace `expect()` with typed error propagation. |
| high | src/ts.rs:39 | P1 zero-tolerance is violated by a production panic path in TypeScript parser setup. | Replace `expect()` with typed error propagation. |
| high | src/py.rs:57 | P1 zero-tolerance is violated by a production panic path in Python parser setup. | Replace `expect()` with typed error propagation. |
| medium | src/parse.rs:28 | P1 zero-tolerance is weakened because syntax errors are logged and accepted. | Make syntax-error acceptance explicit and gateable, or return `SyntaxError`. |

## Confidence
high — Read `AGENTS.md`, `src/lib.rs`, all 8 source files, both integration tests, and ran crate-scoped clippy cleanly.
