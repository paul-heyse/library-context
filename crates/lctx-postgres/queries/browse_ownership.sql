SELECT member_id,detail::jsonb->>'module' AS module,detail::jsonb->>'class_owner' AS class_owner FROM lctx_serving.catalog_selection_domains WHERE generation_digest=$1 ORDER BY member_id LIMIT 200001
