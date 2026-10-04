//! Actual conditional Summary alternatives retain their premises through Catalog and wire.
#[path = "fixtures/serving_support.rs"]
mod support;
use lctx_model::domain::{assumptions::AssumptionSet, serving::*};
use support::*;

#[tokio::test]
async fn typing_conditional_and_runtime_alternatives_keep_separate_packet_bases() {
    let source = br#"from typing import Literal
__all__ = ['finite_zero', 'fail', 'normal', 'dynamic']
def finite_zero(value: Literal[0]):
    """Return the input when it equals zero."""
    if value == 0:
        return value
    return None
def fail():
    raise ValueError('invalid')
def normal():
    return None
def dynamic(error):
    raise error
"#;
    let fixture =
        ServingFixture::start_with_seeds(source, "behavioral", &["demo.finite_zero"], 16).await;
    let execution = fixture.service.execution().await.unwrap();
    let response = fixture
        .catalog
        .operation(
            &execution,
            &GetOperationRequest {
                library: Name::new("demo").unwrap(),
                operation: path("demo.finite_zero"),
                comparison: Optional::default(),
                reference_parameter: Optional::default(),
                sections: vec![OperationSection::Behavior, OperationSection::Briefs],
                page: PageRequest {
                    size: 100,
                    expanded: true,
                    ..Default::default()
                },
            },
        )
        .await;
    if response.is_err() {
        use lctx_model::domain::{analysis, assertion, conditions, execution, Record};
        let diagnostic = fixture.service.execution().await.unwrap();
        diagnostic
            .query(|lease| {
                Box::pin(async move {
                    macro_rules! inspect {
                        ($ty:ty) => {
                            let rows = lease.read::<$ty>().await?;
                            eprintln!("CONDITIONAL_DIAGNOSTIC {} {:?}", <$ty>::NAME, rows.rows());
                        };
                    }
                    inspect!(execution::summary_consequences::ClaimConclusion);
                    inspect!(execution::summary_consequences::ClaimProof);
                    inspect!(execution::summary_consequences::SummaryClaim);
                    inspect!(lctx_model::domain::transfer::summary::SummaryPremise);
                    inspect!(lctx_model::domain::transfer::summary::TransferAlternative);
                    inspect!(lctx_model::domain::assumptions::AssumptionSet);
                    inspect!(lctx_model::domain::assumptions::AssumptionSetMember);
                    inspect!(lctx_model::domain::assumptions::Assumption);
                    inspect!(analysis::summary::AnalysisInvocation);
                    inspect!(analysis::AnalysisDefinition);
                    inspect!(analysis::MethodParameters);
                    inspect!(assertion::AssertionQualification);
                    inspect!(conditions::Condition);
                    Ok(())
                })
            })
            .await
            .unwrap();
    }
    let response = response.unwrap();
    let OperationResolution::Unique { packet } = response.operation else {
        panic!("source operation missing")
    };
    let empty = AssumptionSet::empty_id();
    assert!(
        packet
            .behavior
            .items
            .iter()
            .any(|answer| answer.claim_basis.set == empty),
        "conservative runtime alternative must survive"
    );
    let conditional = packet
        .behavior
        .items
        .iter()
        .filter(|answer| answer.claim_basis.set != empty)
        .collect::<Vec<_>>();
    assert!(
        !conditional.is_empty(),
        "typed truth must reach a served Summary answer"
    );
    for answer in conditional {
        assert_eq!(answer.claim_basis.definitions.len(), 1);
        let ClaimAssumptionPacket::TypeConformance {
            observation,
            subject,
            ..
        } = &answer.claim_basis.definitions[0]
        else {
            panic!("literal typing does not justify a closed-world premise")
        };
        let (observations, sets) = execution
            .query({
                let observation = *observation;
                let set = answer.claim_basis.set;
                move |lease| {
                    Box::pin(async move {
                        Ok((
                            lease
                                .read_ids::<lctx_model::domain::types::TypeObservation>(&[
                                    observation,
                                ])
                                .await?,
                            lease.read_ids::<AssumptionSet>(&[set]).await?,
                        ))
                    })
                }
            })
            .await
            .unwrap();
        assert_eq!(observations.rows()[0].subject, *subject);
        assert_eq!(sets.rows()[0].count, 1);
        assert_eq!(sets.rows()[0].members, answer.claim_basis.members_digest);
    }
    let assertions = packet
        .briefs
        .items
        .iter()
        .flat_map(|brief| &brief.assertions)
        .collect::<Vec<_>>();
    let conditional_text = assertions
        .iter()
        .filter(|claim| {
            claim
                .text
                .as_str()
                .contains("Conditional on the stated typing")
        })
        .collect::<Vec<_>>();
    assert!(
        !conditional_text.is_empty(),
        "the mechanical brief must disclose its typing premise"
    );
    for claim in conditional_text {
        assert_ne!(claim.claim_basis.set, empty);
        assert_eq!(claim.claim_basis.definitions.len(), 1);
    }
    assert!(assertions.iter().any(|claim| {
        claim
            .text
            .as_str()
            .contains("No additional typing or closed-world assumptions.")
            && claim.claim_basis.set == empty
    }));
    let json = serde_json::to_value(&packet).unwrap();
    assert!(
        json["behavior"]["items"]
            .as_array()
            .unwrap()
            .iter()
            .all(|item| item["claim_basis"].is_object())
    );
    // Exact exception output is an independently modeled body-entry question, not typing.
    use lctx_model::domain::{execution::ExactRuntimeException, selection::*};
    for (operation, expected, exception) in [
        (
            "demo.fail",
            Outcome::Supported,
            Some(ExactRuntimeException::ValueError),
        ),
        ("demo.normal", Outcome::Contradicted, None),
        ("demo.dynamic", Outcome::Unresolved, None),
    ] {
        let comparison = fixture
            .catalog
            .compare(
                &execution,
                &CompareOperationsRequest {
                    library: Name::new("demo").unwrap(),
                    operations: vec![path(operation)],
                    selection: SelectionInput(Selection {
                        requirements: vec![Requirement {
                            predicate: Predicate::BehavioralRaises {
                                exception: ExactRuntimeException::ValueError,
                            },
                            quantifier: Quantifier::AnyApplicable,
                        }],
                        mode: Mode::Discovery,
                        joint: JointPolicy::IndependentRecords,
                    }),
                    page: PageRequest {
                        expanded: true,
                        ..Default::default()
                    },
                },
            )
            .await
            .unwrap();
        let candidate = &comparison.operations[0].candidates[0];
        assert_eq!(
            candidate.requirements[0].outcome, expected,
            "actual finite raises answer for {operation}"
        );
        let exceptions = candidate.requirements[0]
            .claims
            .iter()
            .flat_map(|claim| &claim.behavioral_exceptions)
            .collect::<Vec<_>>();
        if expected == Outcome::Unresolved {
            assert!(
                exceptions.is_empty(),
                "dynamic raise cannot manufacture normal completion"
            );
        } else {
            assert!(!exceptions.is_empty());
            for answer in exceptions {
                assert!(answer.under_body_entry);
                assert_eq!(answer.exception.0, exception);
                assert_eq!(answer.claim_basis.set, empty);
                assert!(answer.claim_basis.definitions.is_empty());
                assert!(
                    answer
                        .proof
                        .iter()
                        .any(|proof| proof.relation.as_str() == "summary_exception_outcomes")
                );
                let encoded = serde_json::to_value(answer).unwrap();
                assert_eq!(encoded["under_body_entry"], true);
                assert!(encoded["claim_basis"].is_object());
            }
        }
    }
    drop(execution);
    fixture.finish().await;
}
