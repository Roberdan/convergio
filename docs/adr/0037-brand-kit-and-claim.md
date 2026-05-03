---
id: 0037
status: accepted
date: 2026-05-03
topics: [brand, cli, tui, daemon, accessibility, i18n]
related_adrs: [0029]
touches_crates: [convergio-brand, convergio-cli, convergio-tui, convergio-server, convergio-i18n]
last_validated: 2026-05-03
---

# 0037. Brand kit, claim, and shared `convergio-brand` crate

- Status: accepted
- Date: 2026-05-03
- Tags: brand, cli, tui, accessibility

## Context

Convergio shipped without a single source of truth for its visual
identity. The TUI carried its own gradient (cyan→magenta), the
daemon had no boot banner, and there was no canonical claim. The
README opened with a working description ("the shovel for the long
tail of vertical AI accelerators") that explained the *mechanism*
(refuse-by-default gates) but did not give the product a slogan
that operators could repeat.

The visual brand kit shipped externally (logo, hexagonal mark,
neon palette, glitch animation, demo Bash + Rust scripts) was
never wired in. Rolling it in piecemeal — every crate hand-coding
`#FF00B4` — would guarantee drift the moment the kit changes.

## Decision

1. **Adopt one claim everywhere:** *Make machines prove it.*
   - Subline: *The machine that builds machines — and proves they
     work.*
   - The claim restates the gate-pipeline mechanism in operator
     vocabulary: gates turn "agent says it works" into "agent
     proves it works".
2. **Introduce a zero-dependency `convergio-brand` crate** as the
   single source of truth for palette, claim, wordmark, gradient,
   glitch, and boot animation. Every other crate that paints
   anything user-facing imports from here.
3. **Brand marks (claim, subline, product name) are not
   translated.** They are trade dress. Surrounding prose
   (descriptions, help text, "type cvg --help to get started")
   still flows through `convergio-i18n` (CONSTITUTION P5). The new
   `brand-about-*` keys live in both `en` and `it` bundles from day
   one.
4. **Boot animation runs by default** on `convergio-server start`
   and `cvg about`, with three guards that keep CONSTITUTION P3
   intact:
   - `NO_COLOR=1` (no-color.org) → static, plain ASCII.
   - Stdout is not a TTY → static, plain ASCII (CI / piped).
   - `CONVERGIO_THEME=hc|mono|color` → explicit operator override.
5. **Asset PNGs live under `assets/branding/`** (logo variants,
   hexagonal mark, screenshot mockup, the original demo
   Bash + Rust scripts). The kit is part of the repo so anyone can
   rebuild marketing surfaces from source.
6. **TUI keeps its semantic palette** (success/warning/danger
   glyphs, focus, highlight) and only swaps the wordmark gradient
   to source from `convergio_brand::{MAGENTA, CYAN}`. Status
   colours were chosen for accessibility (CONSTITUTION P3); the
   brand kit overrides only what is decorative.

## Consequences

- Any future palette change is a one-line edit in
  `convergio-brand/src/palette.rs` and propagates everywhere.
- Daemon `convergio start` now produces visible output before the
  first `tracing::info!` line. Operators running under
  `systemd`/`launchd` (no TTY) get the existing log-only behaviour
  unchanged.
- The `cvg about` subcommand gives operators a stable way to
  identify which Convergio they are talking to (binary version,
  source URL, claim) without paying for a daemon round-trip.
- Adding a new locale now requires translating three additional
  Fluent keys (`brand-about-tagline`, `brand-about-source`,
  `brand-about-help`).

## Alternatives considered

- **Inline the palette in each crate.** Rejected: guaranteed drift,
  identical to the failure mode that prompted ADR-0029.
- **Replace TUI palette wholesale with brand neon.** Rejected:
  would invalidate the WCAG audit baked into `convergio-tui`'s
  semantic palette (CONSTITUTION P3).
- **Skip the boot animation, keep it strictly opt-in.** Rejected:
  the user explicitly asked for a default boot animation; the
  three-way guard (NO_COLOR / TTY / theme) keeps the default safe
  for CI and accessibility.
