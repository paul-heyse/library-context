//! Pure native composition over explicitly hydrated rows. No lease or SQL authority is held.
use crate::domain::{
    analysis::{local, native::NativeQualification},
    attribution::{AnalysisContext, CoverageStatus},
    conditions::{entry::*, *},
    derivation::RowRef,
    execution::{
        summary_path::{PathEmission, SummaryPathRoute, SummaryPathWitness},
        summary_production::SummaryRun,
    },
    local_semantics::{LocalContribution, LocalData, LocalGuardContribution},
    native_requests::{CheckedAtom, NativeContext, NativeInventory, NativePath},
    normalized::{Rows, callables::*, entities::*},
    resources::{Reservation, ResourceBudget},
    serving::InspectValuePathsRequest,
    transfer::{
        self, TransferBranch,
        summary::{SummaryPremise, SummaryWitness, WitnessBranch},
    },
    value::*,
    *,
};
use std::collections::{BTreeMap, BTreeSet};

type ContextKey = (Id<EntityRef>, Id<ParameterEntity>, Id<AnalysisContext>);
struct Context {
    context: ContextState,
    place: Id<Place>,
    atoms: Vec<CheckedAtom>,
    paths: Vec<NativePath>,
}
#[allow(
    clippy::large_enum_variant,
    reason = "Inline refusal frames are included in charged native context preparation"
)]
enum ContextState {
    Admitted(NativeContext),
    Unexamined(DerivedEntryValue, obligation::ObligationKind),
}
macro_rules! rows {
    ($($field:ident:$ty:ty,)*) => {
        pub struct PreparationRows { $(pub $field:Rows<$ty>,)* }
        impl PreparationRows {
            pub fn new(b:&ResourceBudget)->Self {Self { $($field:Rows::new(b),)* }}
            pub fn inputs()->Vec<ValidationInput> {vec![$(ValidationInput::of::<$ty>(&["id"]),)*]}
            pub fn visit(&mut self,n:&str,b:&arrow_array::RecordBatch)->Result<(),ModelError> {$(if n==<$ty>::NAME{self.$field.decode(b)?;})*Ok(())}
            fn actual(&self,out:&mut BTreeSet<RowRef>) {$(for row in self.$field.iter(){out.insert(RowRef::of(row.id()));})*}
            fn len(&self)->usize {0 $(+self.$field.len())*}
        }
    };
}
rows! {
    invocations:local::AnalysisInvocation,
    coverage:local::AnalysisCoverage,
    native:NativeQualification,
    entries:EntryValueWitness,
    sources:EntryAccessSource,
    contributions:LocalContribution,
    local_keys:transfer::local::TransferKey,
    local_alternatives:transfer::local::TransferAlternative,
    model_keys:transfer::model::TransferKey,
    model_alternatives:transfer::model::TransferAlternative,
    summary_keys:transfer::summary::TransferKey,
    witnesses:SummaryWitness,
    premises:SummaryPremise,
    routes:SummaryPathRoute,
    path_witnesses:SummaryPathWitness,
    conditions:Condition,
    nodes:ConditionNode,
    qualifications:assertion::AssertionQualification,
    roots:PlaceRoot,
    places:Place,
    paths:AccessPath,
    predicates:Predicate,
    atoms:EvaluationAtom,
    stability:stability::StabilityWitness,
    substitutions:stability::GuardSubstitution,
}

