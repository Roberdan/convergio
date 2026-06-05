-- LLM gateway response cache (W11: LLM Gateway MVP)
-- Keyed by (prompt_hash, model_id, retrieval_set_hash).

CREATE TABLE IF NOT EXISTS llm_gateway_cache (
    prompt_hash         TEXT NOT NULL,
    model_id            TEXT NOT NULL,
    retrieval_set_hash  TEXT NOT NULL,
    provider_id         TEXT NOT NULL,
    response_json       TEXT NOT NULL,
    input_tokens        INTEGER,
    output_tokens       INTEGER,
    created_at          TEXT NOT NULL,
    PRIMARY KEY (prompt_hash, model_id, retrieval_set_hash)
);

CREATE INDEX IF NOT EXISTS idx_llm_gateway_cache_created_at
    ON llm_gateway_cache(created_at);
