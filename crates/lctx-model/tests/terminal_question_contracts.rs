//! Independent scoped-question oracles; completion proof APIs are deliberately absent.
use lctx_model::domain::{
    analysis::{
        self, policy::EvidenceStatus, summary::support::EvidencePremise,
        support::QualificationOperation,
    },
    assertion::*,
    assumptions::*,
    attribution::*,
    conditions::Diagram,
    execution::{
        closed_targets::*, protocol_interpretation::*, summary_consequences::SummaryClaim,
        summary_terminal::SummaryTerminalWitness,
    },
    resources::ResourceBudget,
    serving::*,
    synthesis::{summary, terminal},
    *,
};
fn id<R>(n: u8) -> Id<R> {
    serde_json::from_value(serde_json::json!(vec![n; 16])).unwrap()
}
fn fixture() -> (
    terminal::Data,
    summary::Data,
    SummaryTerminalWitness,
    AssertionQualification,
) {
    fixture_scope(false, false)
}
fn fixture_scope(
    input_scope: bool,
    foreign: bool,
) -> (
    terminal::Data,
    summary::Data,
    SummaryTerminalWitness,
    AssertionQualification,
) {
    let budget = ResourceBudget::fixed(8 << 20).unwrap();
    let mut d = terminal::Data::new(&budget);
    let mut s = summary::Data::new(&budget);
    let input = id(1);
    let context = id(2);
    let conformance = Assumption::TypeConformance {
        observation: id(3),
        support: id(4),
    };
    let basis = AssumptionSet::new([conformance.id()]).unwrap();
    d.sets.insert(basis.set.clone()).unwrap();
    for member in basis.members {
        d.members.insert(member).unwrap();
    }
    d.assumptions.insert(conformance.clone()).unwrap();
    let artifact = source::SourceArtifact::from_bytes(
        if foreign { id(20) } else { input },
        "demo.py".into(),
        b"stop()\nprint('following')\n",
    )
    .unwrap();
    let scope = if input_scope {
        source::CoverageScope::Input {
            input: if foreign { id(20) } else { input },
        }
    } else {
        source::CoverageScope::Artifact {
            artifact: artifact.id(),
        }
    };
    let model = lctx_model::domain::model().unwrap();
    d.visit(
        source::SourceArtifact::NAME,
        Batch::new(&model, vec![artifact], &budget).unwrap().arrow(),
    )
    .unwrap();
    d.visit(
        source::CoverageScope::NAME,
        Batch::new(&model, vec![scope.clone()], &budget)
            .unwrap()
            .arrow(),
    )
    .unwrap();
    let q = AssertionQualification {
        context,
        scope: scope.id(),
        condition: Diagram::always().id(),
        modality: Modality::Definite,
        approximation: Approximation::Exact,
        assumptions: basis.set.id(),
    };
    let definition = analysis::AnalysisDefinition {
        method: analysis::AnalysisMethod::Summaries,
        parameters: id(5),
        semantic_version: ContentHash::of(b"scoped question test"),
        interpretation: analysis::Interpretation::Structural,
    };
    let model = analysis::model::AnalysisInvocation {
        input,
        context,
        definition: id(6),
        subject: None,
        inputs: ContentHash::of(b"model input"),
        sources: ContentHash::of(b"model source"),
        projections: ContentHash::of(b"model projection"),
    };
    let invocation = analysis::summary::AnalysisInvocation {
        input,
        context,
        definition: definition.id(),
        subject: None,
        inputs: ContentHash::of(b"summary input"),
        sources: ContentHash::of(b"summary source"),
        projections: ContentHash::of(b"summary projection"),
    };
    let native = analysis::native::NativeAssertionPremise::NativeTerminal {
        assertion: id(7),
        support: id(8),
    };
    d.native.insert(native.clone()).unwrap();
    let placement = analysis::native::NativeAssertionPremise::SyntaxPlacement {
        assertion: id(9),
        support: id(10),
    };
    d.native.insert(placement.clone()).unwrap();
    let following = analysis::native::NativeAssertionPremise::SyntaxPlacement {
        assertion: id(18),
        support: id(21),
    };
    d.native.insert(following.clone()).unwrap();
    let target = ClosedTargetAssessment {
        invocation: model.id(),
        attempt: id(11),
        event: id(12),
        target: Some(id(13)),
        original_qualification: Some(id(14)),
        target_native: None,
        receiver_observation: None,
        receiver_qualification: None,
        basis: TargetBasis::ExactRuntime,
        qualification: Some(q.id()),
        reason: None,
        runtime_receiver: None,
        ancestry: None,
        receiver_conformance: None,
        no_extra_overrides: None,
        universe_support: None,
        final_metadata: None,
        final_member: None,
        final_native: None,
        member_native: None,
    };
    let f = ConditionalTerminalFrontier {
        invocation: model.id(),
        observation: id(7),
        native: native.id(),
        target: target.id(),
        declared_return: id(3),
        conformance: conformance.id(),
        qualification: q.id(),
        owner: id(15),
        call: id(16),
        statement: id(17),
        question: InvocationQuestion::GivenInvocationEntered,
        effects_unknown: true,
        exceptions_unknown: true,
    };
    let edge = NormalContinuationRestriction {
        frontier: f.id(),
        statement: id(9),
        following: id(18),
        statement_native: placement.id(),
        following_native: following.id(),
        owner: f.owner,
        from: f.statement,
        to: id(19),
        qualification: q.id(),
    };
    let claim = SummaryClaim::NoNormalContinuation {
        owner: f.owner,
        frontier: f.id(),
        restriction: edge.id(),
        qualification: q.id(),
        question: f.question,
    };
    let w = SummaryTerminalWitness {
        invocation: invocation.id(),
        frontier: f.id(),
        restriction: edge.id(),
        qualification: q.id(),
        claim: claim.id(),
        question: f.question,
        status: EvidenceStatus::StructurallyObserved,
    };
    let source = analysis::summary::SupportSource::TerminalFrontier { witness: w.id() };
    let subject = analysis::summary::ObligationSubject::SummaryClaim {
        transfer: claim.id(),
    };
    let diagram = Diagram::always();
    let evidence = EvidencePremise::derived(&source, &w, &q, &diagram).unwrap();
    let (derive, prop, premises, _) = analysis::summary::AnalysisDerivation::emit(
        &invocation,
        &definition,
        subject.id(),
        analysis::AnalysisChannel::Role,
        calls::CallPhase::Call,
        QualificationOperation::Conjunction,
        &[evidence],
        &budget,
    )
    .unwrap();
    s.claims.insert(claim).unwrap();
    s.sources.insert(source).unwrap();
    s.subjects.insert(subject).unwrap();
    s.derivations.insert(derive).unwrap();
    s.propositions.insert(prop).unwrap();
    for premise in premises {
        s.premises.insert(premise).unwrap();
    }
    d.model_invocations.insert(model).unwrap();
    d.summary_invocations.insert(invocation).unwrap();
    d.targets.insert(target).unwrap();
    d.frontiers.insert(f).unwrap();
    d.restrictions.insert(edge).unwrap();
    d.witnesses.insert(w.clone()).unwrap();
    (d, s, w, q)
}
#[test]
fn checked_terminal_packet_preserves_given_entry_and_refuses_missing_or_changed_basis() {
    let (d, s, w, q) = fixture();
    let checked = terminal::checked(&d, &s, w.id(), &q, id(1), id(2)).unwrap();
    let packet = TerminalQuestionPacket::from_canonical(&checked).unwrap();
    assert_eq!(packet.scope, q.scope);
    assert_eq!(
        (
            packet.question,
            packet.effects_unknown,
            packet.exceptions_unknown,
            packet.cleanup_unknown
        ),
        (InvocationQuestion::GivenInvocationEntered, true, true, true)
    );
    assert_eq!(
        (
            packet.witness,
            packet.claim,
            packet.owner,
            packet.call,
            packet.following
        ),
        (w.id(), w.claim, id(15), id(16), id(19))
    );
    for relation in [
        "summary_terminal_witnesses",
        "conditional_terminal_frontiers",
        "normal_continuation_restrictions",
        "summary_analysis_derivations",
        "summary_analysis_derivation_premises",
        "summary_support_sources",
    ] {
        assert!(
            checked.proof().iter().any(|p| p.relation() == relation),
            "actual scoped source: {relation}"
        );
    }
    assert!(!checked.proof().iter().any(|p| {
        ["claim_proofs", "claim_conclusions", "body_executions"].contains(&p.relation())
    }));
    assert!(terminal::checked(&d, &s, w.id(), &q, id(20), id(2)).is_err());
    assert!(terminal::checked(&d, &s, w.id(), &q, id(1), id(20)).is_err());
    assert!(
        terminal::checked(
            &d,
            &s,
            w.id(),
            &AssertionQualification {
                assumptions: AssumptionSet::empty_id(),
                ..q
            },
            id(1),
            id(2)
        )
        .is_err()
    );
}
#[test]
fn terminal_lowering_requires_source13_role_call_derivation_and_unknown_effects() {
    let (mut d, s, w, q) = fixture();
    let mut f = d.frontiers.iter().next().unwrap().clone();
    f.effects_unknown = false;
    d.frontiers = normalized::Rows::new(&ResourceBudget::fixed(8 << 20).unwrap());
    d.frontiers.insert(f).unwrap();
    assert!(terminal::checked(&d, &s, w.id(), &q, id(1), id(2)).is_err());
    let (d, mut s, w, q) = fixture();
    let budget = ResourceBudget::fixed(8 << 20).unwrap();
    s.premises = normalized::Rows::new(&budget);
    assert!(terminal::checked(&d, &s, w.id(), &q, id(1), id(2)).is_err());
    let (d, mut s, w, q) = fixture();
    let mut derivation = s.derivations.iter().next().unwrap().clone();
    derivation.status = EvidenceStatus::Documented;
    s.derivations = normalized::Rows::new(&ResourceBudget::fixed(8 << 20).unwrap());
    s.derivations.insert(derivation).unwrap();
    assert!(terminal::checked(&d, &s, w.id(), &q, id(1), id(2)).is_err());
}
#[test]
fn terminal_schema_has_required_question_finite_code_and_nullable_assertion_slot() {
    let (d, s, w, q) = fixture();
    let packet = TerminalQuestionPacket::from_canonical(
        &terminal::checked(&d, &s, w.id(), &q, id(1), id(2)).unwrap(),
    ).unwrap();
    let schema = schema_for::<TerminalQuestionPacket>(true);
    let validator = jsonschema::validator_for(&schema).unwrap();
    let question = serde_json::to_value(packet).unwrap();
    let mut value = question.clone();
    assert!(validator.is_valid(&value));
    value["question"] = serde_json::json!(1);
    assert!(!validator.is_valid(&value));
    assert!(serde_json::from_value::<TerminalQuestionPacket>(value).is_err());
    let assertion = schema_for::<AssertionPacket>(true);
    // Validate the complete root: nullable fields can be represented by local references.
    let validator = jsonschema::validator_for(&assertion).unwrap();
    let empty = AssumptionSet::empty();
    let mut claim = serde_json::json!({"assertion":vec![1u8;16],"kind":0,"section":0,"status":1,"qualification":vec![2u8;16],"claim_basis":{"set":empty.id(),"members_digest":empty.members,"definitions":[]},"terminal_question":null,"text":"Given-entry typing question.","supports":[]});
    assert!(validator.is_valid(&claim), "explicit null is admitted");
    assert!(serde_json::from_value::<AssertionPacket>(claim.clone()).is_ok());
    claim["terminal_question"] = question;
    assert!(
        validator.is_valid(&claim),
        "a complete scoped question is admitted"
    );
    assert!(serde_json::from_value::<AssertionPacket>(claim.clone()).is_ok());
    let mut absent = claim.clone();
    absent.as_object_mut().unwrap().remove("terminal_question");
    assert!(
        !validator.is_valid(&absent),
        "the nullable field is still required"
    );
    assert!(serde_json::from_value::<AssertionPacket>(absent).is_err());
    let mut primitive = claim.clone();
    primitive["terminal_question"] = serde_json::json!(false);
    assert!(!validator.is_valid(&primitive));
    assert!(serde_json::from_value::<AssertionPacket>(primitive).is_err());
    let mut incomplete = claim.clone();
    incomplete["terminal_question"]
        .as_object_mut()
        .unwrap()
        .remove("qualification");
    assert!(!validator.is_valid(&incomplete));
    assert!(serde_json::from_value::<AssertionPacket>(incomplete).is_err());
    let mut invalid = claim;
    invalid["terminal_question"]["question"] = serde_json::json!(1);
    assert!(!validator.is_valid(&invalid));
    assert!(serde_json::from_value::<AssertionPacket>(invalid).is_err());
    let binding = <CapabilityPacket as mappings::PacketOutput>::binding();
    for relation in [
        execution::summary_terminal::SummaryTerminalWitness::NAME,
        execution::protocol_interpretation::ConditionalTerminalFrontier::NAME,
        execution::protocol_interpretation::NormalContinuationRestriction::NAME,
    ] {
        assert!(binding.permits_relation(relation));
    }
}

#[test]
fn terminal_scope_is_owned_artifact_or_input_and_never_rewritten() {
    for input_scope in [false, true] {
        let (d, s, w, q) = fixture_scope(input_scope, false);
        let packet = TerminalQuestionPacket::from_canonical(
            &terminal::checked(&d, &s, w.id(), &q, id(1), id(2)).unwrap(),
        ).unwrap();
        assert_eq!(packet.scope, q.scope);
        if !input_scope {
            assert_ne!(
                packet.scope,
                source::CoverageScope::Input { input: id(1) }.id(),
                "native Artifact scope must survive"
            );
        }
        let (d, s, w, q) = fixture_scope(input_scope, true);
        assert!(
            terminal::checked(&d, &s, w.id(), &q, id(1), id(2)).is_err(),
            "same well-formed lower chain cannot cross foreign scope ownership"
        );
    }
}
