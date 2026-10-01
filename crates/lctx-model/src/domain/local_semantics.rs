//! Local semantic observations replay exact native flow and private entry-value proofs.
//! This bounded direct operator retains unsupported paths as assessments, never false flow.
use crate::Domain;
use crate::domain::{
    analysis::{
        self, local as publication, native::*, policy::EvidenceStatus, support::SourceFacts,
    },
    assertion::*,
    attribution::*,
    conditions::{entry::*, stability::*, *},
    flow::*,
    normalized::{Rows, entities::*},
    source::*,
    transfer::{local::*, *},
    value::*,
    *,
};
/// The sole Local configuration: code and supported finite work limits determine identity.
pub fn definition() -> (analysis::MethodParameters, analysis::AnalysisDefinition) {
    let parameters = analysis::MethodParameters {
        depth: Some(256),
        proof_steps: Some(32_768),
        work: Some(100_000),
        members: Some(4096),
        seed: None,
        iterations: None,
        threshold: None,
        resolution: None,
        damping: None,
        model_catalog: None,
    };
    let mut sink = KeySink::new("Local semantic definition");
    for code in [
        include_bytes!("local_semantics.rs").as_slice(),
        include_bytes!("local_theory.rs").as_slice(),
        include_bytes!("local_fields.rs").as_slice(),
        include_bytes!("conditions/entry.rs").as_slice(),
        include_bytes!("conditions/stability.rs").as_slice(),
    ] {
        ContentHash::of(code).encode(&mut sink);
    }
    parameters.id().encode(&mut sink);
    let definition = analysis::AnalysisDefinition {
        method: analysis::AnalysisMethod::LocalTransfers,
        semantic_version: ContentHash::of(sink.finish().0.as_slice()),
        parameters: parameters.id(),
        interpretation: analysis::Interpretation::ExactUnderContext,
    };
    (parameters, definition)
}
pub fn check_definition(selected: &analysis::AnalysisDefinition) -> Result<(), ModelError> {
    if *selected != definition().1 {
        return Err(invalid(
            "Local definition differs from supported code/configuration",
        ));
    }
    Ok(())
}
fn check_invocation(invocation: &publication::AnalysisInvocation) -> Result<(), ModelError> {
    if invocation.definition != definition().1.id() {
        return Err(invalid(
            "Local invocation differs from supported code/configuration",
        ));
    }
    Ok(())
}
#[macro_export]
macro_rules! local_semantic_inputs {
    ($apply:ident) => {
        $apply! {
         native:$crate::domain::analysis::native::NativeQualification,
        }
    };
}
macro_rules! data {($($field:ident:$ty:ty,)*)=>{
 pub struct LocalData {pub entry:EntryData,pub theory:crate::domain::local_theory::TheoryInventory,pub fields:crate::domain::local_fields::FieldInventory,$(pub $field:Rows<$ty>,)*}
 impl LocalData {
 pub fn new(budget:&resources::ResourceBudget)->Self {Self {entry:EntryData::new(budget),theory:crate::domain::local_theory::TheoryInventory::new(budget),fields:crate::domain::local_fields::FieldInventory::new(budget),$($field:Rows::new(budget),)*}}
 pub fn visit(&mut self,name:&str,batch:&arrow_array::RecordBatch)->Result<bool,ModelError>{if self.entry.visit(name,batch)?||self.theory.visit(name,batch)?||self.fields.visit(name,batch)? {return Ok(true);}$(if name==<$ty>::NAME {self.$field.decode(batch)?;return Ok(true);})*Ok(false)}
 pub fn validation_inputs()->Vec<ValidationInput>{let mut inputs=EntryData::validation_inputs();inputs.extend(crate::domain::local_theory::TheoryInventory::validation_inputs());inputs.extend(crate::domain::local_fields::FieldInventory::validation_inputs());inputs.extend([$(ValidationInput::of::<$ty>(&["id"]),)*]);for input in &mut inputs {if stages::is_vocabulary(input.name()) {*input=input.clone().at_epoch(stages::PublicationBoundary::Facts);}}inputs}
 }
};}
crate::local_semantic_inputs!(data);
#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(name="local_flow_assessments",invariants=local_invariants)]
pub struct LocalAssessment {
    #[model(key)]
    pub invocation: Id<publication::AnalysisInvocation>,
    #[model(key)]
    pub value: Id<FlowValueObservation>,
    #[model(key)]
    pub support: Id<FlowValueSupport>,
    pub contribution: Option<Id<LocalContribution>>,
    pub reason: Option<ObligationKind>,
}
/// One witnessed semantic occurrence; aggregate keys never substitute for this proof.
#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(name = "local_flow_contributions", rule = "local_entry_value_transfer")]
pub struct LocalContribution {
    #[model(key)]
    pub invocation: Id<publication::AnalysisInvocation>,
    #[model(key, premise)]
    pub value: Id<FlowValueObservation>,
    #[model(key, premise)]
    pub support: Id<FlowValueSupport>,
    #[model(key, premise)]
    pub entry: Id<EntryValueWitness>,
    #[model(key, premise)]
    pub definition: Option<Id<FlowDefinitionObservation>>,
    #[model(key, premise)]
    pub definition_support: Option<Id<FlowDefinitionSupport>>,
    #[model(key)]
    pub transfer: Id<TransferKey>,
    #[model(key)]
    pub qualification: Id<AssertionQualification>,
    status: EvidenceStatus,
}
impl analysis::support::sealed::DerivedEvidence for LocalContribution {}
impl analysis::support::DerivedEvidence for LocalContribution {
    fn source_facts(&self) -> SourceFacts {
        SourceFacts {
            qualification: self.qualification,
            status: self.status,
            heuristic: false,
        }
    }
}
/// Private producer result retains its entry proof and condition allocations through publication.
pub struct LocalEmission {
    contribution: LocalContribution,
    entry: DerivedEntryValue,
    branch: TransferBranch,
    output_root: PlaceRoot,
    output_place: Place,
}
impl LocalEmission {
    pub fn contribution(&self) -> &LocalContribution {
        &self.contribution
    }
    pub fn entry(&self) -> &DerivedEntryValue {
        &self.entry
    }
    pub fn branch(&self) -> &TransferBranch {
        &self.branch
    }
    pub fn output_root(&self) -> &PlaceRoot {
        &self.output_root
    }
    pub fn output_place(&self) -> &Place {
        &self.output_place
    }
}
fn need<R: Record>(rows: &Rows<R>, id: Id<R>) -> Result<&R, ObligationKind> {
    rows.get(id).ok_or(ObligationKind::MissingEvidence)
}
fn invalid(message: &str) -> ModelError {
    ModelError::Invalid(message.into())
}
impl LocalContribution {
    pub fn derive(
        data: &LocalData,
        invocation: &publication::AnalysisInvocation,
        value: Id<FlowValueObservation>,
        support: Id<FlowValueSupport>,
        budget: &resources::ResourceBudget,
    ) -> Result<Result<LocalEmission, ObligationKind>, ModelError> {
        check_invocation(invocation)?;
        let value = need(&data.entry.values, value)
            .map_err(|e| invalid(&format!("local value absent: {e:?}")))?;
        let support = need(&data.entry.value_supports, support)
            .map_err(|e| invalid(&format!("local support absent: {e:?}")))?;
        let setup = (|| {
            if support.assertion != value.id() {
                return Err(ObligationKind::MissingEvidence);
            }
            let attribution = support
                .attribution()
                .ok_or(ObligationKind::MissingEvidence)?;
            let run = need(&data.entry.runs, attribution.run)?;
            let q = need(&data.entry.qualifications, value.qualification)?;
            if (run.input, run.context) != (invocation.input, invocation.context)
                || q.context != invocation.context
            {
                return Err(ObligationKind::IncompatibleContexts);
            }
            if q.modality != Modality::Definite || q.approximation != Approximation::Exact {
                return Err(ObligationKind::Approximation);
            }
            if value.through_call {
                return Err(ObligationKind::CallTransfer);
            }
            let use_ = need(&data.entry.uses, value.use_)?;
            let place = need(&data.entry.places, use_.place)?;
            let PlaceRoot::Formal { declaration } = need(&data.entry.roots, place.root)? else {
                return Err(ObligationKind::EntryValueUnknown);
            };
            if place.path != AccessPath::empty().id() {
                return Err(ObligationKind::EntryValueUnknown);
            }
            let owner = data
                .entry
                .owners
                .iter()
                .find(|row| row.occurrence == use_.occurrence)
                .ok_or(ObligationKind::MissingEvidence)?;
            let EntityRef::Callable { callable } = need(&data.entry.refs, owner.entity)? else {
                return Err(ObligationKind::EntryValueUnknown);
            };
            let CallableEntity::Source {
                declaration: callee,
                ..
            } = need(&data.entry.callables, *callable)?
            else {
                return Err(ObligationKind::EntryValueUnknown);
            };
            if !data
                .entry
                .owners
                .iter()
                .any(|row| row.occurrence == value.sink && row.entity == owner.entity)
            {
                return Err(ObligationKind::CapturedStateUnavailable);
            }
            let formal = ParameterEntity::Source {
                declaration: *declaration,
            };
            let request = EntryRequest {
                owner: owner.entity,
                formal: formal.id(),
                access: use_.occurrence,
                context: run.context,
                run: run.id(),
            };
            let root =
                match value.kind {
                    FlowSinkKind::Return => PlaceRoot::Return { callable: *callee },
                    FlowSinkKind::Yield => PlaceRoot::Yield { callable: *callee },
                    FlowSinkKind::Raise => PlaceRoot::Raise { callable: *callee },
                    FlowSinkKind::Argument => PlaceRoot::Occurrence {
                        occurrence: value.sink,
                    },
                    FlowSinkKind::Definition => {
                        let mut selected = None;
                        for observation in data
                            .entry
                            .definition_observations
                            .iter()
                            .filter(|d| d.value == Some(value.sink))
                        {
                            if !matches!(
                                observation.kind,
                                crate::domain::lexical::BindingEventKind::Assignment
                                    | crate::domain::lexical::BindingEventKind::Walrus
                            ) {
                                continue;
                            }
                            let dq = need(&data.entry.qualifications, observation.qualification)?;
                            if (
                                dq.context,
                                dq.scope,
                                dq.modality,
                                dq.approximation,
                                dq.condition,
                            ) != (
                                q.context,
                                q.scope,
                                Modality::Definite,
                                Approximation::Exact,
                                Diagram::always().id(),
                            ) {
                                continue;
                            }
                            let row = need(&data.entry.definitions, observation.definition)?;
                            let target = need(&data.entry.occurrences, row.occurrence)?;
                            let rhs = need(&data.entry.occurrences, value.sink)?;
                            if target.source != rhs.source
                                || !data.entry.owners.iter().any(|o| {
                                    o.occurrence == target.id() && o.entity == owner.entity
                                })
                            {
                                continue;
                            }
                            if !data.entry.use_observations.iter().any(|u| {
                                u.use_ == use_.id()
                                    && u.scope == observation.scope
                                    && data
                                        .entry
                                        .qualifications
                                        .get(u.qualification)
                                        .is_some_and(|uq| uq.context == q.context)
                            }) {
                                continue;
                            }
                            for ds in data.entry.definition_supports.iter().filter(|s| {
                                s.assertion == observation.id()
                                    && s.attribution().is_some_and(|a| a.run == run.id())
                            }) {
                                let premise = NativeAssertionPremise::Definition {
                                    assertion: observation.id(),
                                    support: ds.id(),
                                };
                                let Some(native) = data.native.iter().find(|n| {
                                    n.premise == premise.id()
                                        && n.qualification == dq.id()
                                        && n.family == FactFamily::Flow
                                        && n.fidelity == Fidelity::NativeStructural
                                }) else {
                                    continue;
                                };
                                if selected.is_some() {
                                    return Err(ObligationKind::MissingEvidence);
                                }
                                selected = Some((row, observation.id(), ds.id(), native.status));
                            }
                        }
                        let (row, definition, definition_support, status) =
                            selected.ok_or(ObligationKind::MissingEvidence)?;
                        let output = need(&data.entry.places, row.place)?;
                        let root = need(&data.entry.roots, output.root)?;
                        if matches!(root, PlaceRoot::Field { .. } | PlaceRoot::Global { .. }) {
                            return Err(ObligationKind::CapturedStateUnavailable);
                        }
                        return Ok((
                            request,
                            q.clone(),
                            root.clone(),
                            Some(output.clone()),
                            Some((definition, definition_support, status)),
                        ));
                    }
                };
            Ok((request, q.clone(), root, None, None))
        })();
        let (request, mut q, root, existing, definition) = match setup {
            Ok(rows) => rows,
            Err(reason) => return Ok(Err(reason)),
        };
        let source = match EntryAccessSource::value(&data.entry, request, value.id(), support.id())
        {
            Ok(source) => source,
            Err(reason) => return Ok(Err(reason)),
        };
        let entry = match EntryValueWitness::derive_for(&data.entry, request, &source, budget)? {
            Ok(proof) => proof,
            Err(reason) => return Ok(Err(reason)),
        };
        let premise = NativeAssertionPremise::Value {
            assertion: value.id(),
            support: support.id(),
        };
        let Some(native) = data.native.iter().find(|row| {
            row.premise == premise.id()
                && row.qualification == q.id()
                && row.family == FactFamily::Flow
                && row.fidelity == Fidelity::NativeStructural
        }) else {
            return Ok(Err(ObligationKind::MissingEvidence));
        };
        let EntryAccessSource::Value {
            region,
            region_support,
            ..
        } = entry.source()
        else {
            unreachable!()
        };
        let region_row =
            need(&data.entry.regions, *region).map_err(|_| invalid("Local entry region absent"))?;
        let region_premise = NativeAssertionPremise::Region {
            assertion: *region,
            support: *region_support,
        };
        let Some(region_native) = data.native.iter().find(|row| {
            row.premise == region_premise.id()
                && row.qualification == region_row.qualification
                && row.family == FactFamily::Flow
                && row.fidelity == Fidelity::NativeStructural
        }) else {
            return Ok(Err(ObligationKind::MissingEvidence));
        };
        if region_native.status != native.status {
            return Ok(Err(ObligationKind::MissingEvidence));
        }
        q = entry.qualification().clone();
        let _clone_allowance = budget.reserve(
            "local_entry_condition_clone",
            entry.condition().allocation_allowance(),
        )?;
        let condition = entry.condition().clone();
        let output = existing.unwrap_or(Place {
            root: root.id(),
            path: AccessPath::empty().id(),
        });
        let key = TransferKey::from_descriptor(TransferDescriptor {
            owner: request.owner,
            input: entry.place().id(),
            output: output.id(),
            context: q.context,
            scope: q.scope,
            modality: q.modality,
            approximation: q.approximation,
            kind: value.transfer,
            call_site: None,
            provenance: ProvenanceClass::FlowLocal,
        });
        let contribution = LocalContribution {
            invocation: invocation.id(),
            value: value.id(),
            support: support.id(),
            entry: entry.witness().id(),
            definition: definition.map(|d| d.0),
            definition_support: definition.map(|d| d.1),
            transfer: key.id(),
            qualification: q.id(),
            status: analysis::policy::derive_status(&[
                (analysis::policy::SupportRole::Support, native.status),
                (analysis::policy::SupportRole::Support, region_native.status),
                (
                    analysis::policy::SupportRole::Support,
                    entry.evidence_status(),
                ),
                (
                    analysis::policy::SupportRole::Support,
                    definition.map_or(native.status, |d| d.2),
                ),
            ]),
        };
        Ok(Ok(LocalEmission {
            contribution,
            entry,
            branch: TransferBranch::new(key, q, condition, budget)?,
            output_root: root,
            output_place: output,
        }))
    }
}
pub fn relations() -> Vec<Relation> {
    vec![
        Relation::of::<LocalAssessment>(),
        Relation::of::<LocalContribution>(),
        Relation::of::<LocalGuardContribution>(),
        Relation::of::<LocalGuardAssessment>(),
    ]
}
fn local_invariants() -> Vec<Invariant> {
    let mut inputs = LocalData::validation_inputs();
    inputs.extend([
        ValidationInput::of::<publication::AnalysisInvocation>(&["id"]),
        ValidationInput::of::<EntryValueWitness>(&["id"]),
        ValidationInput::of::<EntryAccessSource>(&["id"]),
        ValidationInput::of::<TransferKey>(&["id"]),
        ValidationInput::of::<LocalContribution>(&["id"]),
        ValidationInput::of::<LocalAssessment>(&["id"]),
        ValidationInput::of::<LocalGuardContribution>(&["id"]),
        ValidationInput::of::<LocalGuardAssessment>(&["id"]),
        ValidationInput::of::<StabilityWitness>(&["id"]),
        ValidationInput::of::<ControlInfluence>(&["id"]),
    ]);
    vec![Invariant {
        name: "local_semantic_replay",
        inputs,
        create: std::sync::Arc::new(|budget| {
            Box::new(LocalCheck {
                data: LocalData::new(budget),
                budget: budget.clone(),
                invocations: Rows::new(budget),
                entries: Rows::new(budget),
                entry_sources: Rows::new(budget),
                keys: Rows::new(budget),
                contributions: Rows::new(budget),
                assessments: Rows::new(budget),
                guards: Rows::new(budget),
                guard_assessments: Rows::new(budget),
                stability: Rows::new(budget),
                influences: Rows::new(budget),
            })
        }),
    }]
}
struct LocalCheck {
    data: LocalData,
    budget: resources::ResourceBudget,
    invocations: Rows<publication::AnalysisInvocation>,
    entries: Rows<EntryValueWitness>,
    entry_sources: Rows<EntryAccessSource>,
    keys: Rows<TransferKey>,
    contributions: Rows<LocalContribution>,
    assessments: Rows<LocalAssessment>,
    guards: Rows<LocalGuardContribution>,
    guard_assessments: Rows<LocalGuardAssessment>,
    stability: Rows<StabilityWitness>,
    influences: Rows<ControlInfluence>,
}
impl InvariantCheck for LocalCheck {
    fn visit_input(
        &mut self,
        input: &ValidationInput,
        batch: &arrow_array::RecordBatch,
    ) -> Result<(), ModelError> {
        if stages::is_vocabulary(input.name())
            && input.prefix() != Some(stages::PublicationBoundary::Facts)
        {
            return Err(invalid("Local native replay requires Facts vocabulary"));
        }
        self.visit(input.name(), batch)
    }
    fn visit(&mut self, name: &str, batch: &arrow_array::RecordBatch) -> Result<(), ModelError> {
        if self.data.visit(name, batch)? {
            return Ok(());
        }
        macro_rules! row {
            ($ty:ty,$field:ident) => {
                if name == <$ty>::NAME {
                    self.$field.decode(batch)?;
                    return Ok(());
                }
            };
        }
        row!(publication::AnalysisInvocation, invocations);
        row!(EntryValueWitness, entries);
        row!(EntryAccessSource, entry_sources);
        row!(TransferKey, keys);
        row!(LocalContribution, contributions);
        row!(LocalAssessment, assessments);
        row!(LocalGuardContribution, guards);
        row!(LocalGuardAssessment, guard_assessments);
        row!(StabilityWitness, stability);
        row!(ControlInfluence, influences);
        Err(invalid("undeclared Local replay input"))
    }
    fn finish(self: Box<Self>) -> Result<(), ModelError> {
        let mut seen = charged::ChargedSet::default();
        let mut charge = charged::StateCharge::new(&self.budget, "local_replay_members");
        for assessment in self.assessments.iter() {
            let invocation = self
                .invocations
                .get(assessment.invocation)
                .ok_or_else(|| invalid("Local assessment invocation absent"))?;
            match LocalContribution::derive(
                &self.data,
                invocation,
                assessment.value,
                assessment.support,
                &self.budget,
            )? {
                Ok(emission) => {
                    if assessment.reason.is_some()
                        || assessment.contribution != Some(emission.contribution.id())
                        || self.contributions.get(emission.contribution.id())
                            != Some(&emission.contribution)
                        || self.entries.get(emission.entry.witness().id())
                            != Some(emission.entry.witness())
                        || self.entry_sources.get(emission.entry.source().id())
                            != Some(emission.entry.source())
                        || self.keys.get(emission.branch.key().id()) != Some(emission.branch.key())
                    {
                        return Err(invalid("stored Local contribution differs from replay"));
                    }
                    seen.insert(&mut charge, emission.contribution.id())?;
                }
                Err(reason) => {
                    if assessment.reason != Some(reason) || assessment.contribution.is_some() {
                        return Err(invalid("stored Local boundary differs from replay"));
                    }
                }
            }
        }
        if seen.len() != self.contributions.len() {
            return Err(invalid("orphan Local contribution"));
        }
        let mut guards = charged::ChargedSet::default();
        let mut influences = charged::ChargedSet::default();
        for assessment in self.guard_assessments.iter() {
            let invocation = self
                .invocations
                .get(assessment.invocation)
                .ok_or_else(|| invalid("Local guard invocation absent"))?;
            match LocalGuardContribution::derive(
                &self.data,
                invocation,
                assessment.leaf,
                assessment.support,
                &self.budget,
            )? {
                Ok(emission) => {
                    if assessment.reason.is_some()
                        || assessment.contribution != Some(emission.proof.id())
                        || self.guards.get(emission.proof.id()) != Some(&emission.proof)
                        || self.entries.get(emission.entry.witness().id())
                            != Some(emission.entry.witness())
                        || self.entry_sources.get(emission.entry.source().id())
                            != Some(emission.entry.source())
                        || self.stability.get(emission.stability.witness().id())
                            != Some(emission.stability.witness())
                        || self.influences.get(emission.influence.id()) != Some(&emission.influence)
                    {
                        return Err(invalid("stored Local guard differs from replay"));
                    }
                    guards.insert(&mut charge, emission.proof.id())?;
                    influences.insert(&mut charge, emission.influence.id())?;
                }
                Err(reason) => {
                    if assessment.reason != Some(reason) || assessment.contribution.is_some() {
                        return Err(invalid("stored Local guard boundary differs from replay"));
                    }
                }
            }
        }
        if guards.len() != self.guards.len() || influences.len() != self.influences.len() {
            return Err(invalid("orphan Local guard/influence"));
        }
        Ok(())
    }
}

