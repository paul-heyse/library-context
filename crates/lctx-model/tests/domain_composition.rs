//! Whole-call composition (C05/C06/C07) with pre-written answers over in-memory frames.
#[path = "fixtures/composition.rs"] mod fixture;
use std::collections::BTreeMap;
use lctx_model::domain::{*, assertion::*, attribution::*, calls::*, composition::*, conditions::{*, rebase::GuardCatalog, stability::*},
    declarations::*, flow::*, input::*, place_composition::PathCatalog, source::*, transfer::*, value::*};

/// Every row the composer may look up, filled as a scenario is built.
#[derive(Default)]
struct Rows {
    places: BTreeMap<Id<Place>, Place>, roots: BTreeMap<Id<PlaceRoot>, PlaceRoot>, paths: BTreeMap<Id<AccessPath>, AccessPath>,
    segments: BTreeMap<Id<PathSegment>, PathSegment>, literals: BTreeMap<Id<Literal>, Literal>,
    atoms: BTreeMap<Id<EvaluationAtom>, EvaluationAtom>, predicates: BTreeMap<Id<Predicate>, Predicate>,
}
struct World { rows: Rows, context: AnalysisContext, base: AssertionQualification, provider: Provider, source: SourceArtifact, next: i64,
    /// The modality the provider asserts for the next call's target.
    target_modality: Modality }
/// A call to a fresh callee. `formals` are the parameter variables, `entries` the entry-value ports.
struct Call { site: Occurrence, qualification: AssertionQualification, target: CallTarget, destination: CallDestination, receiver: Receiver, bound: BoundCall,
    arguments: Vec<CallArgument>, symbol: ProviderSymbol, declaration: SymbolDeclaration, members: Vec<SignatureParameter>,
    links: Vec<ParameterDeclaration>, formals: Vec<PlaceRoot>, entries: Vec<PlaceRoot>, actuals: Vec<Occurrence> }
struct Caller { symbol: ProviderSymbol, declaration: SymbolDeclaration }

