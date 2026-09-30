//! Native Pysa call events lowered directly into attributed model records (plan A10).
#![deny(clippy::wildcard_enum_match_arm)]
use crate::{natives::Natives, symbol_records::Locator, syntax_records::Spans};
use lctx_model::domain::{
    assertion::AssertionQualification, attribution::*, calls::*, charged::StateCharge,
    lexical::SyntaxField, obligation::ObligationKind, resources::ResourceBudget,
    source::SyntaxKind, *,
};
use pyrefly::report::pysa::{
    PysaModuleCallGraphs,
    call_graph::{
        CallCallees, ExpressionCallees, ExpressionIdentifier, ImplicitReceiver, OriginKind,
        PysaCallTarget, Target, Unresolved, UnresolvedReason,
    },
    function::{FunctionId, FunctionRef},
};
use std::collections::BTreeSet;

pub type ResolveFunction<'a> =
    dyn FnMut(&mut Natives, &FunctionRef) -> Result<Id<ProviderSymbol>, ModelError> + 'a;
pub struct Records {
    charge: StateCharge,
    pub callers: Vec<ProviderCallable>,
    pub origins: Vec<CallOrigin>,
    pub steps: Vec<CallOriginStep>,
    pub sites: Vec<ProviderCallSite>,
    pub normalized: Vec<NormalizedSite>,
    pub unattached: Vec<UnattachedEvent>,
    pub boundaries: Vec<(
        Option<Id<lctx_model::domain::source::Occurrence>>,
        ObligationKind,
        String,
    )>,
}
pub struct UnattachedEvent {
    pub range: ruff_text_size::TextRange,
    pub syntax_kind: Option<SyntaxKind>,
    pub candidates: Vec<Id<lctx_model::domain::source::Occurrence>>,
    pub reason: ObligationKind,
    pub detail: String,
}
impl Records {
    fn new(budget: &ResourceBudget) -> Self {
        Self {
            charge: StateCharge::new(budget, "native_call_records"),
            callers: vec![],
            origins: vec![],
            steps: vec![],
            sites: vec![],
            normalized: vec![],
            unattached: vec![],
            boundaries: vec![],
        }
    }
    fn hold<T: HeapSize>(&mut self, row: &T) -> Result<(), ModelError> {
        self.charge.grow(
            size_of::<T>()
                .saturating_mul(4)
                .saturating_add(row.heap_bytes()),
        )
    }
    fn boundary(
        &mut self,
        subject: Option<Id<lctx_model::domain::source::Occurrence>>,
        reason: ObligationKind,
        detail: String,
    ) -> Result<(), ModelError> {
        self.hold(&detail)?;
        self.charge.grow(128)?;
        self.boundaries.push((subject, reason, detail));
        Ok(())
    }
}
fn invalid(message: impl Into<String>) -> ModelError {
    ModelError::Invalid(message.into())
}
fn steps(kind: &OriginKind, out: &mut Vec<(OriginStep, Option<i64>)>) -> Result<(), ModelError> {
    if out.len() >= MAX_ORIGIN_STEPS {
        return Err(invalid("call origin exceeds work limit"));
    }
    let step = match kind {
        OriginKind::GetAttrConstantLiteral => OriginStep::GetAttrConstantLiteral,
        OriginKind::ComparisonOperator => OriginStep::Comparison,
        OriginKind::GeneratorIter => OriginStep::GeneratorIter,
        OriginKind::GeneratorNext => OriginStep::GeneratorNext,
        OriginKind::WithEnter => OriginStep::WithEnter,
        OriginKind::ForDecoratedTarget => OriginStep::ForDecoratedTarget,
        OriginKind::SubscriptGetItem => OriginStep::SubscriptGetItem,
        OriginKind::SubscriptSetItem => OriginStep::SubscriptSetItem,
        OriginKind::BinaryOperator => OriginStep::BinaryOperator,
        OriginKind::AugmentedAssignDunderCall => OriginStep::AugmentedAssignDunderCall,
        OriginKind::AugmentedAssignRHS => OriginStep::AugmentedAssignRhs,
        OriginKind::AugmentedAssignStatement => OriginStep::AugmentedAssignStatement,
        OriginKind::ForIter => OriginStep::ForIter,
        OriginKind::ForNext => OriginStep::ForNext,
        OriginKind::ForAssign => OriginStep::ForAssign,
        OriginKind::ReprCall => OriginStep::ReprCall,
        OriginKind::AbsCall => OriginStep::AbsCall,
        OriginKind::IterCall => OriginStep::IterCall,
        OriginKind::NextCall => OriginStep::NextCall,
        OriginKind::StrCallToDunderMethod => OriginStep::StrCallToDunderMethod,
        OriginKind::Slice => OriginStep::Slice,
        OriginKind::ChainedAssign { index } => {
            out.push((
                OriginStep::ChainedAssign,
                Some(
                    i64::try_from(*index)
                        .map_err(|_| invalid("chained assignment index overflow"))?,
                ),
            ));
            return Ok(());
        }
        OriginKind::Nested { head, tail } => {
            steps(tail, out)?;
            return steps(head, out);
        }
    };
    out.push((step, None));
    Ok(())
}
fn reason(reason: UnresolvedReason) -> PysaUnresolvedReason {
    match reason {
        UnresolvedReason::LambdaArgument => PysaUnresolvedReason::LambdaArgument,
        UnresolvedReason::UnexpectedPyreflyTarget => PysaUnresolvedReason::UnexpectedPyreflyTarget,
        UnresolvedReason::EmptyPyreflyCallTarget => PysaUnresolvedReason::EmptyPyreflyCallTarget,
        UnresolvedReason::UnknownClassField => PysaUnresolvedReason::UnknownClassField,
        UnresolvedReason::ClassFieldOnlyExistInObject => {
            PysaUnresolvedReason::ClassFieldOnlyExistInObject
        }
        UnresolvedReason::UnsupportedFunctionTarget => {
            PysaUnresolvedReason::UnsupportedFunctionTarget
        }
        UnresolvedReason::UnexpectedDefiningClass => PysaUnresolvedReason::UnexpectedDefiningClass,
        UnresolvedReason::UnexpectedInitMethod => PysaUnresolvedReason::UnexpectedInitMethod,
        UnresolvedReason::UnexpectedNewMethod => PysaUnresolvedReason::UnexpectedNewMethod,
        UnresolvedReason::UnexpectedCalleeExpression => {
            PysaUnresolvedReason::UnexpectedCalleeExpression
        }
        UnresolvedReason::UnresolvedMagicDunderAttr => {
            PysaUnresolvedReason::UnresolvedMagicDunderAttr
        }
        UnresolvedReason::UnresolvedMagicDunderAttrDueToNoBase => {
            PysaUnresolvedReason::UnresolvedMagicDunderAttrDueToNoBase
        }
        UnresolvedReason::UnresolvedMagicDunderAttrDueToNoAttribute => {
            PysaUnresolvedReason::UnresolvedMagicDunderAttrDueToNoAttribute
        }
        UnresolvedReason::Mixed => PysaUnresolvedReason::Mixed,
    }
}
fn rest(unresolved: &Unresolved) -> Option<PysaUnresolvedReason> {
    match unresolved {
        Unresolved::False => None,
        Unresolved::True(r) => Some(reason(*r)),
    }
}
fn symbol(
    natives: &Natives,
    module: Id<ProviderModule>,
    key: &str,
) -> Result<Id<ProviderSymbol>, ModelError> {
    natives
        .symbols
        .values()
        .find(|s| s.module == module && s.native_key == key)
        .map(Record::id)
        .ok_or_else(|| invalid(format!("Pysa graph owner {key} has no native definition")))
}
fn caller(
    id: &FunctionId,
    module: Id<ProviderModule>,
    natives: &Natives,
    qualification: &AssertionQualification,
    provider: Id<Provider>,
) -> Result<ProviderCallable, ModelError> {
    Ok(match id {
        FunctionId::ModuleTopLevel => ProviderCallable::ModuleBody {
            provider,
            context: qualification.context,
            module,
        },
        FunctionId::ClassTopLevel { class_id } => ProviderCallable::ClassBody {
            class: symbol(natives, module, &class_id.to_int().to_string())?,
        },
        FunctionId::FunctionDecoratedTarget { func_def_index } => {
            ProviderCallable::DecoratorApplication {
                function: symbol(natives, module, &format!("F:{}", func_def_index.0))?,
            }
        }
        FunctionId::Function { .. } | FunctionId::ClassField { .. } => ProviderCallable::Symbol {
            symbol: symbol(natives, module, &id.serialize_to_string())?,
        },
    })
}
struct Event<'a, 'r> {
    actual: Option<Id<lctx_model::domain::source::Occurrence>>,
    callees: Vec<NativeCallee>,
    complete: BTreeSet<(Id<CallChannel>, CallPhase)>,
    natives: &'a mut Natives,
    resolve: &'a mut ResolveFunction<'r>,
    out: &'a mut Records,
}
impl Event<'_, '_> {
    fn target(
        &mut self,
        target: &PysaCallTarget<FunctionRef>,
        phase: CallPhase,
        channel: &CallChannel,
        modality: Modality,
    ) -> Result<(), ModelError> {
        let destination = match &target.target {
            Target::Function(function) => {
                if let FunctionId::FunctionDecoratedTarget { func_def_index } = function.function_id
                {
                    let base = FunctionRef {
                        function_id: FunctionId::Function { func_def_index },
                        ..function.clone()
                    };
                    let callable = ProviderCallable::DecoratorApplication {
                        function: (self.resolve)(self.natives, &base)?,
                    };
                    self.out.hold(&callable)?;
                    self.out.callers.push(callable.clone());
                    CallDestination::Callable {
                        callable: callable.id(),
                    }
                } else {
                    CallDestination::Resolved {
                        symbol: (self.resolve)(self.natives, function)?,
                    }
                }
            }
            Target::Overrides(function) => CallDestination::Overrides {
                symbol: (self.resolve)(self.natives, function)?,
            },
            Target::FormatString => CallDestination::SyntheticFormatting,
        };
        let receiver_class = target
            .receiver_class
            .as_ref()
            .map(|class| {
                let module = class.class.module();
                let module = self
                    .natives
                    .module(&module.name().to_string(), module.path())?;
                self.natives.symbol(
                    module,
                    class.class_id.to_int().to_string(),
                    class.class.name().to_string(),
                    SymbolKind::Class,
                )
            })
            .transpose()?;
        let passing = match target.implicit_receiver {
            ImplicitReceiver::False => ReceiverPassing::NotPassed,
            ImplicitReceiver::TrueWithClassReceiver => ReceiverPassing::Class,
            ImplicitReceiver::TrueWithObjectReceiver => ReceiverPassing::Object,
        };
        let modality = if matches!(destination, CallDestination::Overrides { .. })
            && modality == Modality::Definite
        {
            Modality::Candidate
        } else {
            modality
        };
        self.out.charge.grow(
            512usize
                .saturating_add(destination.heap_bytes())
                .saturating_add(channel.heap_bytes()),
        )?;
        self.callees.push(NativeCallee {
            phase,
            channel: channel.clone(),
            destination,
            receiver: ReceiverEvidence {
                passing: Some(passing),
                static_method: Some(target.is_static_method),
                class_method: Some(target.is_class_method),
                actual: self.actual,
            },
            implicit: target.implicit_dunder_call,
            modality,
            receiver_class,
        });
        Ok(())
    }
    fn list(
        &mut self,
        targets: &[PysaCallTarget<FunctionRef>],
        phase: CallPhase,
        channel: CallChannel,
        remainder: Option<PysaUnresolvedReason>,
        potential: bool,
        conditional: bool,
    ) -> Result<(), ModelError> {
        let modality = if potential {
            Modality::Potential
        } else if targets.len() == 1 && remainder.is_none() && !conditional {
            Modality::Definite
        } else {
            Modality::Candidate
        };
        for target in targets {
            self.target(target, phase, &channel, modality)?;
        }
        if let Some(native) = remainder {
            self.out.charge.grow(512)?;
            self.callees.push(NativeCallee {
                phase,
                channel: channel.clone(),
                destination: CallDestination::Unresolved {
                    reason: ObligationKind::UnresolvedTarget,
                    native: Some(native),
                },
                receiver: ReceiverEvidence {
                    passing: None,
                    static_method: None,
                    class_method: None,
                    actual: None,
                },
                implicit: false,
                modality: if potential {
                    Modality::Potential
                } else {
                    Modality::Candidate
                },
                receiver_class: None,
            });
        }
        if remainder.is_none() && !targets.is_empty() {
            self.out.charge.grow(128)?;
            self.complete.insert((channel.id(), phase));
        }
        Ok(())
    }
    fn call(
        &mut self,
        calls: &CallCallees<FunctionRef>,
        potential: bool,
    ) -> Result<(), ModelError> {
        // The unresolved direct remainder belongs to Call, not every constructor phase.
        self.list(
            &calls.call_targets,
            CallPhase::Call,
            CallChannel::Direct,
            rest(&calls.unresolved),
            potential,
            false,
        )?;
        self.list(
            &calls.init_targets,
            CallPhase::Init,
            CallChannel::Direct,
            None,
            potential,
            false,
        )?;
        self.list(
            &calls.new_targets,
            CallPhase::New,
            CallChannel::Direct,
            None,
            potential,
            false,
        )?;
        let mut higher: Vec<_> = calls.higher_order_parameters.values().collect();
        higher.sort_by_key(|h| h.index);
        for h in higher {
            self.list(
                &h.call_targets,
                CallPhase::Call,
                CallChannel::HigherOrder {
                    argument_index: i64::from(h.index),
                },
                rest(&h.unresolved),
                true,
                false,
            )?;
        }
        Ok(())
    }
}
/// One analyzed module, using only its retained parse and native graphs. Missing or ambiguous
/// exact ranges are reported, never attached to an innermost occurrence or by name.
#[allow(
    clippy::too_many_arguments,
    reason = "Shared generated contracts and fixtures require this scoped exception"
)]
pub fn records(
    graphs: &PysaModuleCallGraphs,
    module: Id<ProviderModule>,
    provider: Id<Provider>,
    qualification: &AssertionQualification,
    locator: &Locator<'_>,
    spans: &Spans,
    calls: &[(CallSyntax, Vec<CallArgument>)],
    natives: &mut Natives,
    resolve: &mut ResolveFunction<'_>,
    budget: &ResourceBudget,
) -> Result<Records, ModelError> {
    let mut out = Records::new(budget);
    let mut regular = BTreeSet::new();
    let mut graphs: Vec<_> = graphs.call_graphs.iter().collect();
    graphs.sort_by(|a, b| a.0.cmp(b.0));
    for (id, graph) in graphs {
        let native_caller = caller(id, module, natives, qualification, provider)?;
        out.hold(&native_caller)?;
        out.callers.push(native_caller.clone());
        let mut sites: Vec<_> = graph.as_map().iter().collect();
        sites.sort_by(|a, b| a.0.cmp(b.0));
        for (identifier, callees) in sites {
            if matches!(
                callees,
                ExpressionCallees::Define(_) | ExpressionCallees::Return(_)
            ) {
                continue;
            }
            let (kind, location, origin_steps) = match identifier {
                ExpressionIdentifier::Regular(location) => {
                    (PysaSiteKind::Regular, location, vec![])
                }
                ExpressionIdentifier::Identifier { location, .. } => {
                    (PysaSiteKind::Identifier, location, vec![])
                }
                ExpressionIdentifier::ArtificialCall(origin) => {
                    let mut v = vec![];
                    steps(&origin.kind, &mut v)?;
                    (PysaSiteKind::ArtificialCall, &origin.location, v)
                }
                ExpressionIdentifier::ArtificialAttributeAccess(origin) => {
                    let mut v = vec![];
                    steps(&origin.kind, &mut v)?;
                    (PysaSiteKind::ArtificialAttributeAccess, &origin.location, v)
                }
                ExpressionIdentifier::FormatStringArtificial(location) => (
                    PysaSiteKind::FormatStringArtificial,
                    location,
                    vec![(OriginStep::FormatStringArtificial, None)],
                ),
                ExpressionIdentifier::FormatStringStringify(location) => (
                    PysaSiteKind::FormatStringStringify,
                    location,
                    vec![(OriginStep::FormatStringStringify, None)],
                ),
            };
            let callee = match callees {
                ExpressionCallees::Call(_) => PysaCalleeKind::Call,
                ExpressionCallees::Identifier(_) => PysaCalleeKind::Identifier,
                ExpressionCallees::AttributeAccess(_) => PysaCalleeKind::AttributeAccess,
                ExpressionCallees::FormatStringArtificial(_) => {
                    PysaCalleeKind::FormatStringArtificial
                }
                ExpressionCallees::FormatStringStringify(_) => {
                    PysaCalleeKind::FormatStringStringify
                }
                ExpressionCallees::Define(_) | ExpressionCallees::Return(_) => {
                    unreachable!("filtered non-call facts")
                }
            };
            let range = locator.range(location);
            let syntax_kind = match kind {
                PysaSiteKind::Identifier => Some(SyntaxKind::ExprName),
                PysaSiteKind::Regular => Some(if callee == PysaCalleeKind::AttributeAccess {
                    SyntaxKind::ExprAttribute
                } else {
                    SyntaxKind::ExprCall
                }),
                PysaSiteKind::FormatStringArtificial => Some(SyntaxKind::ExprFString),
                PysaSiteKind::ArtificialCall
                | PysaSiteKind::ArtificialAttributeAccess
                | PysaSiteKind::FormatStringStringify => None,
            };
            let Some(site) = spans.event(range, syntax_kind) else {
                let candidates = spans.event_candidates(range, syntax_kind);
                let reason = if candidates.len() > 1 {
                    ObligationKind::AttachmentAmbiguous
                } else {
                    ObligationKind::AttachmentUnmatched
                };
                out.charge
                    .grow(512usize.saturating_add(candidates.len().saturating_mul(64)))?;
                let detail =
                    format!("Pysa {kind:?}/{callee:?} has no unique exact occurrence at {range:?}");
                out.hold(&detail)?;
                out.unattached.push(UnattachedEvent {
                    range,
                    syntax_kind,
                    candidates,
                    reason,
                    detail,
                });
                continue;
            };
            if kind == PysaSiteKind::Regular && callee == PysaCalleeKind::Call {
                out.charge.grow(128)?;
                regular.insert(site);
            }
            let (origin, members) = CallOrigin::new(&origin_steps)?;
            out.hold(&origin)?;
            for row in &members {
                out.hold(row)?;
            }
            out.origins.push(origin.clone());
            out.steps.extend(members);
            let is_attribute = match callees {
                ExpressionCallees::AttributeAccess(a) => Some(a.is_attribute),
                ExpressionCallees::Call(_)
                | ExpressionCallees::Identifier(_)
                | ExpressionCallees::Define(_)
                | ExpressionCallees::FormatStringArtificial(_)
                | ExpressionCallees::FormatStringStringify(_)
                | ExpressionCallees::Return(_) => None,
            };
            let row = ProviderCallSite {
                qualification: qualification.id(),
                site,
                origin: origin.id(),
                kind,
                caller: native_caller.id(),
                callee,
                is_attribute,
            };
            out.hold(&row)?;
            out.sites.push(row);
            let callee_occurrence = calls
                .iter()
                .find(|(c, _)| c.site == site)
                .map(|(c, _)| c.callee)
                .unwrap_or(site);
            let actual = spans.child(callee_occurrence, SyntaxField::Value);
            let mut event = Event {
                actual,
                callees: vec![],
                complete: BTreeSet::new(),
                natives,
                resolve,
                out: &mut out,
            };
            match callees {
                ExpressionCallees::Call(c) => event.call(c, false)?,
                ExpressionCallees::Identifier(c) => event.call(&c.if_called, true)?,
                ExpressionCallees::AttributeAccess(c) => {
                    event.call(&c.if_called, true)?;
                    event.list(
                        &c.property_getters,
                        CallPhase::PropertyGet,
                        CallChannel::Direct,
                        None,
                        false,
                        c.is_attribute,
                    )?;
                    event.list(
                        &c.property_setters,
                        CallPhase::PropertySet,
                        CallChannel::Direct,
                        None,
                        false,
                        c.is_attribute,
                    )?;
                }
                ExpressionCallees::FormatStringArtificial(c) => event.list(
                    &c.targets,
                    CallPhase::Call,
                    CallChannel::Direct,
                    None,
                    false,
                    false,
                )?,
                ExpressionCallees::FormatStringStringify(c) => event.list(
                    &c.targets,
                    CallPhase::Call,
                    CallChannel::Direct,
                    rest(&c.unresolved),
                    false,
                    false,
                )?,
                ExpressionCallees::Define(_) | ExpressionCallees::Return(_) => {
                    unreachable!("filtered non-call facts")
                }
            }
            if !event.callees.is_empty() {
                let normalized = normalize_site(
                    qualification,
                    site,
                    origin.id(),
                    &event.callees,
                    &event.complete,
                    None,
                )?;
                macro_rules! hold { ($($field:ident),+) => { $(for row in &normalized.$field { event.out.hold(row)?; })+ }; }
                hold!(
                    qualifications,
                    channels,
                    destinations,
                    receivers,
                    targets,
                    resolutions,
                    members
                );
                event
                    .out
                    .charge
                    .grow(size_of::<NormalizedSite>().saturating_mul(4))?;
                event.out.normalized.push(normalized);
            }
        }
    }
    for (call, _) in calls {
        if !regular.contains(&call.site) {
            out.boundary(
                Some(call.site),
                if call.in_annotation {
                    ObligationKind::OutsideProviderModel
                } else {
                    ObligationKind::MissingEvidence
                },
                if call.in_annotation {
                    "call inside an annotation".into()
                } else {
                    "Ruff call has no native Pysa record".into()
                },
            )?;
        }
    }
    Ok(out)
}
