# AGENTS.md — convergio-i18n

For repo-wide rules see [../../AGENTS.md](../../AGENTS.md).

This crate protects the internationalization-first rule.

## Invariants

- English and Italian ship together for localized strings.
- Do not hardcode new user-facing CLI strings outside the i18n path when
  the surface is localized.
- Keep fallback behavior explicit and tested.
- Accessibility matters: translated strings must still be clear in plain
  terminal output.

## Crate stats

The block below is rewritten by `cvg docs regenerate` (ADR-0015) —
do not edit between the markers.

<!-- BEGIN AUTO:crate_stats -->
**`convergio-i18n` stats:** 4 `*.rs` files / 16 public items / 328 lines (under `src/`).

No files within 50 lines of the 300-line cap.
<!-- END AUTO -->
