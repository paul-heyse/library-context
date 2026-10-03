//! Cohesive nested packets over canonical identities, not an arbitrary row-flattening language.
use super::*;
use crate::domain::{self, *};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
macro_rules! packet {($name:ident {$($(#[$attr:meta])* $field:ident:$ty:ty),*$(,)?})=>{
    #[derive(Debug,Clone,PartialEq,Eq,Serialize,Deserialize,JsonSchema)]
    #[serde(deny_unknown_fields)] pub struct $name {$ ($(#[$attr])* pub $field:$ty,)*}
};}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "status", rename_all = "snake_case", deny_unknown_fields)]
pub enum Availability {
    Available {},
    Partial {
        #[doc = "Declared canonical reason for this result; interpret it with the accompanying status and evidence."]
        reason: Name,
    },
    NotRequested {},
    Unavailable {
        #[doc = "Declared canonical reason for this result; interpret it with the accompanying status and evidence."]
        reason: Name,
    },
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SectionPage<T> {
    /// Section availability; unavailable or not-requested data must not be interpreted as absence.
    pub availability: Availability,
    pub items: Vec<T>,
    #[serde(default, skip_serializing_if = "Optional::is_absent")]
    pub continuation: Optional<CursorToken>,
    /// Number of results omitted from this page; pagination retains the original request identity.
    pub omitted: u64,
    /// The page omitted results because of its declared bound.
    pub truncated: bool,
}
packet!(PacketLimits {
    maximum_page_rows: u32,
    maximum_response_bytes: u64,
    signature_indivisible: bool
});
packet!(ReleaseIdentity {input:Id<input::InputRevision>,release:Id<input::Release>,distribution:Name,version:Name});
packet!(AccessProvenance {module:Id<source::Module>,path:Vec<Name>,exposures:Vec<Id<catalog::CatalogExposure>>,candidates:Vec<Id<catalog::CatalogCandidate>>,#[doc = "Canonical evidence or derivation basis for this result, rather than a confidence score."] basis:Nullable<catalog::CatalogContractBasis>});
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum DefaultValue {
    Absent {},
    Unavailable {},
    Unknown {},
    Literal { literal: Id<value::Literal> },
    Expression { expression: Id<source::Occurrence> },
    Factory { expression: Id<source::Occurrence> },
}
impl DefaultValue {
    pub fn from_canonical(value: &catalog::CatalogDefault) -> Self {
        match value {
            catalog::CatalogDefault::Absent {} => Self::Absent {},
            catalog::CatalogDefault::Unavailable {} => Self::Unavailable {},
            catalog::CatalogDefault::Unknown {} => Self::Unknown {},
            catalog::CatalogDefault::Literal { literal } => Self::Literal { literal: *literal },
            catalog::CatalogDefault::Expression { expression } => Self::Expression {
                expression: *expression,
            },
            catalog::CatalogDefault::Factory { expression } => Self::Factory {
                expression: *expression,
            },
        }
    }
}
packet!(ParameterPacket {parameter:Id<calls::SignatureParameter>,slot:Nullable<Id<normalized::callables::SignatureSlot>>,#[doc = "Normalized parameter identities. A source identity denotes an observed source formal; a native slot identity does not imply a source declaration."] formals:Vec<Id<normalized::entities::ParameterEntity>>,ordinal:i64,name:Nullable<Name>,#[doc = "Finite canonical kind; the numeric codebook lists supported choices."] kind:calls::ParameterKind,required:bool,types:Vec<Id<types::TypeTerm>>,type_evidence:Vec<ProofReference>,default:DefaultValue});
packet!(SignaturePacket {signature:Id<calls::Signature>,role:calls::SignatureRole,native:Nullable<Id<types::NativeSignatureObservation>>,variant:Id<normalized::callables::SignatureVariant>,analysis:Id<attribution::AnalysisContext>,#[doc = "Declared canonical signature form."] form:calls::SignatureForm,#[doc = "Normalized signature adjustment applied to the effective callable, preserving the source signature."] adjustment:normalized::callables::SignatureAdjustment,parameters:Vec<ParameterPacket>,effective_parameters:Vec<ParameterPacket>,return_types:Vec<Id<types::TypeTerm>>,return_evidence:Vec<ProofReference>,typing:Vec<SignatureTypingPacket>,complete:bool});
/// Typing characterization is separate from unrestricted runtime behavior.
#[derive(Debug,Clone,PartialEq,Eq,Serialize,Deserialize,JsonSchema)]
#[serde(tag="kind",rename_all="snake_case",deny_unknown_fields)]
pub enum SignatureTypingOrigin {
    SourceDeclared { observation:Id<types::TypeObservation>, subject:Id<source::Occurrence> },
    NativeObserved { observation:Id<types::SignatureTypeObservation>, subject:Id<types::SignatureTypeSubject> },
}
packet!(SignatureTypingPacket {origin:SignatureTypingOrigin,term:Id<types::TypeTerm>,qualification:Id<assertion::AssertionQualification>,claim_basis:ClaimBasisPacket,proof:Vec<ProofReference>});
packet!(InvocationPacket {callable:Id<catalog::CatalogCallable>,invocation:Id<catalog::CatalogInvocation>,assessment:Id<normalized::callables::EffectiveCallableAssessment>,analysis:Id<attribution::AnalysisContext>,#[doc = "Completeness of the effective callable evidence; unknown does not mean absent."] knowledge:normalized::callables::Knowledge,#[doc = "Declared invocation or signature form; an absent value means form evidence is unavailable."] form:Nullable<selection::InvocationForm>});
packet!(OptionPacket {option:Id<catalog::CatalogOption>,subject:Id<catalog::CatalogOptionSubject>,evidence:Id<catalog::CatalogOptionEvidence>,default:DefaultValue});
packet!(OperationCore {member:Id<catalog::CatalogMember>,name:Name,release:ReleaseIdentity,access:AccessProvenance,invocations:Vec<InvocationPacket>,signatures:Vec<SignaturePacket>,#[doc = "Completeness of signature evidence; unknown signatures remain visible."] signature_knowledge:normalized::callables::Knowledge,options:Vec<OptionPacket>,literal_values:Vec<LiteralPacket>,type_presentations:Vec<TypePresentationPacket>,limits:PacketLimits});
/// Address an actual stored nominal source; wrappers are not manufactured during serving.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum OriginalReference {
    Catalog {
        source: Id<catalog::evidence::OriginalSource>,
    },
    Anchor {
        anchor: Id<retrieval::OriginalAnchor>,
    },
    Prose {
        slice: Id<synthesis::documentary::ProseSlice>,
    },
    Artifact {
        artifact: Id<source::SourceArtifact>,
    },
    Occurrence {
        occurrence: Id<source::Occurrence>,
    },
    Span {
        span: assertion::EvidenceSourceSpanId,
    },
}
packet!(OriginalRange {source:OriginalReference,artifact:Id<source::SourceArtifact>,start:u64,end:u64,digest:ContentHash,encoding:Name,release:Id<input::Release>,context:Id<attribution::AnalysisContext>});
packet!(ScenarioPacket {scenario:Id<catalog::evidence::CatalogScenario>,#[doc = "Declared scenario intent; choose a scenario according to its original evidence and check results."] intent:catalog::evidence::Intent,#[doc = "Canonical evidence or derivation basis for this result, rather than a confidence score."] basis:catalog::evidence::AssociationBasis,spans:Vec<OriginalRange>,#[doc = "Parse check status from the pinned deployment evidence; not-run is distinct from a failed check."] parse:deployment::CheckStatus,#[doc = "Binding check status; passing parsing alone does not establish correct binding."] binding:deployment::CheckStatus,#[doc = "Environment check status under the recorded pinned deployment context."] environment:deployment::CheckStatus,#[doc = "Execution check status; not-requested or unavailable evidence does not establish a successful run."] execution:deployment::CheckStatus});
packet!(DeploymentPacket {deployment:Id<catalog::evidence::CatalogDeployment>,field:Name,name:Name,value:Text<0,8192>,originals:Vec<OriginalRange>});
packet!(RelationshipPacket {target:Id<catalog::CatalogMember>,analysis:Id<attribution::AnalysisContext>,#[doc = "Declared relationship or support role; interpret it within the accompanying evidence context."] role:selection::RelationRole,#[doc = "Fidelity of the declared relationship; unresolved correspondence is not exact identity."] fidelity:selection::Fidelity,witnesses:Vec<Id<selection::Witness>>,proof:Vec<ProofReference>});
packet!(ConflictPacket {requirement:selection::Requirement,#[doc = "Declared canonical reason for this result; interpret it with the accompanying status and evidence."] reason:selection::Reason,contexts:Vec<Id<selection::Context>>,positive:Vec<Id<selection::Witness>>,negative:Vec<Id<selection::Witness>>});
packet!(AssertionSupportPacket {support:Id<synthesis::assertions::ProgrammaticAssertionSupport>,#[doc = "Declared relationship or support role; interpret it within the accompanying evidence context."] role:analysis::policy::SupportRole,source:Id<synthesis::assertions::AssertionSource>,proof:Vec<ProofReference>});
// Fully resolved premises of a conditional claim. Empty is an explicit canonical set row.
packet!(ClaimBasisPacket {set:Id<assumptions::AssumptionSet>,members_digest:ContentHash,definitions:Vec<ClaimAssumptionPacket>});
packet!(AssumptionNativeSupportPacket {support:ProofReference,run:Id<attribution::ProviderRun>,input:Id<input::InputRevision>,context:Id<attribution::AnalysisContext>,environment:ContentHash,provider:Name,provider_revision:Name,provider_build:ContentHash,surface:Name,evidence:AssumptionEvidencePacket,fidelity:attribution::Fidelity});
#[derive(Debug,Clone,PartialEq,Eq,Serialize,Deserialize,JsonSchema)]
#[serde(tag="kind",rename_all="snake_case",deny_unknown_fields)]
pub enum AssumptionEvidencePacket {
    Invocation { run:Id<attribution::ProviderRun> },
    Source { artifact:Id<source::SourceArtifact>,path:Name,content:ContentHash,start:i64,end:i64 },
}
packet!(AssumptionUniversePacket {universe:Id<assumptions::AssumptionUniverse>,context:Id<attribution::AnalysisContext>,input:Id<input::InputRevision>,environment:ContentHash,model_definition:ContentHash,support:Id<assumptions_universe::AssumptionUniverseSupport>,catalog:Id<models::ModelCatalog>,model:Id<models::AuthoredModel>,source_name:Name,format:i64,source:Text<0,262144>});
#[derive(Debug,Clone,PartialEq,Eq,Serialize,Deserialize,JsonSchema)]
#[serde(tag="kind",rename_all="snake_case",deny_unknown_fields)]
pub enum ClaimAssumptionPacket {
    TypeConformance { assumption:Id<assumptions::Assumption>,observation:Id<types::TypeObservation>,subject:Id<source::Occurrence>,role:types::TypeRole,declared:bool,term:Id<types::TypeTerm>,support:AssumptionNativeSupportPacket },
    NoExtraOverrides { assumption:Id<assumptions::Assumption>,class:Id<symbols::ClassTraitObservation>,symbol:Id<calls::ProviderSymbol>,synthesized:bool,dataclass:bool,named_tuple:bool,typed_dict:bool,support:AssumptionNativeSupportPacket,universe:AssumptionUniversePacket },
}
impl ClaimAssumptionPacket {
    pub fn assumption(&self) -> Id<assumptions::Assumption> { match self { Self::TypeConformance {assumption,..}|Self::NoExtraOverrides {assumption,..} => *assumption } }
}
impl ClaimBasisPacket {
    pub fn from_canonical(basis:&assumptions::ResolvedAssumptions, mut definitions:Vec<ClaimAssumptionPacket>) -> Result<Self,ModelError> {
        basis.check(basis.set.id())?;
        definitions.sort_by_key(ClaimAssumptionPacket::assumption);
        if definitions.iter().map(ClaimAssumptionPacket::assumption).collect::<Vec<_>>() != basis.members.iter().map(|m|m.assumption).collect::<Vec<_>>() { return Err(ModelError::Invalid("packet assumption definitions missing or differ".into())); }
        Ok(Self {set:basis.set.id(),members_digest:basis.set.members,definitions})
    }
}
packet!(AssertionPacket {assertion:Id<synthesis::assertions::ProgrammaticAssertion>,#[doc = "Finite canonical kind; the numeric codebook lists supported choices."] kind:analysis::policy::AssertionKind,#[doc = "Authored brief section to which the assertion belongs."] section:analysis::policy::BriefSection,#[doc = "Evidence status of the canonical result; unsupported and unexamined evidence remain distinct."] status:analysis::policy::EvidenceStatus,qualification:Id<assertion::AssertionQualification>,claim_basis:ClaimBasisPacket,text:Text<0,262144>,supports:Vec<AssertionSupportPacket>});
packet!(CapabilityPacket {capability:Id<synthesis::briefs::Brief>,title:Name,rendered:Text<0,262144>,assertions:Vec<AssertionPacket>,originals:Vec<OriginalRange>,#[doc = "Section availability; unavailable or not-requested data must not be interpreted as absence."] availability:Availability,unreviewed:bool,documentation_only:bool});
packet!(BehaviorPacket {#[doc = "Direct capture provenance for this selected answer. Empty does not establish absence of captures."] captures:Vec<BehavioralCapturePacket>,claim_basis:ClaimBasisPacket,condition:Id<conditions::Condition>,#[doc = "Model-qualified verdict, never proof of unrestricted runtime behavior."] verdict:obligation::Verdict,model:Id<models::ModelCatalog>,proof:Vec<ProofReference>,presentation:RenderedConditionPacket,presentation_truncated:bool});
/// A runtime value source is explicit and independent of native snapshot candidates.
#[derive(Debug,Clone,PartialEq,Eq,Serialize,Deserialize,JsonSchema)]
#[serde(tag="kind",rename_all="snake_case",deny_unknown_fields)]
pub enum CapturedValueSourcePacket {
    Entry { formal:Id<normalized::entities::ParameterEntity>,parameter:Id<calls::SignatureParameter>,declaration:Id<source::Occurrence>,signature:Id<calls::Signature>,ordinal:i64,name:Nullable<Name>,source_correspondence:CaptureSourceDeclarationPacket },
    Literal { value:Id<source::Occurrence>,statement:Id<source::Occurrence>,literal:LiteralPacket },
}
// Source declaration attribution preserves projection fidelity; it supplies correspondence only.
packet!(CaptureSourceSupportPacket {support:ProofReference,run:Id<attribution::ProviderRun>,input:Id<input::InputRevision>,context:Id<attribution::AnalysisContext>,environment:ContentHash,provider_id:Id<attribution::Provider>,provider:Name,provider_revision:Name,provider_build:ContentHash,surface:Name,evidence:AssumptionEvidencePacket,origin:attribution::Origin,mode:attribution::ExtractionMode,fidelity:attribution::Fidelity});
packet!(CaptureSourceDeclarationPacket {assertion:Id<declarations::ParameterDeclaration>,qualification:Id<assertion::AssertionQualification>,scope:Id<source::CoverageScope>,condition:Id<conditions::Condition>,modality:attribution::Modality,approximation:assertion::Approximation,claim_basis:ClaimBasisPacket,#[doc = "Source formal correspondence, never captured runtime value authority."] source_correspondence_only:bool,support:CaptureSourceSupportPacket});
packet!(CaptureNativeProofPacket {assertion:ProofReference,qualification:Id<assertion::AssertionQualification>,scope:Id<source::CoverageScope>,condition:Id<conditions::Condition>,modality:attribution::Modality,approximation:assertion::Approximation,claim_basis:ClaimBasisPacket,support:AssumptionNativeSupportPacket});
packet!(CaptureOriginPacket {definition:Id<flow::FlowDefinition>,occurrence:Id<source::Occurrence>,place:Id<value::Place>,scope:Id<lexical::LexicalScope>,declaring:Id<source::Occurrence>,kind:lexical::BindingEventKind,proof:CaptureNativeProofPacket});
packet!(NativeCapturePacket {function:Id<calls::ProviderSymbol>,declaring:Nullable<Id<calls::ProviderSymbol>>,name:Name,origin:captures::CaptureOrigin,mutable:Nullable<bool>,timing:captures::CaptureTiming,proof:CaptureNativeProofPacket});
#[derive(Debug,Clone,PartialEq,Eq,Serialize,Deserialize,JsonSchema)]
#[serde(tag="kind",rename_all="snake_case",deny_unknown_fields)]
pub enum CaptureCandidateTargetPacket {
    Bound {definition:Id<flow::FlowDefinition>,occurrence:Id<source::Occurrence>,place:Id<value::Place>},
    Undefined {},Deleted {},Nested {},LoopHeader {},Unattached {},
}
packet!(CaptureCandidatePacket {ordinal:i64,target:CaptureCandidateTargetPacket,condition:Id<conditions::Condition>,narrowing:Id<conditions::Condition>,precision_lost:bool});
packet!(CaptureTimingPacket {use_:Id<flow::FlowUse>,nested_scope:Id<lexical::LexicalScope>,enclosing_scope:Id<lexical::LexicalScope>,origin:flow_capture::FlowCaptureOrigin,timing:captures::CaptureTiming,state:flow_capture::FlowSnapshotState,candidates:Vec<CaptureCandidatePacket>,constraint:Nullable<Id<conditions::Condition>>,constraint_precision_lost:bool,#[doc = "Native eager/lazy snapshot characterization; this is never call-time value authority."] characterization_only:bool,proof:CaptureNativeProofPacket});
packet!(CaptureFramePacket {header:Id<execution::source_call_records::SourceCallHeader>,call:Id<execution::enriched_records::SourceExecutionInvocation>,caller_body:Id<execution::enriched_records::BodyExecution>,callee_body:Id<execution::enriched_records::BodyExecution>,event:Id<normalized::events::NormalizedCallEvent>,call_site:Id<source::Occurrence>,caller:Id<normalized::entities::EntityRef>,callee:Id<normalized::entities::EntityRef>,caller_declaration:Id<source::Occurrence>,callee_declaration:Id<source::Occurrence>,returned_at:Id<source::Occurrence>,#[doc = "Conditional on entry to this exact direct synchronous caller frame, under the bounded source-call model."] under_caller_entry:bool,qualification:Id<assertion::AssertionQualification>});
packet!(BehavioralCapturePacket {witness:Id<execution::summary_capture::SummaryCaptureWitness>,binding:Id<execution::capture_bridge::CapturedEntryBinding>,alternative:Id<transfer::summary::TransferAlternative>,transfer:Id<transfer::summary::TransferKey>,input:Id<input::InputRevision>,context:Id<attribution::AnalysisContext>,read:Id<source::Occurrence>,value_source:CapturedValueSourcePacket,frame:CaptureFramePacket,origin:CaptureOriginPacket,native_capture:NativeCapturePacket,timing:CaptureTimingPacket,claim_basis:ClaimBasisPacket,proof:Vec<ProofReference>});
packet!(OperationPacket {core:OperationCore,scenarios:SectionPage<ScenarioPacket>,deployment:SectionPage<DeploymentPacket>,relationships:SectionPage<RelationshipPacket>,conflicts:SectionPage<ConflictPacket>,briefs:SectionPage<CapabilityPacket>,behavior:SectionPage<BehaviorPacket>});
packet!(BehavioralExceptionPacket {summary:Id<execution::summary_exceptions::SummaryExceptionOutcome>,body:Id<execution::enriched_records::BodyExecution>,input:Id<input::InputRevision>,context:Id<attribution::AnalysisContext>,owner:Id<normalized::entities::EntityRef>,qualification:Id<assertion::AssertionQualification>,scope:Id<source::CoverageScope>,condition:Id<conditions::Condition>,#[doc = "Exact finite builtin runtime exception under body entry. Null means this admitted body completed normally; missing bodies never produce absence evidence."] exception:Nullable<execution::ExactRuntimeException>,#[doc = "This result is conditional on entering the source body and uses the bounded runtime model, independently of typing characterization."] under_body_entry:bool,claim_basis:ClaimBasisPacket,proof:Vec<ProofReference>});
impl BehavioralExceptionPacket {
    pub fn from_canonical(result: &execution::summary_exceptions::SummaryExceptionOutcome, q: &assertion::AssertionQualification) -> Result<Self, ModelError> {
        let empty = assumptions::AssumptionSet::empty();
        if result.qualification != q.id() || result.context != q.context || q.assumptions != empty.id()
            || q.condition != conditions::Diagram::always().id() || q.modality != attribution::Modality::Definite
            || q.approximation != assertion::Approximation::Exact {
            return Err(ModelError::Invalid("runtime exception packet changes its exact entry basis".into()));
        }
        Ok(Self { summary: result.id(), body: result.body, input: result.input, context: result.context, owner: result.owner,
            qualification: q.id(), scope: q.scope, condition: q.condition, exception: Nullable(result.exception), under_body_entry: true,
            claim_basis: ClaimBasisPacket { set: empty.id(), members_digest: empty.members, definitions: Vec::new() },
            proof: vec![ProofReference::from_canonical(derivation::RowRef::of(result.id())), ProofReference::from_canonical(derivation::RowRef::of(result.body))],
        })
    }
}
packet!(RequirementWitnessPacket {context:Id<selection::Context>,#[doc = "Canonical evidence or derivation basis for this result, rather than a confidence score."] basis:selection::EvidenceBasis,positive:Vec<Id<selection::Witness>>,negative:Vec<Id<selection::Witness>>,behavioral_exceptions:Vec<BehavioralExceptionPacket>});
packet!(RequirementResult {claims:Vec<RequirementWitnessPacket>,closure:Vec<Id<selection::Witness>>,requirement:selection::Requirement,#[doc = "Selection outcome; unresolved evidence stays separate from conflicting and supported results."] outcome:selection::Outcome,#[doc = "Declared canonical reason for this result; interpret it with the accompanying status and evidence."] reason:selection::Reason,contexts:Vec<Id<selection::Context>>,positive:Vec<Id<selection::Witness>>,negative:Vec<Id<selection::Witness>>,corpus_complete:bool,analyzer_complete:bool,examined:u64,total:Nullable<u64>});
packet!(OperationCandidate {member:Id<catalog::CatalogMember>,analysis:Id<attribution::AnalysisContext>,name:Name,requirements:Vec<RequirementResult>,#[doc = "Whether the requirements have a shared applicable context; separate supported results alone do not prove joint applicability."] joint:selection::JointApplicability,#[doc = "Completeness of signature evidence; unknown signatures remain visible."] signature_knowledge:normalized::callables::Knowledge});
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "extent", rename_all = "snake_case", deny_unknown_fields)]
pub enum SelectionExtent {
    CompleteDomain {
        total: u64,
    },
    Ranked {
        returned: u64,
    },
    EmptyUnderCoverage {
        corpus_complete: bool,
        analyzer_complete: bool,
    },
}
packet!(ProofReference {
    relation: Name,
    row: [u8; 16]
});
impl ProofReference {
    pub fn from_canonical(row: domain::derivation::RowRef) -> Self {
        Self {
            relation: Name::new(row.relation()).expect("canonical relation name bounded"),
            row: *row.bytes(),
        }
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct EvidenceBodyPage {
    pub start: u64,
    pub end: u64,
    pub bytes: Vec<u8>,
    #[serde(default, skip_serializing_if = "Optional::is_absent")]
    pub continuation: Optional<CursorToken>,
    /// Number of results omitted from this page; pagination retains the original request identity.
    pub omitted: u64,
    /// The page omitted results because of its declared bound.
    pub truncated: bool,
}
packet!(EvidencePacket {original:OriginalRange,body:EvidenceBodyPage,#[doc="Selected positive native source observations within this original range. An empty section proves neither diagnostic absence nor successful execution."] source_characterization:SectionPage<SourceCharacterizationPacket>,#[doc = "Evidence status of the canonical result; unsupported and unexamined evidence remain distinct."] status:analysis::policy::EvidenceStatus,derivation:SectionPage<DerivationStep>});
packet!(DerivationStep {source:ProofReference,rule:Name,conclusion:ProofReference,premises:Vec<PremisePacket>});
packet!(PremisePacket {
    #[doc = "Declared relationship or support role; interpret it within the accompanying evidence context."]
    role: Name,
    premise: ProofReference
});
packet!(EvidenceHit {unit:Id<retrieval::Unit>,family:retrieval::Family,title:Name,originals:Vec<OriginalRange>,associated_members:Vec<Id<catalog::CatalogMember>>});
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum BrowseEntry {
    Module {
        module: Id<source::Module>,
        name: Name,
        members: u64,
    },
    Class {
        member: Id<catalog::CatalogMember>,
        name: Name,
        members: u64,
    },
    Member {
        candidate: OperationCandidate,
        ownership: Availability,
    },
    Vocabulary {
        facet: selection::Facet,
        values: Vec<Name>,
        members: u64,
    },
}
packet!(ComparisonEntry {requested:OperationSelector,candidates:Vec<OperationCandidate>,ambiguous:bool});
packet!(RenderedAtom {atom:Id<conditions::EvaluationAtom>,value:bool});
packet!(RenderedConditionPacket {terms:Vec<Vec<RenderedAtom>>,#[doc = "The finite condition rendering exceeded its declared bound; the canonical condition remains unchanged."] truncated:bool});
impl RenderedConditionPacket {
    pub fn from_canonical(value: &conditions::RenderedCondition) -> Self {
        Self {
            terms: value
                .terms
                .iter()
                .map(|term| {
                    term.iter()
                        .map(|(atom, value)| RenderedAtom {
                            atom: *atom,
                            value: *value,
                        })
                        .collect()
                })
                .collect(),
            truncated: value.truncated,
        }
    }
}

/// A named rendering of the canonical literal owner; transient ExactScalar has a narrower domain.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum LiteralValue {
    None {},
    Bool { value: bool },
    Integer { decimal: String },
    String { value: String },
    Bytes { value: Vec<u8> },
    Float { bits: i64 },
}
packet!(LiteralPacket {literal:Id<value::Literal>,value:LiteralValue});
impl LiteralPacket {
    pub fn from_canonical(row: &value::Literal) -> Result<Self, ModelError> {
        row.validate()?;
        let value = match row {
            value::Literal::None => LiteralValue::None {},
            value::Literal::Bool { value } => LiteralValue::Bool { value: *value },
            value::Literal::Integer { decimal } => LiteralValue::Integer {
                decimal: decimal.clone(),
            },
            value::Literal::String { value } => LiteralValue::String {
                value: value.as_str().to_owned(),
            },
            value::Literal::Bytes { value } => LiteralValue::Bytes {
                value: value.0.clone(),
            },
            value::Literal::Float { bits } => LiteralValue::Float { bits: *bits },
        };
        Ok(Self {
            literal: row.id(),
            value,
        })
    }
}
packet!(TypePresentationPacket {presentation:Id<types::TypePresentation>,term:Id<types::TypeTerm>,qualification:Id<assertion::AssertionQualification>,claim_basis:ClaimBasisPacket,display:Text<0,262144>,detail:Nullable<Text<0,262144>>});
impl TypePresentationPacket {
    pub fn from_canonical(row: &types::TypePresentation, claim_basis:ClaimBasisPacket) -> Result<Self, WireError> {
        Ok(Self {
            claim_basis,
            presentation: row.id(),
            term: row.term,
            qualification: row.qualification,
            display: Text::new(row.display.clone())?,
            detail: Nullable(row.detail.clone().map(Text::new).transpose()?),
        })
    }
}

impl CapabilityPacket {
    /// Keep canonical authored bytes intact; expose the same attributed claims as the packet.
    pub fn resource_text(&self) -> Result<String, WireError> {
        let mut text = self.rendered.as_str().to_owned();
        text.push_str("\n\n## Assertion evidence\n");
        for assertion in &self.assertions {
            let metadata = serde_json::json!({"assertion":assertion.assertion,"status":assertion.status,"status_name":format!("{:?}",assertion.status),"kind":assertion.kind,"qualification":assertion.qualification,"claim_basis":assertion.claim_basis,"text":assertion.text,"supports":assertion.supports});
            text.push_str("\n```json\n");
            text.push_str(&serde_json::to_string(&metadata)?);
            text.push_str("\n```\n");
        }
        Ok(text)
    }
}

// A location is available only inside this request's already granted original range.
packet!(SourceCharacterizationSpan {artifact:Id<source::SourceArtifact>,start:u64,end:u64});
packet!(SourceAnnotationPacket {annotation:Id<diagnostics::DiagnosticAnnotation>,location:Availability,span:Nullable<SourceCharacterizationSpan>,label:Nullable<Text<0,16384>>});
packet!(NativeSourceSupportPacket {support:ProofReference,run:Id<attribution::ProviderRun>,input:Id<input::InputRevision>,context:Id<attribution::AnalysisContext>,environment:ContentHash,provider:Name,revision:Name,build:ContentHash,surface:Name,evidence:ProofReference,fidelity:attribution::Fidelity});
#[derive(Debug,Clone,PartialEq,Eq,Serialize,Deserialize,JsonSchema)]
#[serde(tag="kind",rename_all="snake_case",deny_unknown_fields)]
pub enum SourceCharacterizationPayload {
    Usage {usage:SourceUsagePacket},
    RuffDiagnostic {observation:Id<diagnostics::RuffDiagnosticObservation>,rule:diagnostics::SelectedRuffRule,native_id:Name,native_code:Name,primary_location:diagnostics::DiagnosticLocation,severity:diagnostics::DiagnosticSeverity,channel:diagnostics::DiagnosticChannel,message:Text<0,16384>,settings:ContentHash,annotations:Vec<SourceAnnotationPacket>},
    PyreflyDiagnostic {observation:Id<diagnostics::PyreflyDiagnosticObservation>,category:Name,primary_location:diagnostics::DiagnosticLocation,severity:diagnostics::DiagnosticSeverity,channel:diagnostics::DiagnosticChannel,baseline:diagnostics::NativeBaselineStatus,header:Text<0,16384>,details:Nullable<Text<0,16384>>,annotations:Vec<SourceAnnotationPacket>},
    ParameterDefinition {observation:Id<diagnostics::NativeParameterDefinitionObservation>,parameter:Id<syntax::ParameterSyntaxObservation>,answer:diagnostics::DefinitionAnswer,answer_count:u64,role:diagnostics::NativeParameterRole,reason:Nullable<obligation::ObligationKind>,metadata:Nullable<diagnostics::NativeDefinitionMetadata>,symbol_kind:Nullable<diagnostics::NativeDefinitionSymbolKind>,target_location:Availability,target:Nullable<SourceCharacterizationSpan>,target_name:Nullable<Text<0,16384>>},
}
packet!(SourceCharacterizationPacket {characterization:Id<catalog::evidence::SourceCharacterization>,qualification:Id<assertion::AssertionQualification>,#[doc="Captured source anchor. A diagnostic with unavailable primary location uses its artifact anchor; this is not a fabricated diagnostic range."] source:SourceCharacterizationSpan,#[doc="Containing scenarios characterize source context only. An empty list is unassociated Unknown, never proof of an API target or test execution."] containing_scenarios:Vec<Id<catalog::evidence::CatalogScenario>>,payload:SourceCharacterizationPayload,support:NativeSourceSupportPacket,proof:Vec<ProofReference>});

// These are source/typing observations, not runtime execution or callback effects.
packet!(UsageArgumentPacket {argument:Id<calls::CallArgument>,ordinal:u64,kind:calls::ArgumentKind,keyword:Nullable<Text<0,16384>>,value:Id<source::Occurrence>,location:Availability,span:Nullable<SourceCharacterizationSpan>});
packet!(UsageBindingPacket {binding:Id<normalized::bindings::CallBinding>,slot:Id<normalized::callables::SignatureSlot>,kind:calls::BindingKind,source:Id<calls::BindingSource>,location:Availability,span:Nullable<SourceCharacterizationSpan>});
packet!(UsageApplicabilityPacket {attempt:Id<normalized::bindings::CallBindingAttempt>,variant:Nullable<Id<normalized::callables::SignatureVariant>>,signature:Nullable<Id<calls::Signature>>,role:Nullable<calls::SignatureRole>,receiver:Id<calls::Receiver>,adjustment:normalized::callables::SignatureAdjustment,authority:normalized::signature_applicability::BindingAuthority,authority_reason:normalized::signature_applicability::AuthorityReason,outcome:normalized::bindings::BindingOutcome,reason:normalized::bindings::BindingReason,refusal:Nullable<obligation::ObligationKind>,bindings:Vec<UsageBindingPacket>});
packet!(UsageAssociationPacket {association:Id<catalog::evidence::ScenarioAssociation>,member:Id<catalog::CatalogMember>,path:Vec<Name>,basis:catalog::evidence::AssociationBasis});
/// Native callback characterization does not establish invocation, effects or execution.
#[derive(Debug,Clone,PartialEq,Eq,Serialize,Deserialize,JsonSchema)]
#[serde(tag="kind",rename_all="snake_case",deny_unknown_fields)]
pub enum UsageChannel {Direct {},HigherOrder {argument_index:u64}}
packet!(UsageTargetPacket {alternative:Id<normalized::events::NormalizedCallAlternative>,target:Id<calls::CallTarget>,qualification:Id<assertion::AssertionQualification>,scope:Id<source::CoverageScope>,condition:Id<conditions::Condition>,modality:attribution::Modality,approximation:assertion::Approximation,claim_basis:ClaimBasisPacket,channel:Id<calls::CallChannel>,channel_kind:UsageChannel,implicit:bool,destination:Id<calls::CallDestination>,symbol:Nullable<Id<calls::ProviderSymbol>>,unresolved:Nullable<obligation::ObligationKind>,native_unresolved:Nullable<calls::PysaUnresolvedReason>,entity:Nullable<Id<normalized::entities::EntityRef>>,resolution:normalized::entities::ResolutionStatus,reason:normalized::links::LinkReason,phase:calls::CallPhase,receiver:Id<calls::Receiver>,receiver_location:Availability,receiver_span:Nullable<SourceCharacterizationSpan>,receiver_class:Nullable<Id<calls::ProviderSymbol>>,passing:Nullable<calls::ReceiverPassing>,associations:Vec<UsageAssociationPacket>,applicability:Vec<UsageApplicabilityPacket>,support:Vec<NativeSourceSupportPacket>});
packet!(UsageOverloadPacket {observation:Id<types::TypeObservation>,role:types::TypeRole,term:Id<types::TypeTerm>,#[doc="Native trace type identity is retained independently of normalized signature variants; shape matching cannot supply missing chosen-variant correspondence."] variant_availability:Availability,variant:Nullable<Id<normalized::callables::SignatureVariant>>,support:Vec<NativeSourceSupportPacket>});
packet!(SourceUsagePacket {usage:Id<catalog::evidence::SourceUsage>,event:Id<normalized::events::NormalizedCallEvent>,site:Id<source::Occurrence>,syntax:Nullable<Id<calls::CallSyntax>>,syntax_location:Availability,callee:Nullable<Id<source::Occurrence>>,callee_location:Availability,callee_span:Nullable<SourceCharacterizationSpan>,arguments:Vec<UsageArgumentPacket>,arguments_support:Vec<NativeSourceSupportPacket>,targets:Vec<UsageTargetPacket>,#[doc="No target/entity association is represented explicitly; matching spelling never resolves it."] association:Availability,overloads:Vec<UsageOverloadPacket>,#[doc="Absence of a chosen native trace is unavailability; candidates, including a closest failed alternative, do not select a variant."] chosen:Availability});
