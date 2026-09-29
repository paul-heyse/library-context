use std::collections::BTreeSet;
use lctx_model::domain::{assertion::Approximation,attribution::CoverageStatus,conditions::{Diagram,KernelBoundary},obligation::*};
#[test]
fn scopes_approximation_and_refusals_cannot_masquerade_as_negative_or_positive_proofs() {
    let yes = Diagram::always(); let no = Diagram::never();
    fn conclude(condition: Option<&Diagram>,open: &[ObligationKind],coverage: CoverageStatus,approximation: Approximation) -> Conclusion {
        verdict(VerdictInput { condition,open,coverage,approximation })
    }
    let complete = CoverageStatus::CompleteUnderStatedModel;
    assert_eq!(conclude(Some(&yes),&[],complete,Approximation::Exact).verdict,Verdict::Established);
    assert_eq!(conclude(Some(&no),&[],complete,Approximation::Exact).verdict,Verdict::RefutedUnderModel);
    assert_eq!(conclude(Some(&no),&[],CoverageStatus::Partial,Approximation::Exact).verdict,Verdict::Unknown);
    assert_eq!(conclude(Some(&yes),&[ObligationKind::ScopeBoundary],complete,Approximation::Exact).verdict,Verdict::Unknown);
    assert_eq!(conclude(None,&[],CoverageStatus::NotRequested,Approximation::Exact).verdict,Verdict::NotAnalyzed);
    for approximation in [Approximation::Over,Approximation::Under,Approximation::Mixed,Approximation::Unknown] {
        for condition in [&yes,&no] {
            assert_eq!(conclude(Some(condition),&[],complete,approximation),Conclusion { verdict: Verdict::Unknown,reason: Some(ObligationKind::Approximation) });
        }
    }
    for boundary in [KernelBoundary::AtomLimit,KernelBoundary::NodeLimit,KernelBoundary::WorkPreflight,KernelBoundary::TransferUnsupported] {
        assert_eq!(conclude(Some(&no),&[from_kernel(boundary)],complete,Approximation::Exact).verdict,Verdict::Unknown);
    }
    assert_eq!(conclude(Some(&yes),&[ObligationKind::ResponseBudget],complete,Approximation::Exact).verdict,Verdict::Established);
    assert_eq!(conclude(None,&[ObligationKind::NotRequested,ObligationKind::ScopeBoundary],complete,Approximation::Exact).verdict,Verdict::Unknown);
    assert_eq!(first([ObligationKind::MissingEvidence,ObligationKind::ConditionNodeLimit]),first([ObligationKind::ConditionNodeLimit,ObligationKind::MissingEvidence]));
}

#[test]
fn obligations_have_one_priority() {
    use ObligationKind as O;
    assert_eq!(first([O::MissingEvidence,O::UnresolvedTarget,O::ConditionWorkLimit]),Some(O::ConditionWorkLimit),"a budget stop comes first");
    assert_eq!(first([O::NotRequested,O::OverrideDispatch]),Some(O::OverrideDispatch),"an unasked question comes last");
    assert_eq!(first([O::EntryValueUnknown,O::UnresolvedTarget]),Some(O::UnresolvedTarget),"resolution precedes evidence");
    assert_eq!(O::CallTransfer.class(),ObligationClass::Resolution);
    assert_eq!(first([]),None);
    let open = |condition: &Diagram,reason| verdict(VerdictInput { condition: Some(condition),open: &[reason],
        coverage: CoverageStatus::CompleteUnderStatedModel,approximation: Approximation::Exact });
    assert_eq!(open(&Diagram::always(),O::SummaryDepthLimit),Conclusion { verdict: Verdict::Unknown,reason: Some(O::SummaryDepthLimit) },"a cut proof is not a negative");
}

#[test]
fn discharge_needs_every_member_proved() {
    let (a,b,c) = (1u8,2u8,3u8);
    let mut decisions = Decisions::<u8,u16>::default();
    decisions.proof(a,10,Verdict::Established);
    decisions.proof(b,11,Verdict::Conditional);
    decisions.proof(b,9,Verdict::Established);
    decisions.proof(c,12,Verdict::Unknown);
    assert_eq!(decisions.decide(b),Standing::Proved { proof: 9 },"the lowest proof");
    assert_eq!(decisions.decide(c),Standing::Open { obligation: ObligationKind::CallTransfer },"an unknown proof proves nothing");
    let members = BTreeSet::from([a,b]);
    assert!(discharge(&members,&decisions).0);
    assert!(!discharge(&BTreeSet::new(),&decisions).0,"no members prove nothing");
    decisions.open(a,ObligationKind::MissingEvidence);
    decisions.open(a,ObligationKind::ConditionNodeLimit);
    assert_eq!(decisions.decide(a),Standing::Open { obligation: ObligationKind::ConditionNodeLimit },"an obligation overrides a proof; the first by priority is kept");
    assert!(!discharge(&members,&decisions).0);
}

#[test]
fn named_budgets_refuse_with_their_own_obligation() {
    let mut meter = Budget::new(ObligationKind::SummaryPairWorkLimit,10).meter();
    assert_eq!(meter.charge(6),Ok(()));
    assert_eq!(meter.charge(4),Ok(()));
    assert_eq!(meter.charge(1),Err(ObligationKind::SummaryPairWorkLimit));
    assert_eq!(meter.charge(u64::MAX),Err(ObligationKind::SummaryPairWorkLimit),"overflow is exhaustion, never wraparound");
    assert_eq!(meter.used(),10);
    assert_eq!(from_kernel(KernelBoundary::NodeLimit),ObligationKind::ConditionNodeLimit);
}
