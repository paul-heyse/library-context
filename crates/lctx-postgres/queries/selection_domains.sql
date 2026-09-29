-- Project only requested domains from a validated immutable canonical record. Preserve the
-- canonical identity for witnesses; this JSON is a request input, never a replacement fact.
SELECT member_id, domain_id,
 jsonb_build_object(
  'module', detail::jsonb->'module',
  'class_owner', detail::jsonb->'class_owner',
  'releases', detail::jsonb->'releases',
  'domains', coalesce((SELECT jsonb_agg(d ORDER BY ordinal)
     FROM jsonb_array_elements(detail::jsonb->'domains') WITH ORDINALITY AS scopes(d,ordinal)
     WHERE d->>'domain'=ANY($2::text[])), '[]'::jsonb)
 )::text AS "detail!"
FROM lctx_serving.catalog_selection_domains
WHERE generation_digest=$1 ORDER BY member_id LIMIT 200001
