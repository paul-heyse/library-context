WITH exported AS (
  SELECT DISTINCT declaration_node_id AS node_id, access_path FROM exports
  WHERE split_part(access_path, '.', 1) IN ('fastmcp')
    AND strpos(access_path, '._') = 0 AND NOT starts_with(access_path, '_')),
direct AS (
  SELECT d.node_id, x.access_path, true AS own FROM declarations d
  JOIN exported x ON x.node_id = d.node_id
  WHERE d.kind IN (0, 1) AND NOT d.is_overload),
classes AS (
  SELECT x.node_id AS class_node_id, x.access_path FROM exported x
  JOIN declarations c ON c.node_id = x.node_id AND c.kind = 2),
lineage AS (
  SELECT class_node_id, class_node_id AS ancestor_node_id, -1 AS ordinal FROM classes
  UNION ALL
  SELECT c.class_node_id, t.ancestor_node_id, a.ordinal FROM classes c
  JOIN ancestry_targets t ON t.class_node_id = c.class_node_id
  JOIN class_ancestry a ON a.fact_id = t.ancestry_fact_id AND a.relation = 1
  WHERE t.ancestor_node_id IS NOT NULL),
candidates AS (
  SELECT l.class_node_id, d.node_id, d.name, d.kind, l.ordinal FROM lineage l
  JOIN declarations d ON d.parent_node_id = l.ancestor_node_id),
nearest AS (
  SELECT class_node_id, name, min(ordinal) AS ordinal FROM candidates
  GROUP BY class_node_id, name),
methods AS (
  SELECT c.node_id, x.access_path || '.' || c.name AS access_path,
         c.ordinal = -1 AS own FROM candidates c
  JOIN nearest n ON n.class_node_id = c.class_node_id AND n.name = c.name
    AND n.ordinal = c.ordinal
  JOIN classes x ON x.class_node_id = c.class_node_id
  JOIN declarations dd ON dd.node_id = c.node_id
  WHERE c.kind IN (0, 1) AND NOT dd.is_overload
    AND (NOT starts_with(c.name, '_') OR c.name IN ('__init__', '__call__'))),
paths AS (
  SELECT node_id, access_path, bool_or(own) AS own
  FROM (SELECT * FROM direct UNION ALL SELECT * FROM methods)
  GROUP BY node_id, access_path)
SELECT encode(CAST(node_id AS BYTEA),'hex') AS node, access_path, own FROM paths
WHERE access_path LIKE '%http_app' OR access_path LIKE '%.FastMCP.tool' OR access_path LIKE '%.run' AND access_path LIKE '%FastMCP%'
ORDER BY node, access_path
