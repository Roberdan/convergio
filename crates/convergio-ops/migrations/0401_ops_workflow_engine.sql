-- 0401_ops_workflow_engine.sql
--
-- Ops workflow engine core tables (Ontology Platform W8: Workflow & Operations Engine).
--
-- Bitemporal posture (ADR-0053): every mutation is append-only.
-- We model bitemporality at the row level via:
--   valid_*  = "what was true in the world"
--   system_* = "what the system believed"
--
-- This migration intentionally stores workflow specs and instance state as JSON
-- blobs so we can iterate on the BPMN subset without blocking on a full
-- ontology/object store.

CREATE TABLE IF NOT EXISTS ops_workflows (
    row_id INTEGER PRIMARY KEY,
    workflow_id TEXT NOT NULL,
    workflow_key TEXT NOT NULL,
    version INTEGER NOT NULL,
    spec_json TEXT NOT NULL,
    valid_from TEXT NOT NULL,
    valid_to TEXT,
    system_from TEXT NOT NULL,
    system_to TEXT,
    created_by_agent TEXT,
    created_at TEXT NOT NULL
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_ops_workflows_current
ON ops_workflows(workflow_id)
WHERE system_to IS NULL;

CREATE INDEX IF NOT EXISTS idx_ops_workflows_key_current
ON ops_workflows(workflow_key, system_to);

CREATE INDEX IF NOT EXISTS idx_ops_workflows_bitemporal
ON ops_workflows(workflow_id, system_from, system_to, valid_from, valid_to);

CREATE TABLE IF NOT EXISTS ops_workflow_instances (
    row_id INTEGER PRIMARY KEY,
    instance_id TEXT NOT NULL,
    workflow_id TEXT NOT NULL,
    workflow_version INTEGER NOT NULL,
    status TEXT NOT NULL,
    state_json TEXT NOT NULL,
    valid_from TEXT NOT NULL,
    valid_to TEXT,
    system_from TEXT NOT NULL,
    system_to TEXT,
    created_by_agent TEXT,
    created_at TEXT NOT NULL
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_ops_instances_current
ON ops_workflow_instances(instance_id)
WHERE system_to IS NULL;

CREATE INDEX IF NOT EXISTS idx_ops_instances_by_workflow
ON ops_workflow_instances(workflow_id, system_to);

CREATE INDEX IF NOT EXISTS idx_ops_instances_bitemporal
ON ops_workflow_instances(instance_id, system_from, system_to, valid_from, valid_to);