impl World {
    fn new() -> Self {
        let input = InputRevision::from_entries(vec![]).unwrap();
        let source = SourceArtifact::from_bytes(input.id(), "m.py".into(), &[b' '; 4096]).unwrap();
        let context = AnalysisContext { python_version: "3.14.7".into(), python_platform: "linux".into(), search_path: vec![], site_package_path: vec![],
            config_digest: ContentHash::of(b"config"), environment_digest: ContentHash::of(b"env"), lock_digest: None };
        let base = AssertionQualification { context: context.id(), scope: CoverageScope::Input { input: input.id() }.id(), condition: Diagram::always().id(),
            modality: Modality::Definite, approximation: Approximation::Exact };
        let provider = Provider { tool: "pysa".into(), revision: "1".into(), build_digest: ContentHash::of(b"pysa") };
        Self { rows: Rows::default(), context, base, provider, source, next: 0, target_modality: Modality::Definite }
    }
    fn occurrence(&mut self, kind: SyntaxKind, role: OccurrenceRole) -> Occurrence {
        self.next += 1;
        Occurrence { source: self.source.id(), start: self.next, end: self.next + 1, syntax_kind: kind, role, structural_path: vec![0, self.next as i32] }
    }
    fn symbol(&mut self, name: &str) -> (ProviderSymbol, SymbolDeclaration) {
        let symbol = ProviderSymbol { provider: self.provider.id(), context: self.context.id(), module: ProviderModule::Bundled { provider: self.provider.id(), name: "m".into() }.id(),
            native_key: name.into(), name: name.into(), kind: SymbolKind::Function };
        let declaration = SymbolDeclaration { qualification: self.base.id(), symbol: symbol.id(), declaration: self.occurrence(SyntaxKind::StmtFunctionDef, OccurrenceRole::Declaration).id() };
        (symbol, declaration)
    }
    fn caller(&mut self) -> Caller { let (symbol, declaration) = self.symbol("caller"); Caller { symbol, declaration } }
    fn segment(&mut self, segment: PathSegment) -> Id<PathSegment> {
        if let PathSegment::Item { key } = &segment { let _ = key; }
        let id = segment.id(); self.rows.segments.insert(id, segment); id
    }
    fn attribute(&mut self, name: &str) -> Id<PathSegment> { self.segment(PathSegment::Attribute { name: name.into() }) }
    fn place(&mut self, root: PlaceRoot, segments: &[Id<PathSegment>]) -> Place {
        let path = segments.iter().fold(AccessPath::empty(), |path, segment| path.extend(*segment));
        let place = Place { root: root.id(), path: path.id() };
        self.rows.roots.insert(root.id(), root); self.rows.paths.insert(path.id(), path); self.rows.places.insert(place.id(), place.clone());
        place
    }
    fn branch(&self, owner: &ProviderSymbol, input: &Place, output: &Place, kind: TransferKind, condition: Diagram) -> TransferBranch {
        let qualification = AssertionQualification { condition: condition.id(), ..self.base.clone() };
        let key = TransferKey { owner: owner.id(), input: input.id(), output: output.id(), context: self.context.id(), scope: self.base.scope,
            modality: Modality::Definite, approximation: Approximation::Exact, kind, call_site: None, provenance: ProvenanceClass::FlowLocal };
        TransferBranch::new(key, qualification, condition).unwrap()
    }
    /// A call to a fresh callee with the given formals and arguments, bound by the one binder.
    fn call(&mut self, name: &str, shapes: &[ParameterShape], actuals: &[(ArgumentKind, Option<&str>)], has_receiver: bool) -> Call {
        let (symbol, declaration) = self.symbol(name);
        let site = self.occurrence(SyntaxKind::ExprCall, OccurrenceRole::Call);
        let callee_name = self.occurrence(SyntaxKind::ExprName, OccurrenceRole::Read);
        let actual_rows: Vec<_> = actuals.iter().map(|_| self.occurrence(SyntaxKind::ExprName, OccurrenceRole::Argument)).collect();
        let receiver_actual = self.occurrence(SyntaxKind::ExprName, OccurrenceRole::Read);
        let (signature, members) = Signature::new(&self.base, symbol.id(), 0, SignatureForm::List, shapes).unwrap();
        let parameter_rows: Vec<_> = shapes.iter().map(|_| self.occurrence(SyntaxKind::Parameter, OccurrenceRole::Parameter)).collect();
        let links: Vec<_> = members.iter().zip(&parameter_rows).map(|(member, occurrence)| ParameterDeclaration { qualification: self.base.id(), parameter: member.id(), declaration: occurrence.id() }).collect();
        let formals: Vec<_> = parameter_rows.iter().map(|o| PlaceRoot::Formal { declaration: o.id() }).collect();
        let entries: Vec<_> = parameter_rows.iter().map(|o| PlaceRoot::Entry { declaration: o.id() }).collect();
        let destination = CallDestination::Resolved { symbol: symbol.id() };
        let receiver = if has_receiver { Receiver::Bound { actual: receiver_actual.id() } } else { Receiver::None };
        let qualification = AssertionQualification { modality: self.target_modality, ..self.base.clone() };
        let target = CallTarget { qualification: qualification.id(), site: site.id(), destination: destination.id(), channel: CallChannel::Direct.id(), phase: CallPhase::Call, receiver: receiver.id(), implicit: false };
        let (syntax, arguments) = CallSyntax::new(self.base.id(), site.id(), callee_name.id(), false, &actuals.iter().zip(&actual_rows)
            .map(|((kind, keyword), occurrence)| Actual { occurrence: occurrence.id(), kind: *kind, keyword: keyword.map(str::to_owned) }).collect::<Vec<_>>()).unwrap();
        let shape_map = shapes.iter().map(|s| (s.id(), s.clone())).collect();
        let bound = bind(BindingInput { target: &target, qualification: &qualification, signature_qualification: &self.base, destination: &destination,
            channel: &CallChannel::Direct, receiver: &receiver, signature: &signature, parameters: &members, shapes: &shape_map, call: &syntax, arguments: &arguments }).unwrap();
        let mut actuals = actual_rows; if has_receiver { actuals.insert(0, receiver_actual); }
        Call { site, qualification, target, destination, receiver, bound, arguments, symbol, declaration, members, links, formals, entries, actuals }
    }
    fn catalog(&self) -> CompositionCatalog<'_> {
        CompositionCatalog { guards: GuardCatalog { atoms: &self.rows.atoms, predicates: &self.rows.predicates, places: &self.rows.places, roots: &self.rows.roots },
            paths: &self.rows.paths, segments: PathCatalog { segments: &self.rows.segments, literals: &self.rows.literals } }
    }
}
fn shape(name: &str, kind: ParameterKind, required: bool) -> ParameterShape { ParameterShape { name: Some(name.into()), kind, required } }
fn positional(name: &str) -> ParameterShape { shape(name, ParameterKind::PositionalOrKeyword, true) }
fn frame<'a>(call: &'a Call, summary: bool) -> CallFrame<'a> {
    CallFrame { site: &call.site, target: &call.target, qualification: &call.qualification, destination: &call.destination, receiver: &call.receiver, bound: Some(&call.bound),
        arguments: &call.arguments, summary_admitted: summary, unique_variant: true }
}
fn callee<'a>(call: &'a Call, witnesses: Option<&'a BTreeMap<Id<EvaluationAtom>, StabilityWitness>>) -> CalleeFrame<'a> {
    CalleeFrame { symbol: &call.symbol, declaration: &call.declaration, parameters: &call.members, links: &call.links, witnesses }
}
fn owner(caller: &Caller) -> CallerFrame<'_> { CallerFrame { declaration: &caller.declaration, site_owner: caller.declaration.declaration } }
fn compose(w: &World, caller: &Caller, from: &TransferBranch, call: &Call, through: &TransferBranch, summary: bool) -> Vec<CallComposition> {
    let empty = BTreeMap::new();
    compose_call(from, through, &frame(call, summary), &owner(caller), &callee(call, Some(&empty)), &w.catalog()).unwrap()
}
fn transfer(results: Vec<CallComposition>) -> ComposedTransfer {
    let mut results = results.into_iter();
    match (results.next(), results.next()) { (Some(CallComposition::Transfer(t)), None) => *t, other => panic!("expected one composed transfer, got {other:?}") }
}
fn obligation(results: Vec<CallComposition>) -> ObligationKind {
    match results.as_slice() { [CallComposition::Obligation(kind)] => *kind, other => panic!("expected one obligation, got {other:?}") }
}
/// The composed key's input and output as (root, path segment ids).
fn ends(w: &World, t: &ComposedTransfer) -> ((PlaceRoot, AccessPath), (PlaceRoot, AccessPath)) {
    let place = |id: Id<Place>| t.records.places.iter().find(|p| p.id() == id).unwrap().clone();
    let root = |id: Id<PlaceRoot>| t.records.roots.iter().chain(w.rows.roots.values()).find(|r| r.id() == id).unwrap().clone();
    let path = |id: Id<AccessPath>| t.records.paths.iter().find(|p| p.id() == id).unwrap().clone();
    let (input, output) = (place(t.branch.key().input), place(t.branch.key().output));
    ((root(input.root), path(input.path)), (root(output.root), path(output.path)))
}
fn path(segments: &[Id<PathSegment>]) -> AccessPath { segments.iter().fold(AccessPath::empty(), |p, s| p.extend(*s)) }

