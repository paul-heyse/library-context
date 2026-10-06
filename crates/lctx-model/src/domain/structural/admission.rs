//! Portable Structural frame and source fidelity. Topology traversal is diagnostic replay.
use super::{build::{invalid, need}, frames::Context, *};
use crate::domain::{
    analysis::{self, settings::AnalyticsConfiguration, structural as owner},
    assertion::{Approximation, AssertionQualification},
    attribution::{Modality, ProviderRun},
    execution::fidelity::{ExecutionMembership, ExecutionScope},
    normalized::{Rows, entities::*, events::*},
    projection::{ProjectionName, ProjectionSourceAssessment},
    resources::ResourceBudget,
    *,
};

fn frame_inputs() -> Vec<ValidationInput> {
    vec![
        ValidationInput::of::<ProviderRun>(&["id"]),
        ValidationInput::of::<analysis::catalog_core::Invocation>(&["id"]),
        ValidationInput::of::<AnalyticsConfiguration>(&["id"]),
        ValidationInput::of::<analysis::AnalysisDefinition>(&["id"]),
        ValidationInput::of::<analysis::MethodParameters>(&["id"]),
        ValidationInput::of::<analysis::local::Invocation>(&["id"]),
        ValidationInput::of::<analysis::local::AnalysisOutcome>(&["id"]),
        ValidationInput::of::<analysis::local::AnalysisCoverage>(&["id"]),
        ValidationInput::of::<owner::Invocation>(&["id"]),
        ValidationInput::of::<owner::InvocationSource>(&["id"]),
        ValidationInput::of::<owner::AnalysisInput>(&["id"]),
        ValidationInput::of::<owner::AnalysisOutcome>(&["id"]),
        ValidationInput::of::<ProjectionSourceAssessment>(&["id"]),
        ValidationInput::of::<StructuralFrame>(&["id"]),
        ValidationInput::of::<Traversal>(&["id"]),
        ValidationInput::of::<controls::ControlTraversal>(&["id"]),
    ]
}
// These global rows are compact frame, membership and stop metadata. No native body, path,
// graph bytes or rich Structural output is retained by this independent inventory predicate.
struct FrameCheck {
    runs: Rows<ProviderRun>,
    core: Rows<analysis::catalog_core::Invocation>,
    context: Context,
    output: Output,
    budget: ResourceBudget,
}
impl FrameCheck {
    fn new(b: &ResourceBudget) -> Self {
        Self { runs: Rows::new(b), core: Rows::new(b), context: Context::new(b),
            output: Output::new(b), budget: b.clone() }
    }
    fn verify(&self) -> Result<(), ModelError> {
        let parents = catalog::evidence::frames::parents(&self.runs, &self.core, &self.budget)?;
        let mut frames = Rows::new(&self.budget);
        let mut sources = Rows::new(&self.budget);
        let mut inputs = Rows::new(&self.budget);
        let mut invocations = charged::ChargedSet::default();
        let mut charge = charged::StateCharge::new(&self.budget, "structural_frame_fidelity");
        for core in parents.iter() {
            let local = self.context.local_parent(core)?;
            let mut outcomes = self.context.local_outcomes.iter().filter(|row| row.invocation == local.id());
            let outcome = outcomes.next().ok_or_else(|| invalid("Structural Local outcome absent"))?;
            if outcomes.next().is_some() {
                return Err(invalid("Structural Local outcome domain ambiguous"));
            }
            outcome.validate()?;
            let frame = frames::frame(&self.context, core)?;
            if frame.controls_requested != (outcome.status != analysis::AnalysisStatus::NotRequested) {
                return Err(invalid("Structural controls eligibility differs from actual Local outcome"));
            }
            frames.insert(frame.clone())?;
            for invocation in [frame.invocation, frame.usage_invocation, frame.handoff_invocation, frame.control_invocation] {
                let row = need(&self.context.invocations, invocation)?;
                if (row.input, row.context, row.subject) != (core.input, core.context, None)
                    || !invocations.insert(&mut charge, invocation)? {
                    return Err(invalid("Structural exact complete invocation domain"));
                }
                for source in [owner::InvocationSource::Local { invocation: local.id() },
                    owner::InvocationSource::CatalogCore { invocation: core.id() }] {
                    let parent = sources.insert(source)?;
                    inputs.insert(owner::AnalysisInput { invocation, parent })?;
                }
            }
            for (id, name) in [(frame.invocation_graph, ProjectionName::CallableInvocation),
                (frame.definition_graph, ProjectionName::DefinitionContainment)] {
                let graph = need(&self.context.graphs.assessments, id)?;
                if (graph.input, graph.context, graph.projection) != (core.input, core.context, name) {
                    return Err(invalid("Structural canonical projection frame differs"));
                }
            }
        }
        if !frames.same(&self.output.frames) || invocations.len() != self.context.invocations.len()
            || !sources.same(&self.context.sources) || !inputs.same(&self.context.inputs) {
            return Err(invalid("Structural exact complete frame/parent domain"));
        }
        for traversal in self.output.traversals.iter() {
            need(&self.output.frames, traversal.frame)?;
        }
        for traversal in self.output.control_traversals.iter() {
            let frame = need(&self.output.frames, traversal.frame)?;
            if !frame.controls_requested {
                return Err(invalid("Structural unrequested Controls has a traversal"));
            }
        }
        // This is the source-owned status policy over actual stops and acknowledged NoScope,
        // not a traversal/evaluation replay or a fabricated completeness receipt.
        if !self.context.outcomes.same(&outcomes::derive(&self.context, &self.output, &self.budget)?) {
            return Err(invalid("Structural outcome differs from actual eligibility/stop/NoScope"));
        }
        Ok(())
    }
}
impl InvariantCheck for FrameCheck {
    fn visit(&mut self, name: &str, batch: &arrow_array::RecordBatch) -> Result<(), ModelError> {
        if name == ProviderRun::NAME { self.runs.decode(batch) }
        else if name == analysis::catalog_core::Invocation::NAME { self.core.decode(batch) }
        else if self.context.visit(name, batch)? || self.output.visit(name, batch)? { Ok(()) }
        else { Err(invalid("undeclared Structural frame fidelity input")) }
    }
    fn finish(self: Box<Self>) -> Result<(), ModelError> { self.verify() }
}

