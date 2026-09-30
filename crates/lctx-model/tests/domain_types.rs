#[path = "fixtures/types.rs"] mod fixture;
use fixture::Fixture;
use lctx_model::domain::{*,attribution::ObligationKind,calls::ProviderModule,types::*,value::Literal};
use fixture::Vocabulary;

#[test]
fn type_structure_keeps_large_literals_variable_identity_and_recursive_restrictions() {
    let f = Fixture::new(false);
    for name in ["type_sequence_membership","structural_type_shapes",TypeSupport::NAME,TypePresentationSupport::NAME,TypeRestrictionSupport::NAME] {
        f.base.check(f.base.model.invariants().iter().find(|i| i.name == name).unwrap()).unwrap();
    }
    let views = f.base.rows::<TypePresentation>(); assert_eq!(views.len(),2);
    assert!(views.iter().all(|v| v.term == f.term.id())); assert_ne!(views[0].id(),views[1].id());
    let mut renamed = f.variable.clone(); renamed.module = ProviderModule::Bundled { provider: f.variable.provider, bundle: lctx_model::domain::calls::ModuleBundle::Typeshed, name: "unrelated".into() }.id(); assert_ne!(renamed.id(),f.variable.id());
    let literal = f.base.rows::<Literal>().into_iter().find(|v| matches!(v,Literal::Integer { .. })).unwrap();
    assert_eq!(Literal::decode(Batch::new(&f.base.model,vec![literal.clone()], &budget()).unwrap().arrow()).unwrap(),vec![literal]);
    let restriction = f.base.rows::<TypeVariableRestriction>()[0].clone();
    let instance = f.base.rows::<TypeTerm>().into_iter().find(|t| t.id() == restriction.term).unwrap();
    assert!(matches!(instance,TypeTerm::ClassInstance { .. }),"recursive bound is a relationship, not key expansion");
}
#[test]
fn a_nested_type_variable_cannot_switch_provider_namespace() {
    let f = Fixture::new(true);
    let error = f.base.check(&TypeSupport::invariants()[0]).unwrap_err();
    assert!(error.to_string().contains("type variable belongs"));
    assert!(f.base.check(&TypeRestrictionSupport::invariants()[0]).is_err());
}
#[test]
fn type_sequences_require_complete_ordered_membership_and_kind_correct_children() {
    let mut f = Fixture::new(false);
    let mut members = f.base.rows::<TypeSequenceMember>(); members.pop(); f.put_members(members);
    assert!(f.base.check(&TypeSequence::invariants()[0]).is_err());
    let mut f = Fixture::new(false);
    let mut members = f.base.rows::<TypeSequenceMember>(); members[0].ordinal += 1; f.put_members(members);
    assert!(f.base.check(&TypeSequence::invariants()[0]).is_err());
    let mut f = Fixture::new(false);
    let wrong = TypeTerm::ParamSpec { variable: f.variable.id() }; let mut terms = f.base.rows::<TypeTerm>(); terms.push(wrong); f.base.put(terms);
    assert!(f.base.check(&TypeTerm::invariants()[0]).is_err());
}

#[test]
fn opaque_type_leaves_cannot_be_promoted_to_structural_fidelity() {
    use lctx_model::domain::attribution::Fidelity;
    for truncated in [false,true] { for nested in [false,true] {
        for fidelity in [Fidelity::Raw,Fidelity::NativeStructural,Fidelity::NormalizedStructural,Fidelity::ReportProjection,Fidelity::DisplayOnly] {
            let mut f = Fixture::new(false); f.opaque(truncated,nested,fidelity);
            let result = f.base.check(&TypeSupport::invariants()[0]);
            assert_eq!(result.is_ok(),fidelity == Fidelity::DisplayOnly,"{truncated}/{nested}/{fidelity:?}: {result:?}");
        }
    } }
}

/// A fresh attempt budget; these controls do not share reservations across batches.
fn budget() -> lctx_model::domain::resources::ResourceBudget {
    lctx_model::domain::resources::ResourceBudget::fixed(lctx_model::domain::resources::DEFAULT_MEMORY_BYTES).unwrap()
}

