//! Real attributed facts and the sole normalized admission replay for transfer controls.
#![allow(dead_code, reason = "Shared behavioral/store conformance fixture")]
#[path = "../typed_driver/mod.rs"]
mod typed_driver;
use lctx_model::domain::{
    calls::*,
    normalized::{
        Rows, binding_normalization::*, bindings::*, callable_normalization, entity_normalization,
        event_normalization, relation_normalization,
    },
    resources::ResourceBudget,
    *,
};
use typed_driver::{files, rows};
pub struct Facts(pub typed_driver::Tables);
impl typed_driver::Inspector for Facts {
    fn demand(&self)->typed_driver::ObservationDemand {
        // Reconstruction and negative admission controls consume the complete inventory.
        typed_driver::ObservationDemand::Complete
    }
    fn tables(&self) -> typed_driver::Tables {
        self.0.clone()
    }
}
pub struct NativeFixture {
    pub fixture: &'static str,
    pub tables: typed_driver::Tables,
    pub data: BindingData,
    pub output: BindingOutput,
    pub verified: VerifiedBindings,
    pub budget: ResourceBudget,
}
impl NativeFixture {
    pub fn source_bytes(&self, source: Id<source::SourceArtifact>) -> Vec<u8> {
        let artifact = self.data.artifacts.get(source).unwrap();
        files(self.fixture)[&artifact.path].clone()
    }
    pub fn rows<R: Record>(&self) -> Vec<R> {
        rows::<R>(&self.tables)
    }
    pub fn attempts_at(&self, text: &str) -> Vec<&CallBindingAttempt> {
        self.output
            .attempts
            .iter()
            .filter(|attempt| {
                let event = self.data.event_events.get(attempt.event).unwrap();
                let o = self.data.occurrences.get(event.site).unwrap();
                let source = self.data.artifacts.get(o.source).unwrap();
                files(self.fixture)[&source.path.to_string()][o.start as usize..o.end as usize]
                    == *text.as_bytes()
            })
            .collect()
    }
    pub fn attempt(&self, text: &str) -> Id<CallBindingAttempt> {
        self.output
            .attempts
            .iter()
            .find(|attempt| {
                let event = self.data.event_events.get(attempt.event).unwrap();
                let occurrence = self.data.occurrences.get(event.site).unwrap();
                let artifact = self.data.artifacts.get(occurrence.source).unwrap();
                let content = files(self.fixture);
                content.get(&artifact.path).is_some_and(|bytes| {
                    bytes[occurrence.start as usize..occurrence.end as usize] == *text.as_bytes()
                }) && self.verified.composition(attempt.id()).is_some()
            })
            .unwrap_or_else(|| {
                for a in self.output.attempts.iter() {
                    let e = self.data.event_events.get(a.event).unwrap();
                    let o = self.data.occurrences.get(e.site).unwrap();
                    let source = self.data.artifacts.get(o.source).unwrap();
                    if files(self.fixture)[&source.path.to_string()]
                        [o.start as usize..o.end as usize]
                        == *text.as_bytes()
                    {
                        eprintln!("unadmitted: {a:?}");
                    }
                }
                panic!("no replayed composition admission for {text}")
            })
            .id()
    }
}
pub async fn native() -> NativeFixture {
    native_from("transfer_composition").await
}
pub async fn native_from(fixture: &'static str) -> NativeFixture {
    let tables = typed_driver::Tables::default();
    typed_driver::run_behavioral(&files(fixture), Facts(tables.clone()))
        .await
        .unwrap();
    let budget = ResourceBudget::fixed(256 << 20).unwrap();
    let mut relations = relation_normalization::RelationData::new(&budget);
    macro_rules! facts { ($($field:ident: $ty:ty => $family:ident,)*) => { $(for row in rows::<$ty>(&tables) { relations.facts.$field.insert(row).unwrap(); })* }; }
    lctx_model::normalized_entity_inputs!(facts);
    relations.entities =
        entity_normalization::normalize(relations.facts.inputs(), &budget).unwrap();
    macro_rules! additional { ($($field:ident: $ty:ty => $family:ident,)*) => { $(for row in rows::<$ty>(&tables) { relations.$field.insert(row).unwrap(); })* }; }
    lctx_model::normalized_relation_inputs!(additional);
    let links = relation_normalization::normalize(&relations, &budget).unwrap();
    let mut data = BindingData::new(&budget);
    macro_rules! raw { ($($field:ident: $ty:ty,)*) => { $(for row in rows::<$ty>(&tables) { data.$field.insert(row).unwrap(); })* }; }
    lctx_model::normalized_binding_inputs!(raw);
    macro_rules! entities { ($($field:ident: $ty:ty,)*) => { $(data.visit(<$ty>::NAME, &<$ty as Record>::encode(&relations.entities.$field.iter().cloned().collect::<Vec<_>>()).unwrap()).unwrap();)* }; }
    lctx_model::normalized_entity_outputs!(entities);
    macro_rules! links { ($($field:ident: $ty:ty,)*) => { $(data.visit(<$ty>::NAME, &<$ty as Record>::encode(&links.$field.iter().cloned().collect::<Vec<_>>()).unwrap()).unwrap();)* }; }
    lctx_model::normalized_relation_outputs!(links);
    rebuild_callables(&mut data, &budget);
    rebuild_events(&mut data, &budget);
    let output = normalize(&data, &budget).unwrap();
    let verified = verify(&data, &output, &budget).unwrap();
    NativeFixture {
        fixture,
        tables,
        data,
        output,
        verified,
        budget,
    }
}
pub(crate) fn rebuild_events(data: &mut BindingData, budget: &ResourceBudget) {
    let mut receivers = lctx_model::domain::normalized::receiver::ReceiverData::new(budget);
    macro_rules! receiver_inputs {($($field:ident: $ty:ty,)*)=>{$(receivers.visit(<$ty>::NAME,&<$ty as Record>::encode(&data.$field.iter().cloned().collect::<Vec<_>>()).unwrap()).unwrap();)*};}
    lctx_model::normalized_binding_inputs!(receiver_inputs);
    let receivers =
        lctx_model::domain::normalized::receiver::normalize(&receivers, budget).unwrap();
    data.receiver_assessments = Rows::new(budget);
    data.receiver_evidence = Rows::new(budget);
    data.receiver_premises = Rows::new(budget);
    macro_rules! receiver_outputs {($($field:ident: $ty:ty,)*)=>{$(data.visit(<$ty>::NAME,&<$ty as Record>::encode(&receivers.$field.iter().cloned().collect::<Vec<_>>()).unwrap()).unwrap();)*};}
    lctx_model::normalized_receiver_outputs!(receiver_outputs);

    let mut events = event_normalization::EventData::new(budget);
    macro_rules! event_inputs { ($($field:ident: $ty:ty,)*) => { $(events.visit(<$ty>::NAME, &<$ty as Record>::encode(&data.$field.iter().cloned().collect::<Vec<_>>()).unwrap()).unwrap();)* }; }
    lctx_model::normalized_binding_inputs!(event_inputs);
    let events = event_normalization::normalize(&events, budget).unwrap();
    let mut retained = BindingData::new(budget);
    let event_relations = event_normalization::EventOutput::validation_inputs();
    macro_rules! retain_inputs { ($($field:ident: $ty:ty,)*) => { $(if !event_relations.iter().any(|input|input.name()==<$ty>::NAME) { retained.$field.decode(&<$ty as Record>::encode(&data.$field.iter().cloned().collect::<Vec<_>>()).unwrap()).unwrap(); })* }; }
    lctx_model::normalized_binding_inputs!(retain_inputs);
    *data = retained;
    macro_rules! event_outputs { ($($field:ident: $ty:ty,)*) => { $(data.visit(<$ty>::NAME, &<$ty as Record>::encode(&events.$field.iter().cloned().collect::<Vec<_>>()).unwrap()).unwrap();)* }; }
    lctx_model::normalized_event_outputs!(event_outputs);
}
pub(crate) fn rebuild_callables(data: &mut BindingData, budget: &ResourceBudget) {
    let mut callables = callable_normalization::CallableData::new(budget);
    macro_rules! inputs { ($($field:ident: $ty:ty,)*) => { $(callables.visit(<$ty>::NAME, &<$ty as Record>::encode(&data.$field.iter().cloned().collect::<Vec<_>>()).unwrap()).unwrap();)* }; }
    lctx_model::normalized_binding_inputs!(inputs);
    let callables = callable_normalization::normalize(&callables, budget).unwrap();
    data.callable_assessments = Rows::new(budget);
    data.callable_decorators = Rows::new(budget);
    data.callable_premises = Rows::new(budget);
    data.callable_evidence = Rows::new(budget);
    data.callable_variants = Rows::new(budget);
    data.callable_slots = Rows::new(budget);
    data.callable_slot_entities = Rows::new(budget);
    macro_rules! outputs { ($($field:ident: $ty:ty,)*) => { $(data.visit(<$ty>::NAME, &<$ty as Record>::encode(&callables.$field.iter().cloned().collect::<Vec<_>>()).unwrap()).unwrap();)* }; }
    lctx_model::normalized_callable_outputs!(outputs);
}