fn need<R: Record>(rows: &Rows<R>, id: Id<R>) -> Result<&R, ModelError> {
    rows.get(id).ok_or(invalid())
}
fn diagram(
    extra: &PreparationRows,
    q: &assertion::AssertionQualification,
    b: &ResourceBudget,
) -> Result<Diagram, ModelError> {
    let _temporary = b.reserve(
        "native-condition-decode",
        extra.nodes.len().saturating_mul(2048).saturating_add(8192),
    )?;
    Diagram::from_records(
        need(&extra.conditions, q.condition)?,
        &extra.nodes.iter().cloned().collect::<Vec<_>>(),
    )
}
fn context(
    native: &NativeInventory,
    entry: &DerivedEntryValue,
) -> Result<NativeContext, obligation::ObligationKind> {
    native.context(entry)
}
fn read_entry(
    native: &NativeInventory,
    access: Id<source::Occurrence>,
    analysis: Id<AnalysisContext>,
    budget: &ResourceBudget,
) -> Result<Option<DerivedEntryValue>, ModelError> {
    for support in native.entry.use_supports.iter() {
        let observation = need(&native.entry.use_observations, support.assertion)?;
        let use_ = need(&native.entry.uses, observation.use_)?;
        if use_.occurrence != access {
            continue;
        }
        let q = need(&native.entry.qualifications, observation.qualification)?;
        if q.context != analysis {
            continue;
        }
        let place = need(&native.entry.places, use_.place)?;
        let Some(PlaceRoot::Formal { declaration }) = native.entry.roots.get(place.root) else {
            continue;
        };
        let Some(owner) = native.entry.owners.iter().find(|r| r.occurrence == access) else {
            continue;
        };
        let request = EntryRequest {
            owner: owner.entity,
            formal: ParameterEntity::Source {
                declaration: *declaration,
            }
            .id(),
            access,
            context: analysis,
            run: support.run,
        };
        let source = EntryAccessSource::Use {
            observation: observation.id(),
            support: support.id(),
        };
        if let Ok(entry) = EntryValueWitness::derive_for(&native.entry, request, &source, budget)? {
            return Ok(Some(entry));
        }
    }
    Ok(None)
}

fn invalid() -> ModelError {
    ModelError::Invalid("native preparation contract".into())
}

/// Declared preparation inventory; decoding confers no publication admission.
pub struct PreparationInputs {
    pub native: NativeInventory,
    pub rows: PreparationRows,
    pub binding: Option<(
        normalized::binding_normalization::BindingData,
        normalized::binding_normalization::BindingOutput,
    )>,
}
impl PreparationInputs {
    pub fn new(budget: &ResourceBudget) -> Self {
        Self {
            native: NativeInventory::new(budget),
            rows: PreparationRows::new(budget),
            binding: None,
        }
    }
    pub fn needs_binding(&self) -> bool {
        !self.rows.substitutions.is_empty()
    }
    pub fn actual_row_bound(&self) -> usize {
        let mut total = self.rows.len()
            + self.native.effective.len()
            + self.native.variants.len()
            + self.native.slots.len()
            + self.native.decorators.len();
        macro_rules! entry_count {($($field:ident:$ty:ty,)*)=>{$(total = total.saturating_add(self.native.entry.$field.len());)*};}
        macro_rules! theory_count {($($field:ident:$ty:ty,)*)=>{$(total = total.saturating_add(self.native.theory.$field.len());)*};}
        crate::entry_value_inputs!(entry_count);
        crate::local_theory_inputs!(theory_count);
        if let Some((data, output)) = &self.binding {
            macro_rules! input_count {($($field:ident:$ty:ty,)*)=>{$(total = total.saturating_add(data.$field.len());)*};}
            macro_rules! output_count {($($field:ident:$ty:ty,)*)=>{$(total = total.saturating_add(output.$field.len());)*};}
            crate::normalized_binding_inputs!(input_count);
            crate::normalized_binding_outputs!(output_count);
        }
        total
    }
    pub fn actual_rows(&self) -> BTreeSet<RowRef> {
        let native = &self.native;
        let extra = &self.rows;
        let mut actual = BTreeSet::new();
        extra.actual(&mut actual);
        macro_rules! entry_actual {($($field:ident:$ty:ty,)*)=>{$(for row in native.entry.$field.iter(){actual.insert(RowRef::of(row.id()));})*};}
        macro_rules! theory_actual {($($field:ident:$ty:ty,)*)=>{$(for row in native.theory.$field.iter(){actual.insert(RowRef::of(row.id()));})*};}
        crate::entry_value_inputs!(entry_actual);
        crate::local_theory_inputs!(theory_actual);
        for row in native.effective.iter() {
            actual.insert(RowRef::of(row.id()));
        }
        for row in native.variants.iter() {
            actual.insert(RowRef::of(row.id()));
        }
        for row in native.slots.iter() {
            actual.insert(RowRef::of(row.id()));
        }
        for row in native.decorators.iter() {
            actual.insert(RowRef::of(row.id()));
        }

        if let Some((data, output)) = &self.binding {
            macro_rules! actual_binding {($($field:ident:$ty:ty,)*)=>{$(for row in data.$field.iter(){actual.insert(RowRef::of(row.id()));})*};}
            macro_rules! actual_output {($($field:ident:$ty:ty,)*)=>{$(for row in output.$field.iter(){actual.insert(RowRef::of(row.id()));})*};}
            crate::normalized_binding_inputs!(actual_binding);
            crate::normalized_binding_outputs!(actual_output);
        }
        actual
    }
}