#[test]
fn paths_compose_only_through_identity_and_roots_map_through_the_binding() {
    let mut w = World::new(); let caller = w.caller();
    let timeout = w.attribute("timeout");
    let call = w.call("read_timeout", &[positional("cfg")], &[(ArgumentKind::Positional, None)], false);
    let x = w.occurrence(SyntaxKind::ExprName, OccurrenceRole::Read);
    let source = w.place(PlaceRoot::Occurrence { occurrence: x.id() }, &[]);
    let into_c = w.place(PlaceRoot::Occurrence { occurrence: call.actuals[0].id() }, &[]);
    let cfg_timeout = w.place(call.entries[0].clone(), &[timeout]);
    let returned = w.place(PlaceRoot::Return { callable: call.declaration.declaration }, &[]);
    let callee_reads = w.branch(&call.symbol, &cfg_timeout, &returned, TransferKind::Identity, Diagram::always());
    // read_timeout(c): x flows into c, the callee returns cfg.timeout, so x.timeout reaches the call.
    let delivered = w.branch(&caller.symbol, &source, &into_c, TransferKind::Identity, Diagram::always());
    let composed = compose(&w, &caller, &delivered, &call, &callee_reads, true);
    let t = transfer(composed);
    let site = PlaceRoot::Occurrence { occurrence: call.site.id() };
    assert_eq!(ends(&w, &t), ((PlaceRoot::Occurrence { occurrence: x.id() }, path(&[timeout])), (site.clone(), AccessPath::empty())));
    let key = t.branch.key();
    assert_eq!((key.owner, key.call_site, key.provenance, key.kind, key.modality), (caller.symbol.id(), Some(call.site.id()), ProvenanceClass::Composed, TransferKind::Identity, Modality::Definite));
    assert_eq!(t.records.step.as_ref().unwrap().caller, delivered.alternative().id());
    // A derived caller (d = parse(s)) does not inherit the callee's field path.
    let parsed = w.branch(&caller.symbol, &source, &into_c, TransferKind::Derived, Diagram::always());
    let t = transfer(compose(&w, &caller, &parsed, &call, &callee_reads, true));
    assert_eq!(ends(&w, &t).0, (PlaceRoot::Occurrence { occurrence: x.id() }, AccessPath::empty()));
    assert_eq!(t.branch.key().kind, TransferKind::Derived);
    // A caller delivering into c.timeout, through a derived callee (json.dumps(cfg)): the whole result, derived.
    let into_c_timeout = w.place(PlaceRoot::Occurrence { occurrence: call.actuals[0].id() }, &[timeout]);
    let cfg = w.place(call.entries[0].clone(), &[]);
    let dumps = w.branch(&call.symbol, &cfg, &returned, TransferKind::Derived, Diagram::always());
    let field_delivery = w.branch(&caller.symbol, &source, &into_c_timeout, TransferKind::Identity, Diagram::always());
    let t = transfer(compose(&w, &caller, &field_delivery, &call, &dumps, true));
    assert_eq!(ends(&w, &t).1, (site.clone(), AccessPath::empty())); assert_eq!(t.branch.key().kind, TransferKind::Derived);
    // ... and through an identity facade the field path survives on the result.
    let facade = w.branch(&call.symbol, &cfg, &returned, TransferKind::Identity, Diagram::always());
    assert_eq!(ends(&w, &transfer(compose(&w, &caller, &field_delivery, &call, &facade, true))).1, (site, path(&[timeout])));
    // A caller delivering elsewhere does not reach this call.
    let elsewhere = w.place(PlaceRoot::Occurrence { occurrence: x.id() }, &[]);
    let unrelated = w.branch(&caller.symbol, &source, &elsewhere, TransferKind::Identity, Diagram::always());
    assert!(matches!(compose(&w, &caller, &unrelated, &call, &callee_reads, true).as_slice(), [CallComposition::Disjoint]));
}

#[test]
fn outputs_map_to_mutated_actuals_fields_and_obligations() {
    let mut w = World::new(); let caller = w.caller();
    let any = w.segment(PathSegment::AnyItem);
    // update(t, k, v): v is stored into t[*]; the caller's value reaches its own t argument.
    let call = w.call("update", &[positional("t"), positional("k"), positional("v")], &[(ArgumentKind::Positional, None); 3], false);
    let x = w.occurrence(SyntaxKind::ExprName, OccurrenceRole::Read);
    let source = w.place(PlaceRoot::Occurrence { occurrence: x.id() }, &[]);
    let into_v = w.place(PlaceRoot::Occurrence { occurrence: call.actuals[2].id() }, &[]);
    let v = w.place(call.entries[2].clone(), &[]);
    let t_items = w.place(call.entries[0].clone(), &[any]);
    let store = w.branch(&call.symbol, &v, &t_items, TransferKind::Identity, Diagram::always());
    let delivered = w.branch(&caller.symbol, &source, &into_v, TransferKind::Identity, Diagram::always());
    let t = transfer(compose(&w, &caller, &delivered, &call, &store, true));
    assert_eq!(ends(&w, &t).1, (PlaceRoot::Occurrence { occurrence: call.actuals[0].id() }, path(&[any])));
    assert_eq!(t.branch.key().owner, caller.symbol.id(), "the composed transfer is the caller's, never the callee's");
    // Config.__init__(timeout): self.timeout = timeout is a field write, unchanged across the call.
    let field = PlaceRoot::Field { class: call.declaration.declaration, name: "timeout".into() };
    let field_place = w.place(field.clone(), &[]);
    let init = w.branch(&call.symbol, &v, &field_place, TransferKind::Identity, Diagram::always());
    assert_eq!(ends(&w, &transfer(compose(&w, &caller, &delivered, &call, &init, true))).1, (field, AccessPath::empty()));
    // Raising or yielding the value is not a caller place.
    for root in [PlaceRoot::Raise { callable: call.declaration.declaration }, PlaceRoot::Yield { callable: call.declaration.declaration }] {
        let out = w.place(root, &[]);
        let escape = w.branch(&call.symbol, &v, &out, TransferKind::Identity, Diagram::always());
        assert_eq!(obligation(compose(&w, &caller, &delivered, &call, &escape, true)), ObligationKind::UnsupportedControlFlow);
    }
    // A callee-local output root cannot cross the call.
    let local = w.occurrence(SyntaxKind::ExprName, OccurrenceRole::Binding);
    let local_place = w.place(PlaceRoot::Occurrence { occurrence: local.id() }, &[]);
    let internal = w.branch(&call.symbol, &v, &local_place, TransferKind::Identity, Diagram::always());
    let empty = BTreeMap::new();
    assert!(compose_call(&delivered, &internal, &frame(&call, true), &owner(&caller), &callee(&call, Some(&empty)), &w.catalog()).is_err());
    // A caller that does not own the call site cannot compose it.
    let elsewhere_owner = w.occurrence(SyntaxKind::StmtFunctionDef, OccurrenceRole::Declaration);
    let misowned = CallerFrame { declaration: &caller.declaration, site_owner: elsewhere_owner.id() };
    assert!(compose_call(&delivered, &store, &frame(&call, true), &misowned, &callee(&call, Some(&empty)), &w.catalog()).is_err());
    // with_default(v, y=[]): mutating a defaulted formal has no caller place.
    let defaulted = w.call("with_default", &[positional("v"), shape("y", ParameterKind::PositionalOrKeyword, false)], &[(ArgumentKind::Positional, None)], false);
    let into_v = w.place(PlaceRoot::Occurrence { occurrence: defaulted.actuals[0].id() }, &[]);
    let v = w.place(defaulted.entries[0].clone(), &[]); let y = w.place(defaulted.entries[1].clone(), &[any]);
    let append = w.branch(&defaulted.symbol, &v, &y, TransferKind::Identity, Diagram::always());
    let delivered = w.branch(&caller.symbol, &source, &into_v, TransferKind::Identity, Diagram::always());
    assert_eq!(obligation(compose(&w, &caller, &delivered, &defaulted, &append, true)), ObligationKind::DefaultUnavailable);
}

