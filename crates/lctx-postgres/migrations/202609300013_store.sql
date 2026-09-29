-- The canonical store kernel (DESIGN §15.11, ADR-0083; cutover plan §3.2 and §4.1 D3–D5).
--
-- lctx_store holds the static kernel: the installed relation catalog, the generation registry and
-- the lifecycle functions. The canonical relations themselves live in schema lctx, which is
-- generated from the relation declarations and installed by `lctx store install` (never by a
-- migration). Each generation's partitions live in their own schema, lctx_g<32 hex>.
--
-- Roles: lctx_migrator owns everything and runs the SECURITY DEFINER functions; lctx_importer
-- (the writer) creates, loads, validates and publishes generations through those functions only;
-- lctx_serving (the reader) reads published relations through their parents.

CREATE SCHEMA lctx_store;
REVOKE ALL ON SCHEMA lctx_store FROM PUBLIC;
GRANT USAGE ON SCHEMA lctx_store TO lctx_importer, lctx_serving;

-- One current install of the generated canonical schema.
CREATE TABLE lctx_store.ddl_installs (
    ddl_digest bytea PRIMARY KEY CHECK (octet_length(ddl_digest) = 32),
    installed_at timestamptz NOT NULL DEFAULT now(),
    is_current boolean NOT NULL
);
CREATE UNIQUE INDEX ddl_installs_current ON lctx_store.ddl_installs (is_current) WHERE is_current;

-- The installed relation catalog: generated templates the lifecycle functions substitute only
-- {schema} and {generation} into. Written by install, in dependency order.
CREATE TABLE lctx_store.relations (
    name text PRIMARY KEY,
    ordinal integer NOT NULL UNIQUE CHECK (ordinal >= 0),
    partition_column text NOT NULL,
    copy_columns text[] NOT NULL,
    create_staging text NOT NULL,
    create_indexes text[] NOT NULL,
    attach text NOT NULL,
    detach text NOT NULL
);

CREATE TABLE lctx_store.generations (
    generation_id bytea PRIMARY KEY CHECK (octet_length(generation_id) = 16),
    library text NOT NULL,
    profile text NOT NULL,
    state text NOT NULL CHECK (state IN ('staging', 'validated', 'published', 'failed', 'retired')),
    ddl_digest bytea NOT NULL CHECK (octet_length(ddl_digest) = 32),
    compiler_digest bytea CHECK (octet_length(compiler_digest) = 32),
    producer_digest bytea CHECK (octet_length(producer_digest) = 32),
    content_digest bytea CHECK (octet_length(content_digest) = 32),
    receipts jsonb NOT NULL DEFAULT '{}'::jsonb,
    created_at timestamptz NOT NULL DEFAULT now(),
    validated_at timestamptz,
    published_at timestamptz,
    retired_at timestamptz
);

CREATE TABLE lctx_store.generation_relations (
    generation_id bytea NOT NULL REFERENCES lctx_store.generations ON DELETE CASCADE,
    relation text NOT NULL,
    row_count bigint NOT NULL CHECK (row_count >= 0),
    schema_digest bytea NOT NULL CHECK (octet_length(schema_digest) = 32),
    PRIMARY KEY (generation_id, relation)
);

CREATE TABLE lctx_store.generation_events (
    generation_id bytea NOT NULL REFERENCES lctx_store.generations ON DELETE CASCADE,
    ordinal bigint GENERATED ALWAYS AS IDENTITY,
    at timestamptz NOT NULL DEFAULT now(),
    event text NOT NULL,
    detail jsonb NOT NULL DEFAULT '{}'::jsonb,
    PRIMARY KEY (generation_id, ordinal)
);

CREATE TABLE lctx_store.selections (
    library text PRIMARY KEY,
    generation_id bytea NOT NULL REFERENCES lctx_store.generations,
    selected_at timestamptz NOT NULL DEFAULT now()
);

GRANT SELECT ON lctx_store.ddl_installs, lctx_store.relations, lctx_store.generations,
    lctx_store.generation_relations, lctx_store.generation_events, lctx_store.selections
    TO lctx_importer, lctx_serving;

-- The generation's schema name.
CREATE FUNCTION lctx_store.generation_schema(g bytea) RETURNS text
LANGUAGE sql IMMUTABLE STRICT
SET search_path = pg_catalog
AS $$ SELECT 'lctx_g' || encode(g, 'hex') $$;

-- Substitute a catalog template. Only the schema identifier and the id's hex digits are inserted.
CREATE FUNCTION lctx_store.expand(template text, g bytea) RETURNS text
LANGUAGE sql IMMUTABLE STRICT
SET search_path = pg_catalog
AS $$
    SELECT replace(replace(template, '{schema}', quote_ident(lctx_store.generation_schema(g))),
                   '{generation}', encode(g, 'hex'))
