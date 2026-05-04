---
id: 0023
status: proposed
date: 2026-05-01
topics: [observability, telemetry, logging, ops]
related_adrs: [0002, 0014, 0015]
touches_crates: [convergio-server, convergio-durability, convergio-cli, convergio-bus]
last_validated: 2026-05-01
---

# 0023. Observability tier — telemetry, structured logging, request correlation

- Status: proposed
- Date: 2026-05-01
- Deciders: Roberdan
- Tags: observability, telemetry, ops

## Context and Problem Statement

As of 2026-05-01 Convergio has zero metrics, no log rotation,
and no request-id correlation across HTTP → durability → audit →
bus. The daemon ships with `tracing` + `tracing-subscriber`
(json + env-filter) wired in `crates/convergio-server/src/main.rs`,
but the writer is plain stdout. When the daemon is started via
`cargo run -p convergio-server -- start` the logs go to the
terminal and disappear when the shell closes; only the macOS
launchd plist captures them to `~/.convergio/convergio.log` —
without rotation. systemd users get journald.

The single existing aspirational reference to metrics lives at
`crates/convergio-durability/src/reaper.rs:56`:

```rust
/// surface via metrics, not by silent loop death.
```

A comment, no implementation.

This is fine for single-user dogfooding. It is **not** fine for:

- the multi-agent runner adapters (ADR-0022 governance, future
  Wave 2 capability bundles) — without per-agent metrics and
  correlatable logs, an agent crashloop is invisible;
- the Tier-3 graph engine (ADR-0014) when build/refresh latency
  matters;
- any external operator (the future "First-5-users" cohort in
  v0.4) who will reasonably ask "is the daemon healthy?" and
  expects an answer beyond "well, the audit chain still verifies".

## Decision Drivers

- **No new always-on cost.** The local-first SLO is sub-100ms
  request latency. Anything we add must be optional or
  near-free in the off path.
- **Single binary still.** No external collector required to
  *run* Convergio. OTLP export is opt-in.
- **Filesystem rotation must work without a daemon manager.**
  Operators who run `cargo run` should still get rotated logs
  on disk, not just terminal echo.
- **Request-id propagates.** A single failed `submitted →
  done` transition must produce log lines from HTTP →
  gate-pipeline → audit row writer that all share one id.
- **No reinvention.** Use the `tracing` + `tracing-opentelemetry`
  ecosystem; do not write a custom telemetry layer.

## Considered Options

### Option A — File logging only (`tracing-appender`)

Drop `tracing-appender` next to the existing subscriber. Files
land at `~/.convergio/logs/convergio.{date}.log` with daily
rotation. **Pros**: ten-line patch, solves the disappearing-log
case for `cargo run` operators. **Cons**: still no metrics, no
trace correlation, no exporter — leaves the harder gaps open.

### Option B — Full OpenTelemetry stack, optional