#[test]
fn aggregates_and_receivers_carry_their_projection() {
    let mut w = World::new(); let caller = w.caller();
    // collect(a, b, k=c) with def collect(*args, **kw).
    let call = w.call("collect", &[shape("args", ParameterKind::VarPositional, false), shape("kw", ParameterKind::VarKeyword, false)],
        &[(ArgumentKind::Positional, None), (ArgumentKind::Positional, None), (ArgumentKind::Keyword, Some("k"))], false);
    let x = w.occurrence(SyntaxKind::ExprName, OccurrenceRole::Read);
    let source = w.place(PlaceRoot::Occurrence { occurrence: x.id() }, &[]);
    let returned = w.place(PlaceRoot::Return { callable: call.declaration.declaration }, &[]);
    let site = PlaceRoot::Occurrence { occurrence: call.site.id() };
    let args = w.place(call.entries[0].clone(), &[]); let kw = w.place(call.entries[1].clone(), &[]);
    let returns_args = w.branch(&call.symbol, &args, &returned, TransferKind::Identity, Diagram::always());
    let returns_kw = w.branch(&call.symbol, &kw, &returned, TransferKind::Identity, Diagram::always());
    let into_b = w.place(PlaceRoot::Occurrence { occurrence: call.actuals[1].id() }, &[]);
    let t = transfer(compose(&w, &caller, &w.branch(&caller.symbol, &source, &into_b, TransferKind::Identity, Diagram::always()), &call, &returns_args, true));
    let one = PathSegment::Item { key: Literal::Integer { decimal: "1".into() }.id() }.id();
    assert_eq!(ends(&w, &t).1, (site.clone(), path(&[one])), "b is args[1] of the result");
    let into_c = w.place(PlaceRoot::Occurrence { occurrence: call.actuals[2].id() }, &[]);
    let t = transfer(compose(&w, &caller, &w.branch(&caller.symbol, &source, &into_c, TransferKind::Identity, Diagram::always()), &call, &returns_kw, true));
    let k = PathSegment::Item { key: Literal::String { value: "k".into() }.id() }.id();
    assert_eq!(ends(&w, &t).1, (site, path(&[k])), "c is kw['k'] of the result");
    // collect() binds only empty aggregates; nothing the caller delivers reaches it.
    let empty_call = w.call("collect", &[shape("args", ParameterKind::VarPositional, false)], &[], false);
    let somewhere = w.place(PlaceRoot::Occurrence { occurrence: x.id() }, &[]);
    let args = w.place(empty_call.entries[0].clone(), &[]);
    let returned = w.place(PlaceRoot::Return { callable: empty_call.declaration.declaration }, &[]);
    let through = w.branch(&empty_call.symbol, &args, &returned, TransferKind::Identity, Diagram::always());
    assert!(matches!(compose(&w, &caller, &w.branch(&caller.symbol, &source, &somewhere, TransferKind::Identity, Diagram::always()), &empty_call, &through, true).as_slice(), [CallComposition::Disjoint]));
    // obj.run(x): the receiver's value reaches the result through the receiver binding.
    let method = w.call("run", &[positional("self")], &[], true);
    let self_place = w.place(method.entries[0].clone(), &[]);
    let returned = w.place(PlaceRoot::Return { callable: method.declaration.declaration }, &[]);
    let returns_self = w.branch(&method.symbol, &self_place, &returned, TransferKind::Identity, Diagram::always());
    let into_obj = w.place(PlaceRoot::Occurrence { occurrence: method.actuals[0].id() }, &[]);
    let t = transfer(compose(&w, &caller, &w.branch(&caller.symbol, &source, &into_obj, TransferKind::Identity, Diagram::always()), &method, &returns_self, true));
    assert_eq!(ends(&w, &t).1, (PlaceRoot::Occurrence { occurrence: method.site.id() }, AccessPath::empty()));
    // A declared method's receiver has one encoding, its first parameter's entry value.
    let receiver_root = w.place(PlaceRoot::Receiver { callable: method.declaration.declaration }, &[]);
    let second_encoding = w.branch(&method.symbol, &receiver_root, &returned, TransferKind::Identity, Diagram::always());
    let empty = BTreeMap::new();
    assert!(compose_call(&w.branch(&caller.symbol, &source, &into_obj, TransferKind::Identity, Diagram::always()), &second_encoding,
        &frame(&method, true), &owner(&caller), &callee(&method, Some(&empty)), &w.catalog()).is_err());
}