/// Supports over one shared wide term walk it once per provider key: 600 presentations of a
/// 2,000-member union were about 1.2M cumulative closure steps, which a global cap refused as an
/// invalid model (resource review F03).
#[test]
fn shared_type_closures_are_verified_once() {
    use lctx_model::domain::attribution::Fidelity;
    let mut f = Fixture::new(false);
    let literals: Vec<_> = (0..2000).map(|n| Literal::Integer { decimal: n.to_string() }).collect();
    let terms: Vec<_> = literals.iter().map(|l| TypeTerm::Literal { value: l.id() }).collect();
    let (union, members) = TypeSequence::new(&terms.iter().map(|t| (TypeChildRole::Member, t.id())).collect::<Vec<_>>()).unwrap();
    let wide = TypeTerm::Union { members: union.id() };
    let base = f.base.rows::<TypePresentation>()[0].clone();
    let support = f.base.rows::<TypePresentationSupport>()[0].clone();
    let presentations: Vec<_> = (0..600).map(|n| TypePresentation { term: wide.id(), display: format!("wide{n}"), ..base.clone() }).collect();
    let supports: Vec<_> = presentations.iter().map(|p| TypePresentationSupport { assertion: p.id(), ..support.clone() }).collect();
    macro_rules! append { ($ty:ty, $rows:expr) => { let mut rows = f.base.rows::<$ty>(); rows.extend($rows); f.base.put(rows); }; }
    append!(Literal, literals); append!(TypeTerm, terms.into_iter().chain([wide])); append!(TypeSequence, vec![union]);
    append!(TypePresentation, presentations); append!(TypePresentationSupport, supports);
    let mut all = f.base.rows::<TypeSequenceMember>(); all.extend(members); f.put_members(all);
    f.base.check(&TypePresentationSupport::invariants()[0]).unwrap();
    // The memo is keyed by fidelity: an opaque term verified for display-only support still
    // refuses a structural support of the same term.
    let mut g = Fixture::new(false); g.opaque(false, true, Fidelity::DisplayOnly);
    g.base.check(&TypeSupport::invariants()[0]).unwrap();
    let observation = TypeObservation { role: TypeRole::CallResult, ..g.base.rows::<TypeObservation>()[0].clone() };
    let structural = TypeSupport { assertion: observation.id(), fidelity: Fidelity::NativeStructural, ..g.base.rows::<TypeSupport>()[0].clone() };
    let mut observations = g.base.rows::<TypeObservation>(); observations.push(observation); g.base.put(observations);
    let mut supports = g.base.rows::<TypeSupport>(); supports.push(structural); g.base.put(supports);
    let error = g.base.check(&TypeSupport::invariants()[0]).unwrap_err();
    assert!(error.to_string().contains("display-only"), "{error}");
}

fn refused(result: Result<(), ModelError>, why: &str, expected: &str) {
    assert!(matches!(&result, Err(ModelError::Invalid(message)) if message.contains(expected)), "{why}: expected `{expected}`, got {result:?}");
}
fn shapes(f: &Fixture) -> Result<(), ModelError> {
    for invariant in [&TypeTerm::invariants()[0], &TypeSequence::invariants()[0], &CallableParameterList::invariants()[0], &TypePresentationSupport::invariants()[0]] {
        f.base.check(invariant)?;
    }
    Ok(())
}

