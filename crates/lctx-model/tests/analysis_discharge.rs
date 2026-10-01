//! A later owner resolves an exact earlier question without rewriting its proof graph.
use lctx_model::domain::{
    analysis::{local, policy::EvidenceStatus, summary, support::QualificationOperation, *},
    assertion::{Approximation, AssertionQualification},
    attribution::Modality,
    calls::CallPhase,
    conditions::Diagram,
    normalized::coverage::EvidenceAvailability,
    obligation::ObligationKind,
    source::CoverageScope,
    *,
};

fn nominal<T>(value: u8) -> Id<T> {
    serde::Deserialize::deserialize(serde::de::value::SeqDeserializer::<
        _,
        serde::de::value::Error,
    >::new([value; 16].into_iter()))
    .unwrap()
}
fn budget() -> resources::ResourceBudget {
    resources::ResourceBudget::fixed(32 << 20).unwrap()
}
fn frame<R: Record>(rows: &[R]) -> (&'static str, arrow_array::RecordBatch) {
    (R::NAME, R::encode(rows).unwrap())
}
struct Fixture {
    local: local::Invocation,
    summary: summary::Invocation,
    local_subject: local::ObligationSubject,
    summary_subject: summary::ObligationSubject,
    obligation: local::Obligation,
    source: summary::ObligationSource,
    proposition: summary::Proposition,
    derivation: summary::Derivation,
    coverage: summary::Coverage,
    evidence: summary::DischargeEvidence,
    scope: CoverageScope,
    qualification: AssertionQualification,
    condition: Diagram,
}
impl Fixture {
    fn new(computation: bool) -> Self {
        let local = local::Invocation::new(nominal(1), nominal(2), nominal(3), None, []).0;
        let summary = summary::Invocation::new(local.input, local.context, nominal(4), None, []).0;
        let scope = CoverageScope::Input { input: local.input };
        let condition = Diagram::always();
        let qualification = AssertionQualification {
            scope: scope.id(),
            context: local.context,
            condition: condition.id(),
            modality: Modality::Definite,
            approximation: Approximation::Exact,
        };
        let local_subject = if computation {
            local::ObligationSubject::Computation {
                invocation: local.id(),
            }
        } else {
            local::ObligationSubject::SourceCall {
                occurrence: nominal(5),
            }
        };
        let summary_subject = if computation {
            summary::ObligationSubject::Local {
                invocation: local.id(),
            }
        } else {
            summary::ObligationSubject::SourceCall {
                occurrence: nominal(5),
            }
        };
        let obligation = local::Obligation {
            invocation: local.id(),
            subject: local_subject.id(),
            channel: AnalysisChannel::Value,
            phase: CallPhase::Call,
            qualification: qualification.id(),
            reason: ObligationKind::MissingEvidence,
            responsible: AnalysisMethod::Summaries,
        };
        let source = summary::ObligationSource::Local {
            obligation: obligation.id(),
        };
        let proposition = summary::Proposition {
            subject: summary_subject.id(),
            channel: AnalysisChannel::Value,
            phase: CallPhase::Call,
            qualification: qualification.id(),
        };
        let derivation = summary::Derivation {
            invocation: summary.id(),
            proposition: proposition.id(),
            qualification: qualification.id(),
            operation: QualificationOperation::Conjunction,
            inputs: ContentHash::of(b"independent discharge control"),
            status: EvidenceStatus::StructurallyObserved,
            heuristic: false,
        };
        let coverage = summary::Coverage {
            invocation: summary.id(),
            capability: AnalysisCapability::Summaries,
            scope: scope.id(),
            context: local.context,
            premises: ContentHash::of(b"membership"),
            availability: EvidenceAvailability::Partial,
            reason: Some(ObligationKind::IncompleteCoverage),
        };
        let evidence = summary::DischargeEvidence {
            obligation: source.id(),
            derivation: derivation.id(),
            coverage: coverage.id(),
        };
        Self {
            local,
            summary,
            local_subject,
            summary_subject,
            obligation,
            source,
            proposition,
            derivation,
            coverage,
            evidence,
            scope,
            qualification,
            condition,
        }
    }
    fn refresh(&mut self) {
        self.obligation.invocation = self.local.id();
        self.obligation.subject = self.local_subject.id();
        self.obligation.qualification = self.qualification.id();
        self.source = summary::ObligationSource::Local {
            obligation: self.obligation.id(),
        };
        self.proposition.subject = self.summary_subject.id();
        self.proposition.qualification = self.qualification.id();
        self.derivation.invocation = self.summary.id();
        self.derivation.proposition = self.proposition.id();
        self.derivation.qualification = self.qualification.id();
        self.coverage.invocation = self.summary.id();
        self.evidence = summary::DischargeEvidence {
            obligation: self.source.id(),
            derivation: self.derivation.id(),
            coverage: self.coverage.id(),
        };
    }
    fn check(&self, omit_subject: bool) -> Result<(), ModelError> {
        let (condition, nodes) = self.condition.records();
        let frames = vec![
            frame(std::slice::from_ref(&self.local)),
            frame(std::slice::from_ref(&self.summary)),
            frame(std::slice::from_ref(&self.local_subject)),
            frame(std::slice::from_ref(&self.summary_subject)),
            frame(std::slice::from_ref(&self.obligation)),
            frame(std::slice::from_ref(&self.source)),
            frame(std::slice::from_ref(&self.proposition)),
            frame(std::slice::from_ref(&self.derivation)),
            frame(std::slice::from_ref(&self.coverage)),
            frame(std::slice::from_ref(&self.evidence)),
            frame(std::slice::from_ref(&self.scope)),
            frame(std::slice::from_ref(&self.qualification)),
            frame(&[condition]),
            frame(&nodes),
        ];
        let invariant = summary::DischargeEvidence::invariants().remove(0);
        let mut check = (invariant.create)(&budget());
        for (name, batch) in frames {
            if !(omit_subject && name == local::ObligationSubject::NAME) {
                check.visit(name, &batch)?;
            }
        }
        check.finish()
    }
}