#[test]
fn conditions_stay_conditional_and_unjustified_guards_refuse() {
    let mut w = World::new(); let caller = w.caller();
    let call = w.call("select_timeout", &[positional("timeout")], &[(ArgumentKind::Positional, None)], false);
    let x = w.occurrence(SyntaxKind::ExprName, OccurrenceRole::Read);
    let source = w.place(PlaceRoot::Occurrence { occurrence: x.id() }, &[]);
    let into_t = w.place(PlaceRoot::Occurrence { occurrence: call.actuals[0].id() }, &[]);
    // The guard tests the parameter variable; the transfer reads the entry value.
    let formal = w.place(call.formals[0].clone(), &[]);
    let entry = w.place(call.entries[0].clone(), &[]);
    let returned = w.place(PlaceRoot::Return { callable: call.declaration.declaration }, &[]);
    let evaluation = w.occurrence(SyntaxKind::ExprCompare, OccurrenceRole::Predicate);
    let guard = EvaluationAtom { evaluation: evaluation.id(), context: w.context.id(), predicate: Predicate::IsNone.id(), operand: Some(formal.id()) };
    let local = EvaluationAtom { evaluation: evaluation.id(), context: w.context.id(), predicate: Predicate::Opaque { text: "debug".into() }.id(), operand: None };
    for (atom, predicate) in [(&guard, Predicate::IsNone), (&local, Predicate::Opaque { text: "debug".into() })] {
        w.rows.atoms.insert(atom.id(), atom.clone()); w.rows.predicates.insert(predicate.id(), predicate);
    }
    let delivered = w.branch(&caller.symbol, &source, &into_t, TransferKind::Identity, Diagram::always());
    // return timeout under `not (timeout is None)`: without a stability witness this refuses, never true.
    let guarded = w.branch(&call.symbol, &entry, &returned, TransferKind::Identity, Diagram::from_atom(guard.id()).not().unwrap());
    assert_eq!(obligation(compose(&w, &caller, &delivered, &call, &guarded, true)), ObligationKind::ConditionTransferUnsupported);
    let none = compose_call(&delivered, &guarded, &frame(&call, true), &owner(&caller), &callee(&call, None), &w.catalog()).unwrap();
    assert_eq!(obligation(none), ObligationKind::NotRequested);
    // With a witness, the guard is restated over the bound actual (C07: its influence selects the flow).
    let reaching = FlowReachingObservation { qualification: w.base.id(), use_: FlowUse { occurrence: x.id(), place: formal.id() }.id(),
        target: ReachingDefinition::Unbound.id(), loop_carried: false };
    let witness = StabilityWitness { atom: guard.id(), reaching: reaching.id(), definition: FlowDefinitionObservation { qualification: w.base.id(),
        definition: FlowDefinition { occurrence: x.id(), place: formal.id() }.id(), scope: lexical::LexicalScope { owner: x.id(), kind: lexical::LexicalScopeKind::Function }.id(),
        kind: lexical::BindingEventKind::Parameter, value: None }.id(), coverage: ProviderCoverage { scope: w.base.scope, provider: w.provider.id(), context: w.context.id(),
        family: FactFamily::Flow, run: None, status: CoverageStatus::CompleteUnderStatedModel, reason: None, diagnostic: None }.id(), basis: StabilityBasis::ParameterOnlyReaching };
    let witnesses = BTreeMap::from([(guard.id(), witness)]);
    let results = compose_call(&delivered, &guarded, &frame(&call, true), &owner(&caller), &callee(&call, Some(&witnesses)), &w.catalog()).unwrap();
    let t = transfer(results);
    let bound = t.records.atoms.iter().find(|a| a.evaluation == call.site.id()).unwrap();
    assert!(t.branch.condition().support().contains(&bound.id()), "the composed flow stays conditional on the restated guard");
    let influence = ControlInfluence { qualification: w.base.id(), input: into_t.id(), atom: bound.id(), evaluation: call.site.id() };
    assert!(t.branch.selection(&influence, &w.base).unwrap().is_some(), "the actual's value selects this composed flow");
    // A callee-local guard stays an opaque, conditional guard at the call.
    let debug_only = w.branch(&call.symbol, &entry, &returned, TransferKind::Identity, Diagram::from_atom(local.id()));
    let t = transfer(compose(&w, &caller, &delivered, &call, &debug_only, true));
    assert_eq!(t.branch.condition().support().len(), 1);
    assert!(t.records.predicates.iter().any(|p| matches!(p, Predicate::InvokedGuard { source } if *source == local.id())));
}

#[test]
fn complementary_caller_conditions_merge_to_true() {
    let mut w = World::new(); let caller = w.caller();
    let call = w.call("select", &[positional("v")], &[(ArgumentKind::Positional, None)], false);
    let x = w.occurrence(SyntaxKind::ExprName, OccurrenceRole::Read);
    let source = w.place(PlaceRoot::Occurrence { occurrence: x.id() }, &[]);
    let into_v = w.place(PlaceRoot::Occurrence { occurrence: call.actuals[0].id() }, &[]);
    let formal = w.place(call.entries[0].clone(), &[]);
    let returned = w.place(PlaceRoot::Return { callable: call.declaration.declaration }, &[]);
    let evaluation = w.occurrence(SyntaxKind::ExprName, OccurrenceRole::Predicate);
    let predicate = Predicate::Opaque { text: "b".into() };
    let b = EvaluationAtom { evaluation: evaluation.id(), context: w.context.id(), predicate: predicate.id(), operand: None };
    w.rows.atoms.insert(b.id(), b.clone()); w.rows.predicates.insert(predicate.id(), predicate);
    // fetch: `select(x)` under b and again under not b; both compose to the same caller key.
    let through = w.branch(&call.symbol, &formal, &returned, TransferKind::Identity, Diagram::always());
    let [when, otherwise] = [Diagram::from_atom(b.id()), Diagram::from_atom(b.id()).not().unwrap()]
        .map(|condition| transfer(compose(&w, &caller, &w.branch(&caller.symbol, &source, &into_v, TransferKind::Identity, condition), &call, &through, true)));
    assert_eq!(when.branch.key(), otherwise.branch.key());
    assert_ne!(when.branch.alternative(), otherwise.branch.alternative(), "each alternative keeps its own identity");
    let merged = merge([when.branch, otherwise.branch]).unwrap();
    assert_eq!(merged.len(), 1);
    assert_eq!((merged[0].condition.id(), merged[0].alternatives.len()), (Diagram::always().id(), 2), "b or not b is unconditional");
}

