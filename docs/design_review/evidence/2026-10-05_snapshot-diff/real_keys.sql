-- Correspondence-key uniqueness on a real generation (read-only). Run:
--   PGOPTIONS='-c default_transaction_read_only=on' psql -h 127.0.0.1 -U lctx_superuser -d lctx \
--     -v g=lctx_g<generation> -f docs/design_review/evidence/2026-10-05_snapshot-diff/real_keys.sql
-- A candidate key is usable for cross-version diff only if unique within one state; stability
-- across versions needs a second library version (not available). Ids are bytea; codes per
-- crates/lctx-model/src/domain/calls.rs (SymbolKind 0 Function 1 Method 2 Class 3 Variable 4 Module 5 Unknown;
-- SignatureRole 0 Source 1 EffectiveTyped 2 Synthesized 3 Stub 4 Specialized).
\pset footer off
SET search_path = :g;

\echo '== populated entity relations (normalized-layer kinds are absent in a facts-only generation)'
SELECT 'callable_entities' rel, count(*) FROM callable_entities UNION ALL
SELECT 'class_entities', count(*) FROM class_entities UNION ALL
SELECT 'parameter_entities', count(*) FROM parameter_entities UNION ALL
SELECT 'field_entities', count(*) FROM field_entities UNION ALL
SELECT 'modules', count(*) FROM modules UNION ALL
SELECT 'provider_symbols', count(*) FROM provider_symbols UNION ALL
SELECT 'signature_observations', count(*) FROM signature_observations UNION ALL
SELECT 'signature_parameters', count(*) FROM signature_parameters UNION ALL
SELECT 'public_name_observations', count(*) FROM public_name_observations;

\echo '== modules: qualified_name, and (qualified_name, stub flag)'
SELECT count(*) AS modules,
       count(DISTINCT m.qualified_name) AS distinct_qname,
       count(DISTINCT (m.qualified_name, s.path LIKE '%.pyi')) AS distinct_qname_stub,
       count(DISTINCT (m.qualified_name, s.path)) AS distinct_qname_path
FROM modules m JOIN source_artifacts s ON s.id = m.source;
\echo 'modules: sample qualified_name collisions not explained by .py/.pyi'
SELECT m.qualified_name, count(*), string_agg(s.path, ' | ' ORDER BY s.path) paths
FROM modules m JOIN source_artifacts s ON s.id = m.source
GROUP BY m.qualified_name, s.path LIKE '%.pyi' HAVING count(*) > 1 ORDER BY 2 DESC LIMIT 5;

\echo '== provider symbols by provider and kind'
SELECT encode(provider, 'hex') provider, kind, count(*), count(DISTINCT native_key) distinct_native_key,
       count(DISTINCT (context, native_key)) distinct_ctx_key, count(DISTINCT context) contexts
FROM provider_symbols GROUP BY 1, 2 ORDER BY 1, 2;
\echo 'provider symbols: (provider, native_key) collisions, sample'
SELECT encode(provider, 'hex') provider, native_key, count(*), count(DISTINCT context) contexts, count(DISTINCT module) modules
FROM provider_symbols GROUP BY 1, 2 HAVING count(*) > 1 ORDER BY 3 DESC LIMIT 5;

\echo '== callables/classes via provider symbols: key (provider, native_key, kind)'
SELECT CASE WHEN kind IN (0, 1) THEN 'callable' WHEN kind = 2 THEN 'class' ELSE 'other' END entity_kind,
       count(*) n, count(DISTINCT (provider, native_key, kind)) distinct_key,
       count(*) - count(DISTINCT (provider, native_key, kind)) duplicates
FROM provider_symbols GROUP BY 1 ORDER BY 1;

\echo '== signatures: (provider, native_key, role, variant, form) and with qualification'
WITH k AS (SELECT p.provider, p.native_key, s.role, s.variant, s.form, s.qualification, s.parameters
           FROM signature_observations s JOIN provider_symbols p ON p.id = s.symbol)
SELECT count(*) n,
       count(DISTINCT (provider, native_key, role, variant, form)) distinct_key,
       count(DISTINCT (provider, native_key, role, variant, form, qualification)) distinct_key_q,
       count(DISTINCT (provider, native_key, role, variant, form, parameters)) distinct_key_params
FROM k;
\echo 'signatures: roles and sample collisions of (provider, native_key, role, variant, form)'
SELECT role, count(*) FROM signature_observations GROUP BY 1 ORDER BY 1;
WITH k AS (SELECT p.provider, p.native_key, s.role, s.variant, s.form, s.qualification, s.scope, s.parameters
           FROM signature_observations s JOIN provider_symbols p ON p.id = s.symbol)
SELECT native_key, role, variant, form, count(*), count(DISTINCT qualification) quals, count(DISTINCT scope) scopes,
       count(DISTINCT parameters) param_lists
FROM k GROUP BY provider, native_key, role, variant, form HAVING count(*) > 1 ORDER BY 5 DESC LIMIT 5;

