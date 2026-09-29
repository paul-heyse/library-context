SELECT artifact_id,release_id,alignment
FROM lctx_serving.catalog_artifacts
WHERE generation_digest=$1 ORDER BY artifact_id LIMIT 200001
