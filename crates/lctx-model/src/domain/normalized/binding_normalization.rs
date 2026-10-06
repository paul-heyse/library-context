//! The sole argument algorithm, replayable stored outcomes and private P3/P4 admission.
use super::{
    Rows, bindings::*, callables::*, entities::*, events::*, policy_revision,
    signature_applicability::*,
};
use crate::domain::{
    attribution::*,
    calls::*,
    charged::{ChargedMap, ChargedSet, StateCharge},
    resources::ResourceBudget,
    source::*,
    *,
};
use std::collections::BTreeMap;
trait Source<R: Record> {
    fn rows(&self) -> &Rows<R>;
}
macro_rules! inputs {
    ($($field:ident: $ty:ty,)*) => {
        pub struct BindingData { $(pub $field: Rows<$ty>,)* }
        $(impl Source<$ty> for BindingData { fn rows(&self) -> &Rows<$ty> { &self.$field } })*
        impl BindingData {
            pub fn new(budget: &ResourceBudget) -> Self { Self { $($field: Rows::new(budget),)* } }
            pub fn visit(&mut self, relation: &str, batch: &arrow_array::RecordBatch) -> Result<bool, ModelError> {
                $(if relation == <$ty>::NAME { self.$field.decode(batch)?; return Ok(true); })* Ok(false)
            }
            pub fn validation_inputs() -> Vec<ValidationInput> { vec![$(ValidationInput::of::<$ty>(&["id"]),)*].into_iter().map(|input|if stages::is_vocabulary(input.name()) {input.at_epoch(stages::PublicationBoundary::Facts)}else {input}).collect() }
            pub fn stage_inputs() -> Vec<stages::RelationUse> { vec![$(stages::RelationUse::completed::<$ty>()),*].into_iter().map(|input|if stages::is_vocabulary(input.name()) {input.at_epoch(stages::PublicationBoundary::Facts)}else {input}).collect() }
        }
    }
}
crate::normalized_binding_inputs!(inputs);
macro_rules! outputs {
    ($($field:ident: $ty:ty,)*) => {
        pub struct BindingOutput { $(pub $field: Rows<$ty>,)* }
        impl BindingOutput {
            pub fn new(budget: &ResourceBudget) -> Self { Self { $($field: Rows::new(budget),)* } }
            pub fn visit(&mut self, relation: &str, batch: &arrow_array::RecordBatch) -> Result<bool, ModelError> {
                $(if relation == <$ty>::NAME { self.$field.decode(batch)?; return Ok(true); })* Ok(false)
            }
            pub fn matches(&self, expected: &Self) -> Result<(), ModelError> {
                $(if !self.$field.same(&expected.$field) { return Err(invalid(format!("normalized binding closure differs: {}", <$ty>::NAME))); })* Ok(())
            }
            pub fn validation_inputs() -> Vec<ValidationInput> { vec![$(ValidationInput::of::<$ty>(&["id"]),)*] }
        }
    }
}
crate::normalized_binding_outputs!(outputs);
fn receiver_proofs(
    data: &BindingData,
    budget: &ResourceBudget,
) -> Result<super::receiver::VerifiedReceivers, ModelError> {
    let mut inputs = super::receiver::ReceiverData::new(budget);
    let mut outputs = super::receiver::ReceiverOutput::new(budget);
    macro_rules! input {($($field:ident: $ty:ty,)*)=>{$(for row in <BindingData as Source<$ty>>::rows(data).iter(){inputs.$field.insert(row.clone())?;})*};}
    macro_rules! output {($($field:ident: $ty:ty,)*)=>{$(for row in <BindingData as Source<$ty>>::rows(data).iter(){outputs.$field.insert(row.clone())?;})*};}
    crate::normalized_receiver_inputs!(input);
    crate::normalized_receiver_outputs!(output);
    super::receiver::prepare(&inputs, &outputs, budget)
}
fn event_proofs(
    data: &BindingData,
    budget: &ResourceBudget,
) -> Result<super::event_normalization::VerifiedEvents, ModelError> {
    let mut inputs = super::event_normalization::EventData::new(budget);
    let mut outputs = super::event_normalization::EventOutput::new(budget);
    macro_rules! input {($($field:ident: $ty:ty,)*)=>{$(for row in <BindingData as Source<$ty>>::rows(data).iter(){inputs.$field.insert(row.clone())?;})*};}
    macro_rules! output {($($field:ident: $ty:ty,)*)=>{$(for row in <BindingData as Source<$ty>>::rows(data).iter(){outputs.$field.insert(row.clone())?;})*};}
    crate::normalized_event_inputs!(input);
    crate::normalized_event_outputs!(output);
    super::event_normalization::prepare_consumed(&inputs, &outputs, budget)
}
fn original_target<'a>(
    data: &'a BindingData,
    alternative: &NormalizedCallAlternative,
) -> Result<&'a CallTarget, ModelError> {
    need(
        &data.targets,
        need(&data.event_alternative_sources, alternative.source)?.target(),
    )
}
fn invalid(message: impl Into<String>) -> ModelError {
    ModelError::Invalid(message.into())
}
fn need<R: Record>(rows: &Rows<R>, id: Id<R>) -> Result<&R, ModelError> {
    rows.get(id)
        .ok_or_else(|| invalid(format!("normalized binding requires {}", R::NAME)))
}
fn scopes(data: &BindingData) -> ScopeCatalog<'_> {
    ScopeCatalog {
        scopes: &data.scopes,
        artifacts: &data.artifacts,
        modules: &data.modules,
    }
}
type SetKey = (
    Id<NormalizedCallEvent>,
    Option<Id<EntityRef>>,
    CallPhase,
    Id<CallChannel>,
);
struct Index<'a> {
    variants: ChargedMap<(Id<CallableEntity>, Id<AnalysisContext>), Vec<&'a SignatureVariant>>,
    raw_variants: ChargedMap<Id<ProviderSymbol>, Vec<&'a SignatureVariant>>,
    syntax: ChargedMap<(Id<Occurrence>, Id<AnalysisContext>), Vec<&'a CallSyntax>>,
    parameters: ChargedMap<Id<Signature>, Vec<SignatureParameter>>,
    arguments: ChargedMap<Id<CallSyntax>, Vec<CallArgument>>,
    slots: ChargedMap<Id<SignatureParameter>, &'a SignatureSlot>,
    supports: ChargedMap<Id<Signature>, Vec<&'a SignatureSupport>>,
    _charge: StateCharge,
}
impl<'a> Index<'a> {
    fn new(data: &'a BindingData, budget: &ResourceBudget) -> Result<Self, ModelError> {
        let mut s = Self {
            variants: Default::default(),
            raw_variants: Default::default(),
            syntax: Default::default(),
            parameters: Default::default(),
            arguments: Default::default(),
            slots: Default::default(),
            supports: Default::default(),
            _charge: StateCharge::new(budget, "binding-index"),
        };
        for v in data
            .callable_variants
            .iter()
            .filter(|v| v.role.runtime_source())
        {
            let raw = need(&data.signatures, v.signature)?;
            s.raw_variants
                .update(&mut s._charge, raw.symbol, |vs| vs.push(v))?;
            if let Some(callable) = v.callable {
                s.variants
                    .update(&mut s._charge, (callable, v.context), |vs| vs.push(v))?;
            }
        }
        for row in data.syntax.iter() {
            let q = need(&data.qualifications, row.qualification)?;
            s.syntax
                .update(&mut s._charge, (row.site, q.context), |vs| vs.push(row))?;
        }
        for row in data.parameters.iter() {
            s.parameters.update(&mut s._charge, row.signature, |vs| {
                vs.push(row.clone());
                vs.sort_by_key(|p| p.ordinal);
            })?;
        }
        for row in data.arguments.iter() {
            s.arguments.update(&mut s._charge, row.call, |vs| {
                vs.push(row.clone());
                vs.sort_by_key(|a| a.ordinal);
            })?;
        }
        for row in data.callable_slots.iter() {
            s.slots.insert(&mut s._charge, row.parameter, row)?;
        }
        for row in data.signature_supports.iter() {
            s.supports
                .update(&mut s._charge, row.assertion, |vs| vs.push(row))?;
        }
        Ok(s)
    }
}
fn digest(bindings: &[Binding]) -> ContentHash {
    let mut sink = KeySink::new("normalized-call-bindings");
    for b in bindings {
        b.formal.encode(&mut sink);
        b.source.encode(&mut sink);
        b.kind.encode(&mut sink);
        b.projection.encode(&mut sink);
    }
    (bindings.len() as i64).encode(&mut sink);
    sink.finish()
}
fn application<'a>(
    data: &'a BindingData,
    alternative: &'a NormalizedCallAlternative,
    variant: &'a SignatureVariant,
    call: &'a CallSyntax,
    receivers: &'a super::receiver::VerifiedReceivers,
    events: &'a super::event_normalization::VerifiedEvents,
) -> Result<ApplicableSignature<'a>, ObligationKind> {
    let missing = ObligationKind::MissingEvidence;
    let target = original_target(data, alternative).map_err(|_| missing)?;
    let signature = data.signatures.get(variant.signature).ok_or(missing)?;
    let callable = data
        .callables
        .get(variant.callable.ok_or(missing)?)
        .ok_or(missing)?;
    establish(Application {
        target,
        qualification: data
            .qualifications
            .get(target.qualification)
            .ok_or(missing)?,
        destination: data.destinations.get(target.destination).ok_or(missing)?,
        channel: data.channels.get(target.channel).ok_or(missing)?,
        receiver: data.receivers.get(target.receiver).ok_or(missing)?,
        receiver_proof: receivers.get(target.id()),
        dispatch_proof: events.dispatch(alternative.id()),
        signature,
        signature_qualification: data
            .qualifications
            .get(signature.qualification)
            .ok_or(missing)?,
        call,
        call_qualification: data.qualifications.get(call.qualification).ok_or(missing)?,
        target_resolution: data
            .symbol_resolutions
            .get(alternative.correspondence.ok_or(missing)?)
            .ok_or(missing)?,
        signature_resolution: data
            .symbol_resolutions
            .get(variant.resolution)
            .ok_or(missing)?,
        entity: data
            .refs
            .get(alternative.entity.ok_or(missing)?)
            .ok_or(missing)?,
        callable,
        variant,
        effective: variant
            .assessment
            .and_then(|id| data.callable_assessments.get(id)),
        scopes: scopes(data),
    })
}
fn bind_application(
    data: &BindingData,
    index: &Index<'_>,
    application: &ApplicableSignature<'_>,
    charge: &mut StateCharge,
) -> Result<Result<BoundCall, BindingFailure>, ModelError> {
    let raw = application.raw();
    let parameters = index
        .parameters
        .get(&raw.signature.id())
        .map(Vec::as_slice)
        .unwrap_or(&[]);
    let arguments = index
        .arguments
        .get(&raw.call.id())
        .map(Vec::as_slice)
        .unwrap_or(&[]);
    // The binder's bounded vectors/sets and owned strings live only during this invocation.
    charge.grow((parameters.len() + arguments.len() + 1).saturating_mul(1024))?;
    let mut shapes = BTreeMap::new();
    for parameter in parameters {
        let shape = need(&data.shapes, parameter.shape)?;
        charge.admit(shape)?;
        shapes.insert(shape.id(), shape.clone());
    }
    Ok(bind(BindingInput {
        application,
        parameters,
        shapes: &shapes,
        arguments,
    }))
}
#[allow(
    clippy::too_many_arguments,
    reason = "One binding attempt threads the alternative, variant, syntax, output and verified receiver/event sources"
)]
fn attempt(
    data: &BindingData,
    index: &Index<'_>,
    alternative: &NormalizedCallAlternative,
    variant: Option<&SignatureVariant>,
    syntax: Option<&CallSyntax>,
    output: &mut BindingOutput,
    receivers: &super::receiver::VerifiedReceivers,
    events: &super::event_normalization::VerifiedEvents,
    budget: &ResourceBudget,
) -> Result<Id<CallBindingAttempt>, ModelError> {
    let mut work = StateCharge::new(budget, "binding-work");
    let target = original_target(data, alternative)?;
    let mut row = CallBindingAttempt {
        alternative: alternative.id(),
        variant: variant.map(Record::id),
        syntax: syntax.map(Record::id),
        policy: policy_revision(),
        event: alternative.event,
        signature: variant.map(|v| v.signature),
        arguments: syntax.map(|s| s.arguments),
        receiver: target.receiver,
        receiver_assessment: receivers.get(target.id()).map(|p| p.assessment()),
        dispatch_member: events.dispatch(alternative.id()).map(|p| p.member()),
        effective: variant.and_then(|v| v.assessment),
        adjustment: variant.map_or(SignatureAdjustment::Unknown, |v| v.adjustment),
        authority: BindingAuthority::SourceInspection,
        authority_reason: AuthorityReason::EffectiveUnknown,
        outcome: BindingOutcome::Undetermined,
        reason: BindingReason::MissingSignature,
        refusal: Some(ObligationKind::MissingEvidence),
        bindings: digest(&[]),
    };
    let mut bound = None;
    if let Some(variant) = variant {
        row.reason = BindingReason::MissingSyntax;
        if let Some(syntax) = syntax {
            // Explicit syntax cannot be reused for a protocol/desugaring event at the same site.
            if target.origin != CallOrigin::explicit() {
                row.reason = BindingReason::ImplicitEvent;
            } else {
                match application(data, alternative, variant, syntax, receivers, events) {
                    Err(reason) => {
                        row.reason = BindingReason::UnprovedApplicability;
                        row.refusal = Some(reason);
                    }
                    Ok(application) => {
                        row.authority = application.authority();
                        row.authority_reason = application.reason();
                        match bind_application(data, index, &application, &mut work)? {
                            Ok(value) => {
                                row.outcome = BindingOutcome::Bound;
                                row.reason = BindingReason::Bound;
                                row.refusal = None;
                                row.bindings = digest(value.bindings());
                                bound = Some(value);
                            }
                            Err(failure) => {
                                row.outcome = match failure.class {
                                    BindingFailureClass::ProvenIncompatible => {
                                        BindingOutcome::ProvenIncompatible
                                    }
                                    BindingFailureClass::Undetermined => {
                                        BindingOutcome::Undetermined
                                    }
                                };
                                row.reason = if row.outcome == BindingOutcome::ProvenIncompatible {
                                    BindingReason::ArgumentMismatch
                                } else {
                                    BindingReason::UnsupportedShape
                                };
                                row.refusal = Some(failure.reason);
                            }
                        }
                    }
                }
            }
        }
    }
    let id = output.attempts.insert(row)?;
    if let Some(bound) = bound {
        for (ordinal, binding) in bound.bindings().iter().enumerate() {
            let slot = index
                .slots
                .get(&binding.formal)
                .ok_or_else(|| invalid("bound formal has no normalized slot"))?;
            if Some(slot.variant) != variant.map(Record::id) {
                return Err(invalid("bound slot belongs to another variant"));
            }
            let source = output.sources.insert(binding.source.clone())?;
            let projection = output.projections.insert(binding.projection.clone())?;
            output.bindings.insert(CallBinding {
                attempt: id,
                ordinal: ordinal as i64,
                slot: slot.id(),
                source,
                kind: binding.kind,
                projection,
            })?;
        }
    }
    Ok(id)
}
pub fn normalize(data: &BindingData, budget: &ResourceBudget) -> Result<BindingOutput, ModelError> {
    let receivers = receiver_proofs(data, budget)?;
    let events = event_proofs(data, budget)?;
    normalize_with(data, &receivers, &events, budget)
}
/// Produce bindings and their private application authority in the same owner operation.
/// Only the compact authority survives after the scoped premises and emitted rows are dropped.
pub fn normalize_prepared(
    data: &BindingData,
    budget: &ResourceBudget,
) -> Result<(BindingOutput, VerifiedBindings), ModelError> {
    let receivers = receiver_proofs(data, budget)?;
    let events = event_proofs(data, budget)?;
    let output = normalize_with(data, &receivers, &events, budget)?;
    verify_enumerations(data, budget)?;
    let verified = admit(data, &output, &receivers, &events, budget)?;
    Ok((output, verified))
}
/// The owning compiler lends compact receiver/event admissions once for each complete event grain.
/// No stored receiver or event assessment creates authority on this route.
pub fn normalize_event_produced(
    data: &BindingData,
    selected: Id<NormalizedCallEvent>,
    receivers: &super::receiver::VerifiedReceivers,
    events: &super::event_normalization::VerifiedEvents,
    budget: &ResourceBudget,
) -> Result<(BindingOutput, VerifiedBindings), ModelError> {
    need(&data.event_events, selected)?;
    if data
        .event_alternatives
        .iter()
        .any(|alternative| alternative.event != selected)
    {
        return Err(invalid(
            "binding grain contains another event candidate domain",
        ));
    }
    normalize_produced(data, receivers, events, budget)
}
pub fn normalize_produced(
    data: &BindingData,
    receivers: &super::receiver::VerifiedReceivers,
    events: &super::event_normalization::VerifiedEvents,
    budget: &ResourceBudget,
) -> Result<(BindingOutput, VerifiedBindings), ModelError> {
    let output = normalize_with(data, receivers, events, budget)?;
    verify_enumerations(data, budget)?;
    let verified = admit(data, &output, receivers, events, budget)?;
    Ok((output, verified))
}
fn normalize_with(
    data: &BindingData,
    receivers: &super::receiver::VerifiedReceivers,
    events: &super::event_normalization::VerifiedEvents,
    budget: &ResourceBudget,
) -> Result<BindingOutput, ModelError> {
    let index = Index::new(data, budget)?;
    let mut output = BindingOutput::new(budget);
    let mut charge = StateCharge::new(budget, "binding-set-index");
    let mut sets: ChargedMap<SetKey, Vec<Id<CallBindingAttempt>>> = Default::default();
    for alternative in data.event_alternatives.iter() {
        let event = need(&data.event_events, alternative.event)?;
        let target = original_target(data, alternative)?;
        let variants = match alternative.entity.and_then(|id| data.refs.get(id)) {
            Some(EntityRef::Callable { callable }) => {
                index.variants.get(&(*callable, event.context))
            }
            _ => need(&data.destinations, target.destination)?
                .symbol()
                .and_then(|s| index.raw_variants.get(&s)),
        };
        let syntax = index.syntax.get(&(event.site, event.context));
        let key = (event.id(), alternative.entity, target.phase, target.channel);
        for v in 0..variants.map_or(1, |v| v.len().max(1)) {
            for s in 0..syntax.map_or(1, |v| v.len().max(1)) {
                let id = attempt(
                    data,
                    &index,
                    alternative,
                    variants.and_then(|vs| vs.get(v).copied()),
                    syntax.and_then(|ss| ss.get(s).copied()),
                    &mut output,
                    receivers,
                    events,
                    budget,
                )?;
                sets.update(&mut charge, key, |ids| ids.push(id))?;
            }
        }
    }
    for (key, attempts) in sets.iter() {
        assess_set(data, &index, *key, attempts, &mut output, budget)?;
    }
    Ok(output)
}
fn assess_set(
    data: &BindingData,
    index: &Index<'_>,
    key: SetKey,
    attempts: &[Id<CallBindingAttempt>],
    output: &mut BindingOutput,
    budget: &ResourceBudget,
) -> Result<(), ModelError> {
    let mut charge = StateCharge::new(budget, "binding-variant-set");
    let mut variants: ChargedMap<Option<Id<SignatureVariant>>, Vec<&CallBindingAttempt>> =
        Default::default();
    for id in attempts {
        let row = need(&output.attempts, *id)?;
        variants.update(&mut charge, row.variant, |vs| vs.push(row))?;
    }
    let mut assessments = Vec::new();
    let mut whole = KeySink::new("binding-variant-set");
    let mut counts = [0i64; 3];
    let mut complete = true;
    let mut coverage: ChargedSet<Id<ProviderCoverage>> = Default::default();
    for (variant, rows) in variants.iter() {
        let mut members = KeySink::new("binding-variant-attempts");
        let first = rows[0];
        let mut outcome = first.outcome;
        for row in rows {
            row.id().encode(&mut members);
            row.bindings.encode(&mut members);
            row.outcome.encode(&mut members);
            row.authority.encode(&mut members);
            row.authority_reason.encode(&mut members);
            row.signature.encode(&mut members);
            row.arguments.encode(&mut members);
            row.receiver.encode(&mut members);
            row.receiver_assessment.encode(&mut members);
            row.dispatch_member.encode(&mut members);
            row.effective.encode(&mut members);
            row.adjustment.encode(&mut members);
            row.reason.encode(&mut members);
            row.refusal.encode(&mut members);
            if row.authority != BindingAuthority::EffectiveInvocation
                || row.outcome != first.outcome
                || row.bindings != first.bindings
                || row.syntax != first.syntax
            {
                outcome = BindingOutcome::Undetermined;
            }
        }
        let members = members.finish();
        variant.encode(&mut whole);
        outcome.encode(&mut whole);
        members.encode(&mut whole);
        counts[outcome.code() as usize] += 1;
        if let Some(v) = variant {
            let raw = need(
                &data.signatures,
                need(&data.callable_variants, *v)?.signature,
            )?;
            let symbol = need(&data.symbols, raw.symbol)?;
            let supports = index
                .supports
                .get(&raw.id())
                .map(Vec::as_slice)
                .unwrap_or(&[]);
            if supports.is_empty() {
                complete = false;
            }
            for support in supports {
                let mut found = false;
                for c in data.coverage.iter().filter(|c| {
                    c.family == FactFamily::Signatures
                        && c.context == symbol.context
                        && c.provider == Some(symbol.provider)
                        && c.scope == raw.scope
                        && c.run == Some(support.run)
                }) {
                    found = true;
                    complete &= c.status == CoverageStatus::CompleteUnderStatedModel;
                    coverage.insert(&mut charge, c.id())?;
                }
                complete &= found;
            }
        } else {
            complete = false;
        }
        charge.grow(128)?;
        assessments.push((*variant, outcome, members));
    }
    for id in coverage.iter() {
        id.encode(&mut whole);
        need(&data.coverage, *id)?.status.encode(&mut whole);
    }
    let unique = complete && counts[0] == 1 && counts[2] == 0;
    let reason = if !complete {
        BindingSetReason::IncompleteCoverage
    } else if counts[2] > 0 {
        BindingSetReason::UndeterminedVariant
    } else if counts[0] == 0 {
        BindingSetReason::NoBoundVariant
    } else if counts[0] > 1 {
        BindingSetReason::MultipleBoundVariants
    } else {
        BindingSetReason::Unique
    };
    let set = output.sets.insert(BindingSetAssessment {
        event: key.0,
        entity: key.1,
        phase: key.2,
        channel: key.3,
        policy: policy_revision(),
        members: whole.finish(),
        bound: counts[0],
        incompatible: counts[1],
        undetermined: counts[2],
        coverage_complete: complete,
        unique,
        reason,
    })?;
    for (variant, outcome, members) in assessments {
        let assessment = output.variants.insert(BindingVariantAssessment {
            set,
            variant,
            outcome,
            members,
        })?;
        for row in &variants[&variant] {
            output.members.insert(BindingSetMember {
                variant: assessment,
                attempt: row.id(),
            })?;
        }
    }
    for coverage in coverage.iter() {
        output.coverage.insert(BindingSetCoverage {
            set,
            coverage: *coverage,
        })?;
    }
    Ok(())
}
/// A whole successful shape replay. No constructor accepts stored binding members.
pub struct ValidatedBoundCall {
    attempt: Id<CallBindingAttempt>,
    event: Id<NormalizedCallEvent>,
    context: Id<AnalysisContext>,
    bound: BoundCall,
    bindings: ContentHash,
}
impl ValidatedBoundCall {
    pub fn event(&self) -> Id<NormalizedCallEvent> {
        self.event
    }
    pub fn context(&self) -> Id<AnalysisContext> {
        self.context
    }
    pub fn attempt(&self) -> Id<CallBindingAttempt> {
        self.attempt
    }
    pub fn bound(&self) -> &BoundCall {
        &self.bound
    }
}
impl HeapSize for ValidatedBoundCall {
    fn heap_bytes(&self) -> usize {
        self.bound.heap_bytes()
    }
}
/// A replayed source binding for an alternative-specific typing question. This capability
/// proves neither event completeness nor runtime invocation or effects.
pub struct SourceBindingShape {
    attempt: Id<CallBindingAttempt>,
    event: Id<NormalizedCallEvent>,
    context: Id<AnalysisContext>,
    input: Id<input::InputRevision>,
    owner_entity: Id<EntityRef>,
    owner_declaration: Id<Occurrence>,
    target: Id<CallTarget>,
    phase: CallPhase,
    bindings: ContentHash,
}
impl SourceBindingShape {
    pub fn context(&self) -> Id<AnalysisContext> {
        self.context
    }
    pub fn input(&self) -> Id<input::InputRevision> {
        self.input
    }
    pub fn owner_entity(&self) -> Id<EntityRef> {
        self.owner_entity
    }
    pub fn owner_declaration(&self) -> Id<Occurrence> {
        self.owner_declaration
    }
    pub fn target(&self) -> Id<CallTarget> {
        self.target
    }
    pub fn phase(&self) -> CallPhase {
        self.phase
    }
    pub fn admits(&self, bound: &ValidatedBoundCall) -> bool {
        self.attempt == bound.attempt
            && self.event == bound.event
            && self.context == bound.context
            && self.target == bound.bound.target()
            && self.bindings == bound.bindings
    }
}
impl HeapSize for SourceBindingShape {}
/// Complete native event and one compatible signature shape. This does not certify the
/// runtime callable identity, its body, definition-time defaults or Summary policy.
/// Model applicability must independently supply an exact authored runtime contract.
pub struct BindingShapeAdmission {
    attempt: Id<CallBindingAttempt>,
    set: Id<BindingSetAssessment>,
    set_members: ContentHash,
    alternative: Id<NormalizedCallAlternative>,
    event: Id<NormalizedCallEvent>,
    context: Id<AnalysisContext>,
    input: Id<input::InputRevision>,
    complete: Id<EventAssessment>,
    event_members: ContentHash,
    owner: Id<OccurrenceOwnership>,
    owner_entity: Id<EntityRef>,
    owner_declaration: Id<Occurrence>,
    callee: Id<EntityRef>,
    target: Id<CallTarget>,
    phase: CallPhase,
    bindings: ContentHash,
    enumeration: Option<Id<SignatureEnumerationObservation>>,
    signature_members: Option<ContentHash>,
}
impl BindingShapeAdmission {
    pub fn attempt(&self) -> Id<CallBindingAttempt> {
        self.attempt
    }
    pub fn set(&self) -> Id<BindingSetAssessment> {
        self.set
    }
    pub fn set_members(&self) -> ContentHash {
        self.set_members
    }
    pub fn alternative(&self) -> Id<NormalizedCallAlternative> {
        self.alternative
    }
    pub fn event(&self) -> Id<NormalizedCallEvent> {
        self.event
    }
    pub fn context(&self) -> Id<AnalysisContext> {
        self.context
    }
    pub fn input(&self) -> Id<input::InputRevision> {
        self.input
    }
    pub fn complete(&self) -> Id<EventAssessment> {
        self.complete
    }
    pub fn event_members(&self) -> ContentHash {
        self.event_members
    }
    pub fn owner(&self) -> Id<OccurrenceOwnership> {
        self.owner
    }
    pub fn owner_entity(&self) -> Id<EntityRef> {
        self.owner_entity
    }
    pub fn owner_declaration(&self) -> Id<Occurrence> {
        self.owner_declaration
    }
    pub fn callee(&self) -> Id<EntityRef> {
        self.callee
    }
    pub fn target(&self) -> Id<CallTarget> {
        self.target
    }
    pub fn phase(&self) -> CallPhase {
        self.phase
    }
    pub fn bindings(&self) -> ContentHash {
        self.bindings
    }
    pub fn enumeration(&self) -> Option<Id<SignatureEnumerationObservation>> {
        self.enumeration
    }
    pub fn signature_members(&self) -> Option<ContentHash> {
        self.signature_members
    }
    pub fn admits(&self, bound: &ValidatedBoundCall) -> bool {
        self.attempt == bound.attempt
            && self.event == bound.event
            && self.context == bound.context
            && self.target == bound.bound.target()
            && self.bindings == bound.bindings
    }
}
impl HeapSize for BindingShapeAdmission {}
/// A complete, uniquely bound effective invocation. Source-body admission and the Summary
/// policy are separate capabilities. External authored runtime contracts consume the structural
/// BindingShapeAdmission instead and independently establish their Model-specific identity.
/// Only full upstream and stored-binding replay in `verify` can construct it.
pub struct EffectiveInvocationAdmission {
    attempt: Id<CallBindingAttempt>,
    set: Id<BindingSetAssessment>,
    set_members: ContentHash,
    alternative: Id<NormalizedCallAlternative>,
    event: Id<NormalizedCallEvent>,
    context: Id<AnalysisContext>,
    input: Id<input::InputRevision>,
    complete: Id<EventAssessment>,
    event_members: ContentHash,
    owner: Id<OccurrenceOwnership>,
    owner_entity: Id<EntityRef>,
    owner_declaration: Id<Occurrence>,
    callee: Id<EntityRef>,
    effective: Id<EffectiveCallableAssessment>,
    target: Id<CallTarget>,
    phase: CallPhase,
    bindings: ContentHash,
}
impl EffectiveInvocationAdmission {
    pub fn attempt(&self) -> Id<CallBindingAttempt> {
        self.attempt
    }
    pub fn set(&self) -> Id<BindingSetAssessment> {
        self.set
    }
    pub fn set_members(&self) -> ContentHash {
        self.set_members
    }
    pub fn alternative(&self) -> Id<NormalizedCallAlternative> {
        self.alternative
    }
    pub fn event(&self) -> Id<NormalizedCallEvent> {
        self.event
    }
    pub fn context(&self) -> Id<AnalysisContext> {
        self.context
    }
    pub fn input(&self) -> Id<input::InputRevision> {
        self.input
    }
    pub fn complete(&self) -> Id<EventAssessment> {
        self.complete
    }
    pub fn event_members(&self) -> ContentHash {
        self.event_members
    }
    pub fn owner(&self) -> Id<OccurrenceOwnership> {
        self.owner
    }
    pub fn owner_entity(&self) -> Id<EntityRef> {
        self.owner_entity
    }
    pub fn owner_declaration(&self) -> Id<Occurrence> {
        self.owner_declaration
    }
    pub fn callee(&self) -> Id<EntityRef> {
        self.callee
    }
    pub fn effective(&self) -> Id<EffectiveCallableAssessment> {
        self.effective
    }
    pub fn target(&self) -> Id<CallTarget> {
        self.target
    }
    pub fn phase(&self) -> CallPhase {
        self.phase
    }
    pub fn bindings(&self) -> ContentHash {
        self.bindings
    }
    /// Both receipts must pin the same replayed binding payload, not merely the attempt key.
    pub fn admits(&self, bound: &ValidatedBoundCall) -> bool {
        self.attempt == bound.attempt
            && self.event == bound.event
            && self.context == bound.context
            && self.target == bound.bound.target()
            && self.bindings == bound.bindings
    }
}
impl HeapSize for EffectiveInvocationAdmission {
    fn heap_bytes(&self) -> usize {
        0
    }
}
/// Signature closure for an independently admitted source body. Selected enumeration
/// closure does not upgrade the artifact's signature family or effective invocation token.
#[derive(Debug, Clone, Copy)]
pub enum SourceBodySignatureClosure {
    GlobalCoverage {
        members: ContentHash,
    },
    DeclaredEnumeration {
        enumeration: Id<SignatureEnumerationObservation>,
        members: ContentHash,
        support: Id<SignatureEnumerationSupport>,
    },
}
/// An admitted source body and complete selected signature domain, after event replay.
pub struct CompositionAdmission {
    attempt: Id<CallBindingAttempt>,
    set: Id<BindingSetAssessment>,
    event: Id<NormalizedCallEvent>,
    complete: Id<EventAssessment>,
    event_members: ContentHash,
    summary: Id<CallPolicyAssessment>,
    signature_closure: SourceBodySignatureClosure,
    owner: Id<OccurrenceOwnership>,
    owner_entity: Id<EntityRef>,
    owner_declaration: Id<Occurrence>,
    callee: Id<EntityRef>,
    effective: Id<EffectiveCallableAssessment>,
    target: Id<CallTarget>,
    phase: CallPhase,
}
impl CompositionAdmission {
    pub fn signature_closure(&self) -> SourceBodySignatureClosure {
        self.signature_closure
    }
    pub fn attempt(&self) -> Id<CallBindingAttempt> {
        self.attempt
    }
    pub fn set(&self) -> Id<BindingSetAssessment> {
        self.set
    }
    pub fn event(&self) -> Id<NormalizedCallEvent> {
        self.event
    }
    pub fn complete(&self) -> Id<EventAssessment> {
        self.complete
    }
    pub fn event_members(&self) -> ContentHash {
        self.event_members
    }
    pub fn summary(&self) -> Id<CallPolicyAssessment> {
        self.summary
    }
    pub fn owner(&self) -> Id<OccurrenceOwnership> {
        self.owner
    }
    pub fn owner_entity(&self) -> Id<EntityRef> {
        self.owner_entity
    }
    pub fn owner_declaration(&self) -> Id<Occurrence> {
        self.owner_declaration
    }
    pub fn callee(&self) -> Id<EntityRef> {
        self.callee
    }
    pub fn effective(&self) -> Id<EffectiveCallableAssessment> {
        self.effective
    }
    pub fn target(&self) -> Id<CallTarget> {
        self.target
    }
    pub fn phase(&self) -> CallPhase {
        self.phase
    }
}
impl HeapSize for CompositionAdmission {
    fn heap_bytes(&self) -> usize {
        0
    }
}
/// Compact native enumeration premises admitted by this binding owner. Constructor metadata
/// can include initializer alternatives that never produce an ordinary bound call.
#[derive(Default)]
struct EnumerationAuthority {
    qualifications: ChargedMap<Id<crate::domain::assertion::AssertionQualification>, ContentHash>,
    symbols: ChargedMap<Id<ProviderSymbol>, ContentHash>,
    signatures: ChargedMap<Id<Signature>, ContentHash>,
    observations:
        ChargedMap<Id<SignatureEnumerationObservation>, (ContentHash, Id<ProviderSymbol>)>,
    members: ChargedMap<Id<SignatureEnumerationMember>, ContentHash>,
    member_counts: ChargedMap<Id<SignatureEnumerationObservation>, usize>,
    symbol_counts: ChargedMap<Id<ProviderSymbol>, usize>,
}
impl EnumerationAuthority {
    fn capture(data: &BindingData, charge: &mut StateCharge) -> Result<Self, ModelError> {
        let mut result = Self::default();
        for row in data.signature_enumerations.iter() {
            result
                .observations
                .insert(charge, row.id(), (row.content_digest(), row.symbol))?;
            let q = need(&data.qualifications, row.qualification)?;
            result
                .qualifications
                .insert(charge, q.id(), q.content_digest())?;
            let symbol = need(&data.symbols, row.symbol)?;
            result
                .symbols
                .insert(charge, symbol.id(), symbol.content_digest())?;
            result.member_counts.insert(charge, row.id(), 0)?;
            result
                .symbol_counts
                .update(charge, row.symbol, |n| *n += 1)?;
        }
        for member in data.signature_enumeration_members.iter() {
            result
                .members
                .insert(charge, member.id(), member.content_digest())?;
            result
                .member_counts
                .update(charge, member.enumeration, |n| *n += 1)?;
            let signature = need(&data.signatures, member.signature)?;
            result
                .signatures
                .insert(charge, signature.id(), signature.content_digest())?;
            let q = need(&data.qualifications, signature.qualification)?;
            result
                .qualifications
                .insert(charge, q.id(), q.content_digest())?;
            let symbol = need(&data.symbols, signature.symbol)?;
            result
                .symbols
                .insert(charge, symbol.id(), symbol.content_digest())?;
        }
        Ok(result)
    }
    fn append(
        &mut self,
        other: &mut Self,
        charge: &mut StateCharge,
        other_charge: &mut StateCharge,
    ) -> Result<(), ModelError> {
        macro_rules! join {
            ($field:ident) => {
                while let Some(key) = other.$field.keys().next().copied() {
                    let value = other
                        .$field
                        .remove(other_charge, &key)
                        .expect("enumeration predecessor entry");
                    if let Some(old) = self.$field.get(&key) {
                        if old != &value {
                            return Err(ModelError::Conflict("enumeration predecessor changed"));
                        }
                    } else {
                        self.$field.insert(charge, key, value)?;
                    }
                }
            };
        }
        join!(qualifications);
        join!(symbols);
        join!(signatures);
        join!(members);
        join!(member_counts);
        while let Some(key) = other.observations.keys().next().copied() {
            let value = other
                .observations
                .remove(other_charge, &key)
                .expect("enumeration predecessor observation");
            if let Some(old) = self.observations.get(&key) {
                if old != &value {
                    return Err(ModelError::Conflict("enumeration predecessor changed"));
                }
            } else {
                self.observations.insert(charge, key, value)?;
                self.symbol_counts.update(charge, value.1, |n| *n += 1)?;
            }
        }
        Ok(())
    }
    fn require(&self, data: &BindingData, budget: &ResourceBudget) -> Result<(), ModelError> {
        fn row<R: Record>(
            index: &ChargedMap<Id<R>, ContentHash>,
            row: &R,
        ) -> Result<(), ModelError> {
            if index.get(&row.id()) != Some(&row.content_digest()) {
                return Err(ModelError::Conflict(
                    "enumeration predecessor membership/content",
                ));
            }
            Ok(())
        }
        let count = data
            .signature_enumerations
            .len()
            .checked_add(data.signature_enumeration_members.len())
            .ok_or_else(|| invalid("enumeration predecessor allowance"))?;
        let _scratch = budget.reserve(
            "construction-enumeration-subset",
            count
                .checked_mul(256)
                .ok_or_else(|| invalid("enumeration predecessor allowance"))?,
        )?;
        let mut members = BTreeMap::<Id<SignatureEnumerationObservation>, usize>::new();
        let mut symbols = BTreeMap::<Id<ProviderSymbol>, usize>::new();
        for enumeration in data.signature_enumerations.iter() {
            if self.observations.get(&enumeration.id()).map(|row| row.0)
                != Some(enumeration.content_digest())
            {
                return Err(ModelError::Conflict(
                    "enumeration predecessor membership/content",
                ));
            }
            row(
                &self.qualifications,
                need(&data.qualifications, enumeration.qualification)?,
            )?;
            row(&self.symbols, need(&data.symbols, enumeration.symbol)?)?;
            members.insert(enumeration.id(), 0);
            *symbols.entry(enumeration.symbol).or_default() += 1;
        }
        for member in data.signature_enumeration_members.iter() {
            row(&self.members, member)?;
            *members
                .get_mut(&member.enumeration)
                .ok_or(ModelError::Conflict(
                    "constructor member has no enumeration",
                ))? += 1;
            let signature = need(&data.signatures, member.signature)?;
            row(&self.signatures, signature)?;
            row(
                &self.qualifications,
                need(&data.qualifications, signature.qualification)?,
            )?;
            row(&self.symbols, need(&data.symbols, signature.symbol)?)?;
        }
        if members
            .iter()
            .any(|(id, count)| self.member_counts.get(id) != Some(count))
            || symbols
                .iter()
                .any(|(id, count)| self.symbol_counts.get(id) != Some(count))
        {
            return Err(ModelError::Conflict(
                "constructor enumeration alternatives incomplete",
            ));
        }
        Ok(())
    }
}
pub struct VerifiedBindings {
    enumerations: EnumerationAuthority,
    bound: ChargedMap<Id<CallBindingAttempt>, ValidatedBoundCall>,
    shape: ChargedMap<Id<CallBindingAttempt>, BindingShapeAdmission>,
    source_shape: ChargedMap<Id<CallBindingAttempt>, SourceBindingShape>,
    effective: ChargedMap<Id<CallBindingAttempt>, EffectiveInvocationAdmission>,
    composition: ChargedMap<Id<CallBindingAttempt>, CompositionAdmission>,
    _charge: StateCharge,
}
impl VerifiedBindings {
    pub fn append(&mut self, mut other: Self) -> Result<(), ModelError> {
        if !self
            ._charge
            .budget()
            .expect("owner budget")
            .shares_pool(other._charge.budget().expect("owner budget"))
        {
            return Err(ModelError::Conflict("binding authority budget"));
        }
        macro_rules! join {
            ($field:ident) => {{
                while let Some(key) = other.$field.keys().next().copied() {
                    if self.$field.contains_key(&key) {
                        return Err(ModelError::Conflict("binding authority domain"));
                    }
                    let value = other
                        .$field
                        .remove(&mut other._charge, &key)
                        .expect("selected owner entry");
                    self.$field.insert(&mut self._charge, key, value)?;
                }
            }};
        }
        self.enumerations.append(
            &mut other.enumerations,
            &mut self._charge,
            &mut other._charge,
        )?;
        join!(bound);
        join!(shape);
        join!(source_shape);
        join!(effective);
        join!(composition);
        Ok(())
    }

