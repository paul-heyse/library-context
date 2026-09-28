-- ADR-0069: content-bound context and finite publication capabilities.
-- Keep deployed migrations 001-004 byte-identical.
DO $$ DECLARE c record; BEGIN
 FOR c IN SELECT conname FROM pg_constraint WHERE conrelid='lctx_serving.generations'::regclass
 AND contype='c' AND (pg_get_constraintdef(oid) LIKE '%bundle_format%' OR pg_get_constraintdef(oid) LIKE '%''format''%')
 LOOP EXECUTE format('ALTER TABLE lctx_serving.generations DROP CONSTRAINT %I',c.conname); END LOOP;
END $$;
ALTER TABLE lctx_serving.generations ADD CHECK
 ((manifest->>'format')::int IN (1,2) AND (manifest->>'bundle_format')::int IN (11,12));
-- Old frozen generations remain retained; the new importer admits only FORMAT 2 / 12.
CREATE TABLE lctx_serving.import_recipes (
 generation_digest bytea PRIMARY KEY REFERENCES lctx_serving.generations,
 recipe integer NOT NULL CHECK(recipe=1),
 transport_digest bytea NOT NULL CHECK(octet_length(transport_digest)=32)
);
CREATE TABLE lctx_serving.import_attempts (
 attempt_id bigint GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
 generation_digest bytea NOT NULL REFERENCES lctx_serving.generations,
 started_at timestamptz NOT NULL DEFAULT clock_timestamp(),
 finished_at timestamptz,
 outcome text NOT NULL DEFAULT 'running' CHECK(outcome IN ('running','completed','interrupted','failed')),
 failure_code text CHECK(octet_length(failure_code)<=128)
);
CREATE TABLE lctx_serving.import_batches (
 generation_digest bytea NOT NULL REFERENCES lctx_serving.generations,
 relation_name text NOT NULL,
 batch_ordinal bigint NOT NULL CHECK(batch_ordinal>=0),
 first_row bigint NOT NULL CHECK(first_row>=0),
 row_count bigint NOT NULL CHECK(row_count BETWEEN 0 AND 1000),
 content_digest bytea NOT NULL CHECK(octet_length(content_digest)=32),
 PRIMARY KEY(generation_digest,relation_name,batch_ordinal)
);
CREATE TABLE lctx_serving.validation_receipts (
 generation_digest bytea PRIMARY KEY REFERENCES lctx_serving.generations,
 validator integer NOT NULL CHECK(validator=1),
 manifest_digest bytea NOT NULL CHECK(octet_length(manifest_digest)=32),
 validated_at timestamptz NOT NULL DEFAULT clock_timestamp(),
 relations jsonb NOT NULL,
 CHECK(manifest_digest=generation_digest)
);
GRANT SELECT ON lctx_serving.import_recipes,lctx_serving.import_attempts,lctx_serving.import_batches,lctx_serving.validation_receipts TO lctx_importer;
GRANT INSERT ON lctx_serving.import_batches TO lctx_importer;
CREATE TRIGGER frozen_batches BEFORE INSERT OR UPDATE OR DELETE ON lctx_serving.import_batches
 FOR EACH ROW EXECUTE FUNCTION lctx_serving.require_loading();

CREATE FUNCTION lctx_serving.lock_key(id bytea) RETURNS bigint LANGUAGE sql IMMUTABLE STRICT
 SET search_path=pg_catalog AS $$ SELECT ('x'||substr(encode(id,'hex'),1,16))::bit(64)::bigint $$;

CREATE FUNCTION lctx_serving.prepare_generation(doc text, transport bytea) RETURNS bigint
 LANGUAGE plpgsql SECURITY DEFINER SET search_path=pg_catalog AS $$
