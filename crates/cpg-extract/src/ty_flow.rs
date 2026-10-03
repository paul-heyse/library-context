//! Native ty coordinates become facts only through exact typed attachment. No provider-local
//! index, predicate key, rendered condition or display-place identity is stored.
use crate::{
    acquisition::Acquisition,
    assembly::Attached,
    bundle::{Declared, ProviderStage, StageContext},
    typed_syntax::SyntaxLimits,
};
use cpg_flow::{ModuleFlow, Span, native};
use lctx_model::domain::{
    assertion::*,
    attribution::*,
    charged::StateCharge,
    conditions::*,
    flow::*,
    lexical::*,
    source::*,
    stages::{Effect, Profile, ProviderOutcome, RelationUse, Stage, StageSink},
    syntax::*,
    value::*,
    *,
};
use std::collections::{BTreeMap, HashMap};
pub const TY_FLOW: &str = "ty_flow";
#[derive(Default)]
pub struct TyFlow {
    pub limits: SyntaxLimits,
}
pub fn provider() -> Provider {
    Provider {
        tool: "ty".into(),
        revision: cpg_flow::PROVIDER.into(),
        build_digest: crate::bundle::build_digest(&[
            include_str!("ty_flow.rs"),
            // This adapter's facts also depend on the provider's native mapping.
            include_str!("../../cpg-flow/src/lib.rs"),
            include_str!("../../cpg-flow/src/predicate.rs"),
        ]),
    }
}
macro_rules! output_types {
    ($apply:ident) => {
        $apply!(
            FlowUse,
            FlowDefinition,
            ReachingDefinition,
            FlowUseObservation,
            FlowUseSupport,
            FlowDefinitionObservation,
            FlowDefinitionSupport,
            FlowReachingObservation,
            FlowReachingSupport,
            FlowNarrowingObservation,
            FlowNarrowingSupport,
            FlowSourceViewObservation,
            FlowSourceViewSupport,
            FlowValueObservation,
            FlowValueSupport,
            FlowRegionObservation,
            FlowRegionSupport,
            FlowTestObservation,
            FlowTestSupport,
            FlowTestLeafObservation,
            FlowTestLeafSupport,
            FlowAttributeLoadObservation,
            FlowAttributeLoadSupport,
            FlowCallPath,
            FlowCallStep,
            FlowValuePathObservation,
            FlowValuePathSupport
        )
    };
}
impl Declared for TyFlow {
    fn declaration(&self, _: Profile) -> Stage {
        macro_rules! uses {($($ty:ty),+)=>{vec![$(RelationUse::of::<$ty>()),+]}}
        let provider = provider();
        Stage {
            name: TY_FLOW,
            inputs: uses!(
                Module,
                Occurrence,
                LexicalScope,
                DeclarationObservation,
                BindingEvent,
                BindingObservation,
                LexicalTarget,
                LexicalResolution,
                ReferenceObservation,
                ImportAliasObservation,
                SyntaxObservation
            ),
            outputs: output_types!(uses),
            contributes: crate::assembly::vocabulary(),
            coverage: vec![FactFamily::Flow].into_iter().map(|family| lctx_model::domain::stages::FamilyCoverage {family,provider:provider.id()}).collect(),
            profiles: vec![Profile::Behavioral],
            effect: Effect::Extraction,
            code: provider.build_digest,
            configuration: ContentHash::of(format!("{:?}", self.limits).as_bytes()),
        }
    }
}
fn invalid(message: &str) -> ModelError {
    ModelError::Invalid(message.into())
}
/// Pipeline-owned typed indexes reserve before retaining their copies. Native analyzer heaps,
/// including ty's parse/semantic index, remain provider-owned and are measured separately at Q.
#[derive(Clone, Copy, PartialEq, Eq)]
enum RuntimeSpecial {
    Checking,
    Typing,
    Sys,
    Os,
    Type,
    Class,
}
type SourceIndex<T> = BTreeMap<Id<SourceArtifact>, Vec<T>>;
struct Index {
    _charge: StateCharge,
    occurrences: BTreeMap<Id<Occurrence>, Occurrence>,
    scopes: BTreeMap<Id<LexicalScope>, LexicalScope>,
    declarations: Vec<DeclarationObservation>,
    bindings: BTreeMap<Id<BindingEvent>, BindingObservation>,
    events: BTreeMap<Id<BindingEvent>, BindingEvent>,
    identifiers: BTreeMap<Id<Occurrence>, String>,
    targets: BTreeMap<Id<LexicalTarget>, LexicalTarget>,
    resolutions: BTreeMap<Id<Occurrence>, Vec<LexicalResolution>>,
    references: Vec<ReferenceObservation>,
    imports: Vec<ImportAliasObservation>,
    runtime_imports: BTreeMap<Id<BindingEvent>, RuntimeSpecial>,
    source_imports: SourceIndex<usize>,
    source_identifiers: SourceIndex<Id<Occurrence>>,
    source_references: SourceIndex<usize>,
    source_scopes: SourceIndex<Id<LexicalScope>>,
    scope_bindings: BTreeMap<Id<LexicalScope>, Vec<Id<BindingEvent>>>,
    occurrence_bindings: BTreeMap<Id<Occurrence>, Vec<Id<BindingEvent>>>,
    owner_declarations: BTreeMap<Id<Occurrence>, Vec<usize>>,
}
impl Index {
    fn new<S: StageSink + 'static>(context: &mut StageContext<S>) -> Result<Self, ModelError> {
        let mut charge = StateCharge::new(context.budget(), "ty_flow_indexes");
        fn read<R: Record, S: StageSink + 'static>(
            context: &mut StageContext<S>,
            charge: &mut StateCharge,
        ) -> Result<Vec<R>, ModelError> {
            let batches = context.handoff::<R>()?;
            let mut rows = Vec::new();
            for batch in batches {
                for row in batch.rows() {
                    charge.grow((size_of::<R>() + row.heap_bytes() + 128).saturating_mul(4))?;
                    rows.push(row.clone());
                }
            }
            Ok(rows)
        }
        let occurrences: BTreeMap<_, _> = read::<Occurrence, _>(context, &mut charge)?
            .into_iter()
            .map(|r| (r.id(), r))
            .collect();
        let scopes: BTreeMap<_, _> = read::<LexicalScope, _>(context, &mut charge)?
            .into_iter()
            .map(|r| (r.id(), r))
            .collect();
        let declarations: Vec<DeclarationObservation> = read(context, &mut charge)?;
        let bindings: BTreeMap<_, _> = read::<BindingObservation, _>(context, &mut charge)?
            .into_iter()
            .map(|r| (r.event, r))
            .collect();
        let events: BTreeMap<_, _> = read::<BindingEvent, _>(context, &mut charge)?
            .into_iter()
            .map(|r| (r.id(), r))
            .collect();
        let targets = read::<LexicalTarget, _>(context, &mut charge)?
            .into_iter()
            .map(|r| (r.id(), r))
            .collect();
        let mut resolutions: BTreeMap<_, Vec<_>> = BTreeMap::new();
        for row in read::<LexicalResolution, _>(context, &mut charge)? {
            resolutions.entry(row.read).or_default().push(row);
        }
        let references: Vec<ReferenceObservation> = read(context, &mut charge)?;
        let imports: Vec<ImportAliasObservation> = read(context, &mut charge)?;
        let identifiers: BTreeMap<_, _> = read::<SyntaxObservation, _>(context, &mut charge)?
            .into_iter()
            .map(|o| (o.occurrence, o.spelling))
            .collect();
        // Derived lookup partitions preserve each original collection's order and predicates.
        // Reserve before retaining each key/slot; these indexes own no additional semantic facts.
        fn push<K: Ord, V>(
            map: &mut BTreeMap<K, Vec<V>>,
            charge: &mut StateCharge,
            key: K,
            value: V,
        ) -> Result<(), ModelError> {
            charge.grow(
                size_of::<K>()
                    .saturating_add(size_of::<V>().saturating_mul(2))
                    .saturating_add(128),
            )?;
            map.entry(key).or_default().push(value);
            Ok(())
        }
        let mut source_imports = BTreeMap::new();
        for (position, import) in imports.iter().enumerate() {
            if let Some(site) = occurrences.get(&import.alias) {
                push(&mut source_imports, &mut charge, site.source, position)?;
            }
        }
        let mut source_identifiers = BTreeMap::new();
        for id in identifiers.keys() {
            if let Some(site) = occurrences.get(id) {
                push(&mut source_identifiers, &mut charge, site.source, *id)?;
            }
        }
        let mut source_references = BTreeMap::new();
        for (position, reference) in references.iter().enumerate() {
            if let Some(site) = occurrences.get(&reference.read) {
                push(&mut source_references, &mut charge, site.source, position)?;
            }
        }
        let mut source_scopes = BTreeMap::new();
        for (id, scope) in &scopes {
            if let Some(site) = occurrences.get(&scope.owner) {
                push(&mut source_scopes, &mut charge, site.source, *id)?;
            }
        }
        let mut scope_bindings = BTreeMap::new();
        let mut occurrence_bindings = BTreeMap::new();
        for (id, binding) in &bindings {
            push(&mut scope_bindings, &mut charge, binding.scope, *id)?;
            if let Some(event) = events.get(id) {
                push(&mut occurrence_bindings, &mut charge, event.site, *id)?;
            }
        }
        let mut owner_declarations = BTreeMap::new();
        for (position, declaration) in declarations.iter().enumerate() {
            push(
                &mut owner_declarations,
                &mut charge,
                declaration.declaration,
                position,
            )?;
        }
        let mut index = Self {
            _charge: charge,
            runtime_imports: BTreeMap::new(),
            source_imports,
            source_identifiers,
            source_references,
            source_scopes,
            scope_bindings,
            occurrence_bindings,
            owner_declarations,
            occurrences,
            scopes,
            declarations,
            bindings,
            events,
            targets,
            resolutions,
            references,
            imports,
            identifiers,
        };
        for event in index.events.values() {
            if let Some(special) = index.import_special(event) {
                index._charge.grow(192)?;
                index.runtime_imports.insert(event.id(), special);
            }
        }
        Ok(index)
    }
    fn scope(
        &self,
        source: Id<SourceArtifact>,
        scope: cpg_flow::Scope,
    ) -> Option<Id<LexicalScope>> {
        let mut found = self
            .source_scopes
            .get(&source)
            .into_iter()
            .flatten()
            .filter_map(|id| self.scopes.get(id))
            .filter(|s| {
                if s.kind != scope.kind {
                    return false;
                }
                let Some(owner) = self.occurrences.get(&s.owner) else {
                    return false;
                };
                if owner.source != source {
                    return false;
                }
                match scope.name {
                    None => scope.kind == LexicalScopeKind::Module,
                    Some(span)
                        if matches!(
                            scope.kind,
                            LexicalScopeKind::Function | LexicalScopeKind::Class
                        ) =>
                    {
                        self.owner_declarations
                            .get(&s.owner)
                            .into_iter()
                            .flatten()
                            .map(|position| &self.declarations[*position])
                            .any(|d| {
                                d.declaration == s.owner
                                    && self.occurrences.get(&d.name).is_some_and(|n| {
                                        n.start == span.start as i64 && n.end == span.end as i64
                                    })
                            })
                    }
                    Some(span) => owner.start == span.start as i64 && owner.end == span.end as i64,
                }
            });
        let first = found.next()?.id();
        found.next().is_none().then_some(first)
    }
    fn root(
        &self,
        scope: Id<LexicalScope>,
        place: &native::Place,
        occurrence: Id<Occurrence>,
    ) -> Result<PlaceRoot, ModelError> {
        let mut owner = scope;
        // Native scope is the evaluation scope, not necessarily the binding owner: a walrus
        // inside a comprehension binds in its enclosing scope. Exact target observations and
        // resolved reads select an owner only when their candidate binding scopes agree.
        let site = self
            .occurrences
            .get(&occurrence)
            .ok_or_else(|| invalid("flow place occurrence missing"))?;
        let mut target_scopes = self
            .occurrence_bindings
            .get(&occurrence)
            .into_iter()
            .flatten()
            .filter_map(|event| self.bindings.get(event))
            .filter(|binding| {
                self.events
                    .get(&binding.event)
                    .is_some_and(|event| event.name == place.root)
            })
            .map(|binding| binding.scope);
        if let Some(first) = target_scopes.next() {
            if target_scopes.any(|candidate| candidate != first) {
                return Err(invalid(
                    "flow binding target has conflicting lexical owners",
                ));
            }
            owner = first;
        }
        let root_read = self
            .source_references
            .get(&site.source)
            .into_iter()
            .flatten()
            .map(|position| &self.references[*position])
            .find(|r| {
                r.name == place.root
                    && self.occurrences.get(&r.read).is_some_and(|o| {
                        o.source == site.source && o.start == site.start && o.end <= site.end
                    })
            });
        if let Some(read) = root_read
            && let Some(resolutions) = self.resolutions.get(&read.read)
        {
            let agreed: Option<Id<LexicalScope>> =
                resolutions
                    .first()
                    .and_then(|r| match self.targets.get(&r.target) {
                        Some(LexicalTarget::Binding { event }) => {
                            self.bindings.get(event).map(|binding| binding.scope)
                        }
                        _ => None,
                    });
            if let Some(scope) = agreed
                && resolutions
                    .iter()
                    .all(|r| match self.targets.get(&r.target) {
                        Some(LexicalTarget::Binding { event }) => self
                            .bindings
                            .get(event)
                            .is_some_and(|binding| binding.scope == scope),
                        _ => false,
                    })
            {
                owner = scope;
            }
        }
        if let Some(binding) = self
            .scope_bindings
            .get(&owner)
            .into_iter()
            .flatten()
            .filter_map(|id| self.bindings.get(id))
            .find(|b| {
                b.scope == owner
                    && b.kind == BindingEventKind::Parameter
                    && self
                        .events
                        .get(&b.event)
                        .is_some_and(|e| e.name == place.root)
            })
        {
            return Ok(PlaceRoot::Formal {
                declaration: self.events[&binding.event].site,
            });
        }
        Ok(PlaceRoot::Local {
            scope: self
                .scopes
                .get(&owner)
                .ok_or_else(|| invalid("flow place scope missing"))?
                .owner,
            name: place.root.clone(),
        })
    }
    fn import_special(&self, event: &BindingEvent) -> Option<RuntimeSpecial> {
        let site = self.occurrences.get(&event.site)?;
        let source = site.source;
        let alias = self
            .source_imports
            .get(&source)
            .into_iter()
            .flatten()
            .map(|position| &self.imports[*position])
            .find(|i| {
                self.occurrences.get(&i.alias).is_some_and(|a| {
                    a.source == site.source && a.start <= site.start && a.end >= site.end
                })
            })?;
        let alias_site = self.occurrences.get(&alias.alias)?;
        let is_from =
            self.occurrences.get(&alias.statement)?.syntax_kind == SyntaxKind::StmtImportFrom;
        let imported = self
            .source_identifiers
            .get(&source)
            .into_iter()
            .flatten()
            .find(|id| {
                self.occurrences.get(*id).is_some_and(|o| {
                    o.source == alias_site.source
                        && o.start == alias_site.start
                        && o.end <= alias_site.end
                })
            })
            .and_then(|id| self.identifiers.get(id))
            .map(String::as_str);
        let special = match (alias.resolved_module.as_deref(), is_from, imported) {
            (Some("typing" | "typing_extensions"), true, Some("TYPE_CHECKING")) => {
                RuntimeSpecial::Checking
            }
            (Some("typing" | "typing_extensions"), false, _) => RuntimeSpecial::Typing,
            (Some("sys"), false, _) => RuntimeSpecial::Sys,
            (Some("os"), false, _) => RuntimeSpecial::Os,
            _ => return None,
        };
        Some(special)
    }
    fn runtime(&self, source: Id<SourceArtifact>) -> cpg_flow::RuntimeBindings {
        let mut out = cpg_flow::RuntimeBindings::default();
        for reference in self
            .source_references
            .get(&source)
            .into_iter()
            .flatten()
            .map(|position| &self.references[*position])
        {
            let Some(occurrence) = self
                .occurrences
                .get(&reference.read)
                .filter(|o| o.source == source)
            else {
                continue;
            };
            let Some(resolutions) = self.resolutions.get(&reference.read) else {
                continue;
            };
            let classify = |r: &LexicalResolution| match self.targets.get(&r.target) {
                Some(LexicalTarget::Binding { event }) => self.runtime_imports.get(event).copied(),
                Some(LexicalTarget::Builtin { name, .. }) if name == "type" => {
                    Some(RuntimeSpecial::Type)
                }
                Some(LexicalTarget::Builtin { name, .. })
                    if matches!(
                        name.as_str(),
                        "int"
                            | "str"
                            | "bool"
                            | "float"
                            | "bytes"
                            | "object"
                            | "list"
                            | "dict"
                            | "tuple"
                            | "set"
                    ) =>
                {
                    Some(RuntimeSpecial::Class)
                }
                _ => None,
            };
            let Some(special) = resolutions.first().and_then(classify) else {
                continue;
            };
            if !resolutions.iter().all(|r| classify(r) == Some(special)) {
                continue;
            }
            let span = Span {
                start: occurrence.start as u32,
                end: occurrence.end as u32,
            };
            match special {
                RuntimeSpecial::Checking => {
                    out.checking_names.insert(span);
                }
                RuntimeSpecial::Typing => {
                    out.typing_modules.insert(span);
                }
                RuntimeSpecial::Sys => {
                    out.sys_modules.insert(span);
                }
                RuntimeSpecial::Os => {
                    out.os_modules.insert(span);
                }
                RuntimeSpecial::Type => {
                    out.builtin_type.insert(span);
                }
                RuntimeSpecial::Class => {
                    out.builtin_classes
                        .entry(reference.name.clone())
                        .or_default()
                        .insert(span);
                }
            }
        }
        out
    }
}
impl<S: StageSink + 'static> ProviderStage<S> for TyFlow {
    fn run(&mut self, context: &mut StageContext<S>) -> Result<ProviderOutcome, ModelError> {
        macro_rules! declare {($($ty:ty),+)=>{$(context.declare::<$ty>()?;)+}}
        output_types!(declare);
        let index = Index::new(context)?;
        let provider = provider();
        context.contribute(provider.clone())?;
        let captured = context.captured();
        let mut partial = false;
        for input in captured.inputs() {
            let library = match input.acquisition() {
                Acquisition::Corpus { library, .. } => captured.inputs().get(*library),
                _ => None,
            };
            let analysis =
                crate::pyrefly_stage::analysis_context(input, library, captured.config())?;
            let version = crate::library::version_triple(&analysis.python_version)
                .map_err(ModelError::codec)?;
            let (run, families) = ProviderRun::new(
                provider.id(),
                analysis.id(),
                input.captured().revision().id(),
                analysis.config_digest,
                [FactFamily::Flow],
            )?;
            let surface = ProviderSurface {
                provider: provider.id(),
                family: FactFamily::Flow,
                name: "native ty semantic index".into(),
            };
            context.contribute(analysis.clone())?;
            context.contribute(run.clone())?;
            for family in families {
                context.contribute(family)?;
            }
            context.contribute(surface.clone())?;
            let roots = crate::pyrefly_stage::roots(input)?;
            let mut native_inputs = Vec::new();
            let mut admitted = Vec::new();
            let mut charge = StateCharge::new(context.budget(), "ty_flow_source_buffers");
            for artifact in roots {
                let scope = CoverageScope::Artifact {
                    artifact: artifact.id(),
                };
                context.contribute(scope.clone())?;
                if crate::typed_syntax::admit(artifact, self.limits).is_err() {
                    context.contribute(coverage(
                        &scope,
                        &provider,
                        &analysis,
                        &run,
                        CoverageStatus::Unavailable,
                        Some(ObligationKind::ResourceRefused),
                        None,
                    ))?;
                    partial = true;
                    continue;
                }
                charge.grow(artifact.byte_len as usize + artifact.path.len() + 128)?;
                let bytes = std::fs::read(input.captured().root().join(&artifact.path))
                    .map_err(ModelError::codec)?;
                if ContentHash::of(&bytes) != artifact.content {
                    return Err(invalid("ty input differs from capture"));
                }
                let text = match String::from_utf8(bytes) {
                    Ok(s) => s,
                    Err(_) => {
                        context.contribute(coverage(
                            &scope,
                            &provider,
                            &analysis,
                            &run,
                            CoverageStatus::Unavailable,
                            Some(ObligationKind::UndecodableSource),
                            None,
                        ))?;
                        partial = true;
                        continue;
                    }
                };
                native_inputs.push(cpg_flow::Input {
                    path: artifact.path.clone(),
                    text,
                    runtime: index.runtime(artifact.id()),
                });
                admitted.push(artifact);
            }
            let flows = cpg_flow::index(
                &native_inputs,
                &cpg_flow::RuntimeContext {
                    python_version: version,
                    platform: analysis.python_platform.clone(),
                },
            );
            for (artifact, flow) in admitted.into_iter().zip(flows) {
                let scope = CoverageScope::Artifact {
                    artifact: artifact.id(),
                };
                let mut writer = Writer {
                    context,
                    index: &index,
                    flow: &flow,
                    artifact,
                    analysis: &analysis,
                    scope: &scope,
                    run: &run,
                    surface: &surface,
                    partial: false,
                    atoms: BTreeMap::new(),
                    _charge: StateCharge::new(
                        &charge.budget().unwrap().clone(),
                        "ty_flow_atom_cache",
                    ),
                };
                if let Some(error) = &flow.error {
                    writer.boundary(None, ObligationKind::OutsideProviderModel, error)?;
                } else {
                    writer.write()?;
                }
                let status = if flow.error.is_some() {
                    CoverageStatus::Failed
                } else if writer.partial || flow.syntax_errors > 0 {
                    CoverageStatus::Partial
                } else {
                    CoverageStatus::CompleteUnderStatedModel
                };
                let reason = if flow.error.is_some() {
                    Some(ObligationKind::OutsideProviderModel)
                } else if flow.syntax_errors > 0 {
                    Some(ObligationKind::SyntaxError)
                } else if writer.partial {
                    Some(ObligationKind::OutsideProviderModel)
                } else {
                    None
                };
                partial |= status != CoverageStatus::CompleteUnderStatedModel;
                writer.context.contribute(coverage(
                    &scope,
                    &provider,
                    &analysis,
                    &run,
                    status,
                    reason,
                    Some(format!(
                        "native syntax errors {}; runtime reach skips {}; ty-false reach skips {}",
                        flow.syntax_errors,
                        flow.skips.reaching_runtime_view,
                        flow.skips.reaching_ty_false
                    )),
                ))?;
            }
        }
        Ok(if partial {
            ProviderOutcome::Partial
        } else {
            ProviderOutcome::Complete
        })
    }
}
fn coverage(
    scope: &CoverageScope,
    provider: &Provider,
    analysis: &AnalysisContext,
    run: &ProviderRun,
    status: CoverageStatus,
    reason: Option<ObligationKind>,
    diagnostic: Option<String>,
) -> ProviderCoverage {
    ProviderCoverage {
        scope: scope.id(),
        provider: Some(provider.id()),
        context: analysis.id(),
        family: FactFamily::Flow,
        run: Some(run.id()),
        status,
        reason,
        diagnostic,
    }
}
struct Writer<'a, S: StageSink + 'static> {
    context: &'a mut StageContext<S>,
    index: &'a Index,
    flow: &'a ModuleFlow,
    artifact: &'a SourceArtifact,
    analysis: &'a AnalysisContext,
    scope: &'a CoverageScope,
    run: &'a ProviderRun,
    surface: &'a ProviderSurface,
    partial: bool,
    atoms: BTreeMap<Id<EvaluationAtom>, EvaluationAtom>,
    _charge: StateCharge,
}
macro_rules! supported {
    ($writer:expr,$ty:ident,$support:ident,$row:expr,$occ:expr) => {{
        let row: $ty = $row;
        let evidence = Evidence::Occurrence { occurrence: $occ };
        $writer.context.contribute(evidence.clone())?;
        $writer.context.emit($support {
            assertion: row.id(),
            run: $writer.run.id(),
            surface: $writer.surface.id(),
            evidence: evidence.id(),
            origin: Origin::AnalyzerAssertion,
            mode: ExtractionMode::NativeTraversal,
            fidelity: Fidelity::NativeStructural,
        })?;
        $writer.context.emit(row)?;
    }};
}
impl<S: StageSink + 'static> Writer<'_, S> {
    fn boundary(
        &mut self,
        occurrence: Option<Id<Occurrence>>,
        reason: ObligationKind,
        detail: &str,
    ) -> Result<(), ModelError> {
        self.partial = true;
        self.context.contribute(SubjectBoundary {
            scope: self.scope.id(),
            provider: self.run.provider,
            context: self.analysis.id(),
            family: FactFamily::Flow,
            subject: occurrence,
            reason,
            detail: Some(detail.into()),
        })
    }
    fn attach(
        &mut self,
        span: Span,
        kind: Option<SyntaxKind>,
    ) -> Result<Option<Id<Occurrence>>, ModelError> {
        let mut shapes: Vec<_> = self
            .flow
            .nodes
            .iter()
            .filter(|n| {
                n.span == span
                    && n.kind
                        .is_some_and(|k| kind.is_none_or(|expected| expected == k))
            })
            .map(|n| (n.kind.unwrap(), n.role))
            .collect();
        // Expression and semantic event coordinates exclude scalar token wrappers unless explicitly
        // requested. The native kind distinguishes identical spans (Parameter/Identifier, etc.).
        if kind.is_none() {
            shapes.retain(|(k, _)| {
                (27..=59).contains(&(*k as i16)) || matches!(k, SyntaxKind::MatchCase)
            });
        }
        shapes.sort_by_key(|(k, r)| (*k as i16, *r as i16));
        shapes.dedup();
        if shapes.len() != 1 {
            self.boundary(
                None,
                ObligationKind::OutsideProviderModel,
                "native coordinate does not identify one syntax kind/role",
            )?;
            return Ok(None);
        }
        let (kind, role) = shapes[0];
        let query = lctx_model::domain::attachment::AttachmentQuery {
            source: self.artifact.id(),
            start: span.start as i64,
            end: span.end as i64,
            syntax_kind: kind,
            role,
            structural_path: None,
        };
        match self.context.attacher()?.attach(&query)? {
            Attached::Exact(id) => Ok(Some(id)),
            Attached::Unattached(result) => {
                use lctx_model::domain::attachment::Attachment;
                let (status, candidates) = match result.value() {
                    Attachment::Innermost(id) => (AttachmentKind::Innermost, vec![*id]),
                    Attachment::Ambiguous(ids) => (AttachmentKind::Ambiguous, ids.clone()),
                    Attachment::Unmatched => (AttachmentKind::Unmatched, vec![]),
                    Attachment::BudgetExceeded => (AttachmentKind::BudgetExceeded, vec![]),
                    Attachment::Exact(_) => unreachable!(),
                };
                let boundary = SubjectBoundary {
                    scope: self.scope.id(),
                    provider: self.run.provider,
                    context: self.analysis.id(),
                    family: FactFamily::Flow,
                    subject: None,
                    reason: status.reason(),
                    detail: Some("native flow coordinate did not attach exactly".into()),
                };
                let outcome = AttachmentOutcome {
                    boundary: boundary.id(),
                    source: self.artifact.id(),
                    start: query.start,
                    end: query.end,
                    syntax_kind: Some(kind),
                    role: Some(role),
                    outcome: status,
                };
                self.partial = true;
                self.context.contribute(boundary)?;
                self.context.contribute(outcome.clone())?;
                for candidate in candidates {
                    self.context.contribute(AttachmentCandidate {
                        outcome: outcome.id(),
                        candidate,
                    })?;
                }
                Ok(None)
            }
        }
    }
    fn qualify(
        &mut self,
        graph: &cpg_flow::Condition,
        scope: Id<LexicalScope>,
        subject: Option<Id<Occurrence>>,
    ) -> Result<Option<AssertionQualification>, ModelError> {
        let graph_bytes = graph.nodes.len().saturating_mul(256).saturating_add(
            graph
                .leaves()
                .map(|leaf| (size_of::<native::Atom>() + leaf.heap_bytes() + 128).saturating_mul(4))
                .sum::<usize>(),
        );
        let _mapped = self
            .context
            .budget()
            .reserve("ty_condition_mapping", graph_bytes)?;
        let mut mapped = HashMap::new();
        for leaf in graph.leaves() {
            let Some(atom) = self.atom(leaf, scope)? else {
                self.boundary(
                    subject,
                    ObligationKind::NativeUnavailable,
                    "condition includes an unattached or synthetic native evaluation",
                )?;
                return Ok(None);
            };
            mapped.insert(leaf.clone(), atom);
        }
        let graph = graph.map(|leaf| mapped[leaf]);
        let lowered = match Diagram::from_graph(&graph) {
            Ok(v) => v,
            Err(boundary) => {
                self.boundary(
                    subject,
                    lctx_model::domain::obligation::from_kernel(boundary),
                    &format!("condition kernel refused {boundary:?}"),
                )?;
                return Ok(None);
            }
        };
        let (condition, nodes) = lowered.diagram.records();
        self.context.contribute(condition.clone())?;
        for node in nodes {
            self.context.contribute(node)?;
        }
        let q = AssertionQualification {
            context: self.analysis.id(),
            scope: self.scope.id(),
            condition: condition.id(),
            modality: Modality::Definite,
            approximation: lowered.approximation,
        };
        self.context.contribute(q.clone())?;
        Ok(Some(q))
    }
    fn literal(&mut self, value: &native::Value) -> Result<Id<Literal>, ModelError> {
        let literal = match value {
            native::Value::None => Literal::None,
            native::Value::Bool(value) => Literal::Bool { value: *value },
            native::Value::Int(value) => Literal::Integer {
                decimal: value.to_string(),
            },
            native::Value::Str(value) => Literal::String {
                value: value.clone().into(),
            },
        };
        self.context.contribute(literal.clone())?;
        Ok(literal.id())
    }
    fn place(
        &mut self,
        place: &native::Place,
        scope: Id<LexicalScope>,
        occurrence: Id<Occurrence>,
    ) -> Result<Id<Place>, ModelError> {
        let root = self.index.root(scope, place, occurrence)?;
        self.context.contribute(root.clone())?;
        let mut path = AccessPath::empty();
        for segment in &place.segments {
            let segment = match segment {
                native::PlaceSegment::Attribute(name) => {
                    PathSegment::Attribute { name: name.clone() }
                }
                native::PlaceSegment::Item(value) => PathSegment::Item {
                    key: self.literal(value)?,
                },
            };
            self.context.contribute(segment.clone())?;
            path = path.extend(segment.id());
        }
        self.context.contribute(path.clone())?;
        let row = Place {
            root: root.id(),
            path: path.id(),
        };
        self.context.contribute(row.clone())?;
        Ok(row.id())
    }
    fn atom(
        &mut self,
        leaf: &native::Atom,
        scope: Id<LexicalScope>,
    ) -> Result<Option<Id<EvaluationAtom>>, ModelError> {
        let Some(span) = leaf.site() else {
            self.boundary(
                None,
                ObligationKind::NativeUnavailable,
                "native synthetic predicate has no source coordinate",
            )?;
            return Ok(None);
        };
        let Some(evaluation) = self.attach(span, None)? else {
            return Ok(None);
        };
        let predicate = match leaf.predicate() {
            native::Atom::IsNone { .. } => Predicate::IsNone,
            native::Atom::Truthy { .. } => Predicate::Truthy,
            native::Atom::IsValue { value, .. } => Predicate::IsValue {
                value: self.literal(value)?,
            },
            native::Atom::Equals { value, .. } => Predicate::Equals {
                value: self.literal(value)?,
            },
            native::Atom::MemberOf { values, .. } => {
                let ids = values
                    .iter()
                    .map(|v| self.literal(v))
                    .collect::<Result<Vec<_>, _>>()?;
                let (set, members) = LiteralSet::of(ids);
                self.context.contribute(set.clone())?;
                for member in members {
                    self.context.contribute(member)?;
                }
                Predicate::MemberOf { values: set.id() }
            }
            native::Atom::IsInstance { class, .. } => Predicate::IsInstance {
                class_expression: class.clone(),
            },
            native::Atom::TypeIs { class, .. } => Predicate::TypeIs {
                class_expression: class.clone(),
            },
            native::Atom::NonTerminalCall { awaiting } => Predicate::NonTerminalCall { awaiting:*awaiting },
            native::Atom::NonEmptyIterable => Predicate::NonEmptyIterable,
            native::Atom::ContextManagerSuppresses { asynchronous } => Predicate::ContextManagerSuppresses { asynchronous:*asynchronous },
            native::Atom::FinallyNormalPathImpossible => Predicate::FinallyNormalPathImpossible,
            native::Atom::Opaque { text } => Predicate::Opaque { text: text.clone() },
            native::Atom::Evaluated { .. } => {
                return Err(invalid("nested native evaluation wrapper"));
            }
        };
        self.context.contribute(predicate.clone())?;
        // Native operand selection follows translator structure; pattern subjects remain outside
        // their case evaluation until the explicit subject bridge is implemented in phase 3.
        let operand_span = self
            .flow
            .test_leaves
            .iter()
            .find(|l| l.atom == *leaf)
            .and_then(|l| l.operand_span);
        let operand = if let Some(span) = operand_span {
            if let (Some(place), Some(occurrence)) = (
                self.flow.places.get(&span).cloned(),
                self.attach(span, None)?,
            ) {
                Some(self.place(&place, scope, occurrence)?)
            } else {
                None
            }
        } else {
            None
        };
        let atom = EvaluationAtom {
            evaluation,
            context: self.analysis.id(),
            predicate: predicate.id(),
            operand,
        };
        if !self.atoms.contains_key(&atom.id()) {
            self._charge.grow(
                size_of::<EvaluationAtom>()
                    + atom.heap_bytes()
                    + size_of::<Id<EvaluationAtom>>()
                    + 128,
            )?;
            self.atoms.insert(atom.id(), atom.clone());
            self.context.contribute(atom.clone())?;
        }
        Ok(Some(atom.id()))
    }
    fn write(&mut self) -> Result<(), ModelError> {
        let flow = self.flow;
        if let (Some(original_content),Some(view_content),Some(byte_len))=(flow.original_content,flow.view_content,flow.view_byte_len) {
            if original_content!=self.artifact.content || byte_len!=self.artifact.byte_len as usize {
                return Err(invalid("ty view does not preserve captured source byte geometry"));
            }
            let (condition,nodes)=Diagram::always().records();
            self.context.contribute(condition.clone())?;
            for node in nodes {self.context.contribute(node)?;}
            let qualification=AssertionQualification { context:self.analysis.id(),scope:self.scope.id(),condition:condition.id(),modality:Modality::Definite,approximation:Approximation::Exact };
            self.context.contribute(qualification.clone())?;
            let row=FlowSourceViewObservation { qualification:qualification.id(),source:self.artifact.id(),original_content,view_content,byte_len:byte_len as i64,renamed_type_checking:i64::from(flow.renamed) };
            row.validate()?;
            let evidence=Evidence::SourceSpan {source:self.artifact.id(),start:0,end:byte_len as i64};
            self.context.contribute(evidence.clone())?;
            self.context.emit(FlowSourceViewSupport { assertion:row.id(),run:self.run.id(),surface:self.surface.id(),evidence:evidence.id(),origin:Origin::AnalyzerAssertion,mode:ExtractionMode::NativeTraversal,fidelity:Fidelity::NativeStructural })?;
            self.context.emit(row)?;
        } else {return Err(invalid("native ty source view receipt is missing"));}
        let mut uses = Vec::new();
        let mut definitions = Vec::new();
        let mut rows_charge = StateCharge::new(self.context.budget(), "ty_flow_nominal_rows");
        for native in &flow.uses {
            let row = if let (Some(scope), Some(occurrence)) = (
                self.index.scope(self.artifact.id(), native.scope),
                self.attach(native.span, None)?,
            ) {
                if let Some(place) = flow.places.get(&native.span) {
                    let row = FlowUse {
                        occurrence,
                        place: self.place(place, scope, occurrence)?,
                    };
                    self.context.emit(row.clone())?;
                    if let Some(q) =
                        self.qualify(&cpg_flow::Condition::always(), scope, Some(occurrence))?
                    {
                        supported!(
                            self,
                            FlowUseObservation,
                            FlowUseSupport,
                            FlowUseObservation {
                                qualification: q.id(),
                                use_: row.id(),
                                scope,
                                annotation: native.annotation
                            },
                            occurrence
                        );
                    }
                    Some((row, scope))
                } else {
                    self.boundary(
                        Some(occurrence),
                        ObligationKind::OutsideProviderModel,
                        "native place shape unavailable",
                    )?;
                    None
                }
            } else {
                self.boundary(
                    None,
                    ObligationKind::ScopeBoundary,
                    "native flow scope or use unattached",
                )?;
                None
            };
            rows_charge.grow(256)?;
            uses.push(row);
        }
        for native in &flow.defs {
            let kind = match native.kind {
                BindingEventKind::Parameter
                | BindingEventKind::FunctionDef
                | BindingEventKind::ClassDef
                | BindingEventKind::Import
                | BindingEventKind::FromImport
                | BindingEventKind::MatchCapture
                | BindingEventKind::ExceptHandler
                | BindingEventKind::TypeParam => Some(SyntaxKind::Identifier),
                _ => None,
            };
            let row = if let (Some(scope), Some(occurrence)) = (
                self.index.scope(self.artifact.id(), native.scope),
                self.attach(native.target, kind)?,
            ) {
                let place = flow.places.get(&native.target).cloned().or_else(|| {
                    (!native.place.contains(['.', '[', ']'])).then(|| native::Place {
                        root: native.place.clone(),
                        segments: vec![],
                    })
                });
                if let Some(place) = place {
                    let value = match native.value {
                        Some(span) => self.attach(span, None)?,
                        None => None,
                    };
                    let row = FlowDefinition {
                        occurrence,
                        place: self.place(&place, scope, occurrence)?,
                    };
                    self.context.emit(row.clone())?;
                    if let Some(q) =
                        self.qualify(&cpg_flow::Condition::always(), scope, Some(occurrence))?
                    {
                        supported!(
                            self,
                            FlowDefinitionObservation,
                            FlowDefinitionSupport,
                            FlowDefinitionObservation {
                                qualification: q.id(),
                                definition: row.id(),
                                scope,
                                kind: native.kind,
                                value
                            },
                            occurrence
                        );
                    }
                    Some((row, scope))
                } else {
                    self.boundary(
                        Some(occurrence),
                        ObligationKind::OutsideProviderModel,
                        "native definition place shape unavailable",
                    )?;
                    None
                }
            } else {
                self.boundary(
                    None,
                    ObligationKind::ScopeBoundary,
                    "native flow scope or definition unattached",
                )?;
                None
            };
            rows_charge.grow(256)?;
            definitions.push(row);
        }
        for native in &flow.reaching {
            let Some(Some((use_, scope))) = uses.get(native.use_ix as usize) else {
                continue;
            };
            let target = if native.nested {
                ReachingDefinition::Nested
            } else if let Some(index) = native.def_ix {
                let Some(Some((definition, _))) = definitions.get(index as usize) else {
                    self.boundary(
                        Some(use_.occurrence),
                        ObligationKind::NativeUnavailable,
                        "native reaching definition unattached",
                    )?;
                    continue;
                };
                if definition.place != use_.place {
                    return Err(invalid(&format!(
                        "native reaching place mismatch in {}: use {:?}; definition {:?}; typed use {:?}; typed definition {:?}",
                        self.artifact.path,
                        flow.uses[native.use_ix as usize],
                        flow.defs[index as usize],
                        use_,
                        definition
                    )));
                }
                ReachingDefinition::Bound {
                    definition: definition.id(),
                }
            } else {
                ReachingDefinition::Unbound
            };
            self.context.emit(target.clone())?;
            if native.narrowing_precision_lost {
                self.boundary(Some(use_.occurrence),ObligationKind::ResourceRefused,"native narrowing scope lost precision; terminal true is over-approximate")?;
            }
            if let Some(q)=self.qualify(&native.narrowing,*scope,Some(use_.occurrence))? {
                supported!(self,FlowNarrowingObservation,FlowNarrowingSupport,FlowNarrowingObservation {
                    qualification:q.id(),use_:use_.id(),target:target.id(),precision_lost:native.narrowing_precision_lost
                },use_.occurrence);
            }
            if let Some(q) = self.qualify(&native.condition, *scope, Some(use_.occurrence))? {
                supported!(
                    self,
                    FlowReachingObservation,
                    FlowReachingSupport,
                    FlowReachingObservation {
                        qualification: q.id(),
                        use_: use_.id(),
                        target: target.id(),
                        loop_carried: native.loop_carried
                    },
                    use_.occurrence
                );
            }
        }
        for native in &flow.values {
            let Some(Some((use_, scope))) = uses.get(native.use_ix as usize) else {
                continue;
            };
            let Some(sink) = self.attach(native.span, None)? else {
                continue;
            };
            let Some(q) = self.qualify(&native.condition, *scope, Some(sink))? else {
                continue;
            };
            let mut steps = Vec::new();
            let mut attached = true;
            for frame in &native.call_path {
                match (
                    self.attach(frame.call, Some(SyntaxKind::ExprCall))?,
                    self.attach(frame.operand, None)?,
                ) {
                    (Some(call), Some(operand)) => steps.push((call, operand, frame.role)),
                    _ => {
                        attached = false;
                        break;
                    }
                }
            }
            if !attached {
                continue;
            }
            let kind = match native.sink {
                cpg_flow::Sink::Definition => FlowSinkKind::Definition,
                cpg_flow::Sink::Argument => FlowSinkKind::Argument,
                cpg_flow::Sink::Return => FlowSinkKind::Return,
                cpg_flow::Sink::Yield => FlowSinkKind::Yield,
                cpg_flow::Sink::Raise => FlowSinkKind::Raise,
            };
            let value = FlowValueObservation {
                qualification: q.id(),
                use_: use_.id(),
                sink,
                kind,
                transfer: if native.identity {
                    lctx_model::domain::transfer::TransferKind::Identity
                } else {
                    lctx_model::domain::transfer::TransferKind::Derived
                },
                through_call: !steps.is_empty(),
            };
            supported!(
                self,
                FlowValueObservation,
                FlowValueSupport,
                value.clone(),
                sink
            );
            if !steps.is_empty() {
                let (path, steps) = FlowCallPath::new(&steps)?;
                self.context.emit(path.clone())?;
                for step in steps {
                    self.context.emit(step)?;
                }
                supported!(
                    self,
                    FlowValuePathObservation,
                    FlowValuePathSupport,
                    FlowValuePathObservation {
                        qualification: q.id(),
                        value: value.id(),
                        path: path.id()
                    },
                    sink
                );
            }
        }
        for native in &flow.regions {
            if let Some(scope) = self.index.scope(self.artifact.id(), native.scope) {
                let kinds: Vec<_> = flow
                    .nodes
                    .iter()
                    .filter(|n| {
                        n.span == native.span
                            && n.kind.is_some_and(|k| (2..=26).contains(&(k as i16)))
                    })
                    .map(|n| n.kind.unwrap())
                    .collect();
                if let [kind] = kinds.as_slice() {
                    if let Some(statement) = self.attach(native.span, Some(*kind))?
                        && let Some(q) = self.qualify(&native.condition, scope, Some(statement))?
                    {
                        supported!(
                            self,
                            FlowRegionObservation,
                            FlowRegionSupport,
                            FlowRegionObservation {
                                qualification: q.id(),
                                statement,
                                scope
                            },
                            statement
                        );
                    }
                } else {
                    self.boundary(
                        None,
                        ObligationKind::OutsideProviderModel,
                        "native statement coordinate missing",
                    )?;
                }
            } else {
                self.boundary(
                    None,
                    ObligationKind::ScopeBoundary,
                    "native region scope unattached",
                )?;
            }
        }
        for native in &flow.tests {
            if let Some(scope) = self.index.scope(self.artifact.id(), native.scope)
                && let Some(test) = self.attach(native.span, None)?
                && let Some(q) = self.qualify(&native.condition, scope, Some(test))?
            {
                supported!(
                    self,
                    FlowTestObservation,
                    FlowTestSupport,
                    FlowTestObservation {
                        qualification: q.id(),
                        test,
                        scope
                    },
                    test
                );
            }
        }
        for native in &flow.test_leaves {
            if let Some(scope) = self.index.scope(self.artifact.id(), native.scope)
                && let Some(test) = self.attach(native.test_span, None)?
                && let (Some(atom), Some(q)) = (
                    self.atom(&native.atom, scope)?,
                    self.qualify(&native.condition, scope, Some(test))?,
                )
            {
                let operand = match native.operand_span {
                    Some(span) => self.attach(span, None)?,
                    None => None,
                };
                supported!(
                    self,
                    FlowTestLeafObservation,
                    FlowTestLeafSupport,
                    FlowTestLeafObservation {
                        qualification: q.id(),
                        test,
                        atom,
                        operand
                    },
                    test
                );
            }
        }
        for native in &flow.attribute_loads {
            if let Some(occurrence) = self.attach(native.span, None)? {
                let scope = self
                    .index
                    .scopes
                    .values()
                    .find(|s| {
                        s.kind == LexicalScopeKind::Module
                            && self
                                .index
                                .occurrences
                                .get(&s.owner)
                                .is_some_and(|o| o.source == self.artifact.id())
                    })
                    .map(Record::id);
                if let Some(scope) = scope
                    && let Some(q) =
                        self.qualify(&cpg_flow::Condition::always(), scope, Some(occurrence))?
                {
                    supported!(
                        self,
                        FlowAttributeLoadObservation,
                        FlowAttributeLoadSupport,
                        FlowAttributeLoadObservation {
                            qualification: q.id(),
                            occurrence,
                            name: native.name.clone()
                        },
                        occurrence
                    );
                }
            }
        }
        Ok(())
    }
}
