//! Fresh binding publication is independent of body outcomes and caller continuation.
use super::{
    evaluation::EvaluationData,
    records::ordered_digest,
    source_call::{CheckedSourceBinding, SourceCallRequest},
};
use crate::domain::{
    analysis::{
        self, native::NativeAssertionPremise, policy::EvidenceStatus, source_call as publication,
    },
    conditions::entry::EntryData,
    normalized::{
        Rows,
        binding_normalization::{BindingData, BindingOutput},
    },
    resources::ResourceBudget,
    source::Occurrence,
    *,
};
use crate::{Domain, DomainSum};
#[derive(Debug, Clone, PartialEq, Eq, Domain, serde::Serialize, serde::Deserialize)]
#[model(name="source_call_headers",rule="fresh_source_binding",invariant_refs=header_fidelity_refs)]
pub struct SourceCallHeader {
    #[model(key, premise)]
    pub invocation: Id<publication::AnalysisInvocation>,
    #[model(key)]
    pub event: Id<normalized::events::NormalizedCallEvent>,
    #[model(premise)]
    pub attempt: Id<normalized::bindings::CallBindingAttempt>,
    pub owner: Id<normalized::entities::EntityRef>,
    pub callee: Id<normalized::entities::EntityRef>,
    pub declaration: Id<Occurrence>,
    pub qualification: Id<assertion::AssertionQualification>,
    pub status: EvidenceStatus,
    pub premises: ContentHash,
}
#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(name="source_call_header_members",rule="source_call_header_premise",conclusion=header)]
pub struct HeaderMember {
    #[model(key)]
    pub header: Id<SourceCallHeader>,
    #[model(key)]
    pub ordinal: i64,
    #[model(premise)]
    pub premise: Id<NativeAssertionPremise>,
}
#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(name = "source_call_boundaries")]
pub struct SourceCallBoundary {
    #[model(key)]
    pub invocation: Id<publication::AnalysisInvocation>,
    #[model(key)]
    pub event: Id<normalized::events::NormalizedCallEvent>,
    pub reason: obligation::ObligationKind,
}
#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(name="source_call_runs",invariant_refs=source_call_invariants_refs,publication_refs=profile_checks_refs)]
pub struct SourceCallRun {
    #[model(key)]
    pub invocation: Id<publication::AnalysisInvocation>,
    pub requested: bool,
    pub bound: i64,
    pub refused: i64,
}
impl analysis::support::sealed::DerivedEvidence for SourceCallHeader {}
impl analysis::support::DerivedEvidence for SourceCallHeader {
    fn source_facts(&self) -> analysis::support::SourceFacts {
        analysis::support::SourceFacts {
            qualification: self.qualification,
            status: self.status,
            heuristic: false,
        }
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Hash, DomainSum, serde::Serialize, serde::Deserialize)]
#[model(name = "source_call_outcomes")]
pub enum SourceCallOutcome {
    #[model(code = 0)]
    Normal,
    #[model(code = 1)]
    Raised {
        site: Id<Occurrence>,
        exception: super::ExactRuntimeException,
    },
}
#[derive(Debug, Clone, PartialEq, Eq, Domain, serde::Serialize, serde::Deserialize)]
#[model(
    name = "source_call_frame_releases",
    rule = "source_call_frame_release",
    invariant_refs = release_fidelity_refs
)]
pub struct SourceFrameRelease {
    #[model(key, premise)]
    pub header: Id<SourceCallHeader>,
    #[model(key, premise)]
    pub body: Id<super::body_records::SourceBodyCompletion>,
    pub arguments: ContentHash,
    pub release: super::evaluation::ReleaseSafety,
}
#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(name="source_frame_arguments",rule="source_frame_argument",conclusion=release,invariant_refs=release_fidelity_refs)]
pub struct SourceFrameArgument {
    #[model(key)]
    pub release: Id<SourceFrameRelease>,
    #[model(key)]
    pub ordinal: i64,
    pub formal: Id<calls::SignatureParameter>,
    pub actual: Id<Occurrence>,
    #[model(premise)]
    pub evaluation: Id<super::records::ExpressionEvaluation>,
}
#[derive(Debug, Clone, PartialEq, Eq, Domain, serde::Serialize, serde::Deserialize)]
#[model(name="source_call_invocations",rule="source_call_invocation",invariant_refs=invocation_fidelity_refs)]
pub struct SourceInvocation {
    #[model(key, premise)]
    pub invocation: Id<publication::AnalysisInvocation>,
    #[model(key, premise)]
    pub release: Id<SourceFrameRelease>,
    pub qualification: Id<assertion::AssertionQualification>,
    pub outcome: Id<SourceCallOutcome>,
    pub status: EvidenceStatus,
}
#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(name = "source_call_invocation_boundaries")]
pub struct InvocationBoundary {
    #[model(key)]
    pub header: Id<SourceCallHeader>,
    pub reason: obligation::ObligationKind,
}
impl analysis::support::sealed::DerivedEvidence for SourceInvocation {}
impl analysis::support::DerivedEvidence for SourceInvocation {
    fn source_facts(&self) -> analysis::support::SourceFacts {
        analysis::support::SourceFacts {
            qualification: self.qualification,
            status: self.status,
            heuristic: false,
        }
    }
}
pub struct SourceCallData {
    pub parameter_syntax: Rows<syntax::ParameterSyntaxObservation>,
    pub evaluation: EvaluationData,
    pub flow: EntryData,
    pub bindings: BindingData,
    pub output: BindingOutput,
    pub completed: super::completion_production::CompletedEvaluations,
    pub base: Rows<analysis::base_completion::AnalysisInvocation>,
    pub definitions: Rows<analysis::AnalysisDefinition>,
    pub bodies: Rows<super::body_records::SourceBodyCompletion>,
    pub body_sources:Rows<super::body_records::BodySource>,
    pub body_members:Rows<super::body_records::BodyMember>,
    pub body_releases:Rows<super::body_records::BodyReleaseInput>,
    pub body_statements:Rows<super::completion_records::StatementCompletion>,
}

