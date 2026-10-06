//! Entry-value stability and substitution through a replayed whole normalized binding.
use super::{EvaluationAtom, entry::*};
use crate::domain::{
    assertion::AssertionQualification,
    attribution::ObligationKind,
    calls::{BindingProjection, BindingSource},
    normalized::{
        Rows,
        binding_normalization::{BindingData, BindingOutput, ValidatedBoundCall},
        bindings::*,
        callables::SignatureSlot,
        entities::*,
        events::NormalizedCallEvent,
    },
    value::*,
    *,
};
use crate::{Domain, DomainCode};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, DomainCode)]
#[repr(i16)]
pub enum StabilityBasis {
    ParameterOnlyReaching = 0,
}
impl StabilityBasis {
    /// Binding identity supports only the retained identity predicates, not mutable state.
    pub fn eligible(self, predicate: &Predicate) -> bool {
        match self {
            Self::ParameterOnlyReaching => {
                matches!(predicate, Predicate::IsNone | Predicate::IsValue { .. })
            }
        }
    }
}
pub fn substitutable(predicate: &Predicate) -> bool {
    StabilityBasis::ParameterOnlyReaching.eligible(predicate)
}
#[derive(Debug, Clone, PartialEq, Eq, Domain, serde::Serialize, serde::Deserialize)]
#[model(name="stability_witnesses",rule="parameter_only_reaching",conclusion=atom,invariant_refs=stability_invariants_refs)]
pub struct StabilityWitness {
    #[model(key)]
    pub atom: Id<EvaluationAtom>,
    #[model(key, premise)]
    pub entry: Id<EntryValueWitness>,
    #[model(key)]
    pub basis: StabilityBasis,
}
/// Only the shared entry operation can create this guard-specific proof.
/// ```compile_fail
/// use lctx_model::domain::conditions::stability::CheckedStability;
/// fn forge(mut proof:CheckedStability) {proof.parameter=todo!();}
/// ```
#[derive(Debug, Clone)]
pub struct CheckedStability {
    witness: StabilityWitness,
    parameter: Id<crate::domain::calls::SignatureParameter>,
    root: Id<PlaceRoot>,
    context: Id<crate::domain::attribution::AnalysisContext>,
    _entry_allowance: std::sync::Arc<charged::StateCharge>,
}
impl CheckedStability {
    pub fn witness(&self) -> &StabilityWitness {
        &self.witness
    }
    pub(crate) fn root(&self) -> Id<PlaceRoot> {
        self.root
    }
}
#[derive(Clone, Debug, PartialEq)]
struct StabilityReceipt {
    witness: StabilityWitness,
    parameter: Id<calls::SignatureParameter>,
    root: Id<PlaceRoot>,
    context: Id<attribution::AnalysisContext>,
}
impl HeapSize for StabilityReceipt {}
pub struct ProducedStability {
    values: charged::ChargedMap<Id<StabilityWitness>, StabilityReceipt>,
    charge: charged::StateCharge,
}
impl ProducedStability {
    pub(crate) fn new(budget: &resources::ResourceBudget) -> Self {
        Self {
            values: Default::default(),
            charge: charged::StateCharge::new(budget, "actual-local-stability-receipts"),
        }
    }
    pub(crate) fn capture(&mut self, value: &CheckedStability) -> Result<(), ModelError> {
        let receipt = StabilityReceipt {
            witness: value.witness.clone(),
            parameter: value.parameter,
            root: value.root,
            context: value.context,
        };
        if self
            .values
            .get(&receipt.witness.id())
            .is_some_and(|previous| previous != &receipt)
        {
            return Err(ModelError::Conflict("actual Local stability receipt"));
        }
        self.values
            .insert(&mut self.charge, receipt.witness.id(), receipt)?;
        Ok(())
    }
    pub fn get(
        &self,
        witness: &StabilityWitness,
        budget: &resources::ResourceBudget,
    ) -> Result<CheckedStability, ModelError> {
        if !self
            .charge
            .budget()
            .is_some_and(|owner| owner.shares_pool(budget))
        {
            return Err(ModelError::Invalid(
                "actual Local stability belongs to another attempt budget".into(),
            ));
        }
        let receipt = self
            .values
            .get(&witness.id())
            .ok_or_else(|| ModelError::Invalid("stability has no actual Local issuer".into()))?;
        if &receipt.witness != witness {
            return Err(ModelError::Conflict("actual Local stability descriptor"));
        }
        let mut charge = charged::StateCharge::new(budget, "actual-local-stability-hydration");
        charge.grow(size_of::<CheckedStability>())?;
        Ok(CheckedStability {
            witness: receipt.witness.clone(),
            parameter: receipt.parameter,
            root: receipt.root,
            context: receipt.context,
            _entry_allowance: std::sync::Arc::new(charge),
        })
    }
    pub(crate) fn append(&mut self, other: Self) -> Result<(), ModelError> {
        if !self
            .charge
            .budget()
            .zip(other.charge.budget())
            .is_some_and(|(a, b)| a.shares_pool(b))
        {
            return Err(ModelError::Invalid(
                "actual Local stability crosses attempt budgets".into(),
            ));
        }
        let Self {
            mut values,
            mut charge,
        } = other;
        while let Some(id) = values.keys().next().copied() {
            let value = values.remove(&mut charge, &id).expect("stability");
            if self
                .values
                .get(&id)
                .is_some_and(|previous| previous != &value)
            {
                return Err(ModelError::Conflict("actual Local stability receipt"));
            }
            self.values.insert(&mut self.charge, id, value)?;
        }
        Ok(())
    }
}
impl StabilityWitness {
    pub fn derive(
        data: &EntryData,
        atom: Id<EvaluationAtom>,
        entry: &DerivedEntryValue,
    ) -> Result<CheckedStability, ObligationKind> {
        let guard = data
            .atoms
            .get(atom)
            .ok_or(ObligationKind::MissingEvidence)?;
        let predicate = data
            .predicates
            .get(guard.predicate)
            .ok_or(ObligationKind::MissingEvidence)?;
        let basis = StabilityBasis::ParameterOnlyReaching;
        if !basis.eligible(predicate) {
            return Err(ObligationKind::ConditionTransferUnsupported);
        }
        match entry.source() {
            EntryAccessSource::Value { .. } => return Err(ObligationKind::EntryValueUnknown),
            EntryAccessSource::Guard { observation, .. } => {
                let leaf = data
                    .leaves
                    .get(*observation)
                    .ok_or(ObligationKind::MissingEvidence)?;
                if leaf.atom != atom {
                    return Err(ObligationKind::EntryValueUnknown);
                }
            }
            EntryAccessSource::Use { .. } => return Err(ObligationKind::EntryValueUnknown),
        }
        let ParameterEntity::Source { declaration } = data
            .formals
            .get(entry.witness().formal)
            .ok_or(ObligationKind::MissingEvidence)?
        else {
            return Err(ObligationKind::EntryValueUnknown);
        };
        let root = PlaceRoot::Formal {
            declaration: *declaration,
        };
        let place = Place {
            root: root.id(),
            path: AccessPath::empty().id(),
        };
        let read = data
            .occurrences
            .get(entry.witness().access)
            .ok_or(ObligationKind::MissingEvidence)?;
        let evaluation = data
            .occurrences
            .get(guard.evaluation)
            .ok_or(ObligationKind::MissingEvidence)?;
        if guard.context != entry.witness().context
            || guard.operand != Some(place.id())
            || read.source != evaluation.source
            || !read
                .structural_path
                .starts_with(&evaluation.structural_path)
            || read.structural_path.len() <= evaluation.structural_path.len()
            || read.start < evaluation.start
            || read.end > evaluation.end
        {
            return Err(ObligationKind::EntryValueUnknown);
        }
        Ok(CheckedStability {
            witness: Self {
                atom,
                entry: entry.witness().id(),
                basis,
            },
            parameter: entry.parameter(),
            root: root.id(),
            context: guard.context,
            _entry_allowance: entry.allowance(),
        })
    }
}
/// A retained exact member of an independently replayed complete binding shape.
/// ```compile_fail
/// use lctx_model::domain::conditions::stability::CheckedGuardBinding;
/// fn forge(mut proof:CheckedGuardBinding) {proof.binding=todo!();}
/// ```
#[derive(Debug, Clone)]
pub struct CheckedGuardBinding {
    binding: Id<CallBinding>,
    source: BindingSource,
    event: Id<NormalizedCallEvent>,
    site: Id<crate::domain::source::Occurrence>,
    context: Id<crate::domain::attribution::AnalysisContext>,
    parameter: Id<crate::domain::calls::SignatureParameter>,
}
impl CheckedGuardBinding {
    pub fn derive(
        checked: &ValidatedBoundCall,
        row: &CallBinding,
        slot: &SignatureSlot,
        source: &BindingSource,
        projection: &BindingProjection,
        attempt: &CallBindingAttempt,
        event: &NormalizedCallEvent,
    ) -> Result<Self, ObligationKind> {
        let raw = checked.bound();
        let member = usize::try_from(row.ordinal)
            .ok()
            .and_then(|i| raw.bindings().get(i))
            .ok_or(ObligationKind::MissingEvidence)?;
        if checked.attempt() != row.attempt
            || checked.event() != event.id()
            || checked.context() != event.context
            || attempt.id() != row.attempt
            || attempt.event != event.id()
            || attempt.signature != Some(raw.signature())
            || event.site != raw.site()
            || row.slot != slot.id()
            || attempt.variant != Some(slot.variant)
            || slot.parameter != member.formal
            || row.source != source.id()
            || row.kind != member.kind
            || row.projection != projection.id()
            || member.source != *source
            || member.projection != *projection
        {
            return Err(ObligationKind::MissingEvidence);
        }
        if *projection != BindingProjection::Whole {
            return Err(ObligationKind::ConditionTransferUnsupported);
        }
        if !matches!(
            source,
            BindingSource::Actual { .. } | BindingSource::ClassOf { .. }
        ) {
            return Err(if matches!(source, BindingSource::Default) {
                ObligationKind::DefaultStabilityUnknown
            } else {
                ObligationKind::ConditionTransferUnsupported
            });
        }
        Ok(Self {
            binding: row.id(),
            source: source.clone(),
            event: event.id(),
            site: event.site,
            context: event.context,
            parameter: member.formal,
        })
    }
    pub fn binding(&self) -> Id<CallBinding> {
        self.binding
    }
    pub fn event(&self) -> Id<NormalizedCallEvent> {
        self.event
    }
    pub(crate) fn root(&self) -> PlaceRoot {
        match self.source {
            BindingSource::Actual { occurrence } => PlaceRoot::Occurrence { occurrence },
            BindingSource::ClassOf { actual } => PlaceRoot::ClassOf { actual },
            _ => unreachable!("checked whole actual source"),
        }
    }
    pub(crate) fn agrees(
        &self,
        proof: &CheckedStability,
        site: Id<crate::domain::source::Occurrence>,
        context: Id<crate::domain::attribution::AnalysisContext>,
    ) -> bool {
        self.parameter == proof.parameter
            && self.site == site
            && self.context == context
            && proof.context == context
    }
    pub(crate) fn source(&self) -> Id<BindingSource> {
        self.source.id()
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Domain, serde::Serialize, serde::Deserialize)]
#[model(name="guard_substitutions",rule="witnessed_guard_substitution",conclusion=atom,invariant_refs=guard_substitution_invariants_refs)]
pub struct GuardSubstitution {
    #[model(key)]
    pub atom: Id<EvaluationAtom>,
    #[model(key, premise)]
    pub witness: Id<StabilityWitness>,
    #[model(key, premise)]
    pub binding: Id<CallBinding>,
    #[model(key)]
    pub source: Id<BindingSource>,
    #[model(key)]
    pub event: Id<NormalizedCallEvent>,
    #[model(key)]
    pub source_atom: Id<EvaluationAtom>,
    #[model(key)]
    pub actual_place: Id<Place>,
    #[model(key)]
    pub qualification: Id<AssertionQualification>,
}
pub fn stability_invariants() -> Vec<Invariant> {
    let mut inputs = EntryData::facts_inputs();
    inputs.extend([
        ValidationInput::of::<EntryValueWitness>(&["id"]),
        ValidationInput::of::<EntryAccessSource>(&["id"]),
        ValidationInput::of::<StabilityWitness>(&["id"]),
    ]);
    vec![Invariant {
        purpose: crate::domain::InvariantPurpose::Admission,
        revision: 2,
        name: "entry_guard_stability_replay",
        inputs,
        create: std::sync::Arc::new(|budget| {
            Box::new(EntryStabilityCheck {
                data: EntryData::new(budget),
                entries: Rows::new(budget),
                sources: Rows::new(budget),
                witnesses: Rows::new(budget),
                budget: budget.clone(),
            })
        }),
    }]
}
struct EntryStabilityCheck {
    data: EntryData,
    entries: Rows<EntryValueWitness>,
    sources: Rows<EntryAccessSource>,
    witnesses: Rows<StabilityWitness>,
    budget: resources::ResourceBudget,
}
fn check_stability(
    data: &EntryData,
    entries: &Rows<EntryValueWitness>,
    sources: &Rows<EntryAccessSource>,
    witnesses: &Rows<StabilityWitness>,
    budget: &resources::ResourceBudget,
) -> Result<(), ModelError> {
    for row in witnesses.iter() {
        let stored = entries
            .get(row.entry)
            .ok_or_else(|| invalid("stability entry absent"))?;
        let entry = EntryValueWitness::derive_for(
            data,
            stored.request(),
            sources
                .get(stored.access_source)
                .ok_or_else(|| invalid("entry source absent"))?,
            budget,
        )?
        .map_err(|_| invalid("stability entry refused"))?;
        if entry.witness() != stored {
            return Err(invalid("stored entry witness differs from replay"));
        }
        let proof = StabilityWitness::derive(data, row.atom, &entry)
            .map_err(|_| invalid("stability predicate or access refused"))?;
        if proof.witness() != row {
            return Err(invalid("stored stability witness differs from replay"));
        }
    }
    Ok(())
}
impl InvariantCheck for EntryStabilityCheck {
    fn visit(&mut self, name: &str, batch: &arrow_array::RecordBatch) -> Result<(), ModelError> {
        if self.data.visit(name, batch)? {
            Ok(())
        } else if name == EntryValueWitness::NAME {
            self.entries.decode(batch)
        } else if name == EntryAccessSource::NAME {
            self.sources.decode(batch)
        } else if name == StabilityWitness::NAME {
            self.witnesses.decode(batch)
        } else {
            Err(invalid("undeclared entry stability input"))
        }
    }
    fn finish(self: Box<Self>) -> Result<(), ModelError> {
        check_stability(
            &self.data,
            &self.entries,
            &self.sources,
            &self.witnesses,
            &self.budget,
        )
    }
}
pub fn guard_substitution_invariants() -> Vec<Invariant> {
    let mut inputs = EntryData::validation_inputs();
    inputs.extend(BindingData::validation_inputs());
    inputs.extend(BindingOutput::validation_inputs());
    inputs.extend([
        ValidationInput::of::<EntryValueWitness>(&["id"]),
        ValidationInput::of::<EntryAccessSource>(&["id"]),
        ValidationInput::of::<StabilityWitness>(&["id"]),
        ValidationInput::of::<GuardSubstitution>(&["id"]),
        ValidationInput::of::<crate::domain::transfer::summary::ControlInfluence>(&["id"]),
    ]);
    inputs.sort_by_key(|i| (i.name(), i.prefix().map(|p| p.code())));
    inputs.dedup_by_key(|i| (i.name(), i.prefix().map(|p| p.code())));
    vec![Invariant {
        purpose: crate::domain::InvariantPurpose::Admission,
        revision: 1,
        name: "guard_substitution_replay",
        inputs,
        create: std::sync::Arc::new(|budget| {
            Box::new(StabilityCheck {
                entry: EntryData::new(budget),
                bindings: BindingData::new(budget),
                bound: BindingOutput::new(budget),
                entries: Rows::new(budget),
                sources: Rows::new(budget),
                witnesses: Rows::new(budget),
                substitutions: Rows::new(budget),
                influences: Rows::new(budget),
                budget: budget.clone(),
            })
        }),
    }]
}
struct StabilityCheck {
    entry: EntryData,
    bindings: BindingData,
    bound: BindingOutput,
    entries: Rows<EntryValueWitness>,
    sources: Rows<EntryAccessSource>,
    witnesses: Rows<StabilityWitness>,
    substitutions: Rows<GuardSubstitution>,
    influences: Rows<crate::domain::transfer::summary::ControlInfluence>,
    budget: resources::ResourceBudget,
}
fn invalid(message: &str) -> ModelError {
    ModelError::Invalid(message.into())
}
impl InvariantCheck for StabilityCheck {
    fn visit(&mut self, name: &str, batch: &arrow_array::RecordBatch) -> Result<(), ModelError> {
        let mut found = self.entry.visit(name, batch)?;
        if !stages::is_vocabulary(name) {
            found |= self.bindings.visit(name, batch)?;
        }
        found |= self.bound.visit(name, batch)?;
        macro_rules! rows {
            ($field:ident,$ty:ty) => {
                if name == <$ty>::NAME {
                    self.$field.decode(batch)?;
                    found = true;
                }
            };
        }
        rows!(entries, EntryValueWitness);
        rows!(sources, EntryAccessSource);
        rows!(witnesses, StabilityWitness);
        rows!(substitutions, GuardSubstitution);
        rows!(
            influences,
            crate::domain::transfer::summary::ControlInfluence
        );
        if found {
            Ok(())
        } else {
            Err(invalid("undeclared stability input"))
        }
    }
    fn visit_input(
        &mut self,
        input: &ValidationInput,
        batch: &arrow_array::RecordBatch,
    ) -> Result<(), ModelError> {
        match input.prefix() {
            Some(stages::PublicationBoundary::Facts) => {
                if self.bindings.visit(input.name(), batch)? {
                    Ok(())
                } else {
                    Err(invalid("undeclared Facts binding input"))
                }
            }
            Some(_) => Err(invalid("guard binding replay requires Facts vocabulary")),
            None => self.visit(input.name(), batch),
        }
    }
    fn finish(self: Box<Self>) -> Result<(), ModelError> {
        check_stability(
            &self.entry,
            &self.entries,
            &self.sources,
            &self.witnesses,
            &self.budget,
        )?;
        let verified = if self.substitutions.is_empty() {
            None
        } else {
            Some(crate::domain::normalized::binding_normalization::verify(
                &self.bindings,
                &self.bound,
                &self.budget,
            )?)
        };
        for row in self.substitutions.iter() {
            let witness = self
                .witnesses
                .get(row.witness)
                .ok_or_else(|| invalid("substitution witness absent"))?;
            let stored = self
                .entries
                .get(witness.entry)
                .ok_or_else(|| invalid("stability entry absent"))?;
            let entry = EntryValueWitness::derive_for(
                &self.entry,
                stored.request(),
                self.sources
                    .get(stored.access_source)
                    .ok_or_else(|| invalid("entry source absent"))?,
                &self.budget,
            )?
            .map_err(|_| invalid("stability entry refused"))?;
            let stability = StabilityWitness::derive(&self.entry, witness.atom, &entry)
                .map_err(|_| invalid("stability refused"))?;
            let binding = self
                .bound
                .bindings
                .get(row.binding)
                .ok_or_else(|| invalid("substitution binding absent"))?;
            let attempt = self
                .bound
                .attempts
                .get(binding.attempt)
                .ok_or_else(|| invalid("substitution attempt absent"))?;
            let checked = verified
                .as_ref()
                .and_then(|v| v.bound(binding.attempt))
                .ok_or_else(|| invalid("substitution binding is not replayed"))?;
            let slot = self
                .bindings
                .callable_slots
                .get(binding.slot)
                .ok_or_else(|| invalid("binding slot absent"))?;
            let source = self
                .bound
                .sources
                .get(binding.source)
                .ok_or_else(|| invalid("binding source absent"))?;
            let projection = self
                .bound
                .projections
                .get(binding.projection)
                .ok_or_else(|| invalid("binding projection absent"))?;
            let event = self
                .bindings
                .event_events
                .get(attempt.event)
                .ok_or_else(|| invalid("binding event absent"))?;
            let binding = CheckedGuardBinding::derive(
                checked, binding, slot, source, projection, attempt, event,
            )
            .map_err(|_| invalid("guard binding refused"))?;
            let atom = self
                .entry
                .atoms
                .get(row.atom)
                .ok_or_else(|| invalid("substitution atom absent"))?;
            let predicate = self
                .entry
                .predicates
                .get(atom.predicate)
                .ok_or_else(|| invalid("substitution predicate absent"))?;
            let place = Place {
                root: binding.root().id(),
                path: AccessPath::empty().id(),
            };
            if !binding.agrees(&stability, atom.evaluation, atom.context)
                || row.event != binding.event()
                || row.source != binding.source()
                || row.source_atom != witness.atom
                || atom.operand != Some(place.id())
                || row.actual_place != place.id()
                || *predicate
                    != (Predicate::BoundGuard {
                        source: witness.atom,
                    })
                || self.entry.places.get(place.id()) != Some(&place)
                || self.entry.roots.get(place.root) != Some(&binding.root())
            {
                return Err(invalid("guard substitution differs from replay"));
            }
            if !self.influences.iter().any(|i| {
                i.atom == atom.id()
                    && i.input == place.id()
                    && i.evaluation == event.site
                    && i.qualification == row.qualification
                    && self
                        .entry
                        .qualifications
                        .get(i.qualification)
                        .is_some_and(|q| q.context == event.context)
            }) {
                return Err(invalid(
                    "guard substitution needs qualified actual-place influence",
                ));
            }
        }
        for atom in self.entry.atoms.iter() {
            if matches!(
                self.entry.predicates.get(atom.predicate),
                Some(Predicate::BoundGuard { .. })
            ) && !self.substitutions.iter().any(|s| s.atom == atom.id())
            {
                return Err(invalid("a bound guard needs a witnessed substitution"));
            }
        }
        Ok(())
    }
}

pub(crate) fn stability_invariants_refs() -> Vec<&'static str> {
    vec!["entry_guard_stability_replay"]
}
pub(crate) fn guard_substitution_invariants_refs() -> Vec<&'static str> {
    vec!["guard_substitution_replay"]
}
