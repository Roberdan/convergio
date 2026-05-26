-- 0016_task_taxonomy.sql — W10 / ADR-0063
-- Closed taxonomy of task kinds used by the eval framework
-- (Cost-of-Pass). Populated once at migration time; the daemon
-- writes through it as a foreign-key reference, never adds rows
-- at runtime. Adding a new kind = new migration.

CREATE TABLE IF NOT EXISTS task_taxonomy (
    kind TEXT PRIMARY KEY
);

INSERT OR IGNORE INTO task_taxonomy (kind) VALUES
    ('generate-test'),
    ('review-code'),
    ('write-docs'),
    ('refactor'),
    ('plan'),
    ('summarise'),
    ('generic');

-- Skeleton evaluation outcome ledger. One row per Thor pipeline
-- terminal verdict; aggregated by the (still-to-come) view
-- `model_evaluations`. We accept a thin shape now so that as soon
-- as Smart Thor (W3) emits pipeline.run audit rows with usage
-- numbers, the eval recorder can start populating this table
-- without another migration.
CREATE TABLE IF NOT EXISTS eval_outcomes (
    id TEXT PRIMARY KEY,
    task_id TEXT NOT NULL,
    plan_id TEXT NOT NULL,
    runner_kind TEXT NOT NULL,         -- e.g. "copilot:gpt-5.2"
    taxonomy_kind TEXT NOT NULL REFERENCES task_taxonomy(kind),
    passed INTEGER NOT NULL,           -- 1 = pipeline_passed, 0 = pipeline_refused
    cost_usd REAL,                     -- nullable until Thor populates
    latency_ms INTEGER,                -- nullable until Thor populates
    recorded_at INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_eval_outcomes_runner_kind
    ON eval_outcomes(runner_kind, taxonomy_kind);
CREATE INDEX IF NOT EXISTS idx_eval_outcomes_recorded_at
    ON eval_outcomes(recorded_at);