use lctx_model::domain::{
    assertion::*,
    composition::*,
    conditions::{rebase::GuardCatalog, stability::CheckedStability, *},
    declarations::*,
    normalized::entities::EntityRef,
    place_composition::PathCatalog,
    source::*,
    transfer::{local::TransferKey, *},
    value::*,
};
use std::collections::BTreeMap;
#[derive(Default)]
pub struct Places {
    pub assumption_sets: BTreeMap<
        Id<lctx_model::domain::assumptions::AssumptionSet>,
        lctx_model::domain::assumptions::AssumptionSet,
    >,
    pub assumption_members: BTreeMap<
        Id<lctx_model::domain::assumptions::AssumptionSetMember>,
        lctx_model::domain::assumptions::AssumptionSetMember,
    >,
    pub assumptions: BTreeMap<
        Id<lctx_model::domain::assumptions::Assumption>,
        lctx_model::domain::assumptions::Assumption,
    >,
    pub roots: BTreeMap<Id<PlaceRoot>, PlaceRoot>,
    pub places: BTreeMap<Id<Place>, Place>,
    pub paths: BTreeMap<Id<AccessPath>, AccessPath>,
    pub segments: BTreeMap<Id<PathSegment>, PathSegment>,
    pub literals: BTreeMap<Id<Literal>, Literal>,
    pub atoms: BTreeMap<Id<EvaluationAtom>, EvaluationAtom>,
    pub predicates: BTreeMap<Id<Predicate>, Predicate>,
}
impl Places {
    pub fn place(&mut self, root: PlaceRoot, path: AccessPath) -> Place {
        let row = Place {
            root: root.id(),
            path: path.id(),
        };
        self.roots.insert(root.id(), root);
        self.paths.insert(path.id(), path);
        self.places.insert(row.id(), row.clone());
        row
    }
    pub fn attribute(&mut self, name: &str) -> Id<PathSegment> {
        let row = PathSegment::Attribute { name: name.into() };
        let id = row.id();
        self.segments.insert(id, row);
        id
    }
    pub fn integer(&mut self, value: i64) -> Id<PathSegment> {
        let literal = Literal::Integer {
            decimal: value.to_string(),
        };
        let segment = PathSegment::Item { key: literal.id() };
        let id = segment.id();
        self.literals.insert(literal.id(), literal);
        self.segments.insert(id, segment);
        id
    }
    pub fn catalog(&self) -> CompositionCatalog<'_> {
        CompositionCatalog {
            assumptions: lctx_model::domain::assumptions::AssumptionCatalog {
                sets: &self.assumption_sets,
                members: &self.assumption_members,
                definitions: &self.assumptions,
            },
            guards: GuardCatalog {
                places: &self.places,
                roots: &self.roots,
                atoms: &self.atoms,
                predicates: &self.predicates,
            },
            paths: &self.paths,
            segments: PathCatalog {
                segments: &self.segments,
                literals: &self.literals,
            },
        }
    }
}
pub struct Case {
    pub attempt: Id<CallBindingAttempt>,
    pub site: Occurrence,
    pub target: CallTarget,
    pub qualification: AssertionQualification,
    pub destination: CallDestination,
    pub receiver: Receiver,
    pub arguments: Vec<CallArgument>,
    pub caller_declaration: SymbolDeclaration,
    pub caller_owner: Id<EntityRef>,
    pub callee_owner: Id<EntityRef>,
    pub callee_symbol: ProviderSymbol,
    pub callee_declaration: SymbolDeclaration,
    pub parameters: Vec<SignatureParameter>,
    pub links: Vec<ParameterDeclaration>,
    pub actuals: Vec<Id<Occurrence>>,
    pub places: Places,
}
impl NativeFixture {
    pub fn case(&self, text: &str) -> Case {
        let attempt = self.attempt(text);
        let checked = self.verified.bound(attempt).unwrap();
        let admission = self.verified.composition(attempt).unwrap();
        let bound = checked.bound();
        let target = self.data.targets.get(admission.target()).unwrap().clone();
        let site = self.data.occurrences.get(bound.site()).unwrap().clone();
        let qualification = self
            .data
            .qualifications
            .get(target.qualification)
            .unwrap()
            .clone();
        let destination = self
            .data
            .destinations
            .get(target.destination)
            .unwrap()
            .clone();
        let receiver = self.data.receivers.get(target.receiver).unwrap().clone();
        let CallDestination::Resolved { symbol } = destination else {
            panic!("fixture requires source callee")
        };
        let callee_symbol = self.data.symbols.get(symbol).unwrap().clone();
        let declarations = self.rows::<SymbolDeclaration>();
        let callee_declaration = declarations
            .iter()
            .find(|d| {
                d.symbol == symbol
                    && self
                        .data
                        .qualifications
                        .get(d.qualification)
                        .is_some_and(|q| q.context == qualification.context)
            })
            .unwrap()
            .clone();
        let caller_declaration = declarations
            .iter()
            .find(|d| {
                d.declaration == admission.owner_declaration()
                    && self
                        .data
                        .qualifications
                        .get(d.qualification)
                        .is_some_and(|q| q.context == qualification.context)
            })
            .unwrap()
            .clone();
        let mut parameters = self
            .data
            .parameters
            .iter()
            .filter(|p| p.signature == bound.signature())
            .cloned()
            .collect::<Vec<_>>();
        parameters.sort_by_key(|p| p.ordinal);
        let links = self
            .rows::<ParameterDeclaration>()
            .into_iter()
            .filter(|d| parameters.iter().any(|p| p.id() == d.parameter))
            .collect();
        let syntax = self.output.attempts.get(attempt).unwrap().syntax.unwrap();
        let mut arguments = self
            .data
            .arguments
            .iter()
            .filter(|a| a.call == syntax)
            .cloned()
            .collect::<Vec<_>>();
        arguments.sort_by_key(|a| a.ordinal);
        let actuals = arguments.iter().map(|a| a.value).collect();
        let mut places = Places::default();
        macro_rules! map {
            ($field:ident,$ty:ty) => {
                for row in self.rows::<$ty>() {
                    places.$field.insert(row.id(), row);
                }
            };
        }
        map!(roots, PlaceRoot);
        map!(places, Place);
        map!(paths, AccessPath);
        map!(segments, PathSegment);
        map!(literals, Literal);
        map!(atoms, EvaluationAtom);
        map!(predicates, Predicate);
        Case {
            attempt,
            site,
            target,
            qualification,
            destination,
            receiver,
            arguments,
            caller_declaration,
            caller_owner: admission.owner_entity(),
            callee_owner: admission.callee(),
            callee_symbol,
            callee_declaration,
            parameters,
            links,
            actuals,
            places,
        }
    }
}
impl Case {
    pub fn call<'a>(&'a self, f: &'a NativeFixture) -> CallFrame<'a> {
        CallFrame {
            site: &self.site,
            target: &self.target,
            qualification: &self.qualification,
            destination: &self.destination,
            receiver: &self.receiver,
            binding: Some(
                CallBindingFrame::new(&f.verified, self.attempt, &f.data, &f.output).unwrap(),
            ),
            arguments: &self.arguments,
        }
    }
    pub fn caller(&self) -> CallerFrame<'_> {
        CallerFrame {
            declaration: &self.caller_declaration,
            owner: self.caller_owner,
        }
    }
    pub fn callee<'a>(
        &'a self,
        witnesses: Option<&'a BTreeMap<Id<EvaluationAtom>, CheckedStability>>,
    ) -> CalleeFrame<'a> {
        CalleeFrame {
            symbol: &self.callee_symbol,
            declaration: &self.callee_declaration,
            parameters: &self.parameters,
            links: &self.links,
            witnesses,
        }
    }
    pub fn entry(&mut self, ordinal: usize, path: AccessPath) -> Place {
        let parameter = self.parameters[ordinal].id();
        let declaration = self
            .links
            .iter()
            .find(|link| link.parameter == parameter)
            .unwrap()
            .declaration;
        self.places.place(PlaceRoot::Entry { declaration }, path)
    }
    pub fn formal(&mut self, ordinal: usize, path: AccessPath) -> Place {
        let parameter = self.parameters[ordinal].id();
        let declaration = self
            .links
            .iter()
            .find(|link| link.parameter == parameter)
            .unwrap()
            .declaration;
        self.places.place(PlaceRoot::Formal { declaration }, path)
    }
    pub fn actual(&mut self, ordinal: usize, path: AccessPath) -> Place {
        self.places.place(
            PlaceRoot::Occurrence {
                occurrence: self.actuals[ordinal],
            },
            path,
        )
    }
    pub fn returned(&mut self, path: AccessPath) -> Place {
        self.places.place(
            PlaceRoot::Return {
                callable: self.callee_declaration.declaration,
            },
            path,
        )
    }
    pub fn branch(
        &self,
        f: &NativeFixture,
        owner: Id<EntityRef>,
        input: &Place,
        output: &Place,
        kind: TransferKind,
        condition: Diagram,
    ) -> TransferBranch {
        let qualification = AssertionQualification {
            condition: condition.id(),
            ..self.qualification.clone()
        };
        let key = TransferKey {
            owner,
            input: input.id(),
            output: output.id(),
            context: qualification.context,
            scope: qualification.scope,
            modality: qualification.modality,
            approximation: qualification.approximation,
            kind,
            call_site: None,
            provenance: ProvenanceClass::FlowLocal,
        };
        TransferBranch::new(key, qualification, condition, &f.budget).unwrap()
    }
    pub fn compose(
        &self,
        f: &NativeFixture,
        caller: &dyn CompositionOperand,
        callee: &dyn CompositionOperand,
    ) -> CompositionResults {
        let empty = BTreeMap::new();
        compose_call(
            caller,
            callee,
            &self.call(f),
            &self.caller(),
            &self.callee(Some(&empty)),
            &self.places.catalog(),
            &f.budget,
        )
        .unwrap()
    }
}
pub fn typed_contents() -> BTreeMap<String, Vec<u8>> {
    files("transfer_composition")
}
/// Algebra conformance rows over actual native attribution. Local producer membership lowering is
/// qualified separately by B0; this fixture does not fabricate a ProviderRun or native assertion.
#[derive(Debug, Clone, Copy)]
pub enum Mutation {
    CalleeFromAnotherOwner,
    NotComposed,
    UnrestatedCondition,
    ForeignOutput,
    ForgedPath,
    ForgedStatus,
    MissingSelectedEnumeration,
    MissingSelectedEnumerationSupport,
    ForeignSelectedEnumeration,
    ForeignSelectedEnumerationSupport,
}
pub struct WitnessFixture {
    pub early: typed_driver::Tables,
    pub tables: typed_driver::Tables,
    pub emission: SummaryEmission,
    pub composed: ComposedTransfer,
}
fn put<R: Record>(tables: &typed_driver::Tables, values: impl IntoIterator<Item = R>) {
    let mut map = rows::<R>(tables)
        .into_iter()
        .map(|r| (r.id(), r))
        .collect::<BTreeMap<_, _>>();
    for r in values {
        map.insert(r.id(), r);
    }
    tables.lock().unwrap().insert(
        R::NAME,
        R::encode(&map.into_values().collect::<Vec<_>>()).unwrap(),
    );
}
impl WitnessFixture {
    pub fn new(f: &NativeFixture) -> Self {
        Self::mutated(f, None)
    }
    pub fn mutated(f: &NativeFixture, mutation: Option<Mutation>) -> Self {
        Self::mutated_at(f, "identity(seed)", mutation)
    }
    pub fn mutated_at(f: &NativeFixture, text: &str, mutation: Option<Mutation>) -> Self {
        let tables = std::sync::Arc::new(std::sync::Mutex::new(f.tables.lock().unwrap().clone()));
        macro_rules! native {($($field:ident:$ty:ty,)*)=>{$(put::<$ty>(&tables,f.data.$field.iter().cloned());)*};}
        lctx_model::normalized_binding_inputs!(native);
        macro_rules! bound {($($field:ident:$ty:ty,)*)=>{$(put::<$ty>(&tables,f.output.$field.iter().cloned());)*};}
        lctx_model::normalized_binding_outputs!(bound);
        let mut inventory = analysis::native::NativeInventory::new(&f.budget);
        for (name, batch) in tables.lock().unwrap().iter() {
            if analysis::native::NativeInventory::inputs()
                .iter()
                .any(|input| input.name() == *name)
            {
                inventory.visit(name, batch).unwrap();
            }
        }
        let inventory = inventory.collect().unwrap();
        put(&tables, inventory.premises.iter().cloned());
        put(&tables, inventory.qualifications.iter().cloned());
        let input = f.rows::<input::InputRevision>()[0].id();
        let parameters = analysis::MethodParameters {
            depth: None,
            proof_steps: None,
            work: None,
            members: None,
            seed: None,
            iterations: None,
            threshold: None,
            resolution: None,
            damping: None,
            model_catalog: None,
        };
        put(&tables, [parameters.clone()]);
        let definition = analysis::AnalysisDefinition {
            method: analysis::AnalysisMethod::LocalTransfers,
            semantic_version: ContentHash::of(b"native transfer algebra conformance"),
            parameters: parameters.id(),
            interpretation: analysis::Interpretation::Structural,
        };
        put(&tables, [definition.clone()]);
        let mut c = f.case(text);
        let actual = c.actual(0, AccessPath::empty());
        let entry = c.entry(0, AccessPath::empty());
        let output = c.returned(AccessPath::empty());
        let caller_observation = f
            .rows::<flow::FlowValueObservation>()
            .into_iter()
            .find(|r| r.sink == c.actuals[0] && r.kind == flow::FlowSinkKind::Argument)
            .unwrap();
        let callee_observation = f
            .rows::<flow::FlowValueObservation>()
            .into_iter()
            .find(|r| {
                let sink = f.data.occurrences.get(r.sink).unwrap();
                r.kind == flow::FlowSinkKind::Return
                    && sink.source
                        == f.data
                            .occurrences
                            .get(c.callee_declaration.declaration)
                            .unwrap()
                            .source
                    && sink.start
                        > f.data
                            .occurrences
                            .get(c.callee_declaration.declaration)
                            .unwrap()
                            .start
                    && sink.end
                        <= f.data
                            .occurrences
                            .get(c.callee_declaration.declaration)
                            .unwrap()
                            .end
            })
            .unwrap();
        let mut branches = Vec::new();
        let mut supports = Vec::new();
        for (owner, input_place, out, observation) in [
            (c.caller_owner, &actual, &actual, caller_observation),
            (c.callee_owner, &entry, &output, callee_observation),
        ] {
            let qualification = f
                .data
                .qualifications
                .get(observation.qualification)
                .unwrap()
                .clone();
            let condition = Diagram::always();
            assert_eq!(qualification.condition, condition.id());
            let key = TransferKey {
                owner,
                input: input_place.id(),
                output: out.id(),
                context: qualification.context,
                scope: qualification.scope,
                modality: qualification.modality,
                approximation: qualification.approximation,
                kind: TransferKind::Identity,
                call_site: None,
                provenance: ProvenanceClass::FlowLocal,
            };
            let branch =
                TransferBranch::new(key, qualification.clone(), condition.clone(), &f.budget)
                    .unwrap();
            let native=inventory.premises.iter().find(|p|matches!(p,analysis::native::NativeAssertionPremise::Value {assertion,..} if *assertion==observation.id())).unwrap();
            let native_q = inventory
                .qualifications
                .iter()
                .find(|q| q.premise == native.id())
                .unwrap();
            let source = analysis::local::SupportSource::NativeAssertion {
                premise: native.id(),
            };
            let (invocation, parents) = analysis::local::AnalysisInvocation::new(
                input,
                qualification.context,
                definition.id(),
                Some(owner),
                [],
            );
            let subject = analysis::local::ObligationSubject::Transfer {
                transfer: branch.key().id(),
            };
            let (derivation, proposition, members, _) = analysis::local::AnalysisDerivation::emit(
                &invocation,
                &definition,
                subject.id(),
                analysis::AnalysisChannel::Value,
                calls::CallPhase::Call,
                analysis::support::QualificationOperation::Conjunction,
                &[analysis::local::support::EvidencePremise::native(
                    &source,
                    native_q,
                    &qualification,
                    &condition,
                )
                .unwrap()],
                &f.budget,
            )
            .unwrap();
            let derived = analysis::local::SupportSource::AnalysisDerivation {
                derivation: derivation.id(),
            };
            let support = transfer::local::TransferSupport {
                assertion: branch.alternative().id(),
                source: derived.id(),
            };
            put(&tables, [invocation]);
            put(&tables, parents);
            put(&tables, [subject]);
            put(&tables, [proposition]);
            put(&tables, [derivation]);
            put(&tables, members);
            put(&tables, [source, derived]);
            put(&tables, [branch.key().clone()]);
            put(&tables, [branch.alternative()]);
            put(&tables, [support.clone()]);
            supports.push(support);
            branches.push(branch);
        }
        put(&tables, c.places.roots.values().cloned());
        put(&tables, c.places.places.values().cloned());
        put(&tables, c.places.paths.values().cloned());
        let mut composed = match c.compose(f, &branches[0], &branches[1]).remove(0) {
            CallComposition::Transfer(t) => *t,
            r => panic!("{r:?}"),
        };
        let mut index = analysis::local::support::EvidenceIndex::new(&f.budget);
        for input in analysis::local::support::EvidenceIndex::inputs() {
            if let Some(batch) = tables.lock().unwrap().get(input.name()) {
                index.visit(input.name(), batch).unwrap();
            }
        }
        let mut evidence = branches
            .iter()
            .zip(&supports)
            .map(|(b, s)| {
                transfer::summary::TransferEvidence::local(
                    &b.alternative(),
                    std::slice::from_ref(s),
                    &index,
                    &f.budget,
                )
                .unwrap()
            })
            .collect::<Vec<_>>();
        if let Some(mutation) = mutation {
            match mutation {
                Mutation::CalleeFromAnotherOwner => {
                    composed.witness.callee = composed.witness.caller.clone();
                    evidence.pop();
                }
                Mutation::NotComposed => {
                    composed.descriptor.provenance = ProvenanceClass::FlowLocal
                }
                Mutation::UnrestatedCondition => {
                    composed.condition = Diagram::never();
                    composed.qualification.condition = composed.condition.id();
                    composed.records.qualification = Some(composed.qualification.clone());
                    let (condition, nodes) = composed.condition.records();
                    composed.records.conditions = vec![condition];
                    composed.records.nodes = nodes;
                }
                Mutation::ForeignOutput => {
                    let place = c.places.place(
                        PlaceRoot::Occurrence {
                            occurrence: c.callee_declaration.declaration,
                        },
                        AccessPath::empty(),
                    );
                    composed.descriptor.output = place.id();
                    put(&tables, c.places.roots.values().cloned());
                    put(&tables, c.places.places.values().cloned());
                }
                Mutation::ForgedPath => {
                    let segment = c.places.attribute("forged");
                    let place = c.actual(0, AccessPath::empty().extend(segment));
                    composed.descriptor.output = place.id();
                    put(&tables, c.places.roots.values().cloned());
                    put(&tables, c.places.places.values().cloned());
                    put(&tables, c.places.paths.values().cloned());
                    put(&tables, c.places.segments.values().cloned());
                }
                Mutation::MissingSelectedEnumeration => {
                    assert!(
                        composed
                            .witness
                            .selected_signature_enumeration
                            .take()
                            .is_some()
                    );
                }
                Mutation::MissingSelectedEnumerationSupport => {
                    assert!(
                        composed
                            .witness
                            .selected_signature_enumeration_support
                            .take()
                            .is_some()
                    );
                }
                Mutation::ForeignSelectedEnumeration => {
                    composed.witness.selected_signature_enumeration = Some(
                        f.data
                            .signature_enumerations
                            .iter()
                            .find(|row| {
                                Some(row.id()) != composed.witness.selected_signature_enumeration
                            })
                            .expect("another actual native enumeration")
                            .id(),
                    );
                }
                Mutation::ForeignSelectedEnumerationSupport => {
                    composed.witness.selected_signature_enumeration_support = Some(
                        f.data
                            .signature_enumeration_supports
                            .iter()
                            .find(|row| {
                                Some(row.id())
                                    != composed.witness.selected_signature_enumeration_support
                            })
                            .expect("another actual native enumeration support")
                            .id(),
                    );
                }
                Mutation::ForgedStatus => {}
            }
        }
        let definition = analysis::AnalysisDefinition {
            method: analysis::AnalysisMethod::Summaries,
            ..definition
        };
        let (invocation, _) = analysis::summary::AnalysisInvocation::new(
            input,
            composed.qualification.context,
            definition.id(),
            Some(c.caller_owner),
            [],
        );
        put(&tables, [definition.clone()]);
        put(&tables, [invocation.clone()]);
        let mut emission = composed.emit(&invocation, &evidence, &f.budget).unwrap();
        if matches!(mutation, Some(Mutation::ForgedStatus)) {
            emission.witness.status = analysis::policy::EvidenceStatus::FixtureChecked;
        }
        let mut result = Self {
            early: f.tables.clone(),
            tables,
            emission,
            composed,
        };
        result.lower(&invocation, &definition, &f.budget);
        result
    }
    fn lower(
        &mut self,
        invocation: &analysis::summary::AnalysisInvocation,
        definition: &analysis::AnalysisDefinition,
        budget: &ResourceBudget,
    ) {
        let e = &self.emission;
        put(&self.tables, [e.key.clone()]);
        put(&self.tables, [e.alternative.clone()]);
        put(&self.tables, e.premises.iter().cloned());
        put(&self.tables, [e.witness.clone()]);
        put(&self.tables, [e.contribution.clone()]);
        let r = &self.composed.records;
        put(&self.tables, r.qualification.iter().cloned());
        put(&self.tables, r.literals.iter().cloned());
        put(&self.tables, r.segments.iter().cloned());
        put(&self.tables, r.roots.iter().cloned());
        put(&self.tables, r.places.iter().cloned());
        put(&self.tables, r.paths.iter().cloned());
        put(&self.tables, r.atoms.iter().cloned());
        put(&self.tables, r.predicates.iter().cloned());
        put(&self.tables, r.conditions.iter().cloned());
        put(&self.tables, r.nodes.iter().cloned());
        put(&self.tables, r.substitutions.iter().cloned());
        put(&self.tables, r.influences.iter().cloned());
        let source = analysis::summary::SupportSource::TransferWitness {
            witness: e.witness.id(),
        };
        let subject = analysis::summary::ObligationSubject::SummaryTransfer {
            transfer: e.key.id(),
        };
        let (derivation, proposition, members, _) = analysis::summary::AnalysisDerivation::emit(
            invocation,
            definition,
            subject.id(),
            analysis::AnalysisChannel::Value,
            calls::CallPhase::Call,
            analysis::support::QualificationOperation::Conjunction,
            &[analysis::summary::support::EvidencePremise::derived(
                &source,
                &e.witness,
                &self.composed.qualification,
                &self.composed.condition,
            )
            .unwrap()],
            budget,
        )
        .unwrap();
        let derived = analysis::summary::SupportSource::AnalysisDerivation {
            derivation: derivation.id(),
        };
        let support = transfer::summary::TransferSupport {
            assertion: e.alternative.id(),
            source: derived.id(),
        };
        put(&self.tables, [source, derived]);
        put(&self.tables, [subject]);
        put(&self.tables, [derivation]);
        put(&self.tables, [proposition]);
        put(&self.tables, members);
        put(&self.tables, [support]);
    }
    pub fn rows<R: Record>(&self) -> Vec<R> {
        rows(&self.tables)
    }
    pub fn check<R: Record>(&self, budget: &ResourceBudget) -> Result<(), ModelError> {
        let model = lctx_model_for_fixture();
        for invariant in lctx_model::domain::validation::invariants_for::<R>() {
            let mut check = (invariant.create)(budget);
            for input in invariant.inputs {
                let source = if input.prefix() == Some(stages::PublicationBoundary::Facts) {
                    &self.early
                } else {
                    &self.tables
                };
                let batch = source
                    .lock()
                    .unwrap()
                    .get(input.name())
                    .cloned()
                    .unwrap_or_else(|| {
                        arrow_array::RecordBatch::new_empty(
                            model
                                .relations()
                                .iter()
                                .find(|r| r.name() == input.name())
                                .unwrap()
                                .schema()
                                .clone(),
                        )
                    });
                check.visit_input(&input, &batch)?;
            }
            check.finish()?;
        }
        Ok(())
    }
}

