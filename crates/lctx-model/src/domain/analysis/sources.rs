//! Source snapshots carry audit metadata only. Actual R0 grants remain the sole read authority.
use super::invalid;
use crate::domain::{
    stages::{CompletedRelation, ReadPermit, StageAccess, StageIdentity},
    *,
};
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceSnapshot {
    pub(crate) relation: String,
    pub(crate) producer: String,
    pub(crate) model: ContentHash,
    pub(crate) schedule: ContentHash,
    pub(crate) content: ContentHash,
    pub(crate) rows: i64,
    pub(crate) physical: String,
    pub(crate) prefix: Option<String>,
}
impl HeapSize for SourceSnapshot {
    fn heap_bytes(&self) -> usize {
        self.relation.heap_bytes()
            + self.producer.heap_bytes()
            + self.physical.heap_bytes()
            + self.prefix.heap_bytes()
    }
}
impl SourceSnapshot {
    fn from_source(source: &CompletedRelation) -> Result<Self, ModelError> {
        Ok(Self {
            relation: source.relation().to_owned(),
            producer: source.producer().to_owned(),
            model: source.model(),
            schedule: source.schedule(),
            content: source.receipt().content,
            rows: source
                .receipt()
                .rows
                .try_into()
                .map_err(|_| invalid("source row count exceeds signed store representation"))?,
            physical: source.physical_relation(),
            prefix: source.prefix().map(|prefix| format!("{prefix:?}")),
        })
    }
    pub fn relation(&self) -> &str {
        &self.relation
    }
    pub fn rows(&self) -> i64 {
        self.rows
    }
    pub fn content(&self) -> ContentHash {
        self.content
    }
    pub fn model(&self) -> ContentHash {
        self.model
    }
    fn encode(&self, sink: &mut KeySink) {
        self.relation.encode(sink);
        self.producer.encode(sink);
        self.model.encode(sink);
        self.schedule.encode(sink);
        self.content.encode(sink);
        self.rows.encode(sink);
        self.physical.encode(sink);
        self.prefix.encode(sink);
    }
}
pub(crate) fn digest(sources: &std::collections::BTreeMap<String, SourceSnapshot>) -> ContentHash {
    let mut sink = KeySink::new("analysis-completed-inputs");
    for source in sources.values() {
        source.encode(&mut sink);
    }
    sink.finish()
}
/// Constructed from declared typed read permits or the access's complete acknowledged sources.
/// No constructor accepts a producer-selected relation, model or completion string.
pub struct CapturedSources {
    charge: charged::StateCharge,
    frame: Option<(StageIdentity, ContentHash)>,
    profile: Option<stages::Profile>,
    sources: charged::ChargedMap<String, SourceSnapshot>,
}
impl CapturedSources {
    pub fn new(budget: &resources::ResourceBudget) -> Self {
        Self {
            charge: charged::StateCharge::new(budget, "analysis_source_capture"),
            frame: None,
            profile: None,
            sources: Default::default(),
        }
    }
    pub fn capture(
        access: &StageAccess<'_, '_>,
        budget: &resources::ResourceBudget,
    ) -> Result<Self, ModelError> {
        let mut captured = Self::new(budget);
        captured.profile = Some(access.profile());
        let _buffer = budget.reserve(
            "analysis_source_capture",
            access
                .stage()
                .inputs
                .len()
                .checked_mul(size_of::<CompletedRelation>())
                .ok_or_else(|| invalid("source capture allocation overflow"))?,
        )?;
        let identity = access.identity();
        for source in access.completed_sources()? {
            if source.identity().attempt() != identity.attempt() {
                return Err(invalid("source capture crosses attempt"));
            }
            captured.insert(SourceSnapshot::from_source(&source)?)?;
        }
        if let Some(source) = captured.sources.values().next() {
            captured.frame = Some((identity, source.model));
        }
        Ok(captured)
    }
    pub fn include<R: Record>(&mut self, permit: &ReadPermit<'_, R>) -> Result<(), ModelError> {
        let source = permit
            .source()
            .ok_or_else(|| invalid("source read has no acknowledged completion"))?;
        let frame = (permit.identity(), permit.model());
        if self.frame.is_some_and(|existing| existing != frame)
            || source.relation() != R::NAME
            || source.model() != permit.model()
            || source.identity().attempt() != permit.identity().attempt()
        {
            return Err(invalid(
                "source capture crosses consumer, model or nominal relation",
            ));
        }
        self.frame = Some(frame);
        self.insert(SourceSnapshot::from_source(source)?)
    }
    fn insert(&mut self, source: SourceSnapshot) -> Result<(), ModelError> {
        if let Some(existing) = self.sources.get(&source.relation) {
            if *existing != source {
                return Err(invalid(
                    "analysis reads conflicting snapshots of one relation",
                ));
            }
            return Ok(());
        }
        self.sources
            .insert(&mut self.charge, source.relation.clone(), source)?;
        Ok(())
    }
    pub fn digest(&self) -> ContentHash {
        digest(&self.sources)
    }
    pub fn iter(&self) -> impl Iterator<Item = &SourceSnapshot> {
        self.sources.values()
    }
    pub fn is_empty(&self) -> bool {
        self.sources.is_empty()
    }
    pub fn profile(&self) -> Option<stages::Profile> {
        self.profile
    }
    pub(crate) fn has_relation(&self, relation: &str) -> bool {
        self.sources.contains_key(relation)
    }
    pub(crate) fn accepts<R: Record>(&self, permit: &ReadPermit<'_, R>) -> Result<(), ModelError> {
        let source = permit
            .source()
            .ok_or_else(|| invalid("coverage input has no completed source"))?;
        if self.frame != Some((permit.identity(), permit.model()))
            || self.sources.get(R::NAME) != Some(&SourceSnapshot::from_source(source)?)
        {
            return Err(invalid("coverage read differs from captured source frame"));
        }
        Ok(())
    }
}
/// Exact comparison against acknowledged effect-owner sources. Copying this metadata cannot grant
/// authority, shrink the required domain, choose a different prefix, or change a source's payload.
pub(crate) fn verify(
    snapshots: &std::collections::BTreeMap<String, SourceSnapshot>,
    actual: &[CompletedRelation],
    budget: &resources::ResourceBudget,
) -> Result<(), ModelError> {
    let mut charge = charged::StateCharge::new(budget, "analysis_source_verify");
    let mut expected = charged::ChargedMap::default();
    for source in actual {
        let snapshot = SourceSnapshot::from_source(source)?;
        if let Some(previous) = expected.get(&snapshot.relation) {
            if *previous != snapshot {
                return Err(invalid("publication has conflicting source snapshots"));
            }
        } else {
            expected.insert(&mut charge, snapshot.relation.clone(), snapshot)?;
        }
    }
    if &*expected != snapshots {
        return Err(invalid(
            "analysis source receipts differ from exact acknowledged inputs",
        ));
    }
    Ok(())
}

pub(crate) fn empty_digest() -> ContentHash {
    digest(&std::collections::BTreeMap::new())
}
