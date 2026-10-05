//! One canonical record/validator template, instantiated for finite publication owners.
//! The predecessor list is static schema, never a producer-selected publication tag.
/// Expand the shared publication schema. Producers still declare and publish every record.
#[macro_export]
macro_rules! analysis_publication {
    ($($apply:ident)::+) => { $($apply)::+! {
        AnalysisInvocation,
        AnalysisInput,
        SourceReceipt,
        ProjectionInput,
        AnalysisOutcome,
        InvocationSource,
        AnalysisCoverage,
        CoverageRequirement,
        CoverageRequiredSource,
        AnalysisCoveragePremise,
        CoverageSource,
    } };
}
#[doc(hidden)]
#[macro_export]
macro_rules! analysis_publication_relations {($($record:ident,)*)=>{vec![$($crate::domain::Relation::of::<$record>(),)*]};}
macro_rules! analysis_family {
    ($owner:ident,$prefix:literal,[$($variant:ident:$code:literal=>$predecessor:ident),* $(,)?],[$($transfer:ty)? $(;$transfer_variant:ident:$transfer_code:literal=>$transfer_type:ty)*],[$($normalized:ty)?],[$($proof_variant:ident:$proof_code:literal=>$proof_type:ty),* $(,)?]) => { analysis_family!(@impl $owner,$prefix,[$($variant:$code=>$predecessor),*],[$($transfer)? $(;$transfer_variant:$transfer_code=>$transfer_type)*],[$($normalized)?],[$($proof_variant:$proof_code=>$proof_type),*],[$($variant:$code=>$predecessor),*],[$($variant:$code=>$predecessor),*]); };
    ($owner:ident,$prefix:literal,[$($variant:ident:$code:literal=>$predecessor:ident),* $(,)?],[$($transfer:ty)? $(;$transfer_variant:ident:$transfer_code:literal=>$transfer_type:ty)*],[$($normalized:ty)?],[$($proof_variant:ident:$proof_code:literal=>$proof_type:ty),* $(,)?],support[$($support_variant:ident:$support_code:literal=>$support_predecessor:ident),* $(,)?],obligations[$($obligation_variant:ident:$obligation_code:literal=>$obligation_predecessor:ident),* $(,)?]) => { analysis_family!(@impl $owner,$prefix,[$($variant:$code=>$predecessor),*],[$($transfer)? $(;$transfer_variant:$transfer_code=>$transfer_type)*],[$($normalized)?],[$($proof_variant:$proof_code=>$proof_type),*],[$($support_variant:$support_code=>$support_predecessor),*],[$($obligation_variant:$obligation_code=>$obligation_predecessor),*]); };
        (@impl $owner:ident,$prefix:literal,[$($variant:ident:$code:literal=>$predecessor:ident),* $(,)?],[$($transfer:ty)? $(;$transfer_variant:ident:$transfer_code:literal=>$transfer_type:ty)*],[$($normalized:ty)?],[$($proof_variant:ident:$proof_code:literal=>$proof_type:ty),* $(,)?],[$($support_variant:ident:$support_code:literal=>$support_predecessor:ident),* $(,)?],[$($obligation_variant:ident:$obligation_code:literal=>$obligation_predecessor:ident),* $(,)?]) => {
        pub mod $owner {
            use super::{invalid,AnalysisDefinition,ProjectionDefinition,AnalysisMethod,AnalysisStatus,AnalysisCapability,AnalysisChannel,Interpretation};
            use crate::domain::{*,assertion::AssertionQualification,attribution::{AnalysisContext,ProviderCoverage,CoverageStatus},source::{CoverageScope,Occurrence},input::InputRevision,normalized::{entities::EntityRef,coverage::{EvidenceAvailability,NormalizationCoverage}},calls::CallPhase,obligation::ObligationKind};
            use crate::{Domain,DomainSum};
            macro_rules! owner_table {($suffix:literal)=>{concat!($prefix,"_",$suffix)}}
            #[derive(Debug,Clone,PartialEq,Eq,Hash,DomainSum, serde::Serialize, serde::Deserialize)]
            #[model(name=owner_table!("invocation_sources"),rule="analysis_invocation_source")]
            pub enum InvocationSource {
                #[model(code=0)] Current {#[model(premise)] invocation:Id<AnalysisInvocation>},
                $(#[model(code=$code)] $variant {#[model(premise)] invocation:Id<super::$predecessor::AnalysisInvocation>},)*
            }
            impl InvocationSource {
                pub fn reference(&self)->derivation::RowRef {match self {Self::Current {invocation}=>derivation::RowRef::of(*invocation),$(Self::$variant {invocation}=>derivation::RowRef::of(*invocation),)*}}
                #[allow(unreachable_patterns,reason="Owners without predecessors have only the current variant")]
                fn current(&self)->Option<Id<AnalysisInvocation>> {match self {Self::Current {invocation}=>Some(*invocation),_=>None}}
            }
            #[derive(Debug,Clone,PartialEq,Eq,Hash,DomainSum, serde::Serialize, serde::Deserialize)]
            #[model(name=owner_table!("support_sources"),rule="analysis_support_source")]
            pub enum SupportSource {
                #[model(code=0)] NativeAssertion {#[model(premise)] premise:Id<super::native::NativeAssertionPremise>},
                #[model(code=1)] AnalysisDerivation {#[model(premise)] derivation:Id<AnalysisDerivation>},
                $(#[model(code=$support_code)] $support_variant {#[model(premise)] derivation:Id<super::$support_predecessor::AnalysisDerivation>},)*
                $(#[model(code=$proof_code)] $proof_variant {#[model(premise)] witness:Id<$proof_type>},)*
            }
            impl SupportSource {
                pub fn reference(&self)->derivation::RowRef {match self {Self::NativeAssertion {premise}=>derivation::RowRef::of(*premise),Self::AnalysisDerivation {derivation}=>derivation::RowRef::of(*derivation),$(Self::$support_variant {derivation}=>derivation::RowRef::of(*derivation),)*$(Self::$proof_variant {witness}=>derivation::RowRef::of(*witness),)*}}
            }
            #[derive(Debug,Clone,PartialEq,Eq,Hash,DomainSum, serde::Serialize, serde::Deserialize)]
            #[model(name=owner_table!("coverage_sources"),rule="analysis_coverage_source")]
            pub enum CoverageSource {
                #[model(code=0)] Native {#[model(premise)] coverage:Id<ProviderCoverage>},
                $(#[model(code=1)] Normalized {#[model(premise)] coverage:Id<$normalized>},)?
                #[model(code=2)] Analysis {#[model(premise)] coverage:Id<AnalysisCoverage>},
                $(#[model(code=$code)] $variant {#[model(premise)] coverage:Id<super::$predecessor::AnalysisCoverage>},)*
            }
            impl CoverageSource {pub fn reference(&self)->derivation::RowRef {match self {Self::Native {coverage}=>derivation::RowRef::of(*coverage),$(Self::Normalized {coverage}=>derivation::RowRef::of::<$normalized>(*coverage),)?Self::Analysis {coverage}=>derivation::RowRef::of(*coverage),$(Self::$variant {coverage}=>derivation::RowRef::of(*coverage),)*}}}
            #[allow(unused_variables,clippy::ptr_arg,reason="Owners without final normalization coverage instantiate an empty repetition")]
            fn normalized_coverage_inputs(inputs:&mut Vec<ValidationInput>) {$(inputs.push(ValidationInput::of::<$normalized>(&["id"]));)?}
            #[allow(unused_variables,unreachable_code,reason="Owners with final normalization coverage return before the fallback error")]
            fn normalized_source(row:&NormalizationCoverage)->Result<CoverageSource,ModelError> {$(let _: &$normalized=row;return Ok(CoverageSource::Normalized {coverage:row.id()});)? Err(invalid("publication owner cannot consume final normalization coverage"))}
            fn normalized_reference(source:&CoverageSource)->Option<Id<NormalizationCoverage>> {match source {$(CoverageSource::Normalized {coverage}=>{let _:Id<$normalized>=*coverage;Some(*coverage)},)?_=>None}}
            fn normalization_enabled()->bool {let types:&[&str]=&[$(stringify!($normalized),)?];!types.is_empty()}
            #[derive(Debug,Clone,PartialEq,Eq,Hash,DomainSum)]
            #[model(name=owner_table!("obligation_subjects"))]
            pub enum ObligationSubject {
                #[model(code=0)] Entity {entity:Id<EntityRef>},
                #[model(code=1)] SourceCall {occurrence:Id<Occurrence>},
                $(#[model(code=2)] Transfer {transfer:Id<$transfer>},)?
                $(#[model(code=$transfer_code)] $transfer_variant {transfer:Id<$transfer_type>},)*
                #[model(code=3)] Computation {invocation:Id<AnalysisInvocation>},
                $(#[model(code=$code)] $variant {invocation:Id<super::$predecessor::AnalysisInvocation>},)*
            }
            impl ObligationSubject {
                /// Canonical semantic subject; invocation subjects keep their exact nominal owner.
                pub fn reference(&self)->derivation::RowRef {match self {
                    Self::Entity {entity}=>derivation::RowRef::of(*entity),
                    Self::SourceCall {occurrence}=>derivation::RowRef::of(*occurrence),
                    $(Self::Transfer {transfer}=>derivation::RowRef::of::<$transfer>(*transfer),)?
                    $(Self::$transfer_variant {transfer}=>derivation::RowRef::of::<$transfer_type>(*transfer),)*
                    Self::Computation {invocation}=>derivation::RowRef::of(*invocation),
                    $(Self::$variant {invocation}=>derivation::RowRef::of(*invocation),)*
                }}
            }
            #[derive(Debug,Clone,PartialEq,Eq,Hash,DomainSum)]
            #[model(name=owner_table!("obligation_sources"),rule="analysis_obligation_source")]
            pub enum ObligationSource {
                #[model(code=0)] Current {#[model(premise)] obligation:Id<AnalysisObligation>},
                $(#[model(code=$obligation_code)] $obligation_variant {#[model(premise)] obligation:Id<super::$obligation_predecessor::AnalysisObligation>},)*
            }
            impl ObligationSource {pub fn reference(&self)->derivation::RowRef {match self {
                Self::Current {obligation}=>derivation::RowRef::of(*obligation),
                $(Self::$obligation_variant {obligation}=>derivation::RowRef::of(*obligation),)*
            }}}
            fn predecessor_obligation_inputs(inputs:&mut Vec<ValidationInput>) {
                $(inputs.push(ValidationInput::of::<super::$obligation_predecessor::AnalysisObligation>(&["id"]));
                  inputs.push(ValidationInput::of::<super::$obligation_predecessor::ObligationSubject>(&["id"]));)*
                predecessor_invocation_inputs(inputs);
            }
            #[allow(unused_variables,unused_imports,reason="Owners without predecessors instantiate an empty repetition")]
            fn visit_predecessor_obligations(
                relation:&str,batch:&arrow_array::RecordBatch,
                questions:&mut charged::ChargedMap<derivation::RowRef,super::obligation_support::Question>,
                subjects:&mut charged::ChargedMap<derivation::RowRef,derivation::RowRef>,
                charge:&mut charged::StateCharge,
            )->Result<bool,ModelError> {
                use super::obligation_support::ObligationQuestion;
                $(if relation==super::$obligation_predecessor::AnalysisObligation::NAME {
                    for row in super::$obligation_predecessor::AnalysisObligation::decode(batch)? {
                        questions.insert(charge,derivation::RowRef::of(row.id()),row.question())?;
                    } return Ok(true);
                }
                if relation==super::$obligation_predecessor::ObligationSubject::NAME {
                    for row in super::$obligation_predecessor::ObligationSubject::decode(batch)? {
                        subjects.insert(charge,derivation::RowRef::of(row.id()),row.reference())?;
                    } return Ok(true);
                })* Ok(false)
            }
            #[allow(unused_variables,clippy::ptr_arg,reason="Owners without predecessors instantiate an empty repetition")]
            fn predecessor_invocation_inputs(inputs:&mut Vec<ValidationInput>) {$(inputs.push(ValidationInput::of::<super::$predecessor::AnalysisInvocation>(&["id"]));)*}
            #[allow(unused_variables,reason="Owners without predecessors instantiate an empty repetition")]
            fn visit_predecessor_invocation(relation:&str,batch:&arrow_array::RecordBatch,frames:&mut charged::ChargedMap<derivation::RowRef,(Id<InputRevision>,Id<AnalysisContext>)>,charge:&mut charged::StateCharge)->Result<bool,ModelError> {$(if relation==super::$predecessor::AnalysisInvocation::NAME {for row in super::$predecessor::AnalysisInvocation::decode(batch)? {frames.insert(charge,derivation::RowRef::of(row.id()),(row.input,row.context))?;}return Ok(true);})* Ok(false)}
            #[allow(unused_variables,clippy::ptr_arg,reason="Owners without predecessors instantiate an empty repetition")]
            fn predecessor_coverage_inputs(inputs:&mut Vec<ValidationInput>) {$(inputs.push(ValidationInput::of::<super::$predecessor::AnalysisCoverage>(&["id"]));)*}
            #[allow(unused_variables,reason="Owners without predecessors instantiate an empty repetition")]
            fn visit_predecessor_coverage(relation:&str,batch:&arrow_array::RecordBatch,rows:&mut charged::ChargedMap<derivation::RowRef,(Id<CoverageScope>,Id<AnalysisContext>,EvidenceAvailability)>,charge:&mut charged::StateCharge)->Result<bool,ModelError> {$(if relation==super::$predecessor::AnalysisCoverage::NAME {for row in super::$predecessor::AnalysisCoverage::decode(batch)? {rows.insert(charge,derivation::RowRef::of(row.id()),(row.scope,row.context,row.availability))?;}return Ok(true);})* Ok(false)}
            #[allow(unused_variables,clippy::ptr_arg,reason="Owners without predecessors instantiate an empty repetition")]
            fn predecessor_support_inputs(inputs:&mut Vec<ValidationInput>) {$(inputs.push(ValidationInput::of::<$proof_type>(&["id"]));)*$(inputs.push(ValidationInput::of::<super::$support_predecessor::AnalysisDerivation>(&["id"]));)*}
            #[allow(unused_variables,reason="Owners without predecessors instantiate an empty repetition")]
            fn visit_predecessor_support(relation:&str,batch:&arrow_array::RecordBatch,rows:&mut charged::ChargedMap<derivation::RowRef,super::support::SourceFacts>,charge:&mut charged::StateCharge)->Result<bool,ModelError> {$(if relation==<$proof_type>::NAME {for row in <$proof_type>::decode(batch)? {rows.insert(charge,derivation::RowRef::of(row.id()),super::support::DerivedEvidence::source_facts(&row))?;}return Ok(true);})*$(if relation==super::$support_predecessor::AnalysisDerivation::NAME {for row in super::$support_predecessor::AnalysisDerivation::decode(batch)? {rows.insert(charge,derivation::RowRef::of(row.id()),row.facts())?;}return Ok(true);})* Ok(false)}
            pub(crate) mod invocation {include!("family/invocation.rs");}
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
            /// The invocation and coverage publication shared by every actual analysis owner.
            pub fn publication_relations()->Vec<Relation> {
                crate::analysis_publication!(crate::analysis_publication_relations)
            }
            pub fn relations()->Vec<Relation> {let mut rows=publication_relations();rows.extend([Relation::of::<AnalysisDiagnostic>(),Relation::of::<ObligationSubject>(),Relation::of::<ObligationSource>()]);rows.extend(support::relations());rows.extend(obligations::relations());rows}
        }
    };
}
pub(crate) use analysis_family;
