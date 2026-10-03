//! Publication repeats the finite worklist from immutable predecessors and stored topology.
use super::summary_production::*;
use crate::domain::{
    analysis::{self, summary as owner},
    normalized::Rows,
    resources::ResourceBudget,
    stages::*,
    *,
};
fn invalid(s: impl Into<String>) -> ModelError {
    ModelError::Invalid(s.into())
}
pub fn inputs() -> Vec<ValidationInput> {
    let mut rows = SummaryData::inputs();
    rows.extend([
        ValidationInput::of::<owner::AnalysisInvocation>(&["id"]),
        ValidationInput::of::<owner::AnalysisOutcome>(&["id"]),
        ValidationInput::of::<owner::AnalysisCoverage>(&["id"]),
    ]);
    macro_rules! append{($($f:ident:$t:ty,)*)=>{$(rows.push(ValidationInput::of::<$t>(&["id"]));)*};}
    crate::summary_outputs!(append);
    crate::summary_vocabulary!(append);
    rows.sort_by_key(|i| (i.name(), i.prefix()));
    rows.dedup_by_key(|i| (i.name(), i.prefix()));
    rows
}
/// Catalog declares the native frame, completed parents and stored topology, without requesting Flow.
pub fn production_inputs(profile: Profile) -> Vec<ValidationInput> {
    if profile == Profile::Behavioral {
        return SummaryData::inputs();
    }
    macro_rules! rows{($($t:ty),*)=>{vec![$(ValidationInput::of::<$t>(&["id"])),*]};}
    rows!(
        attribution::ProviderRun,
        analysis::MethodParameters,
        analysis::AnalysisDefinition,
        analysis::local::AnalysisInvocation,
        analysis::model::AnalysisInvocation,
        analysis::enriched_execution::AnalysisInvocation,
        analysis::source_call::AnalysisInvocation,
        analysis::local::AnalysisOutcome,
        analysis::model::AnalysisOutcome,
        analysis::enriched_execution::AnalysisOutcome,
        analysis::source_call::AnalysisOutcome,
        projection::ProjectionSourceAssessment,
        projection::ProjectionSnapshot,
        projection::ProjectionSnapshotChunk
    )
}
/// A closed-store replay may omit unrequested native inputs; Catalog run checks never consume them.
pub fn inputs_for_profile(profile: Profile) -> Vec<ValidationInput> {
    if profile == Profile::Behavioral {
        return inputs();
    }
    let mut rows = production_inputs(profile);
    rows.extend([
        ValidationInput::of::<owner::AnalysisInvocation>(&["id"]),
        ValidationInput::of::<owner::AnalysisOutcome>(&["id"]),
        ValidationInput::of::<owner::AnalysisCoverage>(&["id"]),
    ]);
    macro_rules! append{($($f:ident:$t:ty,)*)=>{$(rows.push(ValidationInput::of::<$t>(&["id"]));)*};}
    crate::summary_outputs!(append);
    crate::summary_vocabulary!(append);
    rows.sort_by_key(|i| (i.name(), i.prefix()));
    rows.dedup_by_key(|i| (i.name(), i.prefix()));
    rows
}
pub fn invariants() -> Vec<Invariant> {
    vec![Invariant {
        name: "finite_summary_inventory_replay",
        inputs: inputs(),
        create: std::sync::Arc::new(|b| Box::new(Check::new(b))),
    }]
}
pub fn parents(
    data: &SummaryData,
    input: Id<input::InputRevision>,
    context: Id<attribution::AnalysisContext>,
    budget: &ResourceBudget,
) -> Result<Rows<owner::InvocationSource>, ModelError> {
    let mut rows = Rows::new(budget);
    macro_rules! parent {
        ($field:ident,$variant:ident) => {
            let mut matching = data
                .$field
                .iter()
                .filter(|p| (p.input, p.context) == (input, context) && p.subject.is_none());
            let p = matching.next().ok_or_else(|| {
                invalid(concat!("Summary predecessor absent: ", stringify!($field)))
            })?;
            if matching.next().is_some() {
                return Err(invalid("Summary predecessor frame ambiguous"));
            }
            rows.insert(owner::InvocationSource::$variant { invocation: p.id() })?;
        };
    }
    parent!(local_invocations, Local);
    parent!(model_invocations, Model);
    parent!(enriched_invocations, EnrichedExecution);
    parent!(source_invocations, SourceCallAnalysis);
    Ok(rows)
}
macro_rules! check{($($field:ident:$ty:ty,)*)=>{
 struct Check{data:SummaryData,invocations:Rows<owner::AnalysisInvocation>,outcomes:Rows<owner::AnalysisOutcome>,coverage:Rows<owner::AnalysisCoverage>,vocabulary:Vocabulary,$($field:Rows<$ty>,)*budget:ResourceBudget}
 impl Check{fn new(b:&ResourceBudget)->Self{Self{data:SummaryData::new(b),invocations:Rows::new(b),outcomes:Rows::new(b),coverage:Rows::new(b),vocabulary:Vocabulary::new(b),$($field:Rows::new(b),)*budget:b.clone()}}}
 impl InvariantCheck for Check{
  fn visit_input(&mut self,i:&ValidationInput,b:&arrow_array::RecordBatch)->Result<(),ModelError>{if is_vocabulary(i.name()){match i.prefix(){Some(PublicationBoundary::Facts|PublicationBoundary::Model)=>self.data.visit_input(i,b),None=>self.vocabulary.visit(i.name(),b),_=>Err(invalid("Summary replay changes vocabulary prefix"))}}else{self.visit(i.name(),b)}}
  fn visit(&mut self,n:&str,b:&arrow_array::RecordBatch)->Result<(),ModelError>{if is_vocabulary(n){return Err(invalid("Summary vocabulary needs explicit prefix"));}self.data.visit(n,b)?;$(if n==<$ty>::NAME{self.$field.decode(b)?;})*if n==owner::AnalysisInvocation::NAME{self.invocations.decode(b)?;}if n==owner::AnalysisOutcome::NAME{self.outcomes.decode(b)?;}if n==owner::AnalysisCoverage::NAME{self.coverage.decode(b)?;}Ok(())}
  fn finish(self:Box<Self>)->Result<(),ModelError>{
   let mut frames=charged::ChargedSet::default();let mut charge=charged::StateCharge::new(&self.budget,"summary-replay-frames");for run in self.data.entry.runs.iter(){frames.insert(&mut charge,(run.input,run.context))?;}
   if frames.len()!=self.invocations.len()||frames.iter().any(|f|self.invocations.iter().filter(|i|(i.input,i.context)==*f&&i.subject.is_none()).count()!=1){return Err(invalid("Summary omitted or duplicated an independently captured frame"));}
   $(let mut $field=Rows::<$ty>::new(&self.budget);)*let mut outcomes=Rows::new(&self.budget);
   for invocation in self.invocations.iter(){
    let parents=parents(&self.data,invocation.input,invocation.context,&self.budget)?;let mut digest=KeySink::new("analysis-invocation-inputs");for p in parents.iter(){p.id().encode(&mut digest);}if digest.finish()!=invocation.inputs{return Err(invalid("Summary predecessor inventory changed"));}
    let definition=self.data.definitions.get(invocation.definition).ok_or_else(||invalid("Summary definition absent"))?;let run=self.runs.iter().find(|r|r.invocation==invocation.id()).ok_or_else(||invalid("Summary run absent"))?;
    let mut assessments=self.data.graphs.assessments.iter().filter(|a|(a.input,a.context,a.projection)==(invocation.input,invocation.context,projection::ProjectionName::CallableInvocation));let assessment=assessments.next().ok_or_else(||invalid("Summary stored invocation graph absent"))?;if assessments.next().is_some(){return Err(invalid("Summary stored invocation graph ambiguous"));}
    let mut headers=self.data.graphs.snapshots.iter().filter(|s|s.assessment==assessment.id());let header=headers.next().ok_or_else(||invalid("Summary snapshot absent"))?;if headers.next().is_some(){return Err(invalid("Summary snapshot ambiguous"));}
    let graph=projection::snapshot::hydrate(header,assessment,&self.data.graphs.chunks,&self.budget)?;
    let mut records=produce(&self.data,invocation,definition,if run.requested{Profile::Behavioral}else{Profile::Catalog},&graph,&self.budget)?;
    records.discharge(&self.coverage)?;
    $(for row in records.$field.iter(){$field.insert(row.clone())?;})*
    contains_vocabulary(&self.vocabulary,&records.vocabulary)?;
    outcomes.insert(records.outcome)?;
   }
   $(if !self.$field.same(&$field){return Err(invalid(concat!("Summary output differs from finite replay: ",stringify!($field))));})*
   if !self.outcomes.same(&outcomes){return Err(invalid("Summary outcome differs from complete inventory"));}Ok(())
  }
 }
};}
crate::summary_outputs!(check);
pub fn profile_checks() -> Vec<PublicationInvariant> {
    vec![PublicationInvariant {
        name: "summary_profile",
        inputs: vec![
            ValidationInput::of::<SummaryRun>(&["id"]),
            ValidationInput::of::<owner::AnalysisInvocation>(&["id"]),
        ],
        create: std::sync::Arc::new(|b| {
            Box::new(ProfileCheck {
                runs: Rows::new(b),
                invocations: Rows::new(b),
            })
        }),
    }]
}
struct ProfileCheck {
    runs: Rows<SummaryRun>,
    invocations: Rows<owner::AnalysisInvocation>,
}
impl PublicationCheck for ProfileCheck {
    fn visit(&mut self, n: &str, b: &arrow_array::RecordBatch) -> Result<(), ModelError> {
        if n == SummaryRun::NAME {
            self.runs.decode(b)?;
        }
        if n == owner::AnalysisInvocation::NAME {
            self.invocations.decode(b)?;
        }
        Ok(())
    }
    fn finish(
        self: Box<Self>,
        _: &[CompletedRelation],
        profile: Profile,
    ) -> Result<(), ModelError> {
        if self.runs.len() != self.invocations.len()
            || self.runs.iter().any(|r| {
                self.invocations.get(r.invocation).is_none()
                    || r.requested != (profile == Profile::Behavioral)
            })
        {
            return Err(invalid("Summary changes actual collection profile"));
        }
        Ok(())
    }
}
pub fn output_relations() -> Vec<Relation> {
    let mut rows = Vec::new();
    macro_rules! output{($($f:ident:$t:ty,)*)=>{$(rows.push(Relation::of::<$t>());)*};}
    crate::summary_outputs!(output);
    rows.sort_by_key(Relation::name);
    rows.dedup_by_key(|r| r.name());
    rows
}

