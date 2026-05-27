# convergio-a11y-axe

W11 local-stub axe-core wrapper for A11yGate phase 2 (ADR-0064).

## Responsibility

Wrap an **external** `axe` binary so that `A11yGate` can extend its
phase-1 built-in subset toward full WCAG coverage without taking on
either a Node.js dependency or the not-yet-built remote capability
registry (W9 follow-up).

## Boundaries

- **Opt-in only.** No work happens unless `CONVERGIO_A11Y_AXE_BIN` is
  set to an absolute path to a runnable binary.
- **No implicit shell-out.** If the env var is missing or the path
  does not point at a file, `run_html` returns `AxeStatus::NotConfigured`
  and the caller falls back to phase-1 checks.
- **No HTML parsing here.** We hand the HTML to the binary on stdin
  and parse a JSON report from stdout. The binary owns axe-core.

## Invariants

- Public API never panics on missing binary, malformed output, or I/O
  errors — every failure is a typed `AxeStatus` variant.
- The crate has zero `unwrap()` / `expect()` outside tests.
- The crate has no `convergio-*` dependencies (it must stay leaf so
  the durability crate can adopt it later without cycles).

## Tests

```bash
cargo test -p convergio-a11y-axe
```
