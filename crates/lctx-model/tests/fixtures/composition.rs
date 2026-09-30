//! `select_timeout(t)` composed through the call. The caller delivers the global `t` into the
//! call's argument; the callee returns `timeout` under `not (timeout is None)`. With the stability
//! witness the composed transfer `t → select_timeout(t)` is the caller's, at the site, and stays
//! conditional on the guard restated over the argument. Mutators produce the refused variants.
#![allow(
    dead_code,
    reason = "Shared contract fixtures expose helpers to multiple targeted suites"
)]
#[path = "binding.rs"]
pub(super) mod binding_fixture;
#[path = "stability.rs"]
mod stability;
use binding_fixture::{InspectionCase, bind_inspection};
use lctx_model::domain::{
    artifact::*,
    assertion::*,
    attribution::*,
    calls::*,
    composition::*,
    conditions::{rebase::GuardCatalog, stability::*, *},
    declarations::*,
    flow::*,
    input::*,
    lexical::*,
    memory::MemoryGeneration,
    occurrence_owner::OwnerTable,
    place_composition::PathCatalog,
    source::*,
    transfer::*,
    value::*,
    *,
};
use std::collections::{BTreeMap, BTreeSet};

/// A refused variant of the stored composition.
#[derive(Debug, Clone, Copy)]
pub enum Mutation {
    /// The step names the caller's own alternative as the callee premise.
    CalleeFromAnotherSymbol,
    /// The step concludes the caller's flow-local alternative instead of a composed one.
    NotComposed,
    /// The composed alternative keeps the callee's guard atom, never restated at the call.
    UnrestatedGuard,
    /// The composed alternative drops the restated guard: its condition is `true`.
    ErasedCondition,
    /// The composed transfer lands on an occurrence of the call that is not its site, an argument
    /// or its receiver (the callee expression).
    ForeignOutput,
}

