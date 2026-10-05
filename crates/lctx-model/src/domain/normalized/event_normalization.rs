//! Assemble the entire event and its attributed members before the single policy evaluator.
use super::{Rows, entities::*, events::*, links::LinkReason, policy_revision};
use crate::domain::{
    assertion::*,
    attribution::*,
    calls::*,
    charged::{ChargedMap, ChargedSet, StateCharge},
    flow::*,
    resources::ResourceBudget,
    source::Occurrence,
    *,
};
trait Source<R: Record> {
    fn rows(&self) -> &Rows<R>;
}
macro_rules! inputs {
    ($($field:ident: $ty:ty,)*) => {
        pub struct EventData { $(pub $field: Rows<$ty>,)* }
        $(impl Source<$ty> for EventData {fn rows(&self)->&Rows<$ty> {&self.$field}})*
        impl EventData {
            pub fn new(budget: &ResourceBudget) -> Self { Self { $($field: Rows::new(budget),)* } }
            pub fn visit(&mut self, relation: &str, batch: &arrow_array::RecordBatch) -> Result<bool, ModelError> {
                $(if relation == <$ty>::NAME { self.$field.decode(batch)?; return Ok(true); })* Ok(false)
            }
            pub fn validation_inputs() -> Vec<ValidationInput> { super::facts_inputs(vec![$(ValidationInput::of::<$ty>(&["id"]),)*]) }
            pub fn stage_inputs() -> Vec<stages::RelationUse> { vec![$(stages::RelationUse::stored::<$ty>()),*] }
        }
    }
}
crate::normalized_event_inputs!(inputs);
macro_rules! outputs {
    ($($field:ident: $ty:ty,)*) => {
        pub struct EventOutput { $(pub $field: Rows<$ty>,)* }
        impl EventOutput {
            pub fn new(budget: &ResourceBudget) -> Self { Self { $($field: Rows::new(budget),)* } }
            pub fn visit(&mut self, relation: &str, batch: &arrow_array::RecordBatch) -> Result<bool, ModelError> {
                $(if relation == <$ty>::NAME { self.$field.decode(batch)?; return Ok(true); })* Ok(false)
            }
            pub fn matches(&self, expected: &Self) -> Result<(), ModelError> {
                $(if !self.$field.same(&expected.$field) { return Err(invalid(format!("normalized event closure differs: {}", <$ty>::NAME))); })* Ok(())
            }
            pub fn validation_inputs() -> Vec<ValidationInput> { vec![$(ValidationInput::of::<$ty>(&["id"]),)*] }
        }
    }
}
crate::normalized_event_outputs!(outputs);
fn receiver_proofs(
    data: &EventData,
    budget: &ResourceBudget,
) -> Result<super::receiver::VerifiedReceivers, ModelError> {
    let mut inputs = super::receiver::ReceiverData::new(budget);
    let mut outputs = super::receiver::ReceiverOutput::new(budget);
    macro_rules! input {($($field:ident: $ty:ty,)*)=>{$(for row in <EventData as Source<$ty>>::rows(data).iter(){inputs.$field.insert(row.clone())?;})*};}
    macro_rules! output {($($field:ident: $ty:ty,)*)=>{$(for row in <EventData as Source<$ty>>::rows(data).iter(){outputs.$field.insert(row.clone())?;})*};}
    crate::normalized_receiver_inputs!(input);
    crate::normalized_receiver_outputs!(output);
    super::receiver::verify(&inputs, &outputs, budget)
}
fn invalid(message: impl Into<String>) -> ModelError {
    ModelError::Invalid(message.into())
}
fn need<R: Record>(rows: &Rows<R>, id: Id<R>) -> Result<&R, ModelError> {
    rows.get(id)
        .ok_or_else(|| invalid(format!("normalized event requires {}", R::NAME)))
}
fn qualification(
    data: &EventData,
    id: Id<AssertionQualification>,
) -> Result<&AssertionQualification, ModelError> {
    need(&data.qualifications, id)
}
fn exact(q: &AssertionQualification) -> bool {
    q.modality == Modality::Definite && q.approximation == Approximation::Exact
}
type EventKey = (Id<Occurrence>, Id<CallOrigin>, Id<AnalysisContext>);
struct Index<'a> {
    universe: ChargedSet<EventKey>,
    owners: ChargedMap<Id<Occurrence>, &'a OccurrenceOwnership>,
    symbols: ChargedMap<Id<ProviderSymbol>, &'a SymbolEntityResolution>,
    sites: ChargedMap<EventKey, Vec<&'a ProviderCallSite>>,
    site_supports: ChargedMap<Id<ProviderCallSite>, Vec<&'a ProviderCallSiteSupport>>,
    targets: ChargedMap<EventKey, Vec<&'a CallTarget>>,
    resolutions: ChargedMap<EventKey, Vec<&'a CallResolution>>,
    members: ChargedMap<Id<CallResolution>, Vec<&'a CallTarget>>,
    supports: ChargedMap<Id<CallTarget>, Vec<&'a CallTargetSupport>>,
    resolution_supports: ChargedMap<Id<CallResolution>, Vec<&'a CallResolutionSupport>>,
    _charge: StateCharge,
}
impl<'a> Index<'a> {
    fn new(data: &'a EventData, budget: &ResourceBudget) -> Result<Self, ModelError> {
        let mut index = Self {
            universe: Default::default(),
            owners: Default::default(),
            symbols: Default::default(),
            sites: Default::default(),
            site_supports: Default::default(),
            targets: Default::default(),
            resolutions: Default::default(),
            members: Default::default(),
            supports: Default::default(),
            resolution_supports: Default::default(),
            _charge: StateCharge::new(budget, "normalized-event-index"),
        };
        let charge = &mut index._charge;
        for row in data.owners.iter() {
            index.owners.insert(charge, row.occurrence, row)?;
        }
        for row in data.symbol_resolutions.iter() {
            index.symbols.insert(charge, row.symbol, row)?;
        }
        for row in data.call_sites.iter() {
            let key = (
                row.site,
                row.origin,
                qualification(data, row.qualification)?.context,
            );
            index.universe.insert(charge, key)?;
            index.sites.update(charge, key, |v| v.push(row))?;
        }
        for row in data.targets.iter() {
            let key = (
                row.site,
                row.origin,
                qualification(data, row.qualification)?.context,
            );
            index.universe.insert(charge, key)?;
            index.targets.update(charge, key, |v| v.push(row))?;
        }
        for row in data.resolutions.iter() {
            let key = (
                row.site,
                row.origin,
                qualification(data, row.qualification)?.context,
            );
            index.universe.insert(charge, key)?;
            index.resolutions.update(charge, key, |v| v.push(row))?;
        }
        for row in data.members.iter() {
            need(&data.resolutions, row.resolution)?;
            let target = need(&data.targets, row.target)?;
            index
                .members
                .update(charge, row.resolution, |v| v.push(target))?;
        }
        for row in data.target_supports.iter() {
            need(&data.targets, row.assertion)?;
            index
                .supports
                .update(charge, row.assertion, |v| v.push(row))?;
        }
        for row in data.site_supports.iter() {
            need(&data.call_sites, row.assertion)?;
            index
                .site_supports
                .update(charge, row.assertion, |v| v.push(row))?;
        }
        for row in data.resolution_supports.iter() {
            need(&data.resolutions, row.assertion)?;
            index
                .resolution_supports
                .update(charge, row.assertion, |v| v.push(row))?;
        }
        Ok(index)
    }
}
/// Only complete normalized input reconstruction creates this token. Neither a stored assessment
/// nor a filtered set of admitted rows has a public conversion to it.
pub(super) struct CompleteEvent {
    event: Id<NormalizedCallEvent>,
    assessment: Id<EventAssessment>,
    members: ContentHash,
}
impl CompleteEvent {
    pub fn assessment(&self) -> Id<EventAssessment> {
        self.assessment
    }
    pub fn members(&self) -> ContentHash {
        self.members
    }
}
impl HeapSize for CompleteEvent {
    fn heap_bytes(&self) -> usize {
        0
    }
}
pub(super) struct VerifiedEvents {
    complete: ChargedMap<Id<NormalizedCallEvent>, CompleteEvent>,
    dispatch: ChargedMap<Id<NormalizedCallAlternative>, super::dispatch::ApplicableDispatch>,
    _charge: StateCharge,
}
impl VerifiedEvents {
    pub(super) fn dispatch(
        &self,
        alternative: Id<NormalizedCallAlternative>,
    ) -> Option<&super::dispatch::ApplicableDispatch> {
        self.dispatch.get(&alternative)
    }
    pub fn get(&self, event: Id<NormalizedCallEvent>) -> Option<&CompleteEvent> {
        self.complete.get(&event)
    }
}
/// Internal reconstruction assumes checked N1/N2 premises. Only full binding replay exposes an
/// admission token outside this module family. Public validation returns no authority token.
pub(super) fn verify(
    data: &EventData,
    stored: &EventOutput,
    budget: &ResourceBudget,
) -> Result<VerifiedEvents, ModelError> {
    let (expected, tokens) = evaluate(data, budget)?;
    stored.matches(&expected)?;
    Ok(tokens)
}
pub fn validate(
    data: &EventData,
    stored: &EventOutput,
    budget: &ResourceBudget,
) -> Result<(), ModelError> {
    stored.matches(&normalize(data, budget)?)
}
pub fn normalize(data: &EventData, budget: &ResourceBudget) -> Result<EventOutput, ModelError> {
    Ok(evaluate(data, budget)?.0)
}

