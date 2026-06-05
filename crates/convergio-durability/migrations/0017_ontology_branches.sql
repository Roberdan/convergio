-- Ontology branches (W4: scenario branching / workshop overlay).
--
-- CoW semantics:
-- - mainline writes go to `ontology_entries`
-- - branch writes go to `ontology_branch_entries` (overlay)
-- - branch reads merge overlay over mainline
--
-- Lifecycle: draft -> review -> merged|discarded

CREATE TABLE IF NOT EXISTS ontology_branches (
    id            TEXT PRIMARY KEY,  -- UUID v4
    name          TEXT NOT NULL UNIQUE,
    status        TEXT NOT NULL,      -- draft|review|merged|discarded
    created_at    TEXT NOT NULL,
    updated_at    TEXT NOT NULL,
    reviewed_at   TEXT,
    merged_at     TEXT,
    discarded_at  TEXT
);

CREATE INDEX IF NOT EXISTS idx_ontology_branches_status
    ON ontology_branches(status);

CREATE TABLE IF NOT EXISTS ontology_entries (
    key         TEXT PRIMARY KEY,
    value       TEXT NOT NULL,
    updated_at  TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS ontology_branch_entries (
    branch_id   TEXT NOT NULL REFERENCES ontology_branches(id) ON DELETE CASCADE,
    key         TEXT NOT NULL,
    op_kind     TEXT NOT NULL,   -- set|delete
    value       TEXT,
    updated_at  TEXT NOT NULL,
    PRIMARY KEY (branch_id, key)
);

CREATE INDEX IF NOT EXISTS idx_ontology_branch_entries_branch
    ON ontology_branch_entries(branch_id);
