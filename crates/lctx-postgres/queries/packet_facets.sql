SELECT facet,value,verdict FROM lctx_serving.operation_facets WHERE generation_digest=$1 AND node_id=ANY($2) ORDER BY facet COLLATE "C",value COLLATE "C",verdict COLLATE "C" LIMIT 20 OFFSET $3
