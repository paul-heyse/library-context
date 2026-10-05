//! Request-time public-name routes. Native qualified import names select slots only after
//! canonical alias/module and public exposure/entity correspondence; names never identify entities.
use crate::domain::{
    catalog::*,
    charged::StateCharge,
    normalized::{Rows, entities::*, links::*},
    resources::ResourceBudget,
    ruff::*,
    selection::classification::ClassificationData,
    symbols::*,
    syntax::*,
    *,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

macro_rules! inputs {($($f:ident:$ty:ty,)*)=>{
 pub struct RouteData {$(pub $f:Rows<$ty>,)*}
 impl RouteData {pub fn new(b:&ResourceBudget)->Self {Self{$($f:Rows::new(b),)*}}pub fn visit(&mut self,n:&str,b:&arrow_array::RecordBatch)->Result<bool,ModelError>{$(if n==<$ty>::NAME{self.$f.decode(b)?;return Ok(true);})*Ok(false)}}
};}
inputs! {
 imports:ImportAliasObservation,
 assessments:ImportModuleAssessment,
 candidates:ImportModuleCandidate,
 resolutions:ModuleResolutionObservation,
 resolution_supports:ModuleResolutionSupport,
 native_bindings:RuffBindingObservation,
 native_supports:RuffBindingSupport,
 names:PublicNameObservation,
 name_supports:PublicNameSupport,
 origins:ExportOrigin,
 classes:ClassEntity,
 runs:attribution::ProviderRun,
 providers:attribution::Provider,
 surfaces:assertion::ProviderSurface,
 evidence:assertion::Evidence,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum RouteStop {
    Declaration,
    UnresolvedNameCorrespondence,
    OutsideCapturedModule,
    Ambiguous,
    Cycle,
    Frontier,
    CandidateOnly,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum RouteHop {
    PublicExposure {
        catalog_candidate: Id<CatalogCandidate>,
        exposure: Id<PublicExposure>,
        name: Id<PublicNameObservation>,
        support: Id<PublicNameSupport>,
    },
    Import {
        alias: Id<ImportAliasObservation>,
        native_name: Id<RuffBindingObservation>,
        native_support: Id<RuffBindingSupport>,
        assessment: Id<ImportModuleAssessment>,
        candidate: Id<ImportModuleCandidate>,
        resolution: Id<ModuleResolutionObservation>,
        support: Id<ModuleResolutionSupport>,
        from: Id<source::Module>,
        to: Id<source::Module>,
        imported_name: String,
        status: ResolutionStatus,
        reason: LinkReason,
    },
    MemberPath {
        path: Id<CatalogPath>,
        parent: Id<CatalogCandidate>,
        declaration: Id<syntax::DeclarationObservation>,
        binding: Id<lexical::BindingObservation>,
        entity: Id<EntityRef>,
        disposition: PublicPathDisposition,
    },
    Reference {
        alias: Id<CatalogAlias>,
        reference: Id<ReferenceEntityCandidate>,
        assessment: Id<ReferenceEntityAssessment>,
        target: Id<ReferenceEntityTarget>,
        entity: Id<EntityRef>,
        status: ResolutionStatus,
        reason: LinkReason,
    },
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct AccessRoute {
    pub member: Id<CatalogMember>,
    pub context: Id<attribution::AnalysisContext>,
    pub candidate: Id<CatalogCandidate>,
    pub exposure: Id<PublicExposure>,
    pub target: Option<Id<EntityRef>>,
    pub hops: Vec<RouteHop>,
    pub stop: RouteStop,
    /// Captured source/stub universe only; candidate public exposure never becomes definite binding.
    pub publicity: PublicPathKnowledge,
    pub public_resolution: ResolutionStatus,
    pub captured_modules: u64,
    pub declaration_artifact: Option<Id<source::SourceArtifact>>,
    pub stub: Option<bool>,
    pub external_consumers_unknown: bool,
    pub installation_requirements_unknown: bool,
    pub omitted_frontiers: u64,
}
pub struct RouteResult {
    pub routes: Vec<AccessRoute>,
    pub partial: bool,
    pub(crate) _charge: StateCharge,
}
fn invalid(s: &str) -> ModelError {
    ModelError::Invalid(s.into())
}
fn need<R: Record>(rows: &Rows<R>, id: Id<R>) -> Result<&R, ModelError> {
    rows.get(id)
        .ok_or_else(|| invalid("access route canonical premise absent"))
}
fn exact(
    d: &ClassificationData,
    id: Id<assertion::AssertionQualification>,
    context: Id<attribution::AnalysisContext>,
) -> bool {
    d.source.core.qualifications.get(id).is_some_and(|q| {
        q.context == context
            && q.approximation == assertion::Approximation::Exact
            && q.modality == attribution::Modality::Definite
            && q.condition == conditions::Diagram::always().id()
            && q.assumptions == assumptions::AssumptionSet::empty_id()
    })
}
fn supported<S: assertion::Support>(
    d: &RouteData,
    s: &S,
    context: Id<attribution::AnalysisContext>,
    family: attribution::FactFamily,
    tool: &str,
) -> bool {
    let Some(a) = s.attribution() else {
        return false;
    };
    let (Some(run), Some(surface)) = (d.runs.get(a.run), d.surfaces.get(a.surface)) else {
        return false;
    };
    run.context == context
        && run.provider == surface.provider
        && surface.family == family
        && d.providers
            .get(run.provider)
            .is_some_and(|p| p.tool == tool)
        && a.origin == attribution::Origin::AnalyzerAssertion
        && a.mode == attribution::ExtractionMode::NativeTraversal
        && a.fidelity == attribution::Fidelity::NativeStructural
        && d.evidence.get(a.evidence).is_some()
}
fn source_module(
    d: &ClassificationData,
    extra: &RouteData,
    entity: Id<EntityRef>,
) -> Result<Option<Id<source::Module>>, ModelError> {
    let occurrence = match need(&d.source.core.refs, entity)? {
        EntityRef::Module { module } => return Ok(Some(*module)),
        EntityRef::Callable { callable } => match need(&d.source.core.source_callables, *callable)?
        {
            CallableEntity::Source { declaration, .. } => Some(*declaration),
            _ => None,
        },
        EntityRef::Occurrence { occurrence } => Some(*occurrence),
        EntityRef::Class { class } => match need(&extra.classes, *class)? {
            ClassEntity::Source { declaration } => Some(*declaration),
            _ => None,
        },
        _ => None,
    };
    let Some(occurrence) = occurrence else {
        return Ok(None);
    };
    let artifact = need(&d.source.core.occurrences, occurrence)?.source;
    let mut modules = d
        .source
        .core
        .modules
        .iter()
        .filter(|m| m.source == artifact);
    let module = modules.next().map(Record::id);
    if modules.next().is_some() {
        return Err(invalid("access route source has ambiguous module identity"));
    }
    Ok(module)
}
fn target(
    d: &ClassificationData,
    candidate: &CatalogCandidate,
) -> Result<Option<Id<EntityRef>>, ModelError> {
    if let Some(entity) = candidate.entity {
        Ok(Some(need(&d.source.core.entity_candidates, entity)?.entity))
    } else if let Some(alias) = candidate.alias {
        Ok(Some(need(&d.source.catalog.aliases, alias)?.entity))
    } else if let Some(path) = candidate.path {
        Ok(Some(need(&d.source.catalog.paths, path)?.entity))
    } else {
        Ok(None)
    }
}
/// Relevant captured module/name metadata only. A module is included when an admitted
/// exposure candidate or source alias reaches one of the selected member's exact entities.
/// This is a semantic hydration plan; serving performs its declared joins without interpreting exports.
pub fn hydration_modules(
    d: &ClassificationData,
    member: Id<CatalogMember>,
    b: &ResourceBudget,
) -> Result<
    crate::domain::normalized::contract_comparison::ChargedResult<Vec<Id<source::Module>>>,
    ModelError,
> {
    let selected = need(&d.source.catalog.members, member)?;
    let charge = b.reserve(
        "access-route-hydration-scope",
        d.source
            .catalog
            .candidates
            .len()
            .saturating_add(d.source.catalog.exposures.len())
            .saturating_mul(256),
    )?;
    let roots = d
        .source
        .catalog
        .exposures
        .iter()
        .filter(|e| e.member == member)
        .map(Record::id)
        .collect::<BTreeSet<_>>();
    let mut targets = BTreeSet::new();
    for c in d
        .source
        .catalog
        .candidates
        .iter()
        .filter(|c| roots.contains(&c.exposure))
    {
        if let Some(entity) = target(d, c)? {
            targets.insert(entity);
        }
    }
    let mut modules = BTreeSet::from([selected.access]);
    for c in d.source.catalog.candidates.iter() {
        if target(d, c)?.is_some_and(|entity| targets.contains(&entity)) {
            let exposure = need(&d.source.catalog.exposures, c.exposure)?;
            let public = need(&d.source.core.exposures, exposure.exposure)?;
            let module = need(&d.source.core.modules, public.access)?;
            if need(&d.source.core.artifacts, module.source)?.input == selected.input {
                modules.insert(public.access);
            }
        }
    }
    Ok(
        crate::domain::normalized::contract_comparison::ChargedResult {
            value: modules.into_iter().collect(),
            _charge: charge,
        },
    )
}
// A child retains a route, a visited-name tree, and two new hops in both search vectors.
// Include string payloads and capacity/B-tree overhead before any fanout clone allocates.
fn child_reservation(
    route: &AccessRoute,
    visited: &BTreeSet<(Id<source::Module>, String)>,
    imported: &str,
) -> usize {
    let strings = route
        .hops
        .iter()
        .map(|hop| match hop {
            RouteHop::Import { imported_name, .. } => imported_name.len(),
            _ => 0,
        })
        .fold(0usize, usize::saturating_add);
    let visits = visited
        .iter()
        .map(|(_, name)| {
            size_of::<(Id<source::Module>, String)>()
                .saturating_add(name.len())
                .saturating_add(64)
        })
        .fold(0usize, usize::saturating_add);
    8192usize
        .saturating_add(
            route
                .hops
                .len()
                .saturating_add(2)
                .saturating_mul(size_of::<RouteHop>())
                .saturating_mul(4),
        )
        .saturating_add(strings)
        .saturating_add(visits)
        .saturating_add(imported.len().saturating_mul(4))
}
/// Finite search is independent of response pagination. Omitted searches stay partial.
pub fn explain(
    d: &ClassificationData,
    extra: &RouteData,
    member: Id<CatalogMember>,
    max_depth: usize,
    max_routes: usize,
    b: &ResourceBudget,
) -> Result<RouteResult, ModelError> {
    if !(1..=32).contains(&max_depth) || !(1..=256).contains(&max_routes) {
        return Err(invalid("access route search bounds outside model policy"));
    }
    let selected = need(&d.source.catalog.members, member)?;
    let captured_modules = d
        .source
        .core
        .modules
        .iter()
        .filter(|m| {
            d.source
                .core
                .artifacts
                .get(m.source)
                .is_some_and(|a| a.input == selected.input)
        })
        .count() as u64;
    let mut charge = StateCharge::new(b, "public-access-route-search");
    charge.grow(
        (d.source.catalog.candidates.len()
            + extra.candidates.len()
            + extra.native_bindings.len()
            + extra.names.len()
            + extra.name_supports.len())
        .saturating_mul(512),
    )?;
    let mut output = RouteResult {
        routes: vec![],
        partial: false,
        _charge: charge,
    };
    for exposure_link in d
        .source
        .catalog
        .exposures
        .iter()
        .filter(|e| e.member == member)
    {
        let public = need(&d.source.core.exposures, exposure_link.exposure)?;
        let name = need(&extra.names, public.observation)?;
        if name.access != selected.access || !exact(d, name.qualification, public.context) {
            return Err(invalid(
                "access route exposure changes selected module/context",
            ));
        }
        for candidate in d
            .source
            .catalog
            .candidates
            .iter()
            .filter(|c| c.exposure == exposure_link.id())
        {
            if output.routes.len() >= max_routes {
                output.partial = true;
                if let Some(last) = output.routes.last_mut() {
                    last.omitted_frontiers = last.omitted_frontiers.saturating_add(1)
                }
                continue;
            }
            let entity = target(d, candidate)?;
            if let Some(ec) = candidate.entity {
                let ec = need(&d.source.core.entity_candidates, ec)?;
                let resolution = need(&d.source.core.resolutions, ec.resolution)?;
                if resolution.context != public.context
                    || resolution.entity.is_some_and(|entity| entity != ec.entity)
                {
                    return Err(invalid(
                        "access route entity resolution changes selected context/target",
                    ));
                }
            }
            output._charge.grow(8192)?;
            let mut root = AccessRoute {
                member,
                context: public.context,
                candidate: candidate.id(),
                exposure: public.id(),
                target: entity,
                hops: vec![],
                stop: RouteStop::UnresolvedNameCorrespondence,
                publicity: public.publicity,
                public_resolution: public.status,
                captured_modules,
                declaration_artifact: None,
                stub: None,
                external_consumers_unknown: true,
                installation_requirements_unknown: true,
                omitted_frontiers: 0,
            };
            let supports = extra
                .name_supports
                .iter()
                .filter(|s| s.assertion == name.id())
                .collect::<Vec<_>>();
            if supports.is_empty()
                || supports.iter().any(|s| {
                    !supported(
                        extra,
                        *s,
                        public.context,
                        attribution::FactFamily::Exports,
                        "pyrefly",
                    )
                })
            {
                return Err(invalid("access route exposure support absent"));
            }
            root.hops.push(RouteHop::PublicExposure {
                catalog_candidate: candidate.id(),
                exposure: public.id(),
                name: name.id(),
                support: supports[0].id(),
            });
            if let Some(path) = candidate.path {
                let path = need(&d.source.catalog.paths, path)?;
                root.hops.push(RouteHop::MemberPath {
                    path: path.id(),
                    parent: path.parent,
                    declaration: path.declaration,
                    binding: path.binding,
                    entity: path.entity,
                    disposition: path.disposition,
                });
                if path.disposition != PublicPathDisposition::Effective {
                    root.public_resolution = ResolutionStatus::Ambiguous;
                }
            }
            if let Some(entity) = entity
                && let Some(module) = source_module(d, extra, entity)?
            {
                let artifact = need(&d.source.core.modules, module)?.source;
                let artifact_row = need(&d.source.core.artifacts, artifact)?;
                if artifact_row.input == selected.input {
                    root.declaration_artifact = Some(artifact);
                    root.stub = Some(artifact_row.path.ends_with(".pyi"));
                }
            }
            if let Some(alias) = candidate.alias {
                let alias = need(&d.source.catalog.aliases, alias)?;
                let reference = need(&d.source.core.reference_candidates, alias.reference)?;
                let assessment = need(&d.source.core.reference_assessments, reference.assessment)?;
                let ReferenceEntityTarget::Binding { entity: actual, .. } =
                    need(&d.source.core.reference_targets, reference.target)?
                else {
                    return Err(invalid("access route alias has nonbinding target"));
                };
                if *actual != alias.entity
                    || Some(*actual) != entity
                    || alias.parent != exposure_link.id()
                {
                    return Err(invalid("access route reference changes selected target"));
                }
                let binding = need(&d.source.core.bindings, alias.binding)?;
                let read = need(&d.source.core.references, assessment.reference)?;
                let q = need(
                    &d.source.core.qualifications,
                    need(&d.source.core.lexical_resolutions, reference.resolution)?.qualification,
                )?;
                if binding.value != Some(read.read)
                    || binding.scope != read.scope
                    || q.context != public.context
                    || !exact(d, binding.qualification, public.context)
                    || !exact(d, read.qualification, public.context)
                {
                    return Err(invalid(
                        "access route alias reference changes exact source/context",
                    ));
                }
                root.hops.push(RouteHop::Reference {
                    alias: alias.id(),
                    reference: reference.id(),
                    assessment: assessment.id(),
                    target: reference.target,
                    entity: *actual,
                    status: assessment.status,
                    reason: assessment.reason,
                });
                root.stop = if root.declaration_artifact.is_none() {
                    RouteStop::OutsideCapturedModule
                } else if assessment.status == ResolutionStatus::Resolved
                    && public.status == ResolutionStatus::Resolved
                    && public.publicity == PublicPathKnowledge::Known
                {
                    RouteStop::Declaration
                } else {
                    RouteStop::CandidateOnly
                };
                output.partial |= root.stop != RouteStop::Declaration;
                output.routes.push(root);
                continue;
            }
            let Some(entity) = entity else {
                output.partial = true;
                output.routes.push(root);
                continue;
            };
            let Some(destination) = source_module(d, extra, entity)? else {
                root.stop = RouteStop::OutsideCapturedModule;
                output.partial = true;
                output.routes.push(root);
                continue;
            };
            let artifact = need(&d.source.core.modules, destination)?.source;
            if need(&d.source.core.artifacts, artifact)?.input != selected.input {
                root.stop = RouteStop::OutsideCapturedModule;
                output.partial = true;
                output.routes.push(root);
                continue;
            }
            root.declaration_artifact = Some(artifact);
            root.stub = Some(
                need(&d.source.core.artifacts, artifact)?
                    .path
                    .ends_with(".pyi"),
            );
            output
                ._charge
                .grow(4096usize.saturating_add(name.name.len().saturating_mul(4)))?;
            let mut pending = vec![(selected.access, name.name.clone(), root, BTreeSet::new())];
            while let Some((module, slot, mut route, mut visited)) = pending.pop() {
                output._charge.grow(4096usize.saturating_add(slot.len()))?;
                if output.routes.len() >= max_routes {
                    output.partial = true;
                    if let Some(last) = output.routes.last_mut() {
                        last.omitted_frontiers = last
                            .omitted_frontiers
                            .saturating_add((pending.len() as u64).saturating_add(1))
                    }
                    break;
                }
                if !visited.insert((module, slot.clone())) {
                    route.stop = RouteStop::Cycle;
                    output.partial = true;
                    output.routes.push(route);
                    continue;
                }
                if module == destination {
                    route.stop = if route.publicity == PublicPathKnowledge::Known
                        && route.public_resolution == ResolutionStatus::Resolved
                    {
                        RouteStop::Declaration
                    } else {
                        RouteStop::CandidateOnly
                    };
                    output.partial |= route.stop != RouteStop::Declaration;
                    output.routes.push(route);
                    continue;
                }
                if visited.len() > max_depth {
                    route.stop = RouteStop::Frontier;
                    output.partial = true;
                    output.routes.push(route);
                    continue;
                }
                let mut next = vec![];
                for binding in d.source.core.bindings.iter().filter(|binding| {
                    binding.kind == lexical::BindingEventKind::FromImport
                        && binding.static_branch.is_none()
                        && exact(d, binding.qualification, route.context)
                }) {
                    let event = need(&d.source.core.binding_events, binding.event)?;
                    let scope = need(&d.source.core.lexical_scopes, binding.scope)?;
                    if event.name != slot
                        || scope.kind != lexical::LexicalScopeKind::Module
                        || need(&d.source.core.occurrences, scope.owner)?.source
                            != need(&d.source.core.modules, module)?.source
                        || need(&d.source.core.occurrences, event.site)?.source
                            != need(&d.source.core.modules, module)?.source
                    {
                        continue;
                    }
                    for native in extra.native_bindings.iter().filter(|n| {
                        n.event == event.id()
                            && n.scope == Some(binding.scope)
                            && n.kind == RuffBindingKind::FromImport
                            && !n.typing
                            && exact(d, n.qualification, route.context)
                    }) {
                        let Some(imported) = native.qualified_name.as_ref().and_then(|q| q.last())
                        else {
                            continue;
                        };
                        for alias in extra.imports.iter().filter(|a| {
                            a.alias == event.site && exact(d, a.qualification, route.context)
                        }) {
                            for assessment in extra
                                .assessments
                                .iter()
                                .filter(|a| a.observation == alias.id())
                            {
                                for candidate in extra
                                    .candidates
                                    .iter()
                                    .filter(|c| c.assessment == assessment.id())
                                {
                                    let resolution =
                                        need(&extra.resolutions, candidate.observation)?;
                                    if resolution.alias != Some(alias.alias)
                                        || resolution.module != candidate.module
                                        || !exact(d, resolution.qualification, route.context)
                                    {
                                        return Err(invalid(
                                            "access route import changes alias/context/module",
                                        ));
                                    }
                                    let calls::ProviderModule::Acquired {
                                        module: target_module,
                                    } = need(&d.source.core.provider_modules, candidate.module)?
                                    else {
                                        continue;
                                    };
                                    // Canonical public name at the resolved target module must admit this exact entity.
                                    let target_exposure =
                                        d.source.core.exposures.iter().find(|e| {
                                            e.access == *target_module
                                                && e.context == route.context
                                                && extra
                                                    .names
                                                    .get(e.observation)
                                                    .is_some_and(|n| n.name == *imported)
                                                && d.source.catalog.exposures.iter().any(|ce| {
                                                    ce.exposure == e.id()
                                                        && d.source.catalog.candidates.iter().any(
                                                            |c| {
                                                                c.exposure == ce.id()
                                                                    && target(d, c).ok().flatten()
                                                                        == Some(entity)
                                                            },
                                                        )
                                                })
                                        });
                                    let Some(target_exposure) = target_exposure else {
                                        continue;
                                    };
                                    let target_name =
                                        need(&extra.names, target_exposure.observation)?;
                                    if !exact(d, target_name.qualification, route.context) {
                                        continue;
                                    }
                                    let target_support = extra
                                        .name_supports
                                        .iter()
                                        .find(|s| {
                                            s.assertion == target_name.id()
                                                && supported(
                                                    extra,
                                                    *s,
                                                    route.context,
                                                    attribution::FactFamily::Exports,
                                                    "pyrefly",
                                                )
                                        })
                                        .ok_or_else(|| {
                                            invalid("access route target exposure support absent")
                                        })?;
                                    let mut target_candidate = None;
                                    for ce in d
                                        .source
                                        .catalog
                                        .exposures
                                        .iter()
                                        .filter(|ce| ce.exposure == target_exposure.id())
                                    {
                                        for c in d
                                            .source
                                            .catalog
                                            .candidates
                                            .iter()
                                            .filter(|c| c.exposure == ce.id())
                                        {
                                            if target(d, c)? == Some(entity) {
                                                target_candidate = Some(c.id());
                                                break;
                                            }
                                        }
                                    }
                                    let target_candidate = target_candidate.ok_or_else(|| {
                                        invalid("access route target candidate absent")
                                    })?;
                                    let native_support = extra
                                        .native_supports
                                        .iter()
                                        .find(|s| s.assertion == native.id())
                                        .ok_or_else(|| {
                                            invalid("access route native import support absent")
                                        })?;
                                    let support = extra
                                        .resolution_supports
                                        .iter()
                                        .find(|s| s.assertion == resolution.id())
                                        .ok_or_else(|| {
                                            invalid("access route module resolution support absent")
                                        })?;
                                    if !supported(
                                        extra,
                                        native_support,
                                        route.context,
                                        attribution::FactFamily::Lexical,
                                        "ruff",
                                    ) || !supported(
                                        extra,
                                        support,
                                        route.context,
                                        attribution::FactFamily::Exports,
                                        "pyrefly",
                                    ) {
                                        return Err(invalid(
                                            "access route import support changes provider/context/fidelity",
                                        ));
                                    }
                                    output
                                        ._charge
                                        .grow(child_reservation(&route, &visited, imported))?;
                                    let mut child = route.clone();
                                    child.hops.push(RouteHop::Import {
                                        alias: alias.id(),
                                        native_name: native.id(),
                                        native_support: native_support.id(),
                                        assessment: assessment.id(),
                                        candidate: candidate.id(),
                                        resolution: resolution.id(),
                                        support: support.id(),
                                        from: module,
                                        to: *target_module,
                                        imported_name: imported.clone(),
                                        status: assessment.status,
                                        reason: assessment.reason,
                                    });
                                    child.hops.push(RouteHop::PublicExposure {
                                        catalog_candidate: target_candidate,
                                        exposure: target_exposure.id(),
                                        name: target_name.id(),
                                        support: target_support.id(),
                                    });
                                    if target_exposure.publicity != PublicPathKnowledge::Known {
                                        child.publicity = PublicPathKnowledge::Candidate
                                    }
                                    if target_exposure.status != ResolutionStatus::Resolved {
                                        child.public_resolution = target_exposure.status
                                    }
                                    if assessment.status != ResolutionStatus::Resolved {
                                        child.public_resolution = ResolutionStatus::Ambiguous
                                    }
                                    next.push((
                                        *target_module,
                                        imported.clone(),
                                        child,
                                        visited.clone(),
                                    ));
                                }
                            }
                        }
                    }
                }
                if next.is_empty() {
                    route.stop = RouteStop::UnresolvedNameCorrespondence;
                    output.partial = true;
                    output.routes.push(route)
                } else {
                    next.sort_by(|left, right| {
                        left.0.cmp(&right.0).then_with(|| left.1.cmp(&right.1))
                    });
                    for child in next.into_iter().rev() {
                        pending.push(child)
                    }
                }
            }
        }
    }
    Ok(output)
}

pub fn definition() -> ContentHash {
    ContentHash::of(b"catalog-access-route-policy/v1")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{assertion::*, attribution::*, source::*};
    fn id<T>(n: u8) -> Id<T> {
        serde::Deserialize::deserialize(serde::de::value::SeqDeserializer::<
            _,
            serde::de::value::Error,
        >::new([n; 16].into_iter()))
        .unwrap()
    }
    fn fixture() -> (
        ResourceBudget,
        ClassificationData,
        RouteData,
        Id<CatalogMember>,
    ) {
        let b = ResourceBudget::fixed(8 << 20).unwrap();
        let mut d = ClassificationData::new(&b);
        let mut extra = RouteData::new(&b);
        let artifact =
            SourceArtifact::from_bytes(id(1), "api.py".into(), b"def parse(): pass\n").unwrap();
        d.source.core.artifacts.insert(artifact.clone()).unwrap();
        let module = Module {
            source: artifact.id(),
            qualified_name: "demo".into(),
        };
        d.source.core.modules.insert(module.clone()).unwrap();
        let context = id(2);
        let q = AssertionQualification {
            context,
            scope: CoverageScope::Artifact {
                artifact: artifact.id(),
            }
            .id(),
            condition: conditions::Diagram::always().id(),
            assumptions: assumptions::AssumptionSet::empty_id(),
            modality: Modality::Definite,
            approximation: Approximation::Exact,
        };
        d.source.core.qualifications.insert(q.clone()).unwrap();
        let occurrence = Occurrence {
            source: artifact.id(),
            start: 0,
            end: 17,
            syntax_kind: SyntaxKind::StmtFunctionDef,
            role: OccurrenceRole::Declaration,
            structural_path: vec![0],
        };
        d.source
            .core
            .occurrences
            .insert(occurrence.clone())
            .unwrap();
        let entity = EntityRef::Occurrence {
            occurrence: occurrence.id(),
        };
        d.source.core.refs.insert(entity.clone()).unwrap();
        let origin = ExportOrigin::Untraced;
        extra.origins.insert(origin.clone()).unwrap();
        let name = PublicNameObservation {
            qualification: q.id(),
            access: module.id(),
            name: "parse".into(),
            via_dunder_all: false,
            origin: origin.id(),
        };
        extra.names.insert(name.clone()).unwrap();
        let public = PublicExposure {
            access: module.id(),
            context,
            observation: name.id(),
            origin: origin.id(),
            enumeration: None,
            publicity: PublicPathKnowledge::Known,
            status: ResolutionStatus::Resolved,
            reason: EntityReason::DeclarationAgreement,
        };
        d.source.core.exposures.insert(public.clone()).unwrap();
        let member = CatalogMember {
            input: artifact.input,
            access: module.id(),
            path: vec!["parse".into()],
            name: "parse".into(),
        };
        d.source.catalog.members.insert(member.clone()).unwrap();
        let ce = CatalogExposure {
            member: member.id(),
            exposure: public.id(),
        };
        d.source.catalog.exposures.insert(ce.clone()).unwrap();
        let resolution = SymbolEntityResolution {
            symbol: id(3),
            context,
            policy: ContentHash::of(b"policy"),
            status: ResolutionStatus::Resolved,
            entity: Some(entity.id()),
            reason: EntityReason::DeclarationAgreement,
        };
        d.source
            .core
            .resolutions
            .insert(resolution.clone())
            .unwrap();
        let ec = SymbolEntityCandidate {
            resolution: resolution.id(),
            entity: entity.id(),
        };
        d.source.core.entity_candidates.insert(ec.clone()).unwrap();
        d.source
            .catalog
            .candidates
            .insert(CatalogCandidate {
                exposure: ce.id(),
                candidate: None,
                entity: Some(ec.id()),
                path: None,
                alias: None,
            })
            .unwrap();
        let provider = Provider {
            tool: "pyrefly".into(),
            revision: "test".into(),
            build_digest: ContentHash::of(b"test"),
        };
        extra.providers.insert(provider.clone()).unwrap();
        let run = ProviderRun::new(
            provider.id(),
            context,
            artifact.input,
            ContentHash::of(b"cfg"),
            [FactFamily::Exports],
        )
        .unwrap()
        .0;
        extra.runs.insert(run.clone()).unwrap();
        let surface = ProviderSurface {
            provider: provider.id(),
            family: FactFamily::Exports,
            name: "native exports".into(),
        };
        extra.surfaces.insert(surface.clone()).unwrap();
        let evidence = Evidence::Invocation { run: run.id() };
        extra.evidence.insert(evidence.clone()).unwrap();
        extra
            .name_supports
            .insert(PublicNameSupport {
                assertion: name.id(),
                run: run.id(),
                surface: surface.id(),
                evidence: evidence.id(),
                origin: Origin::AnalyzerAssertion,
                mode: ExtractionMode::NativeTraversal,
                fidelity: Fidelity::NativeStructural,
            })
            .unwrap();
        (b, d, extra, member.id())
    }
    fn imported_fixture(
        cycle: bool,
    ) -> (
        ResourceBudget,
        ClassificationData,
        RouteData,
        Id<CatalogMember>,
    ) {
        let (b, mut d, mut extra, member) = fixture();
        let selected = d.source.catalog.members.get(member).unwrap().clone();
        let root_module = d.source.core.modules.get(selected.access).unwrap().clone();
        let q = d.source.core.qualifications.iter().next().unwrap().clone();
        let artifact = SourceArtifact::from_bytes(
            selected.input,
            "implementation.py".into(),
            b"def operation(): pass\n",
        )
        .unwrap();
        d.source.core.artifacts.insert(artifact.clone()).unwrap();
        let destination = Module {
            source: artifact.id(),
            qualified_name: "demo.api".into(),
        };
        d.source.core.modules.insert(destination.clone()).unwrap();
        let declaration = Occurrence {
            source: artifact.id(),
            start: 0,
            end: 21,
            syntax_kind: SyntaxKind::StmtFunctionDef,
            role: OccurrenceRole::Declaration,
            structural_path: vec![0],
        };
        d.source
            .core
            .occurrences
            .insert(declaration.clone())
            .unwrap();
        let entity = EntityRef::Occurrence {
            occurrence: declaration.id(),
        };
        d.source.core.refs.insert(entity.clone()).unwrap();
        let resolution = SymbolEntityResolution {
            symbol: id(4),
            context: q.context,
            policy: ContentHash::of(b"policy"),
            status: ResolutionStatus::Resolved,
            entity: Some(entity.id()),
            reason: EntityReason::DeclarationAgreement,
        };
        d.source
            .core
            .resolutions
            .insert(resolution.clone())
            .unwrap();
        let ec = SymbolEntityCandidate {
            resolution: resolution.id(),
            entity: entity.id(),
        };
        d.source.core.entity_candidates.insert(ec.clone()).unwrap();
        let parent = d
            .source
            .catalog
            .exposures
            .iter()
            .find(|ce| ce.member == member)
            .unwrap()
            .id();
        d.source.catalog.candidates = Rows::new(&b);
        d.source
            .catalog
            .candidates
            .insert(CatalogCandidate {
                exposure: parent,
                candidate: None,
                entity: Some(ec.id()),
                path: None,
                alias: None,
            })
            .unwrap();
        let owner = Occurrence {
            source: root_module.source,
            start: 0,
            end: 18,
            syntax_kind: SyntaxKind::ModModule,
            role: OccurrenceRole::Syntax,
            structural_path: vec![1],
        };
        d.source.core.occurrences.insert(owner.clone()).unwrap();
        let scope = lexical::LexicalScope {
            owner: owner.id(),
            kind: lexical::LexicalScopeKind::Module,
        };
        d.source.core.lexical_scopes.insert(scope.clone()).unwrap();
        let alias = Occurrence {
            source: root_module.source,
            start: 1,
            end: 4,
            syntax_kind: SyntaxKind::Alias,
            role: OccurrenceRole::Binding,
            structural_path: vec![2],
        };
        d.source.core.occurrences.insert(alias.clone()).unwrap();
        let event = lexical::BindingEvent {
            site: alias.id(),
            name: "parse".into(),
        };
        d.source.core.binding_events.insert(event.clone()).unwrap();
        d.source
            .core
            .bindings
            .insert(lexical::BindingObservation {
                qualification: q.id(),
                event: event.id(),
                scope: scope.id(),
                kind: lexical::BindingEventKind::FromImport,
                ordinal: 0,
                value: None,
                static_branch: None,
                static_polarity: None,
            })
            .unwrap();
        let native = RuffBindingObservation {
            qualification: q.id(),
            event: event.id(),
            kind: RuffBindingKind::FromImport,
            native_name: "parse".into(),
            scope: Some(scope.id()),
            scope_location: AttachmentStatus::Located,
            shadowed: None,
            shadowed_location: NativeRelationLocation::Absent,
            outer_shadowed: None,
            outer_shadowed_location: NativeRelationLocation::Absent,
            definition_scope: None,
            definition_scope_location: NativeRelationLocation::Absent,
            typing: false,
            qualified_name: Some(vec![
                "demo".into(),
                "api".into(),
                if cycle {
                    "parse".into()
                } else {
                    "operation".into()
                },
            ]),
            explicit_export: true,
            external: false,
            alias: true,
            nonlocal: false,
            global: false,
            deleted: false,
            invalid_all_format: false,
            invalid_all_object: false,
            private_declaration: false,
            unpacked_assignment: false,
            in_except_handler: false,
            annotated_type_alias: false,
            deferred_type_alias: false,
            in_assert_statement: false,
            lazy: false,
        };
        extra.native_bindings.insert(native.clone()).unwrap();
        let ruff = Provider {
            tool: "ruff".into(),
            revision: "test".into(),
            build_digest: ContentHash::of(b"ruff"),
        };
        extra.providers.insert(ruff.clone()).unwrap();
        let run = ProviderRun::new(
            ruff.id(),
            q.context,
            selected.input,
            ContentHash::of(b"cfg"),
            [FactFamily::Lexical],
        )
        .unwrap()
        .0;
        extra.runs.insert(run.clone()).unwrap();
        let surface = ProviderSurface {
            provider: ruff.id(),
            family: FactFamily::Lexical,
            name: "native bindings".into(),
        };
        extra.surfaces.insert(surface.clone()).unwrap();
        let evidence = Evidence::Invocation { run: run.id() };
        extra.evidence.insert(evidence.clone()).unwrap();
        extra
            .native_supports
            .insert(RuffBindingSupport {
                assertion: native.id(),
                run: run.id(),
                surface: surface.id(),
                evidence: evidence.id(),
                origin: Origin::AnalyzerAssertion,
                mode: ExtractionMode::NativeTraversal,
                fidelity: Fidelity::NativeStructural,
            })
            .unwrap();
        let import = ImportAliasObservation {
            qualification: q.id(),
            statement: owner.id(),
            alias: alias.id(),
            level: 0,
            resolved_module: Some("demo.api".into()),
        };
        extra.imports.insert(import.clone()).unwrap();
        let assessment = ImportModuleAssessment {
            observation: import.id(),
            status: ResolutionStatus::Resolved,
            reason: LinkReason::ExplicitIdentity,
        };
        extra.assessments.insert(assessment.clone()).unwrap();
        let pm = calls::ProviderModule::Acquired {
            module: if cycle {
                root_module.id()
            } else {
                destination.id()
            },
        };
        d.source.core.provider_modules.insert(pm.clone()).unwrap();
        let native_resolution = ModuleResolutionObservation {
            qualification: q.id(),
            module: pm.id(),
            alias: Some(alias.id()),
            location: Some("implementation.py".into()),
        };
        extra.resolutions.insert(native_resolution.clone()).unwrap();
        extra
            .candidates
            .insert(ImportModuleCandidate {
                assessment: assessment.id(),
                observation: native_resolution.id(),
                module: pm.id(),
            })
            .unwrap();
        let support = extra.name_supports.iter().next().unwrap().clone();
        extra
            .resolution_supports
            .insert(ModuleResolutionSupport {
                assertion: native_resolution.id(),
                run: support.run,
                surface: support.surface,
                evidence: support.evidence,
                origin: Origin::AnalyzerAssertion,
                mode: ExtractionMode::NativeTraversal,
                fidelity: Fidelity::NativeStructural,
            })
            .unwrap();
        if !cycle {
            let mut target_q = q.clone();
            target_q.scope = CoverageScope::Artifact {
                artifact: artifact.id(),
            }
            .id();
            d.source
                .core
                .qualifications
                .insert(target_q.clone())
                .unwrap();
            let name = PublicNameObservation {
                qualification: target_q.id(),
                access: destination.id(),
                name: "operation".into(),
                via_dunder_all: true,
                origin: ExportOrigin::Untraced.id(),
            };
            extra.names.insert(name.clone()).unwrap();
            extra
                .name_supports
                .insert(PublicNameSupport {
                    assertion: name.id(),
                    ..support
                })
                .unwrap();
            let public = PublicExposure {
                access: destination.id(),
                context: q.context,
                observation: name.id(),
                origin: name.origin,
                enumeration: None,
                publicity: PublicPathKnowledge::Known,
                status: ResolutionStatus::Resolved,
                reason: EntityReason::DeclarationAgreement,
            };
            d.source.core.exposures.insert(public.clone()).unwrap();
            let target_member = CatalogMember {
                input: selected.input,
                access: destination.id(),
                path: vec!["operation".into()],
                name: "operation".into(),
            };
            d.source
                .catalog
                .members
                .insert(target_member.clone())
                .unwrap();
            let ce = CatalogExposure {
                member: target_member.id(),
                exposure: public.id(),
            };
            d.source.catalog.exposures.insert(ce.clone()).unwrap();
            d.source
                .catalog
                .candidates
                .insert(CatalogCandidate {
                    exposure: ce.id(),
                    candidate: None,
                    entity: Some(ec.id()),
                    path: None,
                    alias: None,
                })
                .unwrap();
        }
        (b, d, extra, member)
    }
    #[test]
    fn import_name_hops_require_exact_target_membership_and_report_cycles() {
        let (b, d, extra, member) = imported_fixture(false);
        let result = explain(&d, &extra, member, 16, 128, &b).unwrap();
        assert_eq!(result.routes[0].stop, RouteStop::Declaration);
        assert!(
            matches!(&result.routes[0].hops[1],RouteHop::Import{imported_name,..} if imported_name=="operation")
        );
        assert!(!result.partial);
        let (b, d, mut extra, member) = imported_fixture(false);
        extra.native_bindings = Rows::new(&b);
        let result = explain(&d, &extra, member, 16, 128, &b).unwrap();
        assert_eq!(
            result.routes[0].stop,
            RouteStop::UnresolvedNameCorrespondence
        );
        assert!(result.partial);
        let (b, d, extra, member) = imported_fixture(true);
        let result = explain(&d, &extra, member, 16, 128, &b).unwrap();
        assert_eq!(result.routes[0].stop, RouteStop::Cycle);
        assert!(result.partial);
    }
    #[test]
    fn route_cap_counts_every_discarded_pending_frontier() {
        let (b, mut d, mut extra, member) = imported_fixture(false);
        let resolution = extra.resolutions.iter().next().unwrap().clone();
        let candidate = extra.candidates.iter().next().unwrap().clone();
        let support = extra.resolution_supports.iter().next().unwrap().clone();
        let selected = d.source.catalog.members.get(member).unwrap().clone();
        let qualification = d
            .source
            .core
            .qualifications
            .get(resolution.qualification)
            .unwrap()
            .clone();
        for scope in [
            CoverageScope::Module {
                module: selected.access,
            },
            CoverageScope::Input {
                input: selected.input,
            },
        ] {
            let qualification = AssertionQualification {
                scope: scope.id(),
                ..qualification.clone()
            };
            d.source
                .core
                .qualifications
                .insert(qualification.clone())
                .unwrap();
            let alternative = ModuleResolutionObservation {
                qualification: qualification.id(),
                ..resolution.clone()
            };
            extra.resolutions.insert(alternative.clone()).unwrap();
            extra
                .candidates
                .insert(ImportModuleCandidate {
                    observation: alternative.id(),
                    ..candidate.clone()
                })
                .unwrap();
            extra
                .resolution_supports
                .insert(ModuleResolutionSupport {
                    assertion: alternative.id(),
                    ..support.clone()
                })
                .unwrap();
        }
        let result = explain(&d, &extra, member, 16, 1, &b).unwrap();
        assert_eq!(result.routes.len(), 1);
        assert_eq!(result.routes[0].stop, RouteStop::Declaration);
        assert_eq!(result.routes[0].omitted_frontiers, 2);
        assert!(result.partial);
    }
    #[test]
    fn same_named_target_without_selected_entity_cannot_complete_import_route() {
        let (b, mut d, extra, member) = imported_fixture(false);
        let selected = d.source.catalog.members.get(member).unwrap();
        let target_exposure = d
            .source
            .catalog
            .exposures
            .iter()
            .find(|ce| d.source.core.exposures.get(ce.exposure).unwrap().access != selected.access)
            .unwrap()
            .id();
        let selected_entity = d
            .source
            .catalog
            .candidates
            .iter()
            .find(|c| c.exposure != target_exposure)
            .unwrap()
            .entity
            .unwrap();
        let wrong_entity = d
            .source
            .core
            .entity_candidates
            .iter()
            .find(|e| e.id() != selected_entity)
            .unwrap()
            .id();
        let candidates = d
            .source
            .catalog
            .candidates
            .iter()
            .cloned()
            .collect::<Vec<_>>();
        d.source.catalog.candidates = Rows::new(&b);
        for mut candidate in candidates {
            if candidate.exposure == target_exposure {
                candidate.entity = Some(wrong_entity)
            }
            d.source.catalog.candidates.insert(candidate).unwrap();
        }
        let result = explain(&d, &extra, member, 16, 128, &b).unwrap();
        assert_eq!(result.routes.len(), 1);
        assert_eq!(
            result.routes[0].stop,
            RouteStop::UnresolvedNameCorrespondence
        );
        assert!(result.partial);
        assert!(
            !result.routes[0]
                .hops
                .iter()
                .any(|hop| matches!(hop, RouteHop::Import { .. }))
        );
    }
    #[test]
    fn candidate_publicity_never_becomes_a_supported_declaration_route() {
        let (b, mut d, extra, member) = fixture();
        let mut public = d.source.core.exposures.iter().next().unwrap().clone();
        public.publicity = PublicPathKnowledge::Candidate;
        d.source.core.exposures = Rows::new(&b);
        d.source.core.exposures.insert(public.clone()).unwrap();
        let mut exposure = d.source.catalog.exposures.iter().next().unwrap().clone();
        exposure.exposure = public.id();
        d.source.catalog.exposures = Rows::new(&b);
        d.source.catalog.exposures.insert(exposure.clone()).unwrap();
        let mut candidate = d.source.catalog.candidates.iter().next().unwrap().clone();
        candidate.exposure = exposure.id();
        d.source.catalog.candidates = Rows::new(&b);
        d.source.catalog.candidates.insert(candidate).unwrap();
        let result = explain(&d, &extra, member, 16, 128, &b).unwrap();
        assert_eq!(result.routes[0].stop, RouteStop::CandidateOnly);
        assert_eq!(result.routes[0].publicity, PublicPathKnowledge::Candidate);
        assert!(result.partial);
    }
    #[test]
    fn direct_route_preserves_exact_identity_and_request_bound() {
        let (b, d, extra, member) = fixture();
        let result = explain(&d, &extra, member, 16, 128, &b).unwrap();
        assert_eq!(result.routes.len(), 1);
        assert_eq!(result.routes[0].stop, RouteStop::Declaration);
        assert_eq!(result.routes[0].hops.len(), 1);
        assert!(!result.partial);
        assert!(explain(&d, &extra, member, 0, 128, &b).is_err());
        assert!(explain(&d, &extra, member, 16, 0, &b).is_err());
        let denied = ResourceBudget::fixed(1).unwrap();
        assert!(matches!(
            explain(&d, &extra, member, 16, 128, &denied),
            Err(ModelError::Resource { .. })
        ));
    }
    #[test]
    fn foreign_context_and_support_cannot_retain_a_supported_public_route() {
        let (b, mut d, extra, member) = fixture();
        let q = d.source.core.qualifications.iter().next().unwrap().id();
        d.source.core.qualifications = Rows::new(&b);
        assert!(explain(&d, &extra, member, 16, 128, &b).is_err());
        let _ = q;
        let (b, d, mut extra, member) = fixture();
        extra.runs = Rows::new(&b);
        assert!(explain(&d, &extra, member, 16, 128, &b).is_err());
    }
}