pub fn lctx_model_for_fixture() -> ValidatedModel {
    model().unwrap()
}
impl NativeFixture {
    pub fn entry_data(&self) -> conditions::entry::EntryData {
        let mut data = conditions::entry::EntryData::new(&self.budget);
        macro_rules! facts {($($field:ident:$ty:ty,)*)=>{$(for row in self.rows::<$ty>() {data.$field.insert(row).unwrap();})*};}
        lctx_model::entry_value_inputs!(facts);
        macro_rules! normalized {($($field:ident:$ty:ty,)*)=>{$(data.visit(<$ty>::NAME,&<$ty as Record>::encode(&self.data.$field.iter().cloned().collect::<Vec<_>>()).unwrap()).unwrap();)*};}
        lctx_model::normalized_binding_inputs!(normalized);
        data
    }
}
impl Case {
    pub fn guards(&self, f: &NativeFixture) -> BTreeMap<Id<EvaluationAtom>, CheckedStability> {
        let data = f.entry_data();
        let declaration = data
            .occurrences
            .get(self.callee_declaration.declaration)
            .unwrap();
        let mut result = BTreeMap::new();
        for atom in data.atoms.iter().filter(|atom| {
            data.predicates.get(atom.predicate) == Some(&Predicate::IsNone)
                && data.occurrences.get(atom.evaluation).is_some_and(|e| {
                    e.source == declaration.source
                        && e.start >= declaration.start
                        && e.end <= declaration.end
                })
        }) {
            let eval = data.occurrences.get(atom.evaluation).unwrap();
            let read = data
                .uses
                .iter()
                .find(|u| {
                    data.occurrences.get(u.occurrence).is_some_and(|r| {
                        r.source == eval.source
                            && r.structural_path.len() > eval.structural_path.len()
                            && r.structural_path.starts_with(&eval.structural_path)
                    })
                })
                .unwrap();
            let PlaceRoot::Formal { declaration } = data
                .roots
                .get(data.places.get(read.place).unwrap().root)
                .unwrap()
            else {
                panic!("native guarded formal")
            };
            let formal = normalized::entities::ParameterEntity::Source {
                declaration: *declaration,
            };
            let support = data
                .use_observations
                .iter()
                .filter(|o| o.use_ == read.id())
                .find_map(|o| data.use_supports.iter().find(|s| s.assertion == o.id()))
                .unwrap();
            let run = data.runs.get(support.run).unwrap();
            let request = conditions::entry::EntryRequest {
                owner: self.callee_owner,
                formal: formal.id(),
                access: read.occurrence,
                context: run.context,
                run: run.id(),
            };
            let entry = conditions::entry::EntryValueWitness::derive_for(
                &data,
                request,
                &{
                    let leaf = data
                        .leaves
                        .iter()
                        .find(|leaf| leaf.atom == atom.id())
                        .expect("actual native guard leaf");
                    let support = data
                        .leaf_supports
                        .iter()
                        .find(|support| support.assertion == leaf.id())
                        .expect("actual native guard support");
                    conditions::entry::EntryAccessSource::guard(
                        &data,
                        request,
                        leaf.id(),
                        support.id(),
                    )
                    .unwrap()
                },
                &f.budget,
            )
            .unwrap()
            .unwrap();
            let stability =
                conditions::stability::StabilityWitness::derive(&data, atom.id(), &entry).unwrap();
            result.insert(atom.id(), stability);
        }
        assert!(!result.is_empty());
        result
    }
}
