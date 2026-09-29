SELECT detail FROM lctx_serving.retrieval_units
WHERE generation_digest=$1 AND unit_id=$2
