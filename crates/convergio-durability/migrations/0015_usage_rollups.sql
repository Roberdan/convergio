-- 0015_usage_rollups.sql
--
-- Evidence kind `usage` rollups: accumulate token + cost usage at the
-- task level, and cache plan + agent totals for dashboards.
--
-- SQLite cannot drop columns; we keep these additive.

ALTER TABLE tasks  ADD COLUMN tokens   INTEGER NOT NULL DEFAULT 0;
ALTER TABLE tasks  ADD COLUMN cost_usd REAL    NOT NULL DEFAULT 0;

ALTER TABLE plans  ADD COLUMN tokens   INTEGER NOT NULL DEFAULT 0;
ALTER TABLE plans  ADD COLUMN cost_usd REAL    NOT NULL DEFAULT 0;

ALTER TABLE agents ADD COLUMN tokens   INTEGER NOT NULL DEFAULT 0;
ALTER TABLE agents ADD COLUMN cost_usd REAL    NOT NULL DEFAULT 0;
