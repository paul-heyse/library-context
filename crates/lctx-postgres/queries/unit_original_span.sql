SELECT s.span_id FROM lctx_serving.catalog_evidence e
JOIN lctx_serving.catalog_artifacts a ON a.generation_digest=e.generation_digest AND a.path=e.path AND a.source_digest=e.source_digest
JOIN lctx_serving.catalog_spans s ON s.generation_digest=a.generation_digest AND s.artifact_id=a.artifact_id AND s.start_byte=e.start_byte AND s.end_byte=e.end_byte
WHERE e.generation_digest=$1 AND e.evidence_id=$2 ORDER BY s.span_id LIMIT 2
