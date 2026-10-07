//! Canonical retrieval preparation. Renderings aid discovery; anchors retain semantic authority.
pub mod build;
pub mod consumption;
pub(crate) mod inventory;
pub mod source;
pub mod partition;
mod construction;
use crate::domain::{
    attribution::AnalysisContext,
    catalog::{self, evidence as c1},
    documents::*,
    input::*,
    *,
};
use crate::{Domain, DomainCode, DomainSum};
pub const RENDER_VERSION: i64 = 3;
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, DomainCode)]
#[repr(i16)]
pub enum Family {
    ApiOptions = 0,
    DocumentationDeployment = 1,
    Scenario = 2,
    Source = 3,
}
#[derive(Debug, Clone, PartialEq, Eq, Domain, serde::Serialize, serde::Deserialize)]
#[model(name="retrieval_definitions",validate=validate_definition)]
pub struct RetrievalDefinition {
    #[model(key)]
    pub rendering_version: i64,
    #[model(key)]
    pub preferred_tokens: i64,
    #[model(key)]
    pub hard_tokens: i64,
    #[model(key)]
    pub embedding_requested: bool,
}
impl RetrievalDefinition {
    pub fn builtin(embedding_requested: bool) -> Self {
        Self {
            rendering_version: RENDER_VERSION,
            preferred_tokens: 1024,
            hard_tokens: 2048,
            embedding_requested,
        }
    }
}
fn validate_definition(r: &RetrievalDefinition) -> Result<(), ModelError> {
    if r.rendering_version != RENDER_VERSION || r.preferred_tokens != 1024 || r.hard_tokens != 2048 {
        return Err(build::invalid(
            "unsupported retrieval renderer or semantic token policy",
        ));
    }
    Ok(())
}
#[derive(Debug, Clone, PartialEq, Eq, Hash, DomainSum, serde::Serialize, serde::Deserialize)]
#[model(name = "retrieval_origins")]
pub enum Origin {
    #[model(code = 0)]
    Api { member: Id<catalog::CatalogMember> },
    #[model(code = 1)]
    Original { source: Id<c1::OriginalSource> },
    #[model(code = 2)]
    Scenario { scenario: Id<c1::CatalogScenario> },
    #[model(code = 3)]
    Document {
        observation: Id<DocumentObservation>,
    },
    #[model(code = 4)]
    Passage { observation: Id<PassageObservation> },
    #[model(code = 5)]
    Deployment {
        deployment: Id<c1::CatalogDeployment>,
    },
    #[model(code = 6)]
    Brief {
        brief: Id<crate::domain::synthesis::briefs::Brief>,
    },
    #[model(code = 7)]
    Definition { member: Id<catalog::CatalogMember>, entity: Id<normalized::entities::EntityRef> },
    #[model(code = 8)]
    Option { option: Id<catalog::CatalogOption> },
    #[model(code = 9)]
    Release { release: Id<Release> },
    #[model(code = 10)]
    UnavailableDefinition { member: Id<catalog::CatalogMember> },
    #[model(code = 11)]
    Source { artifact: Id<crate::domain::source::SourceArtifact> },
}
/// Exact text/family deduplication does not discard any unit's contextual occurrence.
#[derive(Debug, Clone, PartialEq, Eq, Domain, serde::Serialize, serde::Deserialize)]
#[model(name="retrieval_corpus_texts",validate=validate_text)]
pub struct CorpusText {
    #[model(key)]
    pub family: Family,
    #[model(key)]
    pub rendering_version: i64,
    #[model(key)]
    pub digest: ContentHash,
    pub text: Utf8Text,
}
fn validate_text(r: &CorpusText) -> Result<(), ModelError> {
    if r.rendering_version != RENDER_VERSION
        || r.digest != ContentHash::of(r.text.as_str().as_bytes())
    {
        return Err(build::invalid(
            "retrieval corpus text digest/version differs",
        ));
    }
    Ok(())
}
#[derive(Debug, Clone, PartialEq, Eq, Domain, serde::Serialize, serde::Deserialize)]
#[model(name="retrieval_units",invariant_refs=build::invariants_refs)]
pub struct Unit {
    #[model(key)]
    pub input: Id<InputRevision>,
    #[model(key)]
    pub context: Id<AnalysisContext>,
    #[model(key)]
    pub family: Family,
    #[model(key)]
    pub origin: Id<Origin>,
    pub corpus: Id<CorpusText>,
    pub title: Utf8Text,
}
#[derive(Debug, Clone, PartialEq, Eq, Hash, DomainSum, serde::Serialize, serde::Deserialize)]
#[model(name = "retrieval_subjects")]
pub enum Subject {
    #[model(code = 0)]
    Member { member: Id<catalog::CatalogMember> },
    #[model(code = 1)]
    Release { release: Id<Release> },
    #[model(code = 2)]
    Option { option: Id<catalog::CatalogOption> },
    #[model(code = 3)]
    Definition { entity: Id<normalized::entities::EntityRef> },
    #[model(code = 4)]
    Source { artifact: Id<crate::domain::source::SourceArtifact> },
}
#[derive(Debug, Clone, PartialEq, Eq, Domain, serde::Serialize, serde::Deserialize)]
#[model(name = "retrieval_unit_subjects")]
pub struct UnitSubject {
    #[model(key)]
    pub unit: Id<Unit>,
    #[model(key)]
    pub subject: Id<Subject>,
}
/// Original references are canonical C1 wrappers, never a duplicate body or inferred API proof.
#[derive(Debug, Clone, PartialEq, Eq, Hash, DomainSum, serde::Serialize, serde::Deserialize)]
#[model(name = "retrieval_anchor_sources")]
pub enum AnchorSource {
    #[model(code = 0)]
    Original { source: Id<c1::OriginalSource> },
    #[model(code = 1)]
    Span {
        span: assertion::EvidenceSourceSpanId,
    },
    #[model(code = 2)]
    Artifact {
        artifact: Id<crate::domain::source::SourceArtifact>,
    },
    #[model(code = 3)]
    Prose {
        slice: Id<crate::domain::synthesis::documentary::ProseSlice>,
    },
    #[model(code = 4)]
    Occurrence { occurrence: Id<crate::domain::source::Occurrence> },
    #[model(code = 5)]
    OccurrenceSlice { occurrence: Id<crate::domain::source::Occurrence>, start:i64, end:i64 },
}
#[derive(Debug, Clone, PartialEq, Eq, Domain, serde::Serialize, serde::Deserialize)]
#[model(name = "retrieval_original_anchors")]
pub struct OriginalAnchor {
    #[model(key)]
    pub unit: Id<Unit>,
    #[model(key)]
    pub ordinal: i64,
    #[model(key)]
    pub original: Id<AnchorSource>,
}
#[derive(Debug, Clone, PartialEq, Eq, Domain, serde::Serialize, serde::Deserialize)]
#[model(name = "retrieval_unit_roots")]
pub struct UnitRoot {
    #[model(key)]
    pub unit: Id<Unit>,
    #[model(key)]
    pub root: Id<c1::EvidenceRoot>,
}
/// Content roles are semantic: context never nominates an applicable target.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, DomainCode)]
#[repr(i16)]
pub enum PartPurpose { Primary = 0, Context = 1 }
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, DomainCode)]
#[repr(i16)]
pub enum WindowAvailability { Ready = 0, LexicalOnly = 1, TokenizerUnavailable = 2 }
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, DomainCode)]
#[repr(i16)]
pub enum BindingBasis { DirectDefinition = 0, PublicContract = 1, ScenarioResolved = 2, ScenarioCandidate = 3, DocumentCandidate = 4, DeclaredRelease = 5, Option = 6, DefinitionCandidate = 7, Source = 8 }
#[derive(Debug, Clone, PartialEq, Eq, Domain, serde::Serialize, serde::Deserialize)]
#[model(name="retrieval_content_parts",validate=validate_part)]
pub struct ContentPart {
    #[model(key)] pub unit: Id<Unit>,
    #[model(key)] pub ordinal: i64,
    pub purpose: PartPurpose,
    pub scope: Option<Id<Subject>>,
    pub qualification: Option<Id<assertion::AssertionQualification>>,
    pub digest: ContentHash,
    pub text: Utf8Text,
}
fn validate_part(r: &ContentPart) -> Result<(),ModelError> {
    if r.ordinal < 0 || r.text.is_empty() || r.digest != ContentHash::of(r.text.as_str().as_bytes()) { return Err(build::invalid("invalid retrieval content part")); } Ok(())
}
/// None is an explicit synthetic range, never an inferred original span.
#[derive(Debug, Clone, PartialEq, Eq, Domain, serde::Serialize, serde::Deserialize)]
#[model(name="retrieval_part_source_maps",validate=validate_part_map)]
pub struct PartSourceMap {
    #[model(key)] pub part: Id<ContentPart>,
    #[model(key)] pub ordinal: i64,
    pub start: i64, pub end: i64,
    pub original: Option<Id<AnchorSource>>,
    pub original_start: Option<i64>, pub original_end: Option<i64>,
}
fn valid_map(start:i64,end:i64,original:Option<Id<AnchorSource>>,os:Option<i64>,oe:Option<i64>) -> bool {
    start >= 0 && end > start && match (original,os,oe) { (None,None,None) => true, (Some(_),Some(a),Some(b)) => a>=0 && b-a == end-start, _=>false }
}
fn validate_part_map(r:&PartSourceMap)->Result<(),ModelError>{ if r.ordinal<0 || !valid_map(r.start,r.end,r.original,r.original_start,r.original_end){return Err(build::invalid("invalid retrieval part source map"));} Ok(()) }
#[derive(Debug, Clone, PartialEq, Eq, Domain, serde::Serialize, serde::Deserialize)]
#[model(name="retrieval_search_windows",validate=validate_window)]
pub struct SearchWindow {
    #[model(key)] pub definition: Id<Definition>,
    #[model(key)] pub unit: Id<Unit>,
    #[model(key)] pub ordinal: i64,
    pub corpus: Id<CorpusText>,
    pub digest: ContentHash,
    pub text: Utf8Text,
    pub input_text: Utf8Text,
    #[model(key)] pub tokenizer: Option<ContentHash>,
    #[model(key)] pub encoded_digest: ContentHash,
    pub tokens: Option<i64>,
    pub availability: WindowAvailability,
}
fn validate_window(r:&SearchWindow)->Result<(),ModelError>{
    if r.ordinal<0 || r.text.is_empty() || r.digest != ContentHash::of(r.text.as_str().as_bytes()) || r.encoded_digest != embedding::value::input_hash(r.input_text.as_str()) || !match r.availability { WindowAvailability::Ready=>r.tokenizer.is_some() && r.tokens.is_some_and(|n| n>0 && n<=2048), WindowAvailability::LexicalOnly=>r.tokenizer.is_some() && r.tokens.is_some_and(|n|n>2048), WindowAvailability::TokenizerUnavailable=>r.tokenizer.is_none() && r.tokens.is_none() } {return Err(build::invalid("invalid semantic search window"));} Ok(())
}
#[derive(Debug, Clone, PartialEq, Eq, Domain, serde::Serialize, serde::Deserialize)]
#[model(name="retrieval_window_parts")]
pub struct WindowPart {
    #[model(key)] pub window: Id<SearchWindow>,
    #[model(key)] pub ordinal: i64,
    pub part: Id<ContentPart>,
    pub start: i64, pub end: i64,
}
#[derive(Debug, Clone, PartialEq, Eq, Domain, serde::Serialize, serde::Deserialize)]
#[model(name="retrieval_window_source_maps",validate=validate_window_map)]
pub struct WindowSourceMap {
    #[model(key)] pub window: Id<SearchWindow>,
    #[model(key)] pub ordinal: i64,
    pub start: i64, pub end: i64,
    pub part: Option<Id<ContentPart>>,
    pub original: Option<Id<AnchorSource>>,
    pub original_start: Option<i64>, pub original_end: Option<i64>,
}
fn validate_window_map(r:&WindowSourceMap)->Result<(),ModelError>{if r.ordinal<0 || !valid_map(r.start,r.end,r.original,r.original_start,r.original_end){return Err(build::invalid("invalid retrieval window source map"));}Ok(())}
#[derive(Debug, Clone, PartialEq, Eq, Domain, serde::Serialize, serde::Deserialize)]
#[model(name="retrieval_window_bindings")]
pub struct WindowBinding {
    #[model(key)] pub window: Id<SearchWindow>,
    #[model(key)] pub part: Id<ContentPart>,
    #[model(key)] pub subject: Id<Subject>,
    #[model(key)] pub basis: BindingBasis,
    #[model(key)] pub qualification: Option<Id<assertion::AssertionQualification>>,
}
pub fn definition_relations() -> Vec<Relation> {
    vec![Relation::of::<RetrievalDefinition>()]
}
pub fn rendering_relations() -> Vec<Relation> {
    macro_rules! output {($($f:ident:$ty:ty,)*)=>{vec![$(Relation::of::<$ty>()),*]};}
    crate::retrieval_outputs!(output)
}
pub fn relations() -> Vec<Relation> {
    let mut r = rendering_relations();
    r.extend(consumption::relations());
    r
}

pub type Definition = RetrievalDefinition;
