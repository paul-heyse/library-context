use lctx_model::domain::analysis::coverage::CoverageExpectation;
fn nominal<T>(value:u8)->lctx_model::domain::Id<T> { serde::Deserialize::deserialize(serde::de::value::SeqDeserializer::<_,serde::de::value::Error>::new([value;16].into_iter())).unwrap() }
use lctx_model::domain::{analysis::*,analysis::coverage::*,analysis::support::*,analysis::obligations::*,assertion::*,attribution::*,conditions::*,input::*,normalized::coverage::{NormalizationCoverage,NormalizationComputation,EvidenceAvailability,Capability},obligation::ObligationKind,source::*,transfer::*,*};
use std::collections::BTreeMap;
fn parameters()->MethodParameters { MethodParameters {depth:Some(8),proof_steps:Some(64),work:Some(100),members:None,seed:Some(2),iterations:None,threshold:None,resolution:None,damping:None,model_catalog:None} }
fn context()->AnalysisContext { AnalysisContext {python_version:"3.14.7".into(),python_platform:"linux".into(),search_path:vec![],site_package_path:vec![],config_digest:ContentHash::of(b"c"),environment_digest:ContentHash::of(b"e"),lock_digest:None} }
fn invocation()->AnalysisInvocation {
    AnalysisInvocation::new(InputRevision::from_entries(vec![]).unwrap().id(),context().id(),AnalysisDefinition {method:AnalysisMethod::LocalTransfers,semantic_version:ContentHash::of(b"version"),parameters:parameters().id(),interpretation:Interpretation::Structural}.id(),None,[]).0
}
fn budget()->resources::ResourceBudget { resources::ResourceBudget::fixed(32<<20).unwrap() }
fn invariant<R:Record>(rows:Vec<(&str,arrow_array::RecordBatch)>)->Result<(),ModelError> {
    let invariant=R::invariants().remove(0);
    let rows=rows.into_iter().collect::<BTreeMap<_,_>>();
    let mut check=(invariant.create)(&budget());
    for input in invariant.inputs { if let Some(batch)=rows.get(input.name()) { check.visit(input.name(),batch)?; } }
    check.finish()
}
#[test]
fn invocation_identity_binds_exact_parent_membership_and_diagnostics_do_not_change_it() {
    let parent=invocation();
    let (child,members)=AnalysisInvocation::new(parent.input,parent.context,parent.definition,parent.subject,[parent.id()]);
    let rows=vec![(AnalysisInvocation::NAME,AnalysisInvocation::encode(&[parent.clone(),child.clone()]).unwrap()),(AnalysisInput::NAME,AnalysisInput::encode(&members).unwrap())];
    invariant::<AnalysisInvocation>(rows.clone()).unwrap();
    assert!(invariant::<AnalysisInvocation>(vec![rows[0].clone()]).is_err());
    let diagnostic=AnalysisDiagnostic {invocation:child.id(),elapsed_micros:Some(1),iterations:None,examined_members:Some(3),residual:None,converged:None};
    assert_eq!(diagnostic.id(),AnalysisDiagnostic {elapsed_micros:Some(99),..diagnostic}.id());
    let mut bad=members[0].clone();bad.parent=child.id();
    assert!(invariant::<AnalysisInvocation>(vec![rows[0].clone(),(AnalysisInput::NAME,AnalysisInput::encode(&[bad]).unwrap())]).is_err());
    assert_ne!(child.id(),parent.id());
}
#[test]
fn qualification_keeps_conditions_modalities_and_approximations_without_strengthening() {
    let scope=CoverageScope::Input {input:invocation().input}.id();
    let first=Diagram::from_atom(nominal(1));
    let second=first.not().unwrap();
    let a=AssertionQualification {context:context().id(),scope,condition:first.id(),modality:Modality::Candidate,approximation:Approximation::Over};
    let b=AssertionQualification {condition:second.id(),modality:Modality::Potential,approximation:Approximation::Under,..a.clone()};
    let source_a=SupportSource::AnalysisDerivation {derivation:nominal(1)};
    let source_b=SupportSource::AnalysisDerivation {derivation:nominal(2)};
    let sources=[QualifiedPremise {source:&source_a,qualification:&a,condition:&first},QualifiedPremise {source:&source_b,qualification:&b,condition:&second}];
    let and=qualify(QualificationOperation::Conjunction,&sources).unwrap();
    assert!(and.condition.is_false());assert_eq!(and.qualification.modality,Modality::Potential);assert_eq!(and.qualification.approximation,Approximation::Mixed);
    assert!(qualify(QualificationOperation::AlternativeUnion,&sources).unwrap().condition.is_true());
    let foreign=AssertionQualification {context:nominal(9),..b};
    assert!(qualify(QualificationOperation::Conjunction,&[sources[0],QualifiedPremise {source:&source_b,qualification:&foreign,condition:&second}]).is_err());
    assert!(qualify(QualificationOperation::Conjunction,&[]).is_err());
}
#[test]
fn coverage_requires_exact_nominal_members_and_never_completes_from_empty_observations() {
    let inv=invocation();
    let scope=CoverageScope::Input {input:inv.input}.id();
    let computation=NormalizationComputation {capability:Capability::Bindings,policy:ContentHash::of(b"policy"),producer:"bindings".into(),declaration:ContentHash::of(b"declaration"),profile:"catalog".into(),availability:EvidenceAvailability::Unavailable};
    let normalized=NormalizationCoverage {computation:computation.id(),scope,context:inv.context,availability:EvidenceAvailability::Unavailable};
    let observed=CoverageObservation::normalized(&normalized);
    let expected=CoverageExpectation {invocation:inv.id(),capability:AnalysisCapability::Transfers,scope,context:inv.context,requested:true,no_scope:false,sources:vec![observed.source().id()]};
    let (row,members)=assess(&expected,std::slice::from_ref(&observed),AnalysisStatus::Completed,None,&budget()).unwrap();
    assert_eq!(row.availability,EvidenceAvailability::Unavailable);assert_eq!(members.len(),1);
    assert!(assess(&expected,&[],AnalysisStatus::Completed,None,&budget()).is_err());
    assert!(assess(&expected,&[observed.clone(),observed.clone()],AnalysisStatus::Completed,None,&budget()).is_err());
    let empty=CoverageExpectation {sources:vec![],no_scope:true,..expected.clone()};
    assert_eq!(assess(&empty,&[],AnalysisStatus::Completed,None,&budget()).unwrap().0.availability,EvidenceAvailability::NoScope);
    let unasked=CoverageExpectation {requested:false,no_scope:false,..empty};
    assert_eq!(assess(&unasked,&[],AnalysisStatus::NotRequested,Some(ObligationKind::NotRequested),&budget()).unwrap().0.availability,EvidenceAvailability::NotRequested);
    let missing=CoverageExpectation {sources:vec![],..expected};
    assert!(assess(&missing,&[],AnalysisStatus::Completed,None,&budget()).is_err());
}
#[test]
fn native_and_derived_sources_have_nominal_proof_edges_and_analysis_support_has_no_run_field() {
    let source=SupportSource::AnalysisDerivation {derivation:nominal(3)};
    assert_eq!(source.proof().unwrap().premises[0].relation(),AnalysisDerivation::NAME);
    assert!(TransferSupport::fields().iter().all(|field|field.name() != "run"));
    assert_eq!(TransferSupport::derivation().unwrap().premises[0].target().1,SupportSource::NAME);
    assert_eq!(AnalysisDerivationPremise::derivation().unwrap().conclusion.unwrap().target().1,AnalysisDerivation::NAME);
    let obligation=AnalysisObligation {invocation:invocation().id(),subject:ObligationSubject::Computation {invocation:invocation().id()}.id(),channel:AnalysisChannel::Value,phase:calls::CallPhase::Call,qualification:nominal(4),responsible:AnalysisMethod::LocalTransfers,reason:ObligationKind::ResponseBudget};
    assert!(obligation.validate().is_err());
}
#[test]
fn stored_coverage_refuses_erased_members_and_producer_selected_strengthening() {
    let inv=invocation();let scope=CoverageScope::Input {input:inv.input}.id();
    let lower=NormalizationCoverage {computation:nominal(7),scope,context:inv.context,availability:EvidenceAvailability::Partial};
    let observation=CoverageObservation::normalized(&lower);
    let expectation=CoverageExpectation {invocation:inv.id(),capability:AnalysisCapability::Transfers,scope,context:inv.context,requested:true,no_scope:false,sources:vec![observation.source().id()]};
    let (requirement,required)=expectation.records().unwrap();
    let (coverage,members)=assess(&expectation,&[observation.clone()],AnalysisStatus::Completed,None,&budget()).unwrap();
    let outcome=AnalysisOutcome {invocation:inv.id(),status:AnalysisStatus::Completed,reason:None};
    let mut rows=vec![(AnalysisInvocation::NAME,AnalysisInvocation::encode(&[inv]).unwrap()),(AnalysisOutcome::NAME,AnalysisOutcome::encode(&[outcome]).unwrap()),(CoverageRequirement::NAME,CoverageRequirement::encode(&[requirement]).unwrap()),(CoverageRequiredSource::NAME,CoverageRequiredSource::encode(&required).unwrap()),(AnalysisCoverage::NAME,AnalysisCoverage::encode(&[coverage.clone()]).unwrap()),(AnalysisCoveragePremise::NAME,AnalysisCoveragePremise::encode(&members).unwrap()),(CoverageSource::NAME,<CoverageSource as Record>::encode(&[observation.source().clone()]).unwrap()),(NormalizationCoverage::NAME,NormalizationCoverage::encode(&[lower]).unwrap())];
    invariant::<AnalysisCoverage>(rows.clone()).unwrap();
    let missing=rows.iter().filter(|(name,_)|*name!=AnalysisCoveragePremise::NAME).cloned().collect();
    assert!(invariant::<AnalysisCoverage>(missing).unwrap_err().to_string().contains("exact lower evidence"));
    rows.iter_mut().find(|(name,_)|*name==AnalysisCoverage::NAME).unwrap().1=AnalysisCoverage::encode(&[AnalysisCoverage {availability:EvidenceAvailability::Complete,reason:None,..coverage}]).unwrap();
    assert!(invariant::<AnalysisCoverage>(rows).unwrap_err().to_string().contains("exact lower evidence"));
}
#[test]
fn discharge_matches_the_exact_question_and_keeps_partial_negative_and_candidates_open() {
    let inv=invocation();let condition=Diagram::always();
    let q=AssertionQualification {context:inv.context,scope:CoverageScope::Input {input:inv.input}.id(),condition:condition.id(),modality:Modality::Definite,approximation:Approximation::Exact};
    let subject=ObligationSubject::Computation {invocation:inv.id()};
    let proposition=AnalysisProposition {subject:subject.id(),channel:AnalysisChannel::Value,phase:calls::CallPhase::Call,qualification:q.id()};
    let open=AnalysisObligation {invocation:inv.id(),subject:subject.id(),channel:proposition.channel,phase:proposition.phase,qualification:q.id(),reason:ObligationKind::MissingEvidence,responsible:AnalysisMethod::LocalTransfers};
    let coverage=AnalysisCoverage {invocation:inv.id(),capability:AnalysisCapability::Transfers,scope:q.scope,context:q.context,premises:ContentHash::of(b"membership"),availability:EvidenceAvailability::Partial,reason:Some(ObligationKind::IncompleteCoverage)};
    assert_eq!(admissible_discharge(&open,&proposition,&q,&condition,&coverage).unwrap().verdict,obligation::Verdict::Established);
    assert!(admissible_discharge(&open,&AnalysisProposition {subject:nominal(8),..proposition.clone()},&q,&condition,&coverage).is_err());
    assert!(admissible_discharge(&open,&AnalysisProposition {phase:calls::CallPhase::PropertyGet,..proposition.clone()},&q,&condition,&coverage).is_err());
    let candidate=AssertionQualification {modality:Modality::Candidate,..q.clone()};
    let negative=Diagram::never();
    for altered in [candidate,AssertionQualification {condition:negative.id(),..q.clone()}] {
        let condition=if altered.condition==negative.id() {&negative} else {&condition};
        let open=AnalysisObligation {qualification:altered.id(),..open.clone()};
        let proposition=AnalysisProposition {qualification:altered.id(),..proposition.clone()};
        assert!(admissible_discharge(&open,&proposition,&altered,condition,&coverage).is_err());
    }
}
#[test]
fn statistical_and_documentary_status_policy_never_turn_navigation_into_behavior() {
    use analysis::findings::{self,EvidenceStatus as S,SupportRole as R,AssertionKind as K,FindingKind as F};
    assert_eq!(findings::derive_status(&[(R::Support,S::Documented),(R::Scope,S::StatisticallyDerived)]),S::StatisticallyDerived);
    assert_eq!(findings::derive_status(&[(R::Support,S::Unresolved),(R::Support,S::FixtureChecked)]),S::Unresolved);
    assert_eq!(findings::derive_status(&[(R::Scope,S::FixtureChecked)]),S::Unresolved);
    assert!(findings::assertion_policy(K::Outcome,S::StructurallyObserved).is_err());
    assert!(findings::assertion_policy(K::Outcome,S::Documented).is_ok());
    assert!(findings::assertion_policy(K::Control,S::StatisticallyDerived).is_err());
    assert!(findings::finding_policy(F::Forwarding,S::StatisticallyDerived).is_err());
    assert!(findings::finding_policy(F::Forwarding,S::Documented).is_err());
    assert!(findings::finding_policy(F::Community,S::StatisticallyDerived).is_ok());
    let bad=AnalysisDefinition {method:AnalysisMethod::PageRank,semantic_version:ContentHash::of(b"v"),parameters:parameters().id(),interpretation:Interpretation::Structural};
    assert!(bad.validate().is_err());
    assert!(AnalysisDefinition {interpretation:Interpretation::Heuristic,..bad}.validate().is_ok());
}

