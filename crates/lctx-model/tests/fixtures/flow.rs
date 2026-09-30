#[allow(
    dead_code,
    reason = "Shared generated contracts and fixtures require this scoped exception"
)]
#[path = "lexical.rs"]
mod lexical_fixture;
use lctx_model::domain::{
    assertion::*, attribution::*, flow::*, lexical::*, source::*, transfer::TransferKind, value::*,
    *,
};
pub struct Fixture {
    pub base: lexical_fixture::Fixture,
    pub use_: FlowUse,
    pub definition: FlowDefinition,
    pub reaching: FlowReachingObservation,
    pub reaching_support: FlowReachingSupport,
}
impl Fixture {
    pub fn new() -> Self {
        let mut base = lexical_fixture::Fixture::new();
        let input = base
            .rows::<lctx_model::domain::input::InputRevision>()
            .pop()
            .unwrap();
        let context = base.rows::<AnalysisContext>().pop().unwrap();
        let provider = Provider {
            tool: "flow-fixture".into(),
            revision: "1".into(),
            build_digest: ContentHash::of(b"flow-fixture"),
        };
        let (run, families) = ProviderRun::new(
            provider.id(),
            context.id(),
            input.id(),
            context.config_digest,
            [FactFamily::Flow],
        )
        .unwrap();
        let surface = ProviderSurface {
            provider: provider.id(),
            family: FactFamily::Flow,
            name: "flow observations".into(),
        };
        let q = base.rows::<AssertionQualification>().pop().unwrap().id();
        let occurrences = base.rows::<Occurrence>();
        let read = occurrences
            .iter()
            .find(|o| o.id() == base.resolution.read)
            .unwrap()
            .clone();
        let site = base.rows::<BindingEvent>().pop().unwrap().site;
        let module = Module {
            source: read.source,
            qualified_name: "example".into(),
        };
        let root = PlaceRoot::Global {
            module: module.id(),
            name: "x".into(),
        };
        let path = AccessPath::empty();
        let place = Place {
            root: root.id(),
            path: path.id(),
        };
        let use_ = FlowUse {
            occurrence: read.id(),
            place: place.id(),
        };
        let definition = FlowDefinition {
            occurrence: site,
            place: place.id(),
        };
        let target = ReachingDefinition::Bound {
            definition: definition.id(),
        };
        let reaching = FlowReachingObservation {
            qualification: q,
            use_: use_.id(),
            target: target.id(),
            loop_carried: false,
        };
        let module_scope = base
            .rows::<LexicalScope>()
            .into_iter()
            .find(|s| s.kind == LexicalScopeKind::Module)
            .unwrap()
            .id();
        let use_obs = FlowUseObservation {
            qualification: q,
            use_: use_.id(),
            scope: base.function_scope.scope,
            annotation: false,
        };
        let def_obs = FlowDefinitionObservation {
            qualification: q,
            definition: definition.id(),
            scope: module_scope,
            kind: BindingEventKind::Assignment,
            value: base.binding.value,
        };
        let returned = Occurrence {
            start: read.start - 7,
            syntax_kind: SyntaxKind::StmtReturn,
            role: OccurrenceRole::Return,
            structural_path: vec![0, 1, 0],
            ..read.clone()
        };
        let value = FlowValueObservation {
            qualification: q,
            use_: use_.id(),
            sink: returned.id(),
            kind: FlowSinkKind::Return,
            transfer: TransferKind::Identity,
            through_call: false,
        };
        let region = FlowRegionObservation {
            qualification: q,
            statement: returned.id(),
            scope: base.function_scope.scope,
        };
        let use_evidence = Evidence::Occurrence {
            occurrence: read.id(),
        };
        let def_evidence = Evidence::Occurrence { occurrence: site };
        let return_evidence = Evidence::Occurrence {
            occurrence: returned.id(),
        };
        macro_rules! support { ($ty:ident,$row:expr,$e:expr) => { $ty { assertion: $row.id(),run: run.id(),surface: surface.id(),evidence: $e.id(),
            origin: Origin::AnalyzerAssertion,mode: ExtractionMode::NativeTraversal,fidelity: Fidelity::NativeStructural } }; }
        let reaching_support = support!(FlowReachingSupport, reaching, use_evidence);
        let coverage = ProviderCoverage {
            scope: base.rows::<AssertionQualification>()[0].scope,
            provider: Some(provider.id()),
            context: context.id(),
            family: FactFamily::Flow,
            run: Some(run.id()),
            status: CoverageStatus::CompleteUnderStatedModel,
            reason: None,
            diagnostic: None,
        };
        macro_rules! append {
            ($ty:ty,$rows:expr) => {
                let mut rows = base.rows::<$ty>();
                rows.extend($rows);
                base.put(rows);
            };
        }
        append!(Provider, vec![provider.clone()]);
        append!(ProviderRun, vec![run.clone()]);
        append!(RunFamily, families);
        append!(ProviderSurface, vec![surface.clone()]);
        append!(Module, vec![module]);
        append!(PlaceRoot, vec![root]);
        append!(AccessPath, vec![path]);
        append!(Place, vec![place]);
        append!(Occurrence, vec![returned]);
        append!(Evidence, vec![return_evidence.clone()]);
        append!(ProviderCoverage, vec![coverage]);
        macro_rules! one { ($($row:expr),+ $(,)?) => { $(base.put(vec![$row.clone()]);)+ }; }
        one!(
            use_,
            definition,
            target,
            reaching,
            reaching_support,
            use_obs,
            support!(FlowUseSupport, use_obs, use_evidence),
            def_obs,
            support!(FlowDefinitionSupport, def_obs, def_evidence),
            value,
            support!(FlowValueSupport, value, return_evidence),
            region,
            support!(FlowRegionSupport, region, return_evidence)
        );
        Self {
            base,
            use_,
            definition,
            reaching,
            reaching_support,
        }
    }
    pub fn foreign_place(&mut self) {
        let root = PlaceRoot::Occurrence {
            occurrence: self.base.foreign.id(),
        };
        let place = Place {
            root: root.id(),
            path: AccessPath::empty().id(),
        };
        let mut roots = self.base.rows::<PlaceRoot>();
        roots.push(root);
        self.base.put(roots);
        let mut places = self.base.rows::<Place>();
        places.push(place.clone());
        self.base.put(places);
        self.definition.place = place.id();
        let mut def = self.base.rows::<FlowDefinitionObservation>().pop().unwrap();
        def.definition = self.definition.id();
        let mut support = self.base.rows::<FlowDefinitionSupport>().pop().unwrap();
        support.assertion = def.id();
        self.base.put(vec![self.definition.clone()]);
        self.base.put(vec![def]);
        self.base.put(vec![support]);
        let target = ReachingDefinition::Bound {
            definition: self.definition.id(),
        };
        self.reaching.target = target.id();
        self.reaching_support.assertion = self.reaching.id();
        self.base.put(vec![target]);
        self.base.put(vec![self.reaching.clone()]);
        self.base.put(vec![self.reaching_support.clone()]);
    }
}
impl Fixture {
    /// Independent contract geometry; raw producer controls separately prove parsed coordinates.
    pub fn extended(&mut self) {
        use lctx_model::domain::{calls::*, conditions::*};
        let q = self.base.rows::<AssertionQualification>()[0].clone();
        let run = self
            .base
            .rows::<ProviderRun>()
            .into_iter()
            .find(|r| r.id() == self.reaching_support.run)
            .unwrap();
        let surface = self
            .base
            .rows::<ProviderSurface>()
            .into_iter()
            .find(|s| s.id() == self.reaching_support.surface)
            .unwrap();
        let read = self
            .base
            .rows::<Occurrence>()
            .into_iter()
            .find(|o| o.id() == self.use_.occurrence)
            .unwrap();
        let call = Occurrence {
            start: read.start - 2,
            end: read.end + 1,
            syntax_kind: SyntaxKind::ExprCall,
            role: OccurrenceRole::Call,
            structural_path: read.structural_path[..read.structural_path.len() - 1].to_vec(),
            ..read.clone()
        };
        let callee = Occurrence {
            start: call.start,
            end: call.start + 1,
            structural_path: vec![0, 1, 0, 1],
            ..read.clone()
        };
        let (syntax, arguments) = CallSyntax::new(
            q.id(),
            call.id(),
            callee.id(),
            false,
            &[Actual {
                occurrence: read.id(),
                kind: ArgumentKind::Positional,
                keyword: None,
            }],
        )
        .unwrap();
        let (syntax_run, families) = ProviderRun::new(
            run.provider,
            run.context,
            run.input,
            ContentHash::of(b"syntax fixture"),
            [FactFamily::Syntax],
        )
        .unwrap();
        let syntax_surface = ProviderSurface {
            provider: run.provider,
            family: FactFamily::Syntax,
            name: "flow call contract".into(),
        };
        let evidence = Evidence::Occurrence {
            occurrence: call.id(),
        };
        let syntax_support = CallSyntaxSupport {
            assertion: syntax.id(),
            run: syntax_run.id(),
            surface: syntax_surface.id(),
            evidence: evidence.id(),
            origin: Origin::SourceObservation,
            mode: ExtractionMode::NativeTraversal,
            fidelity: Fidelity::NativeStructural,
        };
        let atom = EvaluationAtom {
            evaluation: read.id(),
            context: q.context,
            predicate: Predicate::Truthy.id(),
            operand: Some(self.use_.place),
        };
        let test = FlowTestObservation {
            qualification: q.id(),
            test: read.id(),
            scope: self.base.function_scope.scope,
        };
        let leaf = FlowTestLeafObservation {
            qualification: q.id(),
            test: read.id(),
            atom: atom.id(),
            operand: Some(read.id()),
        };
        let load = FlowAttributeLoadObservation {
            qualification: q.id(),
            occurrence: call.id(),
            name: "x".into(),
        };
        let value = FlowValueObservation {
            qualification: q.id(),
            use_: self.use_.id(),
            sink: call.id(),
            kind: FlowSinkKind::Return,
            transfer: TransferKind::Derived,
            through_call: true,
        };
        let (path, steps) =
            FlowCallPath::new(&[(call.id(), read.id(), FlowCallOperandRole::Argument)]).unwrap();
        let link = FlowValuePathObservation {
            qualification: q.id(),
            value: value.id(),
            path: path.id(),
        };
        macro_rules! append {
            ($ty:ty,$rows:expr) => {
                let mut rows = self.base.rows::<$ty>();
                rows.extend($rows);
                self.base.put(rows);
            };
        }
        append!(Occurrence, vec![call, callee]);
        append!(Evidence, vec![evidence.clone()]);
        append!(ProviderRun, vec![syntax_run]);
        append!(RunFamily, families);
        append!(ProviderSurface, vec![syntax_surface]);
        append!(
            ProviderCoverage,
            vec![ProviderCoverage {
                scope: q.scope,
                provider: Some(run.provider),
                context: run.context,
                family: FactFamily::Syntax,
                run: Some(syntax_support.run),
                status: CoverageStatus::CompleteUnderStatedModel,
                reason: None,
                diagnostic: None
            }]
        );
        self.base.put(vec![syntax]);
        self.base.put(arguments);
        self.base.put(vec![syntax_support]);
        self.base.put(vec![Predicate::Truthy]);
        self.base.put(vec![atom]);
        self.base.put(vec![path]);
        self.base.put(steps);
        macro_rules! supported {
            ($ty:ident,$support:ident,$row:expr) => {{
                let row = $row;
                self.base.put(vec![$support {
                    assertion: row.id(),
                    run: run.id(),
                    surface: surface.id(),
                    evidence: evidence.id(),
                    origin: Origin::AnalyzerAssertion,
                    mode: ExtractionMode::NativeTraversal,
                    fidelity: Fidelity::NativeStructural,
                }]);
                self.base.put::<$ty>(vec![row]);
            }};
        }
        supported!(FlowTestObservation, FlowTestSupport, test);
        supported!(FlowTestLeafObservation, FlowTestLeafSupport, leaf);
        supported!(FlowAttributeLoadObservation, FlowAttributeLoadSupport, load);
        supported!(FlowValuePathObservation, FlowValuePathSupport, link);
        // Keep the existing direct value alongside the new through-call alternative.
        append!(
            FlowValueSupport,
            vec![FlowValueSupport {
                assertion: value.id(),
                run: run.id(),
                surface: surface.id(),
                evidence: evidence.id(),
                origin: Origin::AnalyzerAssertion,
                mode: ExtractionMode::NativeTraversal,
                fidelity: Fidelity::NativeStructural
            }]
        );
        append!(FlowValueObservation, vec![value]);
    }
}
impl Fixture {
    pub fn nested_path(&mut self) {
        use lctx_model::domain::calls::*;
        self.extended();
        let inner = self.base.rows::<CallSyntax>()[0].clone();
        let occurrences = self.base.rows::<Occurrence>();
        let read = occurrences
            .iter()
            .find(|o| o.id() == self.use_.occurrence)
            .unwrap()
            .clone();
        let inner_occ = occurrences
            .iter()
            .find(|o| o.id() == inner.site)
            .unwrap()
            .clone();
        let outer = Occurrence {
            start: inner_occ.start - 2,
            end: inner_occ.end,
            structural_path: vec![0, 1],
            ..inner_occ.clone()
        };
        let callee = Occurrence {
            start: outer.start,
            end: outer.start + 1,
            structural_path: vec![0, 1, 1],
            ..read.clone()
        };
        let (syntax, arguments) = CallSyntax::new(
            inner.qualification,
            outer.id(),
            callee.id(),
            false,
            &[Actual {
                occurrence: inner.site,
                kind: ArgumentKind::Positional,
                keyword: None,
            }],
        )
        .unwrap();
        let value = FlowValueObservation {
            sink: outer.id(),
            ..self
                .base
                .rows::<FlowValueObservation>()
                .into_iter()
                .find(|v| v.through_call)
                .unwrap()
        };
        let (path, steps) = FlowCallPath::new(&[
            (outer.id(), inner.site, FlowCallOperandRole::Argument),
            (inner.site, read.id(), FlowCallOperandRole::Argument),
        ])
        .unwrap();
        let link = FlowValuePathObservation {
            qualification: value.qualification,
            value: value.id(),
            path: path.id(),
        };
        macro_rules! append {
            ($ty:ty,$rows:expr) => {
                let mut rows = self.base.rows::<$ty>();
                rows.extend($rows);
                self.base.put(rows);
            };
        }
        let evidence = Evidence::Occurrence {
            occurrence: outer.id(),
        };
        let syntax_support = CallSyntaxSupport {
            assertion: syntax.id(),
            evidence: evidence.id(),
            ..self.base.rows::<CallSyntaxSupport>()[0].clone()
        };
        let mut value_support = self.base.rows::<FlowValueSupport>();
        let old = self
            .base
            .rows::<FlowValueObservation>()
            .into_iter()
            .find(|v| v.through_call)
            .unwrap();
        let support = value_support
            .iter_mut()
            .find(|s| s.assertion == old.id())
            .unwrap();
        support.assertion = value.id();
        support.evidence = evidence.id();
        self.base.put(value_support);
        let link_support = FlowValuePathSupport {
            assertion: link.id(),
            evidence: evidence.id(),
            ..self.base.rows::<FlowValuePathSupport>()[0].clone()
        };
        self.base.put(vec![link_support]);
        append!(Occurrence, vec![outer, callee]);
        append!(CallSyntax, vec![syntax]);
        append!(CallArgument, arguments);
        append!(CallSyntaxSupport, vec![syntax_support]);
        append!(Evidence, vec![evidence]);
        let mut values = self.base.rows::<FlowValueObservation>();
        values.retain(|v| !v.through_call);
        values.push(value);
        self.base.put(values);
        self.base.put(vec![path]);
        self.base.put(steps);
        self.base.put(vec![link]);
    }
}
