-- ADR-0038 § 5.2.3 — fleet tables.
-- Migration range 800-899 reserved by ADR-0003 for convergio-fleet.
-- F2 scope: fleet_repos, fleet_plans, fleet_plan_repos.

CREATE TABLE IF NOT EXISTS fleet_repos (
    name           TEXT    PRIMARY KEY,
    path           TEXT    NOT NULL,
    language       TEXT    NOT NULL,   -- e.g. "rust", "typescript", "python"
    parser         TEXT    NOT NULL,   -- e.g. "syn", "tree-sitter"
    role           TEXT    NOT NULL DEFAULT 'downstream',  -- engine|library|downstream|sandbox
    derives_from   TEXT,               -- parent repo name (optional)
    last_built_at  TEXT,
    enabled        INTEGER NOT NULL DEFAULT 1
);

CREATE INDEX IF NOT EXISTS idx_fleet_repos_enabled
    ON fleet_repos(enabled);

CREATE TABLE IF NOT EXISTS fleet_plans (
    id          TEXT PRIMARY KEY,       -- UUID v4
    title       TEXT NOT NULL,
    scope       TEXT NOT NULL,          -- "fleet" | repo name
    created_at  TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS fleet_plan_repos (
    fleet_plan_id   TEXT NOT NULL REFERENCES fleet_plans(id),
    repo            TEXT NOT NULL REFERENCES fleet_repos(name),
    repo_plan_id    TEXT NOT NULL,      -- per-repo plan id in convergio-durability
    PRIMARY KEY (fleet_plan_id, repo)
);

CREATE INDEX IF NOT EXISTS idx_fleet_plan_repos_fleet_plan_id
    ON fleet_plan_repos(fleet_plan_id);
