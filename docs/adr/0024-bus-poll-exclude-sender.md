---
id: 0024
status: accepted
date: 2026-05-01
topics: [bus, layer-2, ergonomics]
related_adrs: [0001, 0023]
touches_crates: [convergio-bus, convergio-server, convergio-cli]
last_validated: 2026-05-03
---

# 0024. Bus poll filter: exclude_sender

- Status: accepted (shipped — `Bus::poll_filtered`, F53, PR #71)
- Date: 2026-05-01
- Deciders: Roberdan
- Tags: bus, ergonomics

## Context and Problem Statement

When two agents share a coordination topic on the bus, each one
sees its own published messages on `Bus::poll`. The 2026-05-01
dogfood session captured this as gap 7: "Bus poll_messages
includes one's own published messages, no for-agent-id filter."

Concrete pattern from that session: agent A publishes on
`coordination/agents`, then polls the same topic to see if agent
B replied. The poll returns A's own message at the top of the
list, the agent has to filter it client-side every single time.
Twice in the same session this caused confusion ("did B reply
yet?") because A's own message was treated as a peer signal.

The fix is small and additive.

## Decision Drivers

- **Backward compatibility.** Existing callers of `Bus::poll`
  must continue to work without any change.
- **System messages are not a sender.** Messages with
  `sender IS NULL` (system-emitted, e.g. ADR-0023's
  `system.*` topics in PR #62) are *not* a per-agent signal and
  must always be returned.
- **Server-side filter, not client-side.** A SQL `WHERE` filter
  is one query; client-side filtering pulls the noisy data
  across the wire and discards it. Server-side wins.

## Considered Options

### Option A — Skip; document the workaround

Tell agents to always filter `m.sender == my_id` after poll.
Costs: every agent re-implements the same filter, every agent
forgets at least once.

### Option B — Add `Bus::poll_for_recipient` keyed on
`payload.to_agent`

Match against an explicit field in the JSON payload. Costs:
forces every publisher to add `to_agent` to the payload schema,
breaks the bus contract for plain broadcast messages.

### Option C — Add `exclude_sender` filter on `Bus::poll`,
optional, additive (chosen)

New method `Bus::poll_filtered(plan_id, topic, cursor, limit,
exclude_sender)`. `Bus::poll` is now a thin wrapper that calls
`poll_filtered(..., None)` — backward compatible by
construction.

The SQL adds a single `AND (sender IS NULL OR sender != ?)`
clause when `exclude_sender = Some(id)`. System messages
(`sender NULL`) are always returned. No payload schema change.

## Decision Outcome

Chosen option: **Option C**, because it is the smallest change
that closes the dogfood gap without breaking the bus contract.

### Implementation

- `convergio-bus`: `Bus::poll_filtered(plan_id, topic, cursor,
  limit, exclude_sender: Option<&str>)` is the new primitive.
  `Bus::poll` calls it with `None`. 3 new tests in
  `tests/lifecycle.rs` cover the filter, the null-sender pass-
  through, and the `None`-equivalence with the legacy `poll`.
- `convergio-server`: `GET /v1/plans/:plan_id/messages` (poll
  route) gains an optional `?exclude_sender=<id>` query
  parameter that threads through to `poll_filtered`.
- `convergio-cli`: `cvg bus tail` gains `--exclude-sender <id>`
  for symmetric human-side ergonomics. (Tail filters
  client-side; the gain there is small but the CLI surface
  stays consistent.)

### What this decision does not do

- It does not add a "messages addressed to me" semantic. That
  would key on `payload.to_agent` and is a separate feature
  (call it ADR-0025 if it ever ships). This ADR is strictly
  *exclude my own*, not *select for me*.
- It does not change the existing `poll` shape, the payload
  schema, the audit chain, or the system-message convention
  (ADR-0023).

## Consequences

### Positive

- Closes dogfood gap 7 + the v0.2 plan task
  `596c6601-…` ("Bus poll_messages: filter out own published
  messages by agent_id").
- Server-side filter — no extra rows on the wire.
- One extra `Option` argument; one new wrapping method;
  every existing caller compiles unchanged.

### Negative

- Two methods now exist (`poll` and `poll_filtered`); a future
  contributor might not know which to reach for. Doc comment
  on `poll` points at `poll_filtered`; the README of the bus
  crate could mention this in a future doc pass.

### Neutral

- Does not solve the sibling concern of "I want only messages
  for me". That stays open for ADR-0025 (the ack/inbox
  semantics future ADR).

## Validation

- `cargo test -p convergio-bus`: **9 passed** (6 pre-existing
  + 3 new).
- The dogfood test: an agent that publishes on its own
  coordination topic and polls with `exclude_sender = my_id`
  sees zero of its own messages and every peer message.
