//! A protocol-owned constructor uses the sole argument algorithm with an ephemeral receiver.
//! It never rewrites a native unknown receiver or mints an ordinary BoundCall.
use super::{
    model_application::{ModelApplicationData, RuntimeEvidenceError, append_native_evidence},
    model_context::{CheckedContextProtocol, ContextValue},
};
use crate::domain::{
    attribution::AnalysisContext, calls::*, normalized::Rows, obligation::ObligationKind,
    resources::ResourceBudget, *,
};
use std::collections::BTreeMap;
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ConstructedResource {
    pub site: Id<source::Occurrence>,
    pub owner: Id<normalized::entities::EntityRef>,
    pub context: Id<AnalysisContext>,
    pub class: Id<ProviderSymbol>,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProtocolArgumentSource {
    Receiver,
    Value(BindingSource),
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProtocolArgument {
    pub formal: Id<SignatureParameter>,
    pub source: ProtocolArgumentSource,
    pub kind: BindingKind,
    pub projection: BindingProjection,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProtocolSignatureOutcome {
    pub signature: Id<Signature>,
    pub outcome: Result<(), BindingFailure>,
    pub arguments: Vec<ProtocolArgument>,
}
pub struct CheckedContextConstruction<'a> {
    protocol: &'a CheckedContextProtocol<'a>,
    input: Id<input::InputRevision>,
    arguments: Vec<CallArgument>,
    resource: ConstructedResource,
    allocation: Id<CallTarget>,
    initialization: Id<CallTarget>,
    enumeration: Id<SignatureEnumerationObservation>,
    outcomes: Vec<ProtocolSignatureOutcome>,
    entry: ContextValue,
    premises: Rows<analysis::native::NativeAssertionPremise>,
    status: analysis::policy::EvidenceStatus,
    _charge: charged::StateCharge,
}
fn need<R: Record>(rows: &Rows<R>, id: Id<R>) -> Result<&R, ObligationKind> {
    rows.get(id).ok_or(ObligationKind::MissingEvidence)
}
fn one<'a, T: 'a>(mut rows: impl Iterator<Item = &'a T>) -> Result<&'a T, ObligationKind> {
    let row = rows.next().ok_or(ObligationKind::MissingEvidence)?;
    if rows.next().is_some() {
        return Err(ObligationKind::AmbiguousBinding);
    }
    Ok(row)
}
impl<'a> CheckedContextConstruction<'a> {
    pub fn protocol(&self) -> &CheckedContextProtocol<'a> {
        self.protocol
    }
    pub fn input(&self) -> Id<input::InputRevision> {
        self.input
    }
    pub fn arguments(&self) -> &[CallArgument] {
        &self.arguments
    }
    pub fn resource(&self) -> ConstructedResource {
        self.resource
    }
    pub fn allocation(&self) -> Id<CallTarget> {
        self.allocation
    }
    pub fn initialization(&self) -> Id<CallTarget> {
        self.initialization
    }
    pub fn enumeration(&self) -> Id<SignatureEnumerationObservation> {
        self.enumeration
    }
    pub fn outcomes(&self) -> &[ProtocolSignatureOutcome] {
        &self.outcomes
    }
    pub fn entry_value(&self) -> ContextValue {
        self.entry
    }
    pub fn premises(&self) -> &Rows<analysis::native::NativeAssertionPremise> {
        &self.premises
    }
    pub fn status(&self) -> analysis::policy::EvidenceStatus {
        self.status
    }
    /// Suppression is evaluated only against exact classes resolved at the actual bound
    /// handler expressions. A catalog formal name never supplies a runtime class value.
    pub fn handler_occurrences(
        &self,
        data: &ModelApplicationData,
        budget: &ResourceBudget,
    ) -> Result<Result<checked_handlers::HandlerOccurrences, ObligationKind>, ModelError> {
        let mut out = checked_handlers::HandlerOccurrences {
            rows: Vec::new(),
            _charge: charged::StateCharge::new(budget, "context-handler-occurrences"),
        };
        let models::ContextExit::SuppressClasses { formal } =
            &self.protocol.compiled().model().exit
        else {
            return Ok(Ok(out));
        };
        let Some(variant) = self.outcomes.iter().find(|o| o.outcome.is_ok()) else {
            return Ok(Err(ObligationKind::MissingEvidence));
        };
        let mut found = false;
        for argument in &variant.arguments {
            let Some(parameter) = data.bindings.parameters.get(argument.formal) else {
                return Ok(Err(ObligationKind::MissingEvidence));
            };
            let Some(shape) = data.bindings.shapes.get(parameter.shape) else {
                return Ok(Err(ObligationKind::MissingEvidence));
            };
            if shape.name.as_ref().is_none_or(|n| n.as_str() != formal) {
                continue;
            }
            found = true;
            match &argument.source {
                ProtocolArgumentSource::Value(BindingSource::EmptyVarargs) => {}
                ProtocolArgumentSource::Value(BindingSource::Actual { occurrence }) => {
                    if !matches!(argument.projection, BindingProjection::Positional { .. }) {
                        return Ok(Err(ObligationKind::UnsupportedUnpacking));
                    }
                    out._charge.grow(size_of::<Id<source::Occurrence>>() * 2)?;
                    out.rows.push(*occurrence);
                }
                _ => return Ok(Err(ObligationKind::OutsideProviderModel)),
            }
        }
        if !found {
            return Ok(Err(ObligationKind::MissingEvidence));
        }
        Ok(Ok(out))
    }
    pub fn derive(
        protocol: &'a CheckedContextProtocol<'a>,
        data: &ModelApplicationData,
        site: Id<source::Occurrence>,
        owner: Id<normalized::entities::EntityRef>,
        input: Id<input::InputRevision>,
        context: Id<AnalysisContext>,
        budget: &ResourceBudget,
    ) -> Result<Result<Self, ObligationKind>, ModelError> {
        normalized::binding_normalization::verify_enumerations(&data.bindings, budget)?;
        let mut charge = charged::StateCharge::new(budget, "checked-context-construction");
        charge.grow(size_of::<Self>())?;
        let setup = (|| {
            let b = &data.bindings;
            let occurrence = need(&b.occurrences, site)?;
            if occurrence.syntax_kind != source::SyntaxKind::ExprCall
                || need(&b.artifacts, occurrence.source)?.input != input
                || need(&b.symbols, protocol.class().symbol())?.context != context
            {
                return Err(ObligationKind::IncompatibleContexts);
            }
            let ownership = one(b.owners.iter().filter(|o| o.occurrence == site))?;
            if ownership.entity != owner {
                return Err(ObligationKind::ScopeBoundary);
            }
            let syntax = one(b.syntax.iter().filter(|s| {
                s.site == site
                    && b.qualifications
                        .get(s.qualification)
                        .is_some_and(|q| q.context == context)
            }))?;
            super::model_application::exact(b, syntax.qualification, context)?;
            let phase = |phase, symbol| -> Result<_, ObligationKind> {
                let resolution = one(b.resolutions.iter().filter(|r| {
                    r.site == site
                        && r.phase == phase
                        && b.channels.get(r.channel) == Some(&CallChannel::Direct)
                        && b.qualifications
                            .get(r.qualification)
                            .is_some_and(|q| q.context == context)
                }))?;
                if !resolution.complete {
                    return Err(ObligationKind::IncompleteCoverage);
                }
                let member = one(b.members.iter().filter(|m| m.resolution == resolution.id()))?;
                let target = need(&b.targets, member.target)?;
                if target.site != site
                    || target.phase != phase
                    || target.origin != resolution.origin
                    || b.destinations.get(target.destination)
                        != Some(&CallDestination::Resolved { symbol })
                {
                    return Err(ObligationKind::MissingEvidence);
                }
                Ok((resolution, target))
            };
            if !matches!(&protocol.compiled().model().allocation,models::Target::Stdlib{module,callable,..}if module=="builtins"&&callable=="object.__new__")
            {
                return Err(ObligationKind::OutsideProviderModel);
            }
            let (allocation_resolution, allocation) = phase(CallPhase::New, protocol.allocation())?;
            let (initialization_resolution, initialization) =
                phase(CallPhase::Init, protocol.initialization())?;
            if allocation.static_method != Some(true)
                || allocation.class_method != Some(false)
                || allocation.passing != Some(ReceiverPassing::NotPassed)
                || b.receivers.get(allocation.receiver) != Some(&Receiver::None)
            {
                return Err(ObligationKind::CallTransfer);
            }
            // The native initializer receiver class binds the actual requested constructor class.
            // An inherited initializer alone cannot establish an overridden subclass protocol.
            if initialization.receiver_class != Some(protocol.class().symbol()) {
                return Err(ObligationKind::MissingEvidence);
            }
            if initialization.static_method != Some(false)
                || initialization.class_method != Some(false)
                || initialization.passing != Some(ReceiverPassing::Object)
                || !matches!(
                    b.receivers.get(initialization.receiver),
                    Some(Receiver::Unknown { .. })
                )
            {
                return Err(ObligationKind::CallTransfer);
            }
            let allocation_traits = one(b
                .traits
                .iter()
                .filter(|t| t.symbol == protocol.allocation()))?;
            let initialization_traits = one(b
                .traits
                .iter()
                .filter(|t| t.symbol == protocol.initialization()))?;
            if !allocation_traits.staticmethod
                || allocation_traits.classmethod
                || initialization_traits.staticmethod
                || initialization_traits.classmethod
                || initialization_traits.defining_class != Some(protocol.class().symbol())
                || allocation_traits.origin != symbols::FunctionOrigin::DefStatement
                || initialization_traits.origin != symbols::FunctionOrigin::DefStatement
            {
                return Err(ObligationKind::OutsideProviderModel);
            }
            // Forwarded user arguments are not an ordinary bind to cls-only object.__new__.
            // The selected protocol owns this allocation rule for its admitted exact initializer.
            let allocation_enumeration = one(b.signature_enumerations.iter().filter(|e| {
                e.symbol == protocol.allocation()
                    && e.complete
                    && b.qualifications
                        .get(e.qualification)
                        .is_some_and(|q| q.context == context)
            }))?;
            let allocation_member = one(b
                .signature_enumeration_members
                .iter()
                .filter(|m| m.enumeration == allocation_enumeration.id()))?;
            let allocation_signature = need(&b.signatures, allocation_member.signature)?;
            let parameter = one(b
                .parameters
                .iter()
                .filter(|p| p.signature == allocation_signature.id()))?;
            let shape = need(&b.shapes, parameter.shape)?;
            if allocation_signature.form != SignatureForm::List
                || shape.name.as_ref().map(|n| n.as_str()) != Some("cls")
                || !shape.required
                || shape.kind != ParameterKind::PositionalOrKeyword
            {
                return Err(ObligationKind::OutsideProviderModel);
            }
            let enumeration = one(b.signature_enumerations.iter().filter(|e| {
                e.symbol == protocol.initialization()
                    && e.complete
                    && b.qualifications
                        .get(e.qualification)
                        .is_some_and(|q| q.context == context)
            }))?;
            Ok((
                syntax,
                allocation_resolution,
                allocation,
                initialization_resolution,
                initialization,
                allocation_traits,
                initialization_traits,
                allocation_enumeration,
                allocation_signature,
                enumeration,
            ))
        })();
        let (
            syntax,
            ar,
            allocation,
            ir,
            initialization,
            at,
            it,
            ae,
            allocation_signature,
            enumeration,
        ) = match setup {
            Ok(v) => v,
            Err(r) => return Ok(Err(r)),
        };
        let mut premises = Rows::new(budget);
        let mut status = protocol.status();
        for premise in protocol.premises().iter() {
            premises.insert(premise.clone())?;
        }
        let evidence = (|| -> Result<(), RuntimeEvidenceError> {
            for (reference, q) in [
                (derivation::RowRef::of(syntax.id()), syntax.qualification),
                (derivation::RowRef::of(ar.id()), ar.qualification),
                (
                    derivation::RowRef::of(allocation.id()),
                    allocation.qualification,
                ),
                (derivation::RowRef::of(ir.id()), ir.qualification),
                (
                    derivation::RowRef::of(initialization.id()),
                    initialization.qualification,
                ),
                (derivation::RowRef::of(at.id()), at.qualification),
                (derivation::RowRef::of(it.id()), it.qualification),
                (derivation::RowRef::of(ae.id()), ae.qualification),
                (
                    derivation::RowRef::of(allocation_signature.id()),
                    allocation_signature.qualification,
                ),
                (
                    derivation::RowRef::of(enumeration.id()),
                    enumeration.qualification,
                ),
            ] {
                append_native_evidence(data, reference, q, context, &mut premises, &mut status)?;
            }
            Ok(())
        })();
        match evidence {
            Err(RuntimeEvidenceError::Boundary(r)) => return Ok(Err(r)),
            Err(RuntimeEvidenceError::Model(e)) => return Err(e),
            Ok(()) => {}
        }
        let b = &data.bindings;
        let count = b.arguments.iter().filter(|a| a.call == syntax.id()).count();
        charge.grow(count.saturating_mul(size_of::<CallArgument>() + 512))?;
        let mut arguments = b
            .arguments
            .iter()
            .filter(|a| a.call == syntax.id())
            .cloned()
            .collect::<Vec<_>>();
        arguments.sort_by_key(|a| a.ordinal);
        charge.grow(arguments.iter().map(HeapSize::heap_bytes).sum())?;
        let mut outcomes = Vec::new();
        let mut entry = None;
        let mut compatible = false;
        for member in b
            .signature_enumeration_members
            .iter()
            .filter(|m| m.enumeration == enumeration.id())
        {
            let signature = match need(&b.signatures, member.signature) {
                Ok(s) => s,
                Err(r) => return Ok(Err(r)),
            };
            match append_native_evidence(
                data,
                derivation::RowRef::of(signature.id()),
                signature.qualification,
                context,
                &mut premises,
                &mut status,
            ) {
                Err(RuntimeEvidenceError::Boundary(r)) => return Ok(Err(r)),
                Err(RuntimeEvidenceError::Model(e)) => return Err(e),
                Ok(()) => {}
            }
            let bytes = b
                .parameters
                .iter()
                .filter(|p| p.signature == signature.id())
                .count()
                .saturating_mul(
                    size_of::<SignatureParameter>() + size_of::<ParameterShape>() + 512,
                );
            let _scratch = budget.reserve(
                "protocol-argument-assignment",
                bytes
                    .saturating_add(count.saturating_mul(1024))
                    .saturating_add(4096),
            )?;
            let mut parameters = b
                .parameters
                .iter()
                .filter(|p| p.signature == signature.id())
                .cloned()
                .collect::<Vec<_>>();
            parameters.sort_by_key(|p| p.ordinal);
            let mut shapes = BTreeMap::new();
            for parameter in &parameters {
                let shape = match need(&b.shapes, parameter.shape) {
                    Ok(s) => s,
                    Err(r) => return Ok(Err(r)),
                };
                shapes.insert(shape.id(), shape.clone());
            }
            let assigned =
                assign_arguments(signature, &parameters, &shapes, syntax, &arguments, true);
            let mut retained = Vec::new();
            let outcome = match assigned {
                Err(failure) => {
                    if failure.class != BindingFailureClass::ProvenIncompatible {
                        return Ok(Err(failure.reason));
                    }
                    Err(failure)
                }
                Ok(assigned) => {
                    if let Some(previous) = outcomes
                        .iter()
                        .find(|o: &&ProtocolSignatureOutcome| o.outcome.is_ok())
                    {
                        if b.signatures.get(previous.signature).is_none_or(|s| {
                            s.parameters != signature.parameters || s.form != signature.form
                        }) {
                            return Ok(Err(ObligationKind::AmbiguousBinding));
                        }
                    }
                    compatible = true;
                    charge.grow(
                        assigned
                            .capacity()
                            .saturating_mul(size_of::<ProtocolArgument>() + 512),
                    )?;
                    for a in assigned {
                        retained.push(ProtocolArgument {
                            formal: a.formal,
                            source: match a.source {
                                ArgumentSource::Receiver => ProtocolArgumentSource::Receiver,
                                ArgumentSource::Value(v) => ProtocolArgumentSource::Value(v),
                            },
                            kind: a.kind,
                            projection: a.projection,
                        });
                    }
                    let value = match &protocol.compiled().model().entry {
                        models::ContextEntry::NoneValue => ContextValue::None,
                        models::ContextEntry::ArgumentOrNone { formal } => {
                            let parameter = parameters.iter().find(|p| {
                                shapes.get(&p.shape).is_some_and(|s| {
                                    s.name.as_ref().is_some_and(|n| n.as_str() == formal)
                                })
                            });
                            match parameter {
                                None => ContextValue::None,
                                Some(parameter) => {
                                    let Some(a) =
                                        retained.iter().find(|a| a.formal == parameter.id())
                                    else {
                                        return Ok(Err(ObligationKind::MissingEvidence));
                                    };
                                    match (&a.source, &a.projection) {
                                        (
                                            ProtocolArgumentSource::Value(BindingSource::Actual {
                                                occurrence,
                                            }),
                                            BindingProjection::Whole,
                                        ) => ContextValue::Actual(*occurrence),
                                        (
                                            ProtocolArgumentSource::Value(BindingSource::Default),
                                            BindingProjection::Whole,
                                        ) => ContextValue::None,
                                        _ => return Ok(Err(ObligationKind::UnsupportedUnpacking)),
                                    }
                                }
                            }
                        }
                    };
                    if entry.is_some_and(|e| e != value) {
                        return Ok(Err(ObligationKind::AmbiguousBinding));
                    }
                    entry = Some(value);
                    Ok(())
                }
            };
            charge.grow(size_of::<ProtocolSignatureOutcome>() * 2)?;
            outcomes.push(ProtocolSignatureOutcome {
                signature: signature.id(),
                outcome,
                arguments: retained,
            });
        }
        if !compatible {
            return Ok(Err(ObligationKind::AmbiguousBinding));
        }
        let resource = ConstructedResource {
            site,
            owner,
            context,
            class: protocol.class().symbol(),
        };
        Ok(Ok(Self {
            protocol,
            input,
            arguments,
            resource,
            allocation: allocation.id(),
            initialization: initialization.id(),
            enumeration: enumeration.id(),
            outcomes,
            entry: entry.unwrap(),
            premises,
            status,
            _charge: charge,
        }))
    }
}
pub mod checked_handlers {
    use super::*;
    pub struct HandlerOccurrences {
        pub(super) rows: Vec<Id<source::Occurrence>>,
        pub(super) _charge: charged::StateCharge,
    }
    impl HandlerOccurrences {
        pub fn occurrences(&self) -> &[Id<source::Occurrence>] {
            &self.rows
        }
    }
    pub struct CheckedContextHandler<'a> {
        resource: ConstructedResource,
        actual: Id<source::Occurrence>,
        class: super::super::model_context::CheckedExactClass<'a>,
        lookup: super::super::builtin_read::CheckedBuiltinRead,
    }
    impl<'a> CheckedContextHandler<'a> {
        pub fn actual(&self) -> Id<source::Occurrence> {
            self.actual
        }
        pub fn class(&self) -> &super::super::model_context::CheckedExactClass<'a> {
            &self.class
        }
        pub fn lookup(&self) -> &super::super::builtin_read::CheckedBuiltinRead {
            &self.lookup
        }
        pub fn derive(
            construction: &CheckedContextConstruction<'_>,
            data: &'a ModelApplicationData,
            evaluation: &super::super::evaluation::EvaluationData,
            input: Id<input::InputRevision>,
            actual: Id<source::Occurrence>,
            budget: &ResourceBudget,
        ) -> Result<Result<Self, ObligationKind>, ModelError> {
            if input != construction.input {
                return Ok(Err(ObligationKind::IncompatibleContexts));
            }
            let occurrences = match construction.handler_occurrences(data, budget)? {
                Ok(o) => o,
                Err(r) => return Ok(Err(r)),
            };
            if !occurrences.occurrences().contains(&actual) {
                return Ok(Err(ObligationKind::IncompatibleContexts));
            }
            let request = super::super::evaluation::ExpressionRequest {
                input,
                context: construction.resource.context,
                owner: construction.resource.owner,
                expression: actual,
            };
            let lookup = match super::super::builtin_read::CheckedBuiltinRead::derive(
                evaluation, request, budget,
            )? {
                Ok(p) => p,
                Err(r) => return Ok(Err(r)),
            };
            let reference = match one(evaluation.references.iter().filter(|r| {
                r.read == actual
                    && evaluation
                        .qualifications
                        .get(r.qualification)
                        .is_some_and(|q| q.context == request.context)
            })) {
                Ok(r) => r,
                Err(r) => return Ok(Err(r)),
            };
            let symbol=match one(data.bindings.symbols.iter().filter(|s|s.name==reference.name&&s.context==request.context&&s.kind==SymbolKind::Class&&matches!(data.bindings.provider_modules.get(s.module),Some(ProviderModule::Bundled{name,bundle:ModuleBundle::Typeshed,..})if name=="builtins"))){Ok(s)=>s,Err(r)=>return Ok(Err(r))};
            let class = match super::super::model_context::CheckedExactClass::derive(
                data,
                symbol.id(),
                request.context,
                budget,
            )? {
                Ok(c) => c,
                Err(r) => return Ok(Err(r)),
            };
            Ok(Ok(Self {
                resource: construction.resource,
                actual,
                class,
                lookup,
            }))
        }
    }
    impl CheckedContextConstruction<'_> {
        pub fn suppresses(
            &self,
            data: &ModelApplicationData,
            raised: &super::super::model_context::CheckedExactClass<'_>,
            handlers: &[&CheckedContextHandler<'_>],
            budget: &ResourceBudget,
        ) -> Result<Result<bool, ObligationKind>, ModelError> {
            let occurrences = match self.handler_occurrences(data, budget)? {
                Ok(o) => o,
                Err(r) => return Ok(Err(r)),
            };
            if handlers.len() != occurrences.occurrences().len()
                || handlers
                    .iter()
                    .zip(occurrences.occurrences())
                    .any(|(h, actual)| h.resource != self.resource || h.actual != *actual)
            {
                return Ok(Err(ObligationKind::IncompatibleContexts));
            }
            for handler in handlers {
                match raised.matches(handler.class()) {
                    Ok(true) => return Ok(Ok(true)),
                    Ok(false) => {}
                    Err(r) => return Ok(Err(r)),
                }
            }
            Ok(Ok(false))
        }
    }
}