#[test]
fn every_native_form_validates_and_keeps_its_distinctions() {
    let mut f = Fixture::new(false);
    let v = f.vocabulary();
    shapes(&f).unwrap();
    // Forms the old kinds told apart only by display text are distinct terms.
    let id = |name: &str| v.terms[name].id();
    assert_ne!(id("args"), id("kwargs")); assert_ne!(id("args"), id("p"));
    assert_ne!(id("type_is"), id("type_guard"));
    assert_ne!(id("type_form"), TypeTerm::TypeOf { target: match &v.terms["type_form"] { TypeTerm::TypeForm { target } => *target, _ => unreachable!() } }.id());
    assert_ne!(id("alias"), id("alias_ref"));
    assert_ne!(id("typed_dict"), id("anonymous"));
    let renamed = match v.terms["callable"].clone() { TypeTerm::Callable { form, parameters, param_spec, returns, .. } =>
        TypeTerm::Callable { function: Some("g".into()), form, parameters, param_spec, returns }, _ => unreachable!() };
    assert_ne!(id("callable"), renamed.id(), "a def's type names its function");
    // A test operand is a type role of its own.
    let observation = TypeObservation { role: TypeRole::TestOperand, term: id("literal_string"), ..f.base.rows::<TypeObservation>()[0].clone() };
    let support = TypeSupport { assertion: observation.id(), ..f.base.rows::<TypeSupport>()[0].clone() };
    let mut observations = f.base.rows::<TypeObservation>(); observations.push(observation); f.base.put(observations);
    let mut supports = f.base.rows::<TypeSupport>(); supports.push(support); f.base.put(supports);
    f.base.check(&TypeSupport::invariants()[0]).unwrap();
}

#[test]
fn nested_truncated_structure_is_still_display_only() {
    use lctx_model::domain::{attribution::{Fidelity, Provider, ProviderRun}, calls::ParameterKind};
    for fidelity in [Fidelity::NativeStructural, Fidelity::DisplayOnly] {
        let mut f = Fixture::new(false);
        f.vocabulary();
        let run = f.base.rows::<ProviderRun>().into_iter().find(|r| r.id() == f.base.rows::<TypeSupport>()[0].run).unwrap();
        let _ = Provider::NAME;
        let cut = TypeTerm::Truncated { provider: run.provider, context: run.context, reason: ObligationKind::BudgetReached, display: "Deep[...]".into() };
        // Truncation two levels down: a callable's parameter, inside an overload, inside a generic.
        let (list, slots) = CallableParameterList::new(&[Slot { name: Some("deep".into()), kind: ParameterKind::PositionalOrKeyword, required: Some(true), term: cut.id() }]).unwrap();
        let callable = TypeTerm::Callable { function: None, form: CallableForm::List, parameters: list.id(), param_spec: None, returns: TypeTerm::None.id() };
        let (signatures, members) = TypeSequence::new(&[(TypeChildRole::Signature, callable.id())]).unwrap();
        let overload = TypeTerm::Overload { function: "deep".into(), signatures: signatures.id() };
        let bound = TypeTerm::BoundMethod { receiver: TypeTerm::None.id(), function: overload.id() };
        let mut terms = f.base.rows::<TypeTerm>(); terms.extend([cut, callable, overload, bound.clone()]); f.base.put(terms);
        let mut sequences = f.base.rows::<TypeSequence>(); sequences.push(signatures); f.base.put(sequences);
        let mut all = f.base.rows::<TypeSequenceMember>(); all.extend(members); f.put_members(all);
        let mut lists = f.base.rows::<CallableParameterList>(); lists.push(list); f.base.put(lists);
        let mut all = f.base.rows::<CallableParameter>(); all.extend(slots); f.put_slots(all);
        f.present(&[&bound], fidelity);
        let result = f.base.check(&TypePresentationSupport::invariants()[0]);
        if fidelity == Fidelity::DisplayOnly { result.expect("display-only support of truncated structure"); }
        else { refused(result, "structural support of a truncated closure", "display-only"); }
    }
}