fn contains_vocabulary(actual: &Vocabulary, expected: &Vocabulary) -> Result<(), ModelError> {
    macro_rules! check{($($f:ident:$t:ty,)*)=>{$(for row in expected.$f.values(){if actual.$f.get(&row.id())!=Some(row){return Err(invalid(concat!("Summary vocabulary missing: ",stringify!($f))));}})*};}
    crate::summary_vocabulary!(check);
    Ok(())
}

pub fn stage(
    profile: Profile,
    definition: &analysis::AnalysisDefinition,
    model: &ValidatedModel,
) -> Result<Stage, ModelError> {
    if definition.method != analysis::AnalysisMethod::Summaries {
        return Err(invalid("Summary stage requires its nominal method"));
    }
    let mut outputs = output_relations();
    outputs.extend(owner::coverage::relations());
    outputs.extend(owner::support::relations());
    macro_rules! output{($($t:ty),*)=>{$(outputs.push(Relation::of::<$t>());)*};}
    output!(
        owner::AnalysisInvocation,
        owner::AnalysisInput,
        owner::SourceReceipt,
        owner::ProjectionInput,
        owner::InvocationSource,
        owner::AnalysisOutcome,
        owner::ObligationSource
    );
    macro_rules! vocabulary{($($f:ident:$t:ty,)*)=>{$(outputs.push(Relation::of::<$t>());)*};}
    crate::summary_vocabulary!(vocabulary);
    outputs.sort_by_key(Relation::name);
    outputs.dedup_by_key(|r| r.name());
    let own = outputs
        .iter()
        .filter(|r| !is_vocabulary(r.name()))
        .map(Relation::name)
        .collect::<std::collections::BTreeSet<_>>();
    let mut requested = production_inputs(profile);
    requested.extend(analysis::expected::inputs(
        analysis::AnalysisMethod::Summaries,
    ));
    requested.push(ValidationInput::of::<analysis::ProjectionDefinition>(&[
        "id",
    ]));
    let relation = |name| {
        model
            .relations()
            .iter()
            .find(|r| r.name() == name)
            .ok_or_else(|| invalid(format!("Summary relation missing: {name}")))
    };
    let mut inputs = std::collections::BTreeMap::new();
    let mut pending = Vec::new();
    for input in requested {
        if own.contains(input.name()) {
            continue;
        }
        if inputs.contains_key(input.name()) {
            continue;
        }
        inputs.insert(
            input.name(),
            RelationUse::of_relation(relation(input.name())?).completed_store(),
        );
        pending.push(input.name());
    }
    let facts = facts_relations()
        .iter()
        .map(Relation::name)
        .collect::<std::collections::BTreeSet<_>>();
    while let Some(name) = pending.pop() {
        let row = relation(name)?;
        for required in row
            .fields()
            .iter()
            .filter_map(|f| f.target().map(|(_, n)| n))
            .chain(
                row.invariants()
                    .iter()
                    .flat_map(|i| i.inputs.iter().map(ValidationInput::name)),
            )
        {
            if own.contains(required) {
                return Err(invalid(format!(
                    "Summary predecessor {name} reads unfinished {required}"
                )));
            }
            if !facts.contains(required) && !inputs.contains_key(required) {
                inputs.insert(
                    required,
                    RelationUse::of_relation(relation(required)?).completed_store(),
                );
                pending.push(required);
            }
        }
    }
    let mut configuration = KeySink::new("summary-stage");
    definition.id().encode(&mut configuration);
    let inputs = inputs
        .into_values()
        .map(|r| {
            if is_vocabulary(r.name()) {
                r.at_epoch(PublicationBoundary::Model)
            } else {
                r
            }
        })
        .collect();
    Ok(Stage {
        name: "analyze_summaries",
        inputs,
        outputs: outputs.iter().map(RelationUse::of_relation).collect(),
        contributes: vec![],
        coverage: vec![],
        profiles: vec![profile],
        effect: Effect::Pure,
        code: ContentHash::of(include_bytes!("summary_replay.rs")),
        configuration: configuration.finish(),
    })
}

pub fn relations() -> Vec<Relation> {
    vec![
        Relation::of::<super::summary_exceptions::SummaryExceptionOutcome>(),
        Relation::of::<SummaryRun>(),
        Relation::of::<SummaryComponent>(),
        Relation::of::<ComponentMember>(),
        Relation::of::<SummaryOrigin>(),
        Relation::of::<ProofOrigin>(),
        Relation::of::<SummaryResidual>(),
        Relation::of::<CallMember>(),
        Relation::of::<OriginBoundary>(),
        Relation::of::<PairOutcome>(),
        Relation::of::<super::summary_control::SummaryControlWitness>(),
        Relation::of::<super::summary_symbolic::SymbolicFieldAlternative>(),
    ]
}
