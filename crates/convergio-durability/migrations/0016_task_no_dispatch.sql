-- 0016_task_no_dispatch.sql
--
-- Tracker-only mode (plan A.2 of the multi-repo dispatch programme).
--
-- A task with `no_dispatch = 1` stays `pending` forever: the executor's
-- `find_dispatchable` filter refuses to pick it up. The operator (or
-- another agent in another repo) is expected to attach evidence and
-- drive the task through the normal `submitted → done` flow manually.
-- The intended use is mirroring work that ships from another
-- repository so it still shows up in the local plan dashboard, the
-- audit chain, and the validator's wave gates.
--
-- Plans get a parallel `no_dispatch_default` column so an operator
-- can flag a whole plan as tracker-only with one flag at creation
-- time. The API layer reads this column when a `POST /v1/plans/:id/tasks`
-- request omits an explicit `no_dispatch` value; explicit values on
-- the task body always win.

ALTER TABLE tasks ADD COLUMN no_dispatch INTEGER NOT NULL DEFAULT 0;
ALTER TABLE plans ADD COLUMN no_dispatch_default INTEGER NOT NULL DEFAULT 0;

CREATE INDEX IF NOT EXISTS idx_tasks_no_dispatch ON tasks(no_dispatch);