impl SourceCallData {
    pub fn new(budget: &ResourceBudget) -> Self {
        Self {
            parameter_syntax: Rows::new(budget),
            evaluation: EvaluationData::new(budget),
            flow: EntryData::new(budget),
            bindings: BindingData::new(budget),
            output: BindingOutput::new(budget),
            completed: super::completion_production::CompletedEvaluations::new(budget),
            base: Rows::new(budget),
            definitions: Rows::new(budget),
            bodies: Rows::new(budget),
            body_sources:Rows::new(budget),body_members:Rows::new(budget),body_releases:Rows::new(budget),body_statements:Rows::new(budget),
        }
    }
    pub fn visit(
        &mut self,
        name: &str,
        batch: &arrow_array::RecordBatch,
    ) -> Result<(), ModelError> {
        self.completed.visit(name, batch)?;
        if name == syntax::ParameterSyntaxObservation::NAME {
            self.parameter_syntax.decode(batch)?;
        }
        if name == analysis::base_completion::AnalysisInvocation::NAME {
            self.base.decode(batch)?;
        } else if name == analysis::AnalysisDefinition::NAME {
            self.definitions.decode(batch)?;
        } else if name == super::body_records::SourceBodyCompletion::NAME {
            self.bodies.decode(batch)?;
        }
        macro_rules! body {($($field:ident:$ty:ty,)*)=>{$(if name==<$ty>::NAME {self.$field.decode(batch)?;})*};}
        body! {body_sources:super::body_records::BodySource,body_members:super::body_records::BodyMember,body_releases:super::body_records::BodyReleaseInput,body_statements:super::completion_records::StatementCompletion,}
        self.evaluation.visit(name, batch)?;
        self.flow.visit(name, batch)?;
        self.bindings.visit(name, batch)?;
        macro_rules! output{($($field:ident:$ty:ty,)*)=>{$(if name==<$ty>::NAME{self.output.$field.decode(batch)?;})*};}
        crate::normalized_binding_outputs!(output);
        Ok(())
    }
    /// Hydrate only memberships selected from actual predecessor body output streams. Scalar
    /// result authority remains private to the issuing owner; these rows cannot mint a body.
    pub(crate) fn hydrate_body(&self,row:&super::body_records::SourceBodyCompletion,value:super::body::ProducedBodyValue,budget:&ResourceBudget)->Result<super::body::CheckedSourceBody,ModelError> {
        use super::body_records::BodySource;
        let invalid=|message:&str|ModelError::Invalid(message.into());
        if self.bodies.get(row.id())!=Some(row) {return Err(invalid("actual body row changed"));}
        let count=self.body_members.iter().filter(|member|member.body==row.id()).count()+self.body_releases.iter().filter(|release|release.body==row.id()).count();
        let _scratch=budget.reserve("actual-body-membership",count.checked_mul(160).ok_or_else(||invalid("actual body membership allowance overflow"))?)?;
        let mut members=self.body_members.iter().filter(|member|member.body==row.id()).collect::<Vec<_>>();members.sort_by_key(|member|member.ordinal);
        let mut native=Vec::new();let mut statements=Vec::new();let mut sources=Vec::new();
        for (ordinal,member) in members.into_iter().enumerate() {
            if member.ordinal!=ordinal as i64 {return Err(invalid("actual body source order differs"));}
            let source=self.body_sources.get(member.source).ok_or_else(||invalid("actual body source absent"))?;sources.push(source.id());
            match source {BodySource::Native {premise}=>native.push(*premise),BodySource::Statement {completion}=> {let statement=self.body_statements.get(*completion).ok_or_else(||invalid("actual body statement absent"))?;if statement.owner!=row.owner || statement.invocation!=row.invocation {return Err(invalid("actual body statement belongs to another owner/frame"));}statements.push(statement.statement);}}
        }
        if ordered_digest("base-source-body-sources",sources.into_iter())!=row.sources {return Err(invalid("actual body sources differ from owner receipt"));}
        let mut selected=self.body_releases.iter().filter(|release|release.body==row.id()).collect::<Vec<_>>();selected.sort_by_key(|release|release.ordinal);
        let mut releases=Vec::with_capacity(selected.len());let mut digest=KeySink::new("base-source-body-releases");
        for (ordinal,release) in selected.into_iter().enumerate() {if release.ordinal!=ordinal as i64 {return Err(invalid("actual body release order differs"));}release.expression.encode(&mut digest);release.safety.encode(&mut digest);releases.push((release.expression,release.safety));}
        if digest.finish()!=row.releases {return Err(invalid("actual body releases differ from owner receipt"));}
        let proof=value.hydrate(native,statements,releases,budget)?;
        let frame=self.base.get(row.invocation).ok_or_else(||invalid("actual body frame absent"))?;
        if proof.request().input!=frame.input || proof.request().context!=frame.context || proof.request().callee!=row.owner || proof.declaration()!=row.declaration || proof.qualification()!=row.qualification || proof.status()!=row.status || super::completion_records::CompletionOutcome::from(proof.outcome()).id()!=row.outcome {return Err(invalid("actual body scalar differs from owner receipt"));}
        Ok(proof)
    }
    pub fn visit_input(
        &mut self,
        input: &ValidationInput,
        batch: &arrow_array::RecordBatch,
    ) -> Result<(), ModelError> {
        super::require_facts_view(input)?;
        self.visit(input.name(), batch)
    }
    pub fn consumed_inputs(profile: stages::Profile) -> Vec<ValidationInput> {
        if profile == stages::Profile::Behavioral {
            return Self::inputs();
        }
        vec![
            ValidationInput::of::<analysis::base_completion::AnalysisInvocation>(&["id"]),
            ValidationInput::of::<analysis::AnalysisDefinition>(&["id"]),
        ]
    }
    pub fn inputs() -> Vec<ValidationInput> {
        let mut inputs = EvaluationData::validation_inputs();
        inputs.push(ValidationInput::of::<syntax::ParameterSyntaxObservation>(
            &["id"],
        ));
        inputs.extend(EntryData::validation_inputs().into_iter().map(|input| {
            if stages::is_vocabulary(input.name()) {
                input.at_epoch(stages::PublicationBoundary::Facts)
            } else {
                input
            }
        }));
        inputs.extend(BindingData::validation_inputs());
        inputs.extend(BindingOutput::validation_inputs());
        inputs.extend(
            super::records::base_invariants()
                .remove(0)
                .inputs
                .into_iter()
                .map(|input| {
                    if stages::is_vocabulary(input.name()) {
                        input.at_epoch(stages::PublicationBoundary::Facts)
                    } else {
                        input
                    }
                }),
        );
        inputs.extend([
            ValidationInput::of::<analysis::base_completion::AnalysisInvocation>(&["id"]),
            ValidationInput::of::<super::body_records::SourceBodyCompletion>(&["id"]),
            ValidationInput::of::<super::body_records::BodySource>(&["id"]),
            ValidationInput::of::<super::body_records::BodyMember>(&["id"]),
            ValidationInput::of::<super::body_records::BodyReleaseInput>(&["id"]),
            ValidationInput::of::<super::completion_records::StatementCompletion>(&["id"]),
        ]);
        inputs.sort_by_key(|i| (i.name(), i.prefix()));
        inputs.dedup_by_key(|i| (i.name(), i.prefix()));
        inputs
    }
}
pub struct SourceCallRecords {
    pub run: SourceCallRun,
    pub headers: Rows<SourceCallHeader>,
    pub members: Rows<HeaderMember>,
    pub boundaries: Rows<SourceCallBoundary>,
    pub outcome: publication::AnalysisOutcome,
    pub invocations: Rows<SourceInvocation>,
    pub releases: Rows<SourceFrameRelease>,
    pub arguments: Rows<SourceFrameArgument>,
    pub call_outcomes: Rows<SourceCallOutcome>,
    pub invocation_boundaries: Rows<InvocationBoundary>,
}
/// Actual SourceCall owner values, held by immutable frame and never reconstructed from rows.
pub struct ProducedSourceCalls {
    frames: Vec<ProducedSourceFrame>,
    charge: charged::StateCharge,
}
struct ProducedSourceFrame {
    invocation: publication::AnalysisInvocation,
    headers: Vec<(CheckedSourceBinding, SourceCallHeader)>,
    calls: Vec<(super::source_invocation::CheckedSourceInvocation, SourceInvocation)>,
    _charge: charged::StateCharge,
}
impl ProducedSourceCalls {
    pub fn append(&mut self, mut other: Self) -> Result<(), ModelError> {
        if !self.charge.budget().expect("bound owner").shares_pool(other.charge.budget().expect("bound owner")) {
            return Err(ModelError::Conflict("produced SourceCall budget"));
        }
        self.charge.grow(other.frames.len().saturating_mul(size_of::<ProducedSourceFrame>() * 2))?;
        self.frames.append(&mut other.frames);
        Ok(())
    }
    pub(crate) fn visit_frame<T>(&self, invocation: &publication::AnalysisInvocation,
        visit: impl FnOnce(&[(CheckedSourceBinding, SourceCallHeader)], &[(super::source_invocation::CheckedSourceInvocation, SourceInvocation)]) -> Result<T, ModelError>,
    ) -> Result<T, ModelError> {
        let mut matching = self.frames.iter().filter(|frame| frame.invocation.id() == invocation.id());
        let frame = matching.next().ok_or(ModelError::Conflict("produced SourceCall frame absent"))?;
        if matching.next().is_some() || frame.invocation != *invocation { return Err(ModelError::Conflict("produced SourceCall frame changed")); }
        visit(&frame.headers, &frame.calls)
    }
}
pub fn prepare_all(
    data: &SourceCallData,
    invocation: &publication::AnalysisInvocation,
    definition: &analysis::AnalysisDefinition,
    profile: stages::Profile,
    budget: &ResourceBudget,
) -> Result<SourceCallRecords, ModelError> {
    prepare_all_with(
        data,
        invocation,
        definition,
        profile,
        budget,
        &mut |_, _| Ok(()),
    )
}
pub(crate) fn prepare_all_with(
    data: &SourceCallData,
    invocation: &publication::AnalysisInvocation,
    definition: &analysis::AnalysisDefinition,
    profile: stages::Profile,
    budget: &ResourceBudget,
    on_complete: &mut impl FnMut(
        &[(CheckedSourceBinding, SourceCallHeader)],
        &[(
            super::source_invocation::CheckedSourceInvocation,
            SourceInvocation,
        )],
    ) -> Result<(), ModelError>,
) -> Result<SourceCallRecords, ModelError> {
    let verified = if profile == stages::Profile::Behavioral {
        Some(normalized::binding_normalization::prepare(&data.bindings, &data.output, budget)?)
    } else { None };
    prepare_with_application(data, invocation, definition, profile, budget, verified.as_ref(), on_complete)
}
/// Consume the binding owner's private application authority; no predecessor preparation runs.
pub fn prepare_all_prepared(
    data: &SourceCallData, invocation: &publication::AnalysisInvocation,
    definition: &analysis::AnalysisDefinition, profile: stages::Profile,
    budget: &ResourceBudget, verified: Option<&normalized::binding_normalization::VerifiedBindings>,
) -> Result<SourceCallRecords, ModelError> {
    prepare_with_application(data, invocation, definition, profile, budget, verified, &mut |_, _| Ok(()))
}
pub(crate) fn prepare_with_application(
    data: &SourceCallData, invocation: &publication::AnalysisInvocation,
    definition: &analysis::AnalysisDefinition, profile: stages::Profile,
    budget: &ResourceBudget, verified: Option<&normalized::binding_normalization::VerifiedBindings>,
    on_complete: &mut impl FnMut(
        &[(CheckedSourceBinding, SourceCallHeader)],
        &[(super::source_invocation::CheckedSourceInvocation, SourceInvocation)],
    ) -> Result<(), ModelError>,
) -> Result<SourceCallRecords, ModelError> {
    prepare_with_values(data, invocation, definition, profile, budget, verified, None, None, on_complete).map(|(records, _)| records)
}
/// Fresh production requires the actual predecessor owners, rather than replaying stored outputs.
pub fn prepare_all_produced(
    data: &SourceCallData, invocation: &publication::AnalysisInvocation,
    definition: &analysis::AnalysisDefinition, profile: stages::Profile,
    budget: &ResourceBudget, verified: Option<&normalized::binding_normalization::VerifiedBindings>,
    evaluations: Option<&super::production::ProducedEvaluations>,
    bodies: Option<&super::completion_production::ProducedBodies>,
) -> Result<(SourceCallRecords, ProducedSourceCalls), ModelError> {
    if profile == stages::Profile::Behavioral && (evaluations.is_none() || bodies.is_none()) {
        return Err(ModelError::Conflict("requested SourceCall predecessor owner absent"));
    }
    prepare_with_values(data, invocation, definition, profile, budget, verified, evaluations, bodies, &mut |_, _| Ok(()))
}
fn prepare_with_values(
    data: &SourceCallData, invocation: &publication::AnalysisInvocation,
    definition: &analysis::AnalysisDefinition, profile: stages::Profile,
    budget: &ResourceBudget, verified: Option<&normalized::binding_normalization::VerifiedBindings>,
    evaluations: Option<&super::production::ProducedEvaluations>,
    bodies: Option<&super::completion_production::ProducedBodies>,
    on_complete: &mut impl FnMut(&[(CheckedSourceBinding, SourceCallHeader)], &[(super::source_invocation::CheckedSourceInvocation, SourceInvocation)]) -> Result<(), ModelError>,
) -> Result<(SourceCallRecords, ProducedSourceCalls), ModelError> {
    let invalid = |s: &str| ModelError::Invalid(s.into());
    if *definition != super::configuration::source_calls().1
        || invocation.definition != definition.id()
        || invocation.subject.is_some()
    {
        return Err(invalid("source call definition/frame is unbound"));
    }
    let mut records = SourceCallRecords {
        run: SourceCallRun {
            invocation: invocation.id(),
            requested: profile == stages::Profile::Behavioral,
            bound: 0,
            refused: 0,
        },
        headers: Rows::new(budget),
        members: Rows::new(budget),
        boundaries: Rows::new(budget),
        outcome: publication::AnalysisOutcome {
            invocation: invocation.id(),
            status: analysis::AnalysisStatus::Completed,
            reason: None,
        },
        invocations: Rows::new(budget),
        releases: Rows::new(budget),
        arguments: Rows::new(budget),
        call_outcomes: Rows::new(budget),
        invocation_boundaries: Rows::new(budget),
    };
    if profile == stages::Profile::Catalog {
        records.outcome.status = analysis::AnalysisStatus::NotRequested;
        records.outcome.reason = Some(obligation::ObligationKind::NotRequested);
        return Ok((records, ProducedSourceCalls { frames: Vec::new(), charge: charged::StateCharge::new(budget, "produced-source-calls") }));
    }
    let mut headers = Vec::new();
    let mut header_charge = charged::StateCharge::new(budget, "source_call_private_headers");
    let verified = verified.ok_or_else(|| invalid("requested SourceCall application authority absent"))?;
    let mut charge = charged::StateCharge::new(budget, "source_call_root_inventory");
    let bytes = data
        .evaluation
        .artifacts
        .iter()
        .try_fold(0usize, |n, r| {
            n.checked_add(size_of::<source::SourceArtifact>() + r.heap_bytes() + 256)
        })
        .and_then(|n| {
            n.checked_add(
                data.evaluation
                    .uses
                    .len()
                    .checked_mul(size_of::<input::ArtifactUse>() + 256)?,
            )
        })
        .ok_or_else(|| invalid("source call root allowance overflow"))?;
    charge.grow(bytes)?;
    let roots = admission::analysis_roots(
        &data
            .evaluation
            .artifacts
            .iter()
            .cloned()
            .collect::<Vec<_>>(),
        &data.evaluation.uses.iter().cloned().collect::<Vec<_>>(),
    )?;
    for event in data
        .bindings
        .event_events
        .iter()
        .filter(|e| e.context == invocation.context)
    {
        let site = data
            .bindings
            .occurrences
            .get(event.site)
            .ok_or_else(|| invalid("source call event site missing"))?;
        let artifact = data
            .evaluation
            .artifacts
            .get(site.source)
            .ok_or_else(|| invalid("source call artifact missing"))?;
        if artifact.input != invocation.input
            || !roots.contains(&artifact.id())
            || admission::ArtifactClass::of(&artifact.path)
                != Some(admission::ArtifactClass::PythonSource)
        {
            continue;
        }
        let mut candidates = data
            .output
            .attempts
            .iter()
            .filter(|a| a.event == event.id())
            .filter_map(|a| verified.composition(a.id()).map(|c| (a, c)));
        let first = candidates.next();
        let result = if candidates.next().is_some() {
            Err(obligation::ObligationKind::AmbiguousBinding)
        } else if let Some((attempt, admission)) = first {
            CheckedSourceBinding::derive(
                &data.evaluation,
                &data.flow,
                &data.bindings,
                &data.output,
                verified
                    .bound(attempt.id())
                    .ok_or_else(|| invalid("composition lacks private bound call"))?,
                admission,
                SourceCallRequest {
                    input: invocation.input,
                    context: invocation.context,
                    event: event.id(),
                },
                budget,
            )?
        } else {
            Err(obligation::ObligationKind::UnresolvedTarget)
        };
        match result {
            Ok(proof) => {
                let header = SourceCallHeader {
                    invocation: invocation.id(),
                    event: event.id(),
                    attempt: proof.attempt(),
                    owner: proof.caller(),
                    callee: proof.callee(),
                    declaration: proof.declaration(),
                    qualification: proof.qualification(),
                    status: proof.status(),
                    premises: ordered_digest(
                        "source-call-header-premises",
                        proof.premises().iter().map(Record::id),
                    ),
                };
                for (ordinal, premise) in proof.premises().iter().enumerate() {
                    records.members.insert(HeaderMember {
                        header: header.id(),
                        ordinal: ordinal
                            .try_into()
                            .map_err(|_| invalid("source call ordinal overflow"))?,
                        premise: premise.id(),
                    })?;
                }
                header_charge.grow(size_of::<(CheckedSourceBinding, SourceCallHeader)>() * 2)?;
                records.headers.insert(header.clone())?;
                headers.push((proof, header));
            }
            Err(reason) => {
                records.boundaries.insert(SourceCallBoundary {
                    invocation: invocation.id(),
                    event: event.id(),
                    reason,
                })?;
            }
        }
    }
    let mut calls = Vec::new();
    let mut resolved = charged::ChargedSet::default();
    let mut resolved_charge =
        charged::StateCharge::new(budget, "source_invocation_body_membership");
    for base in data
        .base
        .iter()
        .filter(|b| (b.input, b.context) == (invocation.input, invocation.context))
    {
        let definition = data
            .definitions
            .get(base.definition)
            .ok_or_else(|| invalid("source call completion definition absent"))?;
        let mut visit_body = |body: &super::body::CheckedSourceBody, row: &super::body_records::SourceBodyCompletion| -> Result<(), ModelError> {
                if data.bodies.get(row.id()) != Some(row) {
                    return Err(invalid(
                        "source invocation body differs from earlier immutable completion",
                    ));
                }
                for (header, record) in &headers {
                    if header.callee() != body.request().callee {
                        continue;
                    }
                    if !resolved.insert(&mut resolved_charge, record.id())? {
                        return Err(invalid("source invocation body mapping is ambiguous"));
                    }
                    match super::source_invocation::CheckedSourceInvocation::derive_with_values(
                        &data.evaluation,
                        header,
                        body,
                        &data.completed,
                        &[],
                        budget,
                        evaluations,
                    )? {
                        Err(reason) => {
                            records.invocation_boundaries.insert(InvocationBoundary {
                                header: record.id(),
                                reason,
                            })?;
                        }
                        Ok(proof) => {
                            let mut digest = KeySink::new("source-frame-arguments");
                            for argument in proof.arguments() {
                                argument.formal.encode(&mut digest);
                                argument.actual.encode(&mut digest);
                                argument.evaluation.encode(&mut digest);
                            }
                            let release = SourceFrameRelease {
                                header: record.id(),
                                body: row.id(),
                                arguments: digest.finish(),
                                release: proof.release(),
                            };
                            for (ordinal, argument) in proof.arguments().iter().enumerate() {
                                records.arguments.insert(SourceFrameArgument {
                                    release: release.id(),
                                    ordinal: ordinal as i64,
                                    formal: argument.formal,
                                    actual: argument.actual,
                                    evaluation: argument.evaluation,
                                })?;
                            }
                            let outcome = match proof.outcome() {
                                super::source_invocation::InvocationOutcome::Normal => {
                                    SourceCallOutcome::Normal
                                }
                                super::source_invocation::InvocationOutcome::Raised {
                                    site,
                                    exception,
                                } => SourceCallOutcome::Raised { site, exception },
                            };
                            let call = SourceInvocation {
                                invocation: invocation.id(),
                                release: release.id(),
                                qualification: proof.qualification(),
                                outcome: outcome.id(),
                                status: proof.status(),
                            };
                            header_charge.grow(
                                size_of::<(
                                    super::source_invocation::CheckedSourceInvocation,
                                    SourceInvocation,
                                )>() * 2,
                            )?;
                            records.invocations.insert(call.clone())?;
                            calls.push((proof, call));
                            records.releases.insert(release)?;
                            records.call_outcomes.insert(outcome)?;
                        }
                    }
                }
                Ok(())
            };
        if let Some(bodies) = bodies {
            bodies.visit_frame(base,data,&mut visit_body)?;
        } else {
            let expected = super::completion_production::complete_all_with_bodies(
                &data.completed, base, definition, profile, budget, &mut visit_body,
            )?;
            for body in expected.bodies.iter() {
                if data.bodies.get(body.id()) != Some(body) {
                    return Err(invalid("source invocation omitted earlier body membership"));
                }
            }
        }
    }
    for (_, header) in &headers {
        if !resolved.contains(&header.id()) {
            records.invocation_boundaries.insert(InvocationBoundary {
                header: header.id(),
                reason: obligation::ObligationKind::MissingEvidence,
            })?;
        }
    }
    on_complete(&headers, &calls)?;
    records.run.bound = records
        .headers
        .len()
        .try_into()
        .map_err(|_| invalid("source call header count overflow"))?;
    records.run.refused = records
        .boundaries
        .len()
        .try_into()
        .map_err(|_| invalid("source call boundary count overflow"))?;
    if !records.boundaries.is_empty() || !records.invocation_boundaries.is_empty() {
        records.outcome.status = analysis::AnalysisStatus::Partial;
        records.outcome.reason = Some(obligation::ObligationKind::UnresolvedTarget);
    }
    let mut charge = charged::StateCharge::new(budget, "produced-source-calls");
    charge.grow(size_of::<ProducedSourceFrame>() * 2)?;
    Ok((records, ProducedSourceCalls { frames: vec![ProducedSourceFrame {
        invocation: invocation.clone(), headers, calls, _charge: header_charge,
    }], charge }))
}
fn invariant_inputs() -> Vec<ValidationInput> {
    let mut inputs = SourceCallData::inputs();
    inputs.extend([
        ValidationInput::of::<publication::AnalysisInvocation>(&["id"]),
        ValidationInput::of::<analysis::base_completion::AnalysisInvocation>(&["id"]),
        ValidationInput::of::<analysis::AnalysisDefinition>(&["id"]),
        ValidationInput::of::<SourceCallRun>(&["id"]),
        ValidationInput::of::<SourceCallHeader>(&["id"]),
        ValidationInput::of::<HeaderMember>(&["id"]),
        ValidationInput::of::<SourceCallBoundary>(&["id"]),
        ValidationInput::of::<publication::AnalysisOutcome>(&["id"]),
        ValidationInput::of::<SourceInvocation>(&["id"]),
        ValidationInput::of::<SourceFrameRelease>(&["id"]),
        ValidationInput::of::<SourceFrameArgument>(&["id"]),
        ValidationInput::of::<SourceCallOutcome>(&["id"]),
        ValidationInput::of::<InvocationBoundary>(&["id"]),
    ]);
    inputs.sort_by_key(|i| (i.name(), i.prefix()));
    inputs.dedup_by_key(|i| (i.name(), i.prefix()));
    inputs
}
pub fn source_call_invariants() -> Vec<Invariant> {
    vec![Invariant {
        purpose: crate::domain::InvariantPurpose::DiagnosticReplay,
        revision: 1,
        name: "source_call_replay",
        inputs: invariant_inputs(),
        create: std::sync::Arc::new(|budget| Box::new(SourceCheck::new(budget))),
    }]
}
struct SourceCheck {
    data: SourceCallData,
    invocations: Rows<publication::AnalysisInvocation>,
    base: Rows<analysis::base_completion::AnalysisInvocation>,
    definitions: Rows<analysis::AnalysisDefinition>,
    runs: Rows<SourceCallRun>,
    headers: Rows<SourceCallHeader>,
    members: Rows<HeaderMember>,
    boundaries: Rows<SourceCallBoundary>,
    outcomes: Rows<publication::AnalysisOutcome>,
    calls: Rows<SourceInvocation>,
    releases: Rows<SourceFrameRelease>,
    arguments: Rows<SourceFrameArgument>,
    call_outcomes: Rows<SourceCallOutcome>,
    invocation_boundaries: Rows<InvocationBoundary>,
    budget: ResourceBudget,
}
impl SourceCheck {
    fn new(budget: &ResourceBudget) -> Self {
        Self {
            data: SourceCallData::new(budget),
            invocations: Rows::new(budget),
            base: Rows::new(budget),
            definitions: Rows::new(budget),
            runs: Rows::new(budget),
            headers: Rows::new(budget),
            members: Rows::new(budget),
            boundaries: Rows::new(budget),
            outcomes: Rows::new(budget),
            calls: Rows::new(budget),
            releases: Rows::new(budget),
            arguments: Rows::new(budget),
            call_outcomes: Rows::new(budget),
            invocation_boundaries: Rows::new(budget),
            budget: budget.clone(),
        }
    }
}
impl InvariantCheck for SourceCheck {
    fn visit(&mut self, name: &str, batch: &arrow_array::RecordBatch) -> Result<(), ModelError> {
        self.data.visit(name, batch)?;
        macro_rules! rows{($($field:ident:$ty:ty,)*)=>{$(if name==<$ty>::NAME{self.$field.decode(batch)?;})*};}
        rows! {invocations:publication::AnalysisInvocation,base:analysis::base_completion::AnalysisInvocation,definitions:analysis::AnalysisDefinition,runs:SourceCallRun,headers:SourceCallHeader,members:HeaderMember,boundaries:SourceCallBoundary,outcomes:publication::AnalysisOutcome,calls:SourceInvocation,releases:SourceFrameRelease,arguments:SourceFrameArgument,call_outcomes:SourceCallOutcome,invocation_boundaries:InvocationBoundary,}
        Ok(())
    }
    fn visit_input(
        &mut self,
        input: &ValidationInput,
        batch: &arrow_array::RecordBatch,
    ) -> Result<(), ModelError> {
        if stages::is_vocabulary(input.name()) {
            self.data.visit_input(input, batch)
        } else {
            self.visit(input.name(), batch)
        }
    }
    fn finish(self: Box<Self>) -> Result<(), ModelError> {
        let invalid = |s: &str| ModelError::Invalid(s.into());
        let mut frames = charged::ChargedSet::default();
        let mut charge = charged::StateCharge::new(&self.budget, "source_call_frames");
        for base in self.base.iter() {
            frames.insert(&mut charge, (base.input, base.context))?;
        }
        if self.invocations.len() != frames.len()
            || frames.iter().any(|frame| {
                self.invocations
                    .iter()
                    .filter(|i| (i.input, i.context) == *frame)
                    .count()
                    != 1
            })
        {
            return Err(invalid("source call omitted/duplicated completion frame"));
        }
        let mut runs = Rows::new(&self.budget);
        let mut headers = Rows::new(&self.budget);
        let mut members = Rows::new(&self.budget);
        let mut boundaries = Rows::new(&self.budget);
        let mut outcomes = Rows::new(&self.budget);
        let mut calls = Rows::new(&self.budget);
        let mut releases = Rows::new(&self.budget);
        let mut arguments = Rows::new(&self.budget);
        let mut call_outcomes = Rows::new(&self.budget);
        let mut invocation_boundaries = Rows::new(&self.budget);
        for invocation in self.invocations.iter() {
            let mut parents = Rows::new(&self.budget);
            for base in self
                .base
                .iter()
                .filter(|b| (b.input, b.context) == (invocation.input, invocation.context))
            {
                parents.insert(publication::InvocationSource::BaseCompletion {
                    invocation: base.id(),
                })?;
            }
            let mut key = KeySink::new("analysis-invocation-inputs");
            for row in parents.iter() {
                row.id().encode(&mut key);
            }
            if key.finish() != invocation.inputs {
                return Err(invalid("source call completion parent inventory differs"));
            }
            let run = self
                .runs
                .iter()
                .find(|r| r.invocation == invocation.id())
                .ok_or_else(|| invalid("source call run absent"))?;
            let definition = self
                .definitions
                .get(invocation.definition)
                .ok_or_else(|| invalid("source call definition absent"))?;
            let expected = prepare_all(
                &self.data,
                invocation,
                definition,
                if run.requested {
                    stages::Profile::Behavioral
                } else {
                    stages::Profile::Catalog
                },
                &self.budget,
            )?;
            runs.insert(expected.run)?;
            outcomes.insert(expected.outcome)?;
            for r in expected.headers.iter() {
                headers.insert(r.clone())?;
            }
            for r in expected.members.iter() {
                members.insert(r.clone())?;
            }
            for r in expected.boundaries.iter() {
                boundaries.insert(r.clone())?;
            }
            for r in expected.invocations.iter() {
                calls.insert(r.clone())?;
            }
            for r in expected.releases.iter() {
                releases.insert(r.clone())?;
            }
            for r in expected.arguments.iter() {
                arguments.insert(r.clone())?;
            }
            for r in expected.call_outcomes.iter() {
                call_outcomes.insert(r.clone())?;
            }
            for r in expected.invocation_boundaries.iter() {
                invocation_boundaries.insert(r.clone())?;
            }
        }
        if !self.runs.same(&runs)
            || !self.headers.same(&headers)
            || !self.members.same(&members)
            || !self.boundaries.same(&boundaries)
            || !self.outcomes.same(&outcomes)
            || !self.calls.same(&calls)
            || !self.releases.same(&releases)
            || !self.arguments.same(&arguments)
            || !self.call_outcomes.same(&call_outcomes)
            || !self.invocation_boundaries.same(&invocation_boundaries)
        {
            return Err(invalid(
                "source call inventory differs from independently replayed binding universe",
            ));
        }
        Ok(())
    }
}
pub(crate) fn profile_checks() -> Vec<PublicationInvariant> {
    vec![PublicationInvariant {
        revision: 1,
        name: "source_call_profile",
        inputs: vec![
            ValidationInput::of::<SourceCallRun>(&["id"]),
            ValidationInput::of::<publication::AnalysisInvocation>(&["id"]),
        ],
        create: std::sync::Arc::new(|b| {
            Box::new(ProfileCheck {
                runs: Rows::new(b),
                invocations: Rows::new(b),
            })
        }),
    }]
}
struct ProfileCheck {
    runs: Rows<SourceCallRun>,
    invocations: Rows<publication::AnalysisInvocation>,
}
impl PublicationCheck for ProfileCheck {
    fn visit(&mut self, name: &str, batch: &arrow_array::RecordBatch) -> Result<(), ModelError> {
        if name == SourceCallRun::NAME {
            self.runs.decode(batch)?;
        } else if name == publication::AnalysisInvocation::NAME {
            self.invocations.decode(batch)?;
        }
        Ok(())
    }
    fn finish(
        self: Box<Self>,
        _sources: &[crate::domain::analysis::sources::SourceSnapshot],
        profile: stages::Profile,
    ) -> Result<(), ModelError> {
        if self.runs.len() != self.invocations.len()
            || self.runs.iter().any(|r| {
                self.invocations.get(r.invocation).is_none()
                    || r.requested != (profile == stages::Profile::Behavioral)
            })
        {
            return Err(ModelError::Invalid(
                "source call changes actual requested profile".into(),
            ));
        }
        Ok(())
    }
}
pub fn relations() -> Vec<Relation> {
    vec![
        Relation::of::<SourceCallHeader>(),
        Relation::of::<HeaderMember>(),
        Relation::of::<SourceCallBoundary>(),
        Relation::of::<SourceCallRun>(),
        Relation::of::<SourceInvocation>(),
        Relation::of::<SourceFrameRelease>(),
        Relation::of::<SourceFrameArgument>(),
        Relation::of::<SourceCallOutcome>(),
        Relation::of::<InvocationBoundary>(),
    ]
}

