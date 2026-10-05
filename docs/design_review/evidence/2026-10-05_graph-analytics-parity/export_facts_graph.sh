#!/usr/bin/env bash
# Facts-layer call graph and containment export (read-only; COPY ... TO STDOUT, no temp objects).
# APPROXIMATES the normalized CallableInvocation / DefinitionContainment projections; it does not
# reproduce them: normalized relations, entity_refs and occurrence_ownership are empty in this
# generation because the compile stopped after the facts stage.
#
# Vertex key: <context8>|<S|M|C|D>|<module qualified name>|<native_key>
#   S provider symbol, M module body, C class body, D decorator application.
# Arc: one call_target_observations row (its id is the arc id, so parallel arcs are preserved).
#   caller = provider_call_sites.caller for the same (site, origin)  [1:1 in this generation]
#   callee = destination Resolved symbol (kind 0), Overrides symbol (kind 2: the dispatch set's
#   named member, as the invocation view admits it) or Callable (kind 3).
#   Unresolved (kind 1) and SyntheticFormatting (kind 4) destinations are gaps: counted, not drawn.
# Containment (replaces the empty occurrence_ownership): module body -> each function/method/class
#   symbol of that module and context; class body -> its class symbol.
# Universe: every provider callable, every provider function/method/class symbol, every callee.
#
# Usage: export_facts_graph.sh <schema> <outdir>
set -euo pipefail
SCHEMA=$1 OUT=$2
mkdir -p "$OUT"
PSQL=(psql -h 127.0.0.1 -U lctx_superuser -d lctx -v ON_ERROR_STOP=1 -Atq)
CTE="
mod_name AS (SELECT m.id, coalesce(md.qualified_name, m.bundled_name, m.unresolved_name, m.namespace_name, '?') AS qname
  FROM $SCHEMA.provider_modules m LEFT JOIN $SCHEMA.modules md ON md.id = m.acquired_module),
sym_key AS (SELECT s.id, s.kind, n.qname, substr(encode(s.context,'hex'),1,8) AS ctx,
  substr(encode(s.context,'hex'),1,8) || '|S|' || n.qname || '|' || s.native_key AS key
  FROM $SCHEMA.provider_symbols s JOIN mod_name n ON n.id = s.module),
callable_key AS (SELECT c.id, c.kind, CASE c.kind
    WHEN 0 THEN sk.key
    WHEN 1 THEN substr(encode(c.modulebody_context,'hex'),1,8) || '|M|' || mb.qname || '|'
    WHEN 2 THEN replace(ck.key, '|S|', '|C|')
    WHEN 3 THEN replace(dk.key, '|S|', '|D|') END AS key,
  mb.qname AS mod_qname, substr(encode(c.modulebody_context,'hex'),1,8) AS mod_ctx
  FROM $SCHEMA.provider_callables c
  LEFT JOIN sym_key sk ON sk.id = c.symbol_symbol LEFT JOIN mod_name mb ON mb.id = c.modulebody_module
  LEFT JOIN sym_key ck ON ck.id = c.classbody_class LEFT JOIN sym_key dk ON dk.id = c.decoratorapplication_function),
arcs AS (SELECT encode(t.id,'hex') AS arc, ck.key AS src,
    CASE d.kind WHEN 0 THEN rs.key WHEN 2 THEN os.key WHEN 3 THEN cc.key END AS dst, t.phase, d.kind AS dest_kind
  FROM $SCHEMA.call_target_observations t
  JOIN $SCHEMA.provider_call_sites p ON p.site = t.site AND p.origin = t.origin
  JOIN callable_key ck ON ck.id = p.caller
  JOIN $SCHEMA.call_destinations d ON d.id = t.destination
  LEFT JOIN sym_key rs ON rs.id = d.resolved_symbol LEFT JOIN sym_key os ON os.id = d.overrides_symbol
  LEFT JOIN callable_key cc ON cc.id = d.callable_callable),
vertices AS (SELECT key FROM callable_key WHERE key IS NOT NULL
  UNION SELECT dst FROM arcs WHERE dst IS NOT NULL
  UNION SELECT key FROM sym_key WHERE kind IN (0,1,2)),
containment AS (SELECT DISTINCT c.key AS src, s.key AS dst, 'module-member' AS role
    FROM callable_key c JOIN sym_key s ON s.qname = c.mod_qname AND s.ctx = c.mod_ctx
    WHERE c.kind = 1 AND s.kind IN (0,1,2)
  UNION ALL SELECT replace(s.key,'|S|','|C|'), s.key, 'class-body'
    FROM sym_key s JOIN $SCHEMA.provider_callables c ON c.classbody_class = s.id WHERE c.kind = 2)
"
q() { "${PSQL[@]}" -c "SET default_transaction_read_only = on" -c "$1"; }
q "COPY (WITH $CTE SELECT arc, src, dst, phase, dest_kind FROM arcs WHERE src IS NOT NULL AND dst IS NOT NULL ORDER BY arc) TO STDOUT CSV HEADER" > "$OUT/arcs.csv"
q "COPY (WITH $CTE SELECT key FROM vertices ORDER BY key) TO STDOUT CSV HEADER" > "$OUT/vertices.csv"
q "COPY (WITH $CTE SELECT src, dst, role FROM containment ORDER BY 1,2) TO STDOUT CSV HEADER" > "$OUT/containment.csv"
q "COPY (WITH $CTE SELECT 'arcs_total' AS k, count(*) AS n FROM arcs
  UNION ALL SELECT 'arcs_drawn', count(*) FROM arcs WHERE src IS NOT NULL AND dst IS NOT NULL
  UNION ALL SELECT 'gaps_unresolved_or_synthetic', count(*) FROM arcs WHERE dest_kind IN (1,4)
  UNION ALL SELECT 'gaps_unkeyed_endpoint', count(*) FROM arcs WHERE dest_kind IN (0,2,3) AND (src IS NULL OR dst IS NULL)
  UNION ALL SELECT 'cross_context_arcs', count(*) FROM arcs WHERE dst IS NOT NULL AND split_part(src,'|',1) <> split_part(dst,'|',1)
  UNION ALL SELECT 'vertices', count(*) FROM vertices
  UNION ALL SELECT 'containment', count(*) FROM containment
  UNION ALL SELECT 'arcs_kind'||dest_kind||'_phase'||phase, count(*) FROM arcs GROUP BY dest_kind, phase) TO STDOUT CSV HEADER" > "$OUT/summary.csv"
