//! Model-owned target closure. A typing class or final flag never supplies runtime identity.
use super::{
    model_application::{self, ModelApplicationData},
    model_construction::CheckedContextConstruction,
    model_context::CheckedExactClass,
};
use crate::domain::{
    analysis,
    assertion::*,
    assumptions::*,
    assumptions_universe::AssumptionUniverseSupport,
    attribution::*,
    calls::*,
    class_metadata::*,
    models::Catalog,
    normalized::{
        Rows,
        binding_normalization::{
            BindingShapeAdmission, EffectiveInvocationAdmission, SourceBindingShape,
            ValidatedBoundCall,
        },
    },
    obligation::ObligationKind,
    resources::ResourceBudget,
    types::*,
    *,
};
use crate::{Domain, DomainCode};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, DomainCode)]
#[repr(i16)]
pub enum TargetBasis {
    ExactRuntime = 0,
    TypingConditional = 1,
    Open = 2,
}
#[derive(Debug, Clone, PartialEq, Eq, Domain,serde::Serialize,serde::Deserialize)]
#[model(name = "closed_target_assessments", rule = "checked_target_basis")]
pub struct ClosedTargetAssessment {
    #[model(key, premise)]
    pub invocation: Id<analysis::model::AnalysisInvocation>,
    #[model(key, premise)]
    pub attempt: Id<normalized::bindings::CallBindingAttempt>,
    pub event: Id<normalized::events::NormalizedCallEvent>,
    pub target: Option<Id<CallTarget>>,
    /// The native candidate remains routing evidence, separately qualified from the question.
    pub original_qualification: Option<Id<AssertionQualification>>,
    #[model(premise)]
    pub target_native: Option<Id<analysis::native::NativeAssertionPremise>>,
    #[model(premise)]
    pub receiver_observation: Option<Id<TypeObservation>>,
    pub receiver_qualification: Option<Id<AssertionQualification>>,
    pub basis: TargetBasis,
    pub qualification: Option<Id<AssertionQualification>>,
    pub reason: Option<ObligationKind>,
    pub runtime_receiver: Option<Id<source::Occurrence>>,
    #[model(premise)]
    pub ancestry: Option<Id<symbols::ClassAncestryObservation>>,
    #[model(premise)]
    pub receiver_conformance: Option<Id<Assumption>>,
    #[model(premise)]
    pub no_extra_overrides: Option<Id<Assumption>>,
    #[model(premise)]
    pub universe_support: Option<Id<AssumptionUniverseSupport>>,
    #[model(premise)]
    pub final_metadata: Option<Id<ClassMetadataObservation>>,
    #[model(premise)]
    pub final_member: Option<Id<ClassMemberObservation>>,
    #[model(premise)]
    pub final_native: Option<Id<analysis::native::NativeAssertionPremise>>,
    #[model(premise)]
    pub member_native: Option<Id<analysis::native::NativeAssertionPremise>>,
}
/// Only replayable native payloads supplement the existing normalized binding owner.
pub struct TargetData {
    pub metadata: Rows<ClassMetadataObservation>,
    pub members: Rows<ClassMemberObservation>,
    pub origins: Rows<CallOrigin>,
    pub origin_steps: Rows<CallOriginStep>,
}
impl TargetData {
    pub fn new(b: &ResourceBudget) -> Self {
        Self {
            metadata: Rows::new(b),
            members: Rows::new(b),
            origins: Rows::new(b),
            origin_steps: Rows::new(b),
        }
    }
    pub fn inputs() -> Vec<ValidationInput> {
        vec![
            ValidationInput::of::<ClassMetadataObservation>(&["id"]),
            ValidationInput::of::<ClassMemberObservation>(&["id"]),
            ValidationInput::of::<CallOrigin>(&["id"]),
            ValidationInput::of::<CallOriginStep>(&["id"]),
        ]
    }
    pub fn visit(&mut self, n: &str, b: &arrow_array::RecordBatch) -> Result<(), ModelError> {
        if n == ClassMetadataObservation::NAME {
            self.metadata.decode(b)?;
        }
        if n == ClassMemberObservation::NAME {
            self.members.decode(b)?;
        }
        if n == CallOrigin::NAME {
            self.origins.decode(b)?;
        }
        if n == CallOriginStep::NAME {
            self.origin_steps.decode(b)?;
        }
        Ok(())
    }
}
pub(super) fn one<'a, T>(mut rows: impl Iterator<Item = &'a T>) -> Result<&'a T, ObligationKind> {
    let row = rows.next().ok_or(ObligationKind::MissingEvidence)?;
    if rows.next().is_some() {
        return Err(ObligationKind::AmbiguousBinding);
    }
    Ok(row)
}
pub(super) fn native(
    data: &ModelApplicationData,
    reference: derivation::RowRef,
    q: Id<AssertionQualification>,
    input: Id<input::InputRevision>,
    context: Id<AnalysisContext>,
) -> Result<&analysis::native::NativeAssertionPremise, ObligationKind> {
    model_application::exact(&data.bindings, q, context)?;
    let premise = one(data.native.iter().filter(|n| {
        n.qualification == q
            && n.fidelity == Fidelity::NativeStructural
            && data
                .premises
                .get(n.premise)
                .is_some_and(|p| p.assertion_and_support().0 == reference)
    }))?;
    let p = data
        .premises
        .get(premise.premise)
        .ok_or(ObligationKind::MissingEvidence)?;
    // Observation qualification and source frame are independent of the provider of canonical syntax.
    let run = match p {
        analysis::native::NativeAssertionPremise::TypeObservation { support, .. } => {
            data.native_type_supports.get(*support).map(|s| s.run)
        }
        analysis::native::NativeAssertionPremise::ClassTraitObservation { support, .. } => {
            data.native_class_supports.get(*support).map(|s| s.run)
        }
        analysis::native::NativeAssertionPremise::ClassMemberObservation { support, .. } => {
            data.native_member_supports.get(*support).map(|s| s.run)
        }
        analysis::native::NativeAssertionPremise::ClassMetadataObservation { support, .. } => {
            data.native_metadata_supports.get(*support).map(|s| s.run)
        }
        analysis::native::NativeAssertionPremise::NativeTerminal { support, .. } => {
            data.native_terminal_supports.get(*support).map(|s| s.run)
        }
        analysis::native::NativeAssertionPremise::NativeExit { support, .. } => {
            data.native_exit_supports.get(*support).map(|s| s.run)
        }
        analysis::native::NativeAssertionPremise::SyntaxPlacement { support, .. } => data
            .bindings
            .placement_supports
            .get(*support)
            .map(|s| s.run),
        _ => None,
    };
    if run
        .and_then(|r| data.bindings.runs.get(r))
        .is_none_or(|r| (r.input, r.context) != (input, context))
    {
        return Err(ObligationKind::IncompatibleContexts);
    }
    Ok(p)
}
pub(super) fn conformance(
    data: &ModelApplicationData,
    row: &TypeObservation,
    input: Id<input::InputRevision>,
    context: Id<AnalysisContext>,
) -> Result<Assumption, ObligationKind> {
    match data.bindings.terms.get(row.term) {
        Some(TypeTerm::Any { .. } | TypeTerm::Other { .. } | TypeTerm::Truncated { .. }) | None => {
            return Err(ObligationKind::MissingEvidence);
        }
        _ => {}
    }
    match native(
        data,
        derivation::RowRef::of(row.id()),
        row.qualification,
        input,
        context,
    )? {
        analysis::native::NativeAssertionPremise::TypeObservation { assertion, support } => {
            Ok(Assumption::TypeConformance {
                observation: *assertion,
                support: *support,
            })
        }
        _ => Err(ObligationKind::MissingEvidence),
    }
}
/// Runtime applicability uses the same identity/descriptor checks as authored applications.
/// Bound receivers additionally require an actual protocol-owned constructor value and complete MRO.
pub(super) struct CheckedRuntimeReceiver {
    target: Id<CallTarget>,
    signature: Id<Signature>,
    input: Id<input::InputRevision>,
    context: Id<AnalysisContext>,
}
impl CheckedRuntimeReceiver {
    pub(super) fn admits(&self, bound: &ValidatedBoundCall, shape: &BindingShapeAdmission) -> bool {
        shape.admits(bound)
            && (self.target, self.signature, self.input, self.context)
                == (
                    shape.target(),
                    bound.bound().signature(),
                    shape.input(),
                    shape.context(),
                )
    }
}
type RuntimeReceiverEvidence = (
    Id<source::Occurrence>,
    Id<symbols::ClassAncestryObservation>,
);
type RuntimeIdentityAssessment = Result<Option<RuntimeReceiverEvidence>, ObligationKind>;
pub(super) fn runtime_identity(
    catalog: &Catalog,
    data: &ModelApplicationData,
    bound: &ValidatedBoundCall,
    shape: &BindingShapeAdmission,
    effective: Option<&EffectiveInvocationAdmission>,
    budget: &ResourceBudget,
) -> Result<RuntimeIdentityAssessment, ModelError> {
    let b = &data.bindings;
    let target = b
        .targets
        .get(shape.target())
        .ok_or_else(|| ModelError::Invalid("closed target absent".into()))?;
    let receiver = match b.receivers.get(target.receiver) {
        Some(Receiver::None) => {
            return Ok(model_application::runtime_identity_native(
                data, bound, shape, effective, None,
            )
            .map(|()| None));
        }
        Some(Receiver::Bound { actual }) => *actual,
        _ => return Ok(Err(ObligationKind::MissingEvidence)),
    };
    let Some(class) = target.receiver_class else {
        return Ok(Err(ObligationKind::MissingEvidence));
    };
    let protocol = match super::model_context::CheckedContextProtocol::derive(
        catalog,
        data,
        class,
        shape.input(),
        shape.context(),
        budget,
    )? {
        Ok(p) => p,
        Err(r) => return Ok(Err(r)),
    };
    let construction = match CheckedContextConstruction::derive(
        &protocol,
        data,
        receiver,
        shape.owner_entity(),
        shape.input(),
        shape.context(),
        budget,
    )? {
        Ok(p) => p,
        Err(r) => return Ok(Err(r)),
    };
    if construction.resource().site != receiver || construction.resource().class != class {
        return Ok(Err(ObligationKind::IncompatibleContexts));
    }
    // Use the captured member inventory and checked MRO; this does not create a second dispatch set.
    let lookup = (|| {
        let signature = b
            .signatures
            .get(bound.bound().signature())
            .ok_or(ObligationKind::MissingEvidence)?;
        let symbol = b
            .symbols
            .get(signature.symbol)
            .ok_or(ObligationKind::MissingEvidence)?;
        let traits = one(b.traits.iter().filter(|t| t.symbol == symbol.id()))?;
        let mro = b
            .ancestry
            .get(protocol.class().ancestry())
            .ok_or(ObligationKind::MissingEvidence)?;
        let mut ancestors = b
            .sequence_members
            .iter()
            .filter(|m| m.sequence == mro.ancestors)
            .collect::<Vec<_>>();
        ancestors.sort_by_key(|m| m.ordinal);
        let mut found = None;
        for owner in std::iter::once(class).chain(ancestors.into_iter().map(|m| m.symbol)) {
            let mut members = b.traits.iter().filter(|t| {
                t.defining_class == Some(owner)
                    && b.symbols.get(t.symbol).is_some_and(|s| {
                        s.name == symbol.name
                            && s.provider == symbol.provider
                            && s.context == shape.context()
                    })
            });
            if let Some(member) = members.next() {
                if members.next().is_some() {
                    return Err(ObligationKind::AmbiguousBinding);
                }
                found = Some(member.symbol);
                break;
            }
        }
        if traits.defining_class.is_none() || found != Some(symbol.id()) {
            return Err(ObligationKind::CallTransfer);
        }
        let identity = one(b.callable_assessments.iter().filter(|e| {
            normalized::entities::EntityRef::Callable {
                callable: e.callable,
            }
            .id()
                == shape.callee()
                && e.context == shape.context()
        }))?;
        if identity.identity != normalized::callables::Knowledge::Known
            || identity.descriptor != normalized::callables::Knowledge::Known
        {
            return Err(ObligationKind::MissingEvidence);
        }
        Ok(CheckedRuntimeReceiver {
            target: target.id(),
            signature: signature.id(),
            input: shape.input(),
            context: shape.context(),
        })
    })();
    let receiver_proof = match lookup {
        Ok(p) => p,
        Err(r) => return Ok(Err(r)),
    };
    if let Err(r) = model_application::runtime_identity_native(
        data,
        bound,
        shape,
        effective,
        Some(&receiver_proof),
    ) {
        return Ok(Err(r));
    }
    Ok(Ok(Some((receiver, protocol.class().ancestry()))))
}
/// Structural native identity only; anonymous, overloaded and opaque callable forms refuse.
pub(super) fn function_identity(
    data: &ModelApplicationData,
    mut term: Id<TypeTerm>,
) -> Option<Id<ProviderSymbol>> {
    for _ in 0..=data.bindings.terms.len() {
        match data.bindings.terms.get(term) {
            Some(TypeTerm::Callable { function, .. }) => return *function,
            Some(TypeTerm::Generic { body, .. }) => term = *body,
            Some(TypeTerm::BoundMethod { function, .. }) => term = *function,
            _ => return None,
        }
    }
    None
}
pub(super) struct ConditionalTarget {
    pub question_base: Id<AssertionQualification>,
    pub receiver_observation: Id<TypeObservation>,
    pub target_native: Id<analysis::native::NativeAssertionPremise>,
    pub basis: ResolvedAssumptions,
    pub receiver: Assumption,
    pub overrides: Assumption,
    pub universe: AssumptionUniverse,
    pub support: AssumptionUniverseSupport,
    pub ancestry: Id<symbols::ClassAncestryObservation>,
    pub metadata: Id<ClassMetadataObservation>,
    pub metadata_native: Id<analysis::native::NativeAssertionPremise>,
    pub member: Option<Id<ClassMemberObservation>>,
    pub member_native: Id<analysis::native::NativeAssertionPremise>,
}
/// Candidate describes the original Overrides route only. Definite question evidence must
/// independently establish the receiver and selected member; this never promotes the target.
fn candidate_route(
    q: &AssertionQualification,
    destination: &CallDestination,
    authority: normalized::signature_applicability::BindingAuthority,
    reason: normalized::signature_applicability::AuthorityReason,
    context: Id<AnalysisContext>,
) -> Result<(), ObligationKind> {
    if q.context != context {
        return Err(ObligationKind::IncompatibleContexts);
    }
    if !matches!(destination, CallDestination::Overrides { .. })
        || authority != normalized::signature_applicability::BindingAuthority::SourceInspection
        || reason != normalized::signature_applicability::AuthorityReason::DispatchOpen
    {
        return Err(ObligationKind::CallTransfer);
    }
    if q.modality != Modality::Candidate
        || q.approximation != Approximation::Exact
        || q.condition != conditions::Diagram::always().id()
        || q.assumptions != AssumptionSet::empty_id()
    {
        return Err(ObligationKind::Approximation);
    }
    Ok(())
}
fn selected_method(
    native_member: Option<Id<ProviderSymbol>>,
    selected: Id<ProviderSymbol>,
    mro_member: Option<Id<ProviderSymbol>>,
    mro: Option<symbols::Linearization>,
    kind: MemberKind,
) -> Result<(), ObligationKind> {
    if mro != Some(symbols::Linearization::Complete) {
        return Err(ObligationKind::IncompleteCoverage);
    }
    if kind == MemberKind::Property {
        return Err(ObligationKind::OutsideProviderModel);
    }
    if native_member != Some(selected) || mro_member != Some(selected) {
        return Err(ObligationKind::CallTransfer);
    }
    Ok(())
}
pub(super) fn question_qualification(
    base: &AssertionQualification,
    basis: &ResolvedAssumptions,
) -> Result<AssertionQualification, ModelError> {
    if base.modality != Modality::Definite
        || base.approximation != Approximation::Exact
        || base.condition != conditions::Diagram::always().id()
        || base.assumptions != AssumptionSet::empty_id()
    {
        return Err(ModelError::Invalid(
            "conditional target receiver is not an exact native question base".into(),
        ));
    }
    basis.check(basis.set.id())?;
    let mut question = base.clone();
    question.assumptions = basis.set.id();
    Ok(question)
}
/// Final contributes only an explicit conditional closure, under a supported receiver type,
/// complete MRO and a no-extra-overrides premise pinned to the selected actual catalog.
pub(super) fn typing_target(
    catalog: &Catalog,
    data: &ModelApplicationData,
    extra: &TargetData,
    bound: &ValidatedBoundCall,
    shape: &SourceBindingShape,
    attempt: &normalized::bindings::CallBindingAttempt,
    budget: &ResourceBudget,
) -> Result<Result<ConditionalTarget, ObligationKind>, ModelError> {
    let b = &data.bindings;
    let result = (|| {
        if !shape.admits(bound) || shape.phase() != CallPhase::Call {
            return Err(ObligationKind::CallTransfer);
        }
        let target = b
            .targets
            .get(shape.target())
            .ok_or(ObligationKind::MissingEvidence)?;
        let original = b
            .qualifications
            .get(target.qualification)
            .ok_or(ObligationKind::MissingEvidence)?;
        let destination = b
            .destinations
            .get(target.destination)
            .ok_or(ObligationKind::MissingEvidence)?;
        candidate_route(
            original,
            destination,
            attempt.authority,
            attempt.authority_reason,
            shape.context(),
        )?;
        let class = target
            .receiver_class
            .ok_or(ObligationKind::MissingEvidence)?;
        let actual = match b.receivers.get(target.receiver) {
            Some(Receiver::Bound { actual }) => *actual,
            _ => return Err(ObligationKind::MissingEvidence),
        };
        let metadata = one(extra.metadata.iter().filter(|m| {
            m.class == class
                && b.qualifications
                    .get(m.qualification)
                    .is_some_and(|q| q.context == shape.context())
        }))?;
        if metadata.protocol || metadata.enumeration || metadata.explicitly_abstract {
            return Err(ObligationKind::OutsideProviderModel);
        }
        let metadata_native = native(
            data,
            derivation::RowRef::of(metadata.id()),
            metadata.qualification,
            shape.input(),
            shape.context(),
        )?
        .id();
        let signature = b
            .signatures
            .get(bound.bound().signature())
            .ok_or(ObligationKind::MissingEvidence)?;
        if !signature.role.runtime_source() {
            return Err(ObligationKind::OutsideProviderModel);
        }
        let symbol = b
            .symbols
            .get(signature.symbol)
            .ok_or(ObligationKind::MissingEvidence)?;
        if destination.symbol() != Some(symbol.id()) {
            return Err(ObligationKind::CallTransfer);
        }
        let target_native=one(data.native.iter().filter(|n|n.qualification==target.qualification && n.family==FactFamily::Calls && n.status==analysis::policy::EvidenceStatus::StructurallyObserved && n.fidelity==Fidelity::NativeStructural && data.premises.get(n.premise).is_some_and(|p|matches!(p,analysis::native::NativeAssertionPremise::CallTarget{assertion,support} if *assertion==target.id() && b.target_supports.get(*support).is_some_and(|s|s.origin==Origin::AnalyzerAssertion && s.mode==ExtractionMode::NativeTraversal && s.fidelity==n.fidelity && b.runs.get(s.run).is_some_and(|r|r.provider==symbol.provider && r.input==shape.input() && r.context==shape.context()) && b.surfaces.get(s.surface).is_some_and(|surface|surface.provider==symbol.provider && surface.family==FactFamily::Calls))))))?.premise;
        let traits = one(b.traits.iter().filter(|t| t.symbol == symbol.id()))?;
        if traits.origin != symbols::FunctionOrigin::DefStatement
            || traits.property_getter
            || traits.property_setter
        {
            return Err(ObligationKind::OutsideProviderModel);
        }
        let member = one(extra.members.iter().filter(|m| {
            m.class == class
                && m.name == symbol.name
                && Some(m.defining_class) == traits.defining_class
                && b.qualifications
                    .get(m.qualification)
                    .is_some_and(|q| q.context == shape.context())
        }))?;
        if member.abstract_declaration
            || member.kind == MemberKind::Property
            || (!metadata.final_declaration && !member.final_declaration)
        {
            return Err(ObligationKind::OutsideProviderModel);
        }
        let member_native = native(
            data,
            derivation::RowRef::of(member.id()),
            member.qualification,
            shape.input(),
            shape.context(),
        )?
        .id();
        // Exact native effective member identity answers only this typing question. It does not
        // recognize source decorator spelling or repair runtime descriptor/effect admission.
        if function_identity(data, member.term) != Some(symbol.id()) {
            return Err(ObligationKind::MissingEvidence);
        }
        let row=one(b.type_observations.iter().filter(|o|o.subject==actual && o.role==TypeRole::AttributeBase && matches!(b.terms.get(o.term),Some(TypeTerm::ClassInstance{class:c,..})if *c==class) && b.qualifications.get(o.qualification).is_some_and(|q|q.context==shape.context())))?;
        let receiver = conformance(data, row, shape.input(), shape.context())?;
        let question_base = b
            .qualifications
            .get(row.qualification)
            .ok_or(ObligationKind::MissingEvidence)?;
        if question_base.assumptions != AssumptionSet::empty_id() {
            return Err(ObligationKind::Approximation);
        }
        let class_row = one(b.class_traits.iter().filter(|r| {
            r.symbol == class
                && b.qualifications
                    .get(r.qualification)
                    .is_some_and(|q| q.context == shape.context())
        }))?;
        let class = match native(
            data,
            derivation::RowRef::of(class_row.id()),
            class_row.qualification,
            shape.input(),
            shape.context(),
        )? {
            analysis::native::NativeAssertionPremise::ClassTraitObservation {
                assertion, ..
            } => *assertion,
            _ => return Err(ObligationKind::MissingEvidence),
        };
        let context = data
            .contexts
            .get(shape.context())
            .ok_or(ObligationKind::MissingEvidence)?;
        let universe = AssumptionUniverse {
            context: shape.context(),
            input: shape.input(),
            environment: context.environment_digest,
            model_definition: catalog.declaration().content,
        };
        let model = catalog
            .models()
            .first()
            .ok_or(ObligationKind::MissingEvidence)?;
        let support =
            AssumptionUniverseSupport::new(&universe, catalog.declaration(), model.declaration())
                .map_err(|_| ObligationKind::MissingEvidence)?;
        let overrides = Assumption::NoExtraOverrides {
            class,
            support: match native(
                data,
                derivation::RowRef::of(class_row.id()),
                class_row.qualification,
                shape.input(),
                shape.context(),
            )? {
                analysis::native::NativeAssertionPremise::ClassTraitObservation {
                    support, ..
                } => *support,
                _ => unreachable!(),
            },
            universe: universe.id(),
        };
        Ok((
            receiver,
            overrides,
            universe,
            support,
            metadata.id(),
            metadata_native,
            Some(member.id()),
            member_native,
            question_base.id(),
            row.id(),
            target_native,
        ))
    })();
    let (
        receiver,
        overrides,
        universe,
        support,
        metadata,
        metadata_native,
        member,
        member_native,
        question_base,
        receiver_observation,
        target_native,
    ) = match result {
        Ok(v) => v,
        Err(r) => return Ok(Err(r)),
    };
    let target = b.targets.get(shape.target()).unwrap();
    let class = match CheckedExactClass::derive(
        data,
        target.receiver_class.unwrap(),
        shape.context(),
        budget,
    )? {
        Ok(p) => p,
        Err(r) => return Ok(Err(r)),
    };
    let signature = b.signatures.get(bound.bound().signature()).unwrap();
    let symbol = b.symbols.get(signature.symbol).unwrap();
    let mro = b.ancestry.get(class.ancestry()).unwrap();
    let mut ancestors = b
        .sequence_members
        .iter()
        .filter(|m| m.sequence == mro.ancestors)
        .collect::<Vec<_>>();
    ancestors.sort_by_key(|m| m.ordinal);
    let mut found = None;
    for owner in std::iter::once(class.symbol()).chain(ancestors.into_iter().map(|m| m.symbol)) {
        let mut methods = b.traits.iter().filter(|t| {
            t.defining_class == Some(owner)
                && b.symbols.get(t.symbol).is_some_and(|s| {
                    s.name == symbol.name
                        && s.provider == symbol.provider
                        && s.context == shape.context()
                })
        });
        if let Some(method) = methods.next() {
            if methods.next().is_some() {
                return Ok(Err(ObligationKind::AmbiguousBinding));
            }
            found = Some(method.symbol);
            break;
        }
    }
    let member_row = extra.members.get(member.unwrap()).unwrap();
    if let Err(r) = selected_method(
        function_identity(data, member_row.term),
        symbol.id(),
        found,
        mro.linearization,
        member_row.kind,
    ) {
        return Ok(Err(r));
    }
    let basis = AssumptionSet::new([receiver.id(), overrides.id()])?;
    Ok(Ok(ConditionalTarget {
        question_base,
        receiver_observation,
        target_native,
        basis,
        receiver,
        overrides,
        universe,
        support,
        ancestry: class.ancestry(),
        metadata,
        metadata_native,
        member,
        member_native,
    }))
}
pub fn relations() -> Vec<Relation> {
    vec![Relation::of::<ClosedTargetAssessment>()]
}

