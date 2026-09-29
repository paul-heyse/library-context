CREATE SCHEMA IF NOT EXISTS lctx_model_store;
REVOKE ALL ON SCHEMA lctx_model_store FROM PUBLIC;
-- Binary array framing plus uncompressed element lengths, used by generated admission checks.
CREATE OR REPLACE FUNCTION lctx_model_store.text_array_wire_bytes(value text[]) RETURNS bigint
LANGUAGE sql IMMUTABLE PARALLEL SAFE AS $$
    SELECT 20::bigint + COALESCE(sum(4::bigint + COALESCE(octet_length(element), 0)), 0)::bigint
    FROM unnest(value) AS element
$$;
CREATE TABLE IF NOT EXISTS lctx_model_store.installation (
    singleton boolean PRIMARY KEY CHECK(singleton),
    model_digest bytea NOT NULL CHECK(octet_length(model_digest)=32),
    physical_digest bytea NOT NULL CHECK(octet_length(physical_digest)=32)
);
CREATE TABLE IF NOT EXISTS lctx_model_store.generations (
    id bytea PRIMARY KEY CHECK(octet_length(id)=16),
    state text NOT NULL CHECK(state IN ('staging','sealed','validated','published','failed','retired')),
    model_digest bytea NOT NULL CHECK(octet_length(model_digest)=32),
    physical_digest bytea NOT NULL CHECK(octet_length(physical_digest)=32),
    producer_digest bytea NOT NULL CHECK(octet_length(producer_digest)=32),
    content_digest bytea CHECK(octet_length(content_digest)=32),
    profile text NOT NULL CHECK(profile IN ('catalog','behavioral')),
    frontier text NOT NULL CHECK(frontier IN ('facts','normalized','analysis','serving')),
    created_at timestamptz NOT NULL DEFAULT now(),
    CHECK(state NOT IN ('validated','published','retired') OR content_digest IS NOT NULL)
);
CREATE TABLE IF NOT EXISTS lctx_model_store.receipts (
    generation_id bytea NOT NULL REFERENCES lctx_model_store.generations(id),
    relation_name text NOT NULL,
    row_count bigint NOT NULL CHECK(row_count>=0),
    content_digest bytea NOT NULL CHECK(octet_length(content_digest)=32),
    PRIMARY KEY(generation_id,relation_name)
);
CREATE TABLE IF NOT EXISTS lctx_model_store.validation_receipts (
    generation_id bytea NOT NULL REFERENCES lctx_model_store.generations(id),
    validator_name text NOT NULL,
    content_digest bytea NOT NULL CHECK(octet_length(content_digest)=32),
    model_digest bytea NOT NULL CHECK(octet_length(model_digest)=32),
    physical_digest bytea NOT NULL CHECK(octet_length(physical_digest)=32),
    PRIMARY KEY(generation_id,validator_name)
);
CREATE TABLE IF NOT EXISTS lctx_model_store.selection (
    singleton boolean PRIMARY KEY CHECK(singleton),
    generation_id bytea REFERENCES lctx_model_store.generations(id)
);
INSERT INTO lctx_model_store.selection VALUES(true,NULL) ON CONFLICT DO NOTHING;
CREATE TABLE IF NOT EXISTS lctx_model_store.events (
    ordinal bigint GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    generation_id bytea NOT NULL REFERENCES lctx_model_store.generations(id),
    state text NOT NULL,
    at timestamptz NOT NULL DEFAULT now()
);
REVOKE ALL ON ALL TABLES IN SCHEMA lctx_model_store FROM PUBLIC;
GRANT USAGE ON SCHEMA lctx_model_store TO lctx_importer,lctx_serving;
GRANT SELECT ON lctx_model_store.generations TO lctx_importer,lctx_serving;
