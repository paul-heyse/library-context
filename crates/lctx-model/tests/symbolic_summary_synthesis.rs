//! Pure S0 consumer controls; native admission and Summary production are qualified separately.
use lctx_model::domain::{
    analysis::{self, synthesis as owner},
    assertion::{Approximation, AssertionQualification},
    attribution::{CoverageStatus, Modality},
    conditions::Diagram,
    execution::{
        summary_consequences::{ClaimConclusion, ClaimProof, SummaryClaim},
        summary_symbolic::SymbolicFieldAlternative,
    },
    flow::FlowSinkKind,
    normalized::Rows,
    obligation::{ObligationKind, Verdict},
    resources::ResourceBudget,
    source::CoverageScope,
    synthesis::{documentary, frames, observations, summary},
    transfer::TransferKind,
    *,
};

fn nominal<T>(value: u8) -> Id<T> {
    serde::Deserialize::deserialize(serde::de::value::SeqDeserializer::<
        _,
        serde::de::value::Error,
    >::new([value; 16].into_iter()))
    .unwrap()
}

struct Fixture {
    budget: ResourceBudget,
    data: summary::Data,
    docs: documentary::Data,
    observations: observations::Data,
    frames: Rows<frames::Frame>,
    invocations: Rows<owner::Invocation>,
    reader: AssertionQualification,
    constructor: AssertionQualification,
    claim: SummaryClaim,
    conclusion: ClaimConclusion,
}

fn fixture(reason: ObligationKind) -> Fixture {
    let budget = ResourceBudget::fixed(32 << 20).unwrap();
    let invocation = owner::Invocation::new(nominal(1), nominal(2), nominal(3), None, []).0;
    let reader_condition = Diagram::from_atom(nominal(4));
    let constructor = AssertionQualification {
        context: invocation.context,
        scope: CoverageScope::Artifact {
            artifact: nominal(5),
        }
        .id(),
        condition: Diagram::always().id(),
        modality: Modality::Definite,
        approximation: Approximation::Exact,
    };
    let reader = AssertionQualification {
        condition: reader_condition.id(),
        ..constructor.clone()
    };
    let alternative = SymbolicFieldAlternative {
        invocation: nominal(6),
        link: nominal(7),
        value: nominal(8),
        support: nominal(9),
        store: None,
        parameter: nominal(10),
        constructor: nominal(11),
        reader: nominal(12),
        constructor_qualification: constructor.id(),
        reader_qualification: reader.id(),
        sink: nominal(13),
        kind: FlowSinkKind::Return,
        transfer: TransferKind::Identity,
        through_call: false,
        depth: 2,
        reason,
    };
    let claim = SummaryClaim::SymbolicFieldAssociation {
        alternative: alternative.id(),
        qualification: reader.id(),
    };
    let subject = analysis::summary::ObligationSubject::SummaryClaim {
        transfer: claim.id(),
    };
    let conclusion = ClaimConclusion {
        invocation: alternative.invocation,
        subject: subject.id(),
        qualification: Some(reader.id()),
        proof: None,
        coverage: CoverageStatus::Partial,
        verdict: Verdict::Unknown,
        reason: Some(reason),
    };
    let mut data = summary::Data::new(&budget);
    data.symbolic_alternatives.insert(alternative).unwrap();
    data.claims.insert(claim.clone()).unwrap();
    data.subjects.insert(subject).unwrap();
    data.conclusions.insert(conclusion.clone()).unwrap();
    let mut observations = observations::Data::new(&budget);
    for q in [&reader, &constructor] {
        observations.qualifications.insert(q.clone()).unwrap();
    }
    for diagram in [reader_condition, Diagram::always()] {
        let (condition, nodes) = diagram.records();
        observations.conditions.insert(condition).unwrap();
        for node in nodes {
            observations.nodes.insert(node).unwrap();
        }
    }
    let mut frames = Rows::new(&budget);
    frames
        .insert(frames::Frame {
            invocation: invocation.id(),
            configuration: nominal(14),
            core: nominal(15),
            evidence: nominal(16),
            selection: nominal(17),
            structural: nominal(18),
            analytic: nominal(19),
            summary: conclusion.invocation,
        })
        .unwrap();
    let mut invocations = Rows::new(&budget);
    invocations.insert(invocation).unwrap();
    Fixture {
        docs: documentary::Data::new(&budget),
        budget,
        data,
        observations,
        frames,
        invocations,
        reader,
        constructor,
        claim,
        conclusion,
    }
}

