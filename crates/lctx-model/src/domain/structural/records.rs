//! Qualified structural observations over catalog-owned public slots and stored program graphs.
//! Traversal rows preserve execution qualifiers on each step; reachability is not normal completion.
use crate::domain::{*,analysis::{structural as publication,settings::AnalyticsConfiguration},catalog::*,normalized::{entities::*,events::*},source::*,projection::*,assertion::AssertionQualification,attribution::Modality,calls::CallPhase};
use crate::{Domain,DomainCode,DomainSum};
#[derive(Debug,Clone,PartialEq,Eq,Domain)]
#[model(name="structural_frames",invariants=super::build::invariants,publication_checks=super::frames::profile_checks)]
pub struct StructuralFrame {
 #[model(key)] pub invocation:Id<publication::AnalysisInvocation>,
 #[model(key)] pub usage_invocation:Id<publication::AnalysisInvocation>,
 #[model(key)] pub handoff_invocation:Id<publication::AnalysisInvocation>,
 #[model(key)] pub control_invocation:Id<publication::AnalysisInvocation>,
 pub controls_requested:bool,
 #[model(key)] pub configuration:Id<AnalyticsConfiguration>,
 #[model(key)] pub invocation_graph:Id<ProjectionSourceAssessment>,
 #[model(key)] pub definition_graph:Id<ProjectionSourceAssessment>,
}
/// This is an admitted public access candidate, preserving C0's resolution and descriptor limits.
#[derive(Debug,Clone,PartialEq,Eq,Domain)]
#[model(name="structural_public_candidates")]
pub struct PublicCandidate {
 #[model(key)] pub frame:Id<StructuralFrame>,
 #[model(key)] pub callable:Id<CatalogCallable>,
 pub member:Id<CatalogMember>,
 pub entity:Id<EntityRef>,
 pub path:String,
 pub configured_ordinal:Option<i64>,
 pub in_subsystem:bool,
}
#[derive(Debug,Clone,PartialEq,Eq,Domain)]
#[model(name="structural_scope_members")]
pub struct ScopeMember {
 #[model(key)] pub frame:Id<StructuralFrame>,
 #[model(key)] pub entity:Id<EntityRef>,
 pub module:Id<Module>,
 pub declaration:Id<Occurrence>,
 pub role:Id<input::ArtifactUse>,
}
#[derive(Debug,Clone,Copy,PartialEq,Eq,Hash,DomainCode)]
#[repr(i16)]
pub enum ReachKind {Direct=0,BoundedPath=1,ExternalBoundary=2,SyntheticBoundary=3,DependencyBoundary=4,SubsystemBoundary=5}
#[derive(Debug,Clone,Copy,PartialEq,Eq,Hash,DomainCode)]
#[repr(i16)]
pub enum TraversalStop {Depth=0,Vertices=1,Arcs=2}
#[derive(Debug,Clone,PartialEq,Eq,Domain)]
#[model(name="structural_traversals")]
pub struct Traversal {
 #[model(key)] pub frame:Id<StructuralFrame>,
 #[model(key)] pub seed:Id<EntityRef>,
 pub stop:Option<TraversalStop>,
 pub partial:bool,
 pub examined_vertices:i64,
 pub examined_arcs:i64,
}
#[derive(Debug,Clone,PartialEq,Eq,Domain)]
#[model(name="structural_reaches")]
pub struct Reach {
 #[model(key)] pub traversal:Id<Traversal>,
 #[model(key)] pub target:Id<EntityRef>,
 pub kind:ReachKind,
 pub depth:i64,
 pub witnesses_omitted:bool,
}
#[derive(Debug,Clone,PartialEq,Eq,Domain)]
#[model(name="structural_paths")]
pub struct Path {
 #[model(key)] pub reach:Id<Reach>,
 #[model(key)] pub ordinal:i64,
 pub length:i64,
}
#[derive(Debug,Clone,PartialEq,Eq,Hash,DomainSum)]
#[model(name="structural_arc_sources")]
pub enum ArcSource {
 #[model(code=0)] Invocation {alternative:Id<NormalizedCallAlternative>},
 #[model(code=1)] Definition {alternative:Id<NormalizedCallAlternative>},
 #[model(code=2)] SourceDefinition {ownership:Id<OccurrenceOwnership>},
}
#[derive(Debug,Clone,PartialEq,Eq,Hash,DomainSum)]
#[model(name="structural_step_evidence")]
pub enum StepEvidence {
 #[model(code=0)] Call {event:Id<NormalizedCallEvent>,site:Id<Occurrence>,phase:CallPhase,qualification:Id<AssertionQualification>,modality:Modality,derived_dispatch:bool},
 #[model(code=1)] Declaration {owner:Id<OccurrenceOwnership>,declaration:Id<Occurrence>,callable:Id<CallableEntity>},
}
#[derive(Debug,Clone,PartialEq,Eq,Domain)]
#[model(name="structural_path_steps")]
pub struct PathStep {
 #[model(key)] pub path:Id<Path>,
 #[model(key)] pub ordinal:i64,
 pub arc:Id<ArcSource>,
 pub source:Id<EntityRef>,
 pub target:Id<EntityRef>,
 pub evidence:Id<StepEvidence>,
}
#[derive(Debug,Clone,PartialEq,Eq,Domain)]
#[model(name="structural_unresolved_events")]
pub struct UnresolvedEvent {
 #[model(key)] pub traversal:Id<Traversal>,
 #[model(key)] pub event:Id<NormalizedCallEvent>,
 pub assessment:Id<EventAssessment>,
 pub site:Id<Occurrence>,
 pub depth:i64,
}
#[derive(Debug,Clone,PartialEq,Eq,Domain)]
#[model(name="structural_unresolved_steps")]
pub struct UnresolvedStep {
 #[model(key)] pub event:Id<UnresolvedEvent>,
 #[model(key)] pub ordinal:i64,
 pub arc:Id<ArcSource>,
 pub source:Id<EntityRef>,
 pub target:Id<EntityRef>,
 pub evidence:Id<StepEvidence>,
}
#[derive(Debug,Clone,PartialEq,Eq,Domain)]
#[model(name="structural_usage_sites")]
pub struct UsageSite {
 #[model(key)] pub frame:Id<StructuralFrame>,
 #[model(key)] pub site:Id<Occurrence>,
 pub targets:i64,
 pub complete:bool,
 pub uncertain:bool,
}
#[derive(Debug,Clone,PartialEq,Eq,Domain)]
#[model(name="structural_usage_evidence")]
pub struct UsageEvidence {
 #[model(key)] pub site:Id<UsageSite>,
 #[model(key)] pub event:Id<NormalizedCallEvent>,
 #[model(key)] pub policy:Id<CallPolicyAssessment>,
 #[model(key)] pub admission:Id<CallPolicyAdmission>,
 #[model(key)] pub alternative:Id<NormalizedCallAlternative>,
 #[model(key)] pub target:Id<EntityRef>,
}
#[derive(Debug,Clone,PartialEq,Eq,Domain)]
#[model(name="structural_usage_scores")]
pub struct UsageScore {
 #[model(key)] pub frame:Id<StructuralFrame>,
 #[model(key)] pub target:Id<EntityRef>,
 pub share:FiniteF64,
 pub contributing_sites:i64,
}
/// Every configured path retains an assessment, including missing, ambiguous and out-of-scope slots.
#[derive(Debug,Clone,PartialEq,Eq,Domain)]
#[model(name="structural_configured_seeds")]
pub struct ConfiguredSeed {
 #[model(key)] pub frame:Id<StructuralFrame>,
 #[model(key)] pub ordinal:i64,
 pub path:String,
 pub candidates:i64,
 pub in_subsystem:i64,
}
