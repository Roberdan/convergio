# Audit — `convergio-brand`

Small, dependency-free crate with clean clippy and no production panic/unsafe paths.
The main risks are accessibility contract drift around `NO_COLOR`, high contrast, and "ASCII-only" wording.
No files are near the 300-line cap; refactor pressure is minimal.

## Bugs / smells
_None._

## Code↔doc drift
| Severity | Location | Finding | Suggested action |
|----------|----------|---------|------------------|
| medium | src/theme.rs:35 | `CONVERGIO_THEME=color` resolves before `NO_COLOR`, despite crate docs claiming every output respects `NO_COLOR`. | Decide the intended precedence and update the resolver or docs/tests. |
| medium | src/theme.rs:25 | `HighContrast` is documented as white-on-black and bold-only, but renderers treat it as plain mono output. | Implement high-contrast styling or narrow the docs. |
| low | src/banner.rs:25 | `Theme::Mono` lockup is documented as ASCII-only, but `hexagonal_c` emits box-drawing Unicode glyphs. | Change the wording to no-color/plain output or add ASCII rows for mono. |

## Refactor / optimization
| Severity | Location | Finding | Suggested action |
|----------|----------|---------|------------------|
| low | src/banner.rs:112 | `strip_ansi` test helper is duplicated with `boot.rs`. | Extract a shared test helper only if more ANSI assertions are added. |

## Constitution compliance
| Severity | Location | Finding | Suggested action |
|----------|----------|---------|------------------|
| medium | src/theme.rs:35 | P3 accessibility promise is weakened because explicit color mode can bypass `NO_COLOR`. | Make `NO_COLOR` win unless an intentional force-color mode is documented. |
| medium | src/theme.rs:25 | P3 high-contrast path is effectively mono, not the documented white-on-black bold-only branch. | Add actual high-contrast rendering and end-to-end assertions. |

## Confidence
high — Read `AGENTS.md`, `src/lib.rs`, every `src/**/*.rs` file, checked crate README absence, and ran `cargo clippy -p convergio-brand --all-targets -- -D warnings`.