$$;

CREATE FUNCTION lctx_store.event(g bytea, name text, detail jsonb DEFAULT '{}'::jsonb) RETURNS void
LANGUAGE sql
SET search_path = pg_catalog
AS $$ INSERT INTO lctx_store.generation_events (generation_id, event, detail) VALUES (g, name, detail) $$;

-- Lock one generation's lifecycle for the transaction.
CREATE FUNCTION lctx_store.lock(g bytea) RETURNS void
LANGUAGE sql
SET search_path = pg_catalog
AS $$ SELECT pg_advisory_xact_lock(hashtextextended(encode(g, 'hex'), 0)) $$;

-- Create a generation: its registry row, its schema and one staging table per installed
-- relation. A retry is a new generation: an existing id is refused.
CREATE FUNCTION lctx_store.create_generation(g bytea, library text, profile text, digest bytea)
RETURNS void
LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog
AS $$
DECLARE
    s text := lctx_store.generation_schema(g);
    r record;
BEGIN
    PERFORM lctx_store.lock(g);
    IF NOT EXISTS (SELECT FROM lctx_store.ddl_installs WHERE is_current AND ddl_digest = digest) THEN
        RAISE EXCEPTION 'installed canonical schema differs from this binary (ddl digest)'
            USING ERRCODE = 'LX001';
    END IF;
    IF EXISTS (SELECT FROM lctx_store.generations WHERE generation_id = g) THEN
        RAISE EXCEPTION 'generation already exists; a retry is a new generation' USING ERRCODE = 'LX002';
    END IF;
    INSERT INTO lctx_store.generations (generation_id, library, profile, state, ddl_digest)
    VALUES (g, library, profile, 'staging', digest);
    EXECUTE format('CREATE SCHEMA %I', s);
    EXECUTE format('REVOKE ALL ON SCHEMA %I FROM PUBLIC', s);
    EXECUTE format('GRANT USAGE ON SCHEMA %I TO lctx_importer', s);
    FOR r IN SELECT * FROM lctx_store.relations ORDER BY ordinal LOOP
        EXECUTE lctx_store.expand(r.create_staging, g);
        EXECUTE format('GRANT SELECT, INSERT ON %I.%I TO lctx_importer', s, r.name);
    END LOOP;
    PERFORM lctx_store.event(g, 'created');
END
$$;

-- Freeze, count-check, index and analyze one loaded relation and record its receipt. One call
-- per relation keeps every statement within the writer's timeouts. A key violation raises here;
-- the caller then fails the generation.
CREATE FUNCTION lctx_store.validate_relation(g bytea, relation text, row_count bigint, schema_digest bytea)
RETURNS void
LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog
AS $$
DECLARE
    s text := lctx_store.generation_schema(g);
    r record;
    counted bigint;
    statement text;
BEGIN
    PERFORM lctx_store.lock(g);
    IF NOT EXISTS (SELECT FROM lctx_store.generations WHERE generation_id = g AND state = 'staging') THEN
        RAISE EXCEPTION 'generation is not staging' USING ERRCODE = 'LX003';
    END IF;
    SELECT * INTO r FROM lctx_store.relations WHERE name = validate_relation.relation;
    IF NOT FOUND THEN
        RAISE EXCEPTION 'relation % is not installed', validate_relation.relation USING ERRCODE = 'LX004';
    END IF;
    IF EXISTS (SELECT FROM lctx_store.generation_relations AS gr
               WHERE gr.generation_id = g AND gr.relation = r.name) THEN
        RAISE EXCEPTION 'relation % is already validated', r.name USING ERRCODE = 'LX004';
    END IF;
    EXECUTE format('REVOKE INSERT ON %I.%I FROM lctx_importer', s, r.name);
    EXECUTE format('SELECT count(*) FROM %I.%I', s, r.name) INTO counted;
    IF counted <> validate_relation.row_count THEN
        RAISE EXCEPTION 'relation % holds % rows, % were copied', r.name, counted, validate_relation.row_count
            USING ERRCODE = 'LX005';
    END IF;
    FOREACH statement IN ARRAY r.create_indexes LOOP
        EXECUTE lctx_store.expand(statement, g);
    END LOOP;
    EXECUTE format('ANALYZE %I.%I', s, r.name);
    INSERT INTO lctx_store.generation_relations (generation_id, relation, row_count, schema_digest)
    VALUES (g, r.name, validate_relation.row_count, validate_relation.schema_digest);
END
$$;

