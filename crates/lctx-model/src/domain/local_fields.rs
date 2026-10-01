//! Field locations retain receiver entry identity and source class candidates, never heap state.
use crate::domain::{
    analysis::local as publication,
    assertion::*,
    attribution::*,
    calls::*,
    conditions::{entry::*, *},
    flow::*,
    lexical::SyntaxField,
    local_theory::{TheoryData, TheoryInventory},
    normalized::{Rows, entities::*},
    source::*,
    syntax::*,
    types::*,
    value::*,
    *,
};
use crate::{Domain, DomainCode};
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, DomainCode)]
#[repr(i16)]
pub enum FieldLocationReason {
    MissingEvidence = 0,
    ReceiverEntryUnknown = 1,
    UnsupportedReceiver = 2,
    MissingReceiverClass = 3,
    NoDeclaredField = 4,
    AllocationStateUnavailable = 5,
    ConflictingClass = 6,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, DomainCode)]
#[repr(i16)]
pub enum FieldStateBoundary {
    AllocationAliasAndMutationUnavailable = 0,
}
#[macro_export]
macro_rules! local_field_inputs {
    ($apply:ident) => {
        $apply! {
         loads:$crate::domain::flow::FlowAttributeLoadObservation,
         load_supports:$crate::domain::flow::FlowAttributeLoadSupport,
         fields:$crate::domain::normalized::entities::FieldEntity,
         class_entities:$crate::domain::normalized::entities::ClassEntity,
         resolutions:$crate::domain::normalized::entities::SymbolEntityResolution,
         field_declarations:$crate::domain::normalized::entities::FieldDeclarationLink,
        }
    };
}
macro_rules! input {($($field:ident:$ty:ty,)*)=>{pub struct FieldInventory{$(pub $field:Rows<$ty>,)*}impl FieldInventory{pub fn new(budget:&resources::ResourceBudget)->Self{Self{$($field:Rows::new(budget),)*}}pub fn visit(&mut self,name:&str,batch:&arrow_array::RecordBatch)->Result<bool,ModelError>{$(if name==<$ty>::NAME{self.$field.decode(batch)?;return Ok(true);})*Ok(false)}pub fn validation_inputs()->Vec<ValidationInput>{vec![$(ValidationInput::of::<$ty>(&["id"]),)*]}}};}
crate::local_field_inputs!(input);
pub struct FieldData<'a> {
    pub entry: &'a EntryData,
    pub theory: &'a TheoryInventory,
    pub inventory: &'a FieldInventory,
}
impl FieldData<'_> {
    pub fn validation_inputs() -> Vec<ValidationInput> {
        let mut inputs = TheoryData::validation_inputs();
        inputs.extend(FieldInventory::validation_inputs());
        inputs
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(name="local_field_location_assessments",invariants=field_invariants)]
pub struct FieldLocationAssessment {
    #[model(key)]
    pub invocation: Id<publication::AnalysisInvocation>,
    #[model(key)]
    pub load: Id<FlowAttributeLoadObservation>,
    #[model(key)]
    pub support: Id<FlowAttributeLoadSupport>,
    pub location: Option<Id<FieldLocation>>,
    pub reason: FieldLocationReason,
}
/// One read-shaped location under its actual receiver read domain. State remains explicitly open.
#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(name = "local_field_locations", rule = "receiver_entry_field_location")]
pub struct FieldLocation {
    #[model(key)]
    pub invocation: Id<publication::AnalysisInvocation>,
    #[model(key, premise)]
    pub load: Id<FlowAttributeLoadObservation>,
    #[model(key, premise)]
    pub support: Id<FlowAttributeLoadSupport>,
    #[model(key, premise)]
    pub placement: Id<SyntaxPlacement>,
    #[model(key, premise)]
    pub placement_support: Id<SyntaxPlacementSupport>,
    #[model(key, premise)]
    pub entry: Id<EntryValueWitness>,
    #[model(key)]
    pub receiver: Id<Place>,
    #[model(key)]
    pub owner: Id<EntityRef>,
    #[model(key)]
    pub qualification: Id<AssertionQualification>,
    pub phase: CallPhase,
    pub state: FieldStateBoundary,
}
/// The declared field is a source typing candidate; it is not an allocated object-state identity.
#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(
    name = "local_field_location_candidates",
    rule = "source_class_field_candidate"
)]
pub struct FieldLocationCandidate {
    #[model(key)]
    pub location: Id<FieldLocation>,
    #[model(key)]
    pub field: Id<FieldEntity>,
    #[model(key, premise)]
    pub receiver_type: Id<TypeObservation>,
    #[model(key, premise)]
    pub type_support: Id<TypeSupport>,
    #[model(key, premise)]
    pub class_resolution: Id<SymbolEntityResolution>,
    #[model(key, premise)]
    pub declaration: Id<FieldDeclarationLink>,
}
fn invalid(s: &str) -> ModelError {
    ModelError::Invalid(s.into())
}
fn need<R: Record>(rows: &Rows<R>, id: Id<R>) -> Result<&R, FieldLocationReason> {
    rows.get(id).ok_or(FieldLocationReason::MissingEvidence)
}
fn same_node(data: &FieldData, a: Id<Occurrence>, b: Id<Occurrence>) -> bool {
    match (data.entry.occurrences.get(a), data.entry.occurrences.get(b)) {
        (Some(a), Some(b)) => {
            a.source == b.source
                && a.structural_path == b.structural_path
                && a.syntax_kind == b.syntax_kind
                && a.start == b.start
                && a.end == b.end
        }
        _ => false,
    }
}
/// Only shared replay can recreate this admission; callers receive no mutable state theorem.
pub struct DerivedFieldLocation {
    reason: FieldLocationReason,
    location: FieldLocation,
    candidates: Rows<FieldLocationCandidate>,
    entry: DerivedEntryValue,
    qualification: AssertionQualification,
    condition: Diagram,
    _condition: Box<dyn resources::Reservation>,
}
impl DerivedFieldLocation {
    pub fn reason(&self) -> FieldLocationReason {
        self.reason
    }
    pub fn location(&self) -> &FieldLocation {
        &self.location
    }
    pub fn candidates(&self) -> impl Iterator<Item = &FieldLocationCandidate> {
        self.candidates.iter()
    }
    pub fn entry(&self) -> &DerivedEntryValue {
        &self.entry
    }
    pub fn qualification(&self) -> &AssertionQualification {
        &self.qualification
    }
    pub fn condition(&self) -> &Diagram {
        &self.condition
    }
}
impl FieldLocation {
    pub fn derive(
        data: &FieldData,
        invocation: &publication::AnalysisInvocation,
        load: Id<FlowAttributeLoadObservation>,
        support_: Id<FlowAttributeLoadSupport>,
        budget: &resources::ResourceBudget,
    ) -> Result<Result<DerivedFieldLocation, FieldLocationReason>, ModelError> {
        if invocation.definition != crate::domain::local_semantics::definition().1.id() {
            return Err(invalid(
                "field location invocation differs from Local definition",
            ));
        }
        let setup = (|| {
            let load = need(&data.inventory.loads, load)?;
            let native = need(&data.inventory.load_supports, support_)?;
            if native.assertion != load.id()
                || native.origin != Origin::AnalyzerAssertion
                || native.mode != ExtractionMode::NativeTraversal
            {
                return Err(FieldLocationReason::MissingEvidence);
            }
            let q = need(&data.entry.qualifications, load.qualification)?;
            let theory = TheoryData {
                entry: data.entry,
                inventory: data.theory,
            };
            crate::domain::local_theory::support(&theory, native, q, invocation, FactFamily::Flow)
                .map_err(|_| FieldLocationReason::MissingEvidence)?;
            let attribute = need(&data.entry.occurrences, load.occurrence)?;
            if attribute.syntax_kind != SyntaxKind::ExprAttribute {
                return Err(FieldLocationReason::UnsupportedReceiver);
            }
            let mut placements = data.entry.placements.iter().filter(|p| {
                p.parent
                    .is_some_and(|parent| same_node(data, parent, load.occurrence))
                    && p.field == SyntaxField::Value
                    && data
                        .entry
                        .qualifications
                        .get(p.qualification)
                        .is_some_and(|q| q.context == invocation.context)
            });
            let placement = placements
                .next()
                .ok_or(FieldLocationReason::MissingEvidence)?;
            if placements.next().is_some() {
                return Err(FieldLocationReason::MissingEvidence);
            }
            let mut supports = data
                .entry
                .placement_supports
                .iter()
                .filter(|s| s.assertion == placement.id());
            let ps = supports
                .next()
                .ok_or(FieldLocationReason::MissingEvidence)?;
            if supports.next().is_some()
                || ps.origin != Origin::SourceObservation
                || ps.mode != ExtractionMode::NativeTraversal
            {
                return Err(FieldLocationReason::MissingEvidence);
            }
            let pq = need(&data.entry.qualifications, placement.qualification)?;
            if pq.condition != Diagram::always().id() {
                return Err(FieldLocationReason::MissingEvidence);
            }
            crate::domain::local_theory::support(&theory, ps, pq, invocation, FactFamily::Syntax)
                .map_err(|_| FieldLocationReason::MissingEvidence)?;
            let mut uses = data.entry.uses.iter().filter(|u| {
                same_node(data, u.occurrence, placement.occurrence)
                    && data
                        .entry
                        .occurrences
                        .get(u.occurrence)
                        .is_some_and(|o| o.role == OccurrenceRole::Read)
            });
            let use_ = uses
                .next()
                .ok_or(FieldLocationReason::ReceiverEntryUnknown)?;
            if uses.next().is_some() {
                return Err(FieldLocationReason::ReceiverEntryUnknown);
            }
            let place = need(&data.entry.places, use_.place)?;
            let PlaceRoot::Formal { declaration } = need(&data.entry.roots, place.root)? else {
                return Err(FieldLocationReason::UnsupportedReceiver);
            };
            let mut owners = data
                .entry
                .owners
                .iter()
                .filter(|o| o.occurrence == use_.occurrence);
            let owner = owners.next().ok_or(FieldLocationReason::MissingEvidence)?;
            if owners.next().is_some() {
                return Err(FieldLocationReason::MissingEvidence);
            }
            let run = need(&data.entry.runs, native.run)?;
            let formal = ParameterEntity::Source {
                declaration: *declaration,
            };
            Ok((
                load,
                native,
                q,
                placement,
                ps,
                EntryRequest {
                    owner: owner.entity,
                    formal: formal.id(),
                    access: use_.occurrence,
                    context: invocation.context,
                    run: run.id(),
                },
            ))
        })();
        let (load, native, q, placement, ps, request) = match setup {
            Ok(value) => value,
            Err(reason) => return Ok(Err(reason)),
        };
        let entry = match EntryValueWitness::derive(data.entry, request, budget)? {
            Ok(entry) => entry,
            Err(_) => return Ok(Err(FieldLocationReason::ReceiverEntryUnknown)),
        };
        if !matches!(entry.source(), EntryAccessSource::Use { .. }) {
            return Err(invalid("field location requires exact Use entry operation"));
        }
        let allowance = data
            .entry
            .condition_nodes
            .len()
            .checked_mul(2048)
            .ok_or_else(|| invalid("field condition allocation overflow"))?;
        let _decode = budget.reserve("local_field_condition_decode", allowance)?;
        let nodes = data
            .entry
            .condition_nodes
            .iter()
            .cloned()
            .collect::<Vec<_>>();
        let condition = Diagram::from_records(
            data.entry
                .conditions
                .get(q.condition)
                .ok_or_else(|| invalid("field load condition absent"))?,
            &nodes,
        )?;
        let combined = condition
            .admitted_binary(entry.condition(), BooleanOperation::Conjunction, budget)
            .map_err(|e| match e {
                DiagramAdmissionError::Resource(e) => e,
                DiagramAdmissionError::Boundary(_) => {
                    invalid("field location condition work refused")
                }
            })?;
        let (condition, reservation) = combined.into_parts();
        let qualification = AssertionQualification {
            condition: condition.id(),
            ..q.clone()
        };
        let location = Self {
            invocation: invocation.id(),
            load: load.id(),
            support: native.id(),
            placement: placement.id(),
            placement_support: ps.id(),
            entry: entry.witness().id(),
            receiver: entry.place().id(),
            owner: request.owner,
            qualification: qualification.id(),
            phase: CallPhase::Call,
            state: FieldStateBoundary::AllocationAliasAndMutationUnavailable,
        };
        let mut candidates = Rows::new(budget);
        let mut class_seen = false;
        let theory = TheoryData {
            entry: data.entry,
            inventory: data.theory,
        };
        for observation in data.theory.type_observations.iter().filter(|o| {
            same_node(data, o.subject, placement.occurrence) && o.role == TypeRole::TestOperand
        }) {
            let Some(TypeTerm::ClassInstance { class, .. } | TypeTerm::SelfType { class, .. }) =
                data.theory.terms.get(observation.term)
            else {
                continue;
            };
            let Some(oq) = data.entry.qualifications.get(observation.qualification) else {
                continue;
            };
            if oq.context != invocation.context || oq.condition != Diagram::always().id() {
                continue;
            }
            let mut supports = data
                .theory
                .type_supports
                .iter()
                .filter(|s| s.assertion == observation.id());
            let Some(ts) = supports.next() else { continue };
            if supports.next().is_some()
                || ts.origin != Origin::AnalyzerAssertion
                || ts.mode != ExtractionMode::NativeTraversal
                || crate::domain::local_theory::support(
                    &theory,
                    ts,
                    oq,
                    invocation,
                    FactFamily::Types,
                )
                .is_err()
            {
                continue;
            }
            let Some(symbol) = data.entry.symbols.get(*class) else {
                continue;
            };
            if symbol.context != invocation.context
                || symbol.kind != SymbolKind::Class
                || !data
                    .entry
                    .runs
                    .get(ts.run)
                    .is_some_and(|run| run.provider == symbol.provider)
            {
                continue;
            }
            for resolution in data.inventory.resolutions.iter().filter(|r| {
                r.symbol == *class
                    && r.context == invocation.context
                    && r.status == ResolutionStatus::Resolved
            }) {
                let Some(EntityRef::Class {
                    class: class_entity,
                }) = resolution.entity.and_then(|id| data.entry.refs.get(id))
                else {
                    continue;
                };
                if data.inventory.class_entities.get(*class_entity).is_none() {
                    continue;
                }
                class_seen = true;
                for field in data
                    .inventory
                    .fields
                    .iter()
                    .filter(|f| f.class == *class_entity && f.name.as_str() == load.name)
                {
                    for declaration in data
                        .inventory
                        .field_declarations
                        .iter()
                        .filter(|d| d.field == field.id())
                    {
                        candidates.insert(FieldLocationCandidate {
                            location: location.id(),
                            field: field.id(),
                            receiver_type: observation.id(),
                            type_support: ts.id(),
                            class_resolution: resolution.id(),
                            declaration: declaration.id(),
                        })?;
                    }
                }
            }
        }
        let first_class = candidates
            .iter()
            .next()
            .and_then(|c| data.inventory.fields.get(c.field))
            .map(|f| f.class);
        let conflicting = candidates.iter().any(|c| {
            data.inventory
                .fields
                .get(c.field)
                .is_some_and(|f| Some(f.class) != first_class)
        });
        let reason = if conflicting {
            FieldLocationReason::ConflictingClass
        } else if !candidates.is_empty() {
            FieldLocationReason::AllocationStateUnavailable
        } else if class_seen {
            FieldLocationReason::NoDeclaredField
        } else {
            FieldLocationReason::MissingReceiverClass
        };
        Ok(Ok(DerivedFieldLocation {
            reason,
            location,
            candidates,
            entry,
            qualification,
            condition,
            _condition: reservation,
        }))
    }
}
#[macro_export]
macro_rules! local_field_outputs {
    ($apply:ident) => {
        $apply! {
         assessments:$crate::domain::local_fields::FieldLocationAssessment,
         locations:$crate::domain::local_fields::FieldLocation,
         candidates:$crate::domain::local_fields::FieldLocationCandidate,
        }
    };
}
macro_rules! output{($($field:ident:$ty:ty,)*)=>{pub struct FieldRecords{$(pub $field:Rows<$ty>,)*}impl FieldRecords{pub fn new(budget:&resources::ResourceBudget)->Self{Self{$($field:Rows::new(budget),)*}}pub fn visit(&mut self,name:&str,batch:&arrow_array::RecordBatch)->Result<bool,ModelError>{$(if name==<$ty>::NAME{self.$field.decode(batch)?;return Ok(true);})*Ok(false)}}};}
crate::local_field_outputs!(output);
pub fn relations() -> Vec<Relation> {
    vec![
        Relation::of::<FieldLocationAssessment>(),
        Relation::of::<FieldLocation>(),
        Relation::of::<FieldLocationCandidate>(),
    ]
}
fn field_invariants() -> Vec<Invariant> {
    let mut inputs = FieldData::validation_inputs();
    inputs.extend([
        ValidationInput::of::<publication::AnalysisInvocation>(&["id"]),
        ValidationInput::of::<EntryValueWitness>(&["id"]),
        ValidationInput::of::<EntryAccessSource>(&["id"]),
    ]);
    macro_rules! output{($($field:ident:$ty:ty,)*)=>{$(inputs.push(ValidationInput::of::<$ty>(&["id"]));)*};}
    crate::local_field_outputs!(output);
    vec![Invariant {
        name: "local_field_location_replay",
        inputs,
        create: std::sync::Arc::new(|budget| {
            Box::new(FieldCheck {
                entry: EntryData::new(budget),
                theory: TheoryInventory::new(budget),
                inventory: FieldInventory::new(budget),
                records: FieldRecords::new(budget),
                invocations: Rows::new(budget),
                entries: Rows::new(budget),
                sources: Rows::new(budget),
                budget: budget.clone(),
            })
        }),
    }]
}
struct FieldCheck {
    entry: EntryData,
    theory: TheoryInventory,
    inventory: FieldInventory,
    records: FieldRecords,
    invocations: Rows<publication::AnalysisInvocation>,
    entries: Rows<EntryValueWitness>,
    sources: Rows<EntryAccessSource>,
    budget: resources::ResourceBudget,
}
impl InvariantCheck for FieldCheck {
    fn visit_input(
        &mut self,
        input: &ValidationInput,
        batch: &arrow_array::RecordBatch,
    ) -> Result<(), ModelError> {
        if stages::is_vocabulary(input.name())
            && input.prefix() != Some(stages::PublicationBoundary::Facts)
        {
            return Err(invalid("field replay requires Facts vocabulary"));
        }
        self.visit(input.name(), batch)
    }
    fn visit(&mut self, name: &str, batch: &arrow_array::RecordBatch) -> Result<(), ModelError> {
        if self.entry.visit(name, batch)?
            || self.theory.visit(name, batch)?
            || self.inventory.visit(name, batch)?
            || self.records.visit(name, batch)?
        {
            return Ok(());
        }
        if name == publication::AnalysisInvocation::NAME {
            self.invocations.decode(batch)?;
        } else if name == EntryValueWitness::NAME {
            self.entries.decode(batch)?;
        } else if name == EntryAccessSource::NAME {
            self.sources.decode(batch)?;
        } else {
            return Err(invalid("undeclared field replay input"));
        }
        Ok(())
    }
    fn finish(self: Box<Self>) -> Result<(), ModelError> {
        let data = FieldData {
            entry: &self.entry,
            theory: &self.theory,
            inventory: &self.inventory,
        };
        let mut expected = FieldRecords::new(&self.budget);
        for row in self.records.assessments.iter() {
            let invocation = self
                .invocations
                .get(row.invocation)
                .ok_or_else(|| invalid("field invocation absent"))?;
            match FieldLocation::derive(&data, invocation, row.load, row.support, &self.budget)? {
                Ok(proof) => {
                    if row.location != Some(proof.location.id())
                        || row.reason != proof.reason
                        || self.entries.get(proof.entry.witness().id())
                            != Some(proof.entry.witness())
                        || self.sources.get(proof.entry.source().id()) != Some(proof.entry.source())
                    {
                        return Err(invalid(
                            "field location assessment or receiver entry differs from replay",
                        ));
                    }
                    expected.locations.insert(proof.location)?;
                    for row in proof.candidates.iter() {
                        expected.candidates.insert(row.clone())?;
                    }
                }
                Err(reason) => {
                    if row.location.is_some() || row.reason != reason {
                        return Err(invalid("field location refusal differs from replay"));
                    }
                }
            }
        }
        if !self.records.locations.same(&expected.locations)
            || !self.records.candidates.same(&expected.candidates)
        {
            return Err(invalid(
                "stored field location or candidate differs from replay",
            ));
        }
        Ok(())
    }
}
