//! Reconcile stored structural outputs by replaying the same canonical transformations.
use super::*;
use crate::domain::{
    analysis::{delegation, settings::AnalyticsConfiguration, structural as publication, usage},
    normalized::{Rows, callables::EffectiveCallableAssessment, event_normalization::EventOutput},
    projection::{normalization::ProjectionData, snapshot::MaterializedGraph},
    resources::ResourceBudget,
    *,
};
pub(super) fn invalid(s: impl Into<String>) -> ModelError {
    ModelError::Invalid(s.into())
}
pub(super) fn need<R: Record>(rows: &Rows<R>, id: Id<R>) -> Result<&R, ModelError> {
    rows.get(id)
        .ok_or_else(|| invalid(format!("structural predecessor absent: {}", R::NAME)))
}
pub struct Data {
    pub projection: ProjectionData,
    pub assumptions: assumptions::AssumptionIndex,
    pub(crate) ownership: ownership::ScopeIndex,
    pub events: EventOutput,
    pub handoffs: super::handoffs::Data,
    pub controls: super::controls::Data,
    pub uses: Rows<input::ArtifactUse>,
    pub members: Rows<catalog::CatalogMember>,
    pub callables: Rows<catalog::CatalogCallable>,
    pub core_links: Rows<catalog::CatalogMemberInvocation>,
    pub core_invocations: Rows<analysis::catalog_core::AnalysisInvocation>,
    pub assessments: Rows<normalized::callables::EffectiveCallableAssessment>,
}
impl Data {
    pub fn new(b: &ResourceBudget) -> Self {
        Self {
            handoffs: super::handoffs::Data::new(b),
            controls: super::controls::Data::new(b),
            projection: ProjectionData::new(b),
            assumptions: assumptions::AssumptionIndex::new(b),
            ownership: ownership::ScopeIndex::new(b, "structural scopes"),
            events: EventOutput::new(b),
            uses: Rows::new(b),
            members: Rows::new(b),
            callables: Rows::new(b),
            core_links: Rows::new(b),
            core_invocations: Rows::new(b),
            assessments: Rows::new(b),
        }
    }
    pub fn visit(
        &mut self,
        name: &str,
        batch: &arrow_array::RecordBatch,
    ) -> Result<bool, ModelError> {
        let assumptions = self.assumptions.visit(name, batch)?;
        let ownership = self.ownership.visit(name, batch)?;
        let a = self.projection.visit(name, batch)?;
        let b = self.events.visit(name, batch)?;
        let c = self.handoffs.visit(name, batch)?;
        let d = self.controls.visit(name, batch)?;
        macro_rules! rows {($($field:ident:$ty:ty,)*)=>{$(if name==<$ty>::NAME{self.$field.decode(batch)?;return Ok(true);})*};}
        rows! {uses:input::ArtifactUse,members:catalog::CatalogMember,callables:catalog::CatalogCallable,core_links:catalog::CatalogMemberInvocation,core_invocations:analysis::catalog_core::AnalysisInvocation,assessments:EffectiveCallableAssessment,}
        Ok(a || b || c || d || assumptions || ownership)
    }
}
fn in_roots(settings: &AnalyticsConfiguration, path: &str) -> bool {
    settings
        .public_roots
        .iter()
        .any(|root| path == root || path.starts_with(&format!("{root}.")))
}
pub fn produce(
    data: &Data,
    frame: &StructuralFrame,
    invocation: &publication::AnalysisInvocation,
    settings: &AnalyticsConfiguration,
    invocation_graph: &MaterializedGraph,
    definition_graph: &MaterializedGraph,
    budget: &ResourceBudget,
) -> Result<Output, ModelError> {
    settings.validate()?;
    if frame.invocation != invocation.id() || frame.configuration != settings.id() {
        return Err(invalid("structural frame changes invocation/configuration"));
    }
    let key = invocation_graph.key();
    if key.input != invocation.input
        || key.context != invocation.context
        || key.name != projection::ProjectionName::CallableInvocation
    {
        return Err(invalid("structural invocation graph has foreign frame"));
    }
    let key = definition_graph.key();
    if key.input != invocation.input
        || key.context != invocation.context
        || key.name != projection::ProjectionName::DefinitionContainment
    {
        return Err(invalid("structural definition graph has foreign frame"));
    }
    let mut output = Output::new(budget);
    output.frames.insert(frame.clone())?;
    let mut charge = charged::StateCharge::new(budget, "structural-selected-scope");
    let mut subsystem = charged::ChargedSet::default();
    for entity in data.projection.refs.iter() {
        let normalized::entities::EntityRef::Callable { callable } = entity else {
            continue;
        };
        let normalized::entities::CallableEntity::Source { declaration, .. } =
            need(&data.projection.callables, *callable)?
        else {
            continue;
        };
        let source = need(&data.projection.occurrences, *declaration)?.source;
        let artifact = need(&data.projection.artifacts, source)?;
        if artifact.input != invocation.input {
            continue;
        }
        let mut modules = data
            .projection
            .modules
            .iter()
            .filter(|m| m.source == source);
        let module = modules
            .next()
            .ok_or_else(|| invalid("source callable has no native module"))?;
        if modules.next().is_some() {
            return Err(invalid("source callable has ambiguous native modules"));
        }
        if !settings.in_subsystem(&module.qualified_name) {
            continue;
        }
        let roles = data.uses.iter().filter(|u| {
            u.input == invocation.input
                && u.artifact == source
                && u.role == input::SourceRole::Release
        });
        for role in roles {
            subsystem.insert(&mut charge, entity.id())?;
            output.scope.insert(ScopeMember {
                frame: frame.id(),
                entity: entity.id(),
                module: module.id(),
                declaration: *declaration,
                role: role.id(),
            })?;
        }
    }
    let mut public = charged::ChargedSet::default();
    for row in data.callables.iter() {
        let member = need(&data.members, row.member)?;
        if member.input != invocation.input {
            continue;
        }
        let applicable = data
            .core_links
            .iter()
            .filter(|link| link.member == member.id())
            .any(|link| {
                data.core_invocations
                    .get(link.invocation)
                    .is_some_and(|i| i.input == invocation.input && i.context == invocation.context)
            });
        if !applicable {
            continue;
        }
        let assessment = need(&data.assessments, row.assessment)?;
        if assessment.context != invocation.context {
            continue;
        }
        let entity = normalized::entities::EntityRef::Callable {
            callable: assessment.callable,
        }
        .id();
        need(&data.projection.refs, entity)?;
        let access = need(&data.projection.modules, member.access)?;
        if need(&data.projection.artifacts, access.source)?.input != member.input {
            return Err(invalid(
                "catalog public slot is foreign to structural input",
            ));
        }
        let bytes = member
            .path
            .iter()
            .try_fold(access.qualified_name.len(), |n, s| {
                n.checked_add(s.len() + 1)
            })
            .ok_or_else(|| invalid("structural public path allocation overflow"))?;
        let _path_charge = budget.reserve(
            "structural-public-path",
            bytes
                .checked_mul(2)
                .ok_or_else(|| invalid("path allocation overflow"))?,
        )?;
        let mut path = String::with_capacity(bytes);
        path.push_str(&access.qualified_name);
        for part in &member.path {
            path.push('.');
            path.push_str(part);
        }
        if !in_roots(settings, &path) {
            continue;
        }
        let inside = subsystem.contains(&entity);
        if inside {
            public.insert(&mut charge, entity)?;
        }
        output.public.insert(PublicCandidate {
            frame: frame.id(),
            callable: row.id(),
            member: member.id(),
            entity,
            path: path.clone(),
            configured_ordinal: settings
                .configured_seeds
                .iter()
                .position(|p| p == &path)
                .map(|n| n as i64),
            in_subsystem: inside,
        })?;
    }
    for (ordinal, path) in settings.configured_seeds.iter().enumerate() {
        let candidates = output
            .public
            .iter()
            .filter(|r| r.configured_ordinal == Some(ordinal as i64))
            .count();
        let inside = output
            .public
            .iter()
            .filter(|r| r.configured_ordinal == Some(ordinal as i64) && r.in_subsystem)
            .count();
        output.configured.insert(ConfiguredSeed {
            frame: frame.id(),
            ordinal: ordinal as i64,
            path: path.clone(),
            candidates: candidates as i64,
            in_subsystem: inside as i64,
        })?;
    }
    charge.grow(
        (public.len() + subsystem.len())
            .checked_mul(size_of::<Id<normalized::entities::EntityRef>>())
            .ok_or_else(|| invalid("structural selection allocation overflow"))?,
    )?;
    let selected = public.iter().copied().collect::<Vec<_>>();
    let selected_scope = subsystem.iter().copied().collect::<Vec<_>>();
    let counts = usage::Inputs {
        input: invocation.input,
        context: invocation.context,
        events: &data.events,
        targets: &data.projection.targets,
        artifacts: &data.projection.artifacts,
        uses: &data.uses,
        occurrences: &data.projection.occurrences,
        entities: &data.projection.refs,
    }
    .count(&selected, budget)?;
    for site in counts.sites() {
        output.usage_sites.insert(UsageSite {
            frame: frame.id(),
            site: site.site,
            targets: site.targets.len() as i64,
            complete: site.complete,
            uncertain: site.uncertain,
        })?;
    }
    for evidence in counts.evidence() {
        let site = output
            .usage_sites
            .iter()
            .find(|s| s.site == evidence.site)
            .ok_or_else(|| invalid("usage evidence site absent"))?
            .id();
        output.usage_evidence.insert(UsageEvidence {
            site,
            event: evidence.event,
            policy: evidence.policy,
            admission: evidence.admission,
            alternative: evidence.alternative,
            target: evidence.target,
        })?;
    }
    for score in counts.scores() {
        output.usage_scores.insert(UsageScore {
            frame: frame.id(),
            target: score.target,
            share: score.share,
            contributing_sites: score.contributing_sites as i64,
        })?;
    }
    let inputs = delegation::Inputs {
        invocation: invocation_graph,
        definition: definition_graph,
        data: &data.projection,
        uses: &data.uses,
    };
    for seed in &selected {
        let result = inputs.traverse(*seed, &selected_scope, settings.bounds()?, budget)?;
        let traversal = Traversal {
            frame: frame.id(),
            seed: *seed,
            stop: result.stop().map(|s| match s {
                delegation::Stop::Depth => TraversalStop::Depth,
                delegation::Stop::Vertices => TraversalStop::Vertices,
                delegation::Stop::Arcs => TraversalStop::Arcs,
            }),
            partial: result.partial(),
            examined_vertices: result.vertices_examined() as i64,
            examined_arcs: result.arcs_examined() as i64,
        };
        for reached in result.reached() {
            let reach = Reach {
                traversal: traversal.id(),
                target: reached.target,
                kind: match reached.kind {
                    delegation::Kind::Direct => ReachKind::Direct,
                    delegation::Kind::BoundedPath => ReachKind::BoundedPath,
                    delegation::Kind::Boundary(b) => match b {
                        delegation::Boundary::External => ReachKind::ExternalBoundary,
                        delegation::Boundary::Synthetic => ReachKind::SyntheticBoundary,
                        delegation::Boundary::Dependency => ReachKind::DependencyBoundary,
                        delegation::Boundary::Subsystem => ReachKind::SubsystemBoundary,
                    },
                },
                depth: reached.depth as i64,
                witnesses_omitted: reached.witnesses_omitted,
            };
            for (ordinal, steps) in reached.paths.iter().enumerate() {
                let path = Path {
                    reach: reach.id(),
                    ordinal: ordinal as i64,
                    length: steps.len() as i64,
                };
                for (ordinal, step) in steps.iter().enumerate() {
                    let (arc, evidence) = step_records(&mut output, step)?;
                    output.steps.insert(PathStep {
                        path: path.id(),
                        ordinal: ordinal as i64,
                        arc,
                        source: step.source,
                        target: step.target,
                        evidence,
                    })?;
                }
                output.paths.insert(path)?;
            }
            output.reaches.insert(reach)?;
        }
        for unresolved in result.unresolved() {
            let row = UnresolvedEvent {
                traversal: traversal.id(),
                event: unresolved.event,
                assessment: unresolved.assessment,
                site: unresolved.site,
                depth: unresolved.depth as i64,
            };
            for (ordinal, step) in unresolved.caller_path.iter().enumerate() {
                let (arc, evidence) = step_records(&mut output, step)?;
                output.unresolved_steps.insert(UnresolvedStep {
                    event: row.id(),
                    ordinal: ordinal as i64,
                    arc,
                    source: step.source,
                    target: step.target,
                    evidence,
                })?;
            }
            output.unresolved.insert(row)?;
        }
        output.traversals.insert(traversal)?;
    }
    if frame.controls_requested {
        super::controls::produce(
            &data.controls,
            data,
            frame,
            invocation,
            settings,
            &mut output,
            budget,
        )?;
    }
    super::handoffs::produce(
        &data.handoffs,
        data,
        frame,
        invocation,
        settings,
        &mut output,
        budget,
    )?;
    super::conclusions::produce(&mut output, invocation, frame, data, budget)?;
    Ok(output)
}
fn step_records(
    out: &mut Output,
    step: &delegation::Step,
) -> Result<(Id<ArcSource>, Id<StepEvidence>), ModelError> {
    let arc = match step.arc {
        projection::ArcId::Invocation(alternative) => ArcSource::Invocation { alternative },
        projection::ArcId::Definition(alternative) => ArcSource::Definition { alternative },
        projection::ArcId::SourceDefinition(ownership) => ArcSource::SourceDefinition { ownership },
        _ => return Err(invalid("structural path has an inadmissible arc role")),
    };
    let evidence = match step.evidence {
        delegation::StepEvidence::Call {
            event,
            site,
            phase,
            qualification,
            modality,
            derived_dispatch,
        } => StepEvidence::Call {
            event,
            site,
            phase,
            qualification,
            modality,
            derived_dispatch,
        },
        delegation::StepEvidence::Declaration {
            owner,
            declaration,
            callable,
        } => StepEvidence::Declaration {
            owner,
            declaration,
            callable,
        },
    };
    Ok((out.arcs.insert(arc)?, out.evidence.insert(evidence)?))
}
impl Data {
    pub fn validation_inputs() -> Vec<ValidationInput> {
        let mut inputs = ProjectionData::validation_inputs();
        inputs.extend(assumptions::AssumptionIndex::inputs());
        inputs.extend(ownership::ScopeIndex::inputs());
        inputs.extend(EventOutput::validation_inputs());
        inputs.extend(super::handoffs::Data::inputs());
        inputs.extend(super::controls::Data::inputs());
        inputs.extend([
            ValidationInput::of::<input::ArtifactUse>(&["id"]),
            ValidationInput::of::<catalog::CatalogMember>(&["id"]),
            ValidationInput::of::<catalog::CatalogCallable>(&["id"]),
            ValidationInput::of::<catalog::CatalogMemberInvocation>(&["id"]),
            ValidationInput::of::<analysis::catalog_core::AnalysisInvocation>(&["id"]),
            ValidationInput::of::<EffectiveCallableAssessment>(&["id"]),
        ]);
        for input in &mut inputs {
            if stages::is_vocabulary(input.name()) {
                *input = input.clone().at_epoch(stages::PublicationBoundary::Local);
            }
        }
        inputs.sort_by_key(|i| (i.name(), i.prefix()));
        inputs.dedup_by_key(|i| (i.name(), i.prefix()));
        inputs
    }
}
pub fn definition(
    settings: &AnalyticsConfiguration,
    method: analysis::AnalysisMethod,
) -> Result<(analysis::MethodParameters, analysis::AnalysisDefinition), ModelError> {
    settings.validate()?;
    if !matches!(
        method,
        analysis::AnalysisMethod::Delegation
            | analysis::AnalysisMethod::DirectUsage
            | analysis::AnalysisMethod::Handoffs
            | analysis::AnalysisMethod::Controls
    ) {
        return Err(invalid("unimplemented structural method"));
    }
    let bounded = method != analysis::AnalysisMethod::DirectUsage;
    let parameters = analysis::MethodParameters {
        depth: bounded.then_some(settings.depth),
        proof_steps: bounded.then_some(settings.witnesses),
        work: bounded.then_some(settings.arcs),
        members: bounded.then_some(settings.vertices),
        seed: None,
        iterations: None,
        threshold: None,
        resolution: None,
        damping: None,
        model_catalog: None,
    };
    let mut hash = KeySink::new("structural-conversion-v1");
    settings.id().encode(&mut hash);
    method.encode(&mut hash);
    ContentHash::of(include_bytes!("build.rs")).encode(&mut hash);
    ContentHash::of(include_bytes!("controls.rs")).encode(&mut hash);
    ContentHash::of(include_bytes!("outcomes.rs")).encode(&mut hash);
    ContentHash::of(include_bytes!("qualifications.rs")).encode(&mut hash);
    ContentHash::of(include_bytes!("handoffs.rs")).encode(&mut hash);
    ContentHash::of(include_bytes!("conclusions.rs")).encode(&mut hash);
    ContentHash::of(include_bytes!("../analysis/delegation.rs")).encode(&mut hash);
    ContentHash::of(include_bytes!("../analysis/usage.rs")).encode(&mut hash);
    let row = analysis::AnalysisDefinition {
        method,
        parameters: parameters.id(),
        semantic_version: hash.finish(),
        interpretation: analysis::Interpretation::Structural,
    };
    Ok((parameters, row))
}
// One graph per frame is hydrated in stored replay. The runtime borrows its collection's graph.
pub(super) fn hydrate(
    graphs: &projection::normalization::ProjectionOutput,
    assessment: Id<projection::ProjectionSourceAssessment>,
    budget: &ResourceBudget,
) -> Result<MaterializedGraph, ModelError> {
    let assessment = need(&graphs.assessments, assessment)?;
    let mut headers = graphs
        .snapshots
        .iter()
        .filter(|s| s.assessment == assessment.id());
    let header = headers
        .next()
        .ok_or_else(|| invalid("structural snapshot absent"))?;
    if headers.next().is_some() {
        return Err(invalid("structural snapshot ambiguous"));
    }
    projection::snapshot::hydrate(header, assessment, &graphs.chunks, budget)
}