type Correspondence = (
    Option<Id<SymbolEntityResolution>>,
    Option<Id<EntityRef>>,
    ResolutionStatus,
    LinkReason,
);
fn correspondence(
    data: &EventData,
    index: &Index<'_>,
    destination: &CallDestination,
) -> Result<Correspondence, ModelError> {
    let symbol = match destination {
        CallDestination::Resolved { symbol } | CallDestination::Overrides { symbol } => {
            Some(*symbol)
        }
        CallDestination::Callable { callable } => {
            match need(&data.provider_callables, *callable)? {
                ProviderCallable::Symbol { symbol }
                | ProviderCallable::DecoratorApplication { function: symbol }
                | ProviderCallable::ClassBody { class: symbol } => Some(*symbol),
                ProviderCallable::ModuleBody { module, .. } => {
                    let entity = match need(&data.provider_modules, *module)? {
                        ProviderModule::Acquired { module } => {
                            Some(EntityRef::Module { module: *module }.id())
                        }
                        _ => None,
                    };
                    if let Some(entity) = entity {
                        need(&data.refs, entity)?;
                    }
                    return Ok((
                        None,
                        entity,
                        if entity.is_some() {
                            ResolutionStatus::Resolved
                        } else {
                            ResolutionStatus::Unresolved
                        },
                        if entity.is_some() {
                            LinkReason::ExplicitIdentity
                        } else {
                            LinkReason::OutsideCapturedScope
                        },
                    ));
                }
            }
        }
        _ => None,
    };
    if let Some(symbol) = symbol {
        let resolution = index
            .symbols
            .get(&symbol)
            .ok_or_else(|| invalid("call target has no total symbol correspondence"))?;
        let reason = match resolution.status {
            ResolutionStatus::Resolved => LinkReason::ExplicitIdentity,
            ResolutionStatus::Ambiguous => LinkReason::ConflictingCandidates,
            ResolutionStatus::Unresolved => LinkReason::MissingCorrespondence,
        };
        Ok((
            Some(resolution.id()),
            resolution.entity,
            resolution.status,
            reason,
        ))
    } else {
        Ok((
            None,
            None,
            ResolutionStatus::Unresolved,
            LinkReason::MissingResolution,
        ))
    }
}
#[allow(
    clippy::too_many_arguments,
    reason = "One call alternative threads the event, target, resolution, output and the alternative rows"
)]
fn alternative(
    data: &EventData,
    index: &Index<'_>,
    output: &mut EventOutput,
    event: Id<NormalizedCallEvent>,
    target: &CallTarget,
    resolution: Option<Id<CallResolution>>,
    alternatives: &mut Rows<NormalizedCallAlternative>,
    budget: &ResourceBudget,
) -> Result<NormalizedCallAlternative, ModelError> {
    let (correspondence, entity, status, reason) =
        correspondence(data, index, need(&data.destinations, target.destination)?)?;
    if let Some(id) = correspondence {
        let mapping = need(&data.symbol_resolutions, id)?;
        if mapping.context != qualification(data, target.qualification)?.context
            || need(&data.symbols, mapping.symbol)?.context != mapping.context
        {
            return Err(invalid("call correspondence crosses analysis context"));
        }
    }
    let row = NormalizedCallAlternative {
        event,
        source: output
            .alternative_sources
            .insert(CallAlternativeSource::Native {
                target: target.id(),
            })?,
        resolution,
        correspondence,
        entity,
        status,
        reason,
    };
    output.alternatives.insert(row.clone())?;
    let supports = index
        .supports
        .get(&target.id())
        .filter(|s| !s.is_empty())
        .ok_or_else(|| invalid("call target has no retained support"))?;
    for support in supports {
        output
            .alternative_evidence
            .insert(CallAlternativeEvidence {
                alternative: row.id(),
                support: support.id(),
            })?;
    }
    let dispatch = super::dispatch::assess(data, event, target, output, budget)?;
    for member in dispatch.members.iter().filter(|m| !m.named) {
        let derived = NormalizedCallAlternative {
            event,
            source: output
                .alternative_sources
                .insert(CallAlternativeSource::DerivedDispatch {
                    target: target.id(),
                    member: member.id(),
                })?,
            resolution,
            correspondence: Some(member.correspondence),
            entity: Some(member.entity),
            status: ResolutionStatus::Resolved,
            reason: LinkReason::ExplicitIdentity,
        };
        output.alternatives.insert(derived.clone())?;
        alternatives.insert(derived.clone())?;
        for support in supports {
            output
                .alternative_evidence
                .insert(CallAlternativeEvidence {
                    alternative: derived.id(),
                    support: support.id(),
                })?;
        }
    }
    Ok(row)
}
fn evaluate(
    data: &EventData,
    budget: &ResourceBudget,
) -> Result<(EventOutput, VerifiedEvents), ModelError> {
    let receiver_proofs = receiver_proofs(data, budget)?;
    let index = Index::new(data, budget)?;
    let mut output = EventOutput::new(budget);
    let mut tokens = VerifiedEvents {
        complete: Default::default(),
        dispatch: Default::default(),
        _charge: StateCharge::new(budget, "complete-event-tokens"),
    };
    for &(site, origin, ctx) in index.universe.iter() {
        let key = (site, origin, ctx);
        let owner = index
            .owners
            .get(&site)
            .ok_or_else(|| invalid("event has no materialized occurrence owner"))?;
        let event = output.events.insert(NormalizedCallEvent {
            site,
            origin,
            context: ctx,
            owner: owner.id(),
        })?;
        let mut held = StateCharge::new(budget, "complete-event-members");
        let mut digest = KeySink::new("complete-normalized-event");
        let mut alternatives = Rows::new(budget);
        let mut reported: ChargedSet<Id<CallTarget>> = Default::default();
        let mut phases: ChargedMap<CallPhase, std::collections::BTreeSet<Id<EntityRef>>> =
            Default::default();
        let mut groups: ChargedSet<super::events::PhaseGroup> = Default::default();
        let (mut direct_count, mut complete, mut certain, mut receivers) =
            (0usize, true, true, true);
        let (mut unresolved, mut dispatch, mut orphan) = (false, false, false);
        let mut declared_runs: ChargedSet<(Id<ProviderRun>, Id<source::CoverageScope>)> =
            Default::default();
        let mut resolved_runs: ChargedSet<(Id<ProviderRun>, Id<source::CoverageScope>)> =
            Default::default();
        for source in index.sites.get(&key).into_iter().flatten() {
            let id = output.sources.insert(CallEventSource {
                event,
                observation: source.id(),
            })?;
            id.encode(&mut digest);
            let supports = index
                .site_supports
                .get(&source.id())
                .filter(|s| !s.is_empty())
                .ok_or_else(|| invalid("event source has no retained support"))?;
            for support in supports {
                declared_runs.insert(
                    &mut held,
                    (
                        support.run,
                        qualification(data, source.qualification)?.scope,
                    ),
                )?;
                let id = output.source_evidence.insert(CallEventSourceEvidence {
                    source: id,
                    support: support.id(),
                })?;
                id.encode(&mut digest);
            }
        }
        for resolution in index.resolutions.get(&key).into_iter().flatten() {
            let q = qualification(data, resolution.qualification)?;
            let direct = matches!(
                need(&data.channels, resolution.channel)?,
                CallChannel::Direct
            );
            if direct {
                direct_count += 1;
                complete &= resolution.complete;
                certain &= exact(q);
                groups.insert(&mut held, super::events::PhaseGroup::of(resolution.phase))?;
            }
            let member = output.resolutions.insert(CallEventResolution {
                event,
                resolution: resolution.id(),
            })?;
            member.encode(&mut digest);
            let supports = index
                .resolution_supports
                .get(&resolution.id())
                .filter(|s| !s.is_empty())
                .ok_or_else(|| invalid("event resolution has no retained support"))?;
            for support in supports {
                if direct {
                    resolved_runs.insert(&mut held, (support.run, q.scope))?;
                }
                let id = output
                    .resolution_evidence
                    .insert(CallEventResolutionEvidence {
                        resolution: member,
                        support: support.id(),
                    })?;
                id.encode(&mut digest);
            }
            let candidates = index
                .members
                .get(&resolution.id())
                .map(Vec::as_slice)
                .unwrap_or(&[]);
            // Reconstruct the declared set before inspecting any policy predicate. This also
            // protects direct pure callers, independently of the facts checkpoint's validator.
            let mut copy = Vec::new();
            for target in candidates {
                held.admit(*target)?;
                copy.push((*target).clone());
                let tq = qualification(data, target.qualification)?;
                if tq.context != ctx || tq.scope != q.scope {
                    return Err(invalid("call event crosses qualification context or scope"));
                }
            }
            let (reconstructed, _) = CallResolution::new(
                q,
                site,
                origin,
                resolution.channel,
                resolution.phase,
                resolution.complete,
                &copy,
            )?;
            if reconstructed != **resolution {
                return Err(invalid("event input omits a declared alternative"));
            }
            if direct && candidates.is_empty() {
                complete = false;
            }
            for target in candidates {
                reported.insert(&mut held, target.id())?;
                let alternative = alternative(
                    data,
                    &index,
                    &mut output,
                    event,
                    target,
                    Some(resolution.id()),
                    &mut alternatives,
                    budget,
                )?;
                if direct {
                    certain &= exact(qualification(data, target.qualification)?);
                    let derived_receiver = receiver_proofs.get(target.id());
                    if let Some(proof) = derived_receiver {
                        proof.assessment().encode(&mut digest);
                    }
                    receivers &= derived_receiver.is_some()
                        || !matches!(
                            need(&data.receivers, target.receiver)?,
                            Receiver::Unknown { .. }
                        );
                    match need(&data.destinations, target.destination)? {
                        CallDestination::Resolved { .. } => {
                            if let Some(entity) = alternative.entity {
                                phases.update(&mut held, target.phase, |set| {
                                    set.insert(entity);
                                })?;
                            } else {
                                unresolved = true;
                            }
                        }
                        CallDestination::Overrides { .. } => dispatch = true,
                        _ => unresolved = true,
                    }
                }
                alternatives.insert(alternative)?;
            }
        }
        // Raw orphan targets remain inspectable and associated, but cannot make a closed event.
        for target in index
            .targets
            .get(&key)
            .into_iter()
            .flatten()
            .filter(|target| !reported.contains(&target.id()))
        {
            if matches!(need(&data.channels, target.channel)?, CallChannel::Direct) {
                orphan = true;
                complete = false;
            }
            let native = alternative(
                data,
                &index,
                &mut output,
                event,
                target,
                None,
                &mut alternatives,
                budget,
            )?;
            alternatives.insert(native)?;
        }
        for alternative in alternatives.iter() {
            let source = need(&output.alternative_sources, alternative.source)?;
            let member = match source {
                CallAlternativeSource::DerivedDispatch { member, .. } => {
                    output.dispatch_members.get(*member)
                }
                CallAlternativeSource::Native { target } => {
                    output.dispatch_members.iter().find(|m| {
                        m.named
                            && output
                                .dispatch_assessments
                                .get(m.assessment)
                                .is_some_and(|a| a.event == event && a.target == *target)
                    })
                }
            };
            if let Some(member) = member {
                let assessment = need(&output.dispatch_assessments, member.assessment)?;
                tokens.dispatch.insert(
                    &mut tokens._charge,
                    alternative.id(),
                    super::dispatch::ApplicableDispatch::from_member(member, assessment, ctx),
                )?;
            }
            alternative.id().encode(&mut digest);
            for support in index
                .supports
                .get(&need(&output.alternative_sources, alternative.source)?.target())
                .into_iter()
                .flatten()
            {
                support.id().encode(&mut digest);
            }
        }
        for assessment in output
            .dispatch_assessments
            .iter()
            .filter(|a| a.event == event)
        {
            assessment.id().encode(&mut digest);
            assessment.members.encode(&mut digest);
        }
        complete &= direct_count > 0 && declared_runs.iter().all(|run| resolved_runs.contains(run));
        let disagreement = phases.values().any(|values| values.len() != 1);
        let unique = !phases.is_empty()
            && groups.len() == 1
            && !disagreement
            && !unresolved
            && !dispatch
            && !orphan;
        let reason = if direct_count == 0 {
            EventReason::NoDirectResolution
        } else if orphan {
            EventReason::UnreportedTarget
        } else if !complete {
            EventReason::OpenResolution
        } else if unresolved {
            EventReason::MissingCorrespondence
        } else if dispatch {
            EventReason::Dispatched
        } else if disagreement {
            EventReason::ConflictingDestinations
        } else if groups.len() != 1 {
            EventReason::MultiplePhaseGroups
        } else if !certain {
            EventReason::QualifiedUncertainty
        } else if !receivers {
            EventReason::UnknownReceiver
        } else {
            EventReason::CompleteUnique
        };
        let assessment = EventAssessment {
            event,
            policy: policy_revision(),
            members: digest.finish(),
            complete,
            unique,
            exact: certain,
            known_receivers: receivers,
            group: if groups.len() == 1 {
                groups.first().copied()
            } else {
                None
            },
            unresolved,
            dispatch,
            disagreement,
            reason,
        };
        let assessment_id = output.assessments.insert(assessment.clone())?;
        for (phase, entities) in phases.iter() {
            for entity in entities {
                output.phase_targets.insert(EventPhaseTarget {
                    assessment: assessment_id,
                    phase: *phase,
                    entity: *entity,
                })?;
            }
        }
        if complete && unique && certain && receivers {
            tokens.complete.insert(
                &mut tokens._charge,
                event,
                CompleteEvent {
                    event,
                    assessment: assessment_id,
                    members: assessment.members,
                },
            )?;
        }
        for policy in super::events::CallPolicy::ALL {
            let mut admitted: ChargedSet<Id<NormalizedCallAlternative>> = Default::default();
            for alternative in alternatives.iter() {
                if admits(
                    policy,
                    data,
                    &index,
                    alternative,
                    tokens.get(event),
                    &output,
                )? {
                    admitted.insert(&mut held, alternative.id())?;
                }
            }
            let mut digest = KeySink::new("call-policy-members");
            for id in admitted.iter() {
                id.encode(&mut digest);
            }
            let reason = if !admitted.is_empty() {
                PolicyReason::Admitted
            } else if alternatives.is_empty() {
                PolicyReason::NoAlternative
            } else if policy == super::events::CallPolicy::Summary && !complete {
                PolicyReason::IncompleteEvent
            } else if policy == super::events::CallPolicy::Summary && !unique {
                PolicyReason::NonUniqueEvent
            } else if policy == super::events::CallPolicy::Summary && (!certain || !receivers) {
                PolicyReason::UncertainEvent
            } else {
                PolicyReason::OutsidePolicy
            };
            let policy_assessment = output.policy_assessments.insert(CallPolicyAssessment {
                event,
                policy,
                event_assessment: assessment_id,
                admitted: admitted.len() as i64,
                members: digest.finish(),
                reason,
            })?;
            for alternative in admitted.iter() {
                output.admissions.insert(CallPolicyAdmission {
                    assessment: policy_assessment,
                    alternative: *alternative,
                })?;
            }
        }
    }
    flow_links(data, &mut output, budget)?;
    Ok((output, tokens))
}
fn admits(
    policy: super::events::CallPolicy,
    data: &EventData,
    index: &Index<'_>,
    alternative: &NormalizedCallAlternative,
    complete: Option<&CompleteEvent>,
    output: &EventOutput,
) -> Result<bool, ModelError> {
    use super::events::CallPolicy as Policy;
    let target = need(
        &data.targets,
        need(&output.alternative_sources, alternative.source)?.target(),
    )?;
    let q = qualification(data, target.qualification)?;
    let destination = need(&data.destinations, target.destination)?;
    let direct = matches!(need(&data.channels, target.channel)?, CallChannel::Direct);
    let ordinary = matches!(q.modality, Modality::Definite | Modality::Candidate);
    let supports = index
        .supports
        .get(&target.id())
        .ok_or_else(|| invalid("policy target has no evidence"))?;
    let origin = |origin| supports.iter().any(|support| support.origin == origin);
    let function = destination
        .symbol()
        .and_then(|id| data.symbols.get(id))
        .is_some_and(|s| matches!(s.kind, SymbolKind::Function | SymbolKind::Method));
    let non_dispatch = !matches!(destination, CallDestination::Overrides { .. });
    Ok(match policy {
        Policy::Invocation => {
            direct
                && ordinary
                && origin(Origin::AnalyzerAssertion)
                && matches!(
                    target.phase,
                    CallPhase::Call | CallPhase::PropertyGet | CallPhase::PropertySet
                )
        }
        Policy::Dataflow => {
            direct
                && non_dispatch
                && ordinary
                && function
                && matches!(target.phase, CallPhase::Call | CallPhase::Init)
        }
        Policy::Summary => {
            complete.is_some_and(|token| token.event == alternative.event)
                && alternative.resolution.is_some()
                && direct
                && non_dispatch
                && function
                && matches!(
                    target.phase,
                    CallPhase::Call | CallPhase::New | CallPhase::Init
                )
        }
        Policy::Usage => {
            ordinary && (origin(Origin::AnalyzerAssertion) || origin(Origin::SourceObservation))
        }
        Policy::Association => true,
    })
}
fn flow_links(
    data: &EventData,
    output: &mut EventOutput,
    budget: &ResourceBudget,
) -> Result<(), ModelError> {
    let mut charge = StateCharge::new(budget, "flow-event-link-index");
    type EventIndex =
        ChargedMap<(Id<Occurrence>, Id<AnalysisContext>), Vec<Id<NormalizedCallEvent>>>;
    let mut events: EventIndex = Default::default();
    for event in output
        .events
        .iter()
        .filter(|e| e.origin == CallOrigin::explicit())
    {
        events.update(&mut charge, (event.site, event.context), |v| {
            v.push(event.id())
        })?;
    }
    let mut steps: ChargedMap<Id<FlowCallPath>, Vec<&FlowCallStep>> = Default::default();
    for row in data.steps.iter() {
        steps.update(&mut charge, row.path, |v| v.push(row))?;
    }
    for path in data.paths.iter() {
        let context = qualification(data, path.qualification)?.context;
        for step in steps.get(&path.path).into_iter().flatten() {
            let events = events
                .get(&(step.call, context))
                .map(Vec::as_slice)
                .unwrap_or(&[]);
            let (event, status, reason) = match events {
                [event] => (
                    Some(*event),
                    ResolutionStatus::Resolved,
                    FlowEventReason::ExactEvent,
                ),
                [] => (
                    None,
                    ResolutionStatus::Unresolved,
                    FlowEventReason::NoReportedEvent,
                ),
                _ => (
                    None,
                    ResolutionStatus::Ambiguous,
                    FlowEventReason::AmbiguousEvent,
                ),
            };
            output.flow_links.insert(FlowCallEventLink {
                observation: path.id(),
                step: step.id(),
                event,
                status,
                reason,
            })?;
        }
    }
    Ok(())
}
pub fn invariants() -> Vec<Invariant> {
    let mut inputs = EventData::validation_inputs();
    inputs.extend(EventOutput::validation_inputs());
    vec![Invariant {
        revision: 1,
        name: "normalized_event_closure",
        inputs,
        create: std::sync::Arc::new(|budget| {
            Box::new(EventCheck {
                data: EventData::new(budget),
                output: EventOutput::new(budget),
                budget: budget.clone(),
            })
        }),
    }]
}
struct EventCheck {
    data: EventData,
    output: EventOutput,
    budget: ResourceBudget,
}
impl InvariantCheck for EventCheck {
    fn visit(
        &mut self,
        relation: &str,
        batch: &arrow_array::RecordBatch,
    ) -> Result<(), ModelError> {
        if !self.data.visit(relation, batch)? && !self.output.visit(relation, batch)? {
            return Err(invalid("undeclared event validation input"));
        }
        Ok(())
    }
    fn finish(self: Box<Self>) -> Result<(), ModelError> {
        verify(&self.data, &self.output, &self.budget).map(|_| ())
    }
}
pub fn stage(profile: stages::Profile) -> stages::Stage {
    let mut inputs = super::callable_normalization::stage(profile).inputs;
    macro_rules! prior { ($($field:ident: $ty:ty,)*) => { $(inputs.push(stages::RelationUse::stored::<$ty>());)* }; }
    crate::normalized_callable_outputs!(prior);
    inputs.extend(EventData::stage_inputs());
    inputs.sort_by_key(|r| r.name());
    inputs.dedup_by_key(|r| r.name());
    if profile == stages::Profile::Catalog {
        inputs.retain(|r| {
            r.name() != FlowValuePathObservation::NAME && r.name() != FlowCallStep::NAME
        });
    }
    stages::Stage {
        name: "normalize_events",
        inputs: super::facts_stage_inputs(inputs),
        outputs: super::events::relations()
            .iter()
            .map(stages::RelationUse::of_relation)
            .collect(),
        contributes: vec![],
        coverage: vec![],
        profiles: vec![profile],
        effect: stages::Effect::Pure,
        code: policy_revision(),
        configuration: ContentHash::of(b"events/v1"),
    }
}

pub(crate) fn invariants_refs() -> Vec<&'static str> { vec!["normalized_event_closure"] }
