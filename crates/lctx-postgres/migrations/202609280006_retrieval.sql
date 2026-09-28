-- Index installation does not activate a route. Only a qualified immutable profile is selectable.
CREATE TABLE lctx_serving.profile_attempts (
 attempt_id bigint GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
 generation_digest bytea NOT NULL REFERENCES lctx_serving.generations,
 profile_digest bytea NOT NULL CHECK(octet_length(profile_digest)=32),
 recorded_at timestamptz NOT NULL DEFAULT clock_timestamp(),
 qualification jsonb NOT NULL
);
GRANT SELECT ON lctx_serving.profile_attempts TO lctx_importer;
CREATE FUNCTION lctx_serving.build_hnsw(id bytea) RETURNS void LANGUAGE plpgsql SECURITY DEFINER SET search_path=pg_catalog AS $$
DECLARE leaf text; idx text; expected text; actual text;
BEGIN
 PERFORM pg_advisory_xact_lock(lctx_serving.lock_key(id));
 IF NOT EXISTS(SELECT FROM lctx_serving.generations WHERE generation_digest=id AND state='ready' AND (manifest->>'dimensions')::int=1024) THEN RAISE EXCEPTION 'ready vector generation required'; END IF;
 PERFORM lctx_serving.check_partitions(id);
 FOREACH leaf IN ARRAY ARRAY['v_'||substr(encode(id,'hex'),1,48),'o_'||substr(encode(id,'hex'),1,48)||'_d','o_'||substr(encode(id,'hex'),1,48)||'_s'] LOOP
  idx:=leaf||'_ann';
  IF to_regclass('lctx_serving.'||idx) IS NULL THEN
   EXECUTE format('CREATE INDEX %I ON lctx_serving.%I USING hnsw(vector lctx_ext.vector_cosine_ops) WITH(m=16,ef_construction=128)',idx,leaf);
  END IF;
  SELECT pg_get_indexdef(i.indexrelid) INTO actual FROM pg_index i WHERE i.indexrelid=to_regclass('lctx_serving.'||idx) AND i.indisready AND i.indisvalid;
  expected:=format('CREATE INDEX %I ON lctx_serving.%I USING hnsw (vector lctx_ext.vector_cosine_ops) WITH (m=''16'', ef_construction=''128'')',idx,leaf);
  IF actual IS DISTINCT FROM expected THEN RAISE EXCEPTION 'HNSW definition mismatch'; END IF;
  EXECUTE format('ANALYZE lctx_serving.%I',leaf);
 END LOOP;
END $$;
CREATE FUNCTION lctx_serving.record_profile(id bytea, policy_text text, result jsonb) RETURNS void LANGUAGE plpgsql SECURITY DEFINER SET search_path=pg_catalog AS $$
DECLARE profile bytea:=sha256(convert_to(policy_text,'UTF8'));
BEGIN
 PERFORM pg_advisory_xact_lock(lctx_serving.lock_key(id));
 IF NOT lctx_serving.is_ready(id) THEN RAISE EXCEPTION 'generation is not ready'; END IF;
 IF policy_text IS DISTINCT FROM '{"format":1,"route":"hnsw","metric":"pgvector-cosine-f32-v1","ties":"entity-id-ascending","rrf_k":60,"hnsw":{"m":16,"ef_construction":128,"ef_search":100,"iterative_scan":"strict_order","max_scan_tuples":20000,"work_mem_kib":8192,"scan_mem_multiplier":2,"depths":[200,800,3200],"minimum_recall_basis_points":9900}}' THEN RAISE EXCEPTION 'unsupported ANN policy'; END IF;
 INSERT INTO lctx_serving.profile_attempts(generation_digest,profile_digest,qualification) VALUES(id,profile,result);
 IF (result->>'passed')::boolean IS TRUE AND (result->>'runner')::int=1 AND result->>'generation'=encode(id,'hex')
 AND result->>'profile'=encode(profile,'hex') AND result->>'pack_sha256'~'^[0-9a-f]{64}$'
 AND (result->>'exact_reference_passed')::boolean IS TRUE AND (result->>'plans_passed')::boolean IS TRUE AND (result->>'latency_passed')::boolean IS TRUE
 AND (SELECT array_agg(x->>'name' ORDER BY x->>'name') FROM jsonb_array_elements(result->'strata') x) = ARRAY['briefs:unfiltered:brief','operations:broad:signature_doc','operations:broad:source_body','operations:selective:signature_doc','operations:selective:source_body','operations:unfiltered:signature_doc','operations:unfiltered:source_body'] AND NOT EXISTS(SELECT FROM jsonb_array_elements(result->'strata') x WHERE
  (x->>'queries')::int IS NULL OR (x->>'queries')::int<1 OR (x->>'recall_at_10')::float8 IS NULL OR (x->>'recall_at_10')::float8<0.99 OR
  (x->>'fused_recall_at_10')::float8 IS NULL OR (x->>'fused_recall_at_10')::float8<0.99)
 THEN
  INSERT INTO lctx_serving.retrieval_profiles(generation_digest,profile_digest,policy,qualification,canonical_policy)
  VALUES(id,profile,policy_text::jsonb,result,policy_text) ON CONFLICT DO NOTHING;
 END IF;
END $$;
REVOKE ALL ON FUNCTION lctx_serving.build_hnsw(bytea),lctx_serving.record_profile(bytea,text,jsonb) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION lctx_serving.build_hnsw(bytea),lctx_serving.record_profile(bytea,text,jsonb) TO lctx_importer;
