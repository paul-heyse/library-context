//! Assemble the entire event and its attributed members before the single policy evaluator.
use super::{Rows, RowsView, entities::*, events::*, links::LinkReason, policy_revision};
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
trait Source<'a, R: Record> {
    fn rows(&self) -> RowsView<'a, R>;
}
macro_rules! inputs {
    ($($field:ident: $ty:ty,)*) => {
        pub struct EventData { $(pub $field: Rows<$ty>,)* }
        #[derive(Clone, Copy)]
        pub struct EventDataView<'a> { $(pub $field: RowsView<'a, $ty>,)* }
        $(impl<'a> Source<'a, $ty> for EventDataView<'a> { fn rows(&self) -> RowsView<'a, $ty> { self.$field } })*
        impl EventData {
            pub fn view(&self) -> EventDataView<'_> { EventDataView { $($field: self.$field.view(),)* } }
            pub fn new(budget: &ResourceBudget) -> Self { Self { $($field: Rows::new(budget),)* } }
            pub fn visit(&mut self, relation: &str, batch: &arrow_array::RecordBatch) -> Result<bool, ModelError> {
                $(if relation == <$ty>::NAME { self.$field.decode(batch)?; return Ok(true); })* Ok(false)
            }
            pub fn validation_inputs() -> Vec<ValidationInput> { super::facts_inputs(vec![$(ValidationInput::of::<$ty>(&["id"]),)*]) }
            pub fn stage_inputs() -> Vec<stages::RelationUse> { vec![$(stages::RelationUse::completed::<$ty>()),*] }
        }
    }
}
crate::normalized_event_inputs!(inputs);
macro_rules! outputs {
    ($($field:ident: $ty:ty,)*) => {
        pub struct EventOutput { $(pub $field: Rows<$ty>,)* }
        #[derive(Clone, Copy)]
        pub struct EventOutputView<'a> { $(pub $field: RowsView<'a, $ty>,)* }
        impl EventOutputView<'_> { pub fn matches(&self, expected: &EventOutput) -> Result<(), ModelError> { $(if !self.$field.same(&expected.$field) { return Err(ModelError::Invalid(format!("normalized event closure differs: {}", <$ty>::NAME))); })* Ok(()) } }
        impl EventOutput {
            pub fn view(&self) -> EventOutputView<'_> { EventOutputView { $($field: self.$field.view(),)* } }
            pub fn new(budget: &ResourceBudget) -> Self { Self { $($field: Rows::new(budget),)* } }
            pub fn visit(&mut self, relation: &str, batch: &arrow_array::RecordBatch) -> Result<bool, ModelError> {
                $(if relation == <$ty>::NAME { self.$field.decode(batch)?; return Ok(true); })* Ok(false)
            }
            pub fn matches(&self, expected: &Self) -> Result<(), ModelError> { self.view().matches(expected) }
            pub fn validation_inputs() -> Vec<ValidationInput> { vec![$(ValidationInput::of::<$ty>(&["id"]),)*] }
        }
    }
}
crate::normalized_event_outputs!(outputs);
fn receiver_proofs(
    data: &EventDataView<'_>,
    budget: &ResourceBudget,
) -> Result<super::receiver::VerifiedReceivers, ModelError> {
    macro_rules! input {($($field:ident: $ty:ty,)*)=>{super::receiver::ReceiverDataView { $($field:<EventDataView<'_> as Source<'_, $ty>>::rows(data),)* } };}
    macro_rules! output {($($field:ident: $ty:ty,)*)=>{super::receiver::ReceiverOutputView { $($field:<EventDataView<'_> as Source<'_, $ty>>::rows(data),)* } };}
    let inputs = crate::normalized_receiver_inputs!(input);
    let outputs = crate::normalized_receiver_outputs!(output);
    super::receiver::prepare_view(&inputs, &outputs, budget)
}
fn invalid(message: impl Into<String>) -> ModelError {
    ModelError::Invalid(message.into())
}
fn need<'a, R: Record>(rows: &RowsView<'a, R>, id: Id<R>) -> Result<&'a R, ModelError> {
    rows.get(id)
        .ok_or_else(|| invalid(format!("normalized event requires {}", R::NAME)))
}
fn qualification<'a>(
    data: &EventDataView<'a>,
    id: Id<AssertionQualification>,
) -> Result<&'a AssertionQualification, ModelError> {
    need(&data.qualifications, id)
}
fn exact(q: &AssertionQualification) -> bool {
    q.modality == Modality::Definite && q.approximation == Approximation::Exact
}
pub type EventKey = (Id<Occurrence>, Id<CallOrigin>, Id<AnalysisContext>);
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
    fn new(data: &EventDataView<'a>, budget: &ResourceBudget) -> Result<Self, ModelError> {
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
#[derive(Clone, Copy)]
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
type EventSite = (Id<Occurrence>, Id<AnalysisContext>);
type EventsBySite = ChargedMap<EventSite, Vec<Id<NormalizedCallEvent>>>;
pub struct VerifiedEvents {
    flow: EventsBySite,
    complete: ChargedMap<Id<NormalizedCallEvent>, CompleteEvent>,
    dispatch: ChargedMap<Id<NormalizedCallAlternative>, super::dispatch::ApplicableDispatch>,
    _charge: StateCharge,
}
impl VerifiedEvents {
    pub fn append(&mut self, other: Self) -> Result<(), ModelError> {
        if !self
            ._charge
            .budget()
            .expect("owner budget")
            .shares_pool(other._charge.budget().expect("owner budget"))
        {
            return Err(ModelError::Conflict("event authority budget"));
        }
        for (key, events) in other.flow.iter() {
            if self.flow.contains_key(key) {
                return Err(ModelError::Conflict("flow event authority domain"));
            }
            self.flow.insert(&mut self._charge, *key, events.clone())?;
        }
        for (event, proof) in other.complete.iter() {
            if self.complete.contains_key(event) {
                return Err(ModelError::Conflict("event authority domain"));
            }
            self.complete.insert(&mut self._charge, *event, *proof)?;
        }
        for (alternative, proof) in other.dispatch.iter() {
            if self.dispatch.contains_key(alternative) {
                return Err(ModelError::Conflict("dispatch authority domain"));
            }
            self.dispatch
                .insert(&mut self._charge, *alternative, *proof)?;
        }
        Ok(())
    }

    pub(super) fn dispatch(
        &self,
        alternative: Id<NormalizedCallAlternative>,
    ) -> Option<&super::dispatch::ApplicableDispatch> {
        self.dispatch.get(&alternative)
    }
    pub(super) fn get(&self, event: Id<NormalizedCallEvent>) -> Option<&CompleteEvent> {
        self.complete.get(&event)
    }
}
/// Internal reconstruction assumes checked N1/N2 premises. Only full binding replay exposes an
/// admission token outside this module family. Public validation returns no authority token.
pub(super) fn verify_view(
    data: &EventDataView<'_>,
    stored: &EventOutputView<'_>,
    budget: &ResourceBudget,
) -> Result<VerifiedEvents, ModelError> {
    let (expected, tokens) = evaluate(data, budget)?;
    stored.matches(&expected)?;
    Ok(tokens)
}
/// Build only consumer admissions from immutable completed event rows. Event production and
/// diagnostic replay remain separate: this path neither emits alternatives nor calls evaluate.
pub(super) fn prepare_view(
    data: &EventDataView<'_>,
    stored: &EventOutputView<'_>,
    budget: &ResourceBudget,
) -> Result<VerifiedEvents, ModelError> {
    prepare_selected(data, stored, None, budget)
}
/// Necessary predicates for one complete actual candidate/advertised event domain.
pub fn admit_event(
    data: &EventData,
    stored: &EventOutput,
    key: EventKey,
    budget: &ResourceBudget,
) -> Result<(), ModelError> {
    admit_event_view(&data.view(), &stored.view(), key, budget)
}
pub fn admit_event_view(
    data: &EventDataView<'_>,
    stored: &EventOutputView<'_>,
    key: EventKey,
    budget: &ResourceBudget,
) -> Result<(), ModelError> {
    prepare_selected(data, stored, Some(&[key]), budget).map(|_| ())
}
pub(super) fn prepare_consumed_view(
    data: &EventDataView<'_>,
    stored: &EventOutputView<'_>,
    budget: &ResourceBudget,
) -> Result<VerifiedEvents, ModelError> {
    let mut charge = StateCharge::new(budget, "consumed-event-root-keys");
    let mut keys = Vec::new();
    for event in stored.events.iter() {
        charge.grow(size_of::<EventKey>())?;
        keys.push((event.site, event.origin, event.context));
    }
    prepare_selected(data, stored, Some(&keys), budget)
}
fn prepare_selected(
    data: &EventDataView<'_>,
    stored: &EventOutputView<'_>,
    selected: Option<&[EventKey]>,
    budget: &ResourceBudget,
) -> Result<VerifiedEvents, ModelError> {
    let index = Index::new(data, budget)?;
    let receiver_proofs = receiver_proofs(data, budget)?;
    let mut tokens = VerifiedEvents {
        flow: Default::default(),
        complete: Default::default(),
        dispatch: Default::default(),
        _charge: StateCharge::new(budget, "prepared-event-admissions"),
    };
    for key in index
        .universe
        .iter()
        .filter(|key| selected.is_none_or(|selected| selected.contains(key)))
    {
        let mut events = stored
            .events
            .iter()
            .filter(|event| (event.site, event.origin, event.context) == *key);
        let event = events
            .next()
            .ok_or_else(|| invalid("captured call event has no required normalized outcome"))?;
        if events.next().is_some() {
            return Err(invalid(
                "captured call event has ambiguous normalized identity",
            ));
        }
        let mut assessments = stored
            .assessments
            .iter()
            .filter(|assessment| assessment.event == event.id());
        if assessments.next().is_none() || assessments.next().is_some() {
            return Err(invalid("event has no exact assessment domain"));
        }
        for policy in CallPolicy::ALL {
            let mut policies = stored
                .policy_assessments
                .iter()
                .filter(|assessment| assessment.event == event.id() && assessment.policy == policy);
            if policies.next().is_none() || policies.next().is_some() {
                return Err(invalid(
                    "event lacks its exact consumer policy outcome domain",
                ));
            }
        }
    }
    if let Some(selected) = selected {
        for key in selected {
            if !index.universe.contains(key) {
                return Err(invalid("advertised event has no captured candidate domain"));
            }
        }
        if stored
            .events
            .iter()
            .any(|event| !selected.contains(&(event.site, event.origin, event.context)))
        {
            return Err(invalid("event grain contains another advertised root"));
        }
    }
    for assessment in stored.assessments.iter() {
        if assessment.policy != policy_revision() {
            return Err(invalid("foreign event policy"));
        }
        let event = need(&stored.events, assessment.event)?;
        let key = (event.site, event.origin, event.context);
        if !index.universe.contains(&key)
            || !index
                .owners
                .get(&event.site)
                .is_some_and(|owner| owner.id() == event.owner)
        {
            return Err(invalid("event is outside captured owner universe"));
        }
        admit_native_domain(data, stored, event, &index, budget)?;
        let mut digest = KeySink::new("complete-normalized-event");
        for site in index.sites.get(&key).into_iter().flatten() {
            let source = CallEventSource {
                event: event.id(),
                observation: site.id(),
            };
            if stored.sources.get(source.id()) != Some(&source) {
                return Err(invalid("event source lineage is incomplete"));
            }
            source.id().encode(&mut digest);
            for support in index.site_supports.get(&site.id()).into_iter().flatten() {
                let evidence = CallEventSourceEvidence {
                    source: source.id(),
                    support: support.id(),
                };
                if stored.source_evidence.get(evidence.id()) != Some(&evidence) {
                    return Err(invalid("event source evidence is incomplete"));
                }
                evidence.id().encode(&mut digest);
            }
        }
        for resolution in index.resolutions.get(&key).into_iter().flatten() {
            let member = CallEventResolution {
                event: event.id(),
                resolution: resolution.id(),
            };
            if stored.resolutions.get(member.id()) != Some(&member) {
                return Err(invalid("event resolution lineage is incomplete"));
            }
            member.id().encode(&mut digest);
            for support in index
                .resolution_supports
                .get(&resolution.id())
                .into_iter()
                .flatten()
            {
                let evidence = CallEventResolutionEvidence {
                    resolution: member.id(),
                    support: support.id(),
                };
                if stored.resolution_evidence.get(evidence.id()) != Some(&evidence) {
                    return Err(invalid("event resolution evidence is incomplete"));
                }
                evidence.id().encode(&mut digest);
            }
            if matches!(
                need(&data.channels, resolution.channel)?,
                CallChannel::Direct
            ) {
                for target in index.members.get(&resolution.id()).into_iter().flatten() {
                    if let Some(proof) = receiver_proofs.get(target.id()) {
                        proof.assessment().encode(&mut digest);
                    }
                }
            }
        }
        for alternative in stored
            .alternatives
            .iter()
            .filter(|alternative| alternative.event == event.id())
        {
            alternative.id().encode(&mut digest);
            for support in index
                .supports
                .get(&need(&stored.alternative_sources, alternative.source)?.target())
                .into_iter()
                .flatten()
            {
                support.id().encode(&mut digest);
            }
        }
        for dispatch in stored
            .dispatch_assessments
            .iter()
            .filter(|dispatch| dispatch.event == event.id())
        {
            dispatch.id().encode(&mut digest);
            dispatch.members.encode(&mut digest);
        }
        if assessment.members != digest.finish() {
            return Err(invalid(
                "event admission has foreign input/evidence membership",
            ));
        }
        // Only the complete, exact, unique event grants shape/composition authority. Recheck
        // its actual candidate and provider domain, rather than trusting stored booleans.
        if assessment.complete
            && assessment.unique
            && assessment.exact
            && assessment.known_receivers
        {
            let mut charge = StateCharge::new(budget, "event-admission-closure");
            let mut declared: ChargedSet<(Id<ProviderRun>, Id<source::CoverageScope>)> =
                Default::default();
            let mut resolved: ChargedSet<(Id<ProviderRun>, Id<source::CoverageScope>)> =
                Default::default();
            let mut reported: ChargedSet<Id<CallTarget>> = Default::default();
            let mut groups: ChargedSet<PhaseGroup> = Default::default();
            let mut phases: ChargedMap<CallPhase, std::collections::BTreeSet<Id<EntityRef>>> =
                Default::default();
            let mut direct_count = 0;
            for site in index.sites.get(&key).into_iter().flatten() {
                let q = qualification(data, site.qualification)?;
                let supports = index
                    .site_supports
                    .get(&site.id())
                    .filter(|rows| !rows.is_empty())
                    .ok_or_else(|| invalid("admitted event site has no support"))?;
                for support in supports {
                    declared.insert(&mut charge, (support.run, q.scope))?;
                }
            }
            for resolution in index.resolutions.get(&key).into_iter().flatten() {
                let q = qualification(data, resolution.qualification)?;
                let direct = matches!(
                    need(&data.channels, resolution.channel)?,
                    CallChannel::Direct
                );
                let candidates = index
                    .members
                    .get(&resolution.id())
                    .map(Vec::as_slice)
                    .unwrap_or(&[]);
                let mut values = Vec::with_capacity(candidates.len());
                for target in candidates {
                    charge.admit(*target)?;
                    values.push((*target).clone());
                }
                let (expected, _) = CallResolution::new(
                    q,
                    event.site,
                    event.origin,
                    resolution.channel,
                    resolution.phase,
                    resolution.complete,
                    &values,
                )?;
                if expected != **resolution {
                    return Err(invalid("event candidate closure is incomplete"));
                }
                if !direct {
                    continue;
                }
                direct_count += 1;
                if !resolution.complete || !exact(q) || candidates.is_empty() {
                    return Err(invalid(
                        "complete event has an open or uncertain direct resolution",
                    ));
                }
                groups.insert(&mut charge, PhaseGroup::of(resolution.phase))?;
                let supports = index
                    .resolution_supports
                    .get(&resolution.id())
                    .filter(|rows| !rows.is_empty())
                    .ok_or_else(|| invalid("complete event resolution has no support"))?;
                for support in supports {
                    resolved.insert(&mut charge, (support.run, q.scope))?;
                }
                for target in candidates {
                    reported.insert(&mut charge, target.id())?;
                    let tq = qualification(data, target.qualification)?;
                    if tq.context != event.context || tq.scope != q.scope || !exact(tq) {
                        return Err(invalid("complete event target crosses frame or certainty"));
                    }
                    let destination = need(&data.destinations, target.destination)?;
                    if !matches!(destination, CallDestination::Resolved { .. }) {
                        return Err(invalid(
                            "complete event has unresolved or dispatched target",
                        ));
                    }
                    let (correspondence, entity, status, _) =
                        correspondence(data, &index, destination)?;
                    let entity = entity
                        .filter(|_| status == ResolutionStatus::Resolved)
                        .ok_or_else(|| {
                            invalid("complete event has no unique entity correspondence")
                        })?;
                    let mapping = need(
                        &data.symbol_resolutions,
                        correspondence.ok_or_else(|| invalid("event correspondence absent"))?,
                    )?;
                    if mapping.context != event.context {
                        return Err(invalid("event correspondence crosses context"));
                    }
                    if receiver_proofs.get(target.id()).is_none()
                        && matches!(
                            need(&data.receivers, target.receiver)?,
                            Receiver::Unknown { .. }
                        )
                    {
                        return Err(invalid("complete event has unknown receiver"));
                    }
                    let mut alternatives = stored.alternatives.iter().filter(|a| a.event == event.id()
                        && a.resolution == Some(resolution.id())
                        && stored.alternative_sources.get(a.source).is_some_and(|s| matches!(s, CallAlternativeSource::Native { target: id } if *id == target.id())));
                    let alternative = alternatives
                        .next()
                        .ok_or_else(|| invalid("complete event target alternative missing"))?;
                    if alternatives.next().is_some()
                        || alternative.entity != Some(entity)
                        || alternative.correspondence != correspondence
                    {
                        return Err(invalid("event alternative correspondence is ambiguous"));
                    }
                    phases.update(&mut charge, target.phase, |entities| {
                        entities.insert(entity);
                    })?;
                }
            }
            if direct_count == 0
                || !declared.iter().all(|run| resolved.contains(run))
                || groups.len() != 1
                || phases.values().any(|entities| entities.len() != 1)
                || index.targets.get(&key).into_iter().flatten().any(|target| {
                    data.channels
                        .get(target.channel)
                        .is_some_and(|channel| matches!(channel, CallChannel::Direct))
                        && !reported.contains(&target.id())
                })
            {
                return Err(invalid(
                    "complete event lacks its closed unique provider/phase domain",
                ));
            }
            tokens.complete.insert(
                &mut tokens._charge,
                event.id(),
                CompleteEvent {
                    event: event.id(),
                    assessment: assessment.id(),
                    members: assessment.members,
                },
            )?;
        }
    }
    // Dispatch membership also conveys eligibility. Check its bounded target/ancestry group
    // once before borrowing members; a stored correspondence alone does not prove ancestry.
    for assessment in stored.dispatch_assessments.iter() {
        let event = need(&stored.events, assessment.event)?;
        let target = need(&data.targets, assessment.target)?;
        let q = qualification(data, target.qualification)?;
        if (target.site, target.origin, q.context) != (event.site, event.origin, event.context) {
            return Err(invalid(
                "dispatch target crosses the normalized event frame",
            ));
        }
        let mut expected = EventOutput::new(budget);
        let members =
            super::dispatch::assess_view(data, event.id(), target, &mut expected, budget)?;
        if expected.dispatch_assessments.get(assessment.id()) != Some(assessment)
            || members
                .members
                .iter()
                .any(|member| stored.dispatch_members.get(member.id()) != Some(member))
            || stored
                .dispatch_members
                .iter()
                .filter(|member| member.assessment == assessment.id())
                .count()
                != members.members.len()
            || expected
                .dispatch_evidence
                .iter()
                .any(|evidence| stored.dispatch_evidence.get(evidence.id()) != Some(evidence))
            || expected
                .dispatch_premises
                .iter()
                .any(|premise| stored.dispatch_premises.get(premise.id()) != Some(premise))
        {
            return Err(invalid(
                "dispatch admission exceeds the actual target/ancestry predicate",
            ));
        }
    }
    for alternative in stored.alternatives.iter() {
        let event = need(&stored.events, alternative.event)?;
        let source = need(&stored.alternative_sources, alternative.source)?;
        let member = match source {
            CallAlternativeSource::DerivedDispatch { member, .. } => {
                Some(need(&stored.dispatch_members, *member)?)
            }
            CallAlternativeSource::Native { target } => {
                stored.dispatch_members.iter().find(|member| {
                    member.named
                        && stored
                            .dispatch_assessments
                            .get(member.assessment)
                            .is_some_and(|a| a.event == event.id() && a.target == *target)
                })
            }
        };
        if let Some(member) = member {
            let assessment = need(&stored.dispatch_assessments, member.assessment)?;
            let mapping = need(&data.symbol_resolutions, member.correspondence)?;
            if assessment.policy != policy_revision()
                || assessment.event != event.id()
                || assessment.target != source.target()
                || mapping.symbol != member.symbol
                || mapping.context != event.context
                || mapping.entity != Some(member.entity)
                || mapping.status != ResolutionStatus::Resolved
                || (member.named && assessment.named != member.symbol)
            {
                return Err(invalid(
                    "dispatch admission differs from checked correspondence",
                ));
            }
            tokens.dispatch.insert(
                &mut tokens._charge,
                alternative.id(),
                super::dispatch::ApplicableDispatch::from_member(member, assessment, event.context),
            )?;
        }
    }
    for policy in stored.policy_assessments.iter() {
        let event = need(&stored.events, policy.event)?;
        let assessment = need(&stored.assessments, policy.event_assessment)?;
        if assessment.event != event.id() {
            return Err(invalid("event policy refers to another event assessment"));
        }
        let mut charge = StateCharge::new(budget, "event-policy-admission-domain");
        let mut expected: ChargedSet<Id<NormalizedCallAlternative>> = Default::default();
        for alternative in stored
            .alternatives
            .iter()
            .filter(|alternative| alternative.event == event.id())
        {
            if admits(
                policy.policy,
                data,
                &index,
                alternative,
                tokens.get(event.id()),
                stored,
            )? {
                expected.insert(&mut charge, alternative.id())?;
            }
        }
        let mut digest = KeySink::new("call-policy-members");
        for id in expected.iter() {
            id.encode(&mut digest);
            if !stored.admissions.iter().any(|admission| {
                admission.assessment == policy.id() && admission.alternative == *id
            }) {
                return Err(invalid(
                    "eligible event alternative is missing its required policy admission",
                ));
            }
        }
        if policy.admitted != expected.len() as i64
            || policy.members != digest.finish()
            || stored
                .admissions
                .iter()
                .filter(|admission| admission.assessment == policy.id())
                .count()
                != expected.len()
        {
            return Err(invalid("event policy closed alternative domain differs"));
        }
    }
    for admission in stored.admissions.iter() {
        let assessment = need(&stored.policy_assessments, admission.assessment)?;
        let alternative = need(&stored.alternatives, admission.alternative)?;
        if assessment.event != alternative.event
            || !admits(
                assessment.policy,
                data,
                &index,
                alternative,
                tokens.get(alternative.event),
                stored,
            )?
        {
            return Err(invalid(
                "event policy admission exceeds eligible alternatives",
            ));
        }
    }
    Ok(tokens)
}

fn admit_native_domain(
    data: &EventDataView<'_>,
    stored: &EventOutputView<'_>,
    event: &NormalizedCallEvent,
    index: &Index<'_>,
    budget: &ResourceBudget,
) -> Result<(), ModelError> {
    let key = (event.site, event.origin, event.context);
    let mut charge = StateCharge::new(budget, "event-native-source-domain");
    let mut reported = ChargedSet::default();
    let mut expected = Rows::<NormalizedCallAlternative>::new(budget);
    let mut evidence = Rows::<CallAlternativeEvidence>::new(budget);
    let mut native =
        |target: &CallTarget, resolution: Option<Id<CallResolution>>| -> Result<(), ModelError> {
            let q = qualification(data, target.qualification)?;
            if (target.site, target.origin, q.context) != key {
                return Err(invalid(
                    "native event target crosses qualified source frame",
                ));
            }
            let (correspondence, entity, status, reason) =
                correspondence(data, index, need(&data.destinations, target.destination)?)?;
            let source = CallAlternativeSource::Native {
                target: target.id(),
            };
            if stored.alternative_sources.get(source.id()) != Some(&source) {
                return Err(invalid("native alternative source absent"));
            }
            let row = NormalizedCallAlternative {
                event: event.id(),
                source: source.id(),
                resolution,
                correspondence,
                entity,
                status,
                reason,
            };
            let alternative = expected.insert(row)?;
            let supports = index
                .supports
                .get(&target.id())
                .filter(|supports| !supports.is_empty())
                .ok_or_else(|| invalid("native event target support absent"))?;
            for support in supports {
                evidence.insert(CallAlternativeEvidence {
                    alternative,
                    support: support.id(),
                })?;
            }
            if matches!(
                need(&data.destinations, target.destination)?,
                CallDestination::Overrides { .. }
            ) {
                let mut assessments = stored.dispatch_assessments.iter().filter(|assessment| {
                    assessment.event == event.id() && assessment.target == target.id()
                });
                let assessment = assessments
                    .next()
                    .ok_or_else(|| invalid("native dispatch eligibility assessment absent"))?;
                if assessments.next().is_some() {
                    return Err(invalid("native dispatch eligibility assessment ambiguous"));
                }
                for member in stored
                    .dispatch_members
                    .iter()
                    .filter(|member| member.assessment == assessment.id() && !member.named)
                {
                    let source = CallAlternativeSource::DerivedDispatch {
                        target: target.id(),
                        member: member.id(),
                    };
                    if stored.alternative_sources.get(source.id()) != Some(&source) {
                        return Err(invalid("dispatch alternative source absent"));
                    }
                    let alternative = expected.insert(NormalizedCallAlternative {
                        event: event.id(),
                        source: source.id(),
                        resolution,
                        correspondence: Some(member.correspondence),
                        entity: Some(member.entity),
                        status: ResolutionStatus::Resolved,
                        reason: LinkReason::ExplicitIdentity,
                    })?;
                    for support in supports {
                        evidence.insert(CallAlternativeEvidence {
                            alternative,
                            support: support.id(),
                        })?;
                    }
                }
            }
            Ok(())
        };
    for resolution in index.resolutions.get(&key).into_iter().flatten() {
        let q = qualification(data, resolution.qualification)?;
        let targets = index
            .members
            .get(&resolution.id())
            .map(Vec::as_slice)
            .unwrap_or(&[]);
        let mut values = Vec::new();
        for target in targets {
            charge.admit(*target)?;
            values.push((*target).clone());
            let tq = qualification(data, target.qualification)?;
            if tq.context != event.context || tq.scope != q.scope {
                return Err(invalid(
                    "native event resolution crosses qualification context or scope",
                ));
            }
        }
        if CallResolution::new(
            q,
            event.site,
            event.origin,
            resolution.channel,
            resolution.phase,
            resolution.complete,
            &values,
        )?
        .0 != **resolution
        {
            return Err(invalid("native event resolution member domain differs"));
        }
        for target in targets {
            reported.insert(&mut charge, target.id())?;
            native(target, Some(resolution.id()))?;
        }
    }
    for target in index
        .targets
        .get(&key)
        .into_iter()
        .flatten()
        .filter(|target| !reported.contains(&target.id()))
    {
        native(target, None)?;
    }
    if expected
        .iter()
        .any(|row| stored.alternatives.get(row.id()) != Some(row))
        || stored
            .alternatives
            .iter()
            .filter(|row| row.event == event.id())
            .count()
            != expected.len()
        || evidence
            .iter()
            .any(|row| stored.alternative_evidence.get(row.id()) != Some(row))
        || stored
            .alternative_evidence
            .iter()
            .filter(|row| {
                stored
                    .alternatives
                    .get(row.alternative)
                    .is_some_and(|alternative| alternative.event == event.id())
            })
            .count()
            != evidence.len()
    {
        return Err(invalid(
            "native event alternative/source member domain differs",
        ));
    }
    if stored
        .sources
        .iter()
        .filter(|source| source.event == event.id())
        .count()
        != index.sites.get(&key).map_or(0, Vec::len)
        || stored
            .resolutions
            .iter()
            .filter(|resolution| resolution.event == event.id())
            .count()
            != index.resolutions.get(&key).map_or(0, Vec::len)
    {
        return Err(invalid(
            "native event site/resolution source domain differs",
        ));
    }
    Ok(())
}

pub fn validate(
    data: &EventData,
    stored: &EventOutput,
    budget: &ResourceBudget,
) -> Result<(), ModelError> {
    validate_view(&data.view(), &stored.view(), budget)
}
pub fn validate_view(
    data: &EventDataView<'_>,
    stored: &EventOutputView<'_>,
    budget: &ResourceBudget,
) -> Result<(), ModelError> {
    stored.matches(&normalize_view(data, budget)?)
}
pub fn normalize(data: &EventData, budget: &ResourceBudget) -> Result<EventOutput, ModelError> {
    normalize_view(&data.view(), budget)
}
pub fn normalize_view(
    data: &EventDataView<'_>,
    budget: &ResourceBudget,
) -> Result<EventOutput, ModelError> {
    Ok(evaluate(data, budget)?.0)
}

type Correspondence = (
    Option<Id<SymbolEntityResolution>>,
    Option<Id<EntityRef>>,
    ResolutionStatus,
    LinkReason,
);
fn correspondence(
    data: &EventDataView<'_>,
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
    data: &EventDataView<'_>,
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
    let dispatch = super::dispatch::assess_view(data, event, target, output, budget)?;
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
    data: &EventDataView<'_>,
    budget: &ResourceBudget,
) -> Result<(EventOutput, VerifiedEvents), ModelError> {
    let receiver_proofs = receiver_proofs(data, budget)?;
    let (mut output, tokens) = evaluate_selected(data, None, &receiver_proofs, budget)?;
    flow_links(data, &mut output, budget)?;
    Ok((output, tokens))
}
/// One complete event candidate domain. Dependency events are not additional output roots.
/// Receiver authority comes from its owning normalization, never a stored assessment conversion.
pub fn normalize_events_produced(
    data: &EventData,
    receivers: &super::receiver::VerifiedReceivers,
    budget: &ResourceBudget,
) -> Result<(EventOutput, VerifiedEvents), ModelError> {
    normalize_events_produced_view(&data.view(), receivers, budget)
}
pub fn normalize_events_produced_view(
    data: &EventDataView<'_>,
    receivers: &super::receiver::VerifiedReceivers,
    budget: &ResourceBudget,
) -> Result<(EventOutput, VerifiedEvents), ModelError> {
    evaluate_selected(data, None, receivers, budget)
}
pub fn normalize_event_produced(
    data: &EventData,
    key: EventKey,
    receivers: &super::receiver::VerifiedReceivers,
    budget: &ResourceBudget,
) -> Result<(EventOutput, VerifiedEvents), ModelError> {
    normalize_event_produced_view(&data.view(), key, receivers, budget)
}
pub fn normalize_event_produced_view(
    data: &EventDataView<'_>,
    key: EventKey,
    receivers: &super::receiver::VerifiedReceivers,
    budget: &ResourceBudget,
) -> Result<(EventOutput, VerifiedEvents), ModelError> {
    evaluate_selected(data, Some(key), receivers, budget)
}
fn evaluate_selected(
    data: &EventDataView<'_>,
    selected: Option<EventKey>,
    receiver_proofs: &super::receiver::VerifiedReceivers,
    budget: &ResourceBudget,
) -> Result<(EventOutput, VerifiedEvents), ModelError> {
    let index = Index::new(data, budget)?;
    if selected.is_some_and(|key| !index.universe.contains(&key)) {
        return Err(invalid("event root absent from actual candidate universe"));
    }
    let mut output = EventOutput::new(budget);
    let mut tokens = VerifiedEvents {
        flow: Default::default(),
        complete: Default::default(),
        dispatch: Default::default(),
        _charge: StateCharge::new(budget, "complete-event-tokens"),
    };
    for &(site, origin, ctx) in index
        .universe
        .iter()
        .filter(|key| selected.is_none_or(|selected| selected == **key))
    {
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
        if origin == CallOrigin::explicit() {
            tokens
                .flow
                .update(&mut tokens._charge, (site, ctx), |events| {
                    events.push(event)
                })?;
        }
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
            let source = need(&output.alternative_sources.view(), alternative.source)?;
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
                let assessment = need(&output.dispatch_assessments.view(), member.assessment)?;
                tokens.dispatch.insert(
                    &mut tokens._charge,
                    alternative.id(),
                    super::dispatch::ApplicableDispatch::from_member(member, assessment, ctx),
                )?;
            }
            alternative.id().encode(&mut digest);
            for support in index
                .supports
                .get(&need(&output.alternative_sources.view(), alternative.source)?.target())
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
                    &output.view(),
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
    Ok((output, tokens))
}
fn admits(
    policy: super::events::CallPolicy,
    data: &EventDataView<'_>,
    index: &Index<'_>,
    alternative: &NormalizedCallAlternative,
    complete: Option<&CompleteEvent>,
    output: &EventOutputView<'_>,
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
    data: &EventDataView<'_>,
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
    flow_links_with(data, output, &events, None, budget)
}
/// Link one observed path only after the complete owning event normalization has finished.
/// Admission of a path's source/event correspondence uses its complete actual stored event bag.
pub fn admit_flow_path(
    data: &EventData,
    stored: &EventOutput,
    path: Id<FlowValuePathObservation>,
    budget: &ResourceBudget,
) -> Result<(), ModelError> {
    admit_flow_path_view(&data.view(), &stored.view(), path, budget)
}
pub fn admit_flow_path_view(
    data: &EventDataView<'_>,
    stored: &EventOutputView<'_>,
    path: Id<FlowValuePathObservation>,
    budget: &ResourceBudget,
) -> Result<(), ModelError> {
    need(&data.paths, path)?;
    let mut index = ChargedMap::default();
    let mut charge = StateCharge::new(budget, "admitted-flow-event-index");
    for event in stored
        .events
        .iter()
        .filter(|event| event.origin == CallOrigin::explicit())
    {
        index.update(
            &mut charge,
            (event.site, event.context),
            |events: &mut Vec<Id<NormalizedCallEvent>>| events.push(event.id()),
        )?;
    }
    let mut expected = EventOutput::new(budget);
    flow_links_with(data, &mut expected, &index, Some(path), budget)?;
    if !stored.flow_links.same(&expected.flow_links) {
        return Err(invalid("flow event source/domain correspondence differs"));
    }
    Ok(())
}
pub fn normalize_flow_path(
    data: &EventData,
    path: Id<FlowValuePathObservation>,
    events: &VerifiedEvents,
    budget: &ResourceBudget,
) -> Result<EventOutput, ModelError> {
    normalize_flow_path_view(&data.view(), path, events, budget)
}
pub fn normalize_flow_path_view(
    data: &EventDataView<'_>,
    path: Id<FlowValuePathObservation>,
    events: &VerifiedEvents,
    budget: &ResourceBudget,
) -> Result<EventOutput, ModelError> {
    need(&data.paths, path)?;
    let mut output = EventOutput::new(budget);
    flow_links_with(data, &mut output, &events.flow, Some(path), budget)?;
    Ok(output)
}
fn flow_links_with(
    data: &EventDataView<'_>,
    output: &mut EventOutput,
    events: &EventsBySite,
    selected: Option<Id<FlowValuePathObservation>>,
    budget: &ResourceBudget,
) -> Result<(), ModelError> {
    let mut charge = StateCharge::new(budget, "flow-event-path-grain");
    let mut steps: ChargedMap<Id<FlowCallPath>, Vec<&FlowCallStep>> = Default::default();
    for row in data.steps.iter() {
        steps.update(&mut charge, row.path, |v| v.push(row))?;
    }
    for path in data
        .paths
        .iter()
        .filter(|path| selected.is_none_or(|selected| selected == path.id()))
    {
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
    vec![
        Invariant {
            purpose: crate::domain::InvariantPurpose::DiagnosticReplay,
            revision: 1,
            name: "normalized_event_closure",
            inputs: inputs.clone(),
            create: std::sync::Arc::new(|budget| {
                Box::new(EventCheck {
                    data: EventData::new(budget),
                    output: EventOutput::new(budget),
                    budget: budget.clone(),
                    admission: false,
                })
            }),
        },
        Invariant {
            purpose: crate::domain::InvariantPurpose::Admission,
            revision: 1,
            name: "normalized_event_admission",
            inputs,
            create: std::sync::Arc::new(|budget| {
                Box::new(EventCheck {
                    data: EventData::new(budget),
                    output: EventOutput::new(budget),
                    budget: budget.clone(),
                    admission: true,
                })
            }),
        },
    ]
}
struct EventCheck {
    data: EventData,
    output: EventOutput,
    budget: ResourceBudget,
    admission: bool,
}
impl InvariantCheck for EventCheck {
    fn normalization_scope(&self) -> Option<super::admission::Scope> {
        self.admission.then_some(super::admission::Scope::Events)
    }
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
        if self.admission {
            return prepare_view(&self.data.view(), &self.output.view(), &self.budget).map(|_| ());
        }
        verify_view(&self.data.view(), &self.output.view(), &self.budget).map(|_| ())
    }
}
pub fn stage(profile: stages::Profile) -> stages::Stage {
    let mut inputs = super::callable_normalization::stage(profile).inputs;
    macro_rules! prior { ($($field:ident: $ty:ty,)*) => { $(inputs.push(stages::RelationUse::completed::<$ty>());)* }; }
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
        captured_binding: None,
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

pub(crate) fn invariants_refs() -> Vec<&'static str> {
    vec!["normalized_event_closure", "normalized_event_admission"]
}

#[cfg(test)]
mod preparation_controls {
    use super::*;
    fn nominal<T>(byte: u8) -> Id<T> {
        serde::Deserialize::deserialize(serde::de::value::SeqDeserializer::<
            _,
            serde::de::value::Error,
        >::new([byte; 16].into_iter()))
        .unwrap()
    }
    fn case() -> (
        EventData,
        EventOutput,
        ResourceBudget,
        Id<NormalizedCallEvent>,
    ) {
        let budget = ResourceBudget::fixed(8 << 20).unwrap();
        let mut data = EventData::new(&budget);
        let q = AssertionQualification {
            assumptions: crate::domain::assumptions::AssumptionSet::empty_id(),
            context: nominal(1),
            scope: nominal(2),
            condition: crate::domain::conditions::Diagram::always().id(),
            modality: Modality::Definite,
            approximation: Approximation::Exact,
        };
        let symbol = ProviderSymbol {
            provider: nominal(3),
            context: q.context,
            module: nominal(4),
            native_key: "function:f".into(),
            name: "f".into(),
            kind: SymbolKind::Function,
        };
        let entity = EntityRef::Callable {
            callable: super::super::entities::CallableEntity::External {
                symbol: symbol.id(),
            }
            .id(),
        };
        let destination = CallDestination::Resolved {
            symbol: symbol.id(),
        };
        let channel = CallChannel::Direct;
        let receiver = Receiver::None;
        let target = CallTarget {
            qualification: q.id(),
            site: nominal(5),
            destination: destination.id(),
            channel: channel.id(),
            phase: CallPhase::Call,
            receiver: receiver.id(),
            implicit: false,
            origin: CallOrigin::explicit(),
            receiver_class: None,
            passing: Some(ReceiverPassing::NotPassed),
            class_method: None,
            static_method: None,
        };
        let (resolution, members) = CallResolution::new(
            &q,
            target.site,
            target.origin,
            target.channel,
            target.phase,
            true,
            std::slice::from_ref(&target),
        )
        .unwrap();
        data.qualifications.insert(q.clone()).unwrap();
        data.symbols.insert(symbol.clone()).unwrap();
        data.refs.insert(entity.clone()).unwrap();
        data.symbol_resolutions
            .insert(SymbolEntityResolution {
                symbol: symbol.id(),
                context: q.context,
                policy: policy_revision(),
                status: ResolutionStatus::Resolved,
                entity: Some(entity.id()),
                reason: super::super::entities::EntityReason::ProviderExternal,
            })
            .unwrap();
        let owner = EntityRef::Occurrence {
            occurrence: target.site,
        };
        data.owners
            .insert(super::super::entities::OccurrenceOwnership {
                occurrence: target.site,
                owner: target.site,
                entity: owner.id(),
            })
            .unwrap();
        data.refs.insert(owner).unwrap();
        data.destinations.insert(destination).unwrap();
        data.channels.insert(channel).unwrap();
        data.receivers.insert(receiver).unwrap();
        data.targets.insert(target.clone()).unwrap();
        data.target_supports
            .insert(CallTargetSupport {
                assertion: target.id(),
                run: nominal(6),
                surface: nominal(7),
                evidence: nominal(8),
                origin: Origin::AnalyzerAssertion,
                mode: ExtractionMode::NativeTraversal,
                fidelity: Fidelity::NativeStructural,
            })
            .unwrap();
        data.resolutions.insert(resolution.clone()).unwrap();
        data.resolution_supports
            .insert(CallResolutionSupport {
                assertion: resolution.id(),
                run: nominal(6),
                surface: nominal(7),
                evidence: nominal(8),
                origin: Origin::AnalyzerAssertion,
                mode: ExtractionMode::NativeTraversal,
                fidelity: Fidelity::NativeStructural,
            })
            .unwrap();
        for member in members {
            data.members.insert(member).unwrap();
        }
        let output = normalize(&data, &budget).unwrap();
        let event = output.events.iter().next().unwrap().id();
        (data, output, budget, event)
    }
    #[test]
    fn completed_event_preparation_retains_exact_admission_and_refuses_missing_candidate() {
        let (mut data, output, budget, event) = case();
        let tokens = prepare_view(&data.view(), &output.view(), &budget).unwrap();
        assert_eq!(
            tokens.get(event).unwrap().assessment(),
            output.assessments.iter().next().unwrap().id()
        );
        drop(tokens);
        output
            .matches(&normalize_view(&data.view(), &budget).unwrap())
            .unwrap();
        let mut selected = data.view();
        selected.members = RowsView::selected(&data.members, &[]).unwrap();
        assert!(prepare_view(&selected, &output.view(), &budget).is_err());
        let mut advertised = output.view();
        advertised.alternatives = RowsView::selected(&output.alternatives, &[]).unwrap();
        assert!(prepare_view(&data.view(), &advertised, &budget).is_err());
        data.members = Rows::new(&budget);
        assert!(prepare_view(&data.view(), &output.view(), &budget).is_err());
    }
    #[test]
    fn completed_event_preparation_refuses_foreign_policy_without_replaying_output() {
        let (data, mut output, budget, _) = case();
        let mut assessment = output.assessments.iter().next().unwrap().clone();
        assessment.policy = ContentHash::of(b"foreign-policy");
        output.assessments = Rows::new(&budget);
        output.assessments.insert(assessment).unwrap();
        assert!(prepare_view(&data.view(), &output.view(), &budget).is_err());
    }
}
