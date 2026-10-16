//! Answer-affecting implementation identity, independent of semantic completed content.
//! These values name a realization; they confer no publication, hold or admission authority.
use lctx_model::domain::{ContentHash, Key, KeySink, ModelError};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DerivationIdentity {
    pub version: u32,
    pub native_implementation: ContentHash,
    pub publication_implementation: ContentHash,
    pub model: ContentHash,
    pub physical_schema: ContentHash,
    pub definition_epoch: ContentHash,
    pub lexical_policy: ContentHash,
    pub vector_policy: ContentHash,
}
impl DerivationIdentity {
    pub fn current(
        publication_implementation: ContentHash,
        definition_epoch: ContentHash,
    ) -> Result<Self, ModelError> {
        Ok(Self {
            version: 1,
            native_implementation: implementation_identity()?,
            publication_implementation,
            model: lctx_model::domain::model()?.digest(),
            physical_schema: crate::installation::base_schema_identity(),
            definition_epoch,
            lexical_policy: crate::lexical_stats::analyzer_identity(),
            vector_policy: ContentHash::of(b"exact-eligible-cosine/v1"),
        })
    }
    pub fn identity(&self) -> Result<ContentHash, ModelError> {
        if self.version != 1 {
            return Err(ModelError::Conflict("native derivation identity version"));
        }
        let mut sink = KeySink::new("native-derivation/v1");
        self.native_implementation.encode(&mut sink);
        self.publication_implementation.encode(&mut sink);
        self.model.encode(&mut sink);
        self.physical_schema.encode(&mut sink);
        self.definition_epoch.encode(&mut sink);
        self.lexical_policy.encode(&mut sink);
        self.vector_policy.encode(&mut sink);
        Ok(sink.finish())
    }
    /// All derived addresses are qualified, including documents whose text is equal.
    /// Sharing within a named derivation remains possible without mixing live realizations.
    pub fn address(
        &self,
        table: &str,
        canonical_key: &[u8],
    ) -> Result<surrealdb::types::RecordId, ModelError> {
        let mut sink = KeySink::new("native-derived-address/v1");
        self.identity()?.encode(&mut sink);
        sink.part(b"family", table.as_bytes());
        sink.part(b"canonical-key", canonical_key);
        Ok(surrealdb::types::RecordId::new(
            table,
            format!("{}:{}", self.identity()?.hex(), sink.finish().hex()),
        ))
    }
}
/// A selected verified realization supplies this namespace. It is data, never authority.
pub fn address(namespace: ContentHash, table: &str, key: &[u8]) -> surrealdb::types::RecordId {
    let mut sink = KeySink::new("native-derived-member/v1");
    sink.part(b"family", table.as_bytes());
    sink.part(b"key", key);
    surrealdb::types::RecordId::new(
        table,
        format!("{}:{}", namespace.hex(), sink.finish().hex()),
    )
}
pub const FAMILIES: &[&str] = &[
    "participant",
    "reference",
    "search_api_options",
    "search_documentation_deployment",
    "search_scenario",
    "search_source",
    "vector",
    "lex_occurs",
    "vec_occurs",
    "lexical_document",
    "lexical_posting",
    "lexical_member",
    "lexical_term",
    "lexical_corpus",
];
pub fn require_namespace(
    row: &surrealdb::types::Object,
    namespace: ContentHash,
) -> Result<(), ModelError> {
    use surrealdb::types::Value;
    let Some(Value::RecordId(id)) = row.get("id") else {
        return Err(ModelError::Schema("derived namespace address"));
    };
    if !FAMILIES.contains(&id.table.as_str())
        || row.get("derivation") != Some(&Value::String(namespace.hex()))
    {
        return Err(ModelError::Conflict("derived realization namespace"));
    }
    Ok(())
}
pub fn implementation_identity() -> Result<ContentHash, ModelError> {
    Ok(ContentHash(
        hex::decode(env!("LCTX_NATIVE_SOURCE_DIGEST"))
            .map_err(ModelError::codec)?
            .try_into()
            .map_err(|_| ModelError::Schema("native implementation digest width"))?,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn every_answer_affecting_owner_changes_the_derived_namespace() {
        let hash = ContentHash::of(b"base");
        let base = DerivationIdentity {
            version: 1,
            native_implementation: hash,
            publication_implementation: hash,
            model: hash,
            physical_schema: hash,
            definition_epoch: hash,
            lexical_policy: hash,
            vector_policy: hash,
        };
        let expected = base.address("vector", b"same projection").unwrap();
        for field in 0..7 {
            let mut changed = base.clone();
            let replacement = ContentHash::of(b"changed");
            match field {
                0 => changed.native_implementation = replacement,
                1 => changed.publication_implementation = replacement,
                2 => changed.model = replacement,
                3 => changed.physical_schema = replacement,
                4 => changed.definition_epoch = replacement,
                5 => changed.lexical_policy = replacement,
                _ => changed.vector_policy = replacement,
            }
            assert_ne!(
                expected,
                changed.address("vector", b"same projection").unwrap()
            );
        }
        assert_ne!(
            expected,
            base.address("lexical_document", b"same projection")
                .unwrap()
        );
        assert_ne!(
            expected,
            base.address("vector", b"different projection").unwrap()
        );
        let mut invalid = base;
        invalid.version = 2;
        assert!(invalid.identity().is_err());
    }
}