#[test]
fn every_admitted_alternative_composes_separately_and_modality_follows_the_site() {
    let mut w = World::new(); let caller = w.caller();
    let x = w.occurrence(SyntaxKind::ExprName, OccurrenceRole::Read);
    let source = w.place(PlaceRoot::Occurrence { occurrence: x.id() }, &[]);
    let first = w.call("f", &[positional("a")], &[(ArgumentKind::Positional, None)], false);
    // A second target of the same site shares its occurrence, arguments and binding shape.
    let mut second = w.call("g", &[positional("a")], &[(ArgumentKind::Positional, None)], false);
    second.site = first.site.clone(); second.actuals = first.actuals.clone();
    second.target = CallTarget { site: first.site.id(), ..second.target.clone() };
    let (syntax, arguments) = CallSyntax::new(w.base.id(), first.site.id(), first.actuals[0].id(), false,
        &[Actual { occurrence: first.actuals[0].id(), kind: ArgumentKind::Positional, keyword: None }]).unwrap();
    let (signature, members) = Signature::new(&w.base, second.symbol.id(), 0, SignatureForm::List, &[positional("a")]).unwrap();
    second.links[0].parameter = members[0].id(); second.members = members.clone();
    second.bound = bind(BindingInput { target: &second.target, qualification: &w.base, signature_qualification: &w.base, destination: &second.destination,
        channel: &CallChannel::Direct, receiver: &Receiver::None, signature: &signature, parameters: &members,
        shapes: &BTreeMap::from([(positional("a").id(), positional("a"))]), call: &syntax, arguments: &arguments }).unwrap();
    second.arguments = arguments;
    let into_a = w.place(PlaceRoot::Occurrence { occurrence: first.actuals[0].id() }, &[]);
    let delivered = w.branch(&caller.symbol, &source, &into_a, TransferKind::Identity, Diagram::always());
    let branches: Vec<Vec<TransferBranch>> = [&first, &second].iter().map(|call| {
        let formal = w.place(call.entries[0].clone(), &[]);
        let returned = w.place(PlaceRoot::Return { callable: call.declaration.declaration }, &[]);
        vec![w.branch(&call.symbol, &formal, &returned, TransferKind::Identity, Diagram::always())]
    }).collect();
    let empty = BTreeMap::new();
    let calls = [SiteCall { frame: frame(&first, false), callee: Some(callee(&first, Some(&empty))), branches: &branches[0] },
        SiteCall { frame: frame(&second, false), callee: Some(callee(&second, Some(&empty))), branches: &branches[1] }];
    let results = compose_site(std::slice::from_ref(&delivered), &calls, &owner(&caller), &w.catalog()).unwrap();
    assert_eq!(results.len(), 2, "two targets are two candidate flows, never one merged invocation");
    for result in &results { let CallComposition::Transfer(t) = result else { panic!() }; assert_eq!(t.branch.key().modality, Modality::Candidate); }
    // One binding variant of a summarized target is definite; two variants make it a candidate.
    let definite = transfer(compose(&w, &caller, &delivered, &first, &branches[0][0], true));
    assert_eq!(definite.branch.key().modality, Modality::Definite);
    let mut several = frame(&first, true); several.unique_variant = false;
    let candidate = compose_call(&delivered, &branches[0][0], &several, &owner(&caller), &callee(&first, Some(&empty)), &w.catalog()).unwrap();
    assert_eq!(transfer(candidate).branch.key().modality, Modality::Candidate);
    // An unresolved target is an obligation, not an absent flow, even with no callee to compose.
    let unresolved = CallDestination::Unresolved { reason: ObligationKind::UnresolvedTarget };
    let open_target = CallTarget { destination: unresolved.id(), ..first.target.clone() };
    let open = CallFrame { target: &open_target, destination: &unresolved, bound: None, ..frame(&first, false) };
    let results = compose_site(std::slice::from_ref(&delivered), &[SiteCall { frame: open, callee: None, branches: &[] }], &owner(&caller), &w.catalog()).unwrap();
    assert_eq!(obligation(results), ObligationKind::UnresolvedTarget);
    // A value delivered into the receiver of an unresolved method call is that call's obligation.
    let method = w.call("m", &[positional("self")], &[], true);
    let into_receiver = w.place(PlaceRoot::Occurrence { occurrence: method.actuals[0].id() }, &[]);
    let open_method = CallTarget { destination: unresolved.id(), ..method.target.clone() };
    let open = CallFrame { target: &open_method, destination: &unresolved, bound: None, ..frame(&method, false) };
    let delivered_receiver = w.branch(&caller.symbol, &source, &into_receiver, TransferKind::Identity, Diagram::always());
    let results = compose_site(std::slice::from_ref(&delivered_receiver), &[SiteCall { frame: open, callee: None, branches: &[] }], &owner(&caller), &w.catalog()).unwrap();
    assert_eq!(obligation(results), ObligationKind::UnresolvedTarget);
    // A frame whose destination is not its target's is refused.
    let forged = CallFrame { destination: &unresolved, ..frame(&first, false) };
    assert!(compose_call(&delivered, &branches[0][0], &forged, &owner(&caller), &callee(&first, Some(&empty)), &w.catalog()).is_err());
}

