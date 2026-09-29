SELECT member_id FROM lctx_serving.catalog_members WHERE generation_digest=$1 AND (access_path=$2 OR member_id=$3 OR operation_node_id=$3) ORDER BY access_path COLLATE "C",member_id LIMIT 101
