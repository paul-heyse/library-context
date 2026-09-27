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

CREATE TABLE lctx_ops.attempts (
    attempt_id bytea PRIMARY KEY CHECK (octet_length(attempt_id) = 16),
    compiler_digest bytea NOT NULL CHECK (octet_length(compiler_digest) = 32),
    library text NOT NULL CHECK (length(library) BETWEEN 1 AND 1024),
    store_path text NOT NULL,
    started_at timestamptz NOT NULL DEFAULT clock_timestamp()
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
CREATE TABLE lctx_ops.snapshots (
    store_path text NOT NULL,
    snapshot_id bytea NOT NULL CHECK (octet_length(snapshot_id) = 16),
    content_digest bytea NOT NULL CHECK (octet_length(content_digest) = 32),
    compiler_digest bytea NOT NULL CHECK (octet_length(compiler_digest) = 32),
    reconciled_at timestamptz NOT NULL DEFAULT clock_timestamp(),
    PRIMARY KEY (store_path, snapshot_id)
);
CREATE TABLE lctx_ops.generations (
    location text PRIMARY KEY,
    generation_key text NOT NULL,
    snapshot_id bytea NOT NULL CHECK (octet_length(snapshot_id) = 16),
    manifest_digest bytea NOT NULL CHECK (octet_length(manifest_digest) = 32),
    reconciled_at timestamptz NOT NULL DEFAULT clock_timestamp()
);
CREATE INDEX generations_snapshot ON lctx_ops.generations (snapshot_id, generation_key);

REVOKE ALL ON SCHEMA lctx_cache, lctx_ops FROM PUBLIC;
GRANT USAGE ON SCHEMA lctx_cache, lctx_ops TO lctx_app;
GRANT SELECT, INSERT ON ALL TABLES IN SCHEMA lctx_cache TO lctx_app;
GRANT SELECT, INSERT ON lctx_ops.attempts, lctx_ops.events TO lctx_app;
GRANT SELECT, INSERT, UPDATE ON lctx_ops.snapshots, lctx_ops.generations TO lctx_app;
GRANT SELECT ON public._sqlx_migrations TO lctx_app;