/// Shared owner validators are executed by the repository against their exact declared epochs.
pub fn preparation_invariants() -> Vec<Invariant> {
    let mut invariants = EntryValueWitness::invariants();
    invariants.extend(EvaluationAtom::invariants());
    invariants.extend(EffectiveCallableAssessment::invariants());
    invariants.extend(SignatureVariant::invariants());
    invariants.extend(SignatureSlot::invariants());
    invariants.extend(LocalGuardContribution::invariants());
    invariants.extend(SummaryWitness::invariants());
    invariants.extend(SummaryRun::invariants());
    invariants.extend(stability::StabilityWitness::invariants());
    invariants.extend(stability::GuardSubstitution::invariants());
    invariants
}

#[derive(Default)]
struct FormalScope {
    owners: BTreeSet<Id<EntityRef>>,
    formals: BTreeSet<(Id<EntityRef>, Id<ParameterEntity>)>,
}
/// Public formals are prepared independently of native scalar-context admission.
pub struct FormalDomain {
    members: BTreeSet<Id<catalog::CatalogMember>>,
    domains: BTreeMap<(Id<catalog::CatalogMember>, Id<AnalysisContext>), FormalScope>,
    _charge: super::super::charged::StateCharge,
}
impl FormalDomain {
    pub fn prepare(
        data: &selection::classification::ClassificationData,
        budget: &ResourceBudget,
    ) -> Result<Self, ModelError> {
        let data = &data.source;
        let mut charge =
            super::super::charged::StateCharge::new(budget, "native-public-formal-domain");
        charge.grow(8192)?;
        let mut members = BTreeSet::new();
        for member in data.catalog.members.iter() {
            if !members.contains(&member.id()) {
                charge.grow(128)?;
                members.insert(member.id());
            }
        }
        let mut domains = BTreeMap::<_, FormalScope>::new();
        for callable in data.catalog.callables.iter() {
            let assessment = need(&data.core.assessments, callable.assessment)?;
            let owner = EntityRef::Callable {
                callable: assessment.callable,
            }
            .id();
            if !domains.contains_key(&(callable.member, assessment.context)) {
                charge.grow(256)?;
            }
            let domain = domains
                .entry((callable.member, assessment.context))
                .or_default();
            if !domain.owners.contains(&owner) {
                charge.grow(128)?;
                domain.owners.insert(owner);
            }
            for invocation in data
                .catalog
                .invocations
                .iter()
                .filter(|r| r.callable == callable.id())
            {
                for slot in data
                    .core
                    .slots
                    .iter()
                    .filter(|r| r.variant == invocation.variant)
                {
                    for link in data
                        .core
                        .parameter_links
                        .iter()
                        .filter(|r| r.parameter == slot.parameter)
                    {
                        if !domain.formals.contains(&(owner, link.entity)) {
                            charge.grow(192)?;
                            domain.formals.insert((owner, link.entity));
                        }
                    }
                }
            }
        }
        Ok(Self {
            members,
            domains,
            _charge: charge,
        })
    }
    pub fn contains_member(&self, member: Id<catalog::CatalogMember>) -> bool {
        self.members.contains(&member)
    }
    pub fn resolve(
        &self,
        request: &mut InspectValuePathsRequest,
    ) -> Result<Id<EntityRef>, ModelError> {
        if request.inputs.is_empty()
            || request.inputs.len() > 32
            || request.page.size == 0
            || request.page.size > 100
        {
            return Err(invalid());
        }
        let domain = self
            .domains
            .get(&(request.member, request.analysis))
            .ok_or_else(invalid)?;
        if domain.owners.len() != 1 {
            return Err(invalid());
        }
        let owner = *domain.owners.iter().next().ok_or_else(invalid)?;
        let mut inputs = BTreeSet::new();
        for input in &request.inputs {
            input.value.validate()?;
            if !inputs.insert(input.formal) || !domain.formals.contains(&(owner, input.formal)) {
                return Err(invalid());
            }
        }
        request.inputs.sort_by_key(|r| r.formal);
        Ok(owner)
    }
}

