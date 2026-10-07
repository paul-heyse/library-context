//! One pure value-admission operation shared by nominal analytic and retrieval consumers.
use super::{
    DocumentRecipe, EmbeddingSpec,
    projection::{ProjectedValue, ProjectionDefinition},
    value,
};
use crate::domain::{resources::ResourceBudget, *};

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct PublishedValue {
    pub value: Id<value::FullValue>,
    pub projection: Id<ProjectedValue>,
    pub input: ContentHash,
    pub tokens: u32,
}
pub struct SelectedConsumption<'a> {
    pub encoder: &'a EmbeddingSpec,
    pub document: &'a DocumentRecipe,
    pub projection: &'a ProjectionDefinition,
}
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct FullIdentity {
    pub encoder: Id<EmbeddingSpec>,
    pub input: ContentHash,
    pub dimensions: i64,
    pub tokens: i64,
    pub digest: ContentHash,
}
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct ProjectionIdentity {
    pub value: Id<value::FullValue>,
    pub definition: Id<ProjectionDefinition>,
    pub source_digest: ContentHash,
    pub dimensions: i64,
    pub digest: ContentHash,
}
impl HeapSize for FullIdentity {
    fn heap_bytes(&self) -> usize {
        0
    }
}
impl HeapSize for ProjectionIdentity {
    fn heap_bytes(&self) -> usize {
        0
    }
}
/// Admission retains compact identities, never all full payloads merely to check later uses.
pub struct ValueIndex {
    full: charged::ChargedMap<Id<value::FullValue>, FullIdentity>,
    projected: charged::ChargedMap<Id<ProjectedValue>, (ProjectionIdentity, bool)>,
    charge: charged::StateCharge,
}
impl ValueIndex {
    pub fn new(b: &ResourceBudget) -> Self {
        Self {
            full: Default::default(),
            projected: Default::default(),
            charge: charged::StateCharge::new(b, "canonical-embedding-value-identities"),
        }
    }
    pub fn admit_full(
        &mut self,
        row: &value::FullValue,
        encoder: &EmbeddingSpec,
        policy: &ProjectionDefinition,
    ) -> Result<(), ModelError> {
        row.verify_encoder(encoder)?;
        let identity = FullIdentity {
            encoder: row.encoder,
            input: row.input,
            dimensions: row.dimensions,
            tokens: row.tokens,
            digest: row.digest,
        };
        if self.full.get(&row.id()).is_some_and(|old| *old != identity) {
            return Err(ModelError::Invalid(
                "conflicting immutable full winners".into(),
            ));
        }
        let expected = ProjectedValue::new(row, policy)?;
        let projection = ProjectionIdentity {
            value: row.id(),
            definition: policy.id(),
            source_digest: row.digest,
            dimensions: policy.dimensions,
            digest: expected.digest,
        };
        if self
            .projected
            .get(&expected.id())
            .is_some_and(|(old, _)| *old != projection)
        {
            return Err(ModelError::Invalid(
                "conflicting projection identities".into(),
            ));
        }
        self.full.insert(&mut self.charge, row.id(), identity)?;
        if !self.projected.contains_key(&expected.id()) {
            self.projected
                .insert(&mut self.charge, expected.id(), (projection, false))?;
        }
        Ok(())
    }
    pub fn admit_projection(&mut self, row: &ProjectedValue) -> Result<(), ModelError> {
        row.validate()?;
        let expected = self
            .projected
            .get(&row.id())
            .copied()
            .ok_or_else(|| {
                ModelError::Invalid("projection missing canonical full value/policy".into())
            })?
            .0;
        if expected
            != (ProjectionIdentity {
                value: row.value,
                definition: row.definition,
                source_digest: row.source_digest,
                dimensions: row.dimensions,
                digest: row.digest,
            })
        {
            return Err(ModelError::Invalid("projection derivation mismatch".into()));
        }
        self.projected
            .insert(&mut self.charge, row.id(), (expected, true))?;
        Ok(())
    }
    pub fn full(&self, id: Id<value::FullValue>) -> Result<FullIdentity, ModelError> {
        self.full
            .get(&id)
            .copied()
            .ok_or_else(|| ModelError::Invalid("missing canonical full winner".into()))
    }
    pub fn projected(&self, id: Id<ProjectedValue>) -> Result<ProjectionIdentity, ModelError> {
        self.projected
            .get(&id)
            .filter(|(_, seen)| *seen)
            .map(|(r, _)| *r)
            .ok_or_else(|| ModelError::Invalid("missing canonical derived projection".into()))
    }
    pub fn verify_use(
        &self,
        encoder: Id<EmbeddingSpec>,
        input: ContentHash,
        tokens: i64,
        full: Id<value::FullValue>,
        projected: Id<ProjectedValue>,
        policy: Id<ProjectionDefinition>,
    ) -> Result<(), ModelError> {
        let f = self.full(full)?;
        let p = self.projected(projected)?;
        if f.encoder != encoder
            || f.input != input
            || f.tokens != tokens
            || p.value != full
            || p.definition != policy
            || p.source_digest != f.digest
        {
            return Err(ModelError::Invalid(
                "canonical embedding consumption reference mismatch".into(),
            ));
        }
        Ok(())
    }
}
