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
        rows.sort_by_key(|r| (r.sequence,r.ordinal)); rows.dedup();
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

/// Every structural form beyond the representative fixture, by name, each with a structural
/// presentation so its closure is checked.
#[allow(dead_code)]
pub struct Vocabulary { pub terms: std::collections::BTreeMap<&'static str, TypeTerm> }
impl Fixture {
    fn append<R: Record>(&mut self, rows: Vec<R>) { let mut all = self.base.rows::<R>(); all.extend(rows); self.base.put(all); }
    pub fn put_slots(&mut self, mut rows: Vec<CallableParameter>) {
        rows.sort_by_key(|r| (r.list,r.ordinal)); rows.dedup();
        self.base.batches.insert(CallableParameter::NAME,CallableParameter::encode(&rows).unwrap());
    }
    /// A structural presentation of each term, supported like the fixture's own.
    pub fn present(&mut self, terms: &[&TypeTerm], fidelity: Fidelity) {
        let base = self.base.rows::<TypePresentation>()[0].clone();
        let support = self.base.rows::<TypePresentationSupport>()[0].clone();
        let presentations: Vec<_> = terms.iter().enumerate().map(|(n,t)| TypePresentation { term: t.id(),display: format!("form{n}-{:?}",t.id()),..base.clone() }).collect();
        let supports: Vec<_> = presentations.iter().map(|p| TypePresentationSupport { assertion: p.id(),fidelity,..support.clone() }).collect();
        self.append(presentations); self.append(supports);
    }
    pub fn vocabulary(&mut self) -> Vocabulary {
        use lctx_model::domain::calls::ParameterKind as K;
        let variable = self.variable.clone();
        let param_spec = TypeVariable { slot: 1,kind: TypeVariableKind::ParamSpec,name: "P".into(),..variable.clone() };
        let class = self.base.rows::<ProviderSymbol>().into_iter().find(|s| s.kind == SymbolKind::Class).unwrap();
        let module = self.base.rows::<ProviderModule>()[0].clone();
        let terms = self.base.rows::<TypeTerm>();
        let instance = terms.iter().find(|t| matches!(t,TypeTerm::ClassInstance { .. })).unwrap().clone();
        let integer = terms.iter().find(|t| matches!(t,TypeTerm::Literal { .. })).unwrap().clone();
        let t = TypeTerm::TypeVar { variable: variable.id() };
        let p = TypeTerm::ParamSpec { variable: param_spec.id() };
        let slot = |name: Option<&str>, kind: K, required: Option<bool>, term: &TypeTerm| Slot { name: name.map(str::to_owned),kind,required,term: term.id() };
        let (params,mut slots) = CallableParameterList::new(&[slot(Some("x"),K::PositionalOrKeyword,Some(true),&instance),slot(Some("kw"),K::VarKeyword,None,&integer)]).unwrap();
        let (prefix,prefix_slots) = CallableParameterList::new(&[slot(None,K::PositionalOnly,Some(true),&integer)]).unwrap();
        let (fields,field_slots) = CallableParameterList::new(&[slot(Some("a"),K::KeywordOnly,Some(false),&integer)]).unwrap();
        let (empty,_) = CallableParameterList::new(&[]).unwrap();
        slots.extend(prefix_slots); slots.extend(field_slots);
        let none = TypeTerm::None;
        let callable = TypeTerm::Callable { function: Some("f".into()),form: CallableForm::List,parameters: params.id(),param_spec: None,returns: none.id() };
        let variadic = TypeTerm::Callable { function: None,form: CallableForm::Ellipsis,parameters: empty.id(),param_spec: None,returns: instance.id() };
        let forwarding = TypeTerm::Callable { function: None,form: CallableForm::ParamSpec,parameters: prefix.id(),param_spec: Some(p.id()),returns: none.id() };
        let (signatures,mut members) = TypeSequence::new(&[(TypeChildRole::Signature,callable.id()),(TypeChildRole::Signature,variadic.id())]).unwrap();
        let (tparams,tparam_members) = TypeSequence::new(&[(TypeChildRole::TypeParameter,t.id())]).unwrap();
        let (no_arguments,_) = TypeSequence::new(&[]).unwrap();
        let (arguments,argument_members) = TypeSequence::new(&[(TypeChildRole::Argument,t.id())]).unwrap();
        members.extend(tparam_members); members.extend(argument_members);
        let overload = TypeTerm::Overload { function: "f".into(),signatures: signatures.id() };
        let map = std::collections::BTreeMap::from([
            ("callable",callable.clone()),("variadic",variadic),("forwarding",forwarding),("overload",overload.clone()),
            ("generic",TypeTerm::Generic { parameters: tparams.id(),body: callable.clone().id() }),
            ("bound",TypeTerm::BoundMethod { receiver: instance.id(),function: overload.id() }),
            ("typed_dict",TypeTerm::TypedDict { class: class.id(),arguments: no_arguments.id(),partial: false }),
            ("anonymous",TypeTerm::AnonymousTypedDict { fields: fields.id(),partial: true }),
            ("module",TypeTerm::Module { module: module.id() }),
            ("alias",TypeTerm::TypeAlias { name: "Alias".into(),untyped: false,target: instance.id() }),
            ("alias_ref",TypeTerm::TypeAliasReference { module: module.id(),name: "Alias".into(),untyped: false,arguments: arguments.id() }),
            ("self",TypeTerm::SelfType { class: class.id(),arguments: arguments.id() }),
            ("annotated",TypeTerm::Annotated { target: instance.id() }),("unpack",TypeTerm::Unpack { target: instance.id() }),
            ("type_is",TypeTerm::TypeGuard { form: GuardForm::TypeIs,target: instance.id() }),
            ("type_guard",TypeTerm::TypeGuard { form: GuardForm::TypeGuard,target: instance.id() }),
            ("type_form",TypeTerm::TypeForm { target: instance.id() }),
            ("concatenate",TypeTerm::ParamList { parameters: prefix.id(),param_spec: Some(p.id()) }),
            ("special",TypeTerm::SpecialForm { form: "Literal".into() }),
            ("args",TypeTerm::VariableForm { variable: param_spec.id(),form: VariableFormKind::Args }),
            ("kwargs",TypeTerm::VariableForm { variable: param_spec.id(),form: VariableFormKind::Kwargs }),
            ("enum",TypeTerm::EnumLiteral { class: class.id(),member: "RED".into() }),
            ("literal_string",TypeTerm::LiteralString),("none",none),("p",p),("t",t),
        ]);
        self.append(vec![param_spec.clone()]);
        self.append(map.values().cloned().collect::<Vec<_>>());
        self.append(vec![signatures,tparams,no_arguments,arguments]);
        let mut all = self.base.rows::<TypeSequenceMember>(); all.extend(members); self.put_members(all);
        self.append(vec![params,prefix,fields,empty]); self.put_slots(slots);
        let presented: Vec<TypeTerm> = map.values().cloned().collect();
        self.present(&presented.iter().collect::<Vec<_>>(),Fidelity::NativeStructural);
        Vocabulary { terms: map }
    }
}

impl Fixture {
    /// A dataclass's two fields on the fixture's class, and the body of the fixture's `def`, each
    /// supported like the fixture's type observation.
    pub fn records(&mut self) -> (Vec<RecordFieldObservation>, FunctionBodyObservation) {
        let class = self.base.rows::<ProviderSymbol>().into_iter().find(|s| s.kind == SymbolKind::Class).unwrap();
        let observation = self.base.rows::<TypeObservation>()[0].clone();
        let field = |name: &str, ordinal: i64| RecordFieldObservation { qualification: observation.qualification, class: class.id(), name: name.into(),
            record: RecordKind::Dataclass, ordinal, term: observation.term, declared: true, declaration: None, has_default: Some(false), init: Some(true),
            alias: None, kw_only: None, required: None, read_only: None };
        let fields = vec![field("x", 0), RecordFieldObservation { has_default: Some(true), alias: Some("why".into()), kw_only: Some(true), ..field("y", 1) }];
        let body = FunctionBodyObservation { qualification: observation.qualification, declaration: observation.subject, body: FunctionBodyKind::Ellipsis,
            abstract_method: true, in_protocol_class: false, in_type_checking_block: false, overload: false };
        self.set_records(fields.clone(), body.clone());
        (fields, body)
    }
    pub fn set_records(&mut self, fields: Vec<RecordFieldObservation>, body: FunctionBodyObservation) {
        let support = self.base.rows::<TypeSupport>()[0].clone();
        let field_supports: Vec<_> = fields.iter().map(|f| RecordFieldSupport { assertion: f.id(), run: support.run, surface: support.surface, evidence: support.evidence,
            origin: support.origin, mode: support.mode, fidelity: support.fidelity }).collect();
        let body_support = FunctionBodySupport { assertion: body.id(), run: support.run, surface: support.surface, evidence: support.evidence,
            origin: support.origin, mode: support.mode, fidelity: support.fidelity };
        self.base.put(fields); self.base.put(field_supports); self.base.put(vec![body]); self.base.put(vec![body_support]);
    }
}