#[path="fixtures/analysis_support.rs"]
mod support_fixture;
fn frame<R:Record>(rows:&[R])->(&'static str,arrow_array::RecordBatch) {(R::NAME,R::encode(rows).unwrap())}
#[test]
fn emitter_preserves_partial_coverage_and_refuses_statistical_control_and_forged_stored_status() {
    use analysis::findings::{self,Finding,FindingSupport,FindingEvidence,FindingKind as F,SupportRole as R,EvidenceStatus as S};
    let initial=invocation();let condition=Diagram::always();
    let q=AssertionQualification {context:initial.context,scope:CoverageScope::Input {input:initial.input}.id(),condition:condition.id(),modality:Modality::Candidate,approximation:Approximation::Over};
    let native=support_fixture::SupportFixture::new(initial.input,nominal(1),nominal(2),nominal(3),&q,&condition,nominal(4),nominal(5));
    let evidence=FindingEvidence::native(R::Support,&native.premise,&native.native,&native.observation,&native.support,&q,&condition).unwrap();
    let coverage=AnalysisCoverage {invocation:native.invocation.id(),capability:AnalysisCapability::Transfers,scope:q.scope,context:q.context,premises:ContentHash::of(b"membership"),availability:EvidenceAvailability::Partial,reason:Some(ObligationKind::IncompleteCoverage)};
    let (finding,_,supports,result)=findings::emit(native.invocation.id(),F::Forwarding,native.subject.id(),&[],&coverage,&[evidence],&budget()).unwrap();
    assert_eq!(result.qualification,q);assert_eq!(finding.status,S::StructurallyObserved);
    let rows=vec![frame(&[finding.clone()]),frame::<FindingSupport>(&supports),frame(&[coverage.clone()]),frame(&[q.clone()]),frame(&[condition.records().0]),frame(&condition.records().1),frame(&[native.native.clone()]),frame(&[native.premise.clone()]),frame(&[native.observation.clone()]),frame(&[native.support.clone()])];
    invariant::<Finding>(rows.clone()).unwrap();
    let mut forged=rows;forged.iter_mut().find(|(name,_)|*name==Finding::NAME).unwrap().1=Finding::encode(&[Finding {status:S::Documented,..finding}]).unwrap();
    assert!(invariant::<Finding>(forged).is_err());
    let definition=AnalysisDefinition {method:AnalysisMethod::PageRank,semantic_version:ContentHash::of(b"rank"),parameters:native.parameters.id(),interpretation:Interpretation::Heuristic};
    let inv=AnalysisInvocation::new(initial.input,initial.context,definition.id(),None,[]).0;
    let (derivation,_,_,result)=AnalysisDerivation::emit(inv.id(),native.subject.id(),AnalysisChannel::Value,calls::CallPhase::Call,QualificationOperation::Conjunction,&[QualifiedPremise {source:&native.native,qualification:&q,condition:&condition}]).unwrap();
    let source=SupportSource::AnalysisDerivation {derivation:derivation.id()};
    let native_evidence=FindingEvidence::native(R::Support,&native.premise,&native.native,&native.observation,&native.support,&q,&condition).unwrap();
    let statistical=FindingEvidence::derived(R::Scope,&source,&derivation,&inv,&definition,&result.qualification,&result.condition,&[native_evidence],&budget()).unwrap();
    let native_evidence=FindingEvidence::native(R::Support,&native.premise,&native.native,&native.observation,&native.support,&q,&condition).unwrap();
    let coverage=AnalysisCoverage {invocation:inv.id(),..coverage};
    assert!(findings::emit(inv.id(),F::Forwarding,native.subject.id(),&[],&coverage,&[native_evidence,statistical],&budget()).is_err());
}
#[test]
fn stored_derivation_refuses_strengthening_and_a_native_support_for_another_assertion() {
    let inv=invocation();let condition=Diagram::always();
    let q=AssertionQualification {context:inv.context,scope:CoverageScope::Input {input:inv.input}.id(),condition:condition.id(),modality:Modality::Candidate,approximation:Approximation::Over};
    let native=support_fixture::SupportFixture::new(inv.input,nominal(1),nominal(2),nominal(3),&q,&condition,nominal(4),nominal(5));
    let rows=vec![frame(&[native.invocation.clone()]),frame(&[q.clone()]),frame(&[condition.records().0]),frame(&condition.records().1),frame(&[native.observation.clone()]),frame(&[native.support.clone()]),frame(&[native.premise.clone()]),frame(&[native.native.clone()]),frame(&[native.proposition.clone()]),frame(&[native.derivation.clone()]),frame(&native.members)];
    invariant::<AnalysisDerivation>(rows.clone()).unwrap();
    let stronger=AssertionQualification {modality:Modality::Definite,approximation:Approximation::Exact,..q.clone()};
    let proposition=AnalysisProposition {qualification:stronger.id(),..native.proposition.clone()};
    let derivation=AnalysisDerivation {qualification:stronger.id(),proposition:proposition.id(),..native.derivation.clone()};
    let members=native.members.iter().cloned().map(|r|AnalysisDerivationPremise {derivation:derivation.id(),..r}).collect::<Vec<_>>();
    let mut forged=rows.clone();
    forged.iter_mut().find(|(name,_)|*name==AssertionQualification::NAME).unwrap().1=AssertionQualification::encode(&[q,stronger]).unwrap();
    forged.iter_mut().find(|(name,_)|*name==AnalysisProposition::NAME).unwrap().1=AnalysisProposition::encode(&[proposition]).unwrap();
    forged.iter_mut().find(|(name,_)|*name==AnalysisDerivation::NAME).unwrap().1=AnalysisDerivation::encode(&[derivation]).unwrap();
    forged.iter_mut().find(|(name,_)|*name==AnalysisDerivationPremise::NAME).unwrap().1=AnalysisDerivationPremise::encode(&members).unwrap();
    assert!(invariant::<AnalysisDerivation>(forged).unwrap_err().to_string().contains("strengthens"));
    let wrong=flow::FlowValueSupport {assertion:nominal(99),..native.support};
    let premise=NativeAssertionPremise::Value {assertion:native.observation.id(),support:wrong.id()};
    let source=SupportSource::NativeAssertion {premise:premise.id()};
    let mut mismatched=rows.into_iter().filter(|(name,_)|![AnalysisProposition::NAME,AnalysisDerivation::NAME,AnalysisDerivationPremise::NAME,flow::FlowValueSupport::NAME,NativeAssertionPremise::NAME,SupportSource::NAME].contains(name)).collect::<Vec<_>>();
    mismatched.extend([frame(&[wrong]),frame(&[premise]),frame(&[source])]);
    assert!(invariant::<AnalysisDerivation>(mismatched).unwrap_err().to_string().contains("another assertion"));
}
#[test]
fn admitted_bdd_apply_refuses_before_allocation_and_reservation_follows_the_result() {
    let left=Diagram::from_atom(nominal(1));let right=Diagram::from_atom(nominal(2));
    let allowance=left.binary_allocation_allowance(&right).unwrap();
    let refused=resources::ResourceBudget::fixed(allowance-1).unwrap();
    assert!(matches!(left.admitted_binary(&right,BooleanOperation::Conjunction,&refused),Err(DiagramAdmissionError::Resource(_))));
    assert_eq!(refused.reserved(),0);
    let budget=resources::ResourceBudget::fixed(allowance).unwrap();
    let result=left.admitted_binary(&right,BooleanOperation::Conjunction,&budget).unwrap();
    assert_eq!(result.id(),left.and(&right).unwrap().id());
    assert!(budget.peak().unwrap()>=allowance);
    assert_eq!(budget.reserved(),result.reserved_bytes());
    assert!(budget.reserved()>0);
    let (diagram,reservation)=result.into_parts();
    assert!(budget.reserved()>0);assert_eq!(diagram.id(),left.and(&right).unwrap().id());
    drop(diagram);drop(reservation);assert_eq!(budget.reserved(),0);
    let union=left.admitted_binary(&left.not().unwrap(),BooleanOperation::Disjunction,&budget).unwrap();
    assert!(union.is_true());drop(union);assert_eq!(budget.reserved(),0);
}