-- Mark a generation validated once every installed relation carries its receipt, recording the
-- digests the attempt computed. Called only after the DataFusion semantic validators passed.
CREATE FUNCTION lctx_store.mark_validated(g bytea, compiler bytea, producer bytea, content bytea, receipts jsonb)
RETURNS void
LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog
AS $$
BEGIN
    PERFORM lctx_store.lock(g);
    IF NOT EXISTS (SELECT FROM lctx_store.generations WHERE generation_id = g AND state = 'staging') THEN
        RAISE EXCEPTION 'generation is not staging' USING ERRCODE = 'LX003';
    END IF;
    IF (SELECT count(*) FROM lctx_store.generation_relations AS gr WHERE gr.generation_id = g)
       <> (SELECT count(*) FROM lctx_store.relations) THEN
        RAISE EXCEPTION 'not every installed relation is validated' USING ERRCODE = 'LX004';
    END IF;
    UPDATE lctx_store.generations
    SET state = 'validated', validated_at = now(), compiler_digest = compiler,
        producer_digest = producer, content_digest = content, receipts = mark_validated.receipts
    WHERE generation_id = g;
    PERFORM lctx_store.event(g, 'validated');
END
$$;

-- Publish: attach every partition, in dependency order, in this one transaction. References are
-- validated by the attach; writer access is revoked and the reader may scan each partition.
CREATE FUNCTION lctx_store.publish_generation(g bytea) RETURNS void
LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog
AS $$
DECLARE
    s text := lctx_store.generation_schema(g);
    r record;
BEGIN
    PERFORM lctx_store.lock(g);
    IF NOT EXISTS (SELECT FROM lctx_store.generations WHERE generation_id = g AND state = 'validated') THEN
        RAISE EXCEPTION 'generation is not validated' USING ERRCODE = 'LX003';
    END IF;
    FOR r IN SELECT * FROM lctx_store.relations ORDER BY ordinal LOOP
        EXECUTE lctx_store.expand(r.attach, g);
        EXECUTE format('REVOKE ALL ON %I.%I FROM lctx_importer', s, r.name);
        -- A pinned reader scans exactly this generation's partition.
        EXECUTE format('GRANT SELECT ON %I.%I TO lctx_serving', s, r.name);
    END LOOP;
    EXECUTE format('REVOKE USAGE ON SCHEMA %I FROM lctx_importer', s);
    EXECUTE format('GRANT USAGE ON SCHEMA %I TO lctx_serving', s);
    UPDATE lctx_store.generations SET state = 'published', published_at = now() WHERE generation_id = g;
    PERFORM lctx_store.event(g, 'published');
END
$$;

-- Fail a staging or validated generation. Its schema stays for inspection until retirement.
CREATE FUNCTION lctx_store.fail_generation(g bytea, reason text) RETURNS void
LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog
AS $$
BEGIN
    PERFORM lctx_store.lock(g);
    UPDATE lctx_store.generations SET state = 'failed'
    WHERE generation_id = g AND state IN ('staging', 'validated');
    IF NOT FOUND THEN
        RAISE EXCEPTION 'only a staging or validated generation can fail' USING ERRCODE = 'LX003';
    END IF;
    PERFORM lctx_store.event(g, 'failed', jsonb_build_object('reason', reason));
END
$$;

-- Select a published generation for its library.
CREATE FUNCTION lctx_store.select_generation(g bytea) RETURNS void
LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog
AS $$
DECLARE
    lib text;
BEGIN
    SELECT library INTO lib FROM lctx_store.generations WHERE generation_id = g AND state = 'published';
    IF lib IS NULL THEN
        RAISE EXCEPTION 'only a published generation can be selected' USING ERRCODE = 'LX003';
    END IF;
    INSERT INTO lctx_store.selections (library, generation_id) VALUES (lib, g)
    ON CONFLICT (library) DO UPDATE SET generation_id = excluded.generation_id, selected_at = now();
    PERFORM lctx_store.event(g, 'selected');
END
$$;

REVOKE ALL ON FUNCTION lctx_store.event(bytea, text, jsonb), lctx_store.lock(bytea),
    lctx_store.expand(text, bytea) FROM PUBLIC;
REVOKE ALL ON FUNCTION lctx_store.create_generation(bytea, text, text, bytea),
    lctx_store.validate_relation(bytea, text, bigint, bytea),
    lctx_store.mark_validated(bytea, bytea, bytea, bytea, jsonb),
    lctx_store.publish_generation(bytea), lctx_store.fail_generation(bytea, text),
    lctx_store.select_generation(bytea) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION lctx_store.create_generation(bytea, text, text, bytea),
    lctx_store.validate_relation(bytea, text, bigint, bytea),
    lctx_store.mark_validated(bytea, bytea, bytea, bytea, jsonb),
    lctx_store.publish_generation(bytea), lctx_store.fail_generation(bytea, text),
    lctx_store.select_generation(bytea) TO lctx_importer;
