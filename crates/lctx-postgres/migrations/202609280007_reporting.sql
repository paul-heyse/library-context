CREATE SCHEMA lctx_report;
REVOKE ALL ON SCHEMA lctx_report FROM PUBLIC;
GRANT USAGE ON SCHEMA lctx_report TO lctx_serving;
CREATE VIEW lctx_report.generation_relations WITH(security_invoker=true) AS
 SELECT g.generation_digest,decode(g.manifest->>'snapshot_id','hex') snapshot_id,
 (g.manifest#>>'{context,library}') COLLATE "C" library,r.key COLLATE "C" relation_name,(r.value->>'rows')::bigint rows
 FROM lctx_serving.generations g CROSS JOIN LATERAL jsonb_each(g.manifest->'relations') r WHERE g.state='ready';
CREATE VIEW lctx_report.operation_outline WITH(security_invoker=true) AS
 SELECT generation_digest,node_id,access_path COLLATE "C" access_path,kind COLLATE "C" kind,is_method,docstring_summary COLLATE "C" docstring_summary
 FROM lctx_serving.operations;
GRANT SELECT ON lctx_report.generation_relations,lctx_report.operation_outline TO lctx_serving;
-- Mutable reporting is captured by one bounded repeatable-read transaction, never a pooled provider.
GRANT USAGE ON SCHEMA lctx_ops TO lctx_serving;
GRANT SELECT ON lctx_ops.attempts,lctx_ops.events,lctx_ops.snapshots,lctx_serving.import_attempts TO lctx_serving;

-- A finite diagnostic capability reveals state and identity, never unready projection rows.
CREATE FUNCTION lctx_report.projection_state(id bytea)
RETURNS TABLE(generation_digest bytea,snapshot_id bytea,content_digest bytea,compiler_digest bytea,library text,state text)
LANGUAGE sql STABLE SECURITY DEFINER SET search_path=pg_catalog AS $$
 SELECT g.generation_digest,decode(g.manifest->>'snapshot_id','hex'),decode(g.manifest->>'snapshot_digest','hex'),
 decode(g.manifest->>'compiler_digest','hex'),g.manifest#>>'{context,library}',g.state
 FROM lctx_serving.generations g WHERE g.generation_digest=id
$$;
REVOKE ALL ON FUNCTION lctx_report.projection_state(bytea) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION lctx_report.projection_state(bytea) TO lctx_serving;
GRANT SELECT ON lctx_serving.profile_attempts TO lctx_serving;
