//! Immutable completed compiler inputs. No database grants or physical names participate.
use super::invalid;
use crate::domain::*;
use std::marker::PhantomData;

/// Metadata from a locally validated immutable producer stream. The compiler supplies this only
/// after successful completion; these values describe inputs, not a capability to read a store.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SourceSnapshot {
    pub(crate) relation: String,
    pub(crate) producer: String,
    pub(crate) model: ContentHash,
    pub(crate) implementation: ContentHash,
    pub(crate) content: ContentHash,
    pub(crate) rows: i64,
}
impl HeapSize for SourceSnapshot {
    fn heap_bytes(&self) -> usize { self.relation.heap_bytes() + self.producer.heap_bytes() }
}
impl SourceSnapshot {
    pub fn of_relation(relation: &Relation, producer: impl Into<String>, model: ContentHash,
                       implementation: ContentHash, content: ContentHash, rows: u64) -> Result<Self, ModelError> {
        let producer=producer.into();
        if producer.is_empty() {return Err(invalid("completed input needs a producer"));}
        Ok(Self {relation:relation.name().into(),producer,model,implementation,content,
                 rows:i64::try_from(rows).map_err(|_|invalid("completed input row count exceeds representation"))?})
    }

    pub fn producer(&self) -> &str { &self.producer }
    pub fn implementation(&self) -> ContentHash { self.implementation }
    pub fn relation(&self) -> &str { &self.relation }
    pub fn rows(&self) -> i64 { self.rows }
    pub fn content(&self) -> ContentHash { self.content }
    pub fn model(&self) -> ContentHash { self.model }
    fn encode(&self, sink: &mut KeySink) {
        self.relation.encode(sink);
        self.producer.encode(sink);
        self.model.encode(sink);
        self.implementation.encode(sink);
        self.content.encode(sink);
        self.rows.encode(sink);
    }
}
/// Nominal view metadata; actual Arrow streams and segment lifetimes belong to the compiler.
#[derive(Debug, Clone)]
pub struct CompletedInput<R> { source: SourceSnapshot, marker: PhantomData<fn() -> R> }
impl<R: Record> CompletedInput<R> {
    pub fn new(producer: impl Into<String>, model: ContentHash, implementation: ContentHash,
               content: ContentHash, rows: u64) -> Result<Self, ModelError> {
        let producer = producer.into();
        if producer.is_empty() { return Err(invalid("completed input needs a producer")); }
        Ok(Self { source: SourceSnapshot { relation: R::NAME.into(), producer, model,
            implementation, content, rows: i64::try_from(rows).map_err(|_| invalid("completed input row count exceeds representation"))? }, marker: PhantomData })
    }
    pub fn source(&self) -> &SourceSnapshot { &self.source }
    pub fn snapshot(&self) -> SourceSnapshot { self.source.clone() }
}
pub(crate) fn digest(sources: &std::collections::BTreeMap<String, SourceSnapshot>) -> ContentHash {
    let mut sink = KeySink::new("analysis-completed-inputs/v3");
    for source in sources.values() { source.encode(&mut sink); }
    sink.finish()
}
/// Exact explicitly supplied completed predecessors. A capture is independent of publication,
/// attempts, schedules, persisted checkpoints and vocabulary epochs.
pub struct CapturedSources {
    charge: charged::StateCharge,
    model: Option<ContentHash>,
    profile: Option<stages::Profile>,
    sources: charged::ChargedMap<String, SourceSnapshot>,
}
impl CapturedSources {
    pub fn new(budget: &resources::ResourceBudget) -> Self {
        Self { charge: charged::StateCharge::new(budget, "analysis_source_capture"),
               model: None, profile: None, sources: Default::default() }
    }
    pub fn capture(profile: stages::Profile, sources: impl IntoIterator<Item=SourceSnapshot>,
                   budget: &resources::ResourceBudget) -> Result<Self, ModelError> {
        let mut capture = Self::new(budget);
        capture.profile = Some(profile);
        for source in sources { capture.insert(source)?; }
        Ok(capture)
    }
    pub fn include<R: Record>(&mut self, input: &CompletedInput<R>) -> Result<(), ModelError> {
        self.insert(input.snapshot())
    }
    fn insert(&mut self, source: SourceSnapshot) -> Result<(), ModelError> {
        if source.relation.is_empty() || source.producer.is_empty() || source.rows < 0 {
            return Err(invalid("completed input has invalid metadata"));
        }
        if self.model.is_some_and(|model| model != source.model) {
            return Err(invalid("completed inputs have different semantic contracts"));
        }
        if let Some(existing) = self.sources.get(&source.relation) {
            if *existing != source { return Err(invalid("analysis reads conflicting completed views of one relation")); }
            return Ok(());
        }
        self.model = Some(source.model);
        self.sources.insert(&mut self.charge, source.relation.clone(), source)?;
        Ok(())
    }
    pub fn digest(&self) -> ContentHash { digest(&self.sources) }
    pub fn iter(&self) -> impl Iterator<Item=&SourceSnapshot> { self.sources.values() }
    pub fn is_empty(&self) -> bool { self.sources.is_empty() }
    pub fn profile(&self) -> Option<stages::Profile> { self.profile }
    pub(crate) fn has_relation(&self, relation: &str) -> bool { self.sources.contains_key(relation) }
    pub(crate) fn accepts<R: Record>(&self, input: &CompletedInput<R>) -> Result<(), ModelError> {
        if self.sources.get(R::NAME) != Some(input.source()) {
            return Err(invalid("coverage read differs from captured completed input"));
        }
        Ok(())
    }
}
/// Exact comparison against effect-owner acknowledged source metadata. The effect owner verifies
/// authority before supplying these snapshots; metadata cannot create runtime read grants.
pub(crate) fn verify(
    snapshots: &std::collections::BTreeMap<String, SourceSnapshot>,
    actual: &[SourceSnapshot],
    budget: &resources::ResourceBudget,
) -> Result<(), ModelError> {
    let mut charge = charged::StateCharge::new(budget, "analysis_source_verify");
    let mut expected = charged::ChargedMap::default();
    for source in actual {
        let snapshot = source.clone();
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

