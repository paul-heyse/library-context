//! Real Catalog compilation, S0 replay and packet hydration of the given-entry question.
#[path = "fixtures/serving_support.rs"]
mod support;
use lctx_model::domain::{
    analysis::policy::{AssertionKind, EvidenceStatus},
    assertion::Approximation,
    assumptions::AssumptionSet,
    attribution::Modality,
    execution::{
        closed_targets::*, protocol_interpretation::*, summary_terminal::SummaryTerminalWitness,
    },
    serving::*,
    synthesis::assertions::AssertionSource,
    *,
};
use std::collections::BTreeSet;
use support::*;

#[tokio::test]
async fn terminal_questions_reach_actual_catalog_briefs_without_completion_authority() {
    let test_started = std::time::Instant::now();
    eprintln!("terminal_question_packets BEGIN fixture_compile");
    let positives = [
        ("demo.declared_use", 1, TargetBasis::ExactRuntime),
        ("demo.final_use", 3, TargetBasis::TypingConditional),
        ("demo.final_method_use", 3, TargetBasis::TypingConditional),
    ];
    let negatives = [
        "demo.open_use",
        "demo.protocol_use",
        "demo.higher_order_use",
        "demo.never_use",
        "demo.property_use",
        "demo.placeholder_use",
        "demo.inferred_use",
        "demo.cleanup_use",
        "demo.async_creation_use",
        "demo.mro_gap_use",
    ];
    let seeds = positives
        .iter()
        .map(|(name, _, _)| *name)
        .chain(negatives.iter().copied())
        .collect::<Vec<_>>();
    let fixture = ServingFixture::start_with_seeds(
        include_bytes!("../../../fixtures/python/terminal_question/cases.py"),
        "behavioral",
        &seeds,
        16,
    )
    .await;
    eprintln!(
        "terminal_question_packets END fixture_compile elapsed_s={:.3}",
        test_started.elapsed().as_secs_f64()
    );
    let execution = fixture.service.execution().await.unwrap();
    let expected = BTreeSet::from([
        "demo.declared_use".to_owned(),
        "demo.final_use".to_owned(),
        "demo.final_method_use".to_owned(),
    ]);
    let mut served = BTreeSet::new();
    let mut witness_ids = BTreeSet::new();
    for (operation, basis_count, target_basis) in positives {
        let operation_started = std::time::Instant::now();
        eprintln!("terminal_question_packets BEGIN positive operation={operation}");
        let response = fixture
            .catalog
            .operation(
                &execution,
                &GetOperationRequest {
                    library: Name::new("demo").unwrap(),
                    operation: path(operation),
                    comparison: Optional::default(),
                    reference_parameter: Optional::default(),
                    sections: vec![OperationSection::Briefs],
                    page: PageRequest {
                        size: 100,
                        expanded: true,
                        ..Default::default()
                    },
                },
            )
            .await
            .unwrap();
        let OperationResolution::Unique { packet } = response.operation else {
            panic!("public terminal operation missing: {operation}")
        };
        assert!(
            !packet.briefs.items.is_empty(),
            "documented public member must have its real S0 brief: {operation}"
        );
        let claims = packet
            .briefs
            .items
            .iter()
            .flat_map(|brief| &brief.assertions)
            .filter(|claim| claim.terminal_question.0.is_some())
            .collect::<Vec<_>>();
        assert_eq!(
            claims.len(),
            1,
            "exact scoped terminal question for {operation}"
        );
        for claim in claims {
            let question = claim.terminal_question.0.as_ref().unwrap();
            served.insert(operation.to_owned());
            witness_ids.insert(question.witness);
            assert_eq!(
                (claim.kind, claim.status),
                (
                    AssertionKind::ApplicableCase,
                    EvidenceStatus::StructurallyObserved
                )
            );
            assert_eq!(question.qualification, claim.qualification);
            assert_eq!(
                (
                    question.question,
                    question.effects_unknown,
                    question.exceptions_unknown,
                    question.cleanup_unknown
                ),
                (InvocationQuestion::GivenInvocationEntered, true, true, true)
            );
            assert_eq!(question.target_basis, target_basis);
            assert_ne!(claim.claim_basis.set, AssumptionSet::empty_id());
            assert_eq!(claim.claim_basis.definitions.len(), basis_count);
            assert!(claim.text.as_str().contains("Given entry"));
            assert!(
                claim
                    .text
                    .as_str()
                    .contains("Invocation entry was not established")
            );
            assert_eq!(
                claim
                    .claim_basis
                    .definitions
                    .iter()
                    .filter(|d| matches!(d, ClaimAssumptionPacket::NoExtraOverrides { .. }))
                    .count(),
                usize::from(target_basis == TargetBasis::TypingConditional)
            );
            for definition in &claim.claim_basis.definitions {
                match definition {
                    ClaimAssumptionPacket::TypeConformance { support, .. } => {
                        assert_eq!(support.fidelity, attribution::Fidelity::NativeStructural)
                    }
                    ClaimAssumptionPacket::NoExtraOverrides {
                        support, universe, ..
                    } => {
                        assert_eq!(support.fidelity, attribution::Fidelity::NativeStructural);
                        assert_eq!(
                            (universe.input, universe.context),
                            (question.input, question.context)
                        );
                        assert!(!universe.source.as_str().is_empty());
                    }
                }
            }
            for relation in [
                "summary_terminal_witnesses",
                "conditional_terminal_frontiers",
                "normal_continuation_restrictions",
                "summary_analysis_derivations",
                "summary_analysis_derivation_premises",
                "summary_support_sources",
                "native_terminal_supports",
                "syntax_placement_supports",
            ] {
                assert!(
                    question
                        .proof
                        .iter()
                        .any(|p| p.relation.as_str() == relation),
                    "hydrated source {relation}"
                );
            }
            assert!(!question.proof.iter().any(|p| {
                ["claim_proofs", "claim_conclusions", "body_executions"]
                    .contains(&p.relation.as_str())
            }));
            let (witnesses, frontiers, restrictions, targets, qualifications, sources) = execution
                .query({
                    let question = question.clone();
                    let sources = claim.supports.iter().map(|s| s.source).collect::<Vec<_>>();
                    move |lease| {
                        Box::pin(async move {
                            Ok((
                                lease
                                    .read_ids::<SummaryTerminalWitness>(&[question.witness])
                                    .await?,
                                lease
                                    .read_ids::<ConditionalTerminalFrontier>(&[question.frontier])
                                    .await?,
                                lease
                                    .read_ids::<NormalContinuationRestriction>(&[
                                        question.restriction
                                    ])
                                    .await?,
                                lease
                                    .read_ids::<ClosedTargetAssessment>(&[question.target])
                                    .await?,
                                lease
                                    .read_ids::<assertion::AssertionQualification>(&[
                                        question.qualification
                                    ])
                                    .await?,
                                lease.read_ids::<AssertionSource>(&sources).await?,
                            ))
                        })
                    }
                })
                .await
                .unwrap();
            let qscope = qualifications.rows()[0].scope;
            assert_eq!(question.scope, qscope);
            let scopes = execution
                .query(move |lease| {
                    Box::pin(
                        async move { lease.read_ids::<source::CoverageScope>(&[qscope]).await },
                    )
                })
                .await
                .unwrap();
            assert!(
                matches!(scopes.rows()[0], source::CoverageScope::Artifact { .. }),
                "actual native Artifact qualification must survive into the terminal packet"
            );
            assert_eq!(
                (witnesses.rows()[0].claim, witnesses.rows()[0].qualification),
                (question.claim, question.qualification)
            );
            assert_eq!(
                (
                    frontiers.rows()[0].owner,
                    frontiers.rows()[0].call,
                    restrictions.rows()[0].to
                ),
                (question.owner, question.call, question.following)
            );
            assert_eq!(targets.rows()[0].basis, target_basis);
            assert!(sources.rows().contains(&AssertionSource::TerminalSummary {
                witness: question.witness
            }));
            let q = &qualifications.rows()[0];
            assert_eq!(
                (q.modality, q.approximation, q.assumptions),
                (
                    Modality::Definite,
                    Approximation::Exact,
                    claim.claim_basis.set
                )
            );
            if target_basis == TargetBasis::TypingConditional {
                let original = question.original_target_qualification.0.unwrap();
                let receiver = question.receiver_qualification.0.unwrap();
                let rows = execution
                    .query(move |lease| {
                        Box::pin(async move {
                            lease
                                .read_ids::<assertion::AssertionQualification>(&[
                                    original, receiver,
                                ])
                                .await
                        })
                    })
                    .await
                    .unwrap();
                assert_eq!(
                    rows.rows()
                        .iter()
                        .find(|q| q.id() == original)
                        .unwrap()
                        .modality,
                    Modality::Candidate
                );
                assert_eq!(
                    rows.rows()
                        .iter()
                        .find(|q| q.id() == receiver)
                        .unwrap()
                        .modality,
                    Modality::Definite
                );
            }
        }
        for brief in &packet.briefs.items {
            let standalone = fixture
                .service
                .capability(
                    &execution,
                    &GetCapabilityRequest {
                        capability: brief.capability,
                        page: PageRequest {
                            expanded: true,
                            ..Default::default()
                        },
                    },
                )
                .await
                .unwrap();
            assert_eq!(standalone.capability, *brief);
            let resource = standalone.resource_text().unwrap();
            assert!(resource.contains("\"terminal_question\""));
            assert!(resource.contains("\"cleanup_unknown\":true"));
        }
        eprintln!(
            "terminal_question_packets END positive operation={operation} elapsed_s={:.3}",
            operation_started.elapsed().as_secs_f64()
        );
    }
    assert_eq!(served, expected);
    assert_eq!(witness_ids.len(), 3);
    for operation in negatives {
        let operation_started = std::time::Instant::now();
        eprintln!("terminal_question_packets BEGIN negative operation={operation}");
        let response = fixture
            .catalog
            .operation(
                &execution,
                &GetOperationRequest {
                    library: Name::new("demo").unwrap(),
                    operation: path(operation),
                    comparison: Optional::default(),
                    reference_parameter: Optional::default(),
                    sections: vec![OperationSection::Briefs],
                    page: PageRequest {
                        size: 100,
                        expanded: true,
                        ..Default::default()
                    },
                },
            )
            .await
            .unwrap();
        let OperationResolution::Unique { packet } = response.operation else {
            panic!("public negative missing: {operation}")
        };
        assert!(
            !packet.briefs.items.is_empty(),
            "negative control must actually reach S0: {operation}"
        );
        assert!(
            packet
                .briefs
                .items
                .iter()
                .flat_map(|brief| &brief.assertions)
                .all(|claim| claim.terminal_question.0.is_none()),
            "negative must not gain terminal closure: {operation}"
        );
        eprintln!(
            "terminal_question_packets END negative operation={operation} elapsed_s={:.3}",
            operation_started.elapsed().as_secs_f64()
        );
    }
    let actual = execution
        .query(|lease| Box::pin(async move { lease.read::<SummaryTerminalWitness>().await }))
        .await
        .unwrap();
    assert_eq!(
        actual
            .rows()
            .iter()
            .map(Record::id)
            .collect::<BTreeSet<_>>(),
        witness_ids,
        "served set equals the independently expected actual scoped questions"
    );
    drop(execution);
    fixture.finish().await;
    eprintln!(
        "terminal_question_packets END total_s={:.3}",
        test_started.elapsed().as_secs_f64()
    );
}