DECLARE id bytea:=sha256(convert_to(doc,'UTF8')); prior text; a bigint; n text; child text; v text;
BEGIN
 IF (doc::jsonb->>'format')::int IS DISTINCT FROM 2 OR (doc::jsonb->>'bundle_format')::int IS DISTINCT FROM 12
 OR coalesce(doc::jsonb#>>'{context,library}','')='' THEN RAISE EXCEPTION 'unsupported projection context'; END IF;
 PERFORM pg_advisory_xact_lock(lctx_serving.lock_key(id));
 INSERT INTO lctx_serving.generations(generation_digest,canonical_manifest) VALUES(id,doc) ON CONFLICT DO NOTHING;
 SELECT canonical_manifest INTO prior FROM lctx_serving.generations WHERE generation_digest=id;
 IF prior<>doc THEN RAISE EXCEPTION 'projection identity conflict'; END IF;
 IF EXISTS(SELECT FROM lctx_serving.generations WHERE generation_digest=id AND state='failed') THEN RAISE EXCEPTION 'terminal projection failure'; END IF;
 INSERT INTO lctx_serving.import_recipes VALUES(id,1,transport) ON CONFLICT DO NOTHING;
 IF EXISTS(SELECT FROM lctx_serving.import_recipes WHERE generation_digest=id AND transport_digest<>transport)
 AND NOT lctx_serving.is_ready(id) THEN RAISE EXCEPTION 'frozen transport conflict'; END IF;
 UPDATE lctx_serving.import_attempts SET outcome='interrupted',finished_at=clock_timestamp(),failure_code='owner_disconnected'
 WHERE generation_digest=id AND outcome='running';
 INSERT INTO lctx_serving.import_attempts(generation_digest) VALUES(id) RETURNING attempt_id INTO a;
 IF NOT lctx_serving.is_ready(id) THEN
  FOREACH n IN ARRAY ARRAY['vectors','operation_vectors'] LOOP
   child:=CASE n WHEN 'vectors' THEN 'v_' ELSE 'o_' END || substr(encode(id,'hex'),1,48);
   IF to_regclass('lctx_serving.'||child) IS NULL THEN
    EXECUTE format('CREATE TABLE lctx_serving.%I PARTITION OF lctx_serving.%I FOR VALUES IN (%L)%s',child,n,id,
     CASE n WHEN 'operation_vectors' THEN ' PARTITION BY LIST (embedding_view)' ELSE '' END);
    IF n='operation_vectors' THEN
     FOREACH v IN ARRAY ARRAY['signature_doc','source_body'] LOOP
      EXECUTE format('CREATE TABLE lctx_serving.%I PARTITION OF lctx_serving.%I FOR VALUES IN (%L)',child||CASE v WHEN 'signature_doc' THEN '_d' ELSE '_s' END,child,v);
     END LOOP;
    END IF;
   ELSIF NOT EXISTS(SELECT FROM pg_class WHERE oid=to_regclass('lctx_serving.'||child)
      AND pg_get_expr(relpartbound,oid) LIKE '%'||encode(id,'hex')||'%') THEN
    RAISE EXCEPTION 'partition identity conflict';
   END IF;
  END LOOP;
 END IF;
 PERFORM lctx_serving.check_partitions(id);
 RETURN a;
END $$;

CREATE FUNCTION lctx_serving.finish_attempt(a bigint, result text, code text DEFAULT NULL) RETURNS void
 LANGUAGE plpgsql SECURITY DEFINER SET search_path=pg_catalog AS $$ BEGIN
 IF result NOT IN ('completed','interrupted','failed') THEN RAISE EXCEPTION 'invalid attempt result'; END IF;
 UPDATE lctx_serving.import_attempts SET outcome=result,finished_at=clock_timestamp(),failure_code=code
 WHERE attempt_id=a AND outcome='running';
END $$;

CREATE FUNCTION lctx_serving.register_artifact(id bytea, artifact_name text, digest bytea, size bigint, version integer, uri text)
 RETURNS void LANGUAGE plpgsql SECURITY DEFINER SET search_path=pg_catalog AS $$
DECLARE phase text; expected jsonb;
BEGIN
 SELECT state,manifest->'artifacts'->artifact_name INTO phase,expected FROM lctx_serving.generations WHERE generation_digest=id FOR SHARE;
 IF phase NOT IN ('loading','validating','ready') OR expected IS NULL OR expected->>'sha256'<>encode(digest,'hex')
 OR (expected->>'bytes')::bigint<>size OR (expected->>'format')::int<>version THEN RAISE EXCEPTION 'artifact manifest mismatch'; END IF;
 IF phase='loading' THEN
  INSERT INTO lctx_serving.artifacts VALUES(id,artifact_name,digest,size,version) ON CONFLICT DO NOTHING;
 END IF;
 IF NOT EXISTS(SELECT FROM lctx_serving.artifacts WHERE generation_digest=id AND name=artifact_name AND content_digest=digest AND bytes=size AND format=version)
 THEN RAISE EXCEPTION 'artifact content conflict'; END IF;
 INSERT INTO lctx_serving.artifact_locations VALUES(id,artifact_name,uri) ON CONFLICT DO NOTHING;
END $$;

ALTER TABLE lctx_serving.retrieval_profiles ADD COLUMN canonical_policy text;
UPDATE lctx_serving.retrieval_profiles SET canonical_policy=policy::text;
ALTER TABLE lctx_serving.retrieval_profiles ALTER COLUMN canonical_policy SET NOT NULL;
ALTER TABLE lctx_serving.retrieval_profiles ADD CHECK(policy=canonical_policy::jsonb AND profile_digest=sha256(convert_to(canonical_policy,'UTF8')));
CREATE FUNCTION lctx_serving.immutable_profile() RETURNS trigger LANGUAGE plpgsql SET search_path=pg_catalog AS $$ BEGIN
 RAISE EXCEPTION 'retrieval profiles are immutable';
END $$;
CREATE TRIGGER immutable_profile BEFORE UPDATE OR DELETE ON lctx_serving.retrieval_profiles FOR EACH ROW EXECUTE FUNCTION lctx_serving.immutable_profile();

CREATE FUNCTION lctx_serving.mark_ready(id bytea, policy_text text, receipts jsonb) RETURNS void
 LANGUAGE plpgsql SECURITY DEFINER SET search_path=pg_catalog AS $$
DECLARE phase text; expected bigint; actual bigint; declared jsonb; r record;
BEGIN
 PERFORM pg_advisory_xact_lock(lctx_serving.lock_key(id));
 SELECT state INTO phase FROM lctx_serving.generations WHERE generation_digest=id FOR UPDATE;
 IF phase='ready' THEN RETURN; END IF;
 IF phase IS DISTINCT FROM 'validating' OR policy_text IS DISTINCT FROM '{"format":1,"route":"exact","metric":"pgvector-cosine-f32-v1","ties":"entity-id-ascending","rrf_k":60,"hnsw":null}'  THEN RAISE EXCEPTION 'not validated exact publication'; END IF;
 SELECT count(*) INTO expected FROM lctx_serving.generations g,jsonb_object_keys(g.manifest->'artifacts') WHERE g.generation_digest=id;
 SELECT count(*) INTO actual FROM lctx_serving.artifacts WHERE generation_digest=id;
 IF expected<>actual OR EXISTS(SELECT FROM lctx_serving.artifacts a WHERE a.generation_digest=id AND NOT EXISTS
  (SELECT FROM lctx_serving.artifact_locations l WHERE l.generation_digest=id AND l.name=a.name)) THEN RAISE EXCEPTION 'missing artifacts'; END IF;
 SELECT manifest->'relations' INTO declared FROM lctx_serving.generations WHERE generation_digest=id;
 IF receipts IS DISTINCT FROM declared THEN RAISE EXCEPTION 'shared validation receipt mismatch'; END IF;
 FOR r IN SELECT key,value FROM jsonb_each(declared) LOOP
  IF NOT EXISTS(SELECT FROM information_schema.columns WHERE table_schema='lctx_serving' AND table_name=r.key AND column_name='row_ordinal')
  THEN RAISE EXCEPTION 'undeclared relation'; END IF;
  EXECUTE format('SELECT count(*) FROM lctx_serving.%I WHERE generation_digest=$1',r.key) INTO actual USING id;
  IF actual IS DISTINCT FROM (r.value->>'rows')::bigint THEN RAISE EXCEPTION 'stored relation count mismatch'; END IF;
 END LOOP;
 PERFORM lctx_serving.check_partitions(id);
 INSERT INTO lctx_serving.validation_receipts VALUES(id,1,id,clock_timestamp(),receipts) ON CONFLICT DO NOTHING;
 INSERT INTO lctx_serving.retrieval_profiles(generation_digest,profile_digest,policy,qualification,canonical_policy)
 VALUES(id,sha256(convert_to(policy_text,'UTF8')),policy_text::jsonb,'{"route":"exact","validator":1}',policy_text);
 UPDATE lctx_serving.generations SET state='ready' WHERE generation_digest=id;
END $$;

CREATE FUNCTION lctx_serving.mark_failed(id bytea) RETURNS void LANGUAGE plpgsql SECURITY DEFINER SET search_path=pg_catalog AS $$ BEGIN
 PERFORM pg_advisory_xact_lock(lctx_serving.lock_key(id));
 UPDATE lctx_serving.generations SET state='failed' WHERE generation_digest=id AND state IN ('loading','validating');
END $$;

CREATE FUNCTION lctx_serving.select_generation(lib text, id bytea, profile bytea) RETURNS void
 LANGUAGE plpgsql SECURITY DEFINER SET search_path=pg_catalog AS $$ BEGIN
 IF NOT EXISTS(SELECT FROM lctx_serving.generations g JOIN lctx_serving.retrieval_profiles p USING(generation_digest)
 WHERE g.generation_digest=id AND g.state='ready' AND g.manifest#>>'{context,library}'=lib AND p.profile_digest=profile)
 THEN RAISE EXCEPTION 'incompatible ready selection'; END IF;
 INSERT INTO lctx_serving.selections VALUES(lib,id,profile) ON CONFLICT(library)
 DO UPDATE SET generation_digest=EXCLUDED.generation_digest,profile_digest=EXCLUDED.profile_digest;
END $$;
GRANT SELECT ON lctx_serving.artifact_locations,lctx_serving.retrieval_profiles,lctx_serving.selections TO lctx_importer;

-- Static query access paths. Generation prefixes enforce isolation; physical ordinals retain duplicates.
CREATE INDEX operation_order ON lctx_serving.operations(generation_digest,access_path COLLATE "C",node_id);
CREATE INDEX facet_lookup ON lctx_serving.operation_facets(generation_digest,facet,value,verdict,node_id);
CREATE INDEX behavior_owner ON lctx_serving.behaviors(generation_digest,operation_node_id,row_ordinal);
CREATE INDEX discharge_owner ON lctx_serving.behavior_discharges(generation_digest,behavior_id,row_ordinal);
CREATE INDEX assertion_owner ON lctx_serving.assertions(generation_digest,brief_id,ordinal);
CREATE INDEX support_owner ON lctx_serving.supports(generation_digest,assertion_id,ordinal);
CREATE INDEX witness_owner ON lctx_serving.support_witnesses(generation_digest,finding_id,row_ordinal);
CREATE INDEX member_owner ON lctx_serving.support_members(generation_digest,finding_id,row_ordinal);
CREATE INDEX incidence_owner ON lctx_serving.support_attribute_incidences(generation_digest,finding_id,row_ordinal);

REVOKE ALL ON ALL FUNCTIONS IN SCHEMA lctx_serving FROM PUBLIC;
GRANT EXECUTE ON FUNCTION lctx_serving.prepare_generation(text,bytea),lctx_serving.finish_attempt(bigint,text,text),
 lctx_serving.register_artifact(bytea,text,bytea,bigint,integer,text),lctx_serving.mark_ready(bytea,text,jsonb),
 lctx_serving.mark_failed(bytea),lctx_serving.select_generation(text,bytea,bytea) TO lctx_importer;
CREATE POLICY importer_read ON lctx_serving.artifact_locations FOR SELECT TO lctx_importer USING(true);
CREATE POLICY importer_read ON lctx_serving.retrieval_profiles FOR SELECT TO lctx_importer USING(true);
CREATE POLICY importer_read ON lctx_serving.selections FOR SELECT TO lctx_importer USING(true);

CREATE FUNCTION lctx_serving.analyze_generation(id bytea) RETURNS void LANGUAGE plpgsql SECURITY DEFINER SET search_path=pg_catalog AS $$
DECLARE n text; BEGIN
 IF NOT EXISTS(SELECT FROM lctx_serving.generations WHERE generation_digest=id AND state='validating') THEN RAISE EXCEPTION 'not validating'; END IF;
 FOREACH n IN ARRAY ARRAY['vectors','operation_vectors','operations','operation_facets','operation_facet_status','behaviors','assertions','supports'] LOOP
  EXECUTE format('ANALYZE lctx_serving.%I',n);
 END LOOP;
END $$;
CREATE OR REPLACE FUNCTION lctx_serving.require_loading() RETURNS trigger LANGUAGE plpgsql SECURITY DEFINER SET search_path=pg_catalog AS $$
DECLARE id bytea; phase text; BEGIN
 id:=CASE WHEN TG_OP='DELETE' THEN OLD.generation_digest ELSE NEW.generation_digest END;
 SELECT state INTO phase FROM lctx_serving.generations WHERE generation_digest=id FOR SHARE;
 IF TG_OP='DELETE' AND phase IN ('loading','validating','failed') THEN RETURN OLD; END IF;
 IF phase IS DISTINCT FROM 'loading' THEN RAISE EXCEPTION 'generation is frozen' USING ERRCODE='55000'; END IF;
 IF TG_OP='UPDATE' AND OLD.generation_digest<>NEW.generation_digest THEN RAISE EXCEPTION 'generation identity is immutable' USING ERRCODE='55000'; END IF;
 RETURN NEW;
END $$;
CREATE FUNCTION lctx_serving.cleanup_generation(id bytea) RETURNS void LANGUAGE plpgsql SECURITY DEFINER SET search_path=pg_catalog AS $$
DECLARE n text; phase text; BEGIN
 IF NOT pg_try_advisory_xact_lock(lctx_serving.lock_key(id)) THEN RAISE EXCEPTION 'generation import is active'; END IF;
 SELECT state INTO phase FROM lctx_serving.generations WHERE generation_digest=id FOR UPDATE;
 IF phase IS NULL OR phase='ready' OR EXISTS(SELECT FROM lctx_serving.selections WHERE generation_digest=id) THEN RAISE EXCEPTION 'generation cannot be cleaned'; END IF;
 SET CONSTRAINTS ALL DEFERRED;
 FOREACH n IN ARRAY ARRAY['briefs','assertions','supports','support_findings','support_witnesses','support_members','support_attributes','support_attribute_incidences','evidence','brief_members','symbol_map','public_paths','lexical_text','embedding_spec','vectors','operations','operation_facets','operation_facet_status','behaviors','conditions','condition_nodes','analysis_conditions','analysis_condition_nodes','callable_parameters','summary_flows','source_context_value_identities','source_body_completions','source_body_steps','source_body_release_inputs','source_call_bindings','source_call_normals','source_call_header_steps','model_frame_exits','model_frame_exit_arguments','model_frame_exit_steps','source_modeled_identities','source_parameter_identities','return_completion_certificates','model_context_protocols','source_context_sites','source_context_arguments','summary_flow_steps','summary_boundaries','behavior_discharges','flow_test_leaves','flow_test_value_links','singletons','ambient_reads','place_claims','operation_text','operation_vectors'] LOOP
  EXECUTE format('DELETE FROM lctx_serving.%I WHERE generation_digest=$1',n) USING id;
 END LOOP;
 DELETE FROM lctx_serving.artifact_locations WHERE generation_digest=id;
 DELETE FROM lctx_serving.artifacts WHERE generation_digest=id;
 IF phase<>'failed' THEN UPDATE lctx_serving.generations SET state='failed' WHERE generation_digest=id; END IF;
 UPDATE lctx_serving.import_attempts SET outcome='interrupted',finished_at=clock_timestamp(),failure_code='owned_cleanup' WHERE generation_digest=id AND outcome='running';
 -- Keep the manifest, transport, attempts and batch receipts as terminal diagnostic history.
 -- Content-addressed files remain retained, including files shared with other generations.
END $$;
REVOKE ALL ON ALL FUNCTIONS IN SCHEMA lctx_serving FROM PUBLIC;
GRANT EXECUTE ON FUNCTION lctx_serving.analyze_generation(bytea),lctx_serving.cleanup_generation(bytea) TO lctx_importer;

-- Full parent/bound checks; names alone are never physical identity.
CREATE FUNCTION lctx_serving.check_partitions(id bytea) RETURNS void LANGUAGE plpgsql SECURITY DEFINER SET search_path=pg_catalog AS $$
DECLARE n text; child text; v text; leaf text; bound text;
BEGIN
 FOREACH n IN ARRAY ARRAY['vectors','operation_vectors'] LOOP
  child:=CASE n WHEN 'vectors' THEN 'v_' ELSE 'o_' END || substr(encode(id,'hex'),1,48);
  SELECT pg_get_expr(c.relpartbound,c.oid) INTO bound FROM pg_class c JOIN pg_inherits i ON i.inhrelid=c.oid
   WHERE c.oid=to_regclass('lctx_serving.'||child) AND i.inhparent=to_regclass('lctx_serving.'||n);
  IF bound IS DISTINCT FROM ('FOR VALUES IN ('''||chr(92)||'x'||encode(id,'hex')||''')')
  THEN RAISE EXCEPTION 'generation partition definition mismatch'; END IF;
  IF n='operation_vectors' THEN
   IF (SELECT pg_get_partkeydef(to_regclass('lctx_serving.'||child))) IS DISTINCT FROM 'LIST (embedding_view)' THEN RAISE EXCEPTION 'view partition key mismatch'; END IF;
   FOREACH v IN ARRAY ARRAY['signature_doc','source_body'] LOOP
    leaf:=child||CASE v WHEN 'signature_doc' THEN '_d' ELSE '_s' END;
    SELECT pg_get_expr(c.relpartbound,c.oid) INTO bound FROM pg_class c JOIN pg_inherits i ON i.inhrelid=c.oid
     WHERE c.oid=to_regclass('lctx_serving.'||leaf) AND i.inhparent=to_regclass('lctx_serving.'||child);
    IF bound IS DISTINCT FROM format('FOR VALUES IN (%L)',v) AND bound IS DISTINCT FROM format('FOR VALUES IN (%L::text)',v)
    THEN RAISE EXCEPTION 'view partition definition mismatch'; END IF;
   END LOOP;
  END IF;
 END LOOP;
END $$;
REVOKE ALL ON FUNCTION lctx_serving.check_partitions(bytea) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION lctx_serving.check_partitions(bytea) TO lctx_importer;
