-- 0015_plan_objectives.sql — W4 / ADR-0055
-- Per-plan OKR objective. Single row per plan; key results follow in a
-- separate migration once the basic gate is in place.

CREATE TABLE IF NOT EXISTS plan_objectives (
    plan_id TEXT PRIMARY KEY
        REFERENCES plans(id) ON DELETE CASCADE,
    objective TEXT NOT NULL,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_plan_objectives_updated
    ON plan_objectives(updated_at);
