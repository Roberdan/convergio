-- 0011_task_runner_fields.sql
--
-- ADR-0034: per-task runner kind / permission profile / budget.
--
-- The executor (Layer 4) needs to know, for each pending task:
--   * which vendor CLI to spawn      → `runner_kind`  (e.g. "claude:sonnet")
--   * which permission envelope      → `profile`      (e.g. "standard")
--   * an optional session budget cap → `max_budget_usd`
--
-- All three are nullable. When NULL, the executor falls back to the
-- daemon defaults (env vars CONVERGIO_RUNNER_DEFAULT,
-- CONVERGIO_PROFILE_DEFAULT). The planner / `cvg task create`
-- populates the columns so the routing decision is per-task.

ALTER TABLE tasks ADD COLUMN runner_kind    TEXT;     -- "claude:sonnet" / "copilot:gpt-5.2" / "qwen:qwen3-coder" / NULL = use daemon default
ALTER TABLE tasks ADD COLUMN profile        TEXT;     -- "standard" / "read_only" / "sandbox" / NULL = standard
ALTER TABLE tasks ADD COLUMN max_budget_usd REAL;     -- session cap forwarded to claude --max-budget-usd; NULL = no cap

CREATE INDEX IF NOT EXISTS idx_tasks_runner_kind ON tasks(runner_kind);