pub fn stage(
    profile: stages::Profile,
    definition: &analysis::AnalysisDefinition,
    model: &ValidatedModel,
    order: &stages::PublicationOrder,
) -> Result<stages::Stage, ModelError> {
    use stages::*;
    if *definition != super::configuration::source_calls().1 {
        return Err(ModelError::Invalid(
            "source call stage definition is unbound".into(),
        ));
    }
    let mut outputs = publication::publication_relations();
    outputs.extend(relations());
    outputs.sort_by_key(Relation::name);
    outputs.dedup_by_key(|r| r.name());
    let mut initial = if profile == Profile::Behavioral {
        invariant_inputs()
    } else {
        vec![
            ValidationInput::of::<analysis::base_completion::AnalysisInvocation>(&["id"]),
            ValidationInput::of::<analysis::AnalysisDefinition>(&["id"]),
        ]
    };
    for relation in &outputs {
        for id in relation.publication_refs() {
            let check = model.publication_check(id)?;
            initial.extend(check.inputs.iter().cloned());
        }
    }
    let owned = outputs
        .iter()
        .map(RelationUse::of_relation)
        .collect::<Vec<_>>();
    // A publication check can name the records being written; only its predecessors are roots.
    initial.retain(|input| {
        is_vocabulary(input.name()) || !owned.iter().any(|row| row.name() == input.name())
    });
    let inputs = dependency_closure::DependencyClosure::stage_grants(
        model,
        initial,
        &owned,
        PublicationBoundary::Facts,
        dependency_closure::LowerLayerPolicy::OmitInferredOrdinaryFacts,
        order,
    )?;
    let mut key = KeySink::new("source-call-definition");
    definition.id().encode(&mut key);
    Ok(Stage {
        name: "prepare_source_calls",
        inputs,
        outputs: outputs.iter().map(RelationUse::of_relation).collect(),
        contributes: vec![],
        coverage: vec![],
        profiles: vec![profile],
        effect: Effect::Pure,
        code: ContentHash::of(include_bytes!("source_call_records.rs")),
        configuration: key.finish(),
    })
}

pub(crate) fn source_call_invariants_refs() -> Vec<&'static str> {
    vec!["source_call_replay"]
}
pub(crate) fn profile_checks_refs() -> Vec<&'static str> {
    vec!["source_call_profile"]
}

pub(crate) fn header_fidelity_refs() -> Vec<&'static str> {vec!["source_call_replay", "execution_header_fidelity"]}
pub(crate) fn release_fidelity_refs() -> Vec<&'static str> {vec!["execution_release_fidelity"]}
pub(crate) fn invocation_fidelity_refs() -> Vec<&'static str> {vec!["source_call_replay", "execution_invocation_fidelity"]}