#[macro_export]
macro_rules! local_semantic_outputs {
    ($apply:ident) => {
        $apply! {
         assessments:$crate::domain::local_semantics::LocalAssessment,
         contributions:$crate::domain::local_semantics::LocalContribution,
         guard_assessments:$crate::domain::local_semantics::LocalGuardAssessment,
         guards:$crate::domain::local_semantics::LocalGuardContribution,
         stability:$crate::domain::conditions::stability::StabilityWitness,
         influences:$crate::domain::transfer::local::ControlInfluence,
         control_supports:$crate::domain::transfer::local::ControlSupport,
         selections:$crate::domain::transfer::local::Selection,
         entries:$crate::domain::conditions::entry::EntryValueWitness,
         entry_sources:$crate::domain::conditions::entry::EntryAccessSource,
         roots:$crate::domain::value::PlaceRoot,
         places:$crate::domain::value::Place,
         conditions:$crate::domain::conditions::Condition,
         nodes:$crate::domain::conditions::ConditionNode,
         qualifications:$crate::domain::assertion::AssertionQualification,
         keys:$crate::domain::transfer::local::TransferKey,
         alternatives:$crate::domain::transfer::local::TransferAlternative,
         supports:$crate::domain::transfer::local::TransferSupport,
         subjects:$crate::domain::analysis::local::ObligationSubject,
         sources:$crate::domain::analysis::local::SupportSource,
         propositions:$crate::domain::analysis::local::AnalysisProposition,
         derivations:$crate::domain::analysis::local::AnalysisDerivation,
         premises:$crate::domain::analysis::local::AnalysisDerivationPremise,
        }
    };
}
macro_rules! output {($($field:ident:$ty:ty,)*)=>{pub struct LocalRecords {pub theory:crate::domain::local_theory::TheoryRecords,pub fields:crate::domain::local_fields::FieldRecords,$(pub $field:Rows<$ty>,)*}impl LocalRecords {pub fn new(budget:&resources::ResourceBudget)->Self {Self {theory:crate::domain::local_theory::TheoryRecords::new(budget),fields:crate::domain::local_fields::FieldRecords::new(budget),$($field:Rows::new(budget),)*}}}};}
crate::local_semantic_outputs!(output);
/// Every actual native value support gets a result. This version intentionally remains Partial:
/// reassigned origins, captured cells, attribute state and crossed calls need later operators.
pub fn produce(
    data: &LocalData,
    invocation: &publication::AnalysisInvocation,
    definition: &analysis::AnalysisDefinition,
    budget: &resources::ResourceBudget,
) -> Result<LocalRecords, ModelError> {
    check_definition(definition)?;
    check_invocation(invocation)?;
    if definition.id() != invocation.definition {
        return Err(invalid("Local producer changes definition"));
    }
    let mut records = LocalRecords::new(budget);
    for support in data.entry.value_supports.iter() {
        let Some(a) = support.attribution() else {
            continue;
        };
        let run = need(&data.entry.runs, a.run).map_err(|_| invalid("Local source run absent"))?;
        if (run.input, run.context) != (invocation.input, invocation.context) {
            continue;
        }
        let emission =
            LocalContribution::derive(data, invocation, support.assertion, support.id(), budget)?;
        let assessment = match emission {
            Err(reason) => LocalAssessment {
                invocation: invocation.id(),
                value: support.assertion,
                support: support.id(),
                contribution: None,
                reason: Some(reason),
            },
            Ok(emission) => {
                let source = publication::SupportSource::LocalWitness {
                    witness: emission.contribution.id(),
                };
                let key = emission.branch.key();
                let q = emission.branch.qualification();
                let subject = publication::ObligationSubject::Transfer { transfer: key.id() };
                let evidence = publication::support::EvidencePremise::derived(
                    &source,
                    &emission.contribution,
                    q,
                    emission.branch.condition(),
                )?;
                let (derivation, proposition, members, qualified) =
                    publication::AnalysisDerivation::emit(
                        invocation,
                        definition,
                        subject.id(),
                        analysis::AnalysisChannel::Value,
                        crate::domain::calls::CallPhase::Call,
                        analysis::support::QualificationOperation::Conjunction,
                        &[evidence],
                        budget,
                    )?;
                let generated = publication::SupportSource::AnalysisDerivation {
                    derivation: derivation.id(),
                };
                let alternative = emission.branch.alternative();
                records.entries.insert(emission.entry.witness().clone())?;
                records
                    .entry_sources
                    .insert(emission.entry.source().clone())?;
                records.roots.insert(emission.entry.root().clone())?;
                records.places.insert(emission.entry.place().clone())?;
                records.roots.insert(emission.output_root.clone())?;
                records.places.insert(emission.output_place.clone())?;
                let (condition, nodes) = qualified.condition.records();
                records.conditions.insert(condition)?;
                for node in nodes {
                    records.nodes.insert(node)?;
                }
                records.qualifications.insert(qualified.qualification)?;
                records.keys.insert(key.clone())?;
                records.alternatives.insert(alternative.clone())?;
                records.supports.insert(TransferSupport {
                    assertion: alternative.id(),
                    source: generated.id(),
                })?;
                records.subjects.insert(subject)?;
                records.sources.insert(source)?;
                records.sources.insert(generated)?;
                records.propositions.insert(proposition)?;
                records.derivations.insert(derivation)?;
                for member in members {
                    records.premises.insert(member)?;
                }
                let assessment = LocalAssessment {
                    invocation: invocation.id(),
                    value: support.assertion,
                    support: support.id(),
                    contribution: Some(emission.contribution.id()),
                    reason: None,
                };
                records.contributions.insert(emission.contribution)?;
                assessment
            }
        };
        records.assessments.insert(assessment)?;
    }
    records.theory = crate::domain::local_theory::produce(
        &crate::domain::local_theory::TheoryData {
            entry: &data.entry,
            inventory: &data.theory,
        },
        invocation,
        budget,
    )?;
    emit_theory(data, invocation, definition, &mut records, budget)?;
    produce_entry_reads(data, invocation, &mut records, budget)?;
    produce_fields(data, invocation, &mut records, budget)?;
    produce_guards(data, invocation, definition, &mut records, budget)?;
    for influence in records.influences.iter() {
        for alternative in records.alternatives.iter() {
            let q = records
                .qualifications
                .get(alternative.qualification)
                .or_else(|| data.entry.qualifications.get(alternative.qualification))
                .ok_or_else(|| invalid("Local transfer qualification absent"))?;
            let iq = records
                .qualifications
                .get(influence.qualification)
                .or_else(|| data.entry.qualifications.get(influence.qualification))
                .ok_or_else(|| invalid("Local control qualification absent"))?;
            let key = records.keys.get(alternative.transfer).unwrap();
            let condition = records
                .conditions
                .get(q.condition)
                .or_else(|| data.entry.conditions.get(q.condition))
                .ok_or_else(|| invalid("Local transfer condition absent"))?;
            let _cursor = budget.reserve(
                "local_selection_work",
                data.entry
                    .condition_nodes
                    .len()
                    .checked_mul(64)
                    .ok_or_else(|| invalid("Local selection buffer overflow"))?,
            )?;
            let mut cursor = vec![condition.root];
            let mut visited = charged::ChargedSet::default();
            let mut visit_charge = charged::StateCharge::new(budget, "local_selection_nodes");
            let mut work = 0;
            let mut mentions = false;
            while let Some(id) = cursor.pop() {
                if !visited.insert(&mut visit_charge, id)? {
                    continue;
                }
                work += 1;
                if work > 100_000 {
                    return Err(invalid("Local selection traversal exhausted"));
                }
                if let Some(ConditionNode::Branch { atom, low, high }) = records
                    .nodes
                    .get(id)
                    .or_else(|| data.entry.condition_nodes.get(id))
                {
                    mentions |= *atom == influence.atom;
                    cursor.push(*low);
                    cursor.push(*high);
                }
            }
            if mentions && q.context == iq.context && q.scope == iq.scope {
                records.selections.insert(Selection {
                    influence: influence.id(),
                    atom: influence.atom,
                    alternative: alternative.id(),
                    transfer: key.id(),
                })?;
            }
        }
    }
    Ok(records)
}
/// The Local group writes vocabulary only through its registered private epoch delta.
pub fn stage(
    profile: stages::Profile,
    definition: &analysis::AnalysisDefinition,
    model: &ValidatedModel,
) -> stages::Stage {
    use stages::*;
    let mut inputs = if profile == Profile::Behavioral {
        LocalData::validation_inputs()
            .into_iter()
            .map(|input| {
                let relation = model
                    .relations()
                    .iter()
                    .find(|r| r.name() == input.name())
                    .expect("declared Local input");
                let use_ = RelationUse::of_relation(relation).completed_store();
                if let Some(prefix) = input.prefix() {
                    use_.at_epoch(prefix)
                } else {
                    use_
                }
            })
            .collect::<Vec<_>>()
    } else {
        vec![
            RelationUse::stored::<ProviderRun>(),
            RelationUse::stored::<Provider>(),
            RelationUse::stored::<NativeQualification>(),
        ]
    };
    inputs.extend([
        RelationUse::stored::<crate::domain::input::InputRevision>(),
        RelationUse::stored::<crate::domain::input::ArtifactUse>(),
        RelationUse::stored::<SourceArtifact>(),
        RelationUse::stored::<CoverageScope>(),
        RelationUse::stored::<ProviderCoverage>(),
        RelationUse::stored::<crate::domain::normalized::coverage::NormalizationComputation>(),
        RelationUse::stored::<crate::domain::normalized::coverage::NormalizationCoverage>(),
        RelationUse::stored::<analysis::AnalysisDefinition>(),
        RelationUse::stored::<crate::domain::normalized::dispatch::DispatchAssessment>(),
        RelationUse::stored::<crate::domain::normalized::dispatch::DispatchMember>(),
    ]);
    let mut outputs = publication::relations()
        .iter()
        .map(RelationUse::of_relation)
        .collect::<Vec<_>>();
    macro_rules! outputs {($($field:ident:$ty:ty,)*)=>{$(outputs.push(RelationUse::of::<$ty>());)*};}
    crate::local_semantic_outputs!(outputs);
    crate::local_theory_outputs!(outputs);
    crate::local_field_outputs!(outputs);
    outputs.sort_by_key(|r| r.name());
    outputs.dedup_by_key(|r| r.name());
    let own = outputs
        .iter()
        .filter(|r| !is_vocabulary(r.name()))
        .map(|r| r.name())
        .collect::<std::collections::BTreeSet<_>>();
    let facts = crate::domain::facts_relations()
        .iter()
        .map(Relation::name)
        .collect::<std::collections::BTreeSet<_>>();
    // Nominal references and shared replay inventories are independently admitted predecessors.
    // Facts-only prerequisites are supplied by the confirmed checkpoint; no later owner is read.
    let mut pending = inputs.iter().map(|r| r.name()).collect::<Vec<_>>();
    while let Some(name) = pending.pop() {
        let relation = model
            .relations()
            .iter()
            .find(|r| r.name() == name)
            .expect("Local predecessor declared");
        let references = relation
            .fields()
            .iter()
            .filter_map(|field| field.target().map(|(_, name)| name));
        let checks = relation
            .invariants()
            .iter()
            .flat_map(|check| check.inputs.iter().map(ValidationInput::name));
        for required in references.chain(checks) {
            if own.contains(required) {
                panic!("Local predecessor depends on its own output: {required}");
            }
            if !facts.contains(required) && !inputs.iter().any(|r| r.name() == required) {
                let relation = model
                    .relations()
                    .iter()
                    .find(|r| r.name() == required)
                    .expect("Local predecessor target declared");
                inputs.push(RelationUse::of_relation(relation).completed_store());
                pending.push(required);
            }
        }
    }
    let inputs = crate::domain::normalized::facts_stage_inputs(inputs);
    Stage {
        name: "analyze_local",
        inputs,
        outputs,
        contributes: vec![],
        coverage: vec![],
        provider: None,
        profiles: vec![profile],
        effect: Effect::Pure,
        code: ContentHash::of(include_bytes!("local_semantics.rs")),
        configuration: {
            let mut sink = KeySink::new("Local definition");
            definition.id().encode(&mut sink);
            sink.finish()
        },
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(name = "local_guard_contributions", rule = "local_entry_guard")]
pub struct LocalGuardContribution {
    #[model(key)]
    pub invocation: Id<publication::AnalysisInvocation>,
    #[model(key, premise)]
    pub leaf: Id<FlowTestLeafObservation>,
    #[model(key, premise)]
    pub support: Id<FlowTestLeafSupport>,
    #[model(key, premise)]
    pub entry: Id<EntryValueWitness>,
    #[model(key, premise)]
    pub stability: Id<StabilityWitness>,
    #[model(key)]
    pub influence: Id<ControlInfluence>,
    #[model(key)]
    pub qualification: Id<AssertionQualification>,
    status: EvidenceStatus,
}
#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(name = "local_guard_assessments")]
pub struct LocalGuardAssessment {
    #[model(key)]
    pub invocation: Id<publication::AnalysisInvocation>,
    #[model(key)]
    pub leaf: Id<FlowTestLeafObservation>,
    #[model(key)]
    pub support: Id<FlowTestLeafSupport>,
    pub contribution: Option<Id<LocalGuardContribution>>,
    pub reason: Option<ObligationKind>,
}
impl analysis::support::sealed::DerivedEvidence for LocalGuardContribution {}
impl analysis::support::DerivedEvidence for LocalGuardContribution {
    fn source_facts(&self) -> SourceFacts {
        SourceFacts {
            qualification: self.qualification,
            status: self.status,
            heuristic: false,
        }
    }
}
pub struct LocalGuardEmission {
    proof: LocalGuardContribution,
    entry: DerivedEntryValue,
    stability: CheckedStability,
    influence: ControlInfluence,
    qualification: AssertionQualification,
    _charge: Box<dyn resources::Reservation>,
    condition: Diagram,
}
impl LocalGuardEmission {
    pub fn entry(&self) -> &DerivedEntryValue {
        &self.entry
    }
    pub fn stability(&self) -> &CheckedStability {
        &self.stability
    }
    pub fn influence(&self) -> &ControlInfluence {
        &self.influence
    }
    pub fn contribution(&self) -> &LocalGuardContribution {
        &self.proof
    }
}
impl LocalGuardContribution {
    pub fn derive(
        data: &LocalData,
        invocation: &publication::AnalysisInvocation,
        leaf: Id<FlowTestLeafObservation>,
        support: Id<FlowTestLeafSupport>,
        budget: &resources::ResourceBudget,
    ) -> Result<Result<LocalGuardEmission, ObligationKind>, ModelError> {
        check_invocation(invocation)?;
        let setup = (|| {
            let leaf = need(&data.entry.leaves, leaf)?;
            let support = need(&data.entry.leaf_supports, support)?;
            if support.assertion != leaf.id() {
                return Err(ObligationKind::MissingEvidence);
            }
            let a = support
                .attribution()
                .ok_or(ObligationKind::MissingEvidence)?;
            let run = need(&data.entry.runs, a.run)?;
            let q = need(&data.entry.qualifications, leaf.qualification)?;
            if (run.input, run.context) != (invocation.input, invocation.context)
                || q.context != invocation.context
            {
                return Err(ObligationKind::IncompatibleContexts);
            }
            let access = leaf.operand.ok_or(ObligationKind::MissingEvidence)?;
            let operand = need(&data.entry.occurrences, access)?;
            let mut uses = data.entry.uses.iter().filter(|row| {
                data.entry
                    .occurrences
                    .get(row.occurrence)
                    .is_some_and(|read| {
                        read.source == operand.source
                            && read.structural_path == operand.structural_path
                            && read.syntax_kind == operand.syntax_kind
                            && read.start == operand.start
                            && read.end == operand.end
                            && read.role == OccurrenceRole::Read
                    })
            });
            let use_ = uses.next().ok_or(ObligationKind::MissingEvidence)?;
            if uses.next().is_some() {
                return Err(ObligationKind::MissingEvidence);
            }
            let place = need(&data.entry.places, use_.place)?;
            let PlaceRoot::Formal { declaration } = need(&data.entry.roots, place.root)? else {
                return Err(ObligationKind::EntryValueUnknown);
            };
            let owner = data
                .entry
                .owners
                .iter()
                .find(|row| row.occurrence == use_.occurrence)
                .ok_or(ObligationKind::MissingEvidence)?;
            let formal = ParameterEntity::Source {
                declaration: *declaration,
            };
            Ok((
                EntryRequest {
                    owner: owner.entity,
                    formal: formal.id(),
                    access: use_.occurrence,
                    context: run.context,
                    run: run.id(),
                },
                leaf,
                support,
                q,
            ))
        })();
        let (request, leaf, support, q) = match setup {
            Ok(rows) => rows,
            Err(reason) => return Ok(Err(reason)),
        };
        let source = match EntryAccessSource::guard(&data.entry, request, leaf.id(), support.id()) {
            Ok(source) => source,
            Err(reason) => return Ok(Err(reason)),
        };
        let entry = match EntryValueWitness::derive_for(&data.entry, request, &source, budget)? {
            Ok(proof) => proof,
            Err(reason) => return Ok(Err(reason)),
        };
        let stability = match StabilityWitness::derive(&data.entry, leaf.atom, &entry) {
            Ok(proof) => proof,
            Err(reason) => return Ok(Err(reason)),
        };
        let premise = NativeAssertionPremise::Leaf {
            assertion: leaf.id(),
            support: support.id(),
        };
        let Some(native) = data.native.iter().find(|row| {
            row.premise == premise.id()
                && row.qualification == q.id()
                && row.family == FactFamily::Flow
                && row.fidelity == Fidelity::NativeStructural
        }) else {
            return Ok(Err(ObligationKind::MissingEvidence));
        };
        let EntryAccessSource::Guard {
            region,
            region_support,
            ..
        } = entry.source()
        else {
            unreachable!()
        };
        let region_row =
            need(&data.entry.regions, *region).map_err(|_| invalid("Local guard region absent"))?;
        let premise = NativeAssertionPremise::Region {
            assertion: *region,
            support: *region_support,
        };
        let Some(region_native) = data.native.iter().find(|row| {
            row.premise == premise.id()
                && row.qualification == region_row.qualification
                && row.family == FactFamily::Flow
                && row.fidelity == Fidelity::NativeStructural
        }) else {
            return Ok(Err(ObligationKind::MissingEvidence));
        };
        if region_native.status != native.status {
            return Ok(Err(ObligationKind::MissingEvidence));
        }
        let decode = budget.reserve(
            "local_guard_condition_decode",
            data.entry
                .condition_nodes
                .len()
                .checked_mul(2048)
                .ok_or_else(|| invalid("Local guard decode overflow"))?,
        )?;
        let nodes = data
            .entry
            .condition_nodes
            .iter()
            .cloned()
            .collect::<Vec<_>>();
        let leaf_condition = Diagram::from_records(
            need(&data.entry.conditions, q.condition)
                .map_err(|_| invalid("Local leaf condition absent"))?,
            &nodes,
        )?;
        let admitted = leaf_condition
            .admitted_binary(entry.condition(), BooleanOperation::Conjunction, budget)
            .map_err(|e| match e {
                DiagramAdmissionError::Resource(error) => error,
                DiagramAdmissionError::Boundary(reason) => {
                    invalid(&format!("Local guard condition refused: {reason:?}"))
                }
            })?;
        let (condition, charge) = admitted.into_parts();
        drop(decode);
        let q = AssertionQualification {
            condition: condition.id(),
            ..q.clone()
        };
        let atom =
            need(&data.entry.atoms, leaf.atom).map_err(|_| invalid("Local guard atom absent"))?;
        let influence = ControlInfluence {
            qualification: q.id(),
            input: entry.place().id(),
            atom: leaf.atom,
            evaluation: atom.evaluation,
        };
        let proof = Self {
            invocation: invocation.id(),
            leaf: leaf.id(),
            support: support.id(),
            entry: entry.witness().id(),
            stability: stability.witness().id(),
            influence: influence.id(),
            qualification: q.id(),
            status: analysis::policy::derive_status(&[
                (analysis::policy::SupportRole::Support, native.status),
                (analysis::policy::SupportRole::Support, region_native.status),
                (
                    analysis::policy::SupportRole::Support,
                    entry.evidence_status(),
                ),
            ]),
        };
        Ok(Ok(LocalGuardEmission {
            proof,
            entry,
            stability,
            influence,
            qualification: q,
            _charge: charge,
            condition,
        }))
    }
}
fn emit_theory(
    data: &LocalData,
    invocation: &publication::AnalysisInvocation,
    definition: &analysis::AnalysisDefinition,
    records: &mut LocalRecords,
    budget: &resources::ResourceBudget,
) -> Result<(), ModelError> {
    let allowance = data
        .entry
        .condition_nodes
        .len()
        .checked_mul(2048)
        .ok_or_else(|| invalid("Local theory condition allocation overflow"))?;
    let _buffer = budget.reserve("local_theory_condition_decode", allowance)?;
    let nodes = data
        .entry
        .condition_nodes
        .iter()
        .cloned()
        .collect::<Vec<_>>();
    for witness in records.theory.witnesses.iter() {
        let q = data
            .entry
            .qualifications
            .get(witness.qualification)
            .ok_or_else(|| invalid("Local theory qualification absent"))?;
        let condition = Diagram::from_records(
            data.entry
                .conditions
                .get(q.condition)
                .ok_or_else(|| invalid("Local theory condition absent"))?,
            &nodes,
        )?;
        let leaf = data
            .entry
            .leaves
            .get(witness.leaf)
            .ok_or_else(|| invalid("Local theory leaf absent"))?;
        let source = publication::SupportSource::TheoryWitness {
            witness: witness.id(),
        };
        let subject = publication::ObligationSubject::SourceCall {
            occurrence: leaf.test,
        };
        let evidence =
            publication::support::EvidencePremise::derived(&source, witness, q, &condition)?;
        let (derivation, proposition, members, result) = publication::AnalysisDerivation::emit(
            invocation,
            definition,
            subject.id(),
            analysis::AnalysisChannel::Role,
            crate::domain::calls::CallPhase::Call,
            analysis::support::QualificationOperation::Conjunction,
            &[evidence],
            budget,
        )?;
        let generated = publication::SupportSource::AnalysisDerivation {
            derivation: derivation.id(),
        };
        records.sources.insert(source)?;
        records.sources.insert(generated)?;
        records.derivations.insert(derivation)?;
        records.propositions.insert(proposition)?;
        records.subjects.insert(subject)?;
        for member in members {
            records.premises.insert(member)?;
        }
        let (condition, nodes) = result.condition.records();
        records.conditions.insert(condition)?;
        for row in nodes {
            records.nodes.insert(row)?;
        }
        records.qualifications.insert(result.qualification)?;
    }
    Ok(())
}
/// Successful exact native Use proofs are the separate read inventory consumed by B1.
/// Value/Guard proofs never substitute for a name-read certificate.
fn produce_entry_reads(
    data: &LocalData,
    invocation: &publication::AnalysisInvocation,
    rows: &mut LocalRecords,
    budget: &resources::ResourceBudget,
) -> Result<(), ModelError> {
    for support in data.entry.use_supports.iter() {
        let Some(attribution) = support.attribution() else {
            continue;
        };
        let run = need(&data.entry.runs, attribution.run)
            .map_err(|_| invalid("Local read run absent"))?;
        if (run.input, run.context) != (invocation.input, invocation.context) {
            continue;
        }
        let Some(observation) = data.entry.use_observations.get(support.assertion) else {
            return Err(invalid("Local read observation absent"));
        };
        let Some(use_) = data.entry.uses.get(observation.use_) else {
            return Err(invalid("Local read identity absent"));
        };
        let Some(place) = data.entry.places.get(use_.place) else {
            return Err(invalid("Local read place absent"));
        };
        let Some(PlaceRoot::Formal { declaration }) = data.entry.roots.get(place.root) else {
            continue;
        };
        let Some(owner) = data
            .entry
            .owners
            .iter()
            .find(|owner| owner.occurrence == use_.occurrence)
        else {
            continue;
        };
        let formal = ParameterEntity::Source {
            declaration: *declaration,
        };
        let request = EntryRequest {
            owner: owner.entity,
            formal: formal.id(),
            access: use_.occurrence,
            context: run.context,
            run: run.id(),
        };
        if let Ok(entry) = EntryValueWitness::derive(&data.entry, request, budget)? {
            rows.entry_sources.insert(entry.source().clone())?;
            rows.entries.insert(entry.witness().clone())?;
            rows.roots.insert(entry.root().clone())?;
            rows.places.insert(entry.place().clone())?;
        }
    }
    Ok(())
}
fn produce_guards(
    data: &LocalData,
    invocation: &publication::AnalysisInvocation,
    definition: &analysis::AnalysisDefinition,
    rows: &mut LocalRecords,
    budget: &resources::ResourceBudget,
) -> Result<(), ModelError> {
    for support in data.entry.leaf_supports.iter() {
        let Some(a) = support.attribution() else {
            continue;
        };
        let run = need(&data.entry.runs, a.run).map_err(|_| invalid("Local guard run absent"))?;
        if (run.input, run.context) != (invocation.input, invocation.context) {
            continue;
        }
        let emission = LocalGuardContribution::derive(
            data,
            invocation,
            support.assertion,
            support.id(),
            budget,
        )?;
        let assessment = match emission {
            Err(reason) => LocalGuardAssessment {
                invocation: invocation.id(),
                leaf: support.assertion,
                support: support.id(),
                contribution: None,
                reason: Some(reason),
            },
            Ok(emission) => {
                let source = publication::SupportSource::LocalGuard {
                    witness: emission.proof.id(),
                };
                let q = &emission.qualification;
                let subject = publication::ObligationSubject::SourceCall {
                    occurrence: emission.influence.evaluation,
                };
                let evidence = publication::support::EvidencePremise::derived(
                    &source,
                    &emission.proof,
                    q,
                    &emission.condition,
                )?;
                let (derivation, proposition, members, result) =
                    publication::AnalysisDerivation::emit(
                        invocation,
                        definition,
                        subject.id(),
                        analysis::AnalysisChannel::Role,
                        crate::domain::calls::CallPhase::Call,
                        analysis::support::QualificationOperation::Conjunction,
                        &[evidence],
                        budget,
                    )?;
                let generated = publication::SupportSource::AnalysisDerivation {
                    derivation: derivation.id(),
                };
                rows.control_supports.insert(ControlSupport {
                    assertion: emission.influence.id(),
                    source: generated.id(),
                })?;
                rows.sources.insert(source)?;
                rows.sources.insert(generated)?;
                rows.derivations.insert(derivation)?;
                rows.propositions.insert(proposition)?;
                rows.subjects.insert(subject)?;
                for row in members {
                    rows.premises.insert(row)?;
                }
                let (condition, nodes) = result.condition.records();
                rows.conditions.insert(condition)?;
                for node in nodes {
                    rows.nodes.insert(node)?;
                }
                rows.qualifications.insert(result.qualification)?;
                rows.roots.insert(emission.entry.root().clone())?;
                rows.places.insert(emission.entry.place().clone())?;
                rows.entries.insert(emission.entry.witness().clone())?;
                rows.entry_sources.insert(emission.entry.source().clone())?;
                rows.stability
                    .insert(emission.stability.witness().clone())?;
                rows.influences.insert(emission.influence)?;
                let assessment = LocalGuardAssessment {
                    invocation: invocation.id(),
                    leaf: emission.proof.leaf,
                    support: emission.proof.support,
                    contribution: Some(emission.proof.id()),
                    reason: None,
                };
                rows.guards.insert(emission.proof)?;
                assessment
            }
        };
        rows.guard_assessments.insert(assessment)?;
    }
    Ok(())
}

fn produce_fields(
    data: &LocalData,
    invocation: &publication::AnalysisInvocation,
    records: &mut LocalRecords,
    budget: &resources::ResourceBudget,
) -> Result<(), ModelError> {
    use crate::domain::local_fields::*;
    let input = FieldData {
        entry: &data.entry,
        theory: &data.theory,
        inventory: &data.fields,
    };
    for native in data.fields.load_supports.iter() {
        if !data
            .entry
            .runs
            .get(native.run)
            .is_some_and(|r| (r.input, r.context) == (invocation.input, invocation.context))
        {
            continue;
        }
        let assessment =
            match FieldLocation::derive(&input, invocation, native.assertion, native.id(), budget)?
            {
                Ok(proof) => {
                    let location = proof.location().clone();
                    let reason = proof.reason();
                    for row in proof.candidates() {
                        records.fields.candidates.insert(row.clone())?;
                    }
                    records.entries.insert(proof.entry().witness().clone())?;
                    records
                        .entry_sources
                        .insert(proof.entry().source().clone())?;
                    records.roots.insert(proof.entry().root().clone())?;
                    records.places.insert(proof.entry().place().clone())?;
                    records
                        .qualifications
                        .insert(proof.qualification().clone())?;
                    let (condition, nodes) = proof.condition().records();
                    records.conditions.insert(condition)?;
                    for node in nodes {
                        records.nodes.insert(node)?;
                    }
                    records.fields.locations.insert(location.clone())?;
                    FieldLocationAssessment {
                        invocation: invocation.id(),
                        load: native.assertion,
                        support: native.id(),
                        location: Some(location.id()),
                        reason,
                    }
                }
                Err(reason) => FieldLocationAssessment {
                    invocation: invocation.id(),
                    load: native.assertion,
                    support: native.id(),
                    location: None,
                    reason,
                },
            };
        records.fields.assessments.insert(assessment)?;
    }
    Ok(())
}
