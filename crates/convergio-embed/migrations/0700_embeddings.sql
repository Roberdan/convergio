-- ADR-0038 § 5.2.3 — embeddings storage + sqlite-vec index.
-- Migration range 700-799 reserved by ADR-0003 for convergio-embed.

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

CREATE INDEX IF NOT EXISTS idx_graph_node_embeddings_model
    ON graph_node_embeddings(model);

CREATE INDEX IF NOT EXISTS idx_graph_node_embeddings_source_hash
    ON graph_node_embeddings(source_hash);

-- sqlite-vec virtual table (loaded as an auto-extension by convergio-db::Pool).
CREATE VIRTUAL TABLE IF NOT EXISTS graph_vec_index USING vec0(
    embedding float[384]
);