pub fn invariants() -> Vec<Invariant> {
    super::frames::invariants()
}
/// Declare only the operational structural owner. Unused generic proof-sum arms do not invent
/// producers; concrete replay and source/publication checks declare every consumed predecessor.
pub fn stage(
    profile: stages::Profile,
    settings: &AnalyticsConfiguration,
    model: &ValidatedModel,
    order: &stages::PublicationOrder,
) -> Result<stages::Stage, ModelError> {
    use stages::*;
    settings.validate()?;
    let mut outputs = super::relations()
        .iter()
        .map(RelationUse::of_relation)
        .collect::<Vec<_>>();
    macro_rules! output {($($ty:ty),*)=>{$(outputs.push(RelationUse::of::<$ty>());)*};}
    outputs.extend(publication::publication_relations().iter().map(RelationUse::of_relation));
    output!(assertion::AssertionQualification,conditions::Condition,conditions::ConditionNode,assumptions::AssumptionSet,assumptions::AssumptionSetMember);
    let own = outputs
        .iter()
        .filter(|r| !is_vocabulary(r.name()))
        .map(|r| r.name())
        .collect::<std::collections::BTreeSet<_>>();
    let mut requested = Data::validation_inputs();
    requested.extend(super::frames::Context::validation_inputs());
    requested.extend(analysis::expected::inputs(
        analysis::AnalysisMethod::Delegation,
    ));
    requested.extend(analysis::expected::inputs(
        analysis::AnalysisMethod::DirectUsage,
    ));
    requested.extend(analysis::expected::inputs(
        analysis::AnalysisMethod::Handoffs,
    ));
    requested.extend(analysis::expected::inputs(
        analysis::AnalysisMethod::Controls,
    ));
    if profile == Profile::Catalog {
        let inherited = ProjectionData::validation_inputs()
            .iter()
            .map(ValidationInput::name)
            .collect::<std::collections::BTreeSet<_>>();
        let mut entry = conditions::entry::EntryData::validation_inputs();
        entry.extend(super::controls::Data::inputs());
        requested.retain(|input| {
            inherited.contains(input.name()) || !entry.iter().any(|e| e.name() == input.name())
        });
    }
    requested.push(ValidationInput::of::<analysis::ProjectionDefinition>(&[
        "id",
    ]));
    requested.retain(|input| !own.contains(input.name()));
    let inputs = dependency_closure::DependencyClosure::stage_grants(
        model, requested, &outputs, PublicationBoundary::Local,
        dependency_closure::LowerLayerPolicy::OmitInferredOrdinaryFacts, order,
    )?;
    let mut key = KeySink::new("structural-stage");
    settings.id().encode(&mut key);
    for method in methods() {
        definition(settings, method)?.1.id().encode(&mut key);
    }
    Ok(Stage {
        name: "analyze_structural",
        inputs,
        outputs,
        contributes: vec![],
        coverage: vec![],
        profiles: vec![profile],
        effect: Effect::Pure,
        code: ContentHash::of(include_bytes!("build.rs")),
        configuration: key.finish(),
    })
}

pub fn methods() -> [analysis::AnalysisMethod; 4] {
    [
        analysis::AnalysisMethod::Delegation,
        analysis::AnalysisMethod::DirectUsage,
        analysis::AnalysisMethod::Handoffs,
        analysis::AnalysisMethod::Controls,
    ]
}
