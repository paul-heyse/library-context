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
