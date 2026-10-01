//! Canonical retrieval preparation. Renderings aid discovery; anchors retain semantic authority.
pub mod build;
pub mod consumption;
mod inventory;
pub mod source;
use crate::domain::{catalog::{self,evidence as c1},input::*,attribution::AnalysisContext,documents::*,*};
use crate::{Domain,DomainCode,DomainSum};
pub const RENDER_VERSION:i64=1;
#[derive(Debug,Clone,Copy,PartialEq,Eq,Hash,DomainCode)]
#[repr(i16)]
pub enum Family {ApiOptions=0,DocumentationDeployment=1,Scenario=2,Source=3}
#[derive(Debug,Clone,PartialEq,Eq,Domain)]
#[model(name="retrieval_definitions",validate=validate_definition)]
pub struct RetrievalDefinition {
 #[model(key)] pub rendering_version:i64,
 #[model(key)] pub fragment_bytes:i64,
 #[model(key)] pub embedding_requested:bool,
}
impl RetrievalDefinition {pub fn builtin(embedding_requested:bool)->Self {Self {rendering_version:RENDER_VERSION,fragment_bytes:4096,embedding_requested}}}
fn validate_definition(r:&RetrievalDefinition)->Result<(),ModelError> {if r.rendering_version!=RENDER_VERSION || !(4..=65536).contains(&r.fragment_bytes) {return Err(build::invalid("unsupported retrieval renderer or fragment window"));}Ok(())}
#[derive(Debug,Clone,PartialEq,Eq,Hash,DomainSum)]
#[model(name="retrieval_origins")]
pub enum Origin {
 #[model(code=0)] Api {member:Id<catalog::CatalogMember>},
 #[model(code=1)] Original {source:Id<c1::OriginalSource>},
 #[model(code=2)] Scenario {scenario:Id<c1::CatalogScenario>},
 #[model(code=3)] Document {observation:Id<DocumentObservation>},
 #[model(code=4)] Passage {observation:Id<PassageObservation>},
 #[model(code=5)] Deployment {deployment:Id<c1::CatalogDeployment>},
 #[model(code=6)] Brief {brief:Id<crate::domain::synthesis::briefs::Brief>},
}
/// Exact text/family deduplication does not discard any unit's contextual occurrence.
#[derive(Debug,Clone,PartialEq,Eq,Domain)]
#[model(name="retrieval_corpus_texts",validate=validate_text)]
pub struct CorpusText {
 #[model(key)] pub family:Family,
 #[model(key)] pub rendering_version:i64,
 #[model(key)] pub digest:ContentHash,
 pub text:Utf8Text,
}
fn validate_text(r:&CorpusText)->Result<(),ModelError> {if r.rendering_version!=RENDER_VERSION || r.digest!=ContentHash::of(r.text.as_str().as_bytes()) {return Err(build::invalid("retrieval corpus text digest/version differs"));}Ok(())}
#[derive(Debug,Clone,PartialEq,Eq,Domain)]
#[model(name="retrieval_units",invariants=build::invariants,semantic_source=include_bytes!("build.rs"))]
pub struct Unit {
 #[model(key)] pub input:Id<InputRevision>,
 #[model(key)] pub context:Id<AnalysisContext>,
 #[model(key)] pub family:Family,
 #[model(key)] pub origin:Id<Origin>,
 pub corpus:Id<CorpusText>,
 pub title:Utf8Text,
}
#[derive(Debug,Clone,PartialEq,Eq,Hash,DomainSum)]
#[model(name="retrieval_subjects")]
pub enum Subject {
 #[model(code=0)] Member {member:Id<catalog::CatalogMember>},
 #[model(code=1)] Release {release:Id<Release>},
}
#[derive(Debug,Clone,PartialEq,Eq,Domain)]
#[model(name="retrieval_unit_subjects")]
pub struct UnitSubject {#[model(key)]pub unit:Id<Unit>,#[model(key)]pub subject:Id<Subject>}
/// Original references are canonical C1 wrappers, never a duplicate body or inferred API proof.
#[derive(Debug,Clone,PartialEq,Eq,Hash,DomainSum)]
#[model(name="retrieval_anchor_sources")]
pub enum AnchorSource {
 #[model(code=0)] Original {source:Id<c1::OriginalSource>},
 #[model(code=1)] Span {span:assertion::EvidenceSourceSpanId},
 #[model(code=2)] Artifact {artifact:Id<crate::domain::source::SourceArtifact>},
 #[model(code=3)] Prose {slice:Id<crate::domain::synthesis::documentary::ProseSlice>},
}
#[derive(Debug,Clone,PartialEq,Eq,Domain)]
#[model(name="retrieval_original_anchors")]
pub struct OriginalAnchor {#[model(key)]pub unit:Id<Unit>,#[model(key)]pub ordinal:i64,#[model(key)]pub original:Id<AnchorSource>}
#[derive(Debug,Clone,PartialEq,Eq,Domain)]
#[model(name="retrieval_unit_roots")]
pub struct UnitRoot {#[model(key)]pub unit:Id<Unit>,#[model(key)]pub root:Id<c1::EvidenceRoot>}
#[derive(Debug,Clone,PartialEq,Eq,Domain)]
#[model(name="retrieval_fragments",validate=validate_fragment)]
pub struct Fragment {
 pub definition:Id<Definition>,
 #[model(key)]pub fragment_bytes:i64,
 #[model(key)]pub corpus:Id<CorpusText>,
 #[model(key)]pub ordinal:i64,
 pub start:i64,pub end:i64,pub digest:ContentHash,pub text:Utf8Text,
}
fn validate_fragment(r:&Fragment)->Result<(),ModelError> {if !(4..=65536).contains(&r.fragment_bytes) || r.ordinal<0 || r.start<0 || r.end<=r.start || (r.end-r.start) as usize!=r.text.as_str().len() || r.digest!=ContentHash::of(r.text.as_str().as_bytes()) {return Err(build::invalid("invalid retrieval fragment coordinates/digest"));}Ok(())}
pub fn definition_relations()->Vec<Relation> {vec![Relation::of::<RetrievalDefinition>()]}
pub fn rendering_relations()->Vec<Relation> {macro_rules! output {($($f:ident:$ty:ty,)*)=>{vec![$(Relation::of::<$ty>()),*]};}crate::retrieval_outputs!(output)}
pub fn relations()->Vec<Relation> {let mut r=rendering_relations();r.extend(consumption::relations());r}

pub type Definition=RetrievalDefinition;
