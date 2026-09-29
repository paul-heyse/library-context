SELECT count(*) AS "count!" FROM lctx_serving.operation_facets WHERE generation_digest=$1 AND node_id=ANY($2)
