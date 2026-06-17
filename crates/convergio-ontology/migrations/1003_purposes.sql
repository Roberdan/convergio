-- Purpose registry (ADR-0054 §B).
-- Migration range 1000-1099 reserved for convergio-ontology (ADR-0003).
--
-- A purpose is a free-form, immutable declaration of WHY data may be
-- read or written. It is the upstream primitive every regulated vertical
-- needs (GDPR Art. 5(1)(b) purpose limitation, FERPA-equivalent, consent
-- registries). Registered once via `cvg purpose register` and immutable
-- thereafter: triggers refuse UPDATE and DELETE so the declared intent
-- cannot be silently rewritten.

CREATE TABLE IF NOT EXISTS purposes (
    id                TEXT PRIMARY KEY,
    label             TEXT NOT NULL UNIQUE,
    description       TEXT NOT NULL DEFAULT '',
    declared_by_plan  TEXT,
    effective_from    TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_purposes_label ON purposes(label);

CREATE TRIGGER IF NOT EXISTS trg_purposes_no_update
BEFORE UPDATE ON purposes
BEGIN
    SELECT RAISE(ABORT, 'purposes are immutable');
END;

CREATE TRIGGER IF NOT EXISTS trg_purposes_no_delete
BEFORE DELETE ON purposes
BEGIN
    SELECT RAISE(ABORT, 'purposes are immutable');
END;
