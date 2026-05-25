-- ADR-0053. Ontology runtime core — initial schema.
--
-- Migration range 1000-1099 reserved for convergio-ontology (ADR-0003).
--
-- The registry persists three peer concepts of the Modulor tuple:
--   - object_type   : a typed thing the user can talk about
--   - link_type     : a typed relation between object types
--   - property_type : a typed attribute of an object or link
--
-- All three carry an immutable `content_hash` (sha256 over the
-- canonical-JSON serialization of the row's semantic fields) and an
-- `schema_version` that monotonically increases inside a single
-- (kind, name) pair. A change that flips the `breaking` flag must
-- bump the major part of the consumer-facing schema_version per
-- ADR-0053 §"Versioning"; the daemon enforces the bump server-side
-- (W1 task 3 / 4) so neither CLI nor MCP can land a silent breaking
-- change.
--
-- Append-on-write: we never UPDATE an existing version row; new
-- versions land as new rows so the audit chain can attribute every
-- semantic change to a specific (audit_seq, content_hash) pair.

CREATE TABLE IF NOT EXISTS ontology_object_types (
    name            TEXT    NOT NULL,
    schema_version  INTEGER NOT NULL,
    content_hash    TEXT    NOT NULL,           -- sha256 of canonical JSON, lowercase hex
    breaking        INTEGER NOT NULL DEFAULT 0, -- 0 = additive, 1 = breaking
    title           TEXT    NOT NULL,
    description     TEXT    NOT NULL DEFAULT '',
    body_json       TEXT    NOT NULL,           -- canonical JSON of the semantic body
    created_at      TEXT    NOT NULL,           -- ISO-8601 UTC
    audit_seq       INTEGER,                    -- audit_log row that introduced this version
    PRIMARY KEY (name, schema_version)
);

CREATE INDEX IF NOT EXISTS idx_ontology_object_types_hash
    ON ontology_object_types(content_hash);

CREATE TABLE IF NOT EXISTS ontology_link_types (
    name            TEXT    NOT NULL,
    schema_version  INTEGER NOT NULL,
    content_hash    TEXT    NOT NULL,
    breaking        INTEGER NOT NULL DEFAULT 0,
    title           TEXT    NOT NULL,
    description     TEXT    NOT NULL DEFAULT '',
    from_object     TEXT    NOT NULL,           -- object_type.name
    to_object       TEXT    NOT NULL,
    body_json       TEXT    NOT NULL,
    created_at      TEXT    NOT NULL,
    audit_seq       INTEGER,
    PRIMARY KEY (name, schema_version)
);

CREATE INDEX IF NOT EXISTS idx_ontology_link_types_hash
    ON ontology_link_types(content_hash);

CREATE INDEX IF NOT EXISTS idx_ontology_link_types_from
    ON ontology_link_types(from_object);

CREATE TABLE IF NOT EXISTS ontology_property_types (
    name            TEXT    NOT NULL,
    schema_version  INTEGER NOT NULL,
    content_hash    TEXT    NOT NULL,
    breaking        INTEGER NOT NULL DEFAULT 0,
    title           TEXT    NOT NULL,
    description     TEXT    NOT NULL DEFAULT '',
    owner_kind      TEXT    NOT NULL,           -- 'object' | 'link'
    owner_name      TEXT    NOT NULL,
    datatype        TEXT    NOT NULL,           -- 'string' | 'integer' | 'boolean' | 'iri' | 'datetime' | ...
    required        INTEGER NOT NULL DEFAULT 0,
    body_json       TEXT    NOT NULL,
    created_at      TEXT    NOT NULL,
    audit_seq       INTEGER,
    PRIMARY KEY (name, schema_version)
);

CREATE INDEX IF NOT EXISTS idx_ontology_property_types_hash
    ON ontology_property_types(content_hash);

CREATE INDEX IF NOT EXISTS idx_ontology_property_types_owner
    ON ontology_property_types(owner_kind, owner_name);
