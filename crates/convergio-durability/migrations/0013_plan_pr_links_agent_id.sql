-- 0013_plan_pr_links_agent_id.sql
--
-- P2-3 (F47): add agent_id to plan_pr_links so every PR-to-plan
-- mapping records *which agent* opened the PR.
--
-- SQLite does not support ADD COLUMN with constraints beyond NOT NULL;
-- the column is nullable to stay backwards-compatible with rows written
-- before this migration.

ALTER TABLE plan_pr_links ADD COLUMN agent_id TEXT;

CREATE INDEX IF NOT EXISTS idx_plan_pr_links_agent ON plan_pr_links(agent_id);