#[test]
fn predecessor_questions_keep_nominal_owner_and_discharge_is_its_own_conclusion() {
    for computation in [false, true] {
        let fixture = Fixture::new(computation);
        fixture.check(false).unwrap();
        let proof = fixture.evidence.proof().unwrap();
        assert_eq!(
            proof.conclusion,
            derivation::RowRef::of(fixture.evidence.id())
        );
        assert!(
            proof
                .premises
                .contains(&derivation::RowRef::of(fixture.source.id()))
        );
        assert_eq!(
            fixture.source.proof().unwrap().premises,
            [derivation::RowRef::of(fixture.obligation.id())]
        );
        assert!(
            !local::DischargeEvidence::invariants()[0]
                .inputs
                .iter()
                .any(|i| i.name() == summary::Obligation::NAME)
        );
        assert!(fixture.check(true).is_err());
    }
    for relation in [
        Relation::of::<catalog_core::ObligationSubject>(),
        Relation::of::<catalog_evidence::ObligationSubject>(),
        Relation::of::<analytic_embedding::ObligationSubject>(),
    ] {
        assert!(
            !relation.fields().iter().any(|field| field
                .target()
                .is_some_and(|(_, name)| name == transfer::local::TransferKey::NAME)),
            "early subject refers to late transfer writer"
        );
    }
    lctx_model::domain::model().unwrap();
}

#[test]
fn wrong_subject_channel_phase_and_foreign_frames_cannot_close_a_question() {
    for case in 0..6 {
        let mut fixture = Fixture::new(false);
        match case {
            0 => {
                fixture.summary_subject = summary::ObligationSubject::SourceCall {
                    occurrence: nominal(99),
                }
            }
            1 => fixture.proposition.channel = AnalysisChannel::Exception,
            2 => fixture.proposition.phase = CallPhase::Init,
            3 => fixture.summary.input = nominal(99),
            4 => fixture.summary.context = nominal(99),
            5 => fixture.derivation.heuristic = true,
            _ => unreachable!(),
        }
        fixture.refresh();
        assert!(fixture.check(false).is_err(), "case {case}");
    }
    let mut fixture = Fixture::new(true);
    fixture.summary_subject = summary::ObligationSubject::Computation {
        invocation: fixture.summary.id(),
    };
    fixture.refresh();
    assert!(
        fixture.check(false).is_err(),
        "different computation owners were equated"
    );
}

#[test]
fn candidate_approximate_absent_and_partial_negative_evidence_stays_open() {
    for case in 0..6 {
        let mut fixture = Fixture::new(false);
        match case {
            0 => fixture.qualification.modality = Modality::Candidate,
            1 => fixture.qualification.approximation = Approximation::Over,
            2 => fixture.coverage.availability = EvidenceAvailability::Unavailable,
            3 => fixture.coverage.availability = EvidenceAvailability::NotRequested,
            4 => fixture.coverage.availability = EvidenceAvailability::NoScope,
            5 => {
                fixture.condition = Diagram::never();
                fixture.qualification.condition = fixture.condition.id();
            }
            _ => unreachable!(),
        }
        fixture.refresh();
        assert!(fixture.check(false).is_err(), "case {case}");
    }
}

#[test]
fn documentary_evidence_cannot_discharge_behavioral_questions() {
    for channel in [
        AnalysisChannel::Value,
        AnalysisChannel::Effect,
        AnalysisChannel::Exception,
        AnalysisChannel::Role,
        AnalysisChannel::Execution,
        AnalysisChannel::Completion,
    ] {
        let mut fixture = Fixture::new(false);
        fixture.obligation.channel = channel;
        fixture.proposition.channel = channel;
        fixture.derivation.status = EvidenceStatus::Documented;
        fixture.refresh();
        assert!(fixture.check(false).is_err());
    }
    let mut fixture = Fixture::new(false);
    fixture.obligation.channel = AnalysisChannel::Catalog;
    fixture.proposition.channel = AnalysisChannel::Catalog;
    fixture.derivation.status = EvidenceStatus::Documented;
    fixture.refresh();
    fixture.check(false).unwrap();
}
