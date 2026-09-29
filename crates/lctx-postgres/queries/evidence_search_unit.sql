WITH metadata AS (
 SELECT ((detail::jsonb - 'text') || jsonb_build_object('text',''))::text AS detail
 FROM lctx_serving.retrieval_units WHERE generation_digest=$1 AND unit_id=$2
)
SELECT CASE WHEN octet_length(detail)<=16777216 THEN detail END FROM metadata
