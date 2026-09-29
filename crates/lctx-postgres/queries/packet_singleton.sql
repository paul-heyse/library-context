SELECT DISTINCT class_node_id FROM lctx_serving.singletons WHERE generation_digest=$1 AND global=$2 ORDER BY class_node_id LIMIT 2
