-- PostgreSQL projection authority is separate from canonical Delta and mutable operations.
DO $$ BEGIN
 IF current_setting('server_version_num')::int / 10000 <> 18 OR NOT EXISTS
 (SELECT FROM pg_extension e JOIN pg_namespace n ON n.oid=e.extnamespace
  WHERE e.extname='vector' AND e.extversion='0.8.6' AND n.nspname='lctx_ext') THEN
  RAISE EXCEPTION 'PG18 with vector 0.8.6 in lctx_ext is required';
 END IF;
END $$;
CREATE SCHEMA lctx_serving;
REVOKE ALL ON SCHEMA lctx_serving FROM PUBLIC;
GRANT USAGE ON SCHEMA lctx_serving TO lctx_importer,lctx_serving;
GRANT SELECT ON public._sqlx_migrations TO lctx_importer,lctx_serving;

CREATE TABLE lctx_serving.generations (
 generation_digest bytea PRIMARY KEY CHECK(octet_length(generation_digest)=32),
 canonical_manifest text NOT NULL CHECK(octet_length(canonical_manifest)<=1048576),
 manifest jsonb GENERATED ALWAYS AS (canonical_manifest::jsonb) STORED,
 state text NOT NULL DEFAULT 'loading' CHECK(state IN ('loading','validating','ready','failed')),
 created_at timestamptz NOT NULL DEFAULT clock_timestamp(),
 CHECK(sha256(convert_to(canonical_manifest,'UTF8'))=generation_digest),
 CHECK((canonical_manifest::jsonb->>'format')::int=1),
 CHECK((canonical_manifest::jsonb->>'bundle_format')::int=11),
 CHECK((canonical_manifest::jsonb->>'dimensions')::int IN (0,1024))
);
ALTER TABLE lctx_serving.generations ENABLE ROW LEVEL SECURITY;
CREATE POLICY ready_read ON lctx_serving.generations FOR SELECT TO lctx_serving USING(state='ready');
CREATE POLICY importer ON lctx_serving.generations TO lctx_importer USING(true) WITH CHECK(state='loading');
GRANT SELECT,INSERT ON lctx_serving.generations TO lctx_importer;
GRANT SELECT ON lctx_serving.generations TO lctx_serving;

CREATE FUNCTION lctx_serving.is_ready(id bytea) RETURNS boolean LANGUAGE sql STABLE
 SET search_path=pg_catalog AS $$ SELECT EXISTS(SELECT FROM lctx_serving.generations WHERE generation_digest=id AND state='ready') $$;
REVOKE ALL ON FUNCTION lctx_serving.is_ready(bytea) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION lctx_serving.is_ready(bytea) TO lctx_serving,lctx_importer;

-- Every mutation locks the generation until transaction end. Freezing takes an exclusive row
-- lock and waits for all admitted writers; validating rejects new writes and ready is terminal.
CREATE FUNCTION lctx_serving.require_loading() RETURNS trigger LANGUAGE plpgsql SECURITY DEFINER
 SET search_path=pg_catalog AS $$
DECLARE id bytea; phase text;
BEGIN
 id := CASE WHEN TG_OP='DELETE' THEN OLD.generation_digest ELSE NEW.generation_digest END;
 SELECT state INTO phase FROM lctx_serving.generations WHERE generation_digest=id FOR SHARE;
 IF phase IS DISTINCT FROM 'loading' THEN RAISE EXCEPTION 'generation is frozen' USING ERRCODE='55000'; END IF;
 IF TG_OP='UPDATE' AND OLD.generation_digest<>NEW.generation_digest THEN RAISE EXCEPTION 'generation identity is immutable' USING ERRCODE='55000'; END IF;
 RETURN CASE WHEN TG_OP='DELETE' THEN OLD ELSE NEW END;
END $$;
REVOKE ALL ON FUNCTION lctx_serving.require_loading() FROM PUBLIC;

CREATE FUNCTION lctx_serving.guard_generation() RETURNS trigger LANGUAGE plpgsql
 SET search_path=pg_catalog AS $$
BEGIN
 IF TG_OP='DELETE' OR NEW.generation_digest<>OLD.generation_digest OR NEW.canonical_manifest<>OLD.canonical_manifest OR NEW.created_at<>OLD.created_at OR
 NOT ((OLD.state='loading' AND NEW.state IN ('validating','failed')) OR (OLD.state='validating' AND NEW.state IN ('ready','failed'))) THEN
  RAISE EXCEPTION 'invalid immutable generation transition' USING ERRCODE='55000';
 END IF;
 RETURN NEW;
