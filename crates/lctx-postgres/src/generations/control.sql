-- The control schema template (cutover plan P1.6/P1.7). `{control}` names the schema; the CHECK
-- lists `{lifecycle}`, `{profiles}`, `{frontiers}`, `{outcomes}` and `{classes}` are rendered from
-- the lifecycle, the model enums and the failure classes, so the physical digest changes with them. Installed once per store; `store check` renders it
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
    state text NOT NULL CHECK(state IN ({lifecycle})),
    -- An attempt-owned generation is advanced only by its attempt, which holds its attempt lock.
    owned boolean NOT NULL,
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
-- The outputs an attempt's schedule declares; publication requires exactly these stage receipts.
CREATE TABLE {control}.planned_outputs (
    generation_id bytea NOT NULL REFERENCES {control}.generations(id),
    stage_name text NOT NULL,
    relation_name text NOT NULL,
    PRIMARY KEY(generation_id,stage_name,relation_name)
);
CREATE TABLE {control}.stage_outcomes (
    generation_id bytea NOT NULL REFERENCES {control}.generations(id),
    stage_name text NOT NULL,
    outcome smallint NOT NULL CHECK(outcome IN ({outcomes})),
    PRIMARY KEY(generation_id,stage_name)
);
-- The facts admission a published facts generation carries; only admission constructs one.
CREATE TABLE {control}.admissions (
    generation_id bytea PRIMARY KEY REFERENCES {control}.generations(id),
    contract_digest bytea NOT NULL CHECK(octet_length(contract_digest)=32),
    model_digest bytea NOT NULL CHECK(octet_length(model_digest)=32),
    schedule_digest bytea NOT NULL CHECK(octet_length(schedule_digest)=32),
    coverage_digest bytea NOT NULL CHECK(octet_length(coverage_digest)=32),
    content_digest bytea NOT NULL CHECK(octet_length(content_digest)=32),
    profile text NOT NULL CHECK(profile IN ({profiles})),
    availability text NOT NULL
);
-- Why an attempt failed and the state it failed from; a failed generation permits only abort.
CREATE TABLE {control}.failures (
    generation_id bytea PRIMARY KEY REFERENCES {control}.generations(id),
    from_state text NOT NULL CHECK(from_state IN ('staging','sealed','validated')),
    class text NOT NULL CHECK(class IN ({classes})),
    detail text NOT NULL CHECK(octet_length(detail) <= 4096)
);
CREATE TABLE {control}.selection (
    singleton boolean PRIMARY KEY CHECK(singleton),
    generation_id bytea REFERENCES {control}.generations(id)
);
INSERT INTO {control}.selection VALUES(true,NULL);
CREATE TABLE {control}.events (
    ordinal bigint GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    generation_id bytea NOT NULL REFERENCES {control}.generations(id),
    state text NOT NULL CHECK(state IN ({lifecycle})),
    at timestamptz NOT NULL DEFAULT now()
);
REVOKE ALL ON ALL TABLES IN SCHEMA {control} FROM PUBLIC;
GRANT USAGE ON SCHEMA {control} TO lctx_importer,lctx_serving;
GRANT SELECT ON {control}.generations TO lctx_importer;
GRANT SELECT ON {control}.installation TO lctx_importer;
GRANT SELECT ON ALL TABLES IN SCHEMA {control} TO lctx_serving;