/// Opaque checked semantic output. It establishes local replay, never generation admission.
pub struct PreparedNativeSemantics {
    contexts: BTreeMap<ContextKey, Context>,
    coverage: BTreeMap<Id<AnalysisContext>, CoverageStatus>,
    domain: FormalDomain,
    _charges: Vec<Box<dyn Reservation>>,
}
impl PreparedNativeSemantics {
    pub fn prepare(
        inputs: PreparationInputs,
        selection: &selection::classification::ClassificationData,
        budget: &ResourceBudget,
    ) -> Result<Self, ModelError> {
        let PreparationInputs {
            mut native,
            rows: extra,
            binding,
        } = inputs;
        let domain = FormalDomain::prepare(selection, budget)?;
        let mut count = extra.len();
        macro_rules! entry_count {($($field:ident:$ty:ty,)*)=>{$(count=count.saturating_add(native.entry.$field.len());)*};}
        macro_rules! theory_count {($($field:ident:$ty:ty,)*)=>{$(count=count.saturating_add(native.theory.$field.len());)*};}
        crate::entry_value_inputs!(entry_count);
        crate::local_theory_inputs!(theory_count);
        count = count.saturating_add(
            native.effective.len()
                + native.variants.len()
                + native.slots.len()
                + native.decorators.len(),
        );
        let charge = budget.reserve(
            "native-prepared-index",
            count
                .saturating_mul(256)
                .saturating_add(size_of::<PreparedNativeSemantics>()),
        )?;
        let mut charges = vec![charge];
        let mut contexts = BTreeMap::<ContextKey, Context>::new();
        for witness in extra.entries.iter() {
            let request = EntryRequest {
                owner: witness.owner,
                formal: witness.formal,
                access: witness.access,
                context: witness.context,
                run: witness.run,
            };
            let source = need(&extra.sources, witness.access_source)?;
            let entry = EntryValueWitness::derive_for(&native.entry, request, source, budget)?
                .map_err(|_| invalid())?;
            if entry.witness() != witness {
                return Err(invalid());
            }
            let place = entry.place().id();
            let context = match context(&native, &entry) {
                Ok(c) => ContextState::Admitted(c),
                Err(cause) => ContextState::Unexamined(entry, cause),
            };
            contexts
                .entry((witness.owner, witness.formal, witness.context))
                .or_insert(Context {
                    context,
                    place,
                    atoms: vec![],
                    paths: vec![],
                });
        }
        // Caller continuations can exist without a published Local Entry witness. Replay the
        // original native read premises; the resulting private witness is never cited as stored.
        for support in native.entry.use_supports.iter() {
            let observation = need(&native.entry.use_observations, support.assertion)?;
            let use_ = need(&native.entry.uses, observation.use_)?;
            let place = need(&native.entry.places, use_.place)?;
            let Some(PlaceRoot::Formal { declaration }) = native.entry.roots.get(place.root) else {
                continue;
            };
            let Some(owner) = native
                .entry
                .owners
                .iter()
                .find(|r| r.occurrence == use_.occurrence)
            else {
                continue;
            };
            let q = need(&native.entry.qualifications, observation.qualification)?;
            let request = EntryRequest {
                owner: owner.entity,
                formal: ParameterEntity::Source {
                    declaration: *declaration,
                }
                .id(),
                access: use_.occurrence,
                context: q.context,
                run: support.run,
            };
            let key = (request.owner, request.formal, request.context);
            if contexts.contains_key(&key) {
                continue;
            }
            let source = EntryAccessSource::Use {
                observation: observation.id(),
                support: support.id(),
            };
            let Ok(entry) = EntryValueWitness::derive_for(&native.entry, request, &source, budget)?
            else {
                continue;
            };
            let place = entry.place().id();
            let context = match context(&native, &entry) {
                Ok(c) => ContextState::Admitted(c),
                Err(cause) => ContextState::Unexamined(entry, cause),
            };
            contexts.insert(
                key,
                Context {
                    context,
                    place,
                    atoms: vec![],
                    paths: vec![],
                },
            );
        }
        // Replay each native guard access; unsupported or missing scalar bridges remain without
        // an assignment. No request value is supplied during preparation or written as a fact.
        for support in native.entry.leaf_supports.iter() {
            let leaf = need(&native.entry.leaves, support.assertion)?;
            let Some(operand) = leaf.operand.and_then(|id| native.entry.occurrences.get(id)) else {
                continue;
            };
            let mut uses = native.entry.uses.iter().filter(|r| {
                native
                    .entry
                    .occurrences
                    .get(r.occurrence)
                    .is_some_and(|read| {
                        read.source == operand.source
                            && read.structural_path == operand.structural_path
                            && read.syntax_kind == operand.syntax_kind
                            && read.start == operand.start
                            && read.end == operand.end
                            && read.role == source::OccurrenceRole::Read
                    })
            });
            let Some(use_) = uses.next() else {
                continue;
            };
            if uses.next().is_some() {
                continue;
            }
            let access = use_.occurrence;
            let Some(owner) = native.entry.owners.iter().find(|r| r.occurrence == access) else {
                continue;
            };
            let Some(place) = native.entry.places.get(use_.place) else {
                continue;
            };
            let Some(PlaceRoot::Formal { declaration }) = native.entry.roots.get(place.root) else {
                continue;
            };
            let q = need(&native.entry.qualifications, leaf.qualification)?;
            let request = EntryRequest {
                owner: owner.entity,
                formal: ParameterEntity::Source {
                    declaration: *declaration,
                }
                .id(),
                access,
                context: q.context,
                run: support.run,
            };
            let Ok(source) =
                EntryAccessSource::guard(&native.entry, request, leaf.id(), support.id())
            else {
                continue;
            };
            let Ok(entry) = EntryValueWitness::derive_for(&native.entry, request, &source, budget)?
            else {
                continue;
            };
            let mut invocation = extra.invocations.iter().filter(|r| {
                r.input
                    == need(&native.entry.runs, request.run)
                        .expect("replayed Entry run")
                        .input
                    && r.context == request.context
                    && r.subject.is_none()
            });
            let Some(invocation) = invocation.next() else {
                return Err(invalid());
            };
            let atom = CheckedAtom::derive(
                &entry,
                &local_theory::TheoryData {
                    entry: &native.entry,
                    inventory: &native.theory,
                },
                invocation,
                budget,
            )?;
            let path = NativePath::guard(&entry, &native.entry, budget)?;
            let place = entry.place().id();
            let context = match context(&native, &entry) {
                Ok(c) => ContextState::Admitted(c),
                Err(cause) => ContextState::Unexamined(entry, cause),
            };
            let slot = contexts
                .entry((request.owner, request.formal, request.context))
                .or_insert(Context {
                    context,
                    place,
                    atoms: vec![],
                    paths: vec![],
                });
            if let Ok(atom) = atom {
                slot.atoms.push(atom);
            }
            slot.paths.push(path);
        }
        let mut local_data = LocalData::new(budget);
        local_data.entry = std::mem::replace(&mut native.entry, EntryData::new(budget));
        for row in extra.native.iter() {
            local_data.native.insert(row.clone())?;
        }
        for row in extra.contributions.iter() {
            let invocation = need(&extra.invocations, row.invocation)?;
            let emission =
                LocalContribution::derive(&local_data, invocation, row.value, row.support, budget)?
                    .map_err(|_| invalid())?;
            if emission.contribution() != row {
                return Err(invalid());
            }
            let entry = emission.entry();
            if let Some(slot) = contexts.get_mut(&(
                entry.witness().owner,
                entry.witness().formal,
                entry.witness().context,
            )) {
                slot.paths.push(NativePath::local(&emission, budget)?);
            }
        }
        native.entry = local_data.entry;
        drop(local_data.native);
        // A stored BoundGuard becomes assignable only through the current opaque binding,
        // stability and Entry operations. A computed/default/projected actual has no bridge.
        if !extra.substitutions.is_empty() {
            use normalized::binding_normalization;
            use rebase::{GuardCatalog, RootBinding, substitute_call_guards};
            use stability::{CheckedGuardBinding, StabilityWitness};
            let (data, output) = binding.ok_or_else(invalid)?;
            let checked = binding_normalization::verify(&data, &output, budget)?;
            let mut count = 0usize;
            macro_rules! count_binding {($($field:ident:$ty:ty,)*)=>{$(count=count.saturating_add(data.$field.len());)*};}
            macro_rules! count_output {($($field:ident:$ty:ty,)*)=>{$(count=count.saturating_add(output.$field.len());)*};}
            lctx_model::normalized_binding_inputs!(count_binding);
            lctx_model::normalized_binding_outputs!(count_output);
            charges.push(budget.reserve("native-binding-proof-index", count.saturating_mul(256))?);
            let _catalog_charge = budget.reserve(
                "native-rebase-vocabulary",
                (extra.atoms.len()
                    + extra.predicates.len()
                    + extra.places.len()
                    + extra.roots.len())
                .saturating_mul(512),
            )?;
            let atoms = extra.atoms.iter().map(|r| (r.id(), r.clone())).collect();
            let predicates = extra
                .predicates
                .iter()
                .map(|r| (r.id(), r.clone()))
                .collect();
            let places = extra.places.iter().map(|r| (r.id(), r.clone())).collect();
            let roots = extra.roots.iter().map(|r| (r.id(), r.clone())).collect();
            let catalog = GuardCatalog {
                atoms: &atoms,
                predicates: &predicates,
                places: &places,
                roots: &roots,
            };
            for row in extra.substitutions.iter() {
                let witness = need(&extra.stability, row.witness)?;
                let stored = need(&extra.entries, witness.entry)?;
                let entry = EntryValueWitness::derive_for(
                    &native.entry,
                    stored.request(),
                    need(&extra.sources, stored.access_source)?,
                    budget,
                )?
                .map_err(|_| invalid())?;
                let stability = StabilityWitness::derive(&native.entry, witness.atom, &entry)
                    .map_err(|_| invalid())?;
                if stability.witness() != witness {
                    return Err(invalid());
                }
                let binding = need(&output.bindings, row.binding)?;
                let attempt = need(&output.attempts, binding.attempt)?;
                let event = need(&data.event_events, row.event)?;
                let source = need(&output.sources, row.source)?;
                let calls::BindingSource::Actual { occurrence: access } = source else {
                    continue;
                };
                let Some(bound) = checked.bound(binding.attempt) else {
                    return Err(invalid());
                };
                let Ok(binding) = CheckedGuardBinding::derive(
                    bound,
                    binding,
                    need(&data.callable_slots, binding.slot)?,
                    source,
                    need(&output.projections, binding.projection)?,
                    attempt,
                    event,
                ) else {
                    continue;
                };
                let actual_entry = if let Some(stored) = extra
                    .entries
                    .iter()
                    .find(|r| r.access == *access && r.context == event.context)
                {
                    EntryValueWitness::derive_for(
                        &native.entry,
                        stored.request(),
                        need(&extra.sources, stored.access_source)?,
                        budget,
                    )?
                    .map_err(|_| invalid())?
                } else {
                    let Some(entry) = read_entry(&native, *access, event.context, budget)? else {
                        continue;
                    };
                    entry
                };
                let Some(slot) = contexts.get_mut(&(
                    actual_entry.witness().owner,
                    actual_entry.witness().formal,
                    event.context,
                )) else {
                    continue;
                };
                let run = need(&native.entry.runs, stored.run)?;
                let invocation = extra
                    .invocations
                    .iter()
                    .find(|r| {
                        r.input == run.input && r.context == run.context && r.subject.is_none()
                    })
                    .ok_or(invalid())?;
                let Ok(atom) = CheckedAtom::derive(
                    &entry,
                    &local_theory::TheoryData {
                        entry: &native.entry,
                        inventory: &native.theory,
                    },
                    invocation,
                    budget,
                )?
                else {
                    continue;
                };
                let source_atom = need(&extra.atoms, row.source_atom)?;
                let source_place = need(&extra.places, source_atom.operand.ok_or(invalid())?)?;
                let bindings =
                    BTreeMap::from([(source_place.root, RootBinding::Actual(binding.clone()))]);
                let witnesses = BTreeMap::from([(row.source_atom, stability.clone())]);
                let rebased = substitute_call_guards(
                    &Diagram::from_atom(row.source_atom),
                    need(&native.entry.occurrences, event.site)?,
                    event.context,
                    &catalog,
                    &bindings,
                    Some(&witnesses),
                    Some(need(&extra.qualifications, row.qualification)?),
                    budget,
                )
                .map_err(|_| invalid())?;
                if !rebased.substitutions.iter().any(|r| r == row) {
                    return Err(invalid());
                }
                if let Ok(atom) =
                    atom.rebase(&rebased, &actual_entry, &binding, &stability, budget)?
                {
                    slot.atoms.push(atom);
                }
            }
        }
        let mut operands: BTreeMap<
            Id<SummaryPremise>,
            Box<dyn composition::CompositionOperand + Send + Sync>,
        > = BTreeMap::new();
        for alternative in extra.local_alternatives.iter() {
            let key = need(&extra.local_keys, alternative.transfer)?.clone();
            let q = need(&extra.qualifications, alternative.qualification)?.clone();
            let branch = TransferBranch::new(key, q.clone(), diagram(&extra, &q, budget)?, budget)?;
            if branch.alternative() != *alternative {
                return Err(invalid());
            }
            operands.insert(
                SummaryPremise::Local {
                    alternative: alternative.id(),
                }
                .id(),
                Box::new(branch),
            );
        }
        for alternative in extra.model_alternatives.iter() {
            let key = need(&extra.model_keys, alternative.transfer)?.clone();
            let q = need(&extra.qualifications, alternative.qualification)?.clone();
            let branch = TransferBranch::new(key, q.clone(), diagram(&extra, &q, budget)?, budget)?;
            if branch.alternative() != *alternative {
                return Err(invalid());
            }
            operands.insert(
                SummaryPremise::Model {
                    alternative: alternative.id(),
                }
                .id(),
                Box::new(branch),
            );
        }
        for row in extra.witnesses.iter() {
            let key = need(&extra.summary_keys, row.transfer)?.clone();
            let q = need(&extra.qualifications, row.qualification)?.clone();
            let branch = WitnessBranch::new(
                row,
                key.clone(),
                q.clone(),
                diagram(&extra, &q, budget)?,
                budget,
            )?;
            if let Some((_, slot)) = contexts.iter_mut().find(|(context, r)| {
                context.0 == key.owner && context.2 == key.context && r.place == key.input
            }) {
                slot.paths.push(NativePath::summary(&branch, budget)?);
            }
            operands.insert(
                SummaryPremise::Witness { witness: row.id() }.id(),
                Box::new(branch),
            );
        }
        // Original canonical continuation DAG is finite. Assemble only after Summary replay;
        // lack of a predecessor is corruption, never implicit feasibility or an ID-only proof.
        let mut remaining = extra.path_witnesses.iter().collect::<Vec<_>>();
        while !remaining.is_empty() {
            let before = remaining.len();
            let mut next = Vec::new();
            for row in remaining {
                let Some(predecessor) = operands.get(&row.source) else {
                    next.push(row);
                    continue;
                };
                let key = need(&extra.summary_keys, row.transfer)?.clone();
                let q = need(&extra.qualifications, row.qualification)?.clone();
                let place = need(&extra.places, key.output)?.clone();
                let root = need(&extra.roots, place.root)?.clone();
                let emission = PathEmission::from_records(
                    row,
                    need(&extra.routes, row.route)?,
                    need(&extra.premises, row.source)?,
                    predecessor.as_ref(),
                    key.clone(),
                    q.clone(),
                    diagram(&extra, &q, budget)?,
                    root,
                    place,
                    budget,
                )?;
                if let Some((_, slot)) = contexts.iter_mut().find(|(context, r)| {
                    context.0 == key.owner && context.2 == key.context && r.place == key.input
                }) {
                    slot.paths
                        .push(NativePath::continuation(&emission, budget)?);
                }
                operands.insert(
                    SummaryPremise::Path { witness: row.id() }.id(),
                    Box::new(emission),
                );
            }
            if next.len() == before {
                return Err(invalid());
            }
            remaining = next;
        }
        let coverage: BTreeMap<_, _> = extra
            .coverage
            .iter()
            .map(|r| {
                use normalized::coverage::EvidenceAvailability as A;
                let value = match r.availability {
                    A::Complete => CoverageStatus::CompleteUnderStatedModel,
                    A::Partial => CoverageStatus::Partial,
                    A::NotRequested => CoverageStatus::NotRequested,
                    A::Unavailable | A::NoScope => CoverageStatus::Unavailable,
                };
                (r.context, value)
            })
            .collect();
        for ((_, _, analysis), slot) in &mut contexts {
            if !coverage.contains_key(analysis) {
                return Err(invalid());
            }
            slot.paths.sort_by_key(NativePath::identity);
            slot.paths.dedup_by_key(|p| p.identity());
        }

        Ok(Self {
            contexts,
            coverage,
            domain,
            _charges: charges,
        })
    }
    pub fn domain(&self) -> &FormalDomain {
        &self.domain
    }
    pub fn coverage(&self, analysis: Id<AnalysisContext>) -> Option<CoverageStatus> {
        self.coverage.get(&analysis).copied()
    }
    pub fn path_count(
        &self,
        owner: Id<EntityRef>,
        formal: Id<ParameterEntity>,
        analysis: Id<AnalysisContext>,
    ) -> Option<usize> {
        self.contexts
            .get(&(owner, formal, analysis))
            .map(|c| c.paths.len())
    }
    pub fn unexamined(
        &self,
        owner: Id<EntityRef>,
        formal: Id<ParameterEntity>,
        analysis: Id<AnalysisContext>,
    ) -> bool {
        self.contexts
            .get(&(owner, formal, analysis))
            .is_none_or(|c| matches!(c.context, ContextState::Unexamined(..)))
    }
    pub fn assess_path(
        &self,
        exact: &super::ExactRequest<'_>,
        analysis: Id<AnalysisContext>,
        index: usize,
        budget: &ResourceBudget,
    ) -> Result<super::Assessment, ModelError> {
        let slot = self
            .contexts
            .get(&(exact.owner, exact.formal, analysis))
            .ok_or_else(invalid)?;
        let path = slot.paths.get(index).ok_or_else(invalid)?;
        match &slot.context {
            ContextState::Admitted(context) => super::assess(
                exact,
                context,
                path,
                &slot.atoms,
                self.coverage(analysis).ok_or_else(invalid)?,
                &[],
                super::Limits::default(),
                budget,
            ),
            ContextState::Unexamined(entry, cause) => {
                super::unexamined(exact, entry, *cause, path, budget)
            }
        }
    }
}