macro_rules! source_rows { ($($field:ident:$ty:ty,)*) => {
    struct SourceData { $( $field: Rows<$ty>, )* }
    impl SourceData {
        fn new(b: &ResourceBudget) -> Self { Self { $( $field: Rows::new(b), )* } }
        fn inputs() -> Vec<ValidationInput> { vec![$(ValidationInput::of::<$ty>(&["id"]),)*] }
        fn visit(&mut self, name: &str, batch: &arrow_array::RecordBatch) -> Result<bool,ModelError> {
            $(if name == <$ty>::NAME { self.$field.decode(batch)?; return Ok(true); })* Ok(false)
        }
    }
}; }
source_rows! {
    conclusions:Conclusion, sources:ConclusionSource, frames:StructuralFrame,
    invocations:owner::Invocation, definitions:analysis::AnalysisDefinition,
    qualifications:AssertionQualification,
    public:PublicCandidate, paths:Path, reaches:Reach, traversals:Traversal,
    steps:PathStep, evidence:StepEvidence, arcs:ArcSource,
    unresolved:UnresolvedEvent, usage:UsageScore, groups:handoffs::Group,
    control_paths:controls::ControlPath, control_traversals:controls::ControlTraversal,
    control_steps:controls::ControlStep, flows:controls::ArgumentFlow,
    literals:controls::LiteralArgument, raises:controls::ConditionalRaise,
    unfollowed:controls::UnfollowedPath,
    events:NormalizedCallEvent, alternatives:NormalizedCallAlternative,
    assessments:EventAssessment, policy:CallPolicyAssessment, admissions:CallPolicyAdmission,
    occurrences:source::Occurrence, ownership:OccurrenceOwnership, callables:CallableEntity,
    entities:EntityRef, targets:calls::CallTarget, alternative_sources:CallAlternativeSource,
    bindings:normalized::bindings::CallBinding, attempts:normalized::bindings::CallBindingAttempt,
    binding_sources:calls::BindingSource, slots:normalized::callables::SignatureSlot, links:ParameterEntityLink, parameters:calls::SignatureParameter,
    contributions:local_semantics::LocalContribution, entries:conditions::entry::EntryValueWitness,
    values:flow::FlowValueObservation, aliases:handoffs::ValueSource,
    uses:flow::FlowUseObservation, reaching:flow::FlowReachingObservation,
    definitions_of_values:flow::FlowDefinitionObservation,
    inventories:flow_inventory::FlowUseInventoryObservation, views:flow::FlowSourceViewObservation,
    regions:flow::FlowRegionObservation,
    conditions:conditions::Condition, nodes:conditions::ConditionNode,
}
struct SourceCheck { data: SourceData, scopes: ownership::ScopeIndex, assumptions: assumptions::AssumptionIndex, budget: ResourceBudget }
impl SourceCheck {
    fn new(b: &ResourceBudget) -> Self {
        Self { data: SourceData::new(b), scopes: ownership::ScopeIndex::new(b,"structural_source_fidelity"), assumptions: assumptions::AssumptionIndex::new(b), budget: b.clone() }
    }
    fn inputs() -> Vec<ValidationInput> {
        let mut inputs = SourceData::inputs();
        inputs.extend(ownership::ScopeIndex::inputs());
        inputs.extend(assumptions::AssumptionIndex::inputs());
        inputs.sort_by_key(|input| (input.name(),input.prefix()));
        inputs.dedup_by_key(|input| (input.name(),input.prefix()));
        inputs
    }
    fn scope() -> ExecutionScope {
        ExecutionScope {
            root: ValidationInput::of::<Conclusion>(&["id"]),
            memberships: vec![
                (ValidationInput::of::<PathStep>(&["id"]), "path", ValidationInput::of::<Path>(&["id"])),
                (ValidationInput::of::<controls::ControlStep>(&["id"]), "path", ValidationInput::of::<controls::ControlPath>(&["id"])),
                (ValidationInput::of::<CallPolicyAdmission>(&["id"]), "alternative", ValidationInput::of::<NormalizedCallAlternative>(&["id"])),
                (ValidationInput::of::<ParameterEntityLink>(&["id"]), "parameter", ValidationInput::of::<calls::SignatureParameter>(&["id"])),
                (ValidationInput::of::<assumptions::AssumptionSetMember>(&["id"]), "set", ValidationInput::of::<assumptions::AssumptionSet>(&["id"])),
            ],
            joins: vec![ExecutionMembership {
                source: ValidationInput::of::<controls::LiteralArgument>(&["id"]), source_key: "caller",
                member: ValidationInput::of::<PublicCandidate>(&["id"]), member_key: "entity", context: None,
            }],
        }
    }
    fn qualification(&self, id: Id<AssertionQualification>, invocation: &owner::Invocation,
        source: Id<source::SourceArtifact>) -> Result<&AssertionQualification,ModelError> {
        let q = need(&self.data.qualifications,id)?;
        if q.context != invocation.context || !self.scopes.within(source,self.scopes.scope(q.scope)?)?
            || !self.scopes.within(source,&source::CoverageScope::Input {input:invocation.input})? {
            return Err(invalid("Structural source qualification crosses input/context/scope"));
        }
        Ok(q)
    }
    fn step(&self, step: &PathStep, invocation: &owner::Invocation) -> Result<(Modality,Approximation),ModelError> {
        let d = &self.data;
        match (need(&d.arcs,step.arc)?, need(&d.evidence,step.evidence)?) {
            (ArcSource::Invocation {alternative} | ArcSource::Definition {alternative},
                StepEvidence::Call {event,site,phase,qualification,modality,derived_dispatch}) => {
                let alternative = need(&d.alternatives,*alternative)?;
                let event_row = need(&d.events,*event)?;
                let site_row = need(&d.occurrences,*site)?;
                let owner = need(&d.ownership,event_row.owner)?;
                let source = need(&d.alternative_sources,alternative.source)?;
                let raw = need(&d.targets,source.target())?;
                if alternative.event != *event || event_row.site != *site || event_row.context != invocation.context
                    || step.source != owner.entity || alternative.entity != Some(step.target)
                    || raw.site != *site || raw.origin != event_row.origin || raw.phase != *phase
                    || raw.qualification != *qualification
                    || *derived_dispatch != matches!(source,CallAlternativeSource::DerivedDispatch {..})
                    || (matches!(need(&d.arcs,step.arc)?,ArcSource::Definition {..})
                        && !matches!(raw.phase,calls::CallPhase::Definition|calls::CallPhase::Decorator))
                    || (matches!(need(&d.arcs,step.arc)?,ArcSource::Invocation {..})
                        && !d.admissions.iter().any(|admission|admission.alternative==alternative.id()
                            && d.policy.get(admission.assessment).is_some_and(|policy|policy.event==event_row.id()
                                && policy.policy==CallPolicy::Invocation))) {
                    return Err(invalid("Structural path step changes normalized source/target"));
                }
                let q = self.qualification(*qualification,invocation,site_row.source)?;
                if *modality != q.modality {return Err(invalid("Structural path step changes source modality"));}
                Ok((q.modality,q.approximation))
            }
            (ArcSource::SourceDefinition {ownership},
                StepEvidence::Declaration {owner,declaration,callable}) => {
                let owned = need(&d.ownership,*ownership)?;
                let occurrence = need(&d.occurrences,*declaration)?;
                let target = need(&d.entities,step.target)?;
                if ownership != owner || owned.occurrence != *declaration || step.source != owned.entity
                    || target != &(EntityRef::Callable {callable:*callable})
                    || !matches!(need(&d.callables,*callable)?,CallableEntity::Source {declaration:d,..} if d==declaration)
                    || !self.scopes.within(occurrence.source,&source::CoverageScope::Input {input:invocation.input})? {
                    return Err(invalid("Structural definition step changes source ownership"));
                }
                Ok((Modality::Definite,Approximation::Exact))
            }
            _ => Err(invalid("Structural path step has incompatible arc/evidence")),
        }
    }
    fn flow(&self, flow: &controls::ArgumentFlow, invocation: &owner::Invocation) -> Result<&AssertionQualification,ModelError> {
        let d=&self.data;
        let binding=need(&d.bindings,flow.binding)?;
        let attempt=need(&d.attempts,binding.attempt)?;
        let alternative=need(&d.alternatives,attempt.alternative)?;
        let event=need(&d.events,alternative.event)?;
        let raw=need(&d.targets,need(&d.alternative_sources,alternative.source)?.target())?;
        let contribution=need(&d.contributions,flow.contribution)?;
        let entry=need(&d.entries,contribution.entry)?;
        let value=need(&d.values,contribution.value)?;
        let slot=need(&d.slots,binding.slot)?;
        let mut links=d.links.iter().filter(|link|link.parameter==slot.parameter);
        let link=links.next().ok_or_else(||invalid("Structural control formal absent"))?;
        let site=need(&d.occurrences,event.site)?;
        let owner=need(&d.ownership,event.owner)?;
        if links.next().is_some() || entry.owner!=flow.caller || owner.entity!=flow.caller
            || entry.formal!=flow.source || flow.formal!=link.entity || alternative.entity!=Some(flow.callee)
            || flow.phase!=raw.phase || event.context!=invocation.context || entry.context!=invocation.context
            || attempt.outcome!=normalized::bindings::BindingOutcome::Bound
            || attempt.authority!=normalized::signature_applicability::BindingAuthority::EffectiveInvocation
            || binding.projection!=calls::BindingProjection::Whole.id()
            || value.transfer!=transfer::TransferKind::Identity {
            return Err(invalid("Structural control source has incompatible binding/entry/frame"));
        }
        let calls::BindingSource::Actual {occurrence}=need(&d.binding_sources,binding.source)? else {
            return Err(invalid("Structural control source is not an actual argument"));
        };
        let mut ids=vec![raw.qualification,contribution.qualification];
        if let Some(alias)=flow.alias {
            let handoffs::ValueSource::Named {observation,reaching,definition,inventory,region,..}=need(&d.aliases,alias)? else {
                return Err(invalid("Structural control alias is not a named source"));
            };
            if value.kind!=flow::FlowSinkKind::Definition || contribution.definition!=Some(*definition) {
                return Err(invalid("Structural control alias changes its Local definition"));
            }
            ids.extend([need(&d.uses,*observation)?.qualification,need(&d.reaching,*reaching)?.qualification,
                need(&d.definitions_of_values,*definition)?.qualification,need(&d.inventories,*inventory)?.qualification,
                need(&d.views,need(&d.inventories,*inventory)?.view)?.qualification]);
            if let Some(region)=region {ids.push(need(&d.regions,*region)?.qualification);}
        } else if value.kind!=flow::FlowSinkKind::Argument || value.sink!=*occurrence {
            return Err(invalid("Structural control contribution changes its actual argument"));
        }
        let _nodes=self.budget.reserve("structural-control-condition-nodes",
            d.nodes.len().checked_mul(2048).ok_or_else(||invalid("Structural condition allocation overflow"))?)?;
        let nodes=d.nodes.iter().cloned().collect::<Vec<_>>();
        let call=self.qualification(ids[0],invocation,site.source)?;
        let local=self.qualification(ids[1],invocation,site.source)?;
        let mut condition=conditions::Diagram::from_records(need(&d.conditions,call.condition)?,&nodes)?;
        let mut modality=call.modality.weakest(local.modality);
        let approximation=call.approximation.join(local.approximation);
        let _basis=self.budget.reserve("structural-control-assumption-bases",
            ids.len()*assumptions::MAX_ASSUMPTIONS*2*size_of::<assumptions::AssumptionSetMember>())?;
        let mut bases=Vec::new();
        let mut condition_charge=None;
        for (position,id) in ids.iter().enumerate() {
            let q=self.qualification(*id,invocation,site.source)?;
            bases.push(self.assumptions.resolve(q.assumptions)?);
            // The producer conjoins call and Local conditions. Its named alias adds the use
            // domain (and containing region); other alias rows govern its assumption basis.
            if position==1 || flow.alias.is_some() && (position==2 || position==7) {
                let premise=conditions::Diagram::from_records(need(&d.conditions,q.condition)?,&nodes)?;
                let admitted=condition.admitted_binary(&premise,conditions::BooleanOperation::Conjunction,&self.budget)
                    .map_err(|error|match error {
                        conditions::DiagramAdmissionError::Resource(error)=>error,
                        conditions::DiagramAdmissionError::Boundary(boundary)=>invalid(format!("Structural control qualification boundary: {boundary:?}")),
                    })?;
                let (result,charge)=admitted.into_parts();condition=result;condition_charge=Some(charge);
            }
        }
        let _condition_charge=condition_charge;
        if flow.alias.is_some() {
            let use_q=self.qualification(ids[2],invocation,site.source)?;
            let mut domain=conditions::Diagram::from_records(need(&d.conditions,use_q.condition)?,&nodes)?;
            let mut domain_charge=None;
            if ids.len()>7 {
                let region_q=self.qualification(ids[7],invocation,site.source)?;
                let region=conditions::Diagram::from_records(need(&d.conditions,region_q.condition)?,&nodes)?;
                let admitted=domain.admitted_binary(&region,conditions::BooleanOperation::Conjunction,&self.budget)
                    .map_err(|error|match error {
                        conditions::DiagramAdmissionError::Resource(error)=>error,
                        conditions::DiagramAdmissionError::Boundary(boundary)=>invalid(format!("Structural alias qualification boundary: {boundary:?}")),
                    })?;
                let (result,charge)=admitted.into_parts(); domain=result; domain_charge=Some(charge);
            }
            let _domain_charge=domain_charge;
            let reaching_q=self.qualification(ids[3],invocation,site.source)?;
            let reaching=conditions::Diagram::from_records(need(&d.conditions,reaching_q.condition)?,&nodes)?;
            if domain.is_false() || !domain.implies(&reaching).map_err(|boundary|invalid(format!("Structural alias condition boundary: {boundary:?}")))? {
                return Err(invalid("Structural control alias is not covered by its reaching condition"));
            }
        }
        let basis=assumptions::ResolvedAssumptions::union(bases.iter())?;
        let expected=AssertionQualification {context:invocation.context,
            scope:source::CoverageScope::Artifact {artifact:site.source}.id(),condition:condition.id(),
            modality,approximation,assumptions:basis.set.id()};
        let q=self.qualification(flow.qualification,invocation,site.source)?;
        let source=need(&d.alternative_sources,alternative.source)?;
        modality=if matches!(source,CallAlternativeSource::DerivedDispatch {..}) {modality.weakest(Modality::Candidate)} else {modality};
        if q!=&expected || flow.modality!=modality || flow.conditional!=(condition.id()!=conditions::Diagram::always().id()) {
            return Err(invalid("Structural control qualification promotes or changes its actual premises"));
        }
        Ok(q)
    }
    fn verify(&self) -> Result<(),ModelError> {
        let d = &self.data;
        for row in d.conclusions.iter() {
            let frame = need(&d.frames,row.frame)?;
            let invocation = need(&d.invocations,row.invocation)?;
            let (source_frame,subject,kind,method,modality,approximation) = match need(&d.sources,row.source)? {
                ConclusionSource::Public {candidate} => {
                    let source = need(&d.public,*candidate)?;
                    (source.frame,source.entity,analysis::policy::FindingKind::PublicAlias,analysis::AnalysisMethod::Delegation,Modality::Candidate,Approximation::Over)
                }
                ConclusionSource::Path {path} => {
                    let source = need(&d.paths,*path)?;
                    let reach = need(&d.reaches,source.reach)?;
                    let traversal = need(&d.traversals,reach.traversal)?;
                    let mut modality = Modality::Definite;
                    let mut approximation = Approximation::Exact;
                    let mut count = 0;
                    let mut previous = None;
                    let _ordered = self.budget.reserve("structural-source-path-members",
                        d.steps.len().checked_mul(2*size_of::<&PathStep>()).ok_or_else(||invalid("Structural member allocation overflow"))?)?;
                    let mut steps = d.steps.iter().filter(|step|step.path==source.id()).collect::<Vec<_>>();
                    steps.sort_by_key(|step|step.ordinal);
                    for step in steps {
                        if step.ordinal != count || previous.is_some_and(|target|target!=step.source) {
                            return Err(invalid("Structural ordered path member domain differs"));
                        }
                        if count==0 && step.source != traversal.seed {return Err(invalid("Structural path seed differs"));}
                        let (m,a) = self.step(step,invocation)?;
                        modality=modality.weakest(m); approximation=approximation.join(a);
                        previous=Some(step.target); count+=1;
                    }
                    if count != source.length || previous.is_some_and(|target|target!=reach.target) {
                        return Err(invalid("Structural ordered path member domain differs"));
                    }
                    let kind = match reach.kind { ReachKind::Direct=>analysis::policy::FindingKind::DirectDelegation,
                        ReachKind::BoundedPath=>analysis::policy::FindingKind::BoundedDelegationPath,
                        _=>analysis::policy::FindingKind::ImplementationBoundary };
                    (traversal.frame,traversal.seed,kind,analysis::AnalysisMethod::Delegation,modality,approximation)
                }
                ConclusionSource::Unresolved {event} => {
                    let source=need(&d.unresolved,*event)?;
                    let traversal=need(&d.traversals,source.traversal)?;
                    let event=need(&d.events,source.event)?;
                    let assessment=need(&d.assessments,source.assessment)?;
                    if event.context!=invocation.context || event.site!=source.site || assessment.event!=event.id() {
                        return Err(invalid("Structural unresolved source crosses event frame"));
                    }
                    (traversal.frame,traversal.seed,analysis::policy::FindingKind::IncompleteResolution,analysis::AnalysisMethod::Delegation,Modality::Candidate,Approximation::Over)
                }
                ConclusionSource::Stop {traversal} => {
                    let source=need(&d.traversals,*traversal)?;
                    if source.stop.is_none() {return Err(invalid("Structural stop source has no actual boundary"));}
                    (source.frame,source.seed,analysis::policy::FindingKind::TraversalStop,analysis::AnalysisMethod::Delegation,Modality::Definite,Approximation::Exact)
                }
                ConclusionSource::Usage {score} => {
                    let source=need(&d.usage,*score)?;
                    (source.frame,source.target,analysis::policy::FindingKind::DirectUsage,analysis::AnalysisMethod::DirectUsage,Modality::Candidate,Approximation::Over)
                }
                ConclusionSource::Handoff {group} => {
                    let source=need(&d.groups,*group)?;
                    (source.frame,source.seed,analysis::policy::FindingKind::Handoff,analysis::AnalysisMethod::Handoffs,source.producer_modality.weakest(source.consumer_modality),Approximation::Over)
                }
                ConclusionSource::Forward {path} => {
                    let source=need(&d.control_paths,*path)?;
                    let traversal=need(&d.control_traversals,source.traversal)?;
                    if source.length<=0 {return Err(invalid("Structural forwarding source has no actual path"));}
                    let mut modality=Modality::Definite;
                    let mut approximation=Approximation::Exact;
                    let mut count=0;
                    let _ordered = self.budget.reserve("structural-source-control-members",
                        d.control_steps.len().checked_mul(2*size_of::<&controls::ControlStep>()).ok_or_else(||invalid("Structural member allocation overflow"))?)?;
                    let mut steps = d.control_steps.iter().filter(|step|step.path==source.id()).collect::<Vec<_>>();
                    steps.sort_by_key(|step|step.ordinal);
                    for step in steps {
                        if step.ordinal!=count {return Err(invalid("Structural ordered control member domain differs"));}
                        let flow=need(&d.flows,step.flow)?;
                        if flow.frame!=traversal.frame {return Err(invalid("Structural control member crosses frame"));}
                        let q=self.flow(flow,invocation)?;
                        modality=modality.weakest(flow.modality).weakest(q.modality);
                        approximation=approximation.join(q.approximation); count+=1;
                    }
                    if count!=source.length {return Err(invalid("Structural ordered control member domain differs"));}
                    (traversal.frame,traversal.seed,analysis::policy::FindingKind::Forwarding,analysis::AnalysisMethod::Controls,modality,approximation)
                }
                ConclusionSource::Literal {argument} => {
                    let source=need(&d.literals,*argument)?;
                    if !d.public.iter().any(|candidate|candidate.frame==source.frame && candidate.entity==source.caller && candidate.in_subsystem) {
                        return Err(invalid("Structural literal source is outside admitted public scope"));
                    }
                    (source.frame,source.caller,analysis::policy::FindingKind::TransformedArgument,analysis::AnalysisMethod::Controls,Modality::Candidate,Approximation::Over)
                }
                ConclusionSource::Raise {observation} => {
                    let source=need(&d.raises,*observation)?;
                    let path=need(&d.control_paths,source.path)?;
                    let traversal=need(&d.control_traversals,path.traversal)?;
                    (traversal.frame,traversal.seed,analysis::policy::FindingKind::ConditionalRaise,analysis::AnalysisMethod::Controls,Modality::Candidate,Approximation::Over)
                }
                ConclusionSource::Unfollowed {observation} => {
                    let source=need(&d.unfollowed,*observation)?;
                    let path=need(&d.control_paths,source.path)?;
                    let traversal=need(&d.control_traversals,path.traversal)?;
                    (traversal.frame,traversal.seed,analysis::policy::FindingKind::UnfollowedArgument,analysis::AnalysisMethod::Controls,Modality::Candidate,Approximation::Over)
                }
                ConclusionSource::ControlStop {traversal} => {
                    let source=need(&d.control_traversals,*traversal)?;
                    if source.stop.is_none() {return Err(invalid("Structural control stop source has no actual boundary"));}
                    (source.frame,source.seed,analysis::policy::FindingKind::TraversalStop,analysis::AnalysisMethod::Controls,Modality::Definite,Approximation::Exact)
                }
            };
            let expected_invocation=match method {
                analysis::AnalysisMethod::Delegation=>frame.invocation,
                analysis::AnalysisMethod::DirectUsage=>frame.usage_invocation,
                analysis::AnalysisMethod::Handoffs=>frame.handoff_invocation,
                analysis::AnalysisMethod::Controls=>frame.control_invocation,
                _=>return Err(invalid("Structural source has a foreign method")),
            };
            if row.frame!=source_frame || row.invocation!=expected_invocation || row.subject!=subject || row.kind!=kind
                || need(&d.definitions,invocation.definition)?.method!=method
                || (method==analysis::AnalysisMethod::Controls && !frame.controls_requested) {
                return Err(invalid("Structural conclusion source/frame/subject/method differs"));
            }
            let expected=conclusions::qualification(invocation,modality,approximation);
            if row.qualification()!=expected.id() || need(&d.qualifications,row.qualification())? != &expected {
                return Err(invalid("Structural conclusion qualification promotes or changes its actual source"));
            }
        }
        Ok(())
    }
}
impl InvariantCheck for SourceCheck {
    fn execution_scope(&self) -> Option<ExecutionScope> { Some(Self::scope()) }
    fn visit(&mut self,name:&str,batch:&arrow_array::RecordBatch)->Result<(),ModelError> {
        let source=self.data.visit(name,batch)?;
        let scope=self.scopes.visit(name,batch)?;
        let basis=self.assumptions.visit(name,batch)?;
        if source || scope || basis { Ok(()) } else { Err(invalid("undeclared Structural source fidelity input")) }
    }
    fn finish(self:Box<Self>)->Result<(),ModelError> { self.verify() }
}
pub(crate) fn invariants() -> Vec<Invariant> {
    vec![
        Invariant {revision:1,name:"structural_frame_fidelity",purpose:InvariantPurpose::Admission,
            inputs:frame_inputs(),create:std::sync::Arc::new(|b|Box::new(FrameCheck::new(b)))},
        Invariant {revision:1,name:"structural_source_fidelity",purpose:InvariantPurpose::Admission,
            inputs:SourceCheck::inputs(),create:std::sync::Arc::new(|b|Box::new(SourceCheck::new(b)))},
    ]
}
pub(crate) fn frame_refs()->Vec<&'static str> {vec!["structural_replay","structural_frame_fidelity"]}
pub(crate) fn source_refs()->Vec<&'static str> {vec!["structural_source_fidelity"]}
