-- Policy/history survive restore; physical ANN admission does not.
ALTER TABLE lctx_serving.retrieval_profiles DROP CONSTRAINT retrieval_profiles_policy_check;
ALTER TABLE lctx_serving.retrieval_profiles ADD CONSTRAINT retrieval_profiles_policy_check
 CHECK(policy->>'route' IN ('exact','hnsw','mixed'));
CREATE FUNCTION lctx_serving.index_realization(id bytea) RETURNS jsonb
LANGUAGE plpgsql SECURITY DEFINER SET search_path=pg_catalog AS $$
DECLARE result jsonb; prefix text:=substr(encode(id,'hex'),1,48);
BEGIN
 PERFORM lctx_serving.check_partitions(id);
 SELECT jsonb_build_object(
  'engine',2,'generation',encode(id,'hex'),
  'cluster',(SELECT system_identifier::text FROM pg_control_system()),
  'database',(SELECT oid FROM pg_database WHERE datname=current_database()),
  'server',current_setting('server_version_num'),
  'extension',(SELECT extversion FROM pg_extension WHERE extname='vector'),
  'valid',count(i.indexrelid)=3 AND coalesce(bool_and(i.indisvalid AND i.indisready AND i.indrelid=to_regclass('lctx_serving.'||x.leaf) AND pg_get_indexdef(i.indexrelid)=format('CREATE INDEX %I ON lctx_serving.%I USING hnsw (vector lctx_ext.vector_cosine_ops) WITH (m=''16'', ef_construction=''128'')',x.leaf||'_ann',x.leaf)),false),
  'indexes',jsonb_agg(jsonb_build_object('view',x.view,'leaf',x.leaf,'oid',i.indexrelid,
    'filenode',pg_relation_filenode(i.indexrelid),'definition',pg_get_indexdef(i.indexrelid),
    'ready',i.indisready,'valid',i.indisvalid) ORDER BY x.view)) INTO result
 FROM (VALUES('brief','v_'||prefix),('signature_doc','o_'||prefix||'_d'),('source_body','o_'||prefix||'_s')) x(view,leaf)
 LEFT JOIN pg_index i ON i.indexrelid=to_regclass('lctx_serving.'||x.leaf||'_ann');
 RETURN result;
END $$;

CREATE TABLE lctx_serving.profile_admissions (
 generation_digest bytea NOT NULL,
 profile_digest bytea NOT NULL,
 realization_digest bytea NOT NULL CHECK(octet_length(realization_digest)=32),
 realization jsonb NOT NULL,
 attempt_id bigint NOT NULL REFERENCES lctx_serving.profile_attempts,
 admitted_at timestamptz NOT NULL DEFAULT clock_timestamp(),
 PRIMARY KEY(generation_digest,profile_digest,realization_digest),
 FOREIGN KEY(generation_digest,profile_digest) REFERENCES lctx_serving.retrieval_profiles
);
ALTER TABLE lctx_serving.profile_admissions ENABLE ROW LEVEL SECURITY;
CREATE POLICY ready_read ON lctx_serving.profile_admissions FOR SELECT TO lctx_serving,lctx_importer
 USING(lctx_serving.is_ready(generation_digest));
GRANT SELECT ON lctx_serving.profile_admissions TO lctx_serving,lctx_importer;

CREATE FUNCTION lctx_serving.record_mixed_profile(id bytea, policy_text text, result jsonb) RETURNS void
LANGUAGE plpgsql SECURITY DEFINER SET search_path=pg_catalog AS $$
DECLARE profile bytea:=sha256(convert_to(policy_text,'UTF8')); policy jsonb:=policy_text::jsonb;
 current_realization jsonb; attempt bigint; classes text[]; checked text[];
