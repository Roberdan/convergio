-- Per-ADR/Doc embedding-alignment snapshots (ADR-0038, F3-6).
--
-- One row per (repo, node_id, model). `snapshot_score` is the
-- average cosine between the doc's embedding and the embeddings of
-- every code node it `claims` or `mentions` at snapshot time. The
-- `find_doc_drift` query compares this against a freshly-computed
-- current score; a negative delta ≥ the configured threshold flags
-- drift.

CREATE TABLE IF NOT EXISTS fleet_doc_snapshots (
    repo            TEXT NOT NULL,
    node_id         TEXT NOT NULL,
    model           TEXT NOT NULL,
    snapshot_score  REAL NOT NULL,
    linked_count    INTEGER NOT NULL DEFAULT 0,
    snapshot_at     TEXT NOT NULL,
    PRIMARY KEY (repo, node_id, model)
);

CREATE INDEX IF NOT EXISTS idx_fleet_doc_snapshots_model
    ON fleet_doc_snapshots(model);
