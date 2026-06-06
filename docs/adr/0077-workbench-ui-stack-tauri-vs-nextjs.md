---
id: 0064
status: proposed
date: 2026-05-26
topics: [workbench, ui, accelerator, distribution]
related_adrs: [0001, 0004, 0016, 0059]
touches_crates: []
last_validated: 2026-05-26
---

# 0064. Choose the Convergio Workbench UI stack (Tauri-first)

- Status: proposed
- Date: 2026-05-26
- Deciders: Roberto, Convergio agents
- Tags: workbench, ui, accelerator, distribution

## Context and Problem Statement

The `convergio-workbench` is a *vertical accelerator* (not core Convergio) that
provides a graphical “workbench” experience on top of the existing local daemon.

We need to decide whether that workbench should be primarily:

- a **desktop application** (Tauri / Electron / Flutter / …), or
- a **web application** (Next.js / other SPA) that is either hosted or run via a
  local server.

This decision must respect Convergio’s non-negotiables, especially **P2 security
first, local first** (single-user, offline-capable, `127.0.0.1`-only) and the
product posture “not a hosted platform” (see `docs/vision.md`).

## Decision Drivers

- **P2 local-first**: the workbench must function without a network and must not
  require a hosted control plane.
- **Minimize new always-on attack surface**: avoid introducing a new listening
  server just to render UI; prefer “no new ports”.
- **Separation of concerns**: the daemon stays the durable core; the workbench is
  a client (same posture as `cvg`), not a new server inside the daemon.
- **Cross-platform packaging**: macOS/Windows/Linux distribution should be
  feasible for a small team.
- **Performance / resource budget**: Convergio targets low overhead on a dev
  laptop (see local-first SLO posture in ADR-0023).
- **Accessibility + i18n readiness**: UI stack must support accessible
  components and localization from day one (Constitution P3/P5).

## Considered Options

1. **Tauri desktop app (recommended)** — local desktop shell with a web frontend
   rendered via OS WebView; Rust sidecar for local glue.
2. **Next.js (hosted web app)** — deploy a web UI on a remote server.
3. **Next.js (local web app)** — run a local Next.js server or ship an embedded
   local server to serve the UI.
4. **Electron** — desktop shell with bundled Chromium + Node.
5. **Flutter desktop** — native-ish UI toolkit with its own rendering.
6. **Stay TUI-only** — treat all “workbench” needs as out-of-scope; only the TUI
   grows (ADR-0059 is explicitly read-only and non-graphical).

## Decision Outcome

Chosen option: **1 — Tauri desktop app**, because it best matches **local-first**
without pushing Convergio toward a hosted platform or a daemon-embedded web UI.

### What “Tauri-first” means (scope)

- The workbench is a **separate client application**.
- It talks to the daemon via the **existing local HTTP API** on
  `http://127.0.0.1:8420` (same boundary as the CLI).
- The UI is shipped as **static assets** embedded in the app (no requirement for
  a local web server for rendering).
- If a future requirement needs “web-served workbench”, that is a **new product
  posture** and must be decided via a follow-up ADR (it changes the threat model
  and distribution model).

### Positive consequences

- **Best fit for local-first**: offline-capable by default; “app runs even if the
  network is down” is not an extra mode.
- **No new server requirement**: avoids opening additional ports merely to serve
  HTML.
- **Smaller resource footprint than Electron** for a comparable web UI.
- **Rust-native glue layer**: can reuse Rust idioms for local integration (while
  still keeping the daemon as the authority).
- **Clear product boundary**: core Convergio remains a local daemon + CLI/TUI;
  the workbench is a vertical accelerator (consistent with ADR-0059’s “no new
  graphical frontend inside core”).

### Negative consequences

- **Packaging complexity**: codesigning/notarization on macOS, installer
  workflows, and OS-specific quirks are real work.
- **WebView variability**: rendering and browser APIs differ across OS WebViews;
  some UI issues may be platform-specific.
- **Still a web UI**: the a11y/i18n discipline must be enforced in the frontend
  (the stack does not guarantee it).

## Pros and Cons of the Options (brief)

### 1) Tauri desktop app

- Good:
  - Aligns with P2 local-first with minimal additional surface.
  - Lets us ship an app without bundling a full Chromium runtime.
- Bad:
  - Desktop release engineering is non-trivial.

### 2) Next.js (hosted web app)

- Good:
  - Familiar deployment model; strong ecosystem.
- Bad:
  - Conflicts with the “not a hosted platform” posture.
  - Changes the threat model (accounts, remote auth, multi-tenant concerns).

### 3) Next.js (local web app)

- Good:
  - Can be “local-first” if everything runs locally.
- Bad:
  - Typically implies a local server process (extra complexity + extra surface).
  - Tends to become “just add another endpoint” pressure inside the daemon.

### 4) Electron

- Good:
  - Very mature; consistent rendering across platforms.
- Bad:
  - High baseline CPU/RAM footprint; heavier update surface.

### 5) Flutter desktop

- Good:
  - UI consistency and performance; strong component model.
- Bad:
  - Less alignment with the existing web-based ecosystem for agent tooling.
  - Adds a parallel UI toolchain and skillset.

### 6) Stay TUI-only

- Good:
  - Preserves minimalism; zero new distribution surface.
- Bad:
  - Does not satisfy the “workbench accelerator” goal; ADR-0059 explicitly
    limits the TUI to read-only inspection.

## Links

- Constitution: `CONSTITUTION.md` — P2 Security first, local first
- Vision: `docs/vision.md` — “Not a hosted platform. Local-first, single-user”
- Related: ADR-0059 (TUI Ontology Inspector) — explicitly punts a graphical
  workbench to a vertical accelerator
