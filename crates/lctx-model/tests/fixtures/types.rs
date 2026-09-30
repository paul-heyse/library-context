#[allow(dead_code)] #[path = "lexical.rs"] mod lexical_fixture;
use lctx_model::domain::{*,assertion::*,attribution::*,calls::*,input::InputRevision,types::*,value::Literal};

pub struct Fixture { pub base: lexical_fixture::Fixture, pub term: TypeTerm, pub variable: TypeVariable }
impl Fixture {
    pub fn new(foreign_variable: bool) -> Self {
        let mut base = lexical_fixture::Fixture::new();
        let context = base.rows::<AnalysisContext>()[0].clone(); let input = base.rows::<InputRevision>()[0].id();
        let q = base.rows::<AssertionQualification>()[0].clone();
        let provider = Provider { tool: "type-contract".into(),revision: "1".into(),build_digest: ContentHash::of(b"types") };
        let alien = Provider { tool: "other-type-provider".into(),..provider.clone() };
        let (run,families) = ProviderRun::new(provider.id(),context.id(),input,context.config_digest,[FactFamily::Types]).unwrap();
        let surface = ProviderSurface { provider: provider.id(),family: FactFamily::Types,name: "native types".into() };
        let module = ProviderModule::Bundled { provider: provider.id(), bundle: ModuleBundle::Typeshed, name: "example".into() };
        let variable_provider = if foreign_variable { alien.id() } else { provider.id() };
        // The variable's module belongs to the variable's own provider, isolating the ownership refusal.
        let variable_module = ProviderModule::Bundled { provider: variable_provider, bundle: ModuleBundle::Typeshed, name: "example".into() };
        let class = ProviderSymbol { provider: provider.id(),context: context.id(),module: module.id(),native_key: "Container".into(),name: "Container".into(),kind: SymbolKind::Class };
        let variable = TypeVariable { provider: variable_provider,context: context.id(),module: variable_module.id(),anchor_start: 6,anchor_end: 12,
            slot: 0,origin: TypeVariableOrigin::Pep695,kind: TypeVariableKind::TypeVar,name: "T".into() };
        let variable_term = TypeTerm::TypeVar { variable: variable.id() };
        let integer = Literal::Integer { decimal: "99999999999999999999999999999999999999999999999999999999".into() };
        let bytes = Literal::Bytes { value: EvidenceBytes(vec![0,255,1,128]) };
        let integer_term = TypeTerm::Literal { value: integer.id() }; let bytes_term = TypeTerm::Literal { value: bytes.id() };
        let (arguments,mut members) = TypeSequence::new(&[(TypeChildRole::Argument,variable_term.id())]).unwrap();
        let instance = TypeTerm::ClassInstance { class: class.id(),arguments: arguments.id() };
        let (union,union_members) = TypeSequence::new(&[(TypeChildRole::Member,integer_term.id()),(TypeChildRole::Member,bytes_term.id()),(TypeChildRole::Member,instance.id())]).unwrap();
        members.extend(union_members);
        let term = TypeTerm::Union { members: union.id() };
        let function = base.rows::<lctx_model::domain::lexical::LexicalScope>().into_iter().find(|s| s.id() == base.function_scope.scope).unwrap().owner;
        let observation = TypeObservation { qualification: q.id(),subject: function,role: TypeRole::Return,declared: true,term: term.id() };
        let presentation = TypePresentation { qualification: q.id(),scope: q.scope,term: term.id(),display: "Literal[large_integer, bytes] | Container[T]".into(),detail: None };
        let alternative = TypePresentation { display: "Union[Container[T], literal values]".into(),..presentation.clone() };
        let restriction = TypeVariableRestriction { qualification: q.id(),scope: q.scope,variable: variable.id(),kind: TypeRestrictionKind::Bound,ordinal: 0,term: instance.id() };
        let evidence = Evidence::Occurrence { occurrence: function };
        macro_rules! support { ($ty:ident,$row:expr) => { $ty { assertion: $row.id(),run: run.id(),surface: surface.id(),evidence: evidence.id(),origin: Origin::AnalyzerAssertion,
            mode: ExtractionMode::NativeTraversal,fidelity: Fidelity::NativeStructural } }; }
        macro_rules! append { ($ty:ty,$rows:expr) => { let mut rows = base.rows::<$ty>(); rows.extend($rows); base.put(rows); }; }
        append!(Provider,vec![provider.clone(),alien]); append!(ProviderRun,vec![run.clone()]); append!(RunFamily,families); append!(ProviderSurface,vec![surface.clone()]);
        macro_rules! one { ($($row:expr),+ $(,)?) => { $(base.put(vec![$row.clone()]);)+ }; }
        base.put(vec![module,variable_module]);
        one!(class,variable,observation,support!(TypeSupport,observation),restriction,support!(TypeRestrictionSupport,restriction));
        base.put(vec![presentation.clone(),alternative.clone()]); base.put(vec![support!(TypePresentationSupport,presentation),support!(TypePresentationSupport,alternative)]);
        base.put(vec![integer,bytes]); base.put(vec![variable_term,integer_term,bytes_term,instance,term.clone()]); base.put(vec![arguments,union]);
        let mut fixture = Self { base,term,variable }; fixture.put_members(members); fixture
    }
    pub fn put_members(&mut self,mut rows: Vec<TypeSequenceMember>) {
        rows.sort_by_key(|r| (r.sequence,r.ordinal));
        self.base.batches.insert(TypeSequenceMember::NAME,TypeSequenceMember::encode(&rows).unwrap());
    }
}

impl Fixture {
    /// Change only the occurrence type claim: presentation/restriction claims remain independent.
    pub fn opaque(&mut self, truncated: bool, nested: bool, fidelity: Fidelity) {
        let mut supports = self.base.rows::<TypeSupport>();
        let run = self.base.rows::<ProviderRun>().into_iter().find(|r| r.id() == supports[0].run).unwrap();
        let opaque = if truncated {
            TypeTerm::Truncated { provider: run.provider,context: run.context,reason: ObligationKind::BudgetReached,display: "Unexpanded[T]".into() }
        } else {
            TypeTerm::Other { provider: run.provider,context: run.context,variant: "future_native_form".into(),display: "Unmodeled[T]".into() }
        };
        let mut terms = self.base.rows::<TypeTerm>(); terms.push(opaque.clone());
        self.term = if nested { TypeTerm::TypeOf { target: opaque.id() } } else { opaque };
        terms.push(self.term.clone()); self.base.put(terms);
        let mut observations = self.base.rows::<TypeObservation>(); observations[0].term = self.term.id();
        supports[0].assertion = observations[0].id(); supports[0].fidelity = fidelity;
        self.base.put(observations); self.base.put(supports);
    }
}
