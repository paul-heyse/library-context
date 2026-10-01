//! One canonical record/validator template, instantiated for finite publication owners.
//! The predecessor list is static schema, never a producer-selected publication tag.
macro_rules! analysis_family {
    ($owner:ident,$prefix:literal,[$($variant:ident:$code:literal=>$predecessor:ident),* $(,)?]) => {
        pub mod $owner {
            use super::{invalid,AnalysisDefinition,ProjectionDefinition,AnalysisMethod,AnalysisStatus,AnalysisCapability,AnalysisChannel,Interpretation};
            use crate::domain::{*,assertion::AssertionQualification,attribution::{AnalysisContext,ProviderCoverage,CoverageStatus},source::{CoverageScope,Occurrence},input::InputRevision,normalized::{entities::EntityRef,coverage::{EvidenceAvailability,NormalizationCoverage}},calls::CallPhase,obligation::ObligationKind,transfer::TransferKey};
            use crate::{Domain,DomainSum};
            macro_rules! owner_table {($suffix:literal)=>{concat!($prefix,"_",$suffix)}}
            #[derive(Debug,Clone,PartialEq,Eq,Hash,DomainSum)]
            #[model(name=owner_table!("invocation_sources"),rule="analysis_invocation_source")]
            pub enum InvocationSource {
                #[model(code=0)] Current {#[model(premise)] invocation:Id<AnalysisInvocation>},
                $(#[model(code=$code)] $variant {#[model(premise)] invocation:Id<super::$predecessor::AnalysisInvocation>},)*
            }
            impl InvocationSource {
                pub fn reference(&self)->derivation::RowRef {match self {Self::Current {invocation}=>derivation::RowRef::of(*invocation),$(Self::$variant {invocation}=>derivation::RowRef::of(*invocation),)*}}
                fn current(&self)->Option<Id<AnalysisInvocation>> {match self {Self::Current {invocation}=>Some(*invocation),_=>None}}
            }
            #[derive(Debug,Clone,PartialEq,Eq,Hash,DomainSum)]
            #[model(name=owner_table!("support_sources"),rule="analysis_support_source")]
            pub enum SupportSource {
                #[model(code=0)] NativeAssertion {#[model(premise)] premise:Id<super::native::NativeAssertionPremise>},
                #[model(code=1)] AnalysisDerivation {#[model(premise)] derivation:Id<AnalysisDerivation>},
                $(#[model(code=$code)] $variant {#[model(premise)] derivation:Id<super::$predecessor::AnalysisDerivation>},)*
            }
            impl SupportSource {
                pub fn reference(&self)->derivation::RowRef {match self {Self::NativeAssertion {premise}=>derivation::RowRef::of(*premise),Self::AnalysisDerivation {derivation}=>derivation::RowRef::of(*derivation),$(Self::$variant {derivation}=>derivation::RowRef::of(*derivation),)*}}
            }
            #[derive(Debug,Clone,PartialEq,Eq,Hash,DomainSum)]
            #[model(name=owner_table!("coverage_sources"),rule="analysis_coverage_source")]
            pub enum CoverageSource {
                #[model(code=0)] Native {#[model(premise)] coverage:Id<ProviderCoverage>},
                #[model(code=1)] Normalized {#[model(premise)] coverage:Id<NormalizationCoverage>},
                #[model(code=2)] Analysis {#[model(premise)] coverage:Id<AnalysisCoverage>},
                $(#[model(code=$code)] $variant {#[model(premise)] coverage:Id<super::$predecessor::AnalysisCoverage>},)*
            }
            impl CoverageSource {pub fn reference(&self)->derivation::RowRef {match self {Self::Native {coverage}=>derivation::RowRef::of(*coverage),Self::Normalized {coverage}=>derivation::RowRef::of(*coverage),Self::Analysis {coverage}=>derivation::RowRef::of(*coverage),$(Self::$variant {coverage}=>derivation::RowRef::of(*coverage),)*}}}
            #[derive(Debug,Clone,PartialEq,Eq,Hash,DomainSum)]
            #[model(name=owner_table!("obligation_subjects"))]
            pub enum ObligationSubject {
                #[model(code=0)] Entity {entity:Id<EntityRef>},
                #[model(code=1)] SourceCall {occurrence:Id<Occurrence>},
                #[model(code=2)] Transfer {transfer:Id<TransferKey>},
                #[model(code=3)] Computation {invocation:Id<AnalysisInvocation>},
                $(#[model(code=$code)] $variant {invocation:Id<super::$predecessor::AnalysisInvocation>},)*
            }
            fn predecessor_invocation_inputs(inputs:&mut Vec<ValidationInput>) {$(inputs.push(ValidationInput::of::<super::$predecessor::AnalysisInvocation>(&["id"]));)*}
            fn visit_predecessor_invocation(relation:&str,batch:&arrow_array::RecordBatch,frames:&mut charged::ChargedMap<derivation::RowRef,(Id<InputRevision>,Id<AnalysisContext>)>,charge:&mut charged::StateCharge)->Result<bool,ModelError> {$(if relation==super::$predecessor::AnalysisInvocation::NAME {for row in super::$predecessor::AnalysisInvocation::decode(batch)? {frames.insert(charge,derivation::RowRef::of(row.id()),(row.input,row.context))?;}return Ok(true);})* Ok(false)}
            fn predecessor_coverage_inputs(inputs:&mut Vec<ValidationInput>) {$(inputs.push(ValidationInput::of::<super::$predecessor::AnalysisCoverage>(&["id"]));)*}
            fn visit_predecessor_coverage(relation:&str,batch:&arrow_array::RecordBatch,rows:&mut charged::ChargedMap<derivation::RowRef,(Id<CoverageScope>,Id<AnalysisContext>,EvidenceAvailability)>,charge:&mut charged::StateCharge)->Result<bool,ModelError> {$(if relation==super::$predecessor::AnalysisCoverage::NAME {for row in super::$predecessor::AnalysisCoverage::decode(batch)? {rows.insert(charge,derivation::RowRef::of(row.id()),(row.scope,row.context,row.availability))?;}return Ok(true);})* Ok(false)}
            fn predecessor_support_inputs(inputs:&mut Vec<ValidationInput>) {$(inputs.push(ValidationInput::of::<super::$predecessor::AnalysisDerivation>(&["id"]));)*}
            fn visit_predecessor_support(relation:&str,batch:&arrow_array::RecordBatch,rows:&mut charged::ChargedMap<derivation::RowRef,super::support::SourceFacts>,charge:&mut charged::StateCharge)->Result<bool,ModelError> {$(if relation==super::$predecessor::AnalysisDerivation::NAME {for row in super::$predecessor::AnalysisDerivation::decode(batch)? {rows.insert(charge,derivation::RowRef::of(row.id()),row.facts())?;}return Ok(true);})* Ok(false)}
            mod invocation {include!("family/invocation.rs");}
            pub use invocation::*;
            pub mod coverage {include!("family/coverage.rs");}
            pub use coverage::{AnalysisCoverage,AnalysisCoveragePremise,CoverageRequirement,CoverageRequiredSource};
            pub mod support {include!("family/support.rs");}
            pub use support::{AnalysisDerivation,AnalysisDerivationPremise,AnalysisProposition};
            pub mod obligations {include!("family/obligations.rs");}
            pub use obligations::{AnalysisObligation,DischargeEvidence};
            pub type Invocation=AnalysisInvocation;
            pub type Outcome=AnalysisOutcome;
            pub type Diagnostic=AnalysisDiagnostic;
            pub type Coverage=AnalysisCoverage;
            pub type Proposition=AnalysisProposition;
            pub type Derivation=AnalysisDerivation;
            pub type Obligation=AnalysisObligation;
            pub fn relations()->Vec<Relation> {let mut rows=vec![Relation::of::<AnalysisInvocation>(),Relation::of::<AnalysisInput>(),Relation::of::<SourceReceipt>(),Relation::of::<ProjectionInput>(),Relation::of::<AnalysisOutcome>(),Relation::of::<AnalysisDiagnostic>(),Relation::of::<InvocationSource>(),Relation::of::<ObligationSubject>()];rows.extend(coverage::relations());rows.extend(support::relations());rows.extend(obligations::relations());rows}
        }
    };
}
pub(crate) use analysis_family;
