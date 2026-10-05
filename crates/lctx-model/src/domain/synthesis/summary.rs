//! Per-value behavioral outcomes retain the earlier Summary question and its five-verdict answer.
use super::{documentary, frames};
use crate::Domain;
use crate::domain::{
    analysis::{self, synthesis as owner},
    catalog::CatalogMemberInvocation,
    execution::summary_consequences::{ClaimConclusion, SummaryClaim},
    normalized::{Rows, entities::EntityRef},
    resources::ResourceBudget,
    *,
};
#[derive(Debug, Clone, PartialEq, Eq, Domain,serde::Serialize,serde::Deserialize)]
#[model(name="synthesis_summary_facets",rule="synthesis_summary_facet",invariant_refs=invariants_refs,semantic_source=include_bytes!("summary.rs"))]
pub struct SummaryFacet {
    #[model(key)]
    pub frame: Id<frames::Frame>,
    #[model(key, premise)]
    pub conclusion: Id<ClaimConclusion>,
    #[model(key)]
    pub member: Option<Id<CatalogMemberInvocation>>,
    pub claim: Option<Id<SummaryClaim>>,
    pub qualification: Option<Id<assertion::AssertionQualification>>,
    pub coverage: attribution::CoverageStatus,
    pub verdict: obligation::Verdict,
    pub reason: Option<obligation::ObligationKind>,
    #[model(premise)]
    pub source: Option<Id<owner::SupportSource>>,
}
#[macro_export]
macro_rules! synthesis_summary_inputs{($m:ident)=>{$m!{
 claims:$crate::domain::execution::summary_consequences::SummaryClaim,conclusions:$crate::domain::execution::summary_consequences::ClaimConclusion,proofs:$crate::domain::execution::summary_consequences::ClaimProof,
 subjects:$crate::domain::analysis::summary::ObligationSubject,sources:$crate::domain::analysis::summary::SupportSource,derivations:$crate::domain::analysis::summary::AnalysisDerivation,propositions:$crate::domain::analysis::summary::AnalysisProposition,premises:$crate::domain::analysis::summary::AnalysisDerivationPremise,
 transfers:$crate::domain::transfer::summary::TransferKey,events:$crate::domain::normalized::events::NormalizedCallEvent,ownership:$crate::domain::normalized::entities::OccurrenceOwnership,
 symbolic_alternatives:$crate::domain::execution::summary_symbolic::SymbolicFieldAlternative,
}};}
macro_rules! data{($($f:ident:$t:ty,)*)=>{pub struct Data{$(pub $f:Rows<$t>,)*}impl Data{pub fn new(b:&ResourceBudget)->Self{Self{$($f:Rows::new(b),)*}}pub fn visit(&mut self,n:&str,b:&arrow_array::RecordBatch)->Result<bool,ModelError>{$(if n==<$t>::NAME{self.$f.decode(b)?;return Ok(true);})*Ok(false)}pub fn inputs()->Vec<ValidationInput>{vec![$(ValidationInput::of::<$t>(&["id"]),)*]}}};}
crate::synthesis_summary_inputs!(data);
fn invalid(s: &str) -> ModelError {
    ModelError::Invalid(s.into())
}
fn need<R: Record>(r: &Rows<R>, id: Id<R>) -> Result<&R, ModelError> {
    r.get(id)
        .ok_or_else(|| invalid("S0 exact Summary premise absent"))
}
fn claim<'a>(
    d: &'a Data,
    row: &ClaimConclusion,
) -> Result<Option<(&'a SummaryClaim, Id<SummaryClaim>)>, ModelError> {
    match need(&d.subjects, row.subject)? {
        analysis::summary::ObligationSubject::SummaryClaim { transfer } => {
            Ok(Some((need(&d.claims, *transfer)?, *transfer)))
        }
        analysis::summary::ObligationSubject::Computation { invocation }
            if *invocation == row.invocation =>
        {
            Ok(None)
        }
        _ => Err(invalid(
            "Summary consequence subject is not its exact claim/computation",
        )),
    }
}
fn entity(d: &Data, claim: &SummaryClaim) -> Result<Id<EntityRef>, ModelError> {
    match claim {
        SummaryClaim::NoNormalContinuation { owner, .. } => Ok(*owner),
        SummaryClaim::FiniteAlternative { transfer, .. } => {
            Ok(need(&d.transfers, *transfer)?.owner)
        }
        SummaryClaim::CallClosure { event, .. } => {
            Ok(need(&d.ownership, need(&d.events, *event)?.owner)?.entity)
        }
        SummaryClaim::SymbolicFieldAssociation { alternative, .. } => {
            Ok(need(&d.symbolic_alternatives, *alternative)?.constructor)
        }
    }
}
pub fn evidence(
    d: &Data,
    row: &ClaimConclusion,
) -> Result<Option<owner::SupportSource>, ModelError> {
    if let Some((
        SummaryClaim::SymbolicFieldAssociation {
            alternative,
            qualification,
        },
        _,
    )) = claim(d, row)?
    {
        let alternative = need(&d.symbolic_alternatives, *alternative)?;
        if row.proof.is_some()
            || row.verdict != obligation::Verdict::Unknown
            || row.qualification != Some(*qualification)
            || *qualification != alternative.reader_qualification
            || row.reason != Some(alternative.reason)
            || row.coverage != attribution::CoverageStatus::Partial
        {
            return Err(invalid(
                "symbolic association acquired behavioral proof authority",
            ));
        }
        return Ok(None);
    }
    let Some(proof) = row.proof else {
        return Ok(None);
    };
    let checked = need(&d.proofs, proof)?;
    let facts = analysis::support::DerivedEvidence::source_facts(checked);
    if row.qualification != Some(facts.qualification) {
        return Err(invalid("Summary consequence changes proof qualification"));
    }
    let Some((claim, claim_id)) = claim(d, row)? else {
        return Err(invalid("computational boundary cannot cite a value proof"));
    };
    if checked.claim() != claim_id {
        return Err(invalid("Summary proof names another exact question"));
    }
    let (channel, phase, qualification) = match claim {
        SummaryClaim::FiniteAlternative {
            channel,
            phase,
            qualification,
            ..
        }
        | SummaryClaim::CallClosure {
            channel,
            phase,
            qualification,
            ..
        } => (*channel, *phase, *qualification),
        SummaryClaim::NoNormalContinuation { .. } => {
            return Err(invalid(
                "terminal frontier requires its dedicated scoped source",
            ));
        }
        SummaryClaim::SymbolicFieldAssociation { .. } => {
            return Err(invalid("symbolic association cannot cite a finite proof"));
        }
    };
    let original = analysis::summary::SupportSource::ClaimProof { witness: proof };
    if d.sources.get(original.id()) != Some(&original) {
        return Err(invalid("Summary consequence proof lacks its owning source"));
    }
    let mut matching = d.derivations.iter().filter(|r| {
        r.invocation == row.invocation
            && r.qualification == qualification
            && d.propositions.get(r.proposition).is_some_and(|p| {
                p.subject == row.subject
                    && p.channel == channel
                    && p.phase == phase
                    && p.qualification == qualification
            })
            && d.premises
                .iter()
                .any(|p| p.derivation == r.id() && p.source == original.id())
    });
    let earlier = matching
        .next()
        .ok_or_else(|| invalid("Summary claim proof lacks its exact generic derivation"))?;
    if matching.next().is_some() {
        return Err(invalid(
            "Summary claim proof has ambiguous generic derivation",
        ));
    }
    Ok(Some(owner::SupportSource::Summary {
        derivation: earlier.id(),
    }))
}
pub fn build(
    d: &Data,
    docs: &documentary::Data,
    frames: &Rows<frames::Frame>,
    invocations: &Rows<owner::Invocation>,
    b: &ResourceBudget,
) -> Result<(Rows<SummaryFacet>, Rows<owner::SupportSource>), ModelError> {
    let mut out = Rows::new(b);
    let mut sources = Rows::new(b);
    for frame in frames.iter() {
        let inv = need(invocations, frame.invocation)?;
        for row in d
            .conclusions
            .iter()
            .filter(|r| r.invocation == frame.summary)
        {
            let c = claim(d, row)?;
            let source = evidence(d, row)?.map(|s| sources.insert(s)).transpose()?;
            let subject = c.map(|(c, _)| entity(d, c)).transpose()?;
            let mut linked = false;
            if let Some(entity) = subject {
                for member in docs.member_frames.iter().filter(|m| {
                    docs.core_invocations
                        .get(m.invocation)
                        .is_some_and(|p| (p.input, p.context) == (inv.input, inv.context))
                }) {
                    let candidate = docs.candidates.iter().any(|c| {
                        docs.exposures.get(c.exposure).is_some_and(|e| {
                            e.member == member.member
                                && docs
                                    .public_exposures
                                    .get(e.exposure)
                                    .is_some_and(|p| p.context == inv.context)
                        }) && documentary::entity(docs, c)
                            .is_ok_and(|candidate| candidate == Some(entity))
                    });
                    let alias = docs.aliases.iter().any(|a| {
                        a.entity == entity
                            && docs.exposures.get(a.parent).is_some_and(|e| {
                                e.member == member.member
                                    && docs
                                        .public_exposures
                                        .get(e.exposure)
                                        .is_some_and(|p| p.context == inv.context)
                            })
                    });
                    if candidate || alias {
                        out.insert(SummaryFacet {
                            frame: frame.id(),
                            conclusion: row.id(),
                            member: Some(member.id()),
                            claim: c.map(|(_, id)| id),
                            qualification: row.qualification,
                            coverage: row.coverage,
                            verdict: row.verdict,
                            reason: row.reason,
                            source,
                        })?;
                        linked = true;
                    }
                }
            }
            if !linked {
                out.insert(SummaryFacet {
                    frame: frame.id(),
                    conclusion: row.id(),
                    member: None,
                    claim: c.map(|(_, id)| id),
                    qualification: row.qualification,
                    coverage: row.coverage,
                    verdict: row.verdict,
                    reason: row.reason,
                    source,
                })?;
            }
        }
    }
    Ok((out, sources))
}
pub fn invariants() -> Vec<Invariant> {
    let mut inputs = Data::inputs();
    inputs.extend(documentary::replay_inputs(documentary::Data::validation_inputs(), stages::PublicationBoundary::Facts));
    inputs.extend([
        ValidationInput::of::<frames::Frame>(&["id"]),
        ValidationInput::of::<owner::Invocation>(&["id"]),
        ValidationInput::of::<SummaryFacet>(&["id"]),
        ValidationInput::of::<owner::SupportSource>(&["id"]),
    ]);
    inputs.sort_by_key(|r| (r.name(), r.prefix()));
    inputs.dedup_by_key(|r| (r.name(), r.prefix()));
    vec![Invariant {
        revision: 2,
        name: "synthesis_exact_summary_facets",
        inputs,
        create: std::sync::Arc::new(|b| {
            Box::new(Check {
                data: Data::new(b),
                docs: documentary::Data::new(b),
                frames: Rows::new(b),
                invocations: Rows::new(b),
                facets: Rows::new(b),
                sources: Rows::new(b),
                budget: b.clone(),
            })
        }),
    }]
}
struct Check {
    data: Data,
    docs: documentary::Data,
    frames: Rows<frames::Frame>,
    invocations: Rows<owner::Invocation>,
    facets: Rows<SummaryFacet>,
    sources: Rows<owner::SupportSource>,
    budget: ResourceBudget,
}
impl InvariantCheck for Check {
    fn visit(&mut self, _name: &str, _batch: &arrow_array::RecordBatch) -> Result<(), ModelError> {
        Err(ModelError::Invalid("S0 replay requires an explicit completed-input selector".into()))
    }
    fn visit_input(&mut self, input: &ValidationInput, b: &arrow_array::RecordBatch) -> Result<(), ModelError> {
        documentary::replay_selector(input)?;
        let n = input.name();
        let d = documentary::replay_visit(input, None, |name| self.data.visit(name, b))?;
        let docs = documentary::replay_visit(input, Some(stages::PublicationBoundary::Facts), |name| self.docs.visit(name, b))?;
        macro_rules! row {
            ($t:ty,$f:ident) => {
                if n == <$t>::NAME {
                    self.$f.decode(b)?;
                    return Ok(());
                }
            };
        }
        row!(frames::Frame, frames);
        row!(owner::Invocation, invocations);
        row!(SummaryFacet, facets);
        row!(owner::SupportSource, sources);
        if !d && !docs {
            return Err(invalid("undeclared S0 Summary facet input"));
        }
        Ok(())
    }
    fn finish(self: Box<Self>) -> Result<(), ModelError> {
        let (rows, sources) = build(
            &self.data,
            &self.docs,
            &self.frames,
            &self.invocations,
            &self.budget,
        )?;
        if !rows.same(&self.facets) {
            return Err(invalid(
                "S0 Summary question/verdict/member closure differs",
            ));
        }
        for row in sources.iter() {
            if self.sources.get(row.id()) != Some(row) {
                return Err(invalid("S0 Summary facet source erased"));
            }
        }
        Ok(())
    }
}

