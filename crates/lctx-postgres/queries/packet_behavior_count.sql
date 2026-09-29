SELECT count(*) AS "count!" FROM lctx_serving.behaviors WHERE generation_digest=$1 AND operation_node_id=ANY($2) AND ($3=false OR kind IN ('delegates','supplies_literal','hands_off_to','takes_from'))