    /// Admit a complete native symbol enumeration grain, including constructor alternatives
    /// that have no ordinary normalized call event. The caller binds these rows to the same
    /// immutable predecessor streams; membership/content checks remain exact when borrowed.
    pub fn admit_enumerations(
        &mut self,
        data: &BindingData,
        budget: &ResourceBudget,
    ) -> Result<(), ModelError> {
        if !self
            ._charge
            .budget()
            .expect("binding owner budget")
            .shares_pool(budget)
        {
            return Err(ModelError::Conflict("enumeration predecessor foreign budget"));
        }
        verify_enumerations(data, budget)?;
        let mut charge = StateCharge::new(budget, "binding-native-enumeration-grain");
        let mut admitted = EnumerationAuthority::capture(data, &mut charge)?;
        self.enumerations
            .append(&mut admitted, &mut self._charge, &mut charge)
    }

    pub(crate) fn require_enumerations(
        &self,
        data: &BindingData,
        budget: &ResourceBudget,
    ) -> Result<(), ModelError> {
        if !self
            ._charge
            .budget()
            .expect("binding owner budget")
            .shares_pool(budget)
        {
            return Err(ModelError::Conflict(
                "constructor enumeration foreign budget",
            ));
        }
        self.enumerations.require(data, budget)
    }
    pub fn bound(&self, attempt: Id<CallBindingAttempt>) -> Option<&ValidatedBoundCall> {
        self.bound.get(&attempt)
    }
    pub fn shape(&self, attempt: Id<CallBindingAttempt>) -> Option<&BindingShapeAdmission> {
        self.shape.get(&attempt)
    }
    pub fn source_shape(&self, attempt: Id<CallBindingAttempt>) -> Option<&SourceBindingShape> {
        self.source_shape.get(&attempt)
    }
    pub fn effective_invocation(
        &self,
        attempt: Id<CallBindingAttempt>,
    ) -> Option<&EffectiveInvocationAdmission> {
        self.effective.get(&attempt)
    }
    pub fn composition(&self, attempt: Id<CallBindingAttempt>) -> Option<&CompositionAdmission> {
        self.composition.get(&attempt)
    }
}
pub fn verify(
    data: &BindingData,
    stored: &BindingOutput,
    budget: &ResourceBudget,
) -> Result<VerifiedBindings, ModelError> {
    stored.matches(&normalize(data, budget)?)?;
    verify_enumerations(data, budget)?;
    let events = verify_upstream(data, budget)?;
    let receivers = receiver_proofs(data, budget)?;
    admit(data, stored, &receivers, &events, budget)
}
/// Prepare the exact bound/shape/composition consumer authority over completed inputs. This
/// checks applicability and the actual bound members without replaying any predecessor owner.
/// The compiler additionally carries admission of immutable predecessor streams in its attempt.
pub fn admit_event(
    data: &BindingData,
    stored: &BindingOutput,
    selected: Id<NormalizedCallEvent>,
    budget: &ResourceBudget,
) -> Result<(), ModelError> {
    need(&data.event_events, selected)?;
    if data
        .event_alternatives
        .iter()
        .any(|alternative| alternative.event != selected)
        || stored
            .attempts
            .iter()
            .any(|attempt| attempt.event != selected)
    {
        return Err(invalid(
            "binding admission grain contains another event root",
        ));
    }
    prepare(data, stored, budget).map(|_| ())
}
pub fn prepare(
    data: &BindingData,
    stored: &BindingOutput,
    budget: &ResourceBudget,
) -> Result<VerifiedBindings, ModelError> {
    verify_enumerations(data, budget)?;
    let events = event_proofs(data, budget)?;
    let receivers = receiver_proofs(data, budget)?;
    admit(data, stored, &receivers, &events, budget)
}
fn admit(
    data: &BindingData,
    stored: &BindingOutput,
    receivers: &super::receiver::VerifiedReceivers,
    events: &super::event_normalization::VerifiedEvents,
    budget: &ResourceBudget,
) -> Result<VerifiedBindings, ModelError> {
    let index = Index::new(data, budget)?;
    admit_attempt_domain(data, stored, &index)?;
    // Proven incompatibility can eliminate an alternative from a unique binding shape. It is
    // therefore an eligibility premise, not a stored status that may be trusted on its own.
    for row in stored
        .attempts
        .iter()
        .filter(|row| row.outcome == BindingOutcome::ProvenIncompatible)
    {
        let alternative = need(&data.event_alternatives, row.alternative)?;
        let variant = need(
            &data.callable_variants,
            row.variant
                .ok_or_else(|| invalid("incompatible attempt has no signature variant"))?,
        )?;
        let syntax = need(
            &data.syntax,
            row.syntax
                .ok_or_else(|| invalid("incompatible attempt has no syntax"))?,
        )?;
        let application = application(data, alternative, variant, syntax, receivers, events)
            .map_err(|_| invalid("incompatible attempt has no applicable signature"))?;
        let mut work = StateCharge::new(budget, "incompatible-binding-admission");
        match bind_application(data, &index, &application, &mut work)? {
            Err(failure)
                if failure.class == BindingFailureClass::ProvenIncompatible
                    && row.refusal == Some(failure.reason) => {}
            _ => {
                return Err(invalid(
                    "stored incompatibility does not eliminate an actual binding",
                ));
            }
        }
    }
    admit_set_predicates(data, stored, &index, budget)?;
    let mut result = VerifiedBindings {
        enumerations: EnumerationAuthority::default(),
        bound: Default::default(),
        shape: Default::default(),
        source_shape: Default::default(),
        effective: Default::default(),
        composition: Default::default(),
        _charge: StateCharge::new(budget, "validated-bindings"),
    };
    result.enumerations = EnumerationAuthority::capture(data, &mut result._charge)?;
    let mut charge = StateCharge::new(budget, "composition-admission-index");
    let mut member_sets: ChargedMap<Id<CallBindingAttempt>, &BindingSetAssessment> =
        Default::default();
    for member in stored.members.iter() {
        let variant = need(&stored.variants, member.variant)?;
        let set = need(&stored.sets, variant.set)?;
        if variant.outcome == BindingOutcome::Bound && set.unique {
            member_sets.insert(&mut charge, member.attempt, set)?;
        }
    }
    for row in stored
        .attempts
        .iter()
        .filter(|a| a.outcome == BindingOutcome::Bound)
    {
        let alternative = need(&data.event_alternatives, row.alternative)?;
        let variant = need(
            &data.callable_variants,
            row.variant
                .ok_or_else(|| invalid("bound attempt has no variant"))?,
        )?;
        let syntax = need(
            &data.syntax,
            row.syntax
                .ok_or_else(|| invalid("bound attempt has no syntax"))?,
        )?;
        let application = application(data, alternative, variant, syntax, receivers, events)
            .map_err(|_| invalid("stored binding applicability changed"))?;
        let mut work = StateCharge::new(budget, "binding-replay-work");
        let bound = bind_application(data, &index, &application, &mut work)?
            .map_err(|_| invalid("stored binding replay refused"))?;
        let target = original_target(data, alternative)?;
        if row.policy != policy_revision()
            || row.event != alternative.event
            || row.receiver != target.receiver
            || row.receiver_assessment != receivers.get(target.id()).map(|proof| proof.assessment())
            || row.dispatch_member
                != events
                    .dispatch(alternative.id())
                    .map(|proof| proof.member())
            || row.effective != variant.assessment
            || row.adjustment != variant.adjustment
            || row.reason != BindingReason::Bound
            || row.refusal.is_some()
            || row.signature != Some(variant.signature)
            || row.arguments != Some(syntax.arguments)
            || row.authority != application.authority()
            || row.authority_reason != application.reason()
            || row.bindings != digest(bound.bindings())
        {
            return Err(invalid(
                "stored bound invocation differs from applicable signature",
            ));
        }
        let mut actual_count = 0;
        for (ordinal, binding) in bound.bindings().iter().enumerate() {
            let slot = index
                .slots
                .get(&binding.formal)
                .ok_or_else(|| invalid("bound formal has no slot"))?;
            if slot.variant != variant.id() {
                return Err(invalid("bound formal slot crosses variant"));
            }
            let member = CallBinding {
                attempt: row.id(),
                ordinal: ordinal as i64,
                slot: slot.id(),
                source: binding.source.id(),
                kind: binding.kind,
                projection: binding.projection.id(),
            };
            if stored.bindings.get(member.id()) != Some(&member)
                || stored.sources.get(member.source) != Some(&binding.source)
                || stored.projections.get(member.projection) != Some(&binding.projection)
            {
                return Err(invalid(
                    "stored bound members differ from applicable argument shape",
                ));
            }
            actual_count += 1;
        }
        if stored
            .bindings
            .iter()
            .filter(|binding| binding.attempt == row.id())
            .count()
            != actual_count
        {
            return Err(invalid("stored bound invocation has extra members"));
        }
        result.bound.insert(
            &mut result._charge,
            row.id(),
            ValidatedBoundCall {
                attempt: row.id(),
                event: row.event,
                context: need(&data.event_events, row.event)?.context,
                bound,
                bindings: row.bindings,
            },
        )?;
        let event = need(&data.event_events, row.event)?;
        let owner = need(&data.owners, event.owner)?;
        let target = original_target(data, alternative)?;
        if owner.occurrence != event.site {
            return Err(invalid("source shape owner differs from event site"));
        }
        result.source_shape.insert(
            &mut result._charge,
            row.id(),
            SourceBindingShape {
                attempt: row.id(),
                event: row.event,
                context: event.context,
                input: application.input(),
                owner_entity: owner.entity,
                owner_declaration: owner.owner,
                target: target.id(),
                phase: target.phase,
                bindings: row.bindings,
            },
        )?;
        if let Some(complete) = events.get(row.event) {
            for member in stored.members.iter().filter(|m| m.attempt == row.id()) {
                let selected = need(&stored.variants, member.variant)?;
                let set = need(&stored.sets, selected.set)?;
                let enumeration = checked_enumeration(data, stored, row, &application)?;
                if !set.coverage_complete && enumeration.is_none() {
                    continue;
                }
                let mut compatible = 0;
                let mut admitted = true;
                for v in stored.variants.iter().filter(|v| v.set == set.id()) {
                    let mut first = None;
                    for m in stored.members.iter().filter(|m| m.variant == v.id()) {
                        let a = need(&stored.attempts, m.attempt)?;
                        if a.outcome == BindingOutcome::Undetermined {
                            admitted = false;
                        }
                        if let Some(previous) = first {
                            let previous: &CallBindingAttempt = previous;
                            if (a.outcome, a.bindings, a.syntax)
                                != (previous.outcome, previous.bindings, previous.syntax)
                            {
                                admitted = false;
                            }
                        } else {
                            first = Some(a);
                        }
                    }
                    match first.map(|a| a.outcome) {
                        Some(BindingOutcome::Bound) => {
                            compatible += 1;
                            if enumeration.is_some()
                                && let Some(other) = first
                                && !equivalent_shapes(data, stored, row, other)?
                            {
                                admitted = false;
                            }
                        }
                        Some(BindingOutcome::ProvenIncompatible) => {}
                        _ => admitted = false,
                    }
                }
                if !admitted || compatible == 0 || compatible != 1 && enumeration.is_none() {
                    continue;
                }
                let event = need(&data.event_events, row.event)?;
                let owner = need(&data.owners, event.owner)?;
                let target = original_target(data, alternative)?;
                if owner.occurrence != event.site {
                    return Err(invalid("shape owner differs from event site"));
                }
                result.shape.insert(
                    &mut result._charge,
                    row.id(),
                    BindingShapeAdmission {
                        attempt: row.id(),
                        set: set.id(),
                        set_members: set.members,
                        alternative: alternative.id(),
                        event: row.event,
                        context: event.context,
                        input: application.input(),
                        complete: complete.assessment(),
                        event_members: complete.members(),
                        owner: owner.id(),
                        owner_entity: owner.entity,
                        owner_declaration: owner.owner,
                        callee: alternative
                            .entity
                            .ok_or_else(|| invalid("shape target has no entity"))?,
                        target: target.id(),
                        phase: target.phase,
                        bindings: row.bindings,
                        enumeration: enumeration.map(Record::id),
                        signature_members: enumeration.map(|e| e.members),
                    },
                )?;
            }
        }
        if row.authority != BindingAuthority::EffectiveInvocation {
            continue;
        }
        let Some(complete) = events.get(row.event) else {
            continue;
        };
        let global_set = member_sets.get(&row.id()).copied();
        let effective = need(
            &data.callable_assessments,
            row.effective
                .ok_or_else(|| invalid("effective attempt has no assessment"))?,
        )?;
        let event = need(&data.event_events, row.event)?;
        let owner = need(&data.owners, event.owner)?;
        if owner.occurrence != event.site {
            return Err(invalid("composition owner differs from event site"));
        }
        let target = original_target(data, alternative)?;
        if let Some(set) = global_set {
            result.effective.insert(
                &mut result._charge,
                row.id(),
                EffectiveInvocationAdmission {
                    attempt: row.id(),
                    set: set.id(),
                    set_members: set.members,
                    alternative: alternative.id(),
                    event: row.event,
                    context: event.context,
                    input: application.input(),
                    complete: complete.assessment(),
                    event_members: complete.members(),
                    owner: owner.id(),
                    owner_entity: owner.entity,
                    owner_declaration: owner.owner,
                    callee: alternative
                        .entity
                        .ok_or_else(|| invalid("effective target has no entity"))?,
                    effective: effective.id(),
                    target: target.id(),
                    phase: target.phase,
                    bindings: row.bindings,
                },
            )?;
        }
        if effective.body != Knowledge::Known || !effective.body_admitted {
            continue;
        }
        let selected = if let Some(set) = global_set {
            Some((
                set.id(),
                SourceBodySignatureClosure::GlobalCoverage {
                    members: set.members,
                },
            ))
        } else if let Some(shape) = result.shape.get(&row.id()) {
            selected_source_body_closure(data, stored, row, shape, effective, application.input())?
                .map(|closure| (shape.set(), closure))
        } else {
            None
        };
        let Some((set, signature_closure)) = selected else {
            continue;
        };
        let summary = data
            .event_policy_assessments
            .iter()
            .find(|p| p.event == row.event && p.policy == CallPolicy::Summary)
            .ok_or_else(|| invalid("event lacks total summary assessment"))?;
        if !data
            .event_admissions
            .iter()
            .any(|a| a.assessment == summary.id() && a.alternative == row.alternative)
        {
            continue;
        }
        let target = original_target(data, alternative)?;
        result.composition.insert(
            &mut result._charge,
            row.id(),
            CompositionAdmission {
                attempt: row.id(),
                set,
                event: row.event,
                complete: complete.assessment(),
                event_members: complete.members(),
                summary: summary.id(),
                signature_closure,
                owner: owner.id(),
                owner_entity: owner.entity,
                owner_declaration: owner.owner,
                callee: alternative
                    .entity
                    .ok_or_else(|| invalid("admitted target has no entity"))?,
                effective: effective.id(),
                target: target.id(),
                phase: target.phase,
            },
        )?;
    }
    Ok(result)
}
fn admit_attempt_domain(
    data: &BindingData,
    stored: &BindingOutput,
    index: &Index<'_>,
) -> Result<(), ModelError> {
    for alternative in data.event_alternatives.iter() {
        let event = need(&data.event_events, alternative.event)?;
        let target = original_target(data, alternative)?;
        let variants = match alternative.entity.and_then(|id| data.refs.get(id)) {
            Some(EntityRef::Callable { callable }) => {
                index.variants.get(&(*callable, event.context))
            }
            _ => need(&data.destinations, target.destination)?
                .symbol()
                .and_then(|symbol| index.raw_variants.get(&symbol)),
        };
        let syntax = index.syntax.get(&(event.site, event.context));
        let mut actual = stored
            .attempts
            .iter()
            .filter(|attempt| attempt.alternative == alternative.id())
            .count();
        for v in 0..variants.map_or(1, |rows| rows.len().max(1)) {
            for s in 0..syntax.map_or(1, |rows| rows.len().max(1)) {
                let variant = variants.and_then(|rows| rows.get(v)).map(|row| row.id());
                let syntax = syntax.and_then(|rows| rows.get(s)).map(|row| row.id());
                let mut members = stored.attempts.iter().filter(|attempt| {
                    attempt.alternative == alternative.id()
                        && attempt.variant == variant
                        && attempt.syntax == syntax
                        && attempt.policy == policy_revision()
                });
                let attempt = members.next().ok_or_else(|| {
                    invalid("binding input candidate pair has no required outcome")
                })?;
                if members.next().is_some() || attempt.event != event.id() {
                    return Err(invalid("binding attempt candidate domain changed"));
                }
                actual = actual
                    .checked_sub(1)
                    .ok_or_else(|| invalid("binding candidate domain duplicated"))?;
                let mut memberships = stored
                    .members
                    .iter()
                    .filter(|member| member.attempt == attempt.id());
                let membership = memberships
                    .next()
                    .ok_or_else(|| invalid("binding attempt has no closed set membership"))?;
                if memberships.next().is_some() {
                    return Err(invalid("binding attempt has ambiguous set membership"));
                }
                let grouping = need(&stored.variants, membership.variant)?;
                let set = need(&stored.sets, grouping.set)?;
                if grouping.variant != variant
                    || (set.event, set.entity, set.phase, set.channel)
                        != (event.id(), alternative.entity, target.phase, target.channel)
                {
                    return Err(invalid(
                        "binding attempt is a member of another event/signature set",
                    ));
                }
            }
        }
        if actual != 0 {
            return Err(invalid(
                "binding attempt exceeds the complete candidate domain",
            ));
        }
    }
    for attempt in stored.attempts.iter() {
        need(&data.event_alternatives, attempt.alternative)?;
    }
    Ok(())
}

