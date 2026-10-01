use lctx_model::domain::analysis::local::coverage::CoverageExpectation;
use lctx_model::domain::normalized::coverage::EvidenceAvailability;
// Controls retained across the immutable owner migration; owner-family controls are adjacent.
use lctx_model::domain::{
    analysis::{
        local::{coverage::*, obligations::*, support::*, *},
        *,
    },
    assertion::*,
    attribution::*,
    conditions::*,
    obligation::ObligationKind,
    source::*,
    transfer::local::TransferSupport,
    *,
};
fn nominal<T>(value: u8) -> Id<T> {
    serde::Deserialize::deserialize(serde::de::value::SeqDeserializer::<
        _,
        serde::de::value::Error,
    >::new([value; 16].into_iter()))
    .unwrap()
}
fn invocation() -> Invocation {
    Invocation::new(nominal(1), nominal(2), nominal(3), None, []).0
}
fn budget() -> resources::ResourceBudget {
    resources::ResourceBudget::fixed(32 << 20).unwrap()
}
#[test]
fn discharge_matches_the_exact_question_and_keeps_partial_negative_and_candidates_open() {
    let inv = invocation();
    let condition = Diagram::always();
    let q = AssertionQualification {
        context: inv.context,
        scope: CoverageScope::Input { input: inv.input }.id(),
        condition: condition.id(),
        modality: Modality::Definite,
        approximation: Approximation::Exact,
    };
    let subject = ObligationSubject::Computation {
        invocation: inv.id(),
    };
    let proposition = AnalysisProposition {
        subject: subject.id(),
        channel: AnalysisChannel::Value,
        phase: calls::CallPhase::Call,
        qualification: q.id(),
    };
    let open = AnalysisObligation {
        invocation: inv.id(),
        subject: subject.id(),
        channel: proposition.channel,
        phase: proposition.phase,
        qualification: q.id(),
        reason: ObligationKind::MissingEvidence,
        responsible: AnalysisMethod::LocalTransfers,
    };
    let coverage = AnalysisCoverage {
        invocation: inv.id(),
        capability: AnalysisCapability::Transfers,
        scope: q.scope,
        context: q.context,
        premises: ContentHash::of(b"membership"),
        availability: EvidenceAvailability::Partial,
        reason: Some(ObligationKind::IncompleteCoverage),
    };
    assert_eq!(
        admissible_discharge(&open, &proposition, &q, &condition, &coverage)
            .unwrap()
            .verdict,
        obligation::Verdict::Established
    );
    assert!(
        admissible_discharge(
            &open,
            &AnalysisProposition {
                subject: nominal(8),
                ..proposition.clone()
            },
            &q,
            &condition,
            &coverage
        )
        .is_err()
    );
    assert!(
        admissible_discharge(
            &open,
            &AnalysisProposition {
                phase: calls::CallPhase::PropertyGet,
                ..proposition.clone()
            },
            &q,
            &condition,
            &coverage
        )
        .is_err()
    );
    let candidate = AssertionQualification {
        modality: Modality::Candidate,
        ..q.clone()
    };
    let negative = Diagram::never();
    for altered in [
        candidate,
        AssertionQualification {
            condition: negative.id(),
            ..q.clone()
        },
    ] {
        let condition = if altered.condition == negative.id() {
            &negative
        } else {
            &condition
        };
        let open = AnalysisObligation {
            qualification: altered.id(),
            ..open.clone()
        };
        let proposition = AnalysisProposition {
            qualification: altered.id(),
            ..proposition.clone()
        };
        assert!(admissible_discharge(&open, &proposition, &altered, condition, &coverage).is_err());
    }
}

#[test]
fn coverage_distinguishes_requested_empty_from_absent_and_unrequested_evidence() {
    let inv = invocation();
    let expected = CoverageExpectation {
        invocation: inv.id(),
        capability: AnalysisCapability::Transfers,
        scope: CoverageScope::Input { input: inv.input }.id(),
        context: inv.context,
        requested: true,
        no_scope: false,
        sources: vec![],
    };
    assert!(assess(&expected, &[], AnalysisStatus::Completed, None, &budget()).is_err());
    let empty = CoverageExpectation {
        no_scope: true,
        ..expected.clone()
    };
    assert_eq!(
        assess(&empty, &[], AnalysisStatus::Completed, None, &budget())
            .unwrap()
            .0
            .availability,
        normalized::coverage::EvidenceAvailability::NoScope
    );
    let unrequested = CoverageExpectation {
        requested: false,
        ..expected
    };
    assert_eq!(
        assess(
            &unrequested,
            &[],
            AnalysisStatus::NotRequested,
            Some(ObligationKind::NotRequested),
            &budget()
        )
        .unwrap()
        .0
        .availability,
        normalized::coverage::EvidenceAvailability::NotRequested
    );
    assert!(
        assess(
            &unrequested,
            &[],
            AnalysisStatus::Completed,
            None,
            &budget()
        )
        .is_err()
    );
}
#[test]
fn derived_companions_keep_nominal_proof_edges_without_provider_run() {
    let fields = TransferSupport::fields();
    assert!(fields.iter().any(|f| f.name() == "source"));
    assert!(!fields.iter().any(|f| f.name() == "run"));
    let support = TransferSupport {
        assertion: nominal(5),
        source: nominal(6),
    };
    let proof = support.proof().unwrap();
    assert_eq!(proof.premises[0], derivation::RowRef::of(support.source));
    let open = AnalysisObligation {
        invocation: invocation().id(),
        subject: nominal(7),
        channel: AnalysisChannel::Value,
        phase: calls::CallPhase::Call,
        qualification: nominal(8),
        reason: ObligationKind::ResponseBudget,
        responsible: AnalysisMethod::LocalTransfers,
    };
    assert!(open.validate().is_err());
}
#[test]
fn admitted_bdd_apply_refuses_before_allocation_and_reservation_follows_the_result() {
    let left = Diagram::from_atom(nominal(1));
    let right = Diagram::from_atom(nominal(2));
    let allowance = left.binary_allocation_allowance(&right).unwrap();
    let refused = resources::ResourceBudget::fixed(allowance - 1).unwrap();
    assert!(matches!(
        left.admitted_binary(&right, BooleanOperation::Conjunction, &refused),
        Err(DiagramAdmissionError::Resource(_))
    ));
    assert_eq!(refused.reserved(), 0);
    let budget = resources::ResourceBudget::fixed(allowance).unwrap();
    let result = left
        .admitted_binary(&right, BooleanOperation::Conjunction, &budget)
        .unwrap();
    assert_eq!(result.id(), left.and(&right).unwrap().id());
    assert!(budget.peak().unwrap() >= allowance);
    assert_eq!(budget.reserved(), result.reserved_bytes());
    assert!(budget.reserved() > 0);
    let (diagram, reservation) = result.into_parts();
    assert!(budget.reserved() > 0);
    assert_eq!(diagram.id(), left.and(&right).unwrap().id());
    drop(diagram);
    drop(reservation);
    assert_eq!(budget.reserved(), 0);
    let union = left
        .admitted_binary(&left.not().unwrap(), BooleanOperation::Disjunction, &budget)
        .unwrap();
    assert!(union.is_true());
    drop(union);
    assert_eq!(budget.reserved(), 0);
}