\echo '== parameters: (signature correspondence key, ordinal) and (signature key, parameter name)'
WITH k AS (SELECT p.provider, p.native_key, s.role, s.variant, s.form, s.qualification, sp.ordinal, ps.name
           FROM signature_parameters sp JOIN signature_observations s ON s.id = sp.signature
           JOIN provider_symbols p ON p.id = s.symbol JOIN parameter_shapes ps ON ps.id = sp.shape)
SELECT count(*) n,
       count(DISTINCT (provider, native_key, role, variant, form, qualification, ordinal)) distinct_ordinal_key,
       count(DISTINCT (provider, native_key, role, variant, form, qualification, name)) distinct_name_key
FROM k;

\echo '== public API: (module qualified_name, public name)'
SELECT count(*) n, count(DISTINCT (m.qualified_name, o.name)) distinct_path,
       count(DISTINCT (m.qualified_name, o.name, o.qualification)) distinct_path_q
FROM public_name_observations o JOIN modules m ON m.id = o.access;

\echo '== derived qualified path: module qualified_name . enclosing declared definitions . name'
\echo '   (enclosing definitions found by walking syntax_placements parents from the declaration occurrence)'
WITH RECURSIVE decl AS (
  SELECT DISTINCT d.symbol, d.declaration, p.context, p.provider, p.name, p.kind, p.module
  FROM symbol_declarations d JOIN provider_symbols p ON p.id = d.symbol),
parent AS (SELECT DISTINCT occurrence, parent FROM syntax_placements WHERE parent IS NOT NULL),
up(symbol, context, occ, depth, path) AS (
    SELECT symbol, context, declaration, 0, ARRAY[]::text[] FROM decl
  UNION ALL
    SELECT u.symbol, u.context, pa.parent, u.depth + 1,
           CASE WHEN a.name IS NOT NULL THEN a.name || u.path ELSE u.path END
    FROM up u JOIN parent pa ON pa.occurrence = u.occ
    LEFT JOIN LATERAL (SELECT min(name) name FROM decl x WHERE x.declaration = pa.parent AND x.context = u.context) a ON true
    WHERE u.depth < 64
), top AS (SELECT DISTINCT ON (symbol, context) symbol, context, path FROM up ORDER BY symbol, context, depth DESC),
q AS (
  SELECT d.symbol, d.context, d.kind, coalesce(m.qualified_name, '?') || '.' ||
         array_to_string(t.path || d.name, '.') qpath, src.path LIKE '%.pyi' stub
  FROM decl d JOIN top t ON t.symbol = d.symbol AND t.context = d.context
  JOIN provider_modules pm ON pm.id = d.module LEFT JOIN modules m ON m.id = pm.acquired_module
  LEFT JOIN source_artifacts src ON src.id = m.source)
SELECT CASE WHEN kind IN (0, 1) THEN 'callable' WHEN kind = 2 THEN 'class' ELSE 'other' END entity_kind,
       count(*) n, count(DISTINCT (context, qpath, stub, kind)) distinct_in_context,
       count(DISTINCT (qpath, stub, kind)) distinct_across_contexts,
       count(DISTINCT context) contexts
FROM q GROUP BY 1 ORDER BY 1;
\echo 'derived path: sample collisions within one context'
WITH RECURSIVE decl AS (
  SELECT DISTINCT d.symbol, d.declaration, p.context, p.provider, p.name, p.kind, p.module
  FROM symbol_declarations d JOIN provider_symbols p ON p.id = d.symbol),
parent AS (SELECT DISTINCT occurrence, parent FROM syntax_placements WHERE parent IS NOT NULL),
up(symbol, context, occ, depth, path) AS (
    SELECT symbol, context, declaration, 0, ARRAY[]::text[] FROM decl
  UNION ALL
    SELECT u.symbol, u.context, pa.parent, u.depth + 1,
           CASE WHEN a.name IS NOT NULL THEN a.name || u.path ELSE u.path END
    FROM up u JOIN parent pa ON pa.occurrence = u.occ
    LEFT JOIN LATERAL (SELECT min(name) name FROM decl x WHERE x.declaration = pa.parent AND x.context = u.context) a ON true
    WHERE u.depth < 64
), top AS (SELECT DISTINCT ON (symbol, context) symbol, context, path FROM up ORDER BY symbol, context, depth DESC),
q AS (
  SELECT d.symbol, d.context, d.kind, coalesce(m.qualified_name, '?') || '.' ||
         array_to_string(t.path || d.name, '.') qpath, src.path LIKE '%.pyi' stub
  FROM decl d JOIN top t ON t.symbol = d.symbol AND t.context = d.context
  JOIN provider_modules pm ON pm.id = d.module LEFT JOIN modules m ON m.id = pm.acquired_module
  LEFT JOIN source_artifacts src ON src.id = m.source)
SELECT qpath, kind, count(*) FROM q GROUP BY context, qpath, stub, kind HAVING count(*) > 1 ORDER BY 3 DESC, 1 LIMIT 8;

\echo '== within-version signature key: symbol identity + Signature key minus parameters'
SELECT count(*) n, count(DISTINCT (symbol, role, variant, form, qualification, scope, native)) distinct_key,
       count(DISTINCT (symbol, role, variant, form, qualification, scope)) distinct_key_without_native
FROM signature_observations;
