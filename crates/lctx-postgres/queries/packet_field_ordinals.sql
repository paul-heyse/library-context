SELECT row_ordinal FROM lctx_serving.ambient_reads WHERE generation_digest=$1 AND global=$2 AND field=ANY($3) ORDER BY row_ordinal LIMIT 200001
