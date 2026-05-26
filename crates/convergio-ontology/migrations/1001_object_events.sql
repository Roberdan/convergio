-- convergio-ontology (range 1000-1099, ADR-0003)
-- W3: bitemporal object event log + current views (ADR-0053).

CREATE TABLE IF NOT EXISTS object_events (
    object_id  TEXT NOT NULL,
    op         TEXT NOT NULL,
    payload    TEXT NOT NULL, -- canonical JSON string

    valid_from TEXT NOT NULL,
    valid_to   TEXT,

    tx_from    TEXT NOT NULL,
    tx_to      TEXT,

    CHECK (tx_to IS NULL OR tx_to > tx_from),
    CHECK (valid_to IS NULL OR valid_to > valid_from)
);

-- Default read paths.
CREATE INDEX IF NOT EXISTS idx_object_events_object_tx ON object_events (object_id, tx_to);
CREATE INDEX IF NOT EXISTS idx_object_events_object_valid ON object_events (object_id, valid_to);

-- At most one open system-time row per object.
CREATE UNIQUE INDEX IF NOT EXISTS idx_object_events_one_open_tx
    ON object_events (object_id)
    WHERE tx_to IS NULL;

-- Transaction-current (what the system currently believes).
CREATE VIEW IF NOT EXISTS object_events_tx_current AS
    SELECT object_id, op, payload, valid_from, valid_to, tx_from, tx_to
    FROM object_events
    WHERE tx_to IS NULL;

-- Fully-current (transaction-current AND open-ended valid-time).
CREATE VIEW IF NOT EXISTS object_events_current AS
    SELECT object_id, op, payload, valid_from, valid_to, tx_from, tx_to
    FROM object_events
    WHERE tx_to IS NULL AND valid_to IS NULL;

CREATE VIEW IF NOT EXISTS objects_tx_current AS
    SELECT object_id, op, payload, valid_from, valid_to, tx_from
    FROM object_events_tx_current;

CREATE VIEW IF NOT EXISTS objects_current AS
    SELECT object_id, op, payload, valid_from, tx_from
    FROM object_events_current;
