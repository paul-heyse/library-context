//! Receipt-bound finite native inputs sharing the service's original canonical guard.
use super::{AdmittedSelection, Error, GenerationGuard, GenerationService, RequestExecution};
use lctx_model::domain::{
    analysis::{local, native::NativeQualification},
    attribution::{AnalysisContext, CoverageStatus},
    conditions::{entry::*, *},
    derivation::RowRef,
    execution::{
        summary_path::{PathEmission, SummaryPathRoute, SummaryPathWitness},
        summary_production::SummaryRun,
    },
    local_semantics::{LocalContribution, LocalData, LocalGuardContribution},
    native_requests::{self, CheckedAtom, NativeContext, NativeInventory, NativePath},
    normalized::{Rows, callables::*, entities::*},
    resources::{Reservation, ResourceBudget},
    serving::{self, InspectValuePathsRequest, InspectValuePathsResponse},
    transfer::{
        self, TransferBranch,
        summary::{SummaryPremise, SummaryWitness, WitnessBranch},
    },
    value::*,
    *,
};
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::Arc,
};

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
    Unexamined(DerivedEntryValue),
}
struct State {
    guard: GenerationGuard,
    selection: AdmittedSelection,
    contexts: BTreeMap<ContextKey, Context>,
    coverage: BTreeMap<Id<AnalysisContext>, CoverageStatus>,
    actual: BTreeSet<RowRef>,
    _charges: Vec<Box<dyn Reservation>>,
}
#[derive(Clone)]
pub struct PreparedNative {
    state: Arc<State>,
}

/// Retains packet allocations until the adapter has encoded the typed result.
struct NativeResponse {
    response: InspectValuePathsResponse,
    _charge: Box<dyn Reservation>,
}

