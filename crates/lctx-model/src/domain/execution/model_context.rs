//! Pinned class protocols and exact native exception ancestry. No ambient import is admissible.
use super::model_application::{
    ModelApplicationData, RuntimeEvidenceError, append_native_evidence,
};
use crate::domain::{
    attribution::{AnalysisContext, ObligationKind},
    calls::ProviderSymbol,
    models::{Catalog, CompiledContextProtocol, ContextExit},
    normalized::Rows,
    resources::ResourceBudget,
    symbols::{AncestryRelation, ClassAncestryObservation, Linearization},
    *,
};
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
/// Borrowed native class evidence used by both model application and source completion.
pub(crate) struct ClassEvidence<'a> {
    input: Option<Id<input::InputRevision>>,
    native_index: Option<&'a charged::ChargedMap<derivation::RowRef, Vec<Id<analysis::native::NativeQualification>>>>,
    symbol_supports: Option<&'a Rows<symbols::SymbolSupport>>,
    symbols: &'a Rows<ProviderSymbol>,
    ancestry: &'a Rows<ClassAncestryObservation>,
    ancestry_supports: &'a Rows<symbols::ClassAncestrySupport>,
    qualifications: &'a Rows<assertion::AssertionQualification>,
    runs: &'a Rows<attribution::ProviderRun>,
    sequences: &'a Rows<symbols::SymbolSequence>,
    sequence_members: &'a Rows<symbols::SymbolSequenceMember>,
    symbol_observations: &'a Rows<symbols::SymbolObservation>,
    native: &'a Rows<analysis::native::NativeQualification>,
    premises: &'a Rows<analysis::native::NativeAssertionPremise>,
}
impl<'a> ClassEvidence<'a> {
    pub(crate) fn evaluation(data: &'a super::evaluation::EvaluationData, input: Id<input::InputRevision>, native_index: &'a charged::ChargedMap<derivation::RowRef, Vec<Id<analysis::native::NativeQualification>>>) -> Self {
        Self { input: Some(input), native_index: Some(native_index), symbol_supports: Some(&data.symbol_supports), symbols: &data.symbols, ancestry: &data.ancestry, ancestry_supports: &data.ancestry_supports,
            qualifications: &data.qualifications, runs: &data.runs, sequences: &data.sequences,
            sequence_members: &data.sequence_members, symbol_observations: &data.symbol_observations,
            native: &data.native, premises: &data.premises }
    }
    fn application(data: &'a ModelApplicationData) -> Self {
        let b = &data.bindings;
        Self { input: None, native_index: None, symbol_supports: None, symbols: &b.symbols, ancestry: &b.ancestry, ancestry_supports: &b.ancestry_supports,
            qualifications: &b.qualifications, runs: &b.runs, sequences: &b.sequences,
            sequence_members: &b.sequence_members, symbol_observations: &b.symbol_observations,
            native: &data.native, premises: &data.premises }
    }
}
/// Complete native ancestry admits both a positive subclass match and an exact negative.
/// Its lifetime retains the input tables and the charged bounded ancestry inventory.
pub struct CheckedExactClass<'a> {
    symbol: &'a ProviderSymbol,
    mro: &'a ClassAncestryObservation,
    ancestors: charged::ChargedSet<Id<ProviderSymbol>>,
    premises: Rows<analysis::native::NativeAssertionPremise>,
    status: analysis::policy::EvidenceStatus,
    _charge: charged::StateCharge,
}
impl<'a> CheckedExactClass<'a> {
    pub fn symbol(&self) -> Id<ProviderSymbol> {
        self.symbol.id()
    }
    pub fn premises(&self) -> &Rows<analysis::native::NativeAssertionPremise> {
        &self.premises
    }
    pub fn status(&self) -> analysis::policy::EvidenceStatus {
        self.status
    }
    pub fn ancestry(&self) -> Id<ClassAncestryObservation> {
        self.mro.id()
    }
    pub(crate) fn contains(&self, class: Id<ProviderSymbol>) -> bool { self.symbol.id() == class || self.ancestors.contains(&class) }
    pub fn matches(&self, handler: &Self) -> Result<bool, ObligationKind> {
        if (self.symbol.provider, self.symbol.context)
            != (handler.symbol.provider, handler.symbol.context)
        {
            return Err(ObligationKind::IncompatibleContexts);
        }
        Ok(
            self.symbol.id() == handler.symbol.id()
                || self.ancestors.contains(&handler.symbol.id()),
        )
    }
    pub fn derive(
        data: &'a ModelApplicationData,
        symbol: Id<ProviderSymbol>,
        context: Id<AnalysisContext>,
        budget: &ResourceBudget,
    ) -> Result<Result<Self, ObligationKind>, ModelError> {
        Self::derive_evidence(ClassEvidence::application(data), symbol, context, budget)
    }
    pub(crate) fn derive_evidence(
        data: ClassEvidence<'a>, symbol: Id<ProviderSymbol>, context: Id<AnalysisContext>, budget: &ResourceBudget,
    ) -> Result<Result<Self, ObligationKind>, ModelError> {
        let mut charge = charged::StateCharge::new(budget, "model-exact-class");
        charge.grow(size_of::<Self>())?;
        let mut ancestors = charged::ChargedSet::default();
        let result = (|| {
            let b = &data;
            let symbol = need(&b.symbols, symbol)?;
            if symbol.kind != calls::SymbolKind::Class || symbol.context != context {
                return Err(ObligationKind::IncompatibleContexts);
            }
            let mro = one(b
                .ancestry
                .iter()
                .filter(|a| a.class == symbol.id() && a.relation == AncestryRelation::Mro))?;
            super::model_application::exact_qualification(b.qualifications, mro.qualification, context)?;
            if mro.linearization != Some(Linearization::Complete) {
                return Err(ObligationKind::IncompleteCoverage);
            }
            if !b.ancestry_supports.iter().any(|s| {
                s.assertion == mro.id() && b.runs.get(s.run).is_some_and(|r| r.context == context && b.input.is_none_or(|input| input == r.input))
            }) {
                return Err(ObligationKind::MissingEvidence);
            }
            let sequence = need(&b.sequences, mro.ancestors)?;
            let _ = sequence;
            for member in b
                .sequence_members
                .iter()
                .filter(|m| m.sequence == mro.ancestors)
            {
                let ancestor = need(&b.symbols, member.symbol)?;
                if ancestor.kind != calls::SymbolKind::Class
                    || ancestor.context != context
                    || ancestor.provider != symbol.provider
                {
                    return Err(ObligationKind::IncompatibleContexts);
                }
            }
            Ok((symbol, mro))
        })();
        match result {
            Err(r) => Ok(Err(r)),
            Ok((symbol, mro)) => {
                let count = data
                    .sequence_members
                    .iter()
                    .filter(|m| m.sequence == mro.ancestors)
                    .count();
                if count > symbols::MAX_SEQUENCE_SYMBOLS {
                    return Ok(Err(ObligationKind::IncompleteCoverage));
                }
                let _sequence = budget.reserve(
                    "model-exact-class-sequence",
                    count
                        .saturating_mul(
                            size_of::<symbols::SymbolSequenceMember>() * 3
                                + size_of::<Id<ProviderSymbol>>(),
                        )
                        .saturating_add(1024),
                )?;
                let mut members = data
                    .sequence_members
                    .iter()
                    .filter(|m| m.sequence == mro.ancestors)
                    .collect::<Vec<_>>();
                members.sort_by_key(|m| m.ordinal);
                if members
                    .iter()
                    .enumerate()
                    .any(|(ordinal, m)| m.ordinal != ordinal as i64)
                {
                    return Ok(Err(ObligationKind::MissingEvidence));
                }
                let ids = members.iter().map(|m| m.symbol).collect::<Vec<_>>();
                let (expected, _) = symbols::SymbolSequence::new(&ids)?;
                if data.sequences.get(mro.ancestors) != Some(&expected) {
                    return Ok(Err(ObligationKind::MissingEvidence));
                }
                for member in data
                    .sequence_members
                    .iter()
                    .filter(|m| m.sequence == mro.ancestors)
                {
                    ancestors.insert(&mut charge, member.symbol)?;
                }
                let mut premises = Rows::new(budget);
                let mut status = analysis::policy::EvidenceStatus::StructurallyObserved;
                let evidence = (|| -> Result<(), RuntimeEvidenceError> {
                    super::model_application::append_native_evidence_rows(
                        data.qualifications, data.native, data.premises, data.native_index,
                        derivation::RowRef::of(mro.id()),
                        mro.qualification,
                        context,
                        &mut premises,
                        &mut status,
                    )?;
                    for id in std::iter::once(symbol.id()).chain(ancestors.iter().copied()) {
                        let observation =
                            one(data.symbol_observations.iter().filter(|o| {
                                o.symbol == id
                                    && data
                                        .qualifications
                                        .get(o.qualification)
                                        .is_some_and(|q| q.context == context)
                            }))?;
                        super::model_application::append_native_evidence_rows(
                            data.qualifications, data.native, data.premises, data.native_index,
                            derivation::RowRef::of(observation.id()),
                            observation.qualification,
                            context,
                            &mut premises,
                            &mut status,
                        )?;
                    }
                    Ok(())
                })();
                match evidence {
                    Err(RuntimeEvidenceError::Boundary(reason)) => Ok(Err(reason)),
                    Err(RuntimeEvidenceError::Model(error)) => Err(error),
                    Ok(()) => {
                        if let Some(input) = data.input {
                            for premise in premises.iter() {
                                let run = match premise {
                                    analysis::native::NativeAssertionPremise::ClassAncestryObservation { support, .. } => data.ancestry_supports.get(*support).map(|support| support.run),
                                    analysis::native::NativeAssertionPremise::SymbolObservation { support, .. } => data.symbol_supports.and_then(|supports| supports.get(*support)).map(|support| support.run),
                                    _ => None,
                                };
                                if run.and_then(|run| data.runs.get(run)).is_none_or(|run| (run.input, run.context) != (input, context)) { return Ok(Err(ObligationKind::IncompatibleContexts)); }
                            }
                        }
                        Ok(Ok(Self {
                        symbol,
                        mro,
                        ancestors,
                        premises,
                        status,
                        _charge: charge,
                    }))
                    },
                }
            }
        }
    }
}
/// Protocol selection pins one native class and explicit allocation/initialization targets.
/// Entry/exit are authored source lifecycle actions. Optional local native declarations are
/// supporting inspection evidence, never runtime body identity or a claim of member absence.
pub struct CheckedContextProtocol<'a> {
    compiled: &'a CompiledContextProtocol,
    class: CheckedExactClass<'a>,
    allocation: Id<ProviderSymbol>,
    initialization: Id<ProviderSymbol>,
    entry: Option<Id<ProviderSymbol>>,
    exit: Option<Id<ProviderSymbol>>,
    premises: Rows<analysis::native::NativeAssertionPremise>,
    status: analysis::policy::EvidenceStatus,
    _charge: charged::StateCharge,
}
impl<'a> CheckedContextProtocol<'a> {
    pub fn premises(&self) -> &Rows<analysis::native::NativeAssertionPremise> {
        &self.premises
    }
    pub fn status(&self) -> analysis::policy::EvidenceStatus {
        self.status
    }
    pub fn compiled(&self) -> &CompiledContextProtocol {
        self.compiled
    }
    pub fn class(&self) -> &CheckedExactClass<'a> {
        &self.class
    }
    pub fn allocation(&self) -> Id<ProviderSymbol> {
        self.allocation
    }
    pub fn initialization(&self) -> Id<ProviderSymbol> {
        self.initialization
    }
    /// Optional local native declarations support inspection only; unavailable does not mean absent.
    pub fn entry_declaration(&self) -> Option<Id<ProviderSymbol>> {
        self.entry
    }
    pub fn exit_declaration(&self) -> Option<Id<ProviderSymbol>> {
        self.exit
    }
    pub fn derive(
        catalog: &'a Catalog,
        data: &'a ModelApplicationData,
        class: Id<ProviderSymbol>,
        input: Id<input::InputRevision>,
        context: Id<AnalysisContext>,
        budget: &ResourceBudget,
    ) -> Result<Result<Self, ObligationKind>, ModelError> {
        let mut charge = charged::StateCharge::new(budget, "model-context-protocol");
        charge.grow(size_of::<Self>())?;
        let symbol = match need(&data.bindings.symbols, class) {
            Ok(s) => s,
            Err(r) => return Ok(Err(r)),
        };
        let compiled = match one(catalog.context_protocols().iter().filter(|c| {
            super::model_application::matches_target(
                data,
                &c.model().target,
                symbol,
                input,
                context,
            )
            .is_ok()
        })) {
            Ok(c) => c,
            Err(r) => return Ok(Err(r)),
        };
        let class = match CheckedExactClass::derive(data, class, context, budget)? {
            Ok(c) => c,
            Err(r) => return Ok(Err(r)),
        };
        let target_bytes = match &compiled.model().target {
            models::Target::Stdlib {
                python,
                module,
                callable,
            } => python.len() + module.len() + callable.len(),
            models::Target::Dependency {
                distribution,
                version,
                module,
                callable,
            } => distribution.len() + version.len() + module.len() + callable.len(),
            models::Target::Release { module, callable } => module.len() + callable.len(),
        };
        charge.grow(target_bytes.saturating_mul(4).saturating_add(256))?;
        let find = |target: &models::Target| {
            one(data.bindings.symbols.iter().filter(|s| {
                s.context == context
                    && super::model_application::matches_target(data, target, s, input, context)
                        .is_ok()
            }))
            .map(Record::id)
        };
        let result = (|| {
            let allocation = find(&compiled.model().allocation)?;
            let initialization = find(&compiled.model().initialization)?;
            let member = |suffix: &str| {
                let mut target = compiled.model().target.clone();
                match &mut target {
                    models::Target::Stdlib { callable, .. }
                    | models::Target::Dependency { callable, .. }
                    | models::Target::Release { callable, .. } => {
                        callable.push('.');
                        callable.push_str(suffix)
                    }
                }
                find(&target)
            };
            let optional = |result: Result<_, ObligationKind>| match result {
                Ok(id) => Ok(Some(id)),
                Err(ObligationKind::MissingEvidence) => Ok(None),
                Err(reason) => Err(reason),
            };
            let entry = optional(member("__enter__"))?;
            let exit = optional(member("__exit__"))?;
            Ok(Self {
                compiled,
                class,
                allocation,
                initialization,
                entry,
                exit,
                premises: Rows::new(budget),
                status: analysis::policy::EvidenceStatus::StructurallyObserved,
                _charge: charge,
            })
        })();
        let mut protocol = match result {
            Ok(p) => p,
            Err(r) => return Ok(Err(r)),
        };
        protocol.status = protocol.class.status();
        for premise in protocol.class.premises().iter() {
            protocol.premises.insert(premise.clone())?;
        }
        let evidence = (|| -> Result<(), RuntimeEvidenceError> {
            let b = &data.bindings;
            for method in [protocol.allocation, protocol.initialization]
                .into_iter()
                .chain(protocol.entry)
                .chain(protocol.exit)
            {
                let traits = one(b.traits.iter().filter(|t| t.symbol == method))?;
                append_native_evidence(
                    data,
                    derivation::RowRef::of(traits.id()),
                    traits.qualification,
                    context,
                    &mut protocol.premises,
                    &mut protocol.status,
                )?;
                let mut current = need(&b.symbols, method)?;
                let mut remaining = b.symbols.len() + 1;
                loop {
                    if remaining == 0 {
                        return Err(ObligationKind::MissingEvidence.into());
                    }
                    remaining -= 1;
                    let observation = one(b.symbol_observations.iter().filter(|o| {
                        o.symbol == current.id()
                            && b.qualifications
                                .get(o.qualification)
                                .is_some_and(|q| q.context == context)
                    }))?;
                    append_native_evidence(
                        data,
                        derivation::RowRef::of(observation.id()),
                        observation.qualification,
                        context,
                        &mut protocol.premises,
                        &mut protocol.status,
                    )?;
                    match observation.parent {
                        Some(parent) => current = need(&b.symbols, parent)?,
                        None => break,
                    }
                }
            }
            Ok(())
        })();
        match evidence {
            Err(RuntimeEvidenceError::Boundary(r)) => Ok(Err(r)),
            Err(RuntimeEvidenceError::Model(e)) => Err(e),
            Ok(()) => Ok(Ok(protocol)),
        }
    }
    pub fn preserves(&self) -> bool {
        matches!(self.compiled.model().exit, ContextExit::Preserve)
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContextValue {
    None,
    Actual(Id<source::Occurrence>),
}