BEGIN
 PERFORM pg_advisory_xact_lock(lctx_serving.lock_key(id));
 IF NOT lctx_serving.is_ready(id) THEN RAISE EXCEPTION 'generation is not ready'; END IF;
 IF policy->>'route' IS DISTINCT FROM 'mixed' OR policy->>'format' IS DISTINCT FROM '2'
 OR policy->>'metric' IS DISTINCT FROM 'pgvector-cosine-f32-v1' OR policy->>'ties' IS DISTINCT FROM 'entity-id-ascending'
 OR policy->>'rrf_k' IS DISTINCT FROM '60' OR (policy#>>'{routing,count_floor}' IS NULL OR policy#>>'{routing,count_floor}' NOT IN ('1024','4096'))
 OR policy->'hnsw' IS DISTINCT FROM '{"m":16,"ef_construction":128,"ef_search":100,"iterative_scan":"strict_order","max_scan_tuples":20000,"work_mem_kib":8192,"scan_mem_multiplier":2,"depths":[200,800,3200],"minimum_recall_basis_points":9900}'::jsonb
 THEN RAISE EXCEPTION 'unsupported mixed policy'; END IF;
 SELECT array_agg(v ORDER BY v) INTO classes FROM jsonb_array_elements_text(policy#>'{routing,classes}') x(v);
 IF classes IS NULL OR cardinality(classes)>2 OR NOT classes <@ ARRAY['broad','unfiltered']
 OR (SELECT count(DISTINCT v) FROM unnest(classes) x(v))<>cardinality(classes)
 THEN RAISE EXCEPTION 'unsupported routing classes'; END IF;
 INSERT INTO lctx_serving.profile_attempts(generation_digest,profile_digest,qualification)
 VALUES(id,profile,result) RETURNING attempt_id INTO attempt;
 IF (result->>'passed')::boolean IS DISTINCT FROM true OR result->>'phase' IS DISTINCT FROM 'confirmation' THEN RETURN; END IF;
 current_realization:=lctx_serving.index_realization(id);
 SELECT array_agg(v->>'class' ORDER BY v->>'class') INTO checked FROM jsonb_array_elements(result->'classes') x(v);
 IF current_realization->>'valid'<>'true' OR result->'realization' IS DISTINCT FROM current_realization
 OR result->>'runner' IS DISTINCT FROM '2' OR result->>'generation' IS DISTINCT FROM encode(id,'hex') OR result->>'profile' IS DISTINCT FROM encode(profile,'hex')
 OR result->'policy' IS DISTINCT FROM policy OR NOT coalesce(result->>'pack_sha256' ~ '^[0-9a-f]{64}$',false)
 OR NOT coalesce(result->>'calibration_sha256' ~ '^[0-9a-f]{64}$',false) OR result->>'calibration_sha256'=result->>'pack_sha256'
 OR result->>'exact_reference_passed' IS DISTINCT FROM 'true' OR checked IS DISTINCT FROM classes
 OR NOT EXISTS(SELECT FROM lctx_serving.profile_attempts p WHERE p.generation_digest=id
   AND p.qualification->>'phase'='calibration' AND p.qualification->>'pack_sha256'=result->>'calibration_sha256'
   AND p.qualification->'chosen_policy'=policy)
 OR EXISTS(SELECT FROM jsonb_array_elements(result->'classes') x(c) WHERE
   coalesce((c->>'queries')::int,0)<8 OR coalesce(jsonb_array_length(c->'runs'),0)<>2 OR
   EXISTS(SELECT FROM jsonb_array_elements(c->'runs') y(r) WHERE
     coalesce((r->>'passed')::boolean,false)=false OR coalesce((r->>'plans_passed')::boolean,false)=false OR coalesce((r->>'ann_execution_passed')::boolean,false)=false
     OR coalesce((r->>'recall_at_10')::float8,0)<0.99 OR coalesce((r->>'fused_recall_at_10')::float8,0)<0.99
     OR NOT coalesce((r->>'ann_p95_ms')::float8 BETWEEN 0 AND 250,false)
     OR NOT coalesce((r->>'ann_p95_ms')::float8 <= (r->>'exact_p95_ms')::float8*0.8,false)))
 THEN RAISE EXCEPTION 'mixed confirmation contract failed'; END IF;
 INSERT INTO lctx_serving.retrieval_profiles(generation_digest,profile_digest,policy,qualification,canonical_policy)
 VALUES(id,profile,policy,result,policy_text) ON CONFLICT DO NOTHING;
 INSERT INTO lctx_serving.profile_admissions VALUES(id,profile,sha256(convert_to(current_realization::text,'UTF8')),current_realization,attempt,clock_timestamp())
 ON CONFLICT DO NOTHING;
END $$;

CREATE OR REPLACE FUNCTION lctx_serving.select_generation(lib text, id bytea, profile bytea) RETURNS void
LANGUAGE plpgsql SECURITY DEFINER SET search_path=pg_catalog AS $$
DECLARE route text;
BEGIN
 PERFORM pg_advisory_xact_lock_shared(lctx_serving.lock_key(id));
 SELECT p.policy->>'route' INTO route FROM lctx_serving.generations g JOIN lctx_serving.retrieval_profiles p USING(generation_digest)
 WHERE g.generation_digest=id AND g.state='ready' AND g.manifest#>>'{context,library}'=lib AND p.profile_digest=profile;
 IF route IS NULL THEN RAISE EXCEPTION 'incompatible ready selection'; END IF;
 IF route<>'exact' AND (route<>'mixed' OR NOT EXISTS(SELECT FROM lctx_serving.profile_admissions a
   WHERE a.generation_digest=id AND a.profile_digest=profile AND a.realization=lctx_serving.index_realization(id)
   AND a.realization->>'valid'='true')) THEN RAISE EXCEPTION 'physical index admission missing or stale'; END IF;
 INSERT INTO lctx_serving.selections VALUES(lib,id,profile) ON CONFLICT(library)
 DO UPDATE SET generation_digest=EXCLUDED.generation_digest,profile_digest=EXCLUDED.profile_digest;
END $$;
REVOKE ALL ON FUNCTION lctx_serving.index_realization(bytea),lctx_serving.record_mixed_profile(bytea,text,jsonb) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION lctx_serving.index_realization(bytea) TO lctx_serving,lctx_importer;
GRANT EXECUTE ON FUNCTION lctx_serving.record_mixed_profile(bytea,text,jsonb) TO lctx_importer;

GRANT EXECUTE ON FUNCTION lctx_serving.lock_key(bytea) TO lctx_serving;

CREATE FUNCTION lctx_serving.import_diagnostics(id bytea) RETURNS jsonb
LANGUAGE sql STABLE SECURITY DEFINER SET search_path=pg_catalog AS $$
 SELECT jsonb_build_object('completed_batches',(SELECT count(*) FROM lctx_serving.import_batches WHERE generation_digest=id),
 'attempts',(SELECT count(*) FROM lctx_serving.import_attempts WHERE generation_digest=id))
 WHERE session_user='lctx_importer' OR lctx_serving.is_ready(id)
$$;
REVOKE ALL ON FUNCTION lctx_serving.import_diagnostics(bytea) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION lctx_serving.import_diagnostics(bytea) TO lctx_serving,lctx_importer;