/// Check shape/effective selection predicates one complete event set at a time. The result
/// replay verifier checks production as a diagnostic; this checks the actual premises needed
/// by a consumer and does not run predecessor normalization.
fn admit_set_predicates(
    data: &BindingData,
    stored: &BindingOutput,
    index: &Index<'_>,
    budget: &ResourceBudget,
) -> Result<(), ModelError> {
    for set in stored.sets.iter() {
        let key = (set.event, set.entity, set.phase, set.channel);
        let mut charge = StateCharge::new(budget, "binding-set-admission");
        let mut ids = Vec::new();
        for attempt in stored.attempts.iter() {
            let alternative = need(&data.event_alternatives, attempt.alternative)?;
            let target = original_target(data, alternative)?;
            if (
                alternative.event,
                alternative.entity,
                target.phase,
                target.channel,
            ) == key
            {
                charge.grow(size_of::<Id<CallBindingAttempt>>().saturating_mul(2))?;
                ids.push(attempt.id());
            }
        }
        if ids.is_empty() {
            return Err(invalid(
                "binding set has no independently selected attempts",
            ));
        }
        let mut expected = BindingOutput::new(budget);
        for id in &ids {
            expected
                .attempts
                .insert(need(&stored.attempts, *id)?.clone())?;
        }
        assess_set(data, index, key, &ids, &mut expected, budget)?;
        if expected.sets.get(set.id()) != Some(set) {
            return Err(invalid("binding set eligibility changed"));
        }
        macro_rules! check {
            ($field:ident, $belongs:expr) => {
                if expected
                    .$field
                    .iter()
                    .any(|row| stored.$field.get(row.id()) != Some(row))
                    || stored.$field.iter().filter($belongs).count() != expected.$field.len()
                {
                    return Err(invalid("binding set closed member domain differs"));
                }
            };
        }
        check!(variants, |row: &&BindingVariantAssessment| row.set
            == set.id());
        check!(members, |row: &&BindingSetMember| stored
            .variants
            .get(row.variant)
            .is_some_and(|variant| variant.set == set.id()));
        check!(coverage, |row: &&BindingSetCoverage| row.set == set.id());
    }
    Ok(())
}