pub struct Fixture {
    pub base: stability::Fixture,
    pub run: ProviderRun,
    pub caller_symbol: ProviderSymbol,
    pub callee_symbol: ProviderSymbol,
    pub caller_declaration: SymbolDeclaration,
    pub callee_declaration: SymbolDeclaration,
    pub signature: Signature,
    pub target: CallTarget,
    pub delivered: Place,
    pub caller: TransferBranch,
    pub callee: TransferBranch,
    pub composed: ComposedTransfer,
    pub selection: Selection,
}
impl Fixture {
    pub fn new() -> Self {
        let base = stability::Fixture::new();
        let (context, qualification) = (base.context.clone(), base.qualification.clone());
        let input = base.rows::<InputRevision>().remove(0);
        let artifacts = base.rows::<SourceArtifact>();
        let artifact = |path: &str| artifacts.iter().find(|a| a.path == path).unwrap();
        let (callee_source, caller_source) = (artifact("callee.py"), artifact("caller.py"));
        let occurrences = base.rows::<Occurrence>();
        let find = |source: &SourceArtifact, kind: SyntaxKind| {
            occurrences
                .iter()
                .find(|o| o.source == source.id() && o.syntax_kind == kind)
                .unwrap()
                .clone()
        };
        let (caller_root, def) = (
            find(caller_source, SyntaxKind::ModModule),
            find(callee_source, SyntaxKind::StmtFunctionDef),
        );
        // A second provider asserts symbols, declarations, the signature, the target and transfers.
        let provider = Provider {
            tool: "composition-fixture".into(),
            revision: "1".into(),
            build_digest: ContentHash::of(b"composition"),
        };
        let (run, families) = ProviderRun::new(
            provider.id(),
            context.id(),
            input.id(),
            context.config_digest,
            [FactFamily::Signatures, FactFamily::Calls, FactFamily::Flow],
        )
        .unwrap();
        let surface = |family: FactFamily, name: &str| ProviderSurface {
            provider: provider.id(),
            family,
            name: name.into(),
        };
        let (declared, called, flows) = (
            surface(FactFamily::Signatures, "declarations"),
            surface(FactFamily::Calls, "targets"),
            surface(FactFamily::Flow, "transfers"),
        );
        let modules = [
            Module {
                source: caller_source.id(),
                qualified_name: "caller".into(),
            },
            Module {
                source: callee_source.id(),
                qualified_name: "callee".into(),
            },
        ];
        let provider_modules = modules.clone().map(|module| ProviderModule::Acquired {
            module: module.id(),
        });
        let symbol =
            |module: &ProviderModule, key: &str, name: &str, kind: SymbolKind| ProviderSymbol {
                provider: provider.id(),
                context: context.id(),
                module: module.id(),
                native_key: key.into(),
                name: name.into(),
                kind,
            };
        let caller_symbol = symbol(&provider_modules[0], "caller", "caller", SymbolKind::Module);
        let callee_symbol = symbol(
            &provider_modules[1],
            "callee.select_timeout",
            "select_timeout",
            SymbolKind::Function,
        );
        // The owner rule places the module-level call in the caller module.
        let owners = OwnerTable::build(&occurrences, &budget()).unwrap();
        assert_eq!(owners.owner(base.site.id()), Some(caller_root.id()));
        let caller_declaration = SymbolDeclaration {
            qualification: qualification.id(),
            symbol: caller_symbol.id(),
            declaration: caller_root.id(),
        };
        let callee_declaration = SymbolDeclaration {
            qualification: qualification.id(),
            symbol: callee_symbol.id(),
            declaration: def.id(),
        };
        let shape = ParameterShape {
            name: Some("timeout".into()),
            kind: ParameterKind::PositionalOrKeyword,
            required: false,
        };
        let (signature, members) = Signature::new(
            &qualification,
            callee_symbol.id(),
            0,
            SignatureForm::List,
            std::slice::from_ref(&shape),
        )
        .unwrap();
        let parameter_declaration = ParameterDeclaration {
            qualification: qualification.id(),
            parameter: members[0].id(),
            declaration: base.parameter.id(),
        };
        let destination = CallDestination::Resolved {
            symbol: callee_symbol.id(),
        };
        let target = CallTarget {
            qualification: qualification.id(),
            site: base.site.id(),
            destination: destination.id(),
            channel: CallChannel::Direct.id(),
            phase: CallPhase::Call,
            receiver: Receiver::None.id(),
            implicit: false,
            origin: CallOrigin::explicit(),
            receiver_class: None,
            passing: Some(ReceiverPassing::NotPassed),
            class_method: None,
            static_method: None,
        };
        let bound = bind_inspection(InspectionCase {
            input: input.id(),
            target: &target,
            qualification: &qualification,
            signature_qualification: &qualification,
            destination: &destination,
            channel: &CallChannel::Direct,
            receiver: &Receiver::None,
            signature: &signature,
            parameters: &members,
            shapes: &BTreeMap::from([(shape.id(), shape.clone())]),
            call: &base.call,
            arguments: &base.arguments,
        })
        .unwrap();
        // Caller: global t reaches the argument. Callee: timeout is returned when it is not None.
        let global = PlaceRoot::Global {
            module: modules[0].id(),
            name: "t".into(),
        };
        let argument = PlaceRoot::Occurrence {
            occurrence: base.actual.id(),
        };
        let returned = PlaceRoot::Return { callable: def.id() };
        let empty = AccessPath::empty();
        // The callee returns its entry value; its guard tests the parameter variable.
        let entry = PlaceRoot::Entry {
            declaration: base.parameter.id(),
        };
        let [source, delivered, result, port] =
            [&global, &argument, &returned, &entry].map(|root| Place {
                root: root.id(),
                path: empty.id(),
            });
        let branch = |owner: &ProviderSymbol, input: &Place, output: &Place, condition: Diagram| {
            let qualification = AssertionQualification {
                condition: condition.id(),
                ..qualification.clone()
            };
            let key = TransferKey {
                owner: owner.id(),
                input: input.id(),
                output: output.id(),
                context: context.id(),
                scope: qualification.scope,
                modality: Modality::Definite,
                approximation: Approximation::Exact,
                kind: TransferKind::Identity,
                call_site: None,
                provenance: ProvenanceClass::FlowLocal,
            };
            TransferBranch::new(key, qualification, condition).unwrap()
        };
        let caller = branch(&caller_symbol, &source, &delivered, Diagram::always());
        let callee = branch(
            &callee_symbol,
            &port,
            &result,
            Diagram::from_atom(base.guard.id()).not().unwrap(),
        );
        // Compose through the call over the stored rows.
        let mut places: BTreeMap<_, _> = base
            .rows::<Place>()
            .into_iter()
            .chain([
                source.clone(),
                delivered.clone(),
                result.clone(),
                port.clone(),
            ])
            .map(|p| (p.id(), p))
            .collect();
        let roots: BTreeMap<_, _> = base
            .rows::<PlaceRoot>()
            .into_iter()
            .chain([global, argument, returned, entry.clone()])
            .map(|r| (r.id(), r))
            .collect();
        let atoms = base
            .rows::<EvaluationAtom>()
            .into_iter()
            .map(|a| (a.id(), a))
            .collect();
        let predicates = base
            .rows::<Predicate>()
            .into_iter()
            .map(|p| (p.id(), p))
            .collect();
        let paths = BTreeMap::from([(empty.id(), empty.clone())]);
        let (segments, literals) = (BTreeMap::new(), BTreeMap::new());
        places.insert(base.place.id(), base.place.clone());
        let catalog = CompositionCatalog {
            guards: GuardCatalog {
                atoms: &atoms,
                predicates: &predicates,
                places: &places,
                roots: &roots,
            },
            paths: &paths,
            segments: PathCatalog {
                segments: &segments,
                literals: &literals,
            },
        };
        let witnesses = BTreeMap::from([(base.guard.id(), base.witness.clone())]);
        let frame = CallFrame {
            site: &base.site,
            target: &target,
            qualification: &qualification,
            destination: &destination,
            receiver: &Receiver::None,
            bound: Some(&bound),
            arguments: &base.arguments,
            summary_admitted: true,
            unique_variant: true,
        };
        let results = compose_call(
            &caller,
            &callee,
            &frame,
            &CallerFrame {
                declaration: &caller_declaration,
                site_owner: caller_root.id(),
            },
            &CalleeFrame {
                symbol: &callee_symbol,
                declaration: &callee_declaration,
                parameters: &members,
                links: std::slice::from_ref(&parameter_declaration),
                witnesses: Some(&witnesses),
            },
            &catalog,
        )
        .unwrap();
        let [CallComposition::Transfer(composed)] =
            <[CallComposition; 1]>::try_from(results).unwrap()
        else {
            panic!("expected one composed transfer")
        };
        let composed = *composed;
        // C07: the argument's value selects the composed flow through the restated guard.
        let bound_guard = composed
            .records
            .atoms
            .iter()
            .find(|a| a.evaluation == base.site.id())
            .unwrap()
            .clone();
        let influence = ControlInfluence {
            qualification: qualification.id(),
            input: delivered.id(),
            atom: bound_guard.id(),
            evaluation: base.site.id(),
        };
        let selection = composed
            .branch
            .selection(&influence, &qualification)
            .unwrap()
            .unwrap();
        let evidence = |occurrence: &Occurrence| Evidence::Occurrence {
            occurrence: occurrence.id(),
        };
        let support = (
            run.id(),
            Origin::AnalyzerAssertion,
            ExtractionMode::NativeTraversal,
            Fidelity::NativeStructural,
        );
        let derived = (
            run.id(),
            Origin::DerivedAnalysis,
            ExtractionMode::GraphAnalysis,
            Fidelity::NormalizedStructural,
        );
        let mut f = Self {
            base,
            run: run.clone(),
            caller_symbol: caller_symbol.clone(),
            callee_symbol: callee_symbol.clone(),
            caller_declaration: caller_declaration.clone(),
            callee_declaration: callee_declaration.clone(),
            signature: signature.clone(),
            target: target.clone(),
            delivered: delivered.clone(),
            caller: caller.clone(),
            callee: callee.clone(),
            composed,
            selection: selection.clone(),
        };
        f.extend(vec![provider]);
        f.extend(vec![run]);
        f.extend(families);
        f.extend(vec![declared.clone(), called.clone(), flows.clone()]);
        f.extend(modules.to_vec());
        f.extend(provider_modules.to_vec());
        f.extend(vec![caller_symbol, callee_symbol]);
        f.extend(vec![caller_declaration.clone(), callee_declaration.clone()]);
        f.extend(vec![parameter_declaration.clone()]);
        f.extend(vec![shape]);
        f.extend(vec![signature.clone()]);
        f.extend(members);
        f.extend(vec![destination]);
        f.extend(vec![CallChannel::Direct]);
        f.extend(vec![Receiver::None]);
        f.extend(vec![CallOrigin::new(&[]).unwrap().0]);
        f.extend(vec![target.clone()]);
        f.extend(vec![evidence(&caller_root), evidence(&def)]);
        f.extend(vec![
            SymbolDeclarationSupport {
                assertion: caller_declaration.id(),
                run: support.0,
                surface: declared.id(),
                evidence: evidence(&caller_root).id(),
                origin: support.1,
                mode: support.2,
                fidelity: support.3,
            },
            SymbolDeclarationSupport {
                assertion: callee_declaration.id(),
                run: support.0,
                surface: declared.id(),
                evidence: evidence(&def).id(),
                origin: support.1,
                mode: support.2,
                fidelity: support.3,
            },
        ]);
        f.extend(vec![ParameterDeclarationSupport {
            assertion: parameter_declaration.id(),
            run: support.0,
            surface: declared.id(),
            evidence: evidence(&f.base.parameter).id(),
            origin: support.1,
            mode: support.2,
            fidelity: support.3,
        }]);
        f.extend(vec![SignatureSupport {
            assertion: signature.id(),
            run: support.0,
            surface: declared.id(),
            evidence: evidence(&def).id(),
            origin: support.1,
            mode: support.2,
            fidelity: support.3,
        }]);
        f.extend(vec![CallTargetSupport {
            assertion: target.id(),
            run: support.0,
            surface: called.id(),
            evidence: evidence(&f.base.site).id(),
            origin: support.1,
            mode: support.2,
            fidelity: support.3,
        }]);
        f.extend(vec![
            source.clone(),
            delivered.clone(),
            result.clone(),
            port,
        ]);
        f.extend(vec![
            PlaceRoot::Global {
                module: modules[0].id(),
                name: "t".into(),
            },
            PlaceRoot::Occurrence {
                occurrence: f.base.actual.id(),
            },
            PlaceRoot::Return { callable: def.id() },
            entry,
        ]);
        f.store_branch(&caller, f.base.site.id(), derived);
        f.store_branch(&callee, def.id(), derived);
        f.store_composed(derived);
        f.extend(vec![influence.clone()]);
        f.extend(vec![selection]);
        f.extend(vec![ControlSupport {
            assertion: influence.id(),
            run: derived.0,
            surface: flows.id(),
            evidence: evidence(&f.base.site).id(),
            origin: derived.1,
            mode: derived.2,
            fidelity: derived.3,
        }]);
        f
    }
    /// Append rows to a relation, keeping one row per identity.
    pub fn extend<R: Record>(&mut self, rows: Vec<R>) {
        let mut seen = BTreeSet::new();
        let merged = self
            .base
            .rows::<R>()
            .into_iter()
            .chain(rows)
            .filter(|row| seen.insert(row.id()))
            .collect();
        self.base.put(merged);
    }
    pub fn rows<R: Record>(&self) -> Vec<R> {
        self.base.rows::<R>()
    }
    fn flows(&self) -> Id<ProviderSurface> {
        self.rows::<ProviderSurface>()
            .into_iter()
            .find(|s| s.family == FactFamily::Flow && s.provider == self.run.provider)
            .unwrap()
            .id()
    }
    /// Store a transfer alternative supported by evidence at `at`, which lies in its own source.
    fn store_branch(
        &mut self,
        branch: &TransferBranch,
        at: Id<Occurrence>,
        (run, origin, mode, fidelity): (Id<ProviderRun>, Origin, ExtractionMode, Fidelity),
    ) {
        let (condition, nodes) = branch.condition().records();
        let alternative = branch.alternative();
        let evidence = Evidence::Occurrence { occurrence: at };
        let surface = self.flows();
        self.extend(vec![condition]);
        self.extend(nodes);
        self.extend(vec![branch.qualification().clone()]);
        self.extend(vec![branch.key().clone()]);
        self.extend(vec![alternative.clone()]);
        self.extend(vec![evidence.clone()]);
        self.extend(vec![TransferSupport {
            assertion: alternative.id(),
            run,
            surface,
            evidence: evidence.id(),
            origin,
            mode,
            fidelity,
        }]);
    }
    fn store_composed(&mut self, derived: (Id<ProviderRun>, Origin, ExtractionMode, Fidelity)) {
        let records = &self.composed.records;
        let (literals, segments, paths, roots, places) = (
            records.literals.clone(),
            records.segments.clone(),
            records.paths.clone(),
            records.roots.clone(),
            records.places.clone(),
        );
        let (atoms, predicates, substitutions) = (
            records.atoms.clone(),
            records.predicates.clone(),
            records.substitutions.clone(),
        );
        let step = records.step.clone().unwrap();
        self.extend(literals);
        self.extend(segments);
        self.extend(paths);
        self.extend(roots);
        self.extend(places);
        self.extend(atoms);
        self.extend(predicates);
        self.extend(substitutions);
        let branch = self.composed.branch.clone();
        self.store_branch(&branch, self.base.site.id(), derived);
        self.extend(vec![step]);
    }
    /// Replace the composition step with a refused variant.
    pub fn mutate(&mut self, mutation: Mutation) {
        let step = self.composed.records.step.clone().unwrap();
        let refused = match mutation {
            Mutation::CalleeFromAnotherSymbol => CallCompositionStep {
                callee: self.caller.alternative().id(),
                ..step
            },
            Mutation::NotComposed => CallCompositionStep {
                composed: self.caller.alternative().id(),
                ..step
            },
            Mutation::UnrestatedGuard | Mutation::ErasedCondition => {
                // Same key and frame, but conditioned on the callee's own guard atom, or on nothing.
                let condition = if matches!(mutation, Mutation::ErasedCondition) {
                    Diagram::always()
                } else {
                    self.callee.condition().clone()
                };
                let qualification = AssertionQualification {
                    condition: condition.id(),
                    ..self.composed.branch.qualification().clone()
                };
                let branch = TransferBranch::new(
                    self.composed.branch.key().clone(),
                    qualification,
                    condition,
                )
                .unwrap();
                let alternative = branch.alternative().id();
                self.store_branch(
                    &branch,
                    self.base.site.id(),
                    (
                        self.run.id(),
                        Origin::DerivedAnalysis,
                        ExtractionMode::GraphAnalysis,
                        Fidelity::NormalizedStructural,
                    ),
                );
                CallCompositionStep {
                    composed: alternative,
                    ..step
                }
            }
            Mutation::ForeignOutput => {
                let root = PlaceRoot::Occurrence {
                    occurrence: self.base.callee_name.id(),
                };
                let output = Place {
                    root: root.id(),
                    path: AccessPath::empty().id(),
                };
                let key = TransferKey {
                    output: output.id(),
                    ..self.composed.branch.key().clone()
                };
                let branch = TransferBranch::new(
                    key,
                    self.composed.branch.qualification().clone(),
                    self.composed.branch.condition().clone(),
                )
                .unwrap();
                self.extend(vec![root]);
                self.extend(vec![output]);
                self.store_branch(
                    &branch,
                    self.base.site.id(),
                    (
                        self.run.id(),
                        Origin::DerivedAnalysis,
                        ExtractionMode::GraphAnalysis,
                        Fidelity::NormalizedStructural,
                    ),
                );
                CallCompositionStep {
                    composed: branch.alternative().id(),
                    ..step
                }
            }
        };
        self.base.put(vec![refused]);
    }
    /// Validate every stored relation and invariant, as the store would.
    pub fn validate(&self) -> Result<ContentHash, ModelError> {
        let model = &self.base.model;
        let generation = MemoryGeneration::conformance(model, &budget());
        macro_rules! each { ($($ty:ty),+ $(,)?) => { $( generation.put(&Batch::new(model, self.rows::<$ty>(), &budget()).unwrap())?; )+ }; }
        each!(
            InputRevision,
            InputOrigin,
            InputAcquisition,
            AnalysisContext,
            Provider,
            ProviderRun,
            RunFamily,
            ProviderSurface,
            CoverageScope,
            ProviderCoverage,
            Condition,
            ConditionNode,
            AssertionQualification,
            SourceArtifact,
            ArtifactChunk,
            Module,
            ProviderModule,
            ProviderSymbol,
            Occurrence,
            LexicalScope,
            Literal,
            PathSegment,
            PlaceRoot,
            AccessPath,
            Place,
            Predicate,
            EvaluationAtom,
            FlowUse,
            FlowDefinition,
            ReachingDefinition,
            FlowDefinitionObservation,
            FlowDefinitionSupport,
            FlowReachingObservation,
            FlowReachingSupport,
            CallSyntax,
            CallSyntaxSupport,
            CallArgument,
            Evidence,
            StabilityWitness,
            GuardSubstitution,
            SymbolDeclaration,
            SymbolDeclarationSupport,
            ParameterShape,
            Signature,
            SignatureParameter,
            SignatureSupport,
            ParameterDeclaration,
            ParameterDeclarationSupport,
            CallDestination,
            CallChannel,
            Receiver,
            CallOrigin,
            CallTarget,
            CallTargetSupport,
            TransferKey,
            TransferAlternative,
            TransferSupport,
            ControlInfluence,
            ControlSupport,
            Selection,
            CallCompositionStep
        );
        generation.validate(model, &budget())
    }
}

/// A fresh attempt budget; these controls do not share reservations across batches.
fn budget() -> lctx_model::domain::resources::ResourceBudget {
    lctx_model::domain::resources::ResourceBudget::fixed(
        lctx_model::domain::resources::DEFAULT_MEMORY_BYTES,
    )
    .unwrap()
}