#[test]
fn an_oversized_condition_is_a_limit_obligation_never_true() {
    let mut w = World::new(); let caller = w.caller();
    let call = w.call("f", &[positional("a")], &[(ArgumentKind::Positional, None)], false);
    let x = w.occurrence(SyntaxKind::ExprName, OccurrenceRole::Read);
    let source = w.place(PlaceRoot::Occurrence { occurrence: x.id() }, &[]);
    let into_a = w.place(PlaceRoot::Occurrence { occurrence: call.actuals[0].id() }, &[]);
    let formal = w.place(call.entries[0].clone(), &[]);
    let returned = w.place(PlaceRoot::Return { callable: call.declaration.declaration }, &[]);
    let atoms = |w: &mut World, count: usize, name: &str| -> Diagram {
        (0..count).fold(Diagram::always(), |condition, n| {
            let evaluation = w.occurrence(SyntaxKind::ExprName, OccurrenceRole::Predicate);
            let predicate = Predicate::Opaque { text: format!("{name}{n}") };
            let atom = EvaluationAtom { evaluation: evaluation.id(), context: w.context.id(), predicate: predicate.id(), operand: None };
            w.rows.atoms.insert(atom.id(), atom.clone()); w.rows.predicates.insert(predicate.id(), predicate);
            condition.and(&Diagram::from_atom(atom.id())).unwrap()
        })
    };
    let (caller_condition, callee_condition) = (atoms(&mut w, 100, "caller"), atoms(&mut w, 100, "callee"));
    let delivered = w.branch(&caller.symbol, &source, &into_a, TransferKind::Identity, caller_condition);
    let through = w.branch(&call.symbol, &formal, &returned, TransferKind::Identity, callee_condition);
    let kind = obligation(compose(&w, &caller, &delivered, &call, &through, true));
    assert!(matches!(kind, ObligationKind::ConditionAtomLimit | ObligationKind::ConditionNodeLimit | ObligationKind::ConditionWorkLimit), "{kind:?}");
}

#[test]
fn stored_compositions_validate_and_mismatched_steps_refuse() {
    let f = fixture::Fixture::new();
    let key = f.composed.branch.key();
    assert_eq!((key.owner, key.call_site, key.provenance), (f.caller_symbol.id(), Some(f.base.site.id()), ProvenanceClass::Composed));
    assert_eq!((key.input, key.output), (f.caller.key().input, Place { root: PlaceRoot::Occurrence { occurrence: f.base.site.id() }.id(), path: AccessPath::empty().id() }.id()));
    assert!(f.composed.records.substitutions.iter().any(|s| s.witness == f.base.witness.id()), "the formal guard is restated through its witness");
    f.validate().unwrap();
    for (mutation, reason) in [(fixture::Mutation::CalleeFromAnotherSymbol, "differ from the target's symbol"),
        (fixture::Mutation::NotComposed, "not the caller's transfer at the target's site"), (fixture::Mutation::ForeignOutput, "not a caller-side place of the call"), (fixture::Mutation::UnrestatedGuard, "restated at the call"), (fixture::Mutation::ErasedCondition, "restated at the call")] {
        let mut refused = fixture::Fixture::new(); refused.mutate(mutation);
        let error = refused.validate().unwrap_err();
        assert!(matches!(error, ModelError::Invalid(ref m) if m.contains(reason)), "{mutation:?}: {error}");
    }
}

#[test]
fn rebound_formals_and_slot_writes_never_reach_the_caller() {
    let mut w = World::new(); let caller = w.caller();
    let any = w.segment(PathSegment::AnyItem); let x_attr = w.attribute("x");
    let call = w.call("f", &[positional("t"), positional("v")], &[(ArgumentKind::Positional, None); 2], false);
    let x = w.occurrence(SyntaxKind::ExprName, OccurrenceRole::Read);
    let source = w.place(PlaceRoot::Occurrence { occurrence: x.id() }, &[]);
    let into_v = w.place(PlaceRoot::Occurrence { occurrence: call.actuals[1].id() }, &[]);
    let into_t = w.place(PlaceRoot::Occurrence { occurrence: call.actuals[0].id() }, &[]);
    let delivered = w.branch(&caller.symbol, &source, &into_v, TransferKind::Identity, Diagram::always());
    let v = w.place(call.entries[1].clone(), &[]);
    let through = |w: &mut World, output: Place| w.branch(&call.symbol, &v, &output, TransferKind::Identity, Diagram::always());
    // `t = []; t.append(v)` and `t = t or []; t.append(v)`: the append targets whatever t now holds.
    let rebound_items = w.place(call.formals[0].clone(), &[any]);
    let append = through(&mut w, rebound_items);
    assert_eq!(obligation(compose(&w, &caller, &delivered, &call, &append, true)), ObligationKind::EntryValueUnknown);
    // `t = v`: rebinding the parameter variable never reaches the caller's `a`.
    let rebound = w.place(call.formals[0].clone(), &[]);
    let rebind = through(&mut w, rebound);
    assert!(matches!(compose(&w, &caller, &delivered, &call, &rebind, true).as_slice(), [CallComposition::Disjoint]));
    // Reading the variable is not reading the caller's value.
    let returned = w.place(PlaceRoot::Return { callable: call.declaration.declaration }, &[]);
    let t_variable = w.place(call.formals[0].clone(), &[]);
    let reads_variable = w.branch(&call.symbol, &t_variable, &returned, TransferKind::Identity, Diagram::always());
    let into_first = w.branch(&caller.symbol, &source, &into_t, TransferKind::Identity, Diagram::always());
    assert_eq!(obligation(compose(&w, &caller, &into_first, &call, &reads_variable, true)), ObligationKind::EntryValueUnknown);
    // `t.x = v` below the entry value mutates the caller's object.
    let entry_x = w.place(call.entries[0].clone(), &[x_attr]);
    let store = through(&mut w, entry_x);
    assert_eq!(ends(&w, &transfer(compose(&w, &caller, &delivered, &call, &store, true))).1, (PlaceRoot::Occurrence { occurrence: call.actuals[0].id() }, path(&[x_attr])));
    // `def g(v, **kw): kw['k'] = v` at `g(x, k=c)`: storing the element replaces a slot of a fresh
    // dict; `kw['k'].x = v` mutates c.
    let g = w.call("g", &[positional("v"), shape("kw", ParameterKind::VarKeyword, false)], &[(ArgumentKind::Positional, None), (ArgumentKind::Keyword, Some("k"))], false);
    let into_gv = w.place(PlaceRoot::Occurrence { occurrence: g.actuals[0].id() }, &[]);
    let delivered = w.branch(&caller.symbol, &source, &into_gv, TransferKind::Identity, Diagram::always());
    let k = w.segment(PathSegment::Item { key: Literal::String { value: "k".into() }.id() });
    w.rows.literals.insert(Literal::String { value: "k".into() }.id(), Literal::String { value: "k".into() });
    let gv = w.place(g.entries[0].clone(), &[]);
    let slot = w.place(g.entries[1].clone(), &[k]);
    let element = w.branch(&g.symbol, &gv, &slot, TransferKind::Identity, Diagram::always());
    assert!(matches!(compose(&w, &caller, &delivered, &g, &element, true).as_slice(), [CallComposition::Disjoint]));
    let below = w.place(g.entries[1].clone(), &[k, x_attr]);
    let mutation = w.branch(&g.symbol, &gv, &below, TransferKind::Identity, Diagram::always());
    assert_eq!(ends(&w, &transfer(compose(&w, &caller, &delivered, &g, &mutation, true))).1, (PlaceRoot::Occurrence { occurrence: g.actuals[1].id() }, path(&[x_attr])));
}