fn selected_source_body_closure(
    data: &BindingData,
    stored: &BindingOutput,
    row: &CallBindingAttempt,
    shape: &BindingShapeAdmission,
    effective: &EffectiveCallableAssessment,
    input: Id<input::InputRevision>,
) -> Result<Option<SourceBodySignatureClosure>, ModelError> {
    if effective.identity != Knowledge::Known
        || effective.body != Knowledge::Known
        || !effective.body_admitted
    {
        return Ok(None);
    }
    let Some(id) = shape.enumeration() else {
        return Ok(None);
    };
    let header = need(&data.signature_enumerations, id)?;
    let signature = need(
        &data.signatures,
        row.signature
            .ok_or_else(|| invalid("body shape signature absent"))?,
    )?;
    let q = need(&data.qualifications, header.qualification)?;
    if signature.role != SignatureRole::Source
        || header.role != SignatureRole::Source
        || !header.complete
        || header.symbol != signature.symbol
        || header.qualification != signature.qualification
        || header.scope != signature.scope
        || shape.signature_members() != Some(header.members)
        || q.context != shape.context()
        || q.scope != header.scope
        || scopes(data).input(q.scope) != Some(input)
        || q.modality != Modality::Definite
        || q.approximation != crate::domain::assertion::Approximation::Exact
        || q.condition != crate::domain::conditions::Diagram::always().id()
        || q.assumptions != crate::domain::assumptions::AssumptionSet::empty_id()
    {
        return Ok(None);
    }
    let CallableEntity::Source {
        declaration,
        kind: CallableKind::Function,
    } = need(&data.callables, effective.callable)?
    else {
        return Ok(None);
    };
    let symbol = need(&data.symbols, header.symbol)?;
    let mut declarations = data
        .entity_declarations
        .iter()
        .filter(|d| d.symbol == header.symbol);
    let Some(declared) = declarations.next() else {
        return Ok(None);
    };
    if declarations.next().is_some()
        || declared.declaration != *declaration
        || declared.qualification != header.qualification
    {
        return Ok(None);
    }
    let valid = |run: Id<ProviderRun>,
                 surface: Id<crate::domain::assertion::ProviderSurface>,
                 origin,
                 mode,
                 fidelity| {
        origin == Origin::AnalyzerAssertion
            && mode == ExtractionMode::NativeTraversal
            && matches!(
                fidelity,
                Fidelity::ReportProjection | Fidelity::NativeStructural
            )
            && data.runs.get(run).is_some_and(|r| {
                r.context == q.context
                    && r.input == input
                    && r.provider == symbol.provider
                    && data.surfaces.get(surface).is_some_and(|s| {
                        s.provider == r.provider && s.family == FactFamily::Signatures
                    })
            })
    };
    let mut supports = data.signature_enumeration_supports.iter().filter(|s| {
        s.assertion == id && valid(s.run, s.surface, s.origin, s.mode, s.fidelity)
            && matches!(data.native_evidence.get(s.evidence), Some(crate::domain::assertion::Evidence::Invocation { run }) if *run == s.run)
    });
    let Some(support) = supports.next() else {
        return Ok(None);
    };
    if supports.next().is_some() || !data.declaration_supports.iter().any(|s| {
        s.assertion == declared.id() && s.run == support.run
            && valid(s.run, s.surface, s.origin, s.mode, s.fidelity)
            && matches!(data.native_evidence.get(s.evidence), Some(crate::domain::assertion::Evidence::Occurrence { occurrence }) if *occurrence == *declaration)
    }) {
        return Ok(None);
    }
    let mut covered = false;
    for coverage in data.coverage.iter().filter(|c| {
        c.scope == header.scope
            && c.context == q.context
            && c.provider == Some(symbol.provider)
            && c.run == Some(support.run)
            && c.family == FactFamily::Signatures
    }) {
        if !matches!(
            coverage.status,
            CoverageStatus::CompleteUnderStatedModel | CoverageStatus::Partial
        ) {
            return Ok(None);
        }
        covered = true;
    }
    if !covered {
        return Ok(None);
    }
    let mut selected = false;
    for member in data
        .signature_enumeration_members
        .iter()
        .filter(|m| m.enumeration == id)
    {
        let member_signature = need(&data.signatures, member.signature)?;
        if member_signature.role != SignatureRole::Source
            || member_signature.form != SignatureForm::List
            || member_signature.symbol != header.symbol
            || member_signature.qualification != header.qualification
            || !data.signature_supports.iter().any(|s| {
                s.assertion == member.signature && s.run == support.run
                    && valid(s.run, s.surface, s.origin, s.mode, s.fidelity)
                    && matches!(data.native_evidence.get(s.evidence), Some(crate::domain::assertion::Evidence::Invocation { run }) if *run == s.run)
            })
        {
            return Ok(None);
        }
        let Some(variant) = data
            .callable_variants
            .iter()
            .find(|v| v.signature == member.signature)
        else {
            return Ok(None);
        };
        let mut attempts = stored.attempts.iter().filter(|a| {
            a.event == row.event
                && a.alternative == row.alternative
                && a.variant == Some(variant.id())
        });
        let Some(first) = attempts.next() else {
            return Ok(None);
        };
        let wanted = if member.signature == signature.id() {
            BindingOutcome::Bound
        } else {
            BindingOutcome::ProvenIncompatible
        };
        if first.outcome != wanted || attempts.any(|a| a.outcome != wanted) {
            return Ok(None);
        }
        selected |= member.signature == signature.id();
    }
    if !selected {
        return Ok(None);
    }
    Ok(Some(SourceBodySignatureClosure::DeclaredEnumeration {
        enumeration: id,
        members: header.members,
        support: support.id(),
    }))
}
pub(crate) fn verify_enumerations(
    data: &BindingData,
    budget: &ResourceBudget,
) -> Result<(), ModelError> {
    let relation = Relation::of::<SignatureEnumerationObservation>();
    let definitions = crate::domain::validation::definitions();
    let definition = definitions
        .invariants
        .iter()
        .find(|check| relation.invariant_refs().contains(&check.name))
        .expect("canonical enumeration definition");
    let mut check = (definition.create)(budget);
    macro_rules! feed {
        ($field:ident,$ty:ty) => {{
            let bytes = data
                .$field
                .iter()
                .try_fold(0usize, |n, row| {
                    n.checked_add(size_of::<$ty>())
                        .and_then(|n| n.checked_add(row.heap_bytes()))
                })
                .ok_or_else(|| invalid("enumeration replay lowering overflow"))?;
            let _scratch = budget.reserve(
                "enumeration-replay-lowering",
                bytes.saturating_mul(8).saturating_add(65536),
            )?;
            let rows = data.$field.iter().cloned().collect::<Vec<_>>();
            check.visit(<$ty>::NAME, &<$ty>::encode(&rows)?)?;
        }};
    }
    feed!(
        qualifications,
        crate::domain::assertion::AssertionQualification
    );
    feed!(symbols, ProviderSymbol);
    feed!(signatures, Signature);
    feed!(signature_enumerations, SignatureEnumerationObservation);
    let mut charge = StateCharge::new(budget, "enumeration-member-order");
    charge.grow(
        data.signature_enumeration_members
            .len()
            .saturating_mul(size_of::<SignatureEnumerationMember>() + 128),
    )?;
    let mut members = data
        .signature_enumeration_members
        .iter()
        .cloned()
        .collect::<Vec<_>>();
    members.sort_by_key(|m| (m.enumeration, m.ordinal));
    check.visit(
        SignatureEnumerationMember::NAME,
        &SignatureEnumerationMember::encode(&members)?,
    )?;
    check.finish()
}
fn checked_enumeration<'a>(
    data: &'a BindingData,
    stored: &BindingOutput,
    row: &CallBindingAttempt,
    application: &ApplicableSignature<'_>,
) -> Result<Option<&'a SignatureEnumerationObservation>, ModelError> {
    let signature = need(
        &data.signatures,
        row.signature
            .ok_or_else(|| invalid("bound shape signature absent"))?,
    )?;
    let mut headers = data.signature_enumerations.iter().filter(|e| {
        e.symbol == signature.symbol
            && e.qualification == signature.qualification
            && e.role == signature.role
            && e.complete
    });
    let Some(header) = headers.next() else {
        return Ok(None);
    };
    if headers.next().is_some() {
        return Ok(None);
    }
    let q = need(&data.qualifications, header.qualification)?;
    if q.context != application.context()
        || q.modality != Modality::Definite
        || q.approximation != crate::domain::assertion::Approximation::Exact
        || q.condition != crate::domain::conditions::Diagram::always().id()
    {
        return Ok(None);
    }
    let symbol = need(&data.symbols, header.symbol)?;
    if !data.signature_enumeration_supports.iter().any(|s| {
        s.assertion == header.id()
            && data.runs.get(s.run).is_some_and(|r| {
                r.context == q.context
                    && r.input == application.input()
                    && r.provider == symbol.provider
            })
            && data
                .surfaces
                .get(s.surface)
                .is_some_and(|p| p.family == FactFamily::Signatures)
            && s.fidelity != Fidelity::DisplayOnly
    }) {
        return Ok(None);
    }
    let mut count = 0;
    for member in data
        .signature_enumeration_members
        .iter()
        .filter(|m| m.enumeration == header.id())
    {
        let member_signature = need(&data.signatures, member.signature)?;
        if member_signature.form != SignatureForm::List {
            return Ok(None);
        }
        let variant = need(
            &data.callable_variants,
            data.callable_variants
                .iter()
                .find(|v| v.signature == member.signature)
                .ok_or_else(|| invalid("enumerated signature variant absent"))?
                .id(),
        )?;
        let attempts = stored.attempts.iter().filter(|a| {
            a.event == row.event
                && a.alternative == row.alternative
                && a.variant == Some(variant.id())
        });
        let mut found = false;
        for attempt in attempts {
            found = true;
            if attempt.outcome == BindingOutcome::Undetermined {
                return Ok(None);
            }
        }
        if !found {
            return Ok(None);
        }
        count += 1;
    }
    if count == 0 {
        return Ok(None);
    }
    Ok(Some(header))
}
fn equivalent_shapes(
    data: &BindingData,
    stored: &BindingOutput,
    left: &CallBindingAttempt,
    right: &CallBindingAttempt,
) -> Result<bool, ModelError> {
    let ls = need(
        &data.signatures,
        left.signature
            .ok_or_else(|| invalid("shape left signature absent"))?,
    )?;
    let rs = need(
        &data.signatures,
        right
            .signature
            .ok_or_else(|| invalid("shape right signature absent"))?,
    )?;
    if ls.form != rs.form
        || ls.parameters != rs.parameters
        || left.syntax != right.syntax
        || left.arguments != right.arguments
        || left.receiver != right.receiver
        || left.adjustment != right.adjustment
    {
        return Ok(false);
    }
    let mut count = 0;
    for l in stored.bindings.iter().filter(|b| b.attempt == left.id()) {
        let Some(r) = stored
            .bindings
            .iter()
            .find(|b| b.attempt == right.id() && b.ordinal == l.ordinal)
        else {
            return Ok(false);
        };
        let lp = need(&data.callable_slots, l.slot)?;
        let rp = need(&data.callable_slots, r.slot)?;
        if (lp.ordinal, l.source, l.kind, l.projection)
            != (rp.ordinal, r.source, r.kind, r.projection)
        {
            return Ok(false);
        }
        count += 1;
    }
    Ok(count
        == stored
            .bindings
            .iter()
            .filter(|b| b.attempt == right.id())
            .count())
}
pub fn invariants() -> Vec<Invariant> {
    let mut inputs = BindingData::validation_inputs();
    inputs.extend(BindingOutput::validation_inputs());
    vec![
        Invariant {
            purpose: crate::domain::InvariantPurpose::DiagnosticReplay,
            revision: 1,
            name: "normalized_binding_closure",
            inputs: inputs.clone(),
            create: std::sync::Arc::new(|budget| {
                Box::new(BindingCheck {
                    data: BindingData::new(budget),
                    output: BindingOutput::new(budget),
                    budget: budget.clone(),
                    admission: false,
                })
            }),
        },
        Invariant {
            purpose: crate::domain::InvariantPurpose::Admission,
            revision: 1,
            name: "normalized_binding_admission",
            inputs,
            create: std::sync::Arc::new(|budget| {
                Box::new(BindingCheck {
                    data: BindingData::new(budget),
                    output: BindingOutput::new(budget),
                    budget: budget.clone(),
                    admission: true,
                })
            }),
        },
    ]
}
struct BindingCheck {
    data: BindingData,
    output: BindingOutput,
    budget: ResourceBudget,
    admission: bool,
}
impl InvariantCheck for BindingCheck {
    fn normalization_scope(&self) -> Option<super::admission::Scope> {
        self.admission.then_some(super::admission::Scope::Bindings)
    }
    fn visit(
        &mut self,
        relation: &str,
        batch: &arrow_array::RecordBatch,
    ) -> Result<(), ModelError> {
        if !self.data.visit(relation, batch)? && !self.output.visit(relation, batch)? {
            return Err(invalid("undeclared binding validation input"));
        }
        Ok(())
    }
    // The validation runner also checks all upstream invariants. Token minting above additionally
    // reconstructs upstream callable/event premises, rather than trusting caller-supplied flags.
    fn finish(self: Box<Self>) -> Result<(), ModelError> {
        if self.admission {
            return prepare(&self.data, &self.output, &self.budget).map(|_| ());
        }
        self.output.matches(&normalize(&self.data, &self.budget)?)
    }
}
pub fn stage(profile: stages::Profile) -> stages::Stage {
    let mut inputs = super::event_normalization::stage(profile).inputs;
    macro_rules! prior { ($($field:ident: $ty:ty,)*) => { $(inputs.push(stages::RelationUse::completed::<$ty>());)* }; }
    crate::normalized_event_outputs!(prior);
    inputs.extend(BindingData::stage_inputs());
    inputs.sort_by_key(|r| r.name());
    inputs.dedup_by_key(|r| r.name());
    if profile == stages::Profile::Catalog {
        inputs.retain(|r| {
            r.name() != flow::FlowValuePathObservation::NAME
                && r.name() != flow::FlowCallStep::NAME
                && r.name() != flow::FlowTestLeafObservation::NAME
        });
    }
    stages::Stage {
        name: "normalize_bindings",
        inputs: super::facts_stage_inputs(inputs),
        outputs: super::bindings::relations()
            .iter()
            .map(stages::RelationUse::of_relation)
            .collect(),
        contributes: vec![],
        coverage: vec![],
        profiles: vec![profile],
        effect: stages::Effect::Pure,
        code: policy_revision(),
        configuration: ContentHash::of(b"bindings/v1"),
    }
}
fn verify_upstream(
    data: &BindingData,
    budget: &ResourceBudget,
) -> Result<super::event_normalization::VerifiedEvents, ModelError> {
    let mut relations = super::relation_normalization::RelationData::new(budget);
    let mut relation_output = super::relation_normalization::RelationOutput::new(budget);
    macro_rules! entity_inputs { ($($field:ident: $ty:ty => $family:ident,)*) => { $(for row in <BindingData as Source<$ty>>::rows(data).iter() { relations.facts.$field.insert(row.clone())?; })* }; }
    macro_rules! entity_outputs { ($($field:ident: $ty:ty,)*) => { $(for row in <BindingData as Source<$ty>>::rows(data).iter() { relations.entities.$field.insert(row.clone())?; })* }; }
    macro_rules! relation_inputs { ($($field:ident: $ty:ty => $family:ident,)*) => { $(for row in <BindingData as Source<$ty>>::rows(data).iter() { relations.$field.insert(row.clone())?; })* }; }
    macro_rules! relation_outputs { ($($field:ident: $ty:ty,)*) => { $(for row in <BindingData as Source<$ty>>::rows(data).iter() { relation_output.$field.insert(row.clone())?; })* }; }
    crate::normalized_entity_inputs!(entity_inputs);
    crate::normalized_entity_outputs!(entity_outputs);
    relations
        .entities
        .matches(&super::entity_normalization::normalize(
            relations.facts.inputs(),
            budget,
        )?)?;
    crate::normalized_relation_inputs!(relation_inputs);
    crate::normalized_relation_outputs!(relation_outputs);
    relation_output.matches(&super::relation_normalization::normalize(
        &relations, budget,
    )?)?;
    drop(relations);
    drop(relation_output);
    let mut callable_data = super::callable_normalization::CallableData::new(budget);
    let mut callable_output = super::callable_normalization::CallableOutput::new(budget);
    let mut event_data = super::event_normalization::EventData::new(budget);
    let mut event_output = super::event_normalization::EventOutput::new(budget);
    // Both inventories remain executable owners. Adding a new upstream premise requires this
    // collector to supply that nominal type, rather than maintaining a second hand-copied list.
    macro_rules! callable_inputs { ($($field:ident: $ty:ty,)*) => { $(for row in <BindingData as Source<$ty>>::rows(data).iter() { callable_data.$field.insert(row.clone())?; })* }; }
    macro_rules! callable_outputs { ($($field:ident: $ty:ty,)*) => { $(for row in <BindingData as Source<$ty>>::rows(data).iter() { callable_output.$field.insert(row.clone())?; })* }; }
    macro_rules! event_inputs { ($($field:ident: $ty:ty,)*) => { $(for row in <BindingData as Source<$ty>>::rows(data).iter() { event_data.$field.insert(row.clone())?; })* }; }
    macro_rules! event_outputs { ($($field:ident: $ty:ty,)*) => { $(for row in <BindingData as Source<$ty>>::rows(data).iter() { event_output.$field.insert(row.clone())?; })* }; }
    crate::normalized_callable_inputs!(callable_inputs);
    crate::normalized_callable_outputs!(callable_outputs);
    callable_output.matches(&super::callable_normalization::normalize(
        &callable_data,
        budget,
    )?)?;
    drop(callable_data);
    drop(callable_output);
    crate::normalized_event_inputs!(event_inputs);
    crate::normalized_event_outputs!(event_outputs);
    super::event_normalization::verify(&event_data, &event_output, budget)
}

pub(crate) fn invariants_refs() -> Vec<&'static str> {
    vec!["normalized_binding_closure", "normalized_binding_admission"]
}