macro_rules! rows {
    ($($field:ident:$ty:ty,)*) => {
        struct Extra { $(pub $field:Rows<$ty>,)* }
        impl Extra {
            fn new(b:&ResourceBudget)->Self {Self { $($field:Rows::new(b),)* }}
            fn inputs()->Vec<ValidationInput> {vec![$(ValidationInput::of::<$ty>(&["id"]),)*]}
            fn visit(&mut self,n:&str,b:&arrow_array::RecordBatch)->Result<(),ModelError> {$(if n==<$ty>::NAME{self.$field.decode(b)?;})*Ok(())}
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

fn need<R: Record>(rows: &Rows<R>, id: Id<R>) -> Result<&R, Error> {
    rows.get(id).ok_or(Error::Contract)
}
fn diagram(
    extra: &Extra,
    q: &assertion::AssertionQualification,
    b: &ResourceBudget,
) -> Result<Diagram, Error> {
    let _temporary = b.reserve(
        "native-condition-decode",
        extra.nodes.len().saturating_mul(2048).saturating_add(8192),
    )?;
    Ok(Diagram::from_records(
        need(&extra.conditions, q.condition)?,
        &extra.nodes.iter().cloned().collect::<Vec<_>>(),
    )?)
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
) -> Result<Option<DerivedEntryValue>, Error> {
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

impl GenerationService {
    /// Preparation is once per consumer and uses the guard's existing bounded preparation scope.
    pub async fn prepare_native(&self) -> Result<PreparedNative, Error> {
        let guard = self.guard();
        guard.check().await?;
        let selection = self.selection();
        let mut locked = guard.state.lease.lock().await;
        let lease = locked.as_mut().ok_or(Error::State)?;
        let budget = lease.budget.clone();
        // Run actual declared owner checks with their original vocabulary epochs. Receipts alone
        // do not grant native Entry, normalized callable or finite Summary proof authority.
        let mut seen = BTreeSet::new();
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
        for invariant in invariants {
            if !seen.insert(invariant.name) {
                continue;
            }
            let mut check = (invariant.create)(&budget);
            for input in &invariant.inputs {
                lease
                    .selection_read(input, |batch| {
                        check.visit_input(input, &batch)?;
                        Ok(())
                    })
                    .await?;
            }
            check.finish()?;
        }
        let mut native = NativeInventory::new(&budget);
        for input in NativeInventory::inputs() {
            lease
                .selection_read(&input, |batch| {
                    native.visit(input.name(), &batch)?;
                    Ok(())
                })
                .await?;
        }
        let mut extra = Extra::new(&budget);
        for input in Extra::inputs() {
            lease
                .selection_read(&input, |batch| {
                    extra.visit(input.name(), &batch)?;
                    Ok(())
                })
                .await?;
        }
        let mut count = extra.len();
        macro_rules! entry_count {($($field:ident:$ty:ty,)*)=>{$(count=count.saturating_add(native.entry.$field.len());)*};}
        macro_rules! theory_count {($($field:ident:$ty:ty,)*)=>{$(count=count.saturating_add(native.theory.$field.len());)*};}
        lctx_model::entry_value_inputs!(entry_count);
        lctx_model::local_theory_inputs!(theory_count);
        count = count.saturating_add(
            native.effective.len()
                + native.variants.len()
                + native.slots.len()
                + native.decorators.len(),
        );
        let charge = budget.reserve(
            "native-prepared-index",
            count.saturating_mul(256).saturating_add(size_of::<State>()),
        )?;
        let mut charges = vec![charge];
        let mut actual = BTreeSet::new();
        extra.actual(&mut actual);
        macro_rules! entry_actual {($($field:ident:$ty:ty,)*)=>{$(for row in native.entry.$field.iter(){actual.insert(RowRef::of(row.id()));})*};}
        macro_rules! theory_actual {($($field:ident:$ty:ty,)*)=>{$(for row in native.theory.$field.iter(){actual.insert(RowRef::of(row.id()));})*};}
        lctx_model::entry_value_inputs!(entry_actual);
        lctx_model::local_theory_inputs!(theory_actual);
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
            let entry = EntryValueWitness::derive_for(&native.entry, request, source, &budget)?
                .map_err(|_| Error::Contract)?;
            if entry.witness() != witness {
                return Err(Error::Contract);
            }
            let place = entry.place().id();
            let context = match context(&native, &entry) {
                Ok(c) => ContextState::Admitted(c),
                Err(_) => ContextState::Unexamined(entry),
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
            let Ok(entry) =
                EntryValueWitness::derive_for(&native.entry, request, &source, &budget)?
            else {
                continue;
            };
            let place = entry.place().id();
            let context = match context(&native, &entry) {
                Ok(c) => ContextState::Admitted(c),
                Err(_) => ContextState::Unexamined(entry),
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
            let Ok(entry) =
                EntryValueWitness::derive_for(&native.entry, request, &source, &budget)?
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
                return Err(Error::Contract);
            };
            let atom = CheckedAtom::derive(
                &entry,
                &local_theory::TheoryData {
                    entry: &native.entry,
                    inventory: &native.theory,
                },
                invocation,
                &budget,
            )?;
            let path = NativePath::guard(&entry, &native.entry, &budget)?;
            let place = entry.place().id();
            let context = match context(&native, &entry) {
                Ok(c) => ContextState::Admitted(c),
                Err(_) => ContextState::Unexamined(entry),
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
        let mut local_data = LocalData::new(&budget);
        local_data.entry = std::mem::replace(&mut native.entry, EntryData::new(&budget));
        for row in extra.native.iter() {
            local_data.native.insert(row.clone())?;
        }
        for row in extra.contributions.iter() {
            let invocation = need(&extra.invocations, row.invocation)?;
            let emission = LocalContribution::derive(
                &local_data,
                invocation,
                row.value,
                row.support,
                &budget,
            )?
            .map_err(|_| Error::Contract)?;
            if emission.contribution() != row {
                return Err(Error::Contract);
            }
            let entry = emission.entry();
            if let Some(slot) = contexts.get_mut(&(
                entry.witness().owner,
                entry.witness().formal,
                entry.witness().context,
            )) {
                slot.paths.push(NativePath::local(&emission, &budget)?);
            }
        }
        native.entry = local_data.entry;
        drop(local_data.native);
        // A stored BoundGuard becomes assignable only through the current opaque binding,
        // stability and Entry operations. A computed/default/projected actual has no bridge.
        if !extra.substitutions.is_empty() {
            use normalized::binding_normalization::{self, BindingData, BindingOutput};
            use rebase::{GuardCatalog, RootBinding, substitute_call_guards};
            use stability::{CheckedGuardBinding, StabilityWitness};
            let mut data = BindingData::new(&budget);
            let mut output = BindingOutput::new(&budget);
            for input in BindingData::validation_inputs() {
                lease
                    .selection_read(&input, |batch| {
                        data.visit(input.name(), &batch)?;
                        Ok(())
                    })
                    .await?;
            }
            for input in BindingOutput::validation_inputs() {
                lease
                    .selection_read(&input, |batch| {
                        output.visit(input.name(), &batch)?;
                        Ok(())
                    })
                    .await?;
            }
            let checked = binding_normalization::verify(&data, &output, &budget)?;
            let mut count = 0usize;
            macro_rules! count_binding {($($field:ident:$ty:ty,)*)=>{$(count=count.saturating_add(data.$field.len());)*};}
            macro_rules! count_output {($($field:ident:$ty:ty,)*)=>{$(count=count.saturating_add(output.$field.len());)*};}
            lctx_model::normalized_binding_inputs!(count_binding);
            lctx_model::normalized_binding_outputs!(count_output);
            charges.push(budget.reserve("native-binding-proof-index", count.saturating_mul(256))?);
            macro_rules! actual_binding {($($field:ident:$ty:ty,)*)=>{$(for row in data.$field.iter(){actual.insert(RowRef::of(row.id()));})*};}
            macro_rules! actual_output {($($field:ident:$ty:ty,)*)=>{$(for row in output.$field.iter(){actual.insert(RowRef::of(row.id()));})*};}
            lctx_model::normalized_binding_inputs!(actual_binding);
            lctx_model::normalized_binding_outputs!(actual_output);
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
                    &budget,
                )?
                .map_err(|_| Error::Contract)?;
                let stability = StabilityWitness::derive(&native.entry, witness.atom, &entry)
                    .map_err(|_| Error::Contract)?;
                if stability.witness() != witness {
                    return Err(Error::Contract);
                }
                let binding = need(&output.bindings, row.binding)?;
                let attempt = need(&output.attempts, binding.attempt)?;
                let event = need(&data.event_events, row.event)?;
                let source = need(&output.sources, row.source)?;
                let calls::BindingSource::Actual { occurrence: access } = source else {
                    continue;
                };
                let Some(bound) = checked.bound(binding.attempt) else {
                    return Err(Error::Contract);
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
                        &budget,
                    )?
                    .map_err(|_| Error::Contract)?
                } else {
                    let Some(entry) = read_entry(&native, *access, event.context, &budget)? else {
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
                    .ok_or(Error::Contract)?;
                let Ok(atom) = CheckedAtom::derive(
                    &entry,
                    &local_theory::TheoryData {
                        entry: &native.entry,
                        inventory: &native.theory,
                    },
                    invocation,
                    &budget,
                )?
                else {
                    continue;
                };
                let source_atom = need(&extra.atoms, row.source_atom)?;
                let source_place =
                    need(&extra.places, source_atom.operand.ok_or(Error::Contract)?)?;
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
                    &budget,
                )
                .map_err(|_| Error::Contract)?;
                if !rebased.substitutions.iter().any(|r| r == row) {
                    return Err(Error::Contract);
                }
                if let Ok(atom) =
                    atom.rebase(&rebased, &actual_entry, &binding, &stability, &budget)?
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
            let branch =
                TransferBranch::new(key, q.clone(), diagram(&extra, &q, &budget)?, &budget)?;
            if branch.alternative() != *alternative {
                return Err(Error::Contract);
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
            let branch =
                TransferBranch::new(key, q.clone(), diagram(&extra, &q, &budget)?, &budget)?;
            if branch.alternative() != *alternative {
                return Err(Error::Contract);
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
                diagram(&extra, &q, &budget)?,
                &budget,
            )?;
            if let Some((_, slot)) = contexts.iter_mut().find(|(context, r)| {
                context.0 == key.owner && context.2 == key.context && r.place == key.input
            }) {
                slot.paths.push(NativePath::summary(&branch, &budget)?);
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
                    diagram(&extra, &q, &budget)?,
                    root,
                    place,
                    &budget,
                )?;
                if let Some((_, slot)) = contexts.iter_mut().find(|(context, r)| {
                    context.0 == key.owner && context.2 == key.context && r.place == key.input
                }) {
                    slot.paths
                        .push(NativePath::continuation(&emission, &budget)?);
                }
                operands.insert(
                    SummaryPremise::Path { witness: row.id() }.id(),
                    Box::new(emission),
                );
            }
            if next.len() == before {
                return Err(Error::Contract);
            }
            remaining = next;
        }
        let coverage = extra
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
        for slot in contexts.values_mut() {
            slot.paths.sort_by_key(NativePath::identity);
            slot.paths.dedup_by_key(|p| p.identity());
        }
        drop(locked);
        guard.check().await?;
        Ok(PreparedNative {
            state: Arc::new(State {
                guard,
                selection,
                contexts,
                coverage,
                actual,
                _charges: charges,
            }),
        })
    }
}

impl PreparedNative {
    pub fn generation(&self) -> super::GenerationId {
        self.state.guard.generation()
    }
    pub async fn inspect(
        &self,
        execution: &RequestExecution,
        request: InspectValuePathsRequest,
    ) -> Result<InspectValuePathsResponse, Error> {
        if !execution.shares_guard(&self.state.guard) {
            return Err(Error::Contract);
        }
        let retained = self.clone();
        let grant = execution.clone();
        execution
            .cpu(move |budget| {
                let result = retained.inspect_pure(request, budget)?;
                let bytes = super::catalog_service::serialized_len(&result.response)?;
                grant.retain(
                    "native-owned-response",
                    bytes.saturating_mul(3).saturating_add(8192),
                )?;
                Ok(result.response)
            })
            .await
    }
    fn inspect_pure(
        &self,
        mut request: InspectValuePathsRequest,
        budget: &ResourceBudget,
    ) -> Result<NativeResponse, Error> {
        let input_bytes = request.inputs.iter().try_fold(
            size_of::<InspectValuePathsRequest>(),
            |sum, input| {
                let bytes = match &input.value {
                    native_requests::ExactScalar::String { value } => value.len(),
                    native_requests::ExactScalar::Integer { decimal } => decimal.len(),
                    native_requests::ExactScalar::None {}
                    | native_requests::ExactScalar::Bool { .. } => 0,
                };
                sum.checked_add(bytes.saturating_mul(4))
                    .and_then(|n| n.checked_add(256))
                    .ok_or(Error::ResourceRefused("native request inputs"))
            },
        )?;
        let _input_charge =
            budget.reserve("native-request-inputs", input_bytes.saturating_add(8192))?;
        let data = &self.state.selection.prepared().data().source;
        if data.catalog.members.get(request.member).is_none() {
            return Err(Error::Absent);
        }
        if request.inputs.is_empty()
            || request.inputs.len() > 32
            || request.page.size == 0
            || request.page.size > 100
        {
            return Err(Error::Contract);
        }
        let mut owners = BTreeSet::new();
        let mut allowed = BTreeSet::new();
        for callable in data
            .catalog
            .callables
            .iter()
            .filter(|r| r.member == request.member)
        {
            let assessment = need(&data.core.assessments, callable.assessment)?;
            if assessment.context != request.analysis {
                continue;
            }
            let owner = EntityRef::Callable {
                callable: assessment.callable,
            }
            .id();
            owners.insert(owner);
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
                        allowed.insert((owner, link.entity));
                    }
                }
            }
        }
        if owners.len() != 1 {
            return Err(Error::Contract);
        }
        let owner = *owners.iter().next().ok_or(Error::Contract)?;
        let mut inputs = BTreeSet::new();
        for input in &request.inputs {
            input.value.validate()?;
            if !inputs.insert(input.formal) || !allowed.contains(&(owner, input.formal)) {
                return Err(Error::Contract);
            }
        }
        request.inputs.sort_by_key(|r| r.formal);
        let binding = serving::CursorBinding {
            generation: serving::GenerationKey(*self.generation().bytes()),
            request: serving::Request::InspectValuePaths(request.clone())
                .canonical_identity()
                .map_err(|e| Error::Codec(e.to_string()))?,
            policy: serving::identity::PolicyIdentity(native_requests::definition()),
            wire: serving::wire_identity(),
            channels: serving::ChannelState {
                lexical: false,
                vector: serving::VectorChannel::Disabled {},
            }
            .identity(),
            group: serving::Name::new("native").map_err(|e| Error::Codec(e.to_string()))?,
            section: serving::Name::new("paths").map_err(|e| Error::Codec(e.to_string()))?,
            member: Some(request.member),
            ordering: ContentHash::of(b"formal,path-relation,path-id/v1"),
        };
        let offset = request
            .page
            .cursor
            .0
            .as_ref()
            .map(|token| serving::Cursor::decode(token, &binding))
            .transpose()
            .map_err(|e| Error::Codec(e.to_string()))?
            .map_or(0, |c| c.offset as usize);
        let charge = budget.reserve(
            "native-response",
            request.page.size as usize * 512 * 1024 + 8192,
        )?;
        let mut packets = Vec::new();
        let mut total = 0usize;
        let mut unavailable = false;
        for input in &request.inputs {
            let Some(slot) = self
                .state
                .contexts
                .get(&(owner, input.formal, request.analysis))
            else {
                unavailable = true;
                continue;
            };
            let coverage = *self
                .state
                .coverage
                .get(&request.analysis)
                .ok_or(Error::Contract)?;
            for path in &slot.paths {
                let index = total;
                total += 1;
                if index < offset || packets.len() >= request.page.size as usize {
                    continue;
                }
                let exact = native_requests::ExactRequest {
                    generation: binding.generation,
                    owner,
                    formal: input.formal,
                    value: &input.value,
                    assumptions: request.assumptions,
                };
                let assessment = match &slot.context {
                    ContextState::Admitted(context) => native_requests::assess(
                        &exact,
                        context,
                        path,
                        &slot.atoms,
                        coverage,
                        &[],
                        native_requests::Limits::default(),
                        budget,
                    )?,
                    ContextState::Unexamined(entry) => {
                        unavailable = true;
                        native_requests::unexamined(&exact, entry, path, budget)?
                    }
                };
                if !self.state.actual.contains(&assessment.path)
                    || !self
                        .state
                        .actual
                        .contains(&RowRef::of(assessment.original_condition))
                    || assessment
                        .proof
                        .iter()
                        .any(|row| !self.state.actual.contains(row))
                {
                    return Err(Error::Contract);
                }
                packets.push(serving::NativeAssessmentPacket::from_canonical(&assessment));
            }
        }
        if offset > total {
            return Err(Error::Contract);
        }
        let end = offset + packets.len();
        let truncated = end < total;
        let continuation = if truncated {
            serving::Optional::supplied(
                serving::Cursor {
                    binding,
                    offset: end as u64,
                }
                .encode()
                .map_err(|e| Error::Codec(e.to_string()))?,
            )
        } else {
            serving::Optional::default()
        };
        let not_requested =
            self.state.coverage.get(&request.analysis) == Some(&CoverageStatus::NotRequested);
        let availability = if not_requested {
            serving::Availability::NotRequested {}
        } else if unavailable {
            serving::Availability::Partial {
                reason: serving::Name::new("EntryValueUnknown")
                    .map_err(|e| Error::Codec(e.to_string()))?,
            }
        } else {
            serving::Availability::Available {}
        };
        Ok(NativeResponse {
            response: InspectValuePathsResponse {
                generation: serving::GenerationKey(*self.generation().bytes()),
                member: request.member,
                paths: serving::SectionPage {
                    availability,
                    items: packets,
                    continuation,
                    omitted: total.saturating_sub(end) as u64,
                    truncated,
                },
            },
            _charge: charge,
        })
    }
}
