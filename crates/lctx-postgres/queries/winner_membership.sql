SELECT NOT EXISTS (
 SELECT 1 FROM unnest($2::bytea[], $3::text[], $4::bytea[], $5::bytea[])
 AS w(member_id, family, unit_id, fragment_id)
 LEFT JOIN lctx_serving.retrieval_subjects s
 ON s.generation_digest=$1 AND s.unit_id=w.unit_id AND s.member_id=w.member_id
 LEFT JOIN lctx_serving.retrieval_fragments f
 ON f.generation_digest=$1 AND f.fragment_id=w.fragment_id
 AND f.unit_id=w.unit_id AND f.family=w.family
 WHERE s.unit_id IS NULL OR f.fragment_id IS NULL
) AS "valid!"
