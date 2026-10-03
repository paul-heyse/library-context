use lctx_model::domain::{
    analysis::{
        self,
        local::{self, coverage::*, support::*, *},
        native::NativeQualification,
        policy::EvidenceStatus,
        *,
    },
    assertion::{Approximation, AssertionQualification, DerivedSupportSource},
    attribution::{FactFamily, Fidelity, Modality},
    conditions::Diagram,
    normalized::coverage::{EvidenceAvailability, NormalizationCoverage},
    obligation::ObligationKind,
    source::CoverageScope,
    *,
};
use std::collections::BTreeMap;
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
fn definition(method: AnalysisMethod, interpretation: Interpretation) -> AnalysisDefinition {
    AnalysisDefinition {
        method,
        interpretation,
        semantic_version: ContentHash::of(b"test-version"),
        parameters: nominal(3),
    }
}
fn invocation(definition: &AnalysisDefinition) -> local::Invocation {
    local::Invocation::new(nominal(1), nominal(2), definition.id(), None, []).0
}
fn check<R: Record>(frames: Vec<(&str, arrow_array::RecordBatch)>) -> Result<(), ModelError> {
    let invariant = R::invariants().remove(0);
    let rows = frames.into_iter().collect::<BTreeMap<_, _>>();
    let mut check = (invariant.create)(&budget());
    for input in invariant.inputs {
        if let Some(batch) = rows.get(input.name()) {
            check.visit(input.name(), batch)?;
        }
    }
    check.finish()
}
#[test]
fn owners_are_distinct_and_early_manifest_excludes_future_results() {
    assert_ne!(local::Invocation::NAME, analysis::summary::Invocation::NAME);
    assert_ne!(
        local::SupportSource::NAME,
        analysis::synthesis::SupportSource::NAME
    );
    let inputs = local::Invocation::invariants().remove(0).inputs;
    assert!(
        !inputs
            .iter()
            .any(|i| i.name() == "dispatch_analysis_invocations")
    );
    assert!(
        !inputs
            .iter()
            .any(|i| i.name() == analysis::summary::Invocation::NAME)
    );
    assert!(
        !analysis::early_relations()
            .iter()
            .any(|r| r.name() == local::Invocation::NAME)
    );
    lctx_model::domain::model().unwrap();
}
#[test]
fn typed_parent_membership_and_global_proof_edges_preserve_owner_frame() {
    let def = definition(AnalysisMethod::LocalTransfers, Interpretation::Structural);
    let parent = invocation(&def);
    let source = InvocationSource::Current {
        invocation: parent.id(),
    };
    let (child, members) = Invocation::new(
        parent.input,
        parent.context,
        parent.definition,
        None,
        [source.id()],
    );
    let frames = vec![
        frame(&[parent.clone(), child.clone()]),
        frame(std::slice::from_ref(&source)),
        frame(&members),
    ];
    check::<Invocation>(frames.clone()).unwrap();
    assert!(check::<Invocation>(frames[..2].to_vec()).is_err());
    let mut foreign = parent.clone();
    foreign.context = nominal(9);
    let foreign_source = InvocationSource::Current {
        invocation: foreign.id(),
    };
    let (bad, edges) = Invocation::new(
        parent.input,
        parent.context,
        parent.definition,
        None,
        [foreign_source.id()],
    );
    assert!(
        check::<Invocation>(vec![
            frame(&[foreign, bad]),
            frame(&[foreign_source]),
            frame(&edges)
        ])
        .is_err()
    );
    let proof = members[0].proof().unwrap();
    assert_eq!(proof.premises[0], derivation::RowRef::of(source.id()));
    let diagnostic = Diagnostic {
        invocation: child.id(),
        elapsed_micros: Some(1),
        iterations: None,
        examined_members: None,
        residual: None,
        converged: None,
    };
    assert_eq!(
        diagnostic.id(),
        Diagnostic {
            elapsed_micros: Some(99),
            ..diagnostic
        }
        .id()
    );
}
#[test]
fn coverage_rechecks_exact_membership_and_lower_uncertainty() {
    let inv = invocation(&definition(
        AnalysisMethod::LocalTransfers,
        Interpretation::Structural,
    ));
    let scope = CoverageScope::Input { input: inv.input };
    let normalized = NormalizationCoverage {
        computation: nominal(8),
        scope: scope.id(),
        context: inv.context,
        availability: EvidenceAvailability::Partial,
    };
    let observed = CoverageObservation::normalized(&normalized).unwrap();
    let expected = CoverageExpectation {
        invocation: inv.id(),
        capability: AnalysisCapability::Transfers,
        scope: scope.id(),
        context: inv.context,
        requested: true,
        no_scope: false,
        sources: vec![observed.source().id()],
    };
    let (coverage, members) = assess(
        &expected,
        std::slice::from_ref(&observed),
        AnalysisStatus::Completed,
        None,
        &budget(),
    )
    .unwrap();
    assert_eq!(coverage.availability, EvidenceAvailability::Partial);
    assert!(assess(&expected, &[], AnalysisStatus::Completed, None, &budget()).is_err());
    let (requirement, required) = expected.records().unwrap();
    let outcome = Outcome {
        invocation: inv.id(),
        status: AnalysisStatus::Completed,
        reason: None,
    };
    let frames = vec![
        frame(&[inv]),
        frame(&[normalized]),
        frame(&[observed.source().clone()]),
        frame(&[requirement]),
        frame(&required),
        frame(std::slice::from_ref(&coverage)),
        frame(&members),
        frame(&[outcome]),
    ];
    check::<Coverage>(frames.clone()).unwrap();
    let mut forged = coverage;
    forged.availability = EvidenceAvailability::Complete;
    forged.reason = None;
    let mut changed = frames;
    changed[5] = frame(&[forged]);
    assert!(check::<Coverage>(changed).is_err());
}
#[test]
fn native_projection_feeds_shared_join_and_stored_lineage_refuses_strengthening() {
    let def = definition(AnalysisMethod::LocalTransfers, Interpretation::Structural);
    let inv = invocation(&def);
    let scope = CoverageScope::Input { input: inv.input };
    let condition = Diagram::always();
    let q = AssertionQualification {
        assumptions: lctx_model::domain::assumptions::AssumptionSet::empty_id(),
        context: inv.context,
        scope: scope.id(),
        condition: condition.id(),
        modality: Modality::Candidate,
        approximation: Approximation::Over,
    };
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
    let evidence = EvidencePremise::native(&source, &native, &q, &condition).unwrap();
    let subject = ObligationSubject::SourceCall {
        occurrence: nominal(8),
    };
    let (derivation, proposition, members, result) = Derivation::emit(
        &inv,
        &def,
        subject.id(),
        AnalysisChannel::Catalog,
        calls::CallPhase::Call,
        QualificationOperation::Conjunction,
        &[evidence],
        &budget(),
    )
    .unwrap();
    assert_eq!(derivation.status, EvidenceStatus::Documented);
    assert!(!derivation.heuristic);
    assert_eq!(result.qualification, q);
    let (stored_condition, nodes) = condition.records();
    let frames = vec![
        frame(&[def]),
        frame(&[inv]),
        frame(&[scope]),
        frame(&[native]),
        frame(&[source]),
        frame(&[proposition]),
        frame(std::slice::from_ref(&derivation)),
        frame(&members),
        frame(&[q]),
        frame(&[stored_condition]),
        frame(&nodes),
        frame(&[assumptions::AssumptionSet::empty()]),
    ];
    check::<Derivation>(frames.clone()).unwrap();
    let mut forged = derivation;
    forged.status = EvidenceStatus::StructurallyObserved;
    let mut changed = frames;
    changed[6] = frame(&[forged]);
    assert!(check::<Derivation>(changed).is_err());
}
#[test]
fn generated_companion_refuses_predecessor_proof_as_direct_support() {
    use analysis::base_evaluation::SupportSource;
    let foreign = SupportSource::Local {
        derivation: nominal(9),
    };
    let mut index = <SupportSource as DerivedSupportSource>::index(&budget());
    index
        .visit(
            SupportSource::NAME,
            &<SupportSource as Record>::encode(std::slice::from_ref(&foreign)).unwrap(),
        )
        .unwrap();
    assert!(index.frame(foreign.id()).is_err());
}
#[test]
fn heuristic_methods_are_pinned_and_scope_cannot_strengthen_findings() {
    assert!(
        definition(
            AnalysisMethod::AnalyticEmbedding,
            Interpretation::Structural
        )
        .validate()
        .is_err()
    );
    definition(AnalysisMethod::AnalyticEmbedding, Interpretation::Heuristic)
        .validate()
        .unwrap();
    use analysis::policy::*;
    assert_eq!(
        derive_status(&[
            (SupportRole::Support, EvidenceStatus::StructurallyObserved),
            (SupportRole::Scope, EvidenceStatus::StatisticallyDerived)
        ]),
        EvidenceStatus::StatisticallyDerived
    );
    assert!(
        finding_policy(
            FindingKind::Forwarding,
            EvidenceStatus::StatisticallyDerived
        )
        .is_err()
    );
    assert!(finding_policy(FindingKind::Forwarding, EvidenceStatus::Documented).is_err());
}
#[test]
fn bdd_qualification_refuses_budget_and_keeps_shared_weakest_policy() {
    let a = Diagram::from_atom(nominal(1));
    let b = a.not().unwrap();
    let q = AssertionQualification {
        assumptions: lctx_model::domain::assumptions::AssumptionSet::empty_id(),
        context: nominal(2),
        scope: nominal(3),
        condition: a.id(),
        modality: Modality::Candidate,
        approximation: Approximation::Over,
    };
    let qb = AssertionQualification {
        condition: b.id(),
        modality: Modality::Potential,
        approximation: Approximation::Under,
        ..q.clone()
    };
    let sa = SupportSource::AnalysisDerivation {
        derivation: nominal(4),
    };
    let sb = SupportSource::AnalysisDerivation {
        derivation: nominal(5),
    };
    let premises = [
        QualifiedPremise {
            source: &sa,
            qualification: &q,
            condition: &a,
        },
        QualifiedPremise {
            source: &sb,
            qualification: &qb,
            condition: &b,
        },
    ];
    let result = qualify(QualificationOperation::Conjunction, &premises, &budget()).unwrap();
    assert!(result.condition.is_false());
    assert_eq!(result.qualification.modality, Modality::Potential);
    assert_eq!(result.qualification.approximation, Approximation::Mixed);
    let tiny = resources::ResourceBudget::fixed(1).unwrap();
    assert!(qualify(QualificationOperation::Conjunction, &premises, &tiny).is_err());
    assert_eq!(tiny.reserved(), 0);
}
#[test]
fn native_preparation_closure_excludes_later_results_and_final_coverage() {
    let model = model().unwrap();
    assert!(
        !model
            .relations()
            .iter()
            .any(|r| r.name().starts_with("dispatch_analysis_"))
    );
    let declarations_by_name = model
        .relations()
        .iter()
        .map(|r| (r.name(), r))
        .collect::<BTreeMap<_, _>>();
    let mut pending = analysis::early_relations()
        .into_iter()
        .map(|r| r.name())
        .collect::<Vec<_>>();
    let mut closure = std::collections::BTreeSet::new();
    while let Some(name) = pending.pop() {
        if !closure.insert(name) {
            continue;
        }
        let relation = declarations_by_name[name];
        for field in relation.fields() {
            if let Some((_, target)) = field.target() {
                pending.push(target);
            }
        }
        for invariant in relation.invariants() {
            pending.extend(invariant.inputs.iter().map(ValidationInput::name));
        }
        for check in relation.publication_checks() {
            pending.extend(check.inputs.iter().map(ValidationInput::name));
        }
    }
    assert!(!closure.contains(NormalizationCoverage::NAME));
    assert!(!closure.contains(transfer::local::TransferKey::NAME));
    assert!(!closure.contains(local::Invocation::NAME));
    assert!(!closure.contains(analysis::summary::Invocation::NAME));
    let row = NormalizationCoverage {
        computation: nominal(1),
        scope: nominal(2),
        context: nominal(3),
        availability: EvidenceAvailability::Complete,
    };
    assert!(local::coverage::CoverageObservation::normalized(&row).is_ok());
    let source = local::CoverageSource::Native {
        coverage: nominal(1),
    };
    assert_eq!(
        source.reference(),
        derivation::RowRef::of::<attribution::ProviderCoverage>(nominal(1))
    );
}
#[test]
fn predecessor_coverage_adapter_preserves_nominal_identity_and_partiality() {
    let row = local::Coverage {
        invocation: nominal(1),
        capability: AnalysisCapability::Transfers,
        scope: nominal(2),
        context: nominal(3),
        premises: ContentHash::of(b"actual membership"),
        availability: EvidenceAvailability::Partial,
        reason: Some(ObligationKind::IncompleteCoverage),
    };
    let source = analysis::base_evaluation::CoverageSource::Local { coverage: row.id() };
    let observed =
        analysis::base_evaluation::coverage::CoverageObservation::predecessor(&source, &row)
            .unwrap();
    let expectation = analysis::base_evaluation::coverage::CoverageExpectation {
        invocation: nominal(4),
        capability: AnalysisCapability::Execution,
        scope: row.scope,
        context: row.context,
        requested: true,
        no_scope: false,
        sources: vec![source.id()],
    };
    let (derived, _) = analysis::base_evaluation::coverage::assess(
        &expectation,
        &[observed],
        AnalysisStatus::Completed,
        None,
        &budget(),
    )
    .unwrap();
    assert_eq!(derived.availability, EvidenceAvailability::Partial);
    let forged = analysis::base_evaluation::CoverageSource::Local {
        coverage: nominal(9),
    };
    assert!(
        analysis::base_evaluation::coverage::CoverageObservation::predecessor(&forged, &row)
            .is_err()
    );
}
