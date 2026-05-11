# Audit — `convergio-i18n`

Small focused i18n crate with no unsafe code, no production unwrap/expect paths, and clean clippy.
English and Italian bundles are embedded, load-tested, and cross-checked for key coverage.
Findings are low severity: plural-format diagnostics are weaker than `t()`, and README locale/test wording has drifted.

## Bugs / smells
| Severity | Location | Finding | Suggested action |
|----------|----------|---------|------------------|
| low | src/bundle.rs:94 | `t_n` collects Fluent format errors but never logs them, unlike `t`. | Emit the same warning used by `t` when plural formatting returns errors. |
| low | src/bundle.rs:115 | `t_n_with` collects Fluent format errors but never logs them, unlike `t`. | Emit the same warning used by `t` when plural formatting returns errors. |

## Code↔doc drift
| Severity | Location | Finding | Suggested action |
|----------|----------|---------|------------------|
| low | README.md:13 | Quickstart says locale detection checks `LANG` before fallback, but code checks `LC_ALL` before `LANG`. | Update the README to list `LC_ALL` and the implemented priority. |
| low | README.md:28 | README names `every_locale_loads` plus generic cross-coverage tests, but actual test names are `italian_has_every_english_key` and `english_has_every_italian_key`. | Update the README wording to avoid stale test-name coupling. |

## Refactor / optimization
| Severity | Location | Finding | Suggested action |
|----------|----------|---------|------------------|
| low | src/bundle.rs:55 | `t`, `t_n`, and `t_n_with` duplicate missing-message, pattern lookup, argument setup, and formatting logic. | Extract a private formatter helper if another translation helper is added. |

## Constitution compliance
| Severity | Location | Finding | Suggested action |
|----------|----------|---------|------------------|
| low | src/bundle.rs:94 | P1 diagnostics are weaker for plural strings because `t_n` silently drops Fluent format errors. | Log plural formatting errors consistently with `t`. |
| low | src/bundle.rs:115 | P1 diagnostics are weaker for plural strings because `t_n_with` silently drops Fluent format errors. | Log plural formatting errors consistently with `t`. |

## Confidence
high — Read `AGENTS.md`, `src/lib.rs`, every `src/**/*.rs` file, crate README, locale bundles, coverage tests, and ran `cargo clippy -p convergio-i18n --all-targets -- -D warnings`.
