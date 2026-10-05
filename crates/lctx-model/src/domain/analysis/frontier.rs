//! Final frontier availability checks the complete scheduled envelope, not behavioral completeness.
//! Prior owner assessments stay immutable; optional unrequested methods remain explicit members.
use crate::domain::{
    analysis::{self, AnalysisMethod as Method},
    attribution::{AnalysisContext, ProviderRun},
    input::InputRevision,
    normalized::{Rows, coverage::EvidenceAvailability},
    obligation::ObligationKind,
    resources::ResourceBudget,
    stages::{self, Profile},
    *,
};
use crate::{Domain, DomainSum};

fn invalid(message: &str) -> ModelError {
    ModelError::Invalid(message.into())
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Target {
    Analysis,
    Catalog,
}
#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(name="analysis_frontier_assessments",publication_refs=analysis_checks_refs)]
pub struct AnalysisAssessment {
    #[model(key)]
    pub input: Id<InputRevision>,
    #[model(key)]
    pub context: Id<AnalysisContext>,
    #[model(key)]
    pub capture: ContentHash,
    #[model(key)]
    pub members: ContentHash,
    pub availability: EvidenceAvailability,
    pub reason: Option<ObligationKind>,
}
#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(name="catalog_frontier_assessments",rule="final_catalog_assessment",publication_refs=catalog_checks_refs)]
pub struct CatalogAssessment {
    #[model(key)]
    pub input: Id<InputRevision>,
    #[model(key)]
    pub context: Id<AnalysisContext>,
    #[model(key, premise)]
    pub analysis: Id<AnalysisAssessment>,
    #[model(key)]
    pub capture: ContentHash,
    #[model(key)]
    pub members: ContentHash,
    pub availability: EvidenceAvailability,
    pub reason: Option<ObligationKind>,
}
macro_rules! member_type{($name:ident,$table:literal,$assessment:ty;$($extra:tt)*)=>{
 #[derive(Debug,Clone,PartialEq,Eq,Hash,DomainSum)]# [model(name=$table,rule=$table)]pub enum $name{
 #[model(code=0)]Local{assessment:Id<$assessment>,method:Method,#[model(premise)]invocation:Id<analysis::local::AnalysisInvocation>,#[model(premise)]outcome:Id<analysis::local::AnalysisOutcome>,#[model(premise)]coverage:Id<analysis::local::AnalysisCoverage>,payload:ContentHash},
 #[model(code=1)]BaseEvaluation{assessment:Id<$assessment>,method:Method,#[model(premise)]invocation:Id<analysis::base_evaluation::AnalysisInvocation>,#[model(premise)]outcome:Id<analysis::base_evaluation::AnalysisOutcome>,#[model(premise)]coverage:Id<analysis::base_evaluation::AnalysisCoverage>,payload:ContentHash},
 #[model(code=2)]BaseCompletion{assessment:Id<$assessment>,method:Method,#[model(premise)]invocation:Id<analysis::base_completion::AnalysisInvocation>,#[model(premise)]outcome:Id<analysis::base_completion::AnalysisOutcome>,#[model(premise)]coverage:Id<analysis::base_completion::AnalysisCoverage>,payload:ContentHash},
 #[model(code=3)]SourceCall{assessment:Id<$assessment>,method:Method,#[model(premise)]invocation:Id<analysis::source_call::AnalysisInvocation>,#[model(premise)]outcome:Id<analysis::source_call::AnalysisOutcome>,#[model(premise)]coverage:Id<analysis::source_call::AnalysisCoverage>,payload:ContentHash},
 #[model(code=4)]Enriched{assessment:Id<$assessment>,method:Method,#[model(premise)]invocation:Id<analysis::enriched_execution::AnalysisInvocation>,#[model(premise)]outcome:Id<analysis::enriched_execution::AnalysisOutcome>,#[model(premise)]coverage:Id<analysis::enriched_execution::AnalysisCoverage>,payload:ContentHash},
 #[model(code=5)]Model{assessment:Id<$assessment>,method:Method,#[model(premise)]invocation:Id<analysis::model::AnalysisInvocation>,#[model(premise)]outcome:Id<analysis::model::AnalysisOutcome>,#[model(premise)]coverage:Id<analysis::model::AnalysisCoverage>,payload:ContentHash},
 #[model(code=6)]Summary{assessment:Id<$assessment>,method:Method,#[model(premise)]invocation:Id<analysis::summary::AnalysisInvocation>,#[model(premise)]outcome:Id<analysis::summary::AnalysisOutcome>,#[model(premise)]coverage:Id<analysis::summary::AnalysisCoverage>,payload:ContentHash},
 #[model(code=7)]Structural{assessment:Id<$assessment>,method:Method,#[model(premise)]invocation:Id<analysis::structural::AnalysisInvocation>,#[model(premise)]outcome:Id<analysis::structural::AnalysisOutcome>,#[model(premise)]coverage:Id<analysis::structural::AnalysisCoverage>,payload:ContentHash},
 #[model(code=8)]Embedding{assessment:Id<$assessment>,method:Method,#[model(premise)]invocation:Id<analysis::analytic_embedding::AnalysisInvocation>,#[model(premise)]outcome:Id<analysis::analytic_embedding::AnalysisOutcome>,#[model(premise)]coverage:Id<analysis::analytic_embedding::AnalysisCoverage>,payload:ContentHash},
 #[model(code=9)]Analytic{assessment:Id<$assessment>,method:Method,#[model(premise)]invocation:Id<analysis::analytic::AnalysisInvocation>,#[model(premise)]outcome:Id<analysis::analytic::AnalysisOutcome>,#[model(premise)]coverage:Id<analysis::analytic::AnalysisCoverage>,payload:ContentHash},
 #[model(code=10)]Core{assessment:Id<$assessment>,method:Method,#[model(premise)]invocation:Id<analysis::catalog_core::AnalysisInvocation>,#[model(premise)]outcome:Id<analysis::catalog_core::AnalysisOutcome>,#[model(premise)]coverage:Id<analysis::catalog_core::AnalysisCoverage>,payload:ContentHash},
 #[model(code=11)]Evidence{assessment:Id<$assessment>,method:Method,#[model(premise)]invocation:Id<analysis::catalog_evidence::AnalysisInvocation>,#[model(premise)]outcome:Id<analysis::catalog_evidence::AnalysisOutcome>,#[model(premise)]coverage:Id<analysis::catalog_evidence::AnalysisCoverage>,payload:ContentHash},
 $($extra)*
 }
};}
member_type!(AnalysisMember,"analysis_frontier_members",AnalysisAssessment;);
member_type!(CatalogMember,"catalog_frontier_members",CatalogAssessment;
 #[model(code=12)]Selection{assessment:Id<CatalogAssessment>,method:Method,#[model(premise)]invocation:Id<analysis::selection::AnalysisInvocation>,#[model(premise)]outcome:Id<analysis::selection::AnalysisOutcome>,#[model(premise)]coverage:Id<analysis::selection::AnalysisCoverage>,payload:ContentHash},
 #[model(code=13)]Synthesis{assessment:Id<CatalogAssessment>,method:Method,#[model(premise)]invocation:Id<analysis::synthesis::AnalysisInvocation>,#[model(premise)]outcome:Id<analysis::synthesis::AnalysisOutcome>,#[model(premise)]coverage:Id<analysis::synthesis::AnalysisCoverage>,payload:ContentHash},
 #[model(code=14)]Retrieval{assessment:Id<CatalogAssessment>,method:Method,#[model(premise)]invocation:Id<analysis::retrieval::AnalysisInvocation>,#[model(premise)]outcome:Id<analysis::retrieval::AnalysisOutcome>,#[model(premise)]coverage:Id<analysis::retrieval::AnalysisCoverage>,payload:ContentHash},
);
// Members refer to an assessment whose membership digest is independent of that reference.
// The digest pins all nominal predecessor IDs and full non-key payloads, avoiding a hash cycle.
macro_rules! owners{($apply:ident)=>{$apply!{
 local:Local:local=>[LocalTransfers],evaluation:BaseEvaluation:base_evaluation=>[Execution],completion:BaseCompletion:base_completion=>[Completion],source:SourceCall:source_call=>[SourceCalls],enriched:Enriched:enriched_execution=>[EnrichedExecution],model:Model:model=>[Models],summary:Summary:summary=>[Summaries],structural:Structural:structural=>[Delegation,DirectUsage,Handoffs,Controls],embedding:Embedding:analytic_embedding=>[AnalyticEmbedding],analytic:Analytic:analytic=>[PageRank,Communities,Concepts,RelationalConcepts,Neighbours],core:Core:catalog_core=>[Catalog],evidence:Evidence:catalog_evidence=>[CatalogEvidence],selection:Selection:selection=>[CatalogSelection],synthesis:Synthesis:synthesis=>[Synthesis],retrieval:Retrieval:retrieval=>[Retrieval],
}};}
struct OwnerRows<I: Record, O: Record, C: Record> {
    invocations: Rows<I>,
    outcomes: Rows<O>,
    coverage: Rows<C>,
}
impl<I: Record, O: Record, C: Record> OwnerRows<I, O, C> {
    fn new(b: &ResourceBudget) -> Self {
        Self {
            invocations: Rows::new(b),
            outcomes: Rows::new(b),
            coverage: Rows::new(b),
        }
    }
    fn visit(&mut self, n: &str, b: &arrow_array::RecordBatch) -> Result<(), ModelError> {
        if n == I::NAME {
            self.invocations.decode(b)?;
        }
        if n == O::NAME {
            self.outcomes.decode(b)?;
        }
        if n == C::NAME {
            self.coverage.decode(b)?;
        }
        Ok(())
    }
}
macro_rules! data{($($field:ident:$variant:ident:$owner:ident=>[$($method:ident),*],)*)=>{
 pub struct FrontierData{pub runs:Rows<ProviderRun>,pub inputs:Rows<InputRevision>,pub definitions:Rows<analysis::AnalysisDefinition>,pub analysis:Rows<AnalysisAssessment>,pub analysis_members:Rows<AnalysisMember>,frontier:analysis::expected::FrontierIndex,$($field:OwnerRows<analysis::$owner::AnalysisInvocation,analysis::$owner::AnalysisOutcome,analysis::$owner::AnalysisCoverage>,)*}
 impl FrontierData{pub fn new(profile:Profile,b:&ResourceBudget)->Self{Self{runs:Rows::new(b),inputs:Rows::new(b),definitions:Rows::new(b),analysis:Rows::new(b),analysis_members:Rows::new(b),frontier:analysis::expected::FrontierIndex::new(profile,b),$($field:OwnerRows::new(b),)*}}
 pub fn visit(&mut self,n:&str,b:&arrow_array::RecordBatch)->Result<(),ModelError>{self.frontier.visit(n,b)?;if n==ProviderRun::NAME{self.runs.decode(b)?;}if n==InputRevision::NAME{self.inputs.decode(b)?;}if n==analysis::AnalysisDefinition::NAME{self.definitions.decode(b)?;}if n==AnalysisAssessment::NAME{self.analysis.decode(b)?;}if n==AnalysisMember::NAME{self.analysis_members.decode(b)?;}$ (self.$field.visit(n,b)?;)*Ok(())}
 pub fn inputs(target:Target)->Vec<ValidationInput>{let mut out=vec![ValidationInput::of::<ProviderRun>(&["id"]),ValidationInput::of::<analysis::AnalysisDefinition>(&["id"])];$(if owner_required(target,stringify!($owner)){out.extend([ValidationInput::of::<analysis::$owner::AnalysisInvocation>(&["id"]),ValidationInput::of::<analysis::$owner::AnalysisOutcome>(&["id"]),ValidationInput::of::<analysis::$owner::AnalysisCoverage>(&["id"])]);$(out.extend(analysis::expected::inputs(Method::$method));)*})*if target==Target::Catalog{out.extend([ValidationInput::of::<AnalysisAssessment>(&["id"]),ValidationInput::of::<AnalysisMember>(&["id"])]);}out.sort_by_key(|i|i.name());out.dedup_by_key(|i|i.name());out}
 }};
}
owners!(data);
fn owner_required(target: Target, owner: &str) -> bool {
    target == Target::Catalog || !["selection", "synthesis", "retrieval"].contains(&owner)
}
pub struct FrontierRecords {
    pub analysis: Rows<AnalysisAssessment>,
    pub analysis_members: Rows<AnalysisMember>,
    pub catalog: Rows<CatalogAssessment>,
    pub catalog_members: Rows<CatalogMember>,
}
impl FrontierRecords {
    fn new(b: &ResourceBudget) -> Self {
        Self {
            analysis: Rows::new(b),
            analysis_members: Rows::new(b),
            catalog: Rows::new(b),
            catalog_members: Rows::new(b),
        }
    }
}
fn digest<T: Key>(domain: &str, items: impl IntoIterator<Item = T>) -> ContentHash {
    let mut k = KeySink::new(domain);
    for item in items {
        item.encode(&mut k);
    }
    k.finish()
}
fn payload<I: Record, O: Record, C: Record>(i: &I, o: &O, c: &C) -> ContentHash {
    digest(
        "final-frontier-predecessor-payload",
        [i.content_digest(), o.content_digest(), c.content_digest()],
    )
}
pub fn analysis_relations() -> Vec<Relation> {
    vec![
        Relation::of::<AnalysisAssessment>(),
        Relation::of::<AnalysisMember>(),
    ]
}
pub fn catalog_relations() -> Vec<Relation> {
    vec![
        Relation::of::<CatalogAssessment>(),
        Relation::of::<CatalogMember>(),
    ]
}
macro_rules! lower_analysis {
    (Selection,$($args:expr),*) => {{
        // These Catalog-only typed slots are inspected but cannot lower into Analysis.
        let _ = ($($args),*);
        Err(invalid("Catalog-only member cannot enter Analysis"))
    }};
    (Synthesis,$($args:expr),*) => {{
        // These Catalog-only typed slots are inspected but cannot lower into Analysis.
        let _ = ($($args),*);
        Err(invalid("Catalog-only member cannot enter Analysis"))
    }};
    (Retrieval,$($args:expr),*) => {{
        // These Catalog-only typed slots are inspected but cannot lower into Analysis.
        let _ = ($($args),*);
        Err(invalid("Catalog-only member cannot enter Analysis"))
    }};
    ($variant:ident,$assessment:expr,$method:expr,$invocation:expr,$outcome:expr,$coverage:expr,$payload:expr) => {
        Ok(AnalysisMember::$variant {
            assessment: $assessment,
            method: $method,
            invocation: $invocation,
            outcome: $outcome,
            coverage: $coverage,
            payload: $payload,
        })
    };
}
macro_rules! drafts{($($field:ident:$variant:ident:$owner:ident=>[$($method:ident),*],)*)=>{
 #[derive(Clone)]enum Draft{$($variant{method:Method,invocation:Id<analysis::$owner::AnalysisInvocation>,outcome:Id<analysis::$owner::AnalysisOutcome>,coverage:Id<analysis::$owner::AnalysisCoverage>,payload:ContentHash},)*}
 impl HeapSize for Draft{}
 impl Draft{fn id(&self)->ContentHash{match self{$(Self::$variant{method,invocation,outcome,coverage,payload}=>{let mut k=KeySink::new("final-frontier-member");k.part(b"owner",stringify!($variant).as_bytes());method.encode(&mut k);invocation.encode(&mut k);outcome.encode(&mut k);coverage.encode(&mut k);payload.encode(&mut k);k.finish()},)*}}
 fn analysis(&self,assessment:Id<AnalysisAssessment>)->Result<AnalysisMember,ModelError>{match self{$(Self::$variant{method,invocation,outcome,coverage,payload}=>lower_analysis!($variant,assessment,*method,*invocation,*outcome,*coverage,*payload),)*}}
 fn catalog(&self,assessment:Id<CatalogAssessment>)->CatalogMember{match self{$(Self::$variant{method,invocation,outcome,coverage,payload}=>CatalogMember::$variant{assessment,method:*method,invocation:*invocation,outcome:*outcome,coverage:*coverage,payload:*payload},)*}}}
};}
owners!(drafts);
/// The expected universe comes from captured native frames and the fixed owner/method set.
/// Actual owner coverage is regenerated by its existing shared operation before it is counted.
pub fn derive(
    data: &FrontierData,
    target: Target,
    b: &ResourceBudget,
) -> Result<FrontierRecords, ModelError> {
    let mut result = FrontierRecords::new(b);
    let mut charge = charged::StateCharge::new(b, "final-frontier-membership");
    let mut frames = charged::ChargedSet::default();
    for run in data.runs.iter() {
        if data.inputs.get(run.input).is_none() {
            return Err(invalid("final frontier native input absent"));
        }
        frames.insert(&mut charge, (run.input, run.context))?;
    }
    if frames.is_empty() && !data.inputs.is_empty() {
        return Err(invalid(
            "final frontier lacks captured native frame universe",
        ));
    }
    let mut invocations = charged::ChargedSet::default();
    let mut outcomes = charged::ChargedSet::default();
    let mut coverages = charged::ChargedSet::default();
    let earlier = if target == Target::Catalog {
        Some(derive(data, Target::Analysis, b)?)
    } else {
        None
    };
    if let Some(earlier) = &earlier
        && (!equal(&earlier.analysis, &data.analysis)
            || !equal(&earlier.analysis_members, &data.analysis_members))
    {
        return Err(invalid(
            "Catalog final assessment changes immutable Analysis checkpoint",
        ));
    }
    for &(input, context) in frames.iter() {
        let mut members = charged::ChargedMap::<ContentHash, Draft>::default();
        let mut frame_charge = charged::StateCharge::new(b, "final-frontier-frame-members");
        let _native = b.reserve(
            "final-frontier-native-digest",
            data.runs
                .len()
                .saturating_mul(size_of::<ContentHash>() + 128),
        )?;
        let capture = digest(
            "final-frontier-captured-native-runs",
            data.runs
                .iter()
                .filter(|r| (r.input, r.context) == (input, context))
                .map(Record::content_digest),
        );
        let mut availability = Vec::new();
        let mut reasons = Vec::new();
        macro_rules! assess_owners{($($field:ident:$variant:ident:$owner:ident=>[$($method:ident),*],)*)=>{$(if owner_required(target,stringify!($owner)){$({
  let method=Method::$method;let mut matching=data.$field.invocations.iter().filter(|i|(i.input,i.context)==(input,context)&&data.definitions.get(i.definition).is_some_and(|d|d.method==method));let invocation=matching.next().ok_or_else(||invalid("required final frontier invocation absent"))?;if matching.next().is_some()||invocation.subject.is_some(){return Err(invalid("final frontier has duplicate or subject-specific invocation"));}invocations.insert(&mut charge,derivation::RowRef::of(invocation.id()))?;
  let definition=data.definitions.get(invocation.definition).ok_or_else(||invalid("final frontier definition absent"))?;definition.validate()?;let outcome=data.$field.outcomes.iter().find(|r|r.invocation==invocation.id()).ok_or_else(||invalid("required final frontier outcome absent"))?;outcome.validate()?;outcomes.insert(&mut charge,derivation::RowRef::of(outcome.id()))?;
  let contract=analysis::expected::method_contract(method)?;let domain=data.frontier.domain(input,context,contract)?;
  for scope in &domain.scopes{let _scratch=b.reserve("final-frontier-scope-observations",(scope.native.len()+scope.normalized.len()).saturating_mul(1024).saturating_add(512))?;let mut observed=Vec::new();for row in &scope.native{observed.push(analysis::$owner::coverage::CoverageObservation::native(row)?);}for row in &scope.normalized{observed.push(analysis::$owner::coverage::CoverageObservation::normalized(row)?);}let expectation=analysis::$owner::coverage::CoverageExpectation{invocation:invocation.id(),capability:contract.capability,scope:scope.scope,context:scope.context,requested:scope.requested,no_scope:scope.no_scope,sources:observed.iter().map(|o|o.source().id()).collect()};let(expected,_)=analysis::$owner::coverage::assess(&expectation,&observed,outcome.status,outcome.reason,b)?;let coverage=data.$field.coverage.get(expected.id()).filter(|r|**r==expected).ok_or_else(||invalid("final frontier differs from canonical owner coverage"))?;coverages.insert(&mut charge,derivation::RowRef::of(coverage.id()))?;
   let member=Draft::$variant{method,invocation:invocation.id(),outcome:outcome.id(),coverage:coverage.id(),payload:payload(invocation,outcome,coverage)};if members.insert(&mut frame_charge,member.id(),member)?.is_some(){return Err(invalid("duplicate final frontier member"));}if scope.requested{frame_charge.grow(size_of::<EvidenceAvailability>()+size_of::<Option<ObligationKind>>()+32)?;availability.push(coverage.availability);reasons.push(coverage.reason);}
  }
 })*})*};}
        owners!(assess_owners);
        if members.is_empty() || availability.is_empty() {
            return Err(invalid(
                "final frontier cannot infer completion from empty observations",
            ));
        }
        let availability = analysis::coverage::combine_availability(availability);
        let reason = match availability {
            EvidenceAvailability::Complete | EvidenceAvailability::NoScope => None,
            EvidenceAvailability::NotRequested => Some(ObligationKind::NotRequested),
            _ => obligation::first(reasons.into_iter().flatten())
                .or(Some(ObligationKind::IncompleteCoverage)),
        };
        let membership = digest("final-frontier-scheduled-members", members.keys().copied());
        match target {
            Target::Analysis => {
                let row = AnalysisAssessment {
                    input,
                    context,
                    capture,
                    members: membership,
                    availability,
                    reason,
                };
                for member in members.values() {
                    result.analysis_members.insert(member.analysis(row.id())?)?;
                }
                result.analysis.insert(row)?;
            }
            Target::Catalog => {
                let earlier = earlier.as_ref().expect("Catalog regenerates Analysis");
                let analysis = earlier
                    .analysis
                    .iter()
                    .find(|r| (r.input, r.context) == (input, context))
                    .ok_or_else(|| invalid("Catalog lacks matching Analysis assessment"))?;
                let row = CatalogAssessment {
                    input,
                    context,
                    analysis: analysis.id(),
                    capture,
                    members: membership,
                    availability,
                    reason,
                };
                for member in members.values() {
                    result.catalog_members.insert(member.catalog(row.id()))?;
                }
                result.catalog.insert(row)?;
            }
        }
    }
    macro_rules! exact_owners{($($field:ident:$variant:ident:$owner:ident=>[$($method:ident),*],)*)=>{$(if owner_required(target,stringify!($owner)){for row in data.$field.invocations.iter(){if !invocations.contains(&derivation::RowRef::of(row.id())){return Err(invalid("extra final frontier invocation"));}}for row in data.$field.outcomes.iter(){if !outcomes.contains(&derivation::RowRef::of(row.id())){return Err(invalid("extra final frontier outcome"));}}for row in data.$field.coverage.iter(){if !coverages.contains(&derivation::RowRef::of(row.id())){return Err(invalid("extra final frontier coverage"));}}})*};}
    owners!(exact_owners);
    Ok(result)
}
fn equal<R: Record + PartialEq>(a: &Rows<R>, b: &Rows<R>) -> bool {
    a.len() == b.len() && a.iter().all(|r| b.get(r.id()) == Some(r))
}
fn checks(target: Target) -> Vec<PublicationInvariant> {
    let mut inputs = FrontierData::inputs(target);
    match target {
        Target::Analysis => inputs.extend([
            ValidationInput::of::<AnalysisAssessment>(&["id"]),
            ValidationInput::of::<AnalysisMember>(&["id"]),
        ]),
        Target::Catalog => inputs.extend([
            ValidationInput::of::<CatalogAssessment>(&["id"]),
            ValidationInput::of::<CatalogMember>(&["id"]),
        ]),
    };
    vec![PublicationInvariant {
        revision: 1,
        name: match target {
            Target::Analysis => "complete_analysis_frontier",
            Target::Catalog => "complete_catalog_frontier",
        },
        inputs,
        create: std::sync::Arc::new(move |b| {
            Box::new(Check {
                data: FrontierData::new(Profile::Catalog, b),
                actual: FrontierRecords::new(b),
                target,
                budget: b.clone(),
            })
        }),
    }]
}
pub(crate) fn analysis_checks() -> Vec<PublicationInvariant> {
    checks(Target::Analysis)
}
pub(crate) fn catalog_checks() -> Vec<PublicationInvariant> {
    checks(Target::Catalog)
}
struct Check {
    data: FrontierData,
    actual: FrontierRecords,
    target: Target,
    budget: ResourceBudget,
}
impl PublicationCheck for Check {
    fn visit(&mut self, n: &str, b: &arrow_array::RecordBatch) -> Result<(), ModelError> {
        match (self.target, n) {
            (Target::Analysis, AnalysisAssessment::NAME) => {
                self.actual.analysis.decode(b)?;
                Ok(())
            }
            (Target::Analysis, AnalysisMember::NAME) => {
                self.actual.analysis_members.decode(b)?;
                Ok(())
            }
            (Target::Catalog, CatalogAssessment::NAME) => {
                self.actual.catalog.decode(b)?;
                Ok(())
            }
            (Target::Catalog, CatalogMember::NAME) => {
                self.actual.catalog_members.decode(b)?;
                Ok(())
            }
            _ => self.data.visit(n, b),
        }
    }
    fn finish(
        mut self: Box<Self>,
        _: &[crate::domain::analysis::sources::SourceSnapshot],
        profile: Profile,
    ) -> Result<(), ModelError> {
        self.data.frontier.set_profile(profile);
        let expected = derive(&self.data, self.target, &self.budget)?;
        if !equal(&expected.analysis, &self.actual.analysis)
            || !equal(&expected.analysis_members, &self.actual.analysis_members)
            || !equal(&expected.catalog, &self.actual.catalog)
            || !equal(&expected.catalog_members, &self.actual.catalog_members)
        {
            return Err(invalid(
                "final frontier inventory differs from independent replay",
            ));
        }
        Ok(())
    }
}
pub fn stage(
    profile: Profile,
    target: Target,
    model: &ValidatedModel,
    order: &stages::PublicationOrder,
) -> Result<stages::Stage, ModelError> {
    let outputs = match target {
        Target::Analysis => analysis_relations(),
        Target::Catalog => catalog_relations(),
    };
    let owned = outputs
        .iter()
        .map(stages::RelationUse::of_relation)
        .collect::<Vec<_>>();
    let inputs = dependency_closure::DependencyClosure::stage_grants(
        model,
        FrontierData::inputs(target),
        &owned,
        match target {
            Target::Analysis => stages::PublicationBoundary::Analytic,
            Target::Catalog => stages::PublicationBoundary::Synthesis,
        },
        dependency_closure::LowerLayerPolicy::OmitInferredOrdinaryFacts,
        order,
    )?;
    let mut code = KeySink::new("final-frontier-shared-kernel");
    for bytes in [
        include_bytes!("frontier.rs").as_slice(),
        include_bytes!("expected.rs").as_slice(),
        include_bytes!("coverage.rs").as_slice(),
        include_bytes!("family/coverage.rs").as_slice(),
    ] {
        ContentHash::of(bytes).encode(&mut code);
    }
    Ok(stages::Stage {
        name: match target {
            Target::Analysis => "assess_analysis_frontier",
            Target::Catalog => "assess_catalog_frontier",
        },
        inputs,
        outputs: outputs
            .iter()
            .map(stages::RelationUse::of_relation)
            .collect(),
        contributes: vec![],
        coverage: vec![],
        profiles: vec![profile],
        effect: stages::Effect::Pure,
        code: code.finish(),
        configuration: ContentHash::of(match target {
            Target::Analysis => b"analysis-final".as_slice(),
            Target::Catalog => b"catalog-final".as_slice(),
        }),
    })
}
/// Typed reader registration; the selected stage filters this superset using FrontierData::inputs.
#[macro_export]
macro_rules! final_frontier_inputs{($apply:ident)=>{$apply!{
 $crate::domain::attribution::ProviderRun,$crate::domain::input::InputRevision,$crate::domain::input::ArtifactUse,$crate::domain::source::SourceArtifact,$crate::domain::source::CoverageScope,$crate::domain::attribution::ProviderCoverage,$crate::domain::normalized::coverage::NormalizationComputation,$crate::domain::normalized::coverage::NormalizationCoverage,$crate::domain::analysis::AnalysisDefinition,$crate::domain::analysis::settings::AnalyticsConfiguration,$crate::domain::embedding::text::TextDefinition,
 $crate::domain::analysis::local::AnalysisInvocation,$crate::domain::analysis::local::AnalysisOutcome,$crate::domain::analysis::local::AnalysisCoverage,
 $crate::domain::analysis::base_evaluation::AnalysisInvocation,$crate::domain::analysis::base_evaluation::AnalysisOutcome,$crate::domain::analysis::base_evaluation::AnalysisCoverage,
 $crate::domain::analysis::base_completion::AnalysisInvocation,$crate::domain::analysis::base_completion::AnalysisOutcome,$crate::domain::analysis::base_completion::AnalysisCoverage,
 $crate::domain::analysis::source_call::AnalysisInvocation,$crate::domain::analysis::source_call::AnalysisOutcome,$crate::domain::analysis::source_call::AnalysisCoverage,
 $crate::domain::analysis::enriched_execution::AnalysisInvocation,$crate::domain::analysis::enriched_execution::AnalysisOutcome,$crate::domain::analysis::enriched_execution::AnalysisCoverage,
 $crate::domain::analysis::model::AnalysisInvocation,$crate::domain::analysis::model::AnalysisOutcome,$crate::domain::analysis::model::AnalysisCoverage,
 $crate::domain::analysis::summary::AnalysisInvocation,$crate::domain::analysis::summary::AnalysisOutcome,$crate::domain::analysis::summary::AnalysisCoverage,
 $crate::domain::analysis::structural::AnalysisInvocation,$crate::domain::analysis::structural::AnalysisOutcome,$crate::domain::analysis::structural::AnalysisCoverage,
 $crate::domain::analysis::analytic_embedding::AnalysisInvocation,$crate::domain::analysis::analytic_embedding::AnalysisOutcome,$crate::domain::analysis::analytic_embedding::AnalysisCoverage,
 $crate::domain::analysis::analytic::AnalysisInvocation,$crate::domain::analysis::analytic::AnalysisOutcome,$crate::domain::analysis::analytic::AnalysisCoverage,
 $crate::domain::analysis::catalog_core::AnalysisInvocation,$crate::domain::analysis::catalog_core::AnalysisOutcome,$crate::domain::analysis::catalog_core::AnalysisCoverage,
 $crate::domain::analysis::catalog_evidence::AnalysisInvocation,$crate::domain::analysis::catalog_evidence::AnalysisOutcome,$crate::domain::analysis::catalog_evidence::AnalysisCoverage,
 $crate::domain::analysis::selection::AnalysisInvocation,$crate::domain::analysis::selection::AnalysisOutcome,$crate::domain::analysis::selection::AnalysisCoverage,
 $crate::domain::analysis::synthesis::AnalysisInvocation,$crate::domain::analysis::synthesis::AnalysisOutcome,$crate::domain::analysis::synthesis::AnalysisCoverage,
 $crate::domain::analysis::retrieval::AnalysisInvocation,$crate::domain::analysis::retrieval::AnalysisOutcome,$crate::domain::analysis::retrieval::AnalysisCoverage,
 $crate::domain::analysis::frontier::AnalysisAssessment,$crate::domain::analysis::frontier::AnalysisMember,
}};}
pub(crate) fn analysis_checks_refs() -> Vec<&'static str> {
    vec!["complete_analysis_frontier"]
}
pub(crate) fn catalog_checks_refs() -> Vec<&'static str> {
    vec!["complete_catalog_frontier"]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{
        attribution::{CoverageStatus, FactFamily, ProviderCoverage},
        normalized::coverage::{Capability, NormalizationComputation, NormalizationCoverage},
        source::CoverageScope,
    };
    fn nominal<T>(n: u8) -> Id<T> {
        serde::Deserialize::deserialize(serde::de::value::SeqDeserializer::<
            _,
            serde::de::value::Error,
        >::new([n; 16].into_iter()))
        .unwrap()
    }
    fn load<R: Record>(data: &mut FrontierData, rows: &[R]) {
        data.visit(R::NAME, &R::encode(rows).unwrap()).unwrap();
    }
    // Pure declared-empty-domain control. These rows are not a provider/runtime qualification.
    fn fixture(profile: Profile, b: &ResourceBudget) -> FrontierData {
        let mut data = FrontierData::new(profile, b);
        let input = InputRevision::from_entries(vec![]).unwrap();
        let context = nominal(2);
        let run = ProviderRun {
            input: input.id(),
            context,
            provider: nominal(3),
            configuration: ContentHash::of(b"pure"),
            requested_families: ContentHash::of(b"pure Deployment"),
        };
        load(&mut data, std::slice::from_ref(&input));
        load(&mut data, std::slice::from_ref(&run));
        let root = CoverageScope::Input { input: input.id() };
        load(&mut data, std::slice::from_ref(&root));
        load(
            &mut data,
            &[ProviderCoverage {
                scope: root.id(),
                context,
                provider: Some(run.provider),
                family: FactFamily::Deployment,
                run: Some(run.id()),
                status: CoverageStatus::CompleteUnderStatedModel,
                reason: None,
                diagnostic: None,
            }],
        );
        for capability in Capability::ALL {
            let computation = NormalizationComputation {
                capability,
                policy: ContentHash::of(b"pure normalization"),
                producer: "pure".into(),
                declaration: ContentHash::of(b"pure"),
                profile: profile.name().into(),
                availability: EvidenceAvailability::Complete,
            };
            load(&mut data, std::slice::from_ref(&computation));
            load(
                &mut data,
                &[NormalizationCoverage {
                    computation: computation.id(),
                    scope: root.id(),
                    context,
                    availability: EvidenceAvailability::Complete,
                }],
            );
        }
        let settings = analysis::settings::AnalyticsConfiguration {
            module_prefixes: vec!["pure".into()],
            public_roots: vec!["pure".into()],
            configured_seeds: vec![],
            depth: 1,
            vertices: 1,
            arcs: 1,
            witnesses: 1,
            brief_budget: 1,
            communities: false,
            pagerank: false,
            fca: false,
            knn: false,
            rca: false,
            type_layer: false,
            mention_layer: false,
            knn_layer: false,
        };
        load(&mut data, &[settings]);
        load(&mut data, &[embedding::text::TextDefinition::builtin()]);
        macro_rules! populate{($($field:ident:$variant:ident:$owner:ident=>[$($method:ident),*],)*)=>{$($({let method=Method::$method;let definition=analysis::AnalysisDefinition{method,interpretation:match method{Method::PageRank|Method::Communities|Method::Neighbours|Method::AnalyticEmbedding=>analysis::Interpretation::Heuristic,Method::Concepts|Method::RelationalConcepts=>analysis::Interpretation::ExactUnderContext,_=>analysis::Interpretation::Structural},parameters:nominal(4),semantic_version:digest("pure method",[method])};load(&mut data,std::slice::from_ref(&definition));let(invocation,_)=analysis::$owner::AnalysisInvocation::new(input.id(),context,definition.id(),None,[]);let contract=analysis::expected::method_contract(method).unwrap();let domain=data.frontier.domain(input.id(),context,contract).unwrap();let requested=domain.scopes.iter().any(|s|s.requested);let outcome=analysis::$owner::AnalysisOutcome{invocation:invocation.id(),status:if requested{analysis::AnalysisStatus::Completed}else{analysis::AnalysisStatus::NotRequested},reason:(!requested).then_some(ObligationKind::NotRequested)};data.$field.invocations.insert(invocation.clone()).unwrap();data.$field.outcomes.insert(outcome.clone()).unwrap();for scope in &domain.scopes{let mut observed=vec![];for row in &scope.native{observed.push(analysis::$owner::coverage::CoverageObservation::native(row).unwrap());}for row in &scope.normalized{observed.push(analysis::$owner::coverage::CoverageObservation::normalized(row).unwrap());}let expectation=analysis::$owner::coverage::CoverageExpectation{invocation:invocation.id(),capability:contract.capability,scope:scope.scope,context:scope.context,requested:scope.requested,no_scope:scope.no_scope,sources:observed.iter().map(|r|r.source().id()).collect()};let(row,_)=analysis::$owner::coverage::assess(&expectation,&observed,outcome.status,outcome.reason,b).unwrap();data.$field.coverage.insert(row).unwrap();}})*)*};}
        owners!(populate);
        data
    }
    #[test]
    fn analysis_member_schema_has_no_catalog_successor_references() {
        let mut declarations = crate::domain::normalized_relations();
        declarations.extend(crate::domain::analysis_relations());
        let mut late = analysis::selection::relations();
        late.extend(analysis::synthesis::relations());
        late.extend(analysis::retrieval::relations());
        late.extend(crate::domain::selection::relations());
        late.extend(crate::domain::synthesis::relations());
        late.extend(crate::domain::retrieval::relations());
        late.extend(analysis::findings::relations());
        let names = late
            .iter()
            .map(Relation::name)
            .collect::<std::collections::BTreeSet<_>>();
        declarations.retain(|r| {
            !names.contains(r.name())
                && r.name() != AnalysisAssessment::NAME
                && r.name() != AnalysisMember::NAME
                && r.name() != CatalogAssessment::NAME
                && r.name() != CatalogMember::NAME
        });
        declarations.extend(analysis_relations());
        ValidatedModel::declared(declarations)
            .expect("Analysis frontier independently closes without Catalog successors");
        assert!(
            Relation::of::<AnalysisMember>()
                .fields()
                .iter()
                .filter_map(|f| f.target().map(|(_, name)| name))
                .all(|name| !names.contains(name))
        );
    }
    #[test]
    fn fixed_methods_optional_outcomes_and_immutable_catalog_membership() {
        for profile in [Profile::Catalog, Profile::Behavioral] {
            let b = ResourceBudget::fixed(16 << 20).unwrap();
            let mut data = fixture(profile, &b);
            let analysis = derive(&data, Target::Analysis, &b).unwrap();
            assert_eq!(analysis.analysis.len(), 1);
            assert_eq!(analysis.analysis_members.len(), 19);
            assert_eq!(
                analysis
                    .analysis_members
                    .iter()
                    .filter(|m| matches!(m, AnalysisMember::Structural { .. }))
                    .count(),
                4
            );
            assert_eq!(
                analysis
                    .analysis_members
                    .iter()
                    .filter(|m| matches!(m, AnalysisMember::Analytic { .. }))
                    .count(),
                5
            );
            assert_eq!(
                analysis.analysis.iter().next().unwrap().availability,
                EvidenceAvailability::Complete,
                "optional NotRequested remains explicit without obscuring requested frontier availability"
            );
            assert_eq!(
                data.analytic
                    .outcomes
                    .iter()
                    .filter(|o| o.status == analysis::AnalysisStatus::NotRequested)
                    .count(),
                5
            );
            for row in analysis.analysis.iter() {
                data.analysis.insert(row.clone()).unwrap();
            }
            for row in analysis.analysis_members.iter() {
                data.analysis_members.insert(row.clone()).unwrap();
            }
            let catalog = derive(&data, Target::Catalog, &b).unwrap();
            assert_eq!(catalog.catalog.len(), 1);
            assert_eq!(catalog.catalog_members.len(), 22);
            assert_eq!(
                catalog.catalog.iter().next().unwrap().analysis,
                analysis.analysis.iter().next().unwrap().id()
            );
            assert!(derive(&data, Target::Analysis, &ResourceBudget::fixed(1).unwrap()).is_err());
            data.analysis_members = Rows::new(&b);
            assert!(derive(&data, Target::Catalog, &b).is_err());
        }
    }
    #[test]
    fn missing_extra_forged_and_coupled_erasure_refuse() {
        let b = ResourceBudget::fixed(16 << 20).unwrap();
        let mut data = fixture(Profile::Catalog, &b);
        let baseline = derive(&data, Target::Analysis, &b).unwrap();
        let actual = FrontierRecords::new(&b);
        assert!(
            Box::new(Check {
                data,
                actual,
                target: Target::Analysis,
                budget: b.clone()
            })
            .finish(&[], Profile::Catalog)
            .is_err(),
            "erased final assessment plus all members cannot reduce native expected frames"
        );
        data = fixture(Profile::Catalog, &b);
        data.local.invocations = Rows::new(&b);
        data.local.outcomes = Rows::new(&b);
        data.local.coverage = Rows::new(&b);
        assert!(
            derive(&data, Target::Analysis, &b).is_err(),
            "coupled owner-frame erasure cannot shrink captured universe"
        );
        data = fixture(Profile::Catalog, &b);
        let extra = analysis::local::AnalysisInvocation::new(
            nominal(7),
            nominal(8),
            data.local.invocations.iter().next().unwrap().definition,
            None,
            [],
        )
        .0;
        data.local.invocations.insert(extra).unwrap();
        assert!(derive(&data, Target::Analysis, &b).is_err());
        data = fixture(Profile::Catalog, &b);
        let old = data.analytic.outcomes.iter().next().unwrap().clone();
        data.analytic.outcomes = Rows::new(&b);
        data.analytic
            .outcomes
            .insert(analysis::analytic::AnalysisOutcome {
                status: analysis::AnalysisStatus::Completed,
                reason: None,
                ..old
            })
            .unwrap();
        assert!(derive(&data, Target::Analysis, &b).is_err());
        data = fixture(Profile::Catalog, &b);
        let mut forged = FrontierRecords::new(&b);
        for row in baseline.analysis.iter() {
            forged
                .analysis
                .insert(AnalysisAssessment {
                    availability: EvidenceAvailability::Unavailable,
                    reason: Some(ObligationKind::MissingEvidence),
                    ..row.clone()
                })
                .unwrap();
        }
        for row in baseline.analysis_members.iter() {
            forged.analysis_members.insert(row.clone()).unwrap();
        }
        assert!(
            Box::new(Check {
                data,
                actual: forged,
                target: Target::Analysis,
                budget: b.clone()
            })
            .finish(&[], Profile::Catalog)
            .is_err()
        );
    }
}
