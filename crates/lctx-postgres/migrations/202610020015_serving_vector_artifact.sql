-- Disposable numerical realization only. Semantic rows remain in canonical generation schemas.
CREATE TABLE lctx_cache.serving_vector_artifacts (
    artifact_key bytea PRIMARY KEY CHECK (octet_length(artifact_key)=32),
    generation_id bytea NOT NULL CHECK (octet_length(generation_id)=16),
    model_digest bytea NOT NULL CHECK (octet_length(model_digest)=32),
    source_content bytea NOT NULL CHECK (octet_length(source_content)=32),
    spec_hash bytea NOT NULL CHECK (octet_length(spec_hash)=32),
    codec smallint NOT NULL CHECK (codec=1),
    physical_digest bytea NOT NULL CHECK (octet_length(physical_digest)=32),
    row_count bigint NOT NULL CHECK (row_count>=0)
);
CREATE TABLE lctx_cache.serving_vectors (
    artifact_key bytea NOT NULL REFERENCES lctx_cache.serving_vector_artifacts(artifact_key) ON DELETE CASCADE,
    retrieval_use bytea NOT NULL CHECK (octet_length(retrieval_use)=16),
    value lctx_ext.vector(1024) NOT NULL,
    PRIMARY KEY(artifact_key,retrieval_use)
);
CREATE INDEX serving_vector_generation ON lctx_cache.serving_vector_artifacts(generation_id);
GRANT USAGE ON SCHEMA lctx_cache TO lctx_serving;
GRANT SELECT ON lctx_cache.serving_vector_artifacts,lctx_cache.serving_vectors TO lctx_serving;
-- The migrator owns explicit preparation and retirement effects. Neither serving startup nor
-- generation importers acquire cache write privileges. There is no FK into a generation schema.