#[test]
fn every_form_keeps_its_shape() {
    use lctx_model::domain::calls::ParameterKind;
    let mutate = |name: &str, change: &dyn Fn(&Vocabulary, &TypeTerm) -> TypeTerm, expected: &str| {
        let mut f = Fixture::new(false);
        let v = f.vocabulary();
        let changed = change(&v, &v.terms[name]);
        let mut terms = f.base.rows::<TypeTerm>(); terms.push(changed); f.base.put(terms);
        refused(shapes(&f), name, expected);
    };
    mutate("overload", &|v, _| TypeTerm::Overload { function: "f".into(), signatures: match &v.terms["generic"] { TypeTerm::Generic { parameters, .. } => *parameters, _ => unreachable!() } },
        "other role");
    mutate("generic", &|v, t| match t { TypeTerm::Generic { parameters, .. } => TypeTerm::Generic { parameters: *parameters, body: v.terms["none"].id() }, _ => unreachable!() },
        "a generic binds type variables");
    mutate("variadic", &|v, t| match t { TypeTerm::Callable { returns, .. } => TypeTerm::Callable { function: None, form: CallableForm::Ellipsis,
        parameters: match &v.terms["callable"] { TypeTerm::Callable { parameters, .. } => *parameters, _ => unreachable!() }, param_spec: None, returns: *returns }, _ => unreachable!() },
        "differ from its form");
    mutate("forwarding", &|_, t| match t { TypeTerm::Callable { parameters, returns, .. } => TypeTerm::Callable { function: None, form: CallableForm::ParamSpec,
        parameters: *parameters, param_spec: None, returns: *returns }, _ => unreachable!() }, "differ from its form");
    mutate("anonymous", &|v, _| TypeTerm::AnonymousTypedDict { fields: match &v.terms["concatenate"] { TypeTerm::ParamList { parameters, .. } => *parameters, _ => unreachable!() },
        partial: false }, "named keyword-only slots");
    mutate("args", &|_, _| TypeTerm::VariableForm { variable: Fixture::new(false).variable.id(), form: VariableFormKind::Args }, "needs its variable's kind");
    mutate("bound", &|v, _| TypeTerm::BoundMethod { receiver: v.terms["none"].id(), function: v.terms["none"].id() }, "binds a callable");
    // Row contracts.
    assert!(TypeTerm::SpecialForm { form: String::new() }.validate().is_err());
    assert!(TypeTerm::Callable { function: Some(String::new()), form: CallableForm::Ellipsis, parameters: CallableParameterList::new(&[]).unwrap().0.id(),
        param_spec: None, returns: TypeTerm::None.id() }.validate().is_err());
    assert!(CallableParameterList::new(&[Slot { name: Some("args".into()), kind: ParameterKind::VarPositional, required: Some(false), term: TypeTerm::None.id() }]).is_err());
    assert!(CallableParameterList::new(&[Slot { name: None, kind: ParameterKind::KeywordOnly, required: Some(true), term: TypeTerm::None.id() }]).is_err());
    // A list missing a member.
    let mut f = Fixture::new(false); f.vocabulary();
    let mut slots = f.base.rows::<CallableParameter>(); slots.pop(); f.put_slots(slots);
    refused(shapes(&f), "a parameter list missing a slot", "callable parameter");
}

#[test]
fn a_module_type_is_its_providers() {
    use lctx_model::domain::{attribution::Provider, calls::ModuleBundle};
    let mut f = Fixture::new(false); let v = f.vocabulary();
    let alien = f.base.rows::<Provider>().into_iter().find(|p| p.tool == "other-type-provider").unwrap();
    let foreign = ProviderModule::Bundled { provider: alien.id(), bundle: ModuleBundle::Typeshed, name: "os".into() };
    let module = TypeTerm::Module { module: foreign.id() };
    let mut modules = f.base.rows::<ProviderModule>(); modules.push(foreign); f.base.put(modules);
    let mut terms = f.base.rows::<TypeTerm>(); terms.push(module.clone()); f.base.put(terms);
    f.present(&[&module], lctx_model::domain::attribution::Fidelity::NativeStructural);
    refused(f.base.check(&TypePresentationSupport::invariants()[0]), "a module another provider bundles", "type module belongs to another provider");
    assert!(v.terms.contains_key("module"));
}

