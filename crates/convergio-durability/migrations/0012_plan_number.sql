-- Add a monotonic plan number, scoped per project group.
-- Plans with the same project value share a 1-based sequence.
-- Plans with project IS NULL form their own shared sequence.
ALTER TABLE plans ADD COLUMN number INTEGER NOT NULL DEFAULT 0;

-- Backfill: assign 1-based numbers ordered by creation time within each
-- project group. SQLite does not support window functions in UPDATE, so
-- we use a correlated COUNT subquery instead.
UPDATE plans
SET number = (
    SELECT COUNT(*)
    FROM plans AS p2
    WHERE
        (p2.project = plans.project OR (p2.project IS NULL AND plans.project IS NULL))
        AND (
            p2.created_at < plans.created_at
            OR (p2.created_at = plans.created_at AND p2.id <= plans.id)
        )
);

-- Unique constraint: no two plans in the same non-NULL project share a number.
CREATE UNIQUE INDEX uq_plan_project_number
    ON plans (project, number)
    WHERE project IS NOT NULL;

-- Unique constraint for the NULL-project group (SQLite treats each NULL
-- value as distinct in standard UNIQUE indexes, so a partial index is
-- required to enforce uniqueness among NULL-project plans).
CREATE UNIQUE INDEX uq_plan_null_project_number
    ON plans (number)
    WHERE project IS NULL;