#[test]
fn ports_are_total_over_the_bound_signature() {
    let mut w = World::new(); let caller = w.caller();
    let x_attr = w.attribute("x");
    let x = w.occurrence(SyntaxKind::ExprName, OccurrenceRole::Read);
    let source = w.place(PlaceRoot::Occurrence { occurrence: x.id() }, &[]);
    // `C.m(obj, v)` with `self.x = v`: the unbound call passes the receiver positionally.
    let method = w.call("m", &[positional("self"), positional("v")], &[(ArgumentKind::Positional, None); 2], false);
    let v = w.place(method.entries[1].clone(), &[]);
    let receiver_x = w.place(method.entries[0].clone(), &[x_attr]);
    let store = w.branch(&method.symbol, &v, &receiver_x, TransferKind::Identity, Diagram::always());
    let into_v = w.place(PlaceRoot::Occurrence { occurrence: method.actuals[1].id() }, &[]);
    let delivered = w.branch(&caller.symbol, &source, &into_v, TransferKind::Identity, Diagram::always());
    assert_eq!(ends(&w, &transfer(compose(&w, &caller, &delivered, &method, &store, true))).1, (PlaceRoot::Occurrence { occurrence: method.actuals[0].id() }, path(&[x_attr])));
    // Links and parameters must be the bound variant's, complete and one-to-one.
    let call = w.call("f", &[positional("a"), shape("b", ParameterKind::PositionalOrKeyword, false)], &[(ArgumentKind::Positional, None)], false);
    let a = w.place(call.entries[0].clone(), &[]);
    let returned = w.place(PlaceRoot::Return { callable: call.declaration.declaration }, &[]);
    let identity = w.branch(&call.symbol, &a, &returned, TransferKind::Identity, Diagram::always());
    let into_a = w.place(PlaceRoot::Occurrence { occurrence: call.actuals[0].id() }, &[]);
    let delivered = w.branch(&caller.symbol, &source, &into_a, TransferKind::Identity, Diagram::always());
    let (_, other) = Signature::new(&w.base, call.symbol.id(), 1, SignatureForm::List, &[positional("a")]).unwrap();
    let shared = vec![ParameterDeclaration { qualification: w.base.id(), parameter: other[0].id(), declaration: call.links[0].declaration }];
    let empty = BTreeMap::new();
    let with = |parameters: &[SignatureParameter], links: &[ParameterDeclaration]| compose_call(&delivered, &identity, &frame(&call, true), &owner(&caller),
        &CalleeFrame { symbol: &call.symbol, declaration: &call.declaration, parameters, links, witnesses: Some(&empty) }, &w.catalog());
    assert!(with(&call.members, &shared).is_err(), "a link of another variant sharing the occurrence is refused");
    assert!(with(&other, &shared).is_err(), "parameters must be the bound variant's");
    assert_eq!(obligation(with(&call.members, &call.links[..1]).unwrap()), ObligationKind::NoSourceDeclaration);
    assert!(matches!(with(&call.members, &call.links).unwrap().as_slice(), [CallComposition::Transfer(_)]));
    // A root naming another callable's parameter (a closure read) is not a port of this call:
    // nothing the caller delivers enters through it.
    let foreign = w.place(PlaceRoot::Entry { declaration: method.links[1].declaration }, &[]);
    let crossing = w.branch(&call.symbol, &foreign, &returned, TransferKind::Identity, Diagram::always());
    assert!(matches!(compose_call(&delivered, &crossing, &frame(&call, true), &owner(&caller), &callee(&call, Some(&empty)), &w.catalog()).unwrap().as_slice(),
        [CallComposition::Disjoint]));
}

#[test]
fn the_target_modality_bounds_the_composition() {
    let mut w = World::new(); let caller = w.caller();
    w.target_modality = Modality::Potential;
    let call = w.call("f", &[positional("a")], &[(ArgumentKind::Positional, None)], false);
    let x = w.occurrence(SyntaxKind::ExprName, OccurrenceRole::Read);
    let source = w.place(PlaceRoot::Occurrence { occurrence: x.id() }, &[]);
    let into_a = w.place(PlaceRoot::Occurrence { occurrence: call.actuals[0].id() }, &[]);
    let a = w.place(call.entries[0].clone(), &[]);
    let returned = w.place(PlaceRoot::Return { callable: call.declaration.declaration }, &[]);
    let through = w.branch(&call.symbol, &a, &returned, TransferKind::Identity, Diagram::always());
    let delivered = w.branch(&caller.symbol, &source, &into_a, TransferKind::Identity, Diagram::always());
    assert_eq!(transfer(compose(&w, &caller, &delivered, &call, &through, true)).branch.key().modality, Modality::Potential,
        "a potential target never composes into a candidate or definite flow");
}
