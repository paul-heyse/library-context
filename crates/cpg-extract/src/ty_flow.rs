//! Native ty coordinates become facts only through exact typed attachment. No provider-local
//! index, predicate key, rendered condition or display-place identity is stored.
use crate::bundle::ProviderSink;
use crate::{
    acquisition::Acquisition,
    assembly::Attached,
    bundle::{Declared, ProviderStage, StageContext},
    typed_syntax::SyntaxLimits,
};
use cpg_flow::{ModuleEvent, Span, native};
use lctx_model::domain::producer_contract::ProducerContract;
pub use lctx_model::domain::producer_contract::facts::TY_FLOW;
use lctx_model::domain::{
    assertion::*,
    attribution::*,
    charged::StateCharge,
    conditions::*,
    flow::*,
    flow_capture::*,
    flow_inventory::*,
    lexical::*,
    source::*,
    stages::{Profile, ProviderOutcome, Stage},
    syntax::*,
    value::*,
    *,
};
use std::collections::{BTreeMap, HashMap};
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
            include_str!("../../lctx-model/src/domain/flow_inventory.rs"),
            // This adapter's facts also depend on the provider's native mapping.
            include_str!("../../cpg-flow/src/lib.rs"),
            include_str!("../../cpg-flow/src/events.rs"),
            include_str!("../../cpg-flow/src/admitted.rs"),
            include_str!("../../cpg-flow/src/narrowing.rs"),
            include_str!("../../cpg-flow/src/native.rs"),
            include_str!("../../cpg-flow/src/db.rs"),
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
            FlowUseInventoryObservation,
            FlowUseInventorySupport,
            FlowUseCandidate,
            FlowUseInventoryMember,
            FlowCaptureTarget,
            FlowCaptureInventory,
            FlowCaptureCandidate,
            FlowCaptureTimingObservation,
            FlowCaptureTimingSupport,
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
pub fn contract() -> ProducerContract {
    lctx_model::domain::producer_contract::facts::ty_flow()
}
impl Declared for TyFlow {
    fn declaration(&self, _: Profile) -> Stage {
        let provider = provider();
        contract().bind(
            provider.build_digest,
            ContentHash::of(format!("{:?}", self.limits).as_bytes()),
            [("ty-flow", provider)],
        )
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
struct Occurrences {
    prepared: crate::bundle::PreparedOccurrences,
    positions: BTreeMap<Id<Occurrence>, usize>,
}
impl Occurrences {
    fn get(&self, id: &Id<Occurrence>) -> Option<&Occurrence> {
        self.positions
            .get(id)
            .map(|position| &self.prepared.rows[*position])
    }
}
struct Index {
    _charge: StateCharge,
    occurrences: Occurrences,
    scopes: BTreeMap<Id<LexicalScope>, LexicalScope>,
    declarations: Vec<DeclarationObservation>,
    bindings: BTreeMap<Id<BindingEvent>, BindingObservation>,
    events: BTreeMap<Id<BindingEvent>, BindingEvent>,
    targets: BTreeMap<Id<LexicalTarget>, LexicalTarget>,
    resolutions: BTreeMap<Id<Occurrence>, Vec<LexicalResolution>>,
    runtime_imports: BTreeMap<Id<BindingEvent>, RuntimeSpecial>,
    source_references: SourceIndex<ReferenceObservation>,
    source_scopes: SourceIndex<Id<LexicalScope>>,
    scope_bindings: BTreeMap<Id<LexicalScope>, Vec<Id<BindingEvent>>>,
    occurrence_bindings: BTreeMap<Id<Occurrence>, Vec<Id<BindingEvent>>>,
    owner_declarations: BTreeMap<Id<Occurrence>, Vec<usize>>,
}
/// Only import classification needs these source spelling/alias preparations. They do not
/// survive construction of the compact cross-source binding classification map.
struct RuntimePreparation {
    identifiers: BTreeMap<Id<Occurrence>, String>,
    imports: Vec<ImportAliasObservation>,
    source_imports: SourceIndex<usize>,
    source_identifiers: SourceIndex<Id<Occurrence>>,
    _charge: StateCharge,
}
impl RuntimePreparation {
    fn import_special(
        &self,
        event: &BindingEvent,
        occurrences: &Occurrences,
    ) -> Option<RuntimeSpecial> {
        let site = occurrences.get(&event.site)?;
        let source = site.source;
        let alias = self
            .source_imports
            .get(&source)
            .into_iter()
            .flatten()
            .map(|position| &self.imports[*position])
            .find(|i| {
                occurrences.get(&i.alias).is_some_and(|a| {
                    a.source == site.source && a.start <= site.start && a.end >= site.end
                })
            })?;
        let alias_site = occurrences.get(&alias.alias)?;
        let is_from = occurrences.get(&alias.statement)?.syntax_kind == SyntaxKind::StmtImportFrom;
        let imported = self
            .source_identifiers
            .get(&source)
            .into_iter()
            .flatten()
            .find(|id| {
                occurrences.get(id).is_some_and(|o| {
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
}
impl Index {
    fn new<S: ProviderSink + 'static>(context: &mut StageContext<S>) -> Result<Self, ModelError> {
        let mut charge = StateCharge::new(context.budget(), "ty_flow_indexes");
        fn read<R: Record, S: ProviderSink + 'static>(
            context: &mut StageContext<S>,
            charge: &mut StateCharge,
        ) -> Result<Vec<R>, ModelError> {
            let batches = context.input::<R>()?;
            let mut rows = Vec::new();
            for batch in batches {
                let batch = batch?;
                for row in batch.rows() {
                    charge.grow((size_of::<R>() + row.heap_bytes() + 128).saturating_mul(4))?;
                    rows.push(row.clone());
                }
            }
            Ok(rows)
        }
        let prepared = context.prepare_occurrence_input()?;
        let mut positions = BTreeMap::new();
        for (position, row) in prepared.rows.iter().enumerate() {
            charge.grow(size_of::<Id<Occurrence>>() + size_of::<usize>() + 128)?;
            if positions.insert(row.id(), position).is_some() {
                return Err(ModelError::Conflict(Occurrence::NAME));
            }
        }
        let occurrences = Occurrences {
            prepared,
            positions,
        };
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
        let mut source_references = BTreeMap::new();
        // Partition the one declared stream directly, retaining encounter order (including
        // duplicate references) without a global row vector plus position vectors.
        for batch in context.input::<ReferenceObservation>()? {
            for reference in batch?.rows() {
                if let Some(site) = occurrences.get(&reference.read) {
                    charge.grow(
                        (size_of::<ReferenceObservation>() + reference.heap_bytes() + 128)
                            .saturating_mul(4),
                    )?;
                    push(
                        &mut source_references,
                        &mut charge,
                        site.source,
                        reference.clone(),
                    )?;
                }
            }
        }
        let mut preparation_charge =
            StateCharge::new(context.budget(), "ty_flow_runtime_preparation");
        let imports: Vec<ImportAliasObservation> = read(context, &mut preparation_charge)?;
        let identifiers: BTreeMap<_, _> =
            read::<SyntaxObservation, _>(context, &mut preparation_charge)?
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
                push(
                    &mut source_imports,
                    &mut preparation_charge,
                    site.source,
                    position,
                )?;
            }
        }
        let mut source_identifiers = BTreeMap::new();
        for id in identifiers.keys() {
            if let Some(site) = occurrences.get(id) {
                push(
                    &mut source_identifiers,
                    &mut preparation_charge,
                    site.source,
                    *id,
                )?;
            }
        }
        let preparation = RuntimePreparation {
            _charge: preparation_charge,
            identifiers,
            imports,
            source_imports,
            source_identifiers,
        };
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
        };
        for event in index.events.values() {
            if let Some(special) = preparation.import_special(event, &index.occurrences) {
                index._charge.grow(192)?;
                index.runtime_imports.insert(event.id(), special);
            }
        }
        // The compact classification map has its own Index charge. No import/spelling
        // preparation or its reservation remains live during native module collection.
        drop(preparation);
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
    fn runtime(
        &self,
        source: Id<SourceArtifact>,
        charge: &mut StateCharge,
    ) -> Result<cpg_flow::RuntimeBindings, ModelError> {
        fn span(
            set: &mut std::collections::BTreeSet<Span>,
            value: Span,
            charge: &mut StateCharge,
        ) -> Result<(), ModelError> {
            if !set.contains(&value) {
                charge.grow(cpg_flow::RuntimeBindings::SPAN_ENTRY_BYTES)?;
                set.insert(value);
            }
            Ok(())
        }
        let mut out = cpg_flow::RuntimeBindings::default();
        for reference in self.source_references.get(&source).into_iter().flatten() {
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
            let location = Span {
                start: occurrence.start as u32,
                end: occurrence.end as u32,
            };
            match special {
                RuntimeSpecial::Checking => {
                    span(&mut out.checking_names, location, charge)?;
                }
                RuntimeSpecial::Typing => {
                    span(&mut out.typing_modules, location, charge)?;
                }
                RuntimeSpecial::Sys => {
                    span(&mut out.sys_modules, location, charge)?;
                }
                RuntimeSpecial::Os => {
                    span(&mut out.os_modules, location, charge)?;
                }
                RuntimeSpecial::Type => {
                    span(&mut out.builtin_type, location, charge)?;
                }
                RuntimeSpecial::Class => {
                    if let Some(set) = out.builtin_classes.get_mut(&reference.name) {
                        span(set, location, charge)?;
                    } else {
                        charge.grow(
                            cpg_flow::RuntimeBindings::CLASS_ENTRY_BYTES
                                .saturating_add(reference.name.len())
                                .saturating_add(cpg_flow::RuntimeBindings::SPAN_ENTRY_BYTES),
                        )?;
                        let mut set = std::collections::BTreeSet::new();
                        set.insert(location);
                        out.builtin_classes.insert(reference.name.clone(), set);
                    }
                }
            }
        }
        Ok(out)
    }
}
/// Read only the captured geometry. Both retained source backing and temporary path growth
/// are admitted before allocation; a changed file cannot grow an unbounded read buffer.
fn read_captured_source(
    artifact: &SourceArtifact,
    root: &std::path::Path,
    charge: &mut StateCharge,
) -> Result<Vec<u8>, ModelError> {
    use std::io::Read;
    let byte_len =
        usize::try_from(artifact.byte_len).map_err(|_| invalid("invalid ty source byte length"))?;
    let path_bytes = root
        .as_os_str()
        .len()
        .saturating_add(artifact.path.len())
        .saturating_add(8)
        .saturating_mul(2);
    charge.grow(
        byte_len
            .saturating_add(artifact.path.len())
            .saturating_add(128)
            .saturating_add(path_bytes),
    )?;
    let mut file = std::fs::File::open(root.join(&artifact.path)).map_err(ModelError::codec)?;
    charge.release(path_bytes);
    let mut bytes = vec![0; byte_len];
    file.read_exact(&mut bytes).map_err(|error| {
        if error.kind() == std::io::ErrorKind::UnexpectedEof {
            invalid("ty input shrank below captured geometry")
        } else {
            ModelError::codec(error)
        }
    })?;
    let mut extra = [0];
    if file.read(&mut extra).map_err(ModelError::codec)? != 0 {
        return Err(invalid("ty input grew beyond captured geometry"));
    }
    if ContentHash::of(&bytes) != artifact.content {
        return Err(invalid("ty input differs from capture"));
    }
    Ok(bytes)
}

impl<S: ProviderSink + 'static> ProviderStage<S> for TyFlow {
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
            let mut charge = StateCharge::new(context.budget(), "ty_flow_input_slots");
            let mut native_inputs = Vec::new();
            let mut admitted = Vec::new();
            fn reserve_slot<T>(
                values: &mut Vec<T>,
                charge: &mut StateCharge,
            ) -> Result<(), ModelError> {
                if values.len() == values.capacity() {
                    let capacity = values.capacity().saturating_mul(2).max(4);
                    charge.grow((capacity - values.capacity()).saturating_mul(size_of::<T>()))?;
                    values.reserve_exact(capacity - values.len());
                }
                Ok(())
            }
            for artifact in roots {
                context.check_cancelled()?;
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
                let mut source_charge = StateCharge::new(context.budget(), "ty_flow_source_buffer");
                let bytes =
                    read_captured_source(artifact, input.captured().root(), &mut source_charge)?;
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
                let runtime = index.runtime(artifact.id(), &mut source_charge)?;
                let native_input = cpg_flow::AdmittedInput::new(
                    cpg_flow::Input {
                        path: artifact.path.clone(),
                        text,
                        runtime,
                    },
                    source_charge,
                )?;
                reserve_slot(&mut native_inputs, &mut charge)?;
                reserve_slot(&mut admitted, &mut charge)?;
                native_inputs.push(native_input);
                admitted.push(artifact);
            }
            context.check_cancelled()?;
            let budget = context.budget().clone();
            let control = context.control();
            let mut module: Option<ModuleState> = None;
            cpg_flow::visit_modules(
                native_inputs,
                &cpg_flow::RuntimeContext {
                    python_version: version,
                    platform: analysis.python_platform.clone(),
                },
                &budget,
                &|| control.check(),
                |token| control.register(move || token.cancel()),
                |input_index, event| {
                    context.check_cancelled()?;
                    let artifact = admitted.get(input_index).copied()
                        .ok_or_else(|| invalid("native flow input index out of range"))?;
                    let scope = CoverageScope::Artifact { artifact: artifact.id() };
                    let event = match event {
                        ModuleEvent::Prelude(prelude) => {
                            if module.is_some() { return Err(invalid("native flow modules overlap")); }
                            let mut state = ModuleState::new(input_index, &budget);
                            state.prelude = Some(prelude);
                            let mut writer = Writer { context, index: &index, state: &mut state,
                                artifact, analysis: &analysis, scope: &scope, run: &run, surface: &surface };
                            writer.source_view()?;
                            module = Some(state);
                            return Ok(());
                        }
                        ModuleEvent::ModuleFailure(failure) => {
                            if module.is_some() { return Err(invalid("native module failure follows preamble")); }
                            let mut state = ModuleState::new(input_index, &budget);
                            let mut writer = Writer { context, index: &index, state: &mut state,
                                artifact, analysis: &analysis, scope: &scope, run: &run, surface: &surface };
                            writer.boundary(None, ObligationKind::OutsideProviderModel, &failure.detail)?;
                            writer.context.contribute(coverage(&scope, &provider, &analysis, &run,
                                CoverageStatus::Failed, Some(ObligationKind::OutsideProviderModel),
                                Some(failure.detail.clone())))?;
                            partial = true;
                            return Ok(());
                        }
                        event => event,
                    };
                    let state = module.as_mut().ok_or_else(|| invalid("native event has no module preamble"))?;
                    if state.index != input_index { return Err(invalid("native event changed module before completion")); }
                    let mut writer = Writer { context, index: &index, state,
                        artifact, analysis: &analysis, scope: &scope, run: &run, surface: &surface };
                    let mut finished = false;
                    match event {
                        ModuleEvent::Definition { ordinal, value } => writer.definition(ordinal, value)?,
                        ModuleEvent::UseGroup { ordinal, value } => writer.use_group(ordinal, value)?,
                        ModuleEvent::UnavailableUse(span) => {
                            let subject = writer.attach(span, None)?;
                            writer.boundary(subject, ObligationKind::ScopeBoundary,
                                "native use enumeration unavailable: no source scope")?;
                        }
                        ModuleEvent::Value(value) => writer.value(value)?,
                        ModuleEvent::Region(value) => writer.region(value)?,
                        ModuleEvent::Attribute(value) => writer.attribute(value)?,
                        ModuleEvent::Test(value) => writer.test(value)?,
                        ModuleEvent::TestLeaf(value) => writer.test_leaf(value)?,
                        ModuleEvent::TraversalComplete => writer.state.traversal_complete(&budget),
                        ModuleEvent::End(skips) => {
                            let syntax_errors = writer.prelude()?.syntax_errors;
                            let (status, reason) = if syntax_errors > 0 {
                                (CoverageStatus::Partial, Some(ObligationKind::SyntaxError))
                            } else if writer.state.partial {
                                (CoverageStatus::Partial, Some(ObligationKind::OutsideProviderModel))
                            } else { (CoverageStatus::CompleteUnderStatedModel, None) };
                            partial |= status != CoverageStatus::CompleteUnderStatedModel;
                            writer.context.contribute(coverage(&scope, &provider, &analysis, &run,
                                status, reason, Some(format!(
                                    "native syntax errors {}; runtime reach skips {}; ty-false reach skips {}",
                                    syntax_errors, skips.reaching_runtime_view, skips.reaching_ty_false))))?;
                            finished = true;
                        }
                        ModuleEvent::Prelude(_) | ModuleEvent::ModuleFailure(_) => unreachable!(),
                    }
                    writer.context.check_cancelled()?;
                    if finished { module = None; }
                    Ok::<_, ModelError>(())
                },
            ).map_err(|error| match error {
                cpg_flow::VisitError::Consumer(error)
                | cpg_flow::VisitError::Operational(cpg_flow::FlowOperationalError::Model(error)) => error,
                cpg_flow::VisitError::Operational(cpg_flow::FlowOperationalError::NativeCancelled(error)) => ModelError::Cause(Box::new(error)),
            })?;
            if module.is_some() {
                return Err(invalid("native module ended without final coverage"));
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
struct ModuleState {
    index: usize,
    prelude: Option<cpg_flow::ModulePrelude>,
    source_view: Option<Id<FlowSourceViewObservation>>,
    partial: bool,
    atoms: BTreeMap<Id<EvaluationAtom>, EvaluationAtom>,
    uses: BTreeMap<u32, Option<(FlowUse, Id<LexicalScope>)>>,
    definitions: BTreeMap<u32, Option<(FlowDefinition, Id<LexicalScope>)>>,
    atom_charge: StateCharge,
    rows_charge: StateCharge,
}
impl ModuleState {
    fn new(index: usize, budget: &lctx_model::domain::resources::ResourceBudget) -> Self {
        Self {
            index,
            prelude: None,
            source_view: None,
            partial: false,
            atoms: BTreeMap::new(),
            uses: BTreeMap::new(),
            definitions: BTreeMap::new(),
            atom_charge: StateCharge::new(budget, "ty_flow_atom_cache"),
            rows_charge: StateCharge::new(budget, "ty_flow_nominal_rows"),
        }
    }
    fn traversal_complete(&mut self, budget: &lctx_model::domain::resources::ResourceBudget) {
        self.uses.clear();
        self.definitions.clear();
        self.rows_charge = StateCharge::new(budget, "ty_flow_nominal_rows");
    }
}
struct Writer<'a, S: ProviderSink + 'static> {
    context: &'a mut StageContext<S>,
    index: &'a Index,
    state: &'a mut ModuleState,
    artifact: &'a SourceArtifact,
    analysis: &'a AnalysisContext,
    scope: &'a CoverageScope,
    run: &'a ProviderRun,
    surface: &'a ProviderSurface,
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
impl<S: ProviderSink + 'static> Writer<'_, S> {
    fn boundary(
        &mut self,
        occurrence: Option<Id<Occurrence>>,
        reason: ObligationKind,
        detail: &str,
    ) -> Result<(), ModelError> {
        self.state.partial = true;
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
        let _scratch = self.context.budget().reserve(
            "ty_flow_attachment_shapes",
            self.prelude()?.nodes(span).len().saturating_mul(64),
        )?;
        let mut shapes: Vec<_> = self
            .prelude()?
            .nodes(span)
            .iter()
            .filter(|n| {
                n.kind
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
                self.state.partial = true;
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
            assumptions: lctx_model::domain::assumptions::AssumptionSet::empty_id(),
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
                let _scratch = self
                    .context
                    .budget()
                    .reserve("ty_flow_literal_set", values.len().saturating_mul(256))?;
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
            native::Atom::NonTerminalCall { awaiting } => Predicate::NonTerminalCall {
                awaiting: *awaiting,
            },
            native::Atom::NonEmptyIterable => Predicate::NonEmptyIterable,
            native::Atom::ContextManagerSuppresses { asynchronous } => {
                Predicate::ContextManagerSuppresses {
                    asynchronous: *asynchronous,
                }
            }
            native::Atom::FinallyNormalPathImpossible => Predicate::FinallyNormalPathImpossible,
            native::Atom::Opaque { text } => Predicate::Opaque { text: text.clone() },
            native::Atom::Evaluated { .. } => {
                return Err(invalid("nested native evaluation wrapper"));
            }
        };
        self.context.contribute(predicate.clone())?;
        // Native operand selection follows translator structure; pattern subjects remain outside
        // their case evaluation until the explicit subject bridge is implemented in phase 3.
        let operand_span = self.prelude()?.operand(leaf);
        let mut operand_charge = StateCharge::new(self.context.budget(), "ty_flow_operand_scratch");
        let operand = if let Some(span) = operand_span {
            if let (Some(place), Some(occurrence)) = (
                self.place_copy(span, &mut operand_charge)?,
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
        if !self.state.atoms.contains_key(&atom.id()) {
            self.state.atom_charge.grow(
                size_of::<EvaluationAtom>()
                    + atom.heap_bytes()
                    + size_of::<Id<EvaluationAtom>>()
                    + 128,
            )?;
            self.state.atoms.insert(atom.id(), atom.clone());
            self.context.contribute(atom.clone())?;
        }
        Ok(Some(atom.id()))
    }
    fn prelude(&self) -> Result<&cpg_flow::ModulePrelude, ModelError> {
        self.state
            .prelude
            .as_ref()
            .ok_or_else(|| invalid("flow event precedes module preamble"))
    }
    fn source_view(&mut self) -> Result<(), ModelError> {
        let prelude = self.prelude()?;
        let (original_content, view_content, byte_len, renamed) = (
            prelude.original_content,
            prelude.view_content,
            prelude.view_byte_len,
            prelude.renamed,
        );
        {
            if original_content != self.artifact.content
                || byte_len != self.artifact.byte_len as usize
            {
                return Err(invalid(
                    "ty view does not preserve captured source byte geometry",
                ));
            }
            let (condition, nodes) = Diagram::always().records();
            self.context.contribute(condition.clone())?;
            for node in nodes {
                self.context.contribute(node)?;
            }
            let qualification = AssertionQualification {
                assumptions: lctx_model::domain::assumptions::AssumptionSet::empty_id(),
                context: self.analysis.id(),
                scope: self.scope.id(),
                condition: condition.id(),
                modality: Modality::Definite,
                approximation: Approximation::Exact,
            };
            self.context.contribute(qualification.clone())?;
            let row = FlowSourceViewObservation {
                qualification: qualification.id(),
                source: self.artifact.id(),
                original_content,
                view_content,
                byte_len: byte_len as i64,
                renamed_type_checking: i64::from(renamed),
            };
            row.validate()?;
            self.state.source_view = Some(row.id());
            let evidence = Evidence::SourceSpan {
                source: self.artifact.id(),
                start: 0,
                end: byte_len as i64,
            };
            self.context.contribute(evidence.clone())?;
            self.context.emit(FlowSourceViewSupport {
                assertion: row.id(),
                run: self.run.id(),
                surface: self.surface.id(),
                evidence: evidence.id(),
                origin: Origin::AnalyzerAssertion,
                mode: ExtractionMode::NativeTraversal,
                fidelity: Fidelity::NativeStructural,
            })?;
            self.context.emit(row)?;
        }
        Ok(())
    }
    fn place_copy(
        &self,
        span: Span,
        charge: &mut StateCharge,
    ) -> Result<Option<native::Place>, ModelError> {
        let Some(place) = self.prelude()?.place(span) else {
            return Ok(None);
        };
        charge.grow((size_of::<native::Place>() + place.heap_bytes()).saturating_mul(2))?;
        Ok(Some(place.clone()))
    }
    fn definition(&mut self, ordinal: u32, native: &cpg_flow::Def) -> Result<(), ModelError> {
        let mut group_charge =
            StateCharge::new(self.context.budget(), "ty_flow_definition_scratch");

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
            let place = match self.place_copy(native.target, &mut group_charge)? {
                Some(place) => Some(place),
                None if !native.place.contains(['.', '[', ']']) => {
                    group_charge.grow(size_of::<native::Place>() + native.place.len())?;
                    Some(native::Place {
                        root: native.place.clone(),
                        segments: vec![],
                    })
                }
                None => None,
            };
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
        self.state.rows_charge.grow(256)?;
        self.state.definitions.insert(ordinal, row);
        Ok(())
    }
    fn use_group(&mut self, ordinal: u32, group: &cpg_flow::UseGroup) -> Result<(), ModelError> {
        let native = &group.use_;
        let mut group_charge = StateCharge::new(self.context.budget(), "ty_flow_group_scratch");

        let row = if let (Some(scope), Some(occurrence)) = (
            self.index.scope(self.artifact.id(), native.scope),
            self.attach(native.span, None)?,
        ) {
            if let Some(place) = self.place_copy(native.span, &mut group_charge)? {
                let row = FlowUse {
                    occurrence,
                    place: self.place(&place, scope, occurrence)?,
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
        self.state.rows_charge.grow(256)?;
        self.state.uses.insert(ordinal, row);
        if let Some(capture) = &group.capture {
            self.capture(capture)?;
        }
        let attached_use = self.state.uses.get(&ordinal).cloned();
        let Some(Some((use_, scope))) = attached_use.as_ref() else {
            return Ok(());
        };
        group_charge.grow(group.reaching.len().saturating_mul(128))?;
        let mut mapped_reaching = vec![None; group.reaching.len()];
        let mut unattached_reaching = vec![false; group.reaching.len()];
        for (native_ix, native) in group.reaching.iter().enumerate() {
            let target = if native.nested {
                ReachingDefinition::Nested
            } else if let Some(index) = native.def_ix {
                let attached_definition = self.state.definitions.get(&index).cloned();
                let Some(Some((definition, _))) = attached_definition.as_ref() else {
                    unattached_reaching[native_ix] = true;
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
                        self.artifact.path, group.use_, native, use_, definition
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
                self.boundary(
                    Some(use_.occurrence),
                    ObligationKind::ResourceRefused,
                    "native narrowing scope lost precision; terminal true is over-approximate",
                )?;
            }
            if let Some(q) = self.qualify(&native.narrowing, *scope, Some(use_.occurrence))? {
                supported!(
                    self,
                    FlowNarrowingObservation,
                    FlowNarrowingSupport,
                    FlowNarrowingObservation {
                        qualification: q.id(),
                        use_: use_.id(),
                        target: target.id(),
                        precision_lost: native.narrowing_precision_lost
                    },
                    use_.occurrence
                );
            }
            if let Some(q) = self.qualify(&native.condition, *scope, Some(use_.occurrence))? {
                let row = FlowReachingObservation {
                    qualification: q.id(),
                    use_: use_.id(),
                    target: target.id(),
                    loop_carried: native.loop_carried,
                };
                let evidence = Evidence::Occurrence {
                    occurrence: use_.occurrence,
                };
                self.context.contribute(evidence.clone())?;
                let support = FlowReachingSupport {
                    assertion: row.id(),
                    run: self.run.id(),
                    surface: self.surface.id(),
                    evidence: evidence.id(),
                    origin: Origin::AnalyzerAssertion,
                    mode: ExtractionMode::NativeTraversal,
                    fidelity: Fidelity::NativeStructural,
                };
                mapped_reaching[native_ix] = Some((row.id(), support.id()));
                self.context.emit(support)?;
                self.context.emit(row)?;
            }
        }

        let native = &group.candidates;
        if native.len() > MAX_USE_CANDIDATES {
            self.boundary(
                Some(use_.occurrence),
                ObligationKind::ResourceRefused,
                "per-use native candidate bound exceeded",
            )?;
            return Ok(());
        }
        group_charge.grow(native.len() * 256 + 128)?;
        let mut candidates = Vec::new();
        let mut members = Vec::new();
        for c in native {
            group_charge.grow(c.reaching.len() * 256)?;
            let mut retained = c
                .reaching
                .clone()
                .filter_map(|i| mapped_reaching[i])
                .collect::<Vec<_>>();
            retained.sort();
            retained.dedup();
            let reachability = self.qualify(&c.condition, *scope, Some(use_.occurrence))?;
            let narrowing = self.qualify(&c.narrowing, *scope, Some(use_.occurrence))?;
            let unavailable = reachability.is_none();
            if c.unattached {
                self.boundary(
                    Some(use_.occurrence),
                    ObligationKind::NativeUnavailable,
                    "native candidate definition unattached",
                )?;
            }
            if c.narrowing_precision_lost {
                self.boundary(
                    Some(use_.occurrence),
                    ObligationKind::ResourceRefused,
                    "native candidate narrowing lost precision",
                )?;
            }
            let unattached = c.unattached || c.reaching.clone().any(|i| unattached_reaching[i]);
            candidates.push(CandidateState {
                kind: match c.kind {
                    cpg_flow::CandidateKind::Bound => FlowCandidateKind::Bound,
                    cpg_flow::CandidateKind::Undefined => FlowCandidateKind::Undefined,
                    cpg_flow::CandidateKind::Deleted => FlowCandidateKind::Deleted,
                    cpg_flow::CandidateKind::Nested => FlowCandidateKind::Nested,
                    cpg_flow::CandidateKind::LoopHeader => FlowCandidateKind::LoopHeader,
                },
                pruned: c.pruned,
                loop_expanded: c.loop_expanded,
                unattached,
                reachability: reachability.as_ref().map(Record::id),
                narrowing: narrowing.as_ref().map(Record::id),
                narrowing_unavailable: narrowing.is_none(),
                narrowing_precision_lost: c.narrowing_precision_lost,
                condition_unavailable: unavailable,
                reachability_lost: c.reachability_lost,
                mapped_count: retained.len() as i64,
            });
            members.extend(
                retained
                    .into_iter()
                    .map(|(r, s)| (i64::from(c.ordinal), r, s)),
            );
        }
        if members.len() > MAX_USE_CANDIDATES {
            self.boundary(
                Some(use_.occurrence),
                ObligationKind::ResourceRefused,
                "per-use mapped candidate bound exceeded",
            )?;
            return Ok(());
        }
        group_charge.grow(members.len() * 512)?;
        let Some(q) = self.qualify(
            &cpg_flow::Condition::always(),
            *scope,
            Some(use_.occurrence),
        )?
        else {
            return Ok(());
        };
        let (inventory, candidates, members) = FlowUseInventoryObservation::new(
            q.id(),
            use_.id(),
            *scope,
            self.state
                .source_view
                .ok_or_else(|| invalid("flow source view not prepared"))?,
            &candidates,
            &members,
        )?;
        for c in candidates {
            self.context.emit(c)?;
        }
        for m in members {
            self.context.emit(m)?;
        }
        supported!(
            self,
            FlowUseInventoryObservation,
            FlowUseInventorySupport,
            inventory,
            use_.occurrence
        );
        Ok(())
    }
    fn capture(&mut self, native: &cpg_flow::CaptureSnapshot) -> Result<(), ModelError> {
        let mut group_charge = StateCharge::new(self.context.budget(), "ty_flow_capture_scratch");

        let attached_use = self.state.uses.get(&native.use_ix).cloned();
        let Some(Some((use_, nested_scope))) = attached_use.as_ref() else {
            return Ok(());
        };
        let Some(enclosing_scope) = self.index.scope(self.artifact.id(), native.enclosing) else {
            self.boundary(
                Some(use_.occurrence),
                ObligationKind::ScopeBoundary,
                "enclosing snapshot scope unattached",
            )?;
            return Ok(());
        };
        if enclosing_scope == *nested_scope {
            self.boundary(
                Some(use_.occurrence),
                ObligationKind::ScopeBoundary,
                "native annotation snapshot scopes collapse to one canonical lexical scope",
            )?;
            return Ok(());
        }
        group_charge.grow(native.candidates.len() * size_of::<CaptureCandidate>() + 512)?;
        let mut candidates = Vec::new();
        let mut attached = true;
        for candidate in &native.candidates {
            let target = match candidate.binding {
                cpg_flow::SnapshotBinding::Bound(index) => {
                    match self.state.definitions.get(&index) {
                        Some(Some((definition, scope))) if *scope == enclosing_scope => {
                            FlowCaptureTarget::Bound {
                                definition: definition.id(),
                            }
                        }
                        _ => {
                            self.boundary(
                                Some(use_.occurrence),
                                ObligationKind::NativeUnavailable,
                                "snapshot candidate definition unattached to enclosing scope",
                            )?;
                            FlowCaptureTarget::Unattached
                        }
                    }
                }
                cpg_flow::SnapshotBinding::Undefined => FlowCaptureTarget::Undefined,
                cpg_flow::SnapshotBinding::Deleted => FlowCaptureTarget::Deleted,
                cpg_flow::SnapshotBinding::Nested => FlowCaptureTarget::Nested,
                cpg_flow::SnapshotBinding::LoopHeader => FlowCaptureTarget::LoopHeader,
                cpg_flow::SnapshotBinding::Unattached => FlowCaptureTarget::Unattached,
            };
            self.context.emit(target.clone())?;
            let (Some(condition), Some(narrowing)) = (
                self.qualify(&candidate.condition, enclosing_scope, Some(use_.occurrence))?,
                self.qualify(&candidate.narrowing, enclosing_scope, Some(use_.occurrence))?,
            ) else {
                attached = false;
                break;
            };
            candidates.push((
                target.id(),
                condition.condition,
                narrowing.condition,
                candidate.precision_lost,
            ));
        }
        if !attached {
            self.boundary(
                Some(use_.occurrence),
                ObligationKind::NativeUnavailable,
                "complete snapshot formula inventory unattached",
            )?;
            return Ok(());
        }
        let inventory = if native.state == cpg_flow::SnapshotState::FoundBindings {
            if candidates.len() > MAX_CAPTURE_CANDIDATES {
                self.boundary(
                    Some(use_.occurrence),
                    ObligationKind::ResourceRefused,
                    "snapshot candidate work bound exceeded",
                )?;
                return Ok(());
            }
            let (inventory, members) = FlowCaptureInventory::new(&candidates)?;
            for member in members {
                self.context.emit(member)?;
            }
            self.context.emit(inventory.clone())?;
            Some(inventory.id())
        } else {
            None
        };
        let constraint = match &native.constraint {
            Some(condition) => {
                match self.qualify(condition, enclosing_scope, Some(use_.occurrence))? {
                    Some(q) => Some(q.condition),
                    None => return Ok(()),
                }
            }
            None => None,
        };
        let Some(mut q) = self.qualify(
            &cpg_flow::Condition::always(),
            *nested_scope,
            Some(use_.occurrence),
        )?
        else {
            return Ok(());
        };
        if native.lazy {
            q.modality = Modality::Candidate;
        }
        if native.constraint_precision_lost {
            q.approximation = Approximation::Over;
        }
        self.context.contribute(q.clone())?;
        let row = FlowCaptureTimingObservation {
            qualification: q.id(),
            use_: use_.id(),
            nested_scope: *nested_scope,
            enclosing_scope,
            origin: match native.origin {
                cpg_flow::CaptureOrigin::OuterLocal => FlowCaptureOrigin::OuterLocal,
                cpg_flow::CaptureOrigin::Global => FlowCaptureOrigin::Global,
                cpg_flow::CaptureOrigin::Nonlocal => FlowCaptureOrigin::Nonlocal,
            },
            timing: if native.lazy {
                lctx_model::domain::captures::CaptureTiming::LazySnapshot
            } else {
                lctx_model::domain::captures::CaptureTiming::EagerSnapshot
            },
            state: match native.state {
                cpg_flow::SnapshotState::FoundBindings => FlowSnapshotState::FoundBindings,
                cpg_flow::SnapshotState::FoundConstraint => FlowSnapshotState::FoundConstraint,
                cpg_flow::SnapshotState::NotFound => FlowSnapshotState::NotFound,
                cpg_flow::SnapshotState::NoLongerInEagerContext => {
                    FlowSnapshotState::NoLongerInEagerContext
                }
            },
            inventory,
            constraint,
            constraint_precision_lost: native.constraint_precision_lost,
        };
        row.validate()?;
        supported!(
            self,
            FlowCaptureTimingObservation,
            FlowCaptureTimingSupport,
            row,
            use_.occurrence
        );
        Ok(())
    }
    fn value(&mut self, native: cpg_flow::ValueSourceRef<'_>) -> Result<(), ModelError> {
        let mut group_charge = StateCharge::new(self.context.budget(), "ty_flow_value_scratch");

        let attached_use = self.state.uses.get(&native.use_ix).cloned();
        let Some(Some((use_, scope))) = attached_use.as_ref() else {
            return Ok(());
        };
        let Some(sink) = self.attach(native.span, None)? else {
            return Ok(());
        };
        let Some(q) = self.qualify(native.condition, *scope, Some(sink))? else {
            return Ok(());
        };
        let mut steps = Vec::new();
        let mut attached = true;
        for frame in native.call_path {
            group_charge.grow(256)?;
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
            return Ok(());
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
        Ok(())
    }
    fn region(&mut self, native: &cpg_flow::Region) -> Result<(), ModelError> {
        let _scratch = self.context.budget().reserve(
            "ty_flow_region_candidates",
            self.prelude()?.nodes(native.span).len().saturating_mul(64),
        )?;

        if let Some(scope) = self.index.scope(self.artifact.id(), native.scope) {
            let kinds: Vec<_> = self
                .prelude()?
                .nodes(native.span)
                .iter()
                .filter(|n| n.kind.is_some_and(|k| (2..=26).contains(&(k as i16))))
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
        Ok(())
    }
    fn test(&mut self, native: &cpg_flow::Test) -> Result<(), ModelError> {
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
        Ok(())
    }
    fn test_leaf(&mut self, native: cpg_flow::TestLeafRef<'_>) -> Result<(), ModelError> {
        if let Some(scope) = self.index.scope(self.artifact.id(), native.scope)
            && let Some(test) = self.attach(native.test_span, None)?
            && let (Some(atom), Some(q)) = (
                self.atom(native.atom, scope)?,
                self.qualify(native.condition, scope, Some(test))?,
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
        Ok(())
    }
    fn attribute(&mut self, native: cpg_flow::AttributeLoadRef<'_>) -> Result<(), ModelError> {
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
                        name: native.name.into()
                    },
                    occurrence
                );
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod preparation_controls {
    use super::*;
    use lctx_model::domain::resources::ResourceBudget;

    fn id<R>(n: u8) -> Id<R> {
        serde_json::from_value(serde_json::json!(vec![n; 16])).unwrap()
    }
    fn occurrence(source: u8, start: i64, end: i64, kind: SyntaxKind, path: i32) -> Occurrence {
        Occurrence {
            source: id(source),
            start,
            end,
            syntax_kind: kind,
            role: OccurrenceRole::Syntax,
            structural_path: vec![path],
        }
    }
    fn index(rows: Vec<Occurrence>, budget: &ResourceBudget) -> Index {
        let mut charge = StateCharge::new(budget, "flow-index-control");
        charge
            .grow(
                rows.capacity() * size_of::<Occurrence>()
                    + rows.iter().map(HeapSize::heap_bytes).sum::<usize>(),
            )
            .unwrap();
        let positions = rows
            .iter()
            .enumerate()
            .map(|(position, row)| (row.id(), position))
            .collect();
        Index {
            _charge: StateCharge::new(budget, "flow-index-control-lookups"),
            occurrences: Occurrences {
                prepared: crate::bundle::PreparedOccurrences {
                    rows,
                    _charge: charge,
                },
                positions,
            },
            scopes: BTreeMap::new(),
            declarations: vec![],
            bindings: BTreeMap::new(),
            events: BTreeMap::new(),
            targets: BTreeMap::new(),
            resolutions: BTreeMap::new(),
            runtime_imports: BTreeMap::new(),
            source_references: BTreeMap::new(),
            source_scopes: BTreeMap::new(),
            scope_bindings: BTreeMap::new(),
            occurrence_bindings: BTreeMap::new(),
            owner_declarations: BTreeMap::new(),
        }
    }
    fn reference(read: &Occurrence, name: &str) -> ReferenceObservation {
        ReferenceObservation {
            qualification: id(9),
            read: read.id(),
            scope: id(10),
            parent: id(11),
            field: SyntaxField::Child,
            name: name.into(),
        }
    }
    fn resolve(index: &mut Index, read: Id<Occurrence>, target: LexicalTarget) {
        let key = target.id();
        index.targets.insert(key, target);
        index
            .resolutions
            .entry(read)
            .or_default()
            .push(LexicalResolution {
                qualification: id(9),
                read,
                target: key,
                captured: true,
            });
    }

    #[test]
    fn preparation_drops_after_foreign_import_classification_and_references_keep_first_order() {
        let budget = ResourceBudget::fixed(1 << 20).unwrap();
        let read = occurrence(1, 0, 1, SyntaxKind::ExprName, 0);
        let later = occurrence(1, 0, 1, SyntaxKind::ExprName, 1);
        let local_owner = occurrence(1, 0, 20, SyntaxKind::ModModule, 2);
        let foreign_owner = occurrence(2, 0, 40, SyntaxKind::ModModule, 3);
        let statement = occurrence(2, 0, 35, SyntaxKind::StmtImportFrom, 4);
        let alias = occurrence(2, 19, 35, SyntaxKind::Alias, 5);
        let imported = occurrence(2, 19, 32, SyntaxKind::Identifier, 6);
        let binding_site = occurrence(2, 33, 35, SyntaxKind::Identifier, 7);
        let mut index = index(
            vec![
                read.clone(),
                later.clone(),
                local_owner.clone(),
                foreign_owner.clone(),
                statement.clone(),
                alias.clone(),
                imported.clone(),
                binding_site.clone(),
            ],
            &budget,
        );
        let local = LexicalScope {
            owner: local_owner.id(),
            kind: LexicalScopeKind::Module,
        };
        let foreign = LexicalScope {
            owner: foreign_owner.id(),
            kind: LexicalScopeKind::Module,
        };
        index.scopes.insert(local.id(), local.clone());
        index.scopes.insert(foreign.id(), foreign.clone());
        let event = BindingEvent {
            site: binding_site.id(),
            name: "guard".into(),
        };
        index.events.insert(event.id(), event.clone());
        index.bindings.insert(
            event.id(),
            BindingObservation {
                qualification: id(9),
                event: event.id(),
                scope: foreign.id(),
                kind: BindingEventKind::FromImport,
                ordinal: 0,
                value: None,
                static_branch: None,
                static_polarity: None,
            },
        );
        let mut preparation_charge = StateCharge::new(&budget, "flow-runtime-preparation-control");
        preparation_charge.grow(4096).unwrap();
        let preparation = RuntimePreparation {
            _charge: preparation_charge,
            identifiers: BTreeMap::from([(imported.id(), "TYPE_CHECKING".into())]),
            imports: vec![ImportAliasObservation {
                qualification: id(9),
                statement: statement.id(),
                alias: alias.id(),
                level: 0,
                resolved_module: Some("typing".into()),
            }],
            source_imports: BTreeMap::from([(id(2), vec![0])]),
            source_identifiers: BTreeMap::from([(id(2), vec![imported.id()])]),
        };
        assert!(matches!(
            preparation.import_special(&event, &index.occurrences),
            Some(RuntimeSpecial::Checking)
        ));
        index._charge.grow(192).unwrap();
        index
            .runtime_imports
            .insert(event.id(), RuntimeSpecial::Checking);
        let before = budget.reserved();
        let released = preparation._charge.accounted_bytes();
        drop(preparation);
        assert_eq!(budget.reserved(), before - released);
        assert!(index.runtime_imports.contains_key(&event.id()));
        resolve(
            &mut index,
            read.id(),
            LexicalTarget::Binding { event: event.id() },
        );
        resolve(
            &mut index,
            later.id(),
            LexicalTarget::Builtin {
                name: "type".into(),
                variable: false,
            },
        );
        let first = reference(&read, "guard");
        let second = reference(&later, "guard");
        index
            .source_references
            .insert(id(1), vec![first.clone(), second.clone(), first.clone()]);
        assert_eq!(
            index.source_references[&id(1)],
            vec![first.clone(), second, first]
        );
        let place = native::Place {
            root: "guard".into(),
            segments: vec![],
        };
        assert_eq!(
            index.root(local.id(), &place, read.id()).unwrap(),
            PlaceRoot::Local {
                scope: foreign_owner.id(),
                name: "guard".into()
            }
        );
        let mut runtime_charge = StateCharge::new(&budget, "flow-runtime-control");
        let runtime = index.runtime(id(1), &mut runtime_charge).unwrap();
        let location = Span { start: 0, end: 1 };
        assert_eq!(
            runtime.checking_names,
            std::collections::BTreeSet::from([location])
        );
        assert_eq!(
            runtime.builtin_type,
            std::collections::BTreeSet::from([location])
        );
        assert_eq!(
            runtime_charge.reserved(),
            2 * cpg_flow::RuntimeBindings::SPAN_ENTRY_BYTES,
            "duplicate reference observations do not retain duplicate runtime backing"
        );
        drop(runtime);
        drop(runtime_charge);
        drop(index);
        assert_eq!(budget.reserved(), 0);
    }

    #[test]
    fn runtime_requires_candidate_agreement_and_admits_unique_class_backing_before_clone() {
        let budget = ResourceBudget::fixed(1 << 20).unwrap();
        let read = occurrence(1, 4, 7, SyntaxKind::ExprName, 0);
        let ambiguous = occurrence(1, 10, 14, SyntaxKind::ExprName, 1);
        let mut index = index(vec![read.clone(), ambiguous.clone()], &budget);
        resolve(
            &mut index,
            read.id(),
            LexicalTarget::Builtin {
                name: "int".into(),
                variable: false,
            },
        );
        resolve(
            &mut index,
            ambiguous.id(),
            LexicalTarget::Builtin {
                name: "int".into(),
                variable: false,
            },
        );
        resolve(
            &mut index,
            ambiguous.id(),
            LexicalTarget::Builtin {
                name: "type".into(),
                variable: false,
            },
        );
        index.source_references.insert(
            id(1),
            vec![
                reference(&read, "int"),
                reference(&read, "int"),
                reference(&ambiguous, "type"),
            ],
        );
        let mut charge = StateCharge::new(&budget, "runtime-class-control");
        let runtime = index.runtime(id(1), &mut charge).unwrap();
        assert_eq!(
            runtime.builtin_classes,
            BTreeMap::from([(
                "int".into(),
                std::collections::BTreeSet::from([Span { start: 4, end: 7 }])
            )])
        );
        assert!(runtime.builtin_type.is_empty());
        assert_eq!(
            charge.reserved(),
            cpg_flow::RuntimeBindings::CLASS_ENTRY_BYTES
                + 3
                + cpg_flow::RuntimeBindings::SPAN_ENTRY_BYTES
        );
        assert_eq!(charge.reserved(), runtime.owned_bytes());
        let refused = ResourceBudget::fixed(1).unwrap();
        let mut refusal_charge = StateCharge::new(&refused, "runtime-refused-control");
        assert!(matches!(
            index.runtime(id(1), &mut refusal_charge),
            Err(ModelError::Resource { .. })
        ));
        assert_eq!(refused.reserved(), 0);
        drop(runtime);
        drop(charge);
        drop(index);
        assert_eq!(budget.reserved(), 0);
    }
}

#[cfg(test)]
mod source_buffer_controls {
    use super::*;
    #[test]
    fn captured_source_growth_shrink_and_same_length_change_are_refused() {
        let root = tempfile::tempdir().unwrap();
        let source = b"x = 1\n";
        let revision = lctx_model::domain::input::InputRevision::from_entries(vec![]).unwrap();
        let artifact =
            SourceArtifact::from_bytes(revision.id(), "sample.py".into(), source).unwrap();
        let budget = lctx_model::domain::resources::ResourceBudget::fixed(1 << 20).unwrap();
        for bytes in [source.as_slice(), b"x = 1\nextra", b"x", b"x = 2\n"] {
            std::fs::write(root.path().join("sample.py"), bytes).unwrap();
            let mut charge = StateCharge::new(&budget, "source-buffer-control");
            let result = read_captured_source(&artifact, root.path(), &mut charge);
            if bytes == source {
                assert_eq!(result.unwrap(), source);
            } else {
                assert!(matches!(result, Err(ModelError::Invalid(_))));
            }
            assert!(charge.reserved() >= source.len());
            drop(charge);
            assert_eq!(budget.reserved(), 0);
        }
        let budget = lctx_model::domain::resources::ResourceBudget::fixed(1).unwrap();
        let mut charge = StateCharge::new(&budget, "source-buffer-refusal");
        assert!(matches!(
            read_captured_source(&artifact, &root.path().join("missing"), &mut charge),
            Err(ModelError::Resource { .. })
        ));
        drop(charge);
        assert_eq!(budget.reserved(), 0);
    }
}
