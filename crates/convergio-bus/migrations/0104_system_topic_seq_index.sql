-- Bus migration 0104: add (topic, seq) index for system-topic polling.
--
-- Audit finding F8 (low, refactor / optimization): migration 0103 added
-- `idx_agent_messages_system_topic` as a partial index on
-- `(topic, created_at) WHERE plan_id IS NULL`. The actual hot path on
-- that table is `Bus::poll_system`:
--
--   SELECT ...
--   FROM agent_messages
--   WHERE plan_id IS NULL AND topic = ? AND seq > ? AND consumed_at IS NULL
--   ORDER BY seq ASC LIMIT ?
--
-- which filters and orders by `seq`, not by `created_at`. SQLite cannot
-- use the existing index to satisfy the ORDER BY, so it falls back to
-- a sort step that grows with the size of the system-topic backlog.
--
-- This migration adds a partial index keyed on `(topic, seq)` that
-- matches the poll predicate and ordering exactly. The previous
-- `(topic, created_at)` index is kept because it still serves
-- chronological inspection queries (`tail`, time-range scans).
--
-- The migration is purely additive: no data is moved, no constraint
-- changes, no rebuild — safe on any existing v0.2.x state.db.

CREATE INDEX IF NOT EXISTS idx_agent_messages_system_topic_seq
    ON agent_messages (topic, seq)
    WHERE plan_id IS NULL;
