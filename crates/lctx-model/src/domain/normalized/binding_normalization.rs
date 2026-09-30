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
            pub fn validation_inputs() -> Vec<ValidationInput> { vec![$(ValidationInput::of::<$ty>(&["id"]),)*] }
            pub fn stage_inputs() -> Vec<stages::RelationUse> { vec![$(stages::RelationUse::stored::<$ty>()),*] }
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
        for v in data.callable_variants.iter() {
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
) -> Result<ApplicableSignature<'a>, ObligationKind> {
    let missing = ObligationKind::MissingEvidence;
    let target = data.targets.get(alternative.target).ok_or(missing)?;
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
fn attempt(
    data: &BindingData,
    index: &Index<'_>,
    alternative: &NormalizedCallAlternative,
    variant: Option<&SignatureVariant>,
    syntax: Option<&CallSyntax>,
    output: &mut BindingOutput,
    budget: &ResourceBudget,
) -> Result<Id<CallBindingAttempt>, ModelError> {
    let mut work = StateCharge::new(budget, "binding-work");
    let target = need(&data.targets, alternative.target)?;
    let mut row = CallBindingAttempt {
        alternative: alternative.id(),
        variant: variant.map(Record::id),
        syntax: syntax.map(Record::id),
        policy: policy_revision(),
        event: alternative.event,
        signature: variant.map(|v| v.signature),
        arguments: syntax.map(|s| s.arguments),
        receiver: target.receiver,
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
                match application(data, alternative, variant, syntax) {
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
    let index = Index::new(data, budget)?;
    let mut output = BindingOutput::new(budget);
    let mut charge = StateCharge::new(budget, "binding-set-index");
    let mut sets: ChargedMap<SetKey, Vec<Id<CallBindingAttempt>>> = Default::default();
    for alternative in data.event_alternatives.iter() {
        let event = need(&data.event_events, alternative.event)?;
        let target = need(&data.targets, alternative.target)?;
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
    bound: BoundCall,
}
impl ValidatedBoundCall {
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
/// P3's complete admission receipt. P4 must explicitly migrate its owner and transfer contracts
/// before using this token; the retained provider-ID composer is not activated here.
pub struct CompositionAdmission {
    attempt: Id<CallBindingAttempt>,
    set: Id<BindingSetAssessment>,
    event: Id<NormalizedCallEvent>,
    complete: Id<EventAssessment>,
    event_members: ContentHash,
    summary: Id<CallPolicyAssessment>,
    owner: Id<OccurrenceOwnership>,
    callee: Id<EntityRef>,
    effective: Id<EffectiveCallableAssessment>,
    target: Id<CallTarget>,
    phase: CallPhase,
}
impl CompositionAdmission {
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
pub struct VerifiedBindings {
    bound: ChargedMap<Id<CallBindingAttempt>, ValidatedBoundCall>,
    composition: ChargedMap<Id<CallBindingAttempt>, CompositionAdmission>,
    _charge: StateCharge,
}
impl VerifiedBindings {
    pub fn bound(&self, attempt: Id<CallBindingAttempt>) -> Option<&ValidatedBoundCall> {
        self.bound.get(&attempt)
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
    let events = verify_upstream(data, budget)?;
    let index = Index::new(data, budget)?;
    let mut result = VerifiedBindings {
        bound: Default::default(),
        composition: Default::default(),
        _charge: StateCharge::new(budget, "validated-bindings"),
    };
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
        let application = application(data, alternative, variant, syntax)
            .map_err(|_| invalid("stored binding applicability changed"))?;
        let mut work = StateCharge::new(budget, "binding-replay-work");
        let bound = bind_application(data, &index, &application, &mut work)?
            .map_err(|_| invalid("stored binding replay refused"))?;
        result.bound.insert(
            &mut result._charge,
            row.id(),
            ValidatedBoundCall {
                attempt: row.id(),
                bound,
            },
        )?;
        if row.authority != BindingAuthority::EffectiveInvocation {
            continue;
        }
        let Some(complete) = events.get(row.event) else {
            continue;
        };
        let Some(set) = member_sets.get(&row.id()) else {
            continue;
        };
        let effective = need(
            &data.callable_assessments,
            row.effective
                .ok_or_else(|| invalid("effective attempt has no assessment"))?,
        )?;
        if effective.body != Knowledge::Known || !effective.body_admitted {
            continue;
        }
        let event = need(&data.event_events, row.event)?;
        let owner = need(&data.owners, event.owner)?;
        if owner.occurrence != event.site {
            return Err(invalid("composition owner differs from event site"));
        }
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
        let target = need(&data.targets, alternative.target)?;
        result.composition.insert(
            &mut result._charge,
            row.id(),
            CompositionAdmission {
                attempt: row.id(),
                set: set.id(),
                event: row.event,
                complete: complete.assessment(),
                event_members: complete.members(),
                summary: summary.id(),
                owner: owner.id(),
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
pub fn invariants() -> Vec<Invariant> {
    let mut inputs = BindingData::validation_inputs();
    inputs.extend(BindingOutput::validation_inputs());
    vec![Invariant {
        name: "normalized_binding_closure",
        inputs,
        create: std::sync::Arc::new(|budget| {
            Box::new(BindingCheck {
                data: BindingData::new(budget),
                output: BindingOutput::new(budget),
                budget: budget.clone(),
            })
        }),
    }]
}
struct BindingCheck {
    data: BindingData,
    output: BindingOutput,
    budget: ResourceBudget,
}
impl InvariantCheck for BindingCheck {
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
        self.output.matches(&normalize(&self.data, &self.budget)?)
    }
}
pub fn stage(profile: stages::Profile) -> stages::Stage {
    let mut inputs = super::event_normalization::stage(profile).inputs;
    macro_rules! prior { ($($field:ident: $ty:ty,)*) => { $(inputs.push(stages::RelationUse::stored::<$ty>());)* }; }
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
        inputs,
        outputs: super::bindings::relations()
            .iter()
            .map(stages::RelationUse::of_relation)
            .collect(),
        contributes: vec![],
        coverage: vec![],
        provider: None,
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
