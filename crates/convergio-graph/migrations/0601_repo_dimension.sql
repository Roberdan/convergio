-- F2-5 (ADR-0038 §6): add repo dimension to graph_nodes.
--
-- Nullable so existing rows remain valid; backfilled to 'convergio'
-- and promoted to NOT NULL in the next migration (0602).

ALTER TABLE graph_nodes ADD COLUMN repo TEXT;
