-- F2-5 (ADR-0038 §6): backfill repo + promote to NOT NULL.
--
-- Step 1 — idempotent backfill: legacy nodes get 'convergio' as repo.
-- Safe to re-run (WHERE clause is a no-op once all rows are non-NULL).
UPDATE graph_nodes SET repo = 'convergio' WHERE repo IS NULL;

-- Step 2 — promote to NOT NULL.
-- SQLite cannot add NOT NULL to an existing column via ALTER TABLE, so
-- we use the standard copy-and-rename pattern. graph_edges references
-- graph_nodes(id) with ON DELETE CASCADE; because PRAGMA foreign_keys
-- is OFF by default in SQLite, the DROP succeeds and the rename
-- immediately restores the referent name for any future FK checks.

CREATE TABLE graph_nodes_v2 (
    id           TEXT PRIMARY KEY,    -- stable hash of (repo, kind, crate, path, name, optional span)
    kind         TEXT NOT NULL,       -- crate | module | item | adr | doc
    name         TEXT NOT NULL,
    file_path    TEXT,                -- NULL for adr/doc-only nodes
    crate_name   TEXT NOT NULL,       -- '__docs__' for non-code nodes
    item_kind    TEXT,                -- struct | enum | fn | trait | impl | const | type | macro
    span_start   INTEGER,             -- byte offset, NULL for non-code
    span_end     INTEGER,
    last_parsed  TEXT NOT NULL,       -- ISO-8601 UTC timestamp of last parse
    source_mtime TEXT NOT NULL,       -- file mtime at parse time (for staleness check)
    repo         TEXT NOT NULL        -- owning repository (e.g. 'convergio')
);

INSERT INTO graph_nodes_v2
    SELECT id, kind, name, file_path, crate_name, item_kind,
           span_start, span_end, last_parsed, source_mtime, repo
    FROM graph_nodes;

DROP TABLE graph_nodes;
ALTER TABLE graph_nodes_v2 RENAME TO graph_nodes;

-- Restore indexes dropped with the old table.
CREATE INDEX IF NOT EXISTS idx_graph_nodes_file
    ON graph_nodes(file_path);

CREATE INDEX IF NOT EXISTS idx_graph_nodes_crate
    ON graph_nodes(crate_name, kind);

CREATE INDEX IF NOT EXISTS idx_graph_nodes_repo
    ON graph_nodes(repo);
