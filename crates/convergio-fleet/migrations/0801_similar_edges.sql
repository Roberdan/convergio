-- Cross-repo similarity edges (ADR-0038, F2-7).
-- Each row records a cosine-similarity relationship between two nodes
-- from different repos, discovered during `POST /v1/fleet/build --refresh-similarity`.
--
-- kind: 'similar_to'  => score >= 0.85
--       'duplicates'  => score >= 0.95
CREATE TABLE IF NOT EXISTS fleet_similar_edges (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    repo_a      TEXT    NOT NULL,
    node_id_a   TEXT    NOT NULL,
    repo_b      TEXT    NOT NULL,
    node_id_b   TEXT    NOT NULL,
    score       REAL    NOT NULL,
    kind        TEXT    NOT NULL CHECK(kind IN ('similar_to', 'duplicates')),
    built_at    TEXT    NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))
);

-- Ensures upsert on (repo_a, node_id_a, repo_b, node_id_b) is unique.
CREATE UNIQUE INDEX IF NOT EXISTS fleet_similar_edges_pair_uq
    ON fleet_similar_edges(repo_a, node_id_a, repo_b, node_id_b);