#[test]
fn record_fields_state_their_models_flags_in_field_order() {
    let mut f = Fixture::new(false);
    let (fields, _) = f.records();
    let stored = |f: &Fixture| -> Result<(), ModelError> {
        for invariant in [&RecordFieldObservation::invariants()[0], &RecordFieldSupport::invariants()[0], &FunctionBodyObservation::invariants()[0], &FunctionBodySupport::invariants()[0]] {
            f.base.check(invariant)?;
        }
        Ok(())
    };
    stored(&f).unwrap();
    // Each record model states exactly its own flags.
    let dataclass = fields[1].clone();
    for (row, why) in [
        (RecordFieldObservation { record: RecordKind::TypedDict, ..dataclass.clone() }, "a typed-dict field with dataclass flags"),
        (RecordFieldObservation { init: None, ..dataclass.clone() }, "a dataclass field without init"),
        (RecordFieldObservation { record: RecordKind::NamedTuple, init: None, ..dataclass.clone() }, "a named-tuple field with an alias"),
        (RecordFieldObservation { record: RecordKind::TypedDict, has_default: None, init: None, alias: None, kw_only: None, required: Some(true), ..dataclass.clone() },
            "a typed-dict field without read-only"),
    ] { assert!(matches!(row.validate(), Err(ModelError::Invalid(ref m)) if m.contains("exactly the flags")), "{why}"); }
    RecordFieldObservation { record: RecordKind::TypedDict, has_default: None, init: None, alias: None, kw_only: None, required: Some(false), read_only: Some(true),
        ..dataclass.clone() }.validate().expect("a typed-dict field's own flags");
    RecordFieldObservation { record: RecordKind::NamedTuple, init: None, alias: None, kw_only: None, ..dataclass.clone() }.validate().expect("a named-tuple field's default");
    // Field order is 0..n, and one class has one record model.
    let mut gap = Fixture::new(false); let (mut rows, body) = gap.records(); rows[1].ordinal = 2; gap.set_records(rows, body);
    refused(stored(&gap), "fields at ordinals 0 and 2", "ordinals 0..n");
    let mut mixed = Fixture::new(false); let (mut rows, body) = mixed.records();
    rows[1] = RecordFieldObservation { record: RecordKind::Attrs, ..rows[1].clone() }; mixed.set_records(rows, body);
    refused(stored(&mixed), "a dataclass field beside an attrs field", "one record model");
    // A function body belongs to a def.
    let mut module = Fixture::new(false); let (rows, body) = module.records();
    let root = module.base.rows::<lctx_model::domain::source::Occurrence>().into_iter().find(|o| o.syntax_kind == lctx_model::domain::source::SyntaxKind::ModModule).unwrap();
    module.set_records(rows, FunctionBodyObservation { declaration: root.id(), ..body.clone() });
    refused(stored(&module), "a module's body", "a function body belongs to a def");
    assert_eq!(body.body, FunctionBodyKind::Ellipsis);
}

#[test]
fn an_inherited_field_refers_to_its_declaration_in_another_module() {
    use lctx_model::domain::source::{Occurrence, SyntaxKind};
    let stored = |f: &Fixture| -> Result<(), ModelError> {
        f.base.check(&RecordFieldObservation::invariants()[0])?;
        f.base.check(&RecordFieldSupport::invariants()[0])
    };
    // The base class sits in `other.py`; the record class's claim stays under `example.py`'s scope.
    let declare = |f: &mut Fixture, inside_class: bool| {
        let (mut fields, body) = f.records();
        let declaration = f.base.foreign.clone();
        let mut occurrences = f.base.rows::<Occurrence>();
        if inside_class { occurrences.push(Occurrence { syntax_kind: SyntaxKind::StmtClassDef, structural_path: vec![7], ..declaration.clone() }); }
        f.base.put(occurrences);
        fields[0].declaration = Some(declaration.id());
        f.set_records(fields, body);
    };
    let mut inherited = Fixture::new(false); declare(&mut inherited, true);
    stored(&inherited).expect("an inherited field declared in another module of the input");
    let mut loose = Fixture::new(false); declare(&mut loose, false);
    refused(stored(&loose), "a declaration outside any class", "lies inside a class statement");
}
