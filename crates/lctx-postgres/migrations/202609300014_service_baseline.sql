-- The service baseline (semantic-model cutover plan P1.5, T11). It holds only the retained
-- services: the embedding cache and the operational attempt history. Generation schemas are
-- installed by the generation store from the typed model, never by migrations. A database whose
-- migration history predates this baseline is refused, not upgraded.
CREATE SCHEMA lctx_cache;
CREATE SCHEMA lctx_ops;

CREATE TABLE lctx_cache.specs (
    spec_hash bytea PRIMARY KEY CHECK (octet_length(spec_hash) = 32),
    canonical_spec text NOT NULL,
    dimensions integer NOT NULL CHECK (dimensions > 0 AND dimensions <= 65536)
);
CREATE TABLE lctx_cache.embedding_values (
    spec_hash bytea NOT NULL REFERENCES lctx_cache.specs(spec_hash),
    input_hash bytea NOT NULL CHECK (octet_length(input_hash) = 32),
    codec smallint NOT NULL CHECK (codec = 1),
    dimensions integer NOT NULL CHECK (dimensions > 0 AND dimensions <= 65536),
    vector_bytes bytea NOT NULL CHECK (octet_length(vector_bytes) = dimensions * 4),
    value_digest bytea NOT NULL CHECK (octet_length(value_digest) = 32),
    admitted_tokens integer NOT NULL CHECK (admitted_tokens >= 0),
    PRIMARY KEY (spec_hash, input_hash)
);

-- Recovered attempts carry unknown historical fields explicitly: observed_at is not a
-- fabricated start time.
CREATE TABLE lctx_ops.attempts (
    attempt_id bytea PRIMARY KEY CHECK (octet_length(attempt_id) = 16),
    compiler_digest bytea NOT NULL CHECK (octet_length(compiler_digest) = 32),
    library text CHECK (length(library) BETWEEN 1 AND 1024),
    store_path text NOT NULL,
    started_at timestamptz DEFAULT clock_timestamp(),
    registration text NOT NULL DEFAULT 'started' CHECK (registration IN ('started', 'reconciled')),
    observed_at timestamptz NOT NULL DEFAULT clock_timestamp(),
    CONSTRAINT attempt_provenance CHECK (
        (registration = 'started' AND library IS NOT NULL AND started_at IS NOT NULL)
        OR (registration = 'reconciled' AND library IS NULL AND started_at IS NULL))
);
CREATE TABLE lctx_ops.events (
    attempt_id bytea NOT NULL REFERENCES lctx_ops.attempts(attempt_id),
    event_key text NOT NULL CHECK (length(event_key) BETWEEN 1 AND 256),
    kind text NOT NULL CHECK (kind IN ('started', 'stage', 'published', 'generated', 'failed', 'interrupted')),
    detail text NOT NULL CHECK (octet_length(detail) <= 4096),
    recorded_at timestamptz NOT NULL DEFAULT clock_timestamp(),
    PRIMARY KEY (attempt_id, event_key)
);
CREATE INDEX events_order ON lctx_ops.events (attempt_id, recorded_at, event_key);

REVOKE ALL ON SCHEMA lctx_cache, lctx_ops FROM PUBLIC;
GRANT USAGE ON SCHEMA lctx_cache, lctx_ops TO lctx_app;
GRANT SELECT, INSERT ON ALL TABLES IN SCHEMA lctx_cache TO lctx_app;
GRANT SELECT, INSERT ON lctx_ops.attempts, lctx_ops.events TO lctx_app;
GRANT SELECT ON public._sqlx_migrations TO lctx_app;