#[test]
fn qualified_symbolic_unknown_retains_reader_scope_and_has_no_finding_authority() {
    for reason in [
        ObligationKind::ScopeBoundary,
        ObligationKind::MissingEvidence,
    ] {
        let f = fixture(reason);
        assert_ne!(f.reader.condition, f.constructor.condition);
        assert_eq!(f.reader.scope, f.constructor.scope);
        let alternative = f.data.symbolic_alternatives.iter().next().unwrap();
        assert_ne!(alternative.constructor, alternative.reader);
        assert_eq!(f.conclusion.proof, None);
        assert_eq!(summary::evidence(&f.data, &f.conclusion).unwrap(), None);
        let (facets, sources) =
            summary::build(&f.data, &f.docs, &f.frames, &f.invocations, &f.budget).unwrap();
        assert_eq!(facets.len(), 1);
        assert!(sources.is_empty());
        let facet = facets.iter().next().unwrap();
        assert_eq!(facet.conclusion, f.conclusion.id());
        assert_eq!(facet.claim, Some(f.claim.id()));
        assert_eq!(facet.qualification, Some(f.reader.id()));
        assert_eq!(facet.coverage, CoverageStatus::Partial);
        assert_eq!(facet.verdict, Verdict::Unknown);
        assert_eq!(facet.reason, Some(reason));
        assert_eq!(facet.source, None);
        assert!(
            summary::text(&f.data, facet, &f.budget)
                .unwrap()
                .contains("whether a later read returns the stored value remains unresolved")
        );
        let mut output = observations::Output::new(&f.budget);
        summary::extend_observations(
            &f.data,
            &f.observations,
            &facets,
            &f.frames,
            &f.invocations,
            &Rows::new(&f.budget),
            &mut output,
            &f.budget,
        )
        .unwrap();
        assert!(output.findings.is_empty());
        assert!(output.supports.is_empty());
        assert!(output.sources.is_empty());
        assert!(output.derivations.is_empty());
        assert!(output.propositions.is_empty());
        assert!(output.premises.is_empty());
    }
}

#[test]
fn symbolic_authority_and_constructor_qualification_forgery_refuse() {
    for case in 0..6 {
        let mut f = fixture(ObligationKind::ScopeBoundary);
        match case {
            0 => f.conclusion.verdict = Verdict::Established,
            1 => {
                let proof = ClaimProof::Closure {
                    claim: f.claim.id(),
                    qualification: f.reader.id(),
                    status: analysis::policy::EvidenceStatus::StructurallyObserved,
                    members: ContentHash::of(b"forged-symbolic-proof"),
                };
                f.conclusion.proof = Some(f.data.proofs.insert(proof).unwrap());
            }
            2 => f.conclusion.qualification = Some(f.constructor.id()),
            3 => {
                // Laundering both the claim and consequence still changes the original reader.
                f.claim = SummaryClaim::SymbolicFieldAssociation {
                    alternative: f.data.symbolic_alternatives.iter().next().unwrap().id(),
                    qualification: f.constructor.id(),
                };
                f.data.claims = Rows::new(&f.budget);
                f.data.claims.insert(f.claim.clone()).unwrap();
                let subject = analysis::summary::ObligationSubject::SummaryClaim {
                    transfer: f.claim.id(),
                };
                f.data.subjects = Rows::new(&f.budget);
                f.conclusion.subject = f.data.subjects.insert(subject).unwrap();
                f.conclusion.qualification = Some(f.constructor.id());
            }
            4 => f.conclusion.reason = Some(ObligationKind::MissingEvidence),
            _ => f.conclusion.coverage = CoverageStatus::CompleteUnderStatedModel,
        }
        f.data.conclusions = Rows::new(&f.budget);
        f.data.conclusions.insert(f.conclusion.clone()).unwrap();
        assert!(
            summary::evidence(&f.data, &f.conclusion).is_err(),
            "forgery {case}"
        );
        assert!(
            summary::build(&f.data, &f.docs, &f.frames, &f.invocations, &f.budget).is_err(),
            "forgery {case}"
        );
    }
}
