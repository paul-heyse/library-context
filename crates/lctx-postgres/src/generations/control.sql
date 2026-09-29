-- The control schema template (cutover plan P1.6). `{control}` names the schema; the CHECK lists
-- `{states}`, `{profiles}` and `{frontiers}` are rendered from the lifecycle and the model enums,
-- so the physical digest changes with them. Installed once per store; `store check` renders it
-- into a shadow schema and compares the two catalogs.
CREATE SCHEMA {control};
REVOKE ALL ON SCHEMA {control} FROM PUBLIC;
-- Binary array framing plus uncompressed element lengths, used by generated admission checks.
CREATE FUNCTION {control}.text_array_wire_bytes(value text[]) RETURNS bigint
LANGUAGE sql IMMUTABLE PARALLEL SAFE AS $$
    SELECT 20::bigint + COALESCE(sum(4::bigint + COALESCE(octet_length(element), 0)), 0)::bigint
    FROM unnest(value) AS element
$$;
CREATE TABLE {control}.installation (
    singleton boolean PRIMARY KEY CHECK(singleton),
    model_digest bytea NOT NULL CHECK(octet_length(model_digest)=32),
    physical_digest bytea NOT NULL CHECK(octet_length(physical_digest)=32)
);
CREATE TABLE {control}.generations (
    id bytea PRIMARY KEY CHECK(octet_length(id)=16),
    state text NOT NULL CHECK(state IN ({states})),
    model_digest bytea NOT NULL CHECK(octet_length(model_digest)=32),
    physical_digest bytea NOT NULL CHECK(octet_length(physical_digest)=32),
    producer_digest bytea NOT NULL CHECK(octet_length(producer_digest)=32),
    schedule_digest bytea CHECK(octet_length(schedule_digest)=32),
    content_digest bytea CHECK(octet_length(content_digest)=32),
    profile text NOT NULL CHECK(profile IN ({profiles})),
    frontier text NOT NULL CHECK(frontier IN ({frontiers})),
    created_at timestamptz NOT NULL DEFAULT now(),
    CHECK(state NOT IN ('validated','published') OR content_digest IS NOT NULL)
);
CREATE TABLE {control}.receipts (
    generation_id bytea NOT NULL REFERENCES {control}.generations(id),
    relation_name text NOT NULL,
    row_count bigint NOT NULL CHECK(row_count>=0),
    content_digest bytea NOT NULL CHECK(octet_length(content_digest)=32),
    PRIMARY KEY(generation_id,relation_name)
);
CREATE TABLE {control}.validation_receipts (
    generation_id bytea NOT NULL REFERENCES {control}.generations(id),
    validator_name text NOT NULL,
    content_digest bytea NOT NULL CHECK(octet_length(content_digest)=32),
    model_digest bytea NOT NULL CHECK(octet_length(model_digest)=32),
    physical_digest bytea NOT NULL CHECK(octet_length(physical_digest)=32),
    PRIMARY KEY(generation_id,validator_name)
);
CREATE TABLE {control}.stage_receipts (
    generation_id bytea NOT NULL REFERENCES {control}.generations(id),
    stage_name text NOT NULL,
    relation_name text NOT NULL,
    schedule_digest bytea NOT NULL CHECK(octet_length(schedule_digest)=32),
    PRIMARY KEY(generation_id,stage_name,relation_name)
);
CREATE TABLE {control}.selection (
    singleton boolean PRIMARY KEY CHECK(singleton),
    generation_id bytea REFERENCES {control}.generations(id)
);
INSERT INTO {control}.selection VALUES(true,NULL);
CREATE TABLE {control}.events (
    ordinal bigint GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    generation_id bytea NOT NULL REFERENCES {control}.generations(id),
    state text NOT NULL CHECK(state IN ({states})),
    at timestamptz NOT NULL DEFAULT now()
);
REVOKE ALL ON ALL TABLES IN SCHEMA {control} FROM PUBLIC;
GRANT USAGE ON SCHEMA {control} TO lctx_importer,lctx_serving;
GRANT SELECT ON {control}.generations TO lctx_importer,lctx_serving;
GRANT SELECT ON {control}.installation TO lctx_importer,lctx_serving;
