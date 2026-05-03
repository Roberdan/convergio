# AGENTS.md — convergio-brand

For repo-wide rules see [../../AGENTS.md](../../AGENTS.md).

This crate owns the **visual identity** of Convergio: the claim,
the palette, the wordmark, and the boot animation. Every other
crate that paints something user-facing imports from here instead
of hard-coding colours or strings.

## Invariants

- **Single source of truth.** `MAGENTA`, `CYAN`, `BLACK`, `CLAIM`,
  `SUBLINE`, `PRODUCT_NAME` live here and nowhere else. If you find
  yourself typing `#FF00B4` in another crate, stop and import.
- **Brand marks are not translated** (CONSTITUTION P5). The claim
  and product name are trade dress; surrounding prose flows through
  `convergio-i18n` instead.
- **Accessibility is a first-class branch** (CONSTITUTION P3). Every
  output respects `NO_COLOR` and TTY detection. The
  `Theme::HighContrast` variant must always be testable end-to-end.
- **No background work, no spawned threads.** The boot orchestrator
  is sink-agnostic: callers pass any `io::Write` plus a `Sleeper`.
  Tests inject `NoSleep`.
- **Zero deps in production.** The crate ships with no external
  runtime dependencies on purpose, so any other crate (CLI, TUI,
  daemon) can pull it in without churning the dependency tree.

## Module map

| File | Owns |
|------|------|
| `src/lib.rs` | Crate doc and re-exports |
| `src/palette.rs` | `Rgb` + brand colour constants |
| `src/claim.rs` | `CLAIM` / `SUBLINE` / `PRODUCT_NAME` |
| `src/theme.rs` | `Theme` enum + env-driven resolver |
| `src/gradient.rs` | RGB lerp + truecolor escape sequences |
| `src/glitch.rs` | Deterministic char-swap frames |
| `src/banner.rs` | Wordmark + hexagonal lockup |
| `src/boot.rs` | Boot-animation orchestrator |

## Tests

Pure unit tests, no fixtures, no I/O. Boot animation is exercised
with a `Vec<u8>` sink and the `NoSleep` sleeper so tests never
block.
