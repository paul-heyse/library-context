-- The control schema template (cutover plan P1.6/P1.7). `{control}` names the schema; the CHECK
-- lists `{lifecycle}`, `{profiles}`, `{frontiers}`, `{outcomes}`, `{families}`, `{availabilities}`
-- and `{classes}` are rendered from the lifecycle, the model's enums and codebooks and the failure
-- classes, so the physical digest changes with them. Installed once per store; `store check` renders it
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
    identity bytea NOT NULL CHECK(octet_length(identity)=16),
    model_digest bytea NOT NULL CHECK(octet_length(model_digest)=32),
    physical_digest bytea NOT NULL CHECK(octet_length(physical_digest)=32)
);
CREATE TABLE {control}.generations (
    id bytea PRIMARY KEY CHECK(octet_length(id)=16),
    -- Every generation is advanced only by the attempt that registered it, which holds its
    -- attempt lock until it ends.
    state text NOT NULL CHECK(state IN ({lifecycle})),
    model_digest bytea NOT NULL CHECK(octet_length(model_digest)=32),
    physical_digest bytea NOT NULL CHECK(octet_length(physical_digest)=32),
    producer_digest bytea NOT NULL CHECK(octet_length(producer_digest)=32),
    schedule_digest bytea NOT NULL CHECK(octet_length(schedule_digest)=32),
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
    definition_digest bytea NOT NULL CHECK(octet_length(definition_digest)=32),
    model_digest bytea NOT NULL CHECK(octet_length(model_digest)=32),
    physical_digest bytea NOT NULL CHECK(octet_length(physical_digest)=32),
    binding_digest bytea NOT NULL CHECK(octet_length(binding_digest)=32),
    -- Replay data, not live read authority; explicit audit challenges this binding independently.
    proof_context jsonb NOT NULL,
    PRIMARY KEY(generation_id,binding_digest)
);
CREATE TABLE {control}.stage_receipts (
    generation_id bytea NOT NULL REFERENCES {control}.generations(id),
    stage_name text NOT NULL,
    relation_name text NOT NULL,
    schedule_digest bytea NOT NULL CHECK(octet_length(schedule_digest)=32),
    row_count bigint NOT NULL CHECK(row_count>=0),
    content_digest bytea NOT NULL CHECK(octet_length(content_digest)=32),
    PRIMARY KEY(generation_id,stage_name,relation_name)
);
CREATE TABLE {control}.publication_groups (
    generation_id bytea NOT NULL REFERENCES {control}.generations(id),
    -- Schedule-local contiguous prefix position; boundary codes never express close order.
    epoch smallint NOT NULL CHECK(epoch >= 0),
    boundary smallint NOT NULL CHECK(boundary IN ({boundaries})),
    schedule_digest bytea NOT NULL CHECK(octet_length(schedule_digest)=32),
    closed boolean NOT NULL DEFAULT false,
    PRIMARY KEY(generation_id,epoch),
    UNIQUE(generation_id,boundary)
);
CREATE TABLE {control}.publication_outputs (
    generation_id bytea NOT NULL,
    epoch smallint NOT NULL,
    stage_name text NOT NULL,
    relation_name text NOT NULL,
    sealed boolean NOT NULL DEFAULT false,
    row_count bigint CHECK(row_count>=0),
    content_digest bytea CHECK(octet_length(content_digest)=32),
    PRIMARY KEY(generation_id,stage_name,relation_name),
    FOREIGN KEY(generation_id,epoch) REFERENCES {control}.publication_groups(generation_id,epoch)
);
CREATE TABLE {control}.epoch_receipts (
    generation_id bytea NOT NULL,
    epoch smallint NOT NULL,
    relation_name text NOT NULL,
    row_count bigint NOT NULL CHECK(row_count>=0),
    content_digest bytea NOT NULL CHECK(octet_length(content_digest)=32),
    PRIMARY KEY(generation_id,epoch,relation_name),
    FOREIGN KEY(generation_id,epoch) REFERENCES {control}.publication_groups(generation_id,epoch)
);
-- The outputs an attempt's schedule declares; publication requires exactly these stage receipts.
CREATE TABLE {control}.stage_read_checks (
    generation_id bytea NOT NULL REFERENCES {control}.generations(id),
    consumer text NOT NULL,
    check_name text NOT NULL,
    binding_digest bytea NOT NULL CHECK(octet_length(binding_digest)=32),
    PRIMARY KEY(generation_id,consumer,check_name),
    FOREIGN KEY(generation_id,binding_digest) REFERENCES {control}.validation_receipts(generation_id,binding_digest)
);
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
-- An immutable validated prefix of a still-staging cumulative generation. This is private
-- attempt evidence; it does not publish any schema or change the selected generation.
CREATE TABLE {control}.checkpoints (
    generation_id bytea NOT NULL REFERENCES {control}.generations(id),
    frontier text NOT NULL CHECK(frontier IN ({frontiers})),
    contract_digest bytea NOT NULL CHECK(octet_length(contract_digest)=32),
    model_digest bytea NOT NULL CHECK(octet_length(model_digest)=32),
    schedule_digest bytea NOT NULL CHECK(octet_length(schedule_digest)=32),
    coverage_digest bytea NOT NULL CHECK(octet_length(coverage_digest)=32),
    content_digest bytea NOT NULL CHECK(octet_length(content_digest)=32),
    PRIMARY KEY(generation_id,frontier)
);
-- The facts admission a published facts generation carries; only admission constructs one.
CREATE TABLE {control}.admissions (
    generation_id bytea PRIMARY KEY REFERENCES {control}.generations(id),
    contract_digest bytea NOT NULL CHECK(octet_length(contract_digest)=32),
    model_digest bytea NOT NULL CHECK(octet_length(model_digest)=32),
    schedule_digest bytea NOT NULL CHECK(octet_length(schedule_digest)=32),
    coverage_digest bytea NOT NULL CHECK(octet_length(coverage_digest)=32),
    content_digest bytea NOT NULL CHECK(octet_length(content_digest)=32),
    profile text NOT NULL CHECK(profile IN ({profiles}))
);
-- Each fact family's availability in an admitted generation, by codebook code.
CREATE TABLE {control}.admission_families (
    generation_id bytea NOT NULL REFERENCES {control}.admissions(generation_id),
    family smallint NOT NULL CHECK(family IN ({families})),
    availability smallint NOT NULL CHECK(availability IN ({availabilities})),
    PRIMARY KEY(generation_id,family)
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
GRANT SELECT ON {control}.stage_receipts TO lctx_importer;
GRANT SELECT ON {control}.checkpoints TO lctx_importer;
GRANT SELECT ON {control}.stage_read_checks TO lctx_importer;
GRANT SELECT ON ALL TABLES IN SCHEMA {control} TO lctx_serving;

GRANT SELECT ON {control}.publication_groups,{control}.publication_outputs,{control}.epoch_receipts TO lctx_importer;