/// Generic consequences retain the original per-value/call phase and scope.
#[allow(
    clippy::too_many_arguments,
    reason = "Public synthesis boundary keeps separately admitted observation, facet, invocation and proof inputs explicit."
)]
pub fn extend_observations(
    d: &Data,
    o: &super::observations::Data,
    facets: &Rows<SummaryFacet>,
    frames: &Rows<frames::Frame>,
    invocations: &Rows<owner::Invocation>,
    coverages: &Rows<owner::AnalysisCoverage>,
    out: &mut super::observations::Output,
    b: &ResourceBudget,
) -> Result<(), ModelError> {
    use analysis::{
        policy::{FindingKind, MemberRole, SupportRole},
        support::QualificationOperation,
    };
    for facet in facets.iter() {
        let Some(source_id) = facet.source else {
            continue;
        };
        let row = need(&d.conclusions, facet.conclusion)?;
        let source = evidence(d, row)?.ok_or_else(|| invalid("Summary facet proof erased"))?;
        if source.id() != source_id {
            return Err(invalid("Summary facet source differs"));
        }
        let owner::SupportSource::Summary {
            derivation: earlier,
        } = source
        else {
            return Err(invalid("Summary facet does not cite nominal owner"));
        };
        let earlier = need(&d.derivations, earlier)?;
        let claim_id = facet
            .claim
            .ok_or_else(|| invalid("Summary value proof lacks exact claim"))?;
        let claim = need(&d.claims, claim_id)?;
        let (channel, phase) = match claim {
            SummaryClaim::FiniteAlternative { channel, phase, .. }
            | SummaryClaim::CallClosure { channel, phase, .. } => (*channel, *phase),
            SummaryClaim::NoNormalContinuation { .. } => {
                return Err(invalid(
                    "terminal frontier requires its dedicated scoped source",
                ));
            }
            SummaryClaim::SymbolicFieldAssociation { .. } => {
                return Err(invalid("symbolic association cannot emit a finding"));
            }
        };
        let frame = need(frames, facet.frame)?;
        let inv = need(invocations, frame.invocation)?;
        let q = need(&o.qualifications, earlier.qualification)?;
        if q.context != inv.context {
            return Err(invalid("S0 Summary consequence changes context"));
        }
        let coverage = coverages
            .iter()
            .find(|c| {
                c.invocation == inv.id()
                    && c.context == q.context
                    && c.scope == q.scope
                    && c.capability == analysis::AnalysisCapability::Synthesis
            })
            .ok_or_else(|| invalid("S0 Summary consequence lacks exact scope readiness"))?;
        let condition = need(&o.conditions, q.condition)?;
        let _buffer = b.reserve(
            "s0-summary-condition",
            o.nodes
                .len()
                .saturating_mul(size_of::<conditions::ConditionNode>()),
        )?;
        let nodes = o.nodes.iter().cloned().collect::<Vec<_>>();
        let diagram = conditions::Diagram::from_records(condition, &nodes)?;
        let subject = owner::ObligationSubject::SummaryClaim { transfer: claim_id };
        out.subjects.insert(subject.clone())?;
        out.sources.insert(source.clone())?;
        let premise = owner::support::EvidencePremise::derived(&source, earlier, q, &diagram)?;
        let (derived, proposition, premises, qualified) = owner::AnalysisDerivation::emit(
            inv,
            &super::build::definition().1,
            subject.id(),
            channel,
            phase,
            QualificationOperation::Conjunction,
            &[premise],
            b,
        )?;
        out.propositions.insert(proposition)?;
        out.derivations.insert(derived.clone())?;
        for premise in premises {
            out.premises.insert(premise)?;
        }
        let own = owner::SupportSource::AnalysisDerivation {
            derivation: derived.id(),
        };
        out.sources.insert(own.clone())?;
        let premise = owner::support::EvidencePremise::derived(
            &own,
            &derived,
            &qualified.qualification,
            &qualified.condition,
        )?;
        let (finding, members, supports, qualified) = analysis::findings::emit(
            inv.id(),
            if facet.verdict == obligation::Verdict::RefutedUnderModel {
                FindingKind::BehavioralRefutation
            } else {
                FindingKind::ApplicableCase
            },
            subject.id(),
            &[(MemberRole::Value, subject.id())],
            coverage,
            &[analysis::findings::FindingEvidence::new(
                SupportRole::Support,
                premise,
            )],
            b,
        )?;
        out.findings.insert(finding)?;
        for member in members {
            out.members.insert(member)?;
        }
        for support in supports {
            out.supports.insert(support)?;
        }
        out.qualifications.insert(qualified.qualification)?;
        let (condition, nodes) = qualified.condition.records();
        out.conditions.insert(condition)?;
        for node in nodes {
            out.nodes.insert(node)?;
        }
    }
    Ok(())
}
pub fn text(
    d: &Data,
    facet: &SummaryFacet,
    qualification: Option<&assertion::AssertionQualification>,
    b: &ResourceBudget,
) -> Result<String, ModelError> {
    if facet.qualification != qualification.map(Record::id) {
        return Err(invalid("Summary text changes its exact qualification"));
    }
    let _charge = b.reserve("s0-summary-facet-render", 1024)?;
    let question = match facet.claim.map(|c| need(&d.claims, c)).transpose()? {
        Some(SummaryClaim::SymbolicFieldAssociation { alternative, .. }) => {
            let _association = need(&d.symbolic_alternatives, *alternative)?;
            "Source associates a constructor parameter with a field read; whether a later read returns the stored value remains unresolved".into()
        }
        Some(SummaryClaim::FiniteAlternative {
            transfer,
            channel,
            phase,
            ..
        }) => format!(
            "Finite value alternative {} ({channel:?}, {phase:?})",
            transfer.hex()
        ),
        Some(SummaryClaim::CallClosure {
            event,
            input,
            output,
            channel,
            phase,
            ..
        }) => format!(
            "Call-member closure {} from {} to {} ({channel:?}, {phase:?})",
            event.hex(),
            input.hex(),
            output.hex()
        ),
        Some(SummaryClaim::NoNormalContinuation {..}) => "Given invocation entered, the qualified direct normal continuation is excluded; exceptions, effects and cleanup remain unresolved".into(),
        None => "Behavioral analysis for this captured frame".into(),
    };
    let basis = match qualification {
        Some(q) if q.assumptions == assumptions::AssumptionSet::empty_id() => {
            "No additional typing or closed-world assumptions".to_owned()
        }
        Some(q) => format!(
            "Conditional on the stated typing or closed-world premises (basis {})",
            q.assumptions.hex()
        ),
        None => "Qualification and assumption basis are unavailable".to_owned(),
    };
    Ok(format!(
        "{question}: {:?}; coverage {:?}; reason {:?}; exact conclusion {}. {basis}.",
        facet.verdict,
        facet.coverage,
        facet.reason,
        facet.conclusion.hex()
    ))
}