END $$;
CREATE TRIGGER immutable_generation BEFORE UPDATE OR DELETE ON lctx_serving.generations FOR EACH ROW EXECUTE FUNCTION lctx_serving.guard_generation();
REVOKE ALL ON FUNCTION lctx_serving.guard_generation() FROM PUBLIC;

-- A narrow writer capability; importer cannot UPDATE metadata or bypass the transition guard.
CREATE FUNCTION lctx_serving.freeze_generation(id bytea) RETURNS void LANGUAGE plpgsql SECURITY DEFINER
 SET search_path=pg_catalog AS $$
BEGIN
 UPDATE lctx_serving.generations SET state='validating' WHERE generation_digest=id AND state='loading';
 IF NOT FOUND THEN RAISE EXCEPTION 'generation is not loading' USING ERRCODE='55000'; END IF;
END $$;
REVOKE ALL ON FUNCTION lctx_serving.freeze_generation(bytea) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION lctx_serving.freeze_generation(bytea) TO lctx_importer;

CREATE TABLE lctx_serving.artifacts (
 generation_digest bytea NOT NULL REFERENCES lctx_serving.generations,
 name text NOT NULL CHECK(name<>'' AND name !~ '/|\.\.'),
 content_digest bytea NOT NULL CHECK(octet_length(content_digest)=32),
 bytes bigint NOT NULL CHECK(bytes BETWEEN 0 AND 134217728),
 format integer NOT NULL CHECK(format>0),
 PRIMARY KEY(generation_digest,name)
);
ALTER TABLE lctx_serving.artifacts ENABLE ROW LEVEL SECURITY;
CREATE POLICY ready_read ON lctx_serving.artifacts FOR SELECT TO lctx_serving USING(lctx_serving.is_ready(generation_digest));
CREATE POLICY import_read ON lctx_serving.artifacts FOR SELECT TO lctx_importer USING(true);
CREATE POLICY import_insert ON lctx_serving.artifacts FOR INSERT TO lctx_importer WITH CHECK(true);
CREATE TRIGGER frozen_projection BEFORE INSERT OR UPDATE OR DELETE ON lctx_serving.artifacts FOR EACH ROW EXECUTE FUNCTION lctx_serving.require_loading();
GRANT SELECT,INSERT ON lctx_serving.artifacts TO lctx_importer;
GRANT SELECT ON lctx_serving.artifacts TO lctx_serving;

-- Location/selection and retrieval qualification are operational, outside generation identity.
CREATE TABLE lctx_serving.artifact_locations (
 generation_digest bytea NOT NULL, name text NOT NULL, location text NOT NULL,
 PRIMARY KEY(generation_digest,name,location),
 FOREIGN KEY(generation_digest,name) REFERENCES lctx_serving.artifacts
);
CREATE TABLE lctx_serving.retrieval_profiles (
 generation_digest bytea NOT NULL REFERENCES lctx_serving.generations,
 profile_digest bytea NOT NULL CHECK(octet_length(profile_digest)=32),
 policy jsonb NOT NULL CHECK(policy->>'route' IN ('exact','hnsw')),
 qualification jsonb NOT NULL,
 PRIMARY KEY(generation_digest,profile_digest)
);
CREATE TABLE lctx_serving.selections (
 library text PRIMARY KEY,
 generation_digest bytea NOT NULL REFERENCES lctx_serving.generations,
 profile_digest bytea,
 FOREIGN KEY(generation_digest,profile_digest) REFERENCES lctx_serving.retrieval_profiles
);
-- PG12/14 own publication/index/selection APIs. PG11 grants no premature promotion capability.
GRANT SELECT ON lctx_serving.artifact_locations,lctx_serving.retrieval_profiles,lctx_serving.selections TO lctx_serving;
REVOKE ALL ON ALL FUNCTIONS IN SCHEMA lctx_serving FROM PUBLIC;

ALTER TABLE lctx_serving.artifact_locations ENABLE ROW LEVEL SECURITY;
CREATE POLICY ready_read ON lctx_serving.artifact_locations FOR SELECT TO lctx_serving USING(lctx_serving.is_ready(generation_digest));
ALTER TABLE lctx_serving.retrieval_profiles ENABLE ROW LEVEL SECURITY;
CREATE POLICY ready_read ON lctx_serving.retrieval_profiles FOR SELECT TO lctx_serving USING(lctx_serving.is_ready(generation_digest));
ALTER TABLE lctx_serving.selections ENABLE ROW LEVEL SECURITY;
CREATE POLICY ready_read ON lctx_serving.selections FOR SELECT TO lctx_serving USING(lctx_serving.is_ready(generation_digest));