Add a `otel` cargo feature on `convergio-server` (off by
default) that pulls in `tracing-opentelemetry` + `opentelemetry-otlp`.
When enabled and `OTEL_EXPORTER_OTLP_ENDPOINT` is set, traces
+ metrics + logs ship to any OTLP collector (Jaeger, Tempo,
Honeycomb, Grafana Cloud, the user's choice). When disabled
the dependency tree does not pull in OTel crates at all.

In parallel: always-on file rotation (Option A) for the local
case. Always-on request-id middleware (axum `TraceLayer` +
custom span field) for in-process correlation regardless of
whether OTel is on.

**Pros**: closes all three gaps (metrics, structured logs,
correlation), respects the local-first SLO via feature flag,
no exporter required in default install. **Cons**: bigger
diff (~200 lines across 3 crates + 1 new module), adds two
optional deps, adds one feature gate.

### Option C — Hand-rolled metrics (no OTel)

Expose a `/v1/metrics` endpoint with bespoke counters/histograms
written by hand. **Pros**: zero new deps. **Cons**: reinvents
the wheel; produces a non-standard surface that no operator
will already know how to scrape. Rejected.

## Decision Outcome

**Option B**, because it is the only one that closes all three
gaps (metrics + structured logs + correlation) without locking
operators into a specific backend or paying the cost when they
do not opt in.

### Implementation status (2026-05-04)

Stays `proposed`. Only the baseline is in place: `tracing-subscriber`
with `EnvFilter` (`CONVERGIO_LOG`) wired in
`crates/convergio-server/src/main.rs`, plus
`tower_http::trace::TraceLayer::new_for_http()` on the router. None of
the ADR's load-bearing pieces ship yet:

- no `tracing-appender` / `CONVERGIO_LOG_DIR` daily rotation,
- no `request_id` middleware injecting `X-Request-Id` / `req_id` span
  field,
- no `req_id` / `agent_id` / `task_id` span propagation through
  durability/bus,
- no `otel` cargo feature, no `tracing-opentelemetry` /
  `opentelemetry-otlp` exporter,
- no `convergio.reaper.tasks_reaped_total` /
  `convergio.watcher.processes_checked_total` counters,
- no `cvg doctor` `log_dir` / `otel` lines.

The reaper aspirational comment is still a comment. The ADR remains
the intended shape; flipping to `accepted` requires the file-rotation
+ request-id middleware to land at minimum.

### Concrete shape

1. **`tracing-appender`** added unconditionally to
   `convergio-server`. New env knob `CONVERGIO_LOG_DIR`
   (default `~/.convergio/logs/`). Daily rotation, retention
   30 days. Format: existing JSON layer.
2. **Request-id middleware** in
   `crates/convergio-server/src/middleware/request_id.rs`:
   - extracts `X-Request-Id` header if present, else generates
     a UUID v4;
   - injects it into the tracing span as field `req_id`;
   - echoes it in the response header for client correlation.
3. **Internal trace fields** (`req_id`, `agent_id`, `task_id`,
   `plan_id`) propagated as span attributes from route handlers
   into durability/bus calls via `tracing::Instrument`. No
   thread-local state.
4. **`otel` feature flag** on `convergio-server`:
   - off by default;
   - when on, `tracing-opentelemetry` exports the same span tree
     to OTLP at `OTEL_EXPORTER_OTLP_ENDPOINT`;
   - includes `opentelemetry-otlp` with `tonic` transport
     (gRPC) and `http-proto` as compile-time alternative.
5. **Reaper + Watcher metrics** (the existing aspirational
   comment in `reaper.rs`): emit
   `convergio.reaper.tasks_reaped_total` counter and
   `convergio.watcher.processes_checked_total` counter as
   `tracing::info!` with `metric=true` field. The OTel layer
   converts these to OTLP metrics; the file layer ignores them
   (they remain readable as JSON).

### What this decision does NOT do

- It does not add a Prometheus `/metrics` endpoint. If demand
  shows up, OTel-collector → Prometheus exporter handles it
  externally.
- It does not change the audit chain's role. The audit chain
  remains the **truth** record (hash-chained, replayable). Logs
  remain the **narrative** record (lossy, rotated). They are
  complementary, not substitutes (ADR-0002 still holds).
- It does not require log shipping for any v0.3 work. The OTel
  feature stays off until v0.4 First-5-users.

### Default operator experience

Without setting any env var, after this ADR ships:

```text
~/.convergio/
├── logs/
│   ├── convergio.2026-05-01.log      # JSON, rotated daily
│   ├── convergio.2026-04-30.log
│   └── ...
└── v3/state.db                       # unchanged
```

`cvg doctor` adds two new lines:

```text
log_dir: /Users/.../.convergio/logs (rotation: daily, retention: 30d)
otel:    disabled (set OTEL_EXPORTER_OTLP_ENDPOINT to enable)
```

## Consequences

### Positive

- The reaper aspirational comment becomes a real counter.
- A failed `submitted → done` transition produces correlatable
  log lines across HTTP, gate, audit. Debugging stops being
  archaeology.
- `cargo run` operators get rotated on-disk logs — closes the
  long-standing gap "where did the daemon log go?".
- The opt-in OTel path means First-5-users (v0.4) can plug
  Convergio into Honeycomb/Grafana without a daemon rebuild.

### Negative

- Bigger compile time when `otel` feature is on (~5–8s extra,
  measured on similar Rust workspaces). Mitigation: feature is
  off by default; CI builds both ways.
- One more env-var surface (`CONVERGIO_LOG_DIR`,
  `OTEL_EXPORTER_OTLP_ENDPOINT`). Documented in `cvg doctor`
  output.
- File-rotation introduces a tiny risk of disk-fill on busy
  agents — 30-day retention bounds it; configurable via
  `CONVERGIO_LOG_RETENTION_DAYS`.

### Neutral

- This ADR is orthogonal to ADR-0002 (audit chain). It does not
  change what is audited, only how the *narrative* surrounding
  audited events is captured.
- It is independent of ADR-0014 (graph) and ADR-0015 (auto-regen
  docs) but enables them to emit metrics on build/refresh
  duration without further design.

## Validation

This ADR is validated when:

1. The daemon, started via plain `cargo run -p convergio-server
   -- start`, writes JSON logs to `~/.convergio/logs/` with
   daily rotation; restart confirms a new file is opened.
2. A `submitted → refused` event for a single task produces log
   entries with the same `req_id` field across HTTP route,
   gate pipeline, and audit row writer.
3. With `OTEL_EXPORTER_OTLP_ENDPOINT=http://localhost:4317`
   pointing at any OTLP collector, span tree and metrics arrive
   in the collector for the same request.
4. `cvg doctor` reports `log_dir` + `otel` state honestly.
5. The reaper counter (`convergio.reaper.tasks_reaped_total`)
   increments by 1 per `task.reaped` audit row and matches the
   audit chain count over a rolling window.
