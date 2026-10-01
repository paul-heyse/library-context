use lctx_model::domain::{
    analysis::{
        self,
        findings::{self, FindingEvidence, *},
        native::NativeQualification,
        synthesis::{support::EvidencePremise, *},
        *,
    },
    assertion::{Approximation, AssertionQualification},
    attribution::{FactFamily, Fidelity, Modality},
    conditions::Diagram,
    normalized::coverage::EvidenceAvailability,
    obligation::ObligationKind,
    source::CoverageScope,
    *,
};
fn nominal<T>(v: u8) -> Id<T> {
    serde::Deserialize::deserialize(serde::de::value::SeqDeserializer::<
        _,
        serde::de::value::Error,
    >::new([v; 16].into_iter()))
    .unwrap()
}
fn budget() -> resources::ResourceBudget {
    resources::ResourceBudget::fixed(32 << 20).unwrap()
}
fn frame<R: Record>(rows: &[R]) -> (&'static str, arrow_array::RecordBatch) {
    (R::NAME, R::encode(rows).unwrap())
}
fn check(frames: Vec<(&str, arrow_array::RecordBatch)>) -> Result<(), ModelError> {
    let invariant = Finding::invariants().remove(0);
    let rows = frames
        .into_iter()
        .collect::<std::collections::BTreeMap<_, _>>();
    let mut check = (invariant.create)(&budget());
    for input in invariant.inputs {
        if let Some(batch) = rows.get(input.name()) {
            check.visit(input.name(), batch)?;
        }
    }
    check.finish()
}
fn invocation() -> Invocation {
    Invocation::new(nominal(1), nominal(2), nominal(3), None, []).0
}
fn q(inv: &Invocation, condition: &Diagram) -> AssertionQualification {
    AssertionQualification {
        context: inv.context,
        scope: CoverageScope::Input { input: inv.input }.id(),
        condition: condition.id(),
        modality: Modality::Candidate,
        approximation: Approximation::Over,
    }
}
fn coverage(inv: &Invocation, q: &AssertionQualification) -> Coverage {
    Coverage {
        invocation: inv.id(),
        capability: AnalysisCapability::Synthesis,
        scope: q.scope,
        context: q.context,
        premises: ContentHash::of(b"membership"),
        availability: EvidenceAvailability::Partial,
        reason: Some(ObligationKind::IncompleteCoverage),
    }
}
#[test]
fn emitter_keeps_partial_frame_and_refuses_forged_stored_status() {
    let inv = invocation();
    let condition = Diagram::always();
    let q = q(&inv, &condition);
    let coverage = coverage(&inv, &q);
    let native = NativeQualification {
        premise: nominal(5),
        qualification: q.id(),
        family: FactFamily::Flow,
        fidelity: Fidelity::NativeStructural,
        status: EvidenceStatus::StructurallyObserved,
    };
    let source = SupportSource::NativeAssertion {
        premise: native.premise,
    };
    let subject = ObligationSubject::SourceCall {
        occurrence: nominal(7),
    };
    let evidence = FindingEvidence::new(
        SupportRole::Support,
        EvidencePremise::native(&source, &native, &q, &condition).unwrap(),
    );
    let (finding, members, supports, result) = findings::emit(
        inv.id(),
        FindingKind::Forwarding,
        subject.id(),
        &[],
        &coverage,
        &[evidence],
        &budget(),
    )
    .unwrap();
    assert_eq!(result.qualification, q);
    assert_eq!(finding.status, EvidenceStatus::StructurallyObserved);
    assert_eq!(coverage.availability, EvidenceAvailability::Partial);
    let (stored, nodes) = condition.records();
    let frames = vec![
        frame(std::slice::from_ref(&finding)),
        frame(&members),
        frame(&supports),
        frame(&[coverage]),
        frame(&[native]),
        frame(&[source]),
        frame(&[q]),
        frame(&[stored]),
        frame(&nodes),
    ];
    check(frames.clone()).unwrap();
    let mut forged = frames;
    forged[0] = frame(&[Finding {
        status: EvidenceStatus::Documented,
        ..finding
    }]);
    assert!(check(forged).is_err());
}
#[test]
fn documentary_evidence_is_extractive_and_never_behavioral_proof() {
    let inv = invocation();
    let condition = Diagram::always();
    let q = q(&inv, &condition);
    let coverage = coverage(&inv, &q);
    let native = NativeQualification {
        premise: nominal(5),
        qualification: q.id(),
        family: FactFamily::Docs,
        fidelity: Fidelity::NativeStructural,
        status: EvidenceStatus::Documented,
    };
    let source = SupportSource::NativeAssertion {
        premise: native.premise,
    };
    let evidence = FindingEvidence::new(
        SupportRole::Support,
        EvidencePremise::native(&source, &native, &q, &condition).unwrap(),
    );
    assert!(
        findings::emit(
            inv.id(),
            FindingKind::Forwarding,
            nominal(7),
            &[],
            &coverage,
            &[evidence],
            &budget()
        )
        .is_err()
    );
    assert!(assertion_policy(AssertionKind::Outcome, EvidenceStatus::Documented).is_ok());
    assert!(assertion_policy(AssertionKind::Control, EvidenceStatus::Documented).is_err());
}
#[test]
fn any_heuristic_scope_lineage_refuses_behavior_even_with_stronger_or_unavailable_siblings() {
    let inv = invocation();
    let condition = Diagram::always();
    let q = q(&inv, &condition);
    let coverage = coverage(&inv, &q);
    let native = NativeQualification {
        premise: nominal(5),
        qualification: q.id(),
        family: FactFamily::Flow,
        fidelity: Fidelity::NativeStructural,
        status: EvidenceStatus::StructurallyObserved,
    };
    let source = SupportSource::NativeAssertion {
        premise: native.premise,
    };
    let statistical = analysis::synthesis::Derivation {
        invocation: nominal(6),
        proposition: nominal(7),
        qualification: q.id(),
        operation: analysis::support::QualificationOperation::Conjunction,
        inputs: ContentHash::of(b"analytic-members"),
        status: EvidenceStatus::StatisticallyDerived,
        heuristic: true,
    };
    let analytic = SupportSource::AnalysisDerivation {
        derivation: statistical.id(),
    };
    for availability in [
        EvidenceAvailability::Partial,
        EvidenceAvailability::Unavailable,
    ] {
        let coverage = Coverage {
            availability,
            ..coverage.clone()
        };
        let strong = FindingEvidence::new(
            SupportRole::Support,
            EvidencePremise::native(&source, &native, &q, &condition).unwrap(),
        );
        let scope = FindingEvidence::new(
            SupportRole::Scope,
            EvidencePremise::derived(&analytic, &statistical, &q, &condition).unwrap(),
        );
        assert!(
            findings::emit(
                inv.id(),
                FindingKind::Forwarding,
                nominal(8),
                &[],
                &coverage,
                &[strong, scope],
                &budget()
            )
            .is_err()
        );
    }
    let strong = FindingEvidence::new(
        SupportRole::Support,
        EvidencePremise::native(&source, &native, &q, &condition).unwrap(),
    );
    let scope = FindingEvidence::new(
        SupportRole::Scope,
        EvidencePremise::derived(&analytic, &statistical, &q, &condition).unwrap(),
    );
    let (finding, _, _, _) = findings::emit(
        inv.id(),
        FindingKind::Community,
        nominal(8),
        &[],
        &coverage,
        &[strong, scope],
        &budget(),
    )
    .unwrap();
    assert_eq!(finding.status, EvidenceStatus::StatisticallyDerived);
}