#[cfg(test)]
mod tests {
    use super::*;
    use normalized::signature_applicability::{AuthorityReason, BindingAuthority};
    fn id<R>(n: u8) -> Id<R> {
        serde_json::from_value(serde_json::json!(vec![n; 16])).unwrap()
    }
    #[test]
    fn candidate_override_route_stays_candidate_while_receiver_question_gets_its_own_basis() {
        let original = AssertionQualification {
            context: id(1),
            scope: id(2),
            condition: conditions::Diagram::always().id(),
            modality: Modality::Candidate,
            approximation: Approximation::Exact,
            assumptions: AssumptionSet::empty_id(),
        };
        let before = original.clone();
        let route = CallDestination::Overrides { symbol: id(3) };
        candidate_route(
            &original,
            &route,
            BindingAuthority::SourceInspection,
            AuthorityReason::DispatchOpen,
            original.context,
        )
        .unwrap();
        let receiver = AssertionQualification {
            modality: Modality::Definite,
            ..original.clone()
        };
        let basis = AssumptionSet::new([id::<Assumption>(4), id::<Assumption>(5)]).unwrap();
        let question = question_qualification(&receiver, &basis).unwrap();
        assert_eq!(original, before);
        assert_eq!(original.modality, Modality::Candidate);
        assert_eq!(question.modality, Modality::Definite);
        assert_eq!(question.assumptions, basis.set.id());
        assert_eq!(question.context, receiver.context);
        assert_eq!(question.scope, receiver.scope);
        assert!(
            question_qualification(&original, &basis).is_err(),
            "candidate is never the definite question premise"
        );
        for destination in [
            CallDestination::Resolved { symbol: id(3) },
            CallDestination::Callable { callable: id(6) },
        ] {
            assert!(
                candidate_route(
                    &original,
                    &destination,
                    BindingAuthority::SourceInspection,
                    AuthorityReason::DispatchOpen,
                    original.context
                )
                .is_err()
            );
        }
        for changed in [
            AssertionQualification {
                modality: Modality::Potential,
                ..original.clone()
            },
            AssertionQualification {
                approximation: Approximation::Over,
                ..original.clone()
            },
            AssertionQualification {
                approximation: Approximation::Unknown,
                ..original.clone()
            },
            AssertionQualification {
                assumptions: basis.set.id(),
                ..original.clone()
            },
        ] {
            assert!(
                candidate_route(
                    &changed,
                    &route,
                    BindingAuthority::SourceInspection,
                    AuthorityReason::DispatchOpen,
                    original.context
                )
                .is_err()
            );
        }
        assert!(
            candidate_route(
                &original,
                &route,
                BindingAuthority::EffectiveInvocation,
                AuthorityReason::DispatchOpen,
                original.context
            )
            .is_err()
        );
        assert!(
            candidate_route(
                &original,
                &route,
                BindingAuthority::SourceInspection,
                AuthorityReason::Established,
                original.context
            )
            .is_err()
        );
    }
    #[test]
    fn conditional_member_selection_refuses_competing_identity_property_and_incomplete_mro() {
        use symbols::Linearization as L;
        let selected = id(1);
        let competing = id(2);
        selected_method(
            Some(selected),
            selected,
            Some(selected),
            Some(L::Complete),
            MemberKind::Other,
        )
        .unwrap();
        for (native, mro, linearization, kind) in [
            (
                Some(competing),
                Some(selected),
                Some(L::Complete),
                MemberKind::Other,
            ),
            (
                Some(selected),
                Some(competing),
                Some(L::Complete),
                MemberKind::Other,
            ),
            (None, Some(selected), Some(L::Complete), MemberKind::Other),
            (
                Some(selected),
                Some(selected),
                Some(L::Prefix),
                MemberKind::Other,
            ),
            (
                Some(selected),
                Some(selected),
                Some(L::Cyclic),
                MemberKind::Other,
            ),
            (Some(selected), Some(selected), None, MemberKind::Other),
            (
                Some(selected),
                Some(selected),
                Some(L::Complete),
                MemberKind::Property,
            ),
        ] {
            assert!(selected_method(native, selected, mro, linearization, kind).is_err());
        }
    }
}
