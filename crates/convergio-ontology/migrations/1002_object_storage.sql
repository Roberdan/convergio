-- Ontology object storage tables.
-- Migration range 1000-1099 reserved for convergio-ontology (ADR-0003).
--
-- Note: Convergio is local-first; we model tenant scoping via `tenant_id`
-- (today: `plans.id` from convergio-durability).

CREATE TABLE IF NOT EXISTS object_instances (
    id          TEXT PRIMARY KEY,
    tenant_id   TEXT NOT NULL REFERENCES plans(id) ON DELETE CASCADE,
    type        TEXT NOT NULL,
    created_at  TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_object_instances_tenant_type
    ON object_instances(tenant_id, type);

-- Append-only edge log: links are asserted/retracted via `op` events.
CREATE TABLE IF NOT EXISTS object_links (
    id          TEXT PRIMARY KEY,
    tenant_id   TEXT NOT NULL REFERENCES plans(id) ON DELETE CASCADE,
    from_id     TEXT NOT NULL REFERENCES object_instances(id) ON DELETE CASCADE,
    to_id       TEXT NOT NULL REFERENCES object_instances(id) ON DELETE CASCADE,
    link_type   TEXT NOT NULL,
    op          TEXT NOT NULL CHECK (op IN ('add', 'remove')),
    created_at  TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_object_links_from_type
    ON object_links(from_id, link_type);

CREATE INDEX IF NOT EXISTS idx_object_links_to_type
    ON object_links(to_id, link_type);

CREATE TRIGGER IF NOT EXISTS trg_object_links_no_update
BEFORE UPDATE ON object_links
BEGIN
    SELECT RAISE(ABORT, 'object_links is append-only');
END;

CREATE TRIGGER IF NOT EXISTS trg_object_links_no_delete
BEFORE DELETE ON object_links
BEGIN
    SELECT RAISE(ABORT, 'object_links is append-only');
END;

-- Property event log (set/unset); used as the object storage layer.
CREATE TABLE IF NOT EXISTS object_properties (
    id            TEXT PRIMARY KEY,
    tenant_id     TEXT NOT NULL REFERENCES plans(id) ON DELETE CASCADE,
    object_id     TEXT NOT NULL REFERENCES object_instances(id) ON DELETE CASCADE,
    property_type TEXT NOT NULL,
    value_json    TEXT NOT NULL,
    op            TEXT NOT NULL CHECK (op IN ('set', 'unset')),
    created_at    TEXT NOT NULL
);
