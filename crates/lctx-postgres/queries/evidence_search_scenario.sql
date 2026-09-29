SELECT jsonb_build_object(
 'context', detail::jsonb->'context', 'intent', detail::jsonb->'intent',
 'checks', detail::jsonb->'checks', 'extraction', detail::jsonb->'extraction',
 'requirements', jsonb_array_length(detail::jsonb->'requirements'),
 'option_bindings', jsonb_array_length(detail::jsonb->'option_bindings'),
 'omitted_options', detail::jsonb->'omitted_options',
 'omitted_requirements', detail::jsonb->'omitted_requirements'
)::text FROM lctx_serving.catalog_scenarios WHERE generation_digest=$1 AND scenario_id=$2
