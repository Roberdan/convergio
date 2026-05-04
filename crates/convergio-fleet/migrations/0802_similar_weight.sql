-- Add integer weight column (cosine × 1000) to fleet_similar_edges (ADR-0038, F2-8).
-- Default 0 fills pre-existing rows; every new upsert supplies the computed value.
ALTER TABLE fleet_similar_edges ADD COLUMN weight INTEGER NOT NULL DEFAULT 0;
