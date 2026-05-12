# convergio-bus

Layer 2 of Convergio — persistent agent message bus.

## Status

**Implemented.** Topic-based publish, polling consumer with cursor,
explicit ack. Scoped per `plan_id`. Persistent via SQLite.

## API

| Op | Function |
|----|----------|
| Publish (plan-scoped) | `Bus::publish(NewMessage { plan_id, topic, sender, payload })` |
| Poll (plan-scoped) | `Bus::poll(plan_id, topic, cursor, limit)` |
| Poll, exclude self | `Bus::poll_filtered(plan_id, topic, cursor, limit, exclude_sender)` |
| Publish (system) | `Bus::publish_system(NewSystemMessage { topic, sender, payload })` |
| Poll (system) | `Bus::poll_system(topic, cursor, limit)` |
| Tail (inspection) | `Bus::tail(plan_id, topic, cursor, limit)` |
| Topics summary | `Bus::topics(plan_id)` |
| Last topic for agent | `Bus::last_topic_for_agent(agent_id)` |
| Ack | `Bus::ack(message_id, consumer)` |

`limit` is bounded at the crate boundary: non-positive values are
rejected with `BusError::InvalidLimit`, and values larger than the
crate-level `MAX_PAGE_LIMIT` (1000) are clamped before binding. The
HTTP layer additionally clamps to 1..=100.

HTTP surface (mounted by `convergio-server`):

| Method | Path |
|--------|------|
| `POST` | `/v1/plans/:plan_id/messages` |
| `GET`  | `/v1/plans/:plan_id/messages?topic=&cursor=&limit=&exclude_sender=` |
| `GET`  | `/v1/plans/:plan_id/messages/tail?topic=&cursor=&limit=` |
| `GET`  | `/v1/plans/:plan_id/topics` |
| `GET`  | `/v1/system-messages?topic=&cursor=&limit=` |
| `POST` | `/v1/system-messages` |
| `POST` | `/v1/messages/:id/ack` |

## Delivery semantics

- **At-least-once** — consumer must be idempotent.
- **Persistent** — messages survive consumer crash until acked.
- **Per-`(plan_id, topic)` FIFO** ordered by `seq`.

## What it is NOT

- Cross-plan broadcast. The narrow exception is the `system.*` topic
  family (ADR-0025): those messages persist with `plan_id IS NULL`
  and are written/read via `publish_system` / `poll_system`. Use this
  for presence and coordination signals (`agent.attached`,
  `agent.heartbeat`, `agent.idle`, `agent.detached`) that have no
  single plan home.
- Sub-millisecond throughput (Kafka territory).
- Content-aware routing — payload is opaque JSON.
