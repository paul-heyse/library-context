WITH best AS (
 SELECT DISTINCT ON (s.member_id,f.family)
   s.member_id,f.family,f.unit_id,f.fragment_id,
   1.0 - (v.vector OPERATOR(lctx_ext.<=>) $3::real[]::lctx_ext.vector(1024)) AS score
 FROM lctx_serving.retrieval_vectors v
 JOIN lctx_serving.retrieval_fragments f
 ON f.generation_digest=v.generation_digest AND f.fragment_id=v.fragment_id
 JOIN lctx_serving.retrieval_subjects s
 ON s.generation_digest=f.generation_digest AND s.unit_id=f.unit_id
 WHERE v.generation_digest=$1 AND s.member_id=ANY($2::bytea[])
 ORDER BY s.member_id,f.family,
 v.vector OPERATOR(lctx_ext.<=>) $3::real[]::lctx_ext.vector(1024),f.unit_id,f.fragment_id
)
SELECT member_id AS "member_id!",family AS "family!",unit_id AS "unit_id!",fragment_id AS "fragment_id!",score AS "score!",
 row_number() OVER (PARTITION BY family ORDER BY score DESC,member_id) AS "rank!"
FROM best ORDER BY family,score DESC,member_id LIMIT 200001
