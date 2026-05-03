-- ADR-0035 § 5.2.3 — graph_node_embeddings table.
-- Migration range 700-799 reserved by ADR-0003 for convergio-embed.
-- F1-α scope: storage table only. The sqlite-vec virtual table
-- (graph_vec_index) lands in F1-β alongside the extension load
-- path; this lets us validate the storage and policy seam without
-- pulling in the platform-specific extension binary in F1-α.

CREATE TABLE IF NOT EXISTS graph_node_embeddings (
    repo         TEXT    NOT NULL,
    node_id      TEXT    NOT NULL,
    model        TEXT    NOT NULL,
    dim          INTEGER NOT NULL,
    vec          BLOB    NOT NULL,
    embedded_at  TEXT    NOT NULL,
    source_hash  TEXT    NOT NULL,
    PRIMARY KEY (repo, node_id, model)
);

CREATE INDEX IF NOT EXISTS idx_graph_node_embeddings_repo
    ON graph_node_embeddings(repo);

CREATE INDEX IF NOT EXISTS idx_graph_node_embeddings_source_hash
    ON graph_node_embeddings(source_hash);