pub(crate) fn invariants_refs() -> Vec<&'static str> {
    vec!["synthesis_exact_summary_facets"]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::execution::summary_consequences::ClaimProof;
    fn id<T>(n: u8) -> Id<T> {
        serde_json::from_value(serde_json::json!(vec![n; 16])).unwrap()
    }
    /// These pure downstream controls assume the earlier Summary owner admitted the proof. Native
    /// worklist and execution correctness are separately qualified by that owner's production tests.
    fn fixture(
        proven: bool,
        verdict: obligation::Verdict,
    ) -> (
        ResourceBudget,
        Data,
        documentary::Data,
        super::super::observations::Data,
        Rows<frames::Frame>,
        Rows<owner::Invocation>,
        Rows<owner::AnalysisCoverage>,
    ) {
        let (b, parents) = frames::tests::fixture();
        let parent = frames::parents(&parents, &b).unwrap().remove(0);
        let inv = owner::Invocation::new(
            parent.input,
            parent.context,
            super::super::build::definition().1.id(),
            None,
            parent.sources.iter().map(Record::id),
        )
        .0;
        let frame = frames::frame(&parent, inv.id());
        let mut d = Data::new(&b);
        let mut o = super::super::observations::Data::new(&b);
        let condition = if verdict == obligation::Verdict::RefutedUnderModel {
            conditions::Diagram::never()
        } else {
            conditions::Diagram::always()
        };
        let q = assertion::AssertionQualification {
            assumptions: lctx_model::domain::assumptions::AssumptionSet::empty_id(),
            context: inv.context,
            scope: source::CoverageScope::Artifact { artifact: id(71) }.id(),
            condition: condition.id(),
            modality: attribution::Modality::Definite,
            approximation: assertion::Approximation::Exact,
        };
        o.qualifications.insert(q.clone()).unwrap();
        let (c, nodes) = condition.records();
        o.conditions.insert(c).unwrap();
        for n in nodes {
            o.nodes.insert(n).unwrap();
        }
        let key = transfer::summary::TransferKey {
            owner: id(72),
            input: id(73),
            output: id(74),
            context: q.context,
            scope: q.scope,
            modality: q.modality,
            approximation: q.approximation,
            kind: transfer::TransferKind::Identity,
            call_site: None,
            provenance: transfer::ProvenanceClass::FlowLocal,
        };
        d.transfers.insert(key.clone()).unwrap();
        let claim = SummaryClaim::FiniteAlternative {
            transfer: key.id(),
            qualification: q.id(),
            channel: analysis::AnalysisChannel::Value,
            phase: calls::CallPhase::Call,
        };
        let subject = analysis::summary::ObligationSubject::SummaryClaim {
            transfer: claim.id(),
        };
        d.claims.insert(claim.clone()).unwrap();
        d.subjects.insert(subject.clone()).unwrap();
        let proof = if verdict == obligation::Verdict::RefutedUnderModel {
            ClaimProof::Refutation {
                invocation: parent.summary,
                claim: claim.id(),
                source: id(75),
                qualification: q.id(),
                status: analysis::policy::EvidenceStatus::StructurallyObserved,
                heuristic: false,
                coverage: ContentHash::of(b"earlier-complete-refutation-coverage"),
            }
        } else {
            ClaimProof::Finite {
                claim: claim.id(),
                source: id(75),
                qualification: q.id(),
                status: analysis::policy::EvidenceStatus::StructurallyObserved,
                heuristic: false,
            }
        };
        if proven {
            let source = analysis::summary::SupportSource::ClaimProof {
                witness: proof.id(),
            };
            let premise = analysis::summary::support::EvidencePremise::derived(
                &source, &proof, &q, &condition,
            )
            .unwrap();
            let earlier = parents.summary.get(parent.summary).unwrap();
            let definition = parents.definitions.get(earlier.definition).unwrap();
            let (derived, proposition, premises, _) = analysis::summary::AnalysisDerivation::emit(
                earlier,
                definition,
                subject.id(),
                analysis::AnalysisChannel::Value,
                calls::CallPhase::Call,
                analysis::support::QualificationOperation::Conjunction,
                &[premise],
                &b,
            )
            .unwrap();
            d.sources.insert(source).unwrap();
            d.derivations.insert(derived).unwrap();
            d.propositions.insert(proposition).unwrap();
            for p in premises {
                d.premises.insert(p).unwrap();
            }
            d.proofs.insert(proof.clone()).unwrap();
        }
        d.conclusions
            .insert(ClaimConclusion {
                invocation: parent.summary,
                subject: subject.id(),
                qualification: Some(q.id()),
                proof: proven.then_some(proof.id()),
                coverage: attribution::CoverageStatus::Partial,
                verdict,
                reason: (!proven).then_some(obligation::ObligationKind::MissingEvidence),
            })
            .unwrap();
        let mut frames = Rows::new(&b);
        frames.insert(frame).unwrap();
        let mut invocations = Rows::new(&b);
        invocations.insert(inv.clone()).unwrap();
        let mut coverage = Rows::new(&b);
        coverage
            .insert(owner::AnalysisCoverage {
                invocation: inv.id(),
                capability: analysis::AnalysisCapability::Synthesis,
                scope: q.scope,
                context: q.context,
                premises: ContentHash::of(b"earlier-admitted-scope-control"),
                availability: normalized::coverage::EvidenceAvailability::Complete,
                reason: None,
            })
            .unwrap();
        let docs = documentary::Data::new(&b);
        (b, d, docs, o, frames, invocations, coverage)
    }
    #[test]
    fn summary_text_preserves_the_exact_assumption_basis() {
        let (b, d, docs, o, f, i, _) = fixture(true, obligation::Verdict::Conditional);
        let (facets, _) = build(&d, &docs, &f, &i, &b).unwrap();
        let facet = facets.iter().next().unwrap();
        let original = o.qualifications.get(facet.qualification.unwrap()).unwrap();
        assert!(
            text(&d, facet, Some(original), &b)
                .unwrap()
                .contains("No additional typing or closed-world assumptions")
        );
        let mut qualified = original.clone();
        qualified.assumptions = id(97);
        let mut conditional = facet.clone();
        conditional.qualification = Some(qualified.id());
        let rendered = text(&d, &conditional, Some(&qualified), &b).unwrap();
        assert!(rendered.contains("Conditional on the stated typing or closed-world premises"));
        assert!(rendered.contains(&qualified.assumptions.hex()));
        assert!(!rendered.contains("No additional typing"));
        assert!(text(&d, &conditional, Some(original), &b).is_err());
    }
    #[test]
    fn summary_question_retains_original_artifact_call_identity_and_negative_verdict() {
        for verdict in [
            obligation::Verdict::Established,
            obligation::Verdict::Conditional,
            obligation::Verdict::RefutedUnderModel,
        ] {
            let (b, d, docs, o, f, i, c) = fixture(true, verdict);
            let (facets, _) = build(&d, &docs, &f, &i, &b).unwrap();
            let facet = facets.iter().next().unwrap();
            assert_eq!(facet.verdict, verdict);
            assert!(
                text(
                    &d,
                    facet,
                    facet.qualification.and_then(|q| o.qualifications.get(q)),
                    &b
                )
                .unwrap()
                .contains(&format!("{verdict:?}"))
            );
            let mut out = super::super::observations::Output::new(&b);
            extend_observations(&d, &o, &facets, &f, &i, &c, &mut out, &b).unwrap();
            assert_eq!(out.findings.len(), 1);
            assert_eq!(
                out.findings.iter().next().unwrap().kind,
                if verdict == obligation::Verdict::RefutedUnderModel {
                    analysis::policy::FindingKind::BehavioralRefutation
                } else {
                    analysis::policy::FindingKind::ApplicableCase
                }
            );
            let proposition = out.propositions.iter().next().unwrap();
            assert_eq!(
                (proposition.channel, proposition.phase),
                (analysis::AnalysisChannel::Value, calls::CallPhase::Call)
            );
            assert_eq!(proposition.qualification, facet.qualification.unwrap());
            assert!(
                matches!(out.subjects.iter().next().unwrap(),owner::ObligationSubject::SummaryClaim{transfer}if Some(*transfer)==facet.claim)
            );
        }
    }
    #[test]
    fn unknown_and_not_analyzed_are_facets_without_finding_authority() {
        for verdict in [
            obligation::Verdict::Unknown,
            obligation::Verdict::NotAnalyzed,
        ] {
            let (b, d, docs, o, f, i, c) = fixture(false, verdict);
            let (facets, sources) = build(&d, &docs, &f, &i, &b).unwrap();
            assert_eq!(facets.len(), 1);
            assert!(sources.is_empty());
            let mut out = super::super::observations::Output::new(&b);
            extend_observations(&d, &o, &facets, &f, &i, &c, &mut out, &b).unwrap();
            assert!(out.findings.is_empty() && out.derivations.is_empty());
        }
    }
    #[test]
    fn missing_or_changed_exact_generic_summary_premise_refuses() {
        for case in 0..4 {
            let (b, mut d, docs, _, f, i, _) = fixture(true, obligation::Verdict::Established);
            match case {
                0 => d.sources = Rows::new(&b),
                1 => d.premises = Rows::new(&b),
                2 => {
                    let row = d.conclusions.iter().next().unwrap().clone();
                    let mut forged = row.clone();
                    forged.subject = d
                        .subjects
                        .insert(analysis::summary::ObligationSubject::SummaryClaim {
                            transfer: d
                                .claims
                                .insert(SummaryClaim::FiniteAlternative {
                                    transfer: d.transfers.iter().next().unwrap().id(),
                                    qualification: row.qualification.unwrap(),
                                    channel: analysis::AnalysisChannel::Catalog,
                                    phase: calls::CallPhase::Call,
                                })
                                .unwrap(),
                        })
                        .unwrap();
                    d.conclusions = Rows::new(&b);
                    d.conclusions.insert(forged).unwrap();
                }
                _ => {
                    let mut p = d.propositions.iter().next().unwrap().clone();
                    p.phase = calls::CallPhase::Definition;
                    d.propositions = Rows::new(&b);
                    d.propositions.insert(p).unwrap();
                }
            }
            assert!(build(&d, &docs, &f, &i, &b).is_err());
        }
    }
    #[test]
    fn complete_facet_replay_refuses_forged_verdict_or_coupled_deletion() {
        let (b, d, docs, _, f, i, _) = fixture(false, obligation::Verdict::Unknown);
        let (facets, sources) = build(&d, &docs, &f, &i, &b).unwrap();
        for case in 0..3 {
            let mut check = Check {
                data: Data::new(&b),
                docs: documentary::Data::new(&b),
                frames: Rows::new(&b),
                invocations: Rows::new(&b),
                facets: Rows::new(&b),
                sources: Rows::new(&b),
                budget: b.clone(),
            };
            macro_rules! data{($($field:ident:$ty:ty,)*)=>{$(check.visit_input(&ValidationInput::of::<$ty>(&["id"]),&<$ty as Record>::encode(&d.$field.iter().cloned().collect::<Vec<_>>()).unwrap()).unwrap();)*};}
            crate::synthesis_summary_inputs!(data);
            macro_rules! docs{($($field:ident:$ty:ty,)*)=>{$(check.visit_input(&documentary::replay_input::<$ty>(stages::PublicationBoundary::Facts),&<$ty as Record>::encode(&docs.$field.iter().cloned().collect::<Vec<_>>()).unwrap()).unwrap();)*};}
            crate::synthesis_documentary_inputs!(docs);
            for row in f.iter() {
                check.frames.insert(row.clone()).unwrap();
            }
            for row in i.iter() {
                check.invocations.insert(row.clone()).unwrap();
            }
            if case != 2 {
                for row in facets.iter() {
                    let mut row = row.clone();
                    if case == 1 {
                        row.verdict = obligation::Verdict::Established;
                    }
                    check.facets.insert(row).unwrap();
                }
                for row in sources.iter() {
                    check.sources.insert(row.clone()).unwrap();
                }
            }
            assert_eq!(Box::new(check).finish().is_ok(), case == 0);
        }
    }
}
