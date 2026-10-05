#[path = "fixtures/types.rs"]
mod fixture;
use fixture::Fixture;
use fixture::Vocabulary;
use lctx_model::domain::{
    attribution::{Fidelity, ObligationKind},
    calls::ProviderModule,
    types::*,
    value::Literal,
    *,
};

#[test]
fn named_callable_closures_keep_provider_context_and_function_kind() {
    for nested in [false, true] {
        for mismatch in ["none", "provider", "context", "kind"] {
            let mut f = Fixture::new(false);
            f.named_callable(mismatch, nested);
            let result = f
                .base
                .check(&lctx_model::domain::validation::invariants_for::<TypeSupport>()[0]);
            assert_eq!(
                result.is_ok(),
                mismatch == "none",
                "{mismatch}/{nested}: {result:?}"
            );
        }
    }
}

#[test]
fn type_structure_keeps_large_literals_variable_identity_and_recursive_restrictions() {
    let f = Fixture::new(false);
    for name in [
        "type_sequence_membership",
        "structural_type_shapes",
        TypeSupport::NAME,
        TypePresentationSupport::NAME,
        TypeRestrictionSupport::NAME,
    ] {
        f.base
            .check(
                f.base
                    .model
                    .invariants()
                    .iter()
                    .find(|i| i.name == name)
                    .unwrap(),
            )
            .unwrap();
    }
    let views = f.base.rows::<TypePresentation>();
    assert_eq!(views.len(), 2);
    assert!(views.iter().all(|v| v.term == f.term.id()));
    assert_ne!(views[0].id(), views[1].id());
    let mut renamed = f.variable.clone();
    renamed.module = ProviderModule::Bundled {
        provider: f.variable.provider,
        bundle: lctx_model::domain::calls::ModuleBundle::Typeshed,
        name: "unrelated".into(),
    }
    .id();
    assert_ne!(renamed.id(), f.variable.id());
    let literal = f
        .base
        .rows::<Literal>()
        .into_iter()
        .find(|v| matches!(v, Literal::Integer { .. }))
        .unwrap();
    assert_eq!(
        Literal::decode(
            Batch::new(&f.base.model, vec![literal.clone()], &budget())
                .unwrap()
                .arrow()
        )
        .unwrap(),
        vec![literal]
    );
    let restriction = f.base.rows::<TypeVariableRestriction>()[0].clone();
    let instance = f
        .base
        .rows::<TypeTerm>()
        .into_iter()
        .find(|t| t.id() == restriction.term)
        .unwrap();
    assert!(
        matches!(instance, TypeTerm::ClassInstance { .. }),
        "recursive bound is a relationship, not key expansion"
    );
}
#[test]
fn a_nested_type_variable_cannot_switch_provider_namespace() {
    let f = Fixture::new(true);
    let error = f
        .base
        .check(&lctx_model::domain::validation::invariants_for::<TypeSupport>()[0])
        .unwrap_err();
    assert!(error.to_string().contains("type variable belongs"));
    assert!(
        f.base
            .check(&lctx_model::domain::validation::invariants_for::<TypeRestrictionSupport>()[0])
            .is_err()
    );
}
#[test]
fn type_sequences_require_complete_ordered_membership_and_kind_correct_children() {
    let mut f = Fixture::new(false);
    let mut members = f.base.rows::<TypeSequenceMember>();
    members.pop();
    f.put_members(members);
    assert!(
        f.base
            .check(&lctx_model::domain::validation::invariants_for::<TypeSequence>()[0])
            .is_err()
    );
    let mut f = Fixture::new(false);
    let mut members = f.base.rows::<TypeSequenceMember>();
    members[0].ordinal += 1;
    f.put_members(members);
    assert!(
        f.base
            .check(&lctx_model::domain::validation::invariants_for::<TypeSequence>()[0])
            .is_err()
    );
    let mut f = Fixture::new(false);
    let wrong = TypeTerm::ParamSpec {
        variable: f.variable.id(),
    };
    let mut terms = f.base.rows::<TypeTerm>();
    terms.push(wrong);
    f.base.put(terms);
    assert!(
        f.base
            .check(&lctx_model::domain::validation::invariants_for::<TypeTerm>()[0])
            .is_err()
    );
}

#[test]
fn opaque_type_leaves_cannot_be_promoted_to_structural_fidelity() {
    use lctx_model::domain::attribution::Fidelity;
    for truncated in [false, true] {
        for nested in [false, true] {
            for fidelity in [
                Fidelity::Raw,
                Fidelity::NativeStructural,
                Fidelity::NormalizedStructural,
                Fidelity::ReportProjection,
                Fidelity::DisplayOnly,
            ] {
                let mut f = Fixture::new(false);
                f.opaque(truncated, nested, fidelity);
                let result = f
                    .base
                    .check(&lctx_model::domain::validation::invariants_for::<TypeSupport>()[0]);
                assert_eq!(
                    result.is_ok(),
                    fidelity == Fidelity::DisplayOnly,
                    "{truncated}/{nested}/{fidelity:?}: {result:?}"
                );
            }
        }
    }
}

/// A fresh attempt budget; these controls do not share reservations across batches.
fn budget() -> lctx_model::domain::resources::ResourceBudget {
    lctx_model::domain::resources::ResourceBudget::fixed(
        lctx_model::domain::resources::DEFAULT_MEMORY_BYTES,
    )
    .unwrap()
}

/// Supports over one shared wide term walk it once per provider key: 600 presentations of a
/// 2,000-member union were about 1.2M cumulative closure steps, which a global cap refused as an
/// invalid model (resource review F03).
#[test]
fn shared_type_closures_are_verified_once() {
    use lctx_model::domain::attribution::Fidelity;
    let mut f = Fixture::new(false);
    let literals: Vec<_> = (0..2000)
        .map(|n| Literal::Integer {
            decimal: n.to_string(),
        })
        .collect();
    let terms: Vec<_> = literals
        .iter()
        .map(|l| TypeTerm::Literal { value: l.id() })
        .collect();
    let (union, members) = TypeSequence::new(
        &terms
            .iter()
            .map(|t| (TypeChildRole::Member, t.id()))
            .collect::<Vec<_>>(),
    )
    .unwrap();
    let wide = TypeTerm::Union {
        members: union.id(),
    };
    let base = f.base.rows::<TypePresentation>()[0].clone();
    let support = f.base.rows::<TypePresentationSupport>()[0].clone();
    let presentations: Vec<_> = (0..600)
        .map(|n| TypePresentation {
            term: wide.id(),
            display: format!("wide{n}"),
            ..base.clone()
        })
        .collect();
    let supports: Vec<_> = presentations
        .iter()
        .map(|p| TypePresentationSupport {
            assertion: p.id(),
            ..support.clone()
        })
        .collect();
    macro_rules! append {
        ($ty:ty, $rows:expr) => {
            let mut rows = f.base.rows::<$ty>();
            rows.extend($rows);
            f.base.put(rows);
        };
    }
    append!(Literal, literals);
    append!(TypeTerm, terms.into_iter().chain([wide]));
    append!(TypeSequence, vec![union]);
    append!(TypePresentation, presentations);
    append!(TypePresentationSupport, supports);
    let mut all = f.base.rows::<TypeSequenceMember>();
    all.extend(members);
    f.put_members(all);
    f.base
        .check(&lctx_model::domain::validation::invariants_for::<TypePresentationSupport>()[0])
        .unwrap();
    // The memo is keyed by fidelity: an opaque term verified for display-only support still
    // refuses a structural support of the same term.
    let mut g = Fixture::new(false);
    g.opaque(false, true, Fidelity::DisplayOnly);
    g.base
        .check(&lctx_model::domain::validation::invariants_for::<TypeSupport>()[0])
        .unwrap();
    let observation = TypeObservation {
        role: TypeRole::CallResult,
        ..g.base.rows::<TypeObservation>()[0].clone()
    };
    let structural = TypeSupport {
        assertion: observation.id(),
        fidelity: Fidelity::NativeStructural,
        ..g.base.rows::<TypeSupport>()[0].clone()
    };
    let mut observations = g.base.rows::<TypeObservation>();
    observations.push(observation);
    g.base.put(observations);
    let mut supports = g.base.rows::<TypeSupport>();
    supports.push(structural);
    g.base.put(supports);
    let error = g
        .base
        .check(&lctx_model::domain::validation::invariants_for::<TypeSupport>()[0])
        .unwrap_err();
    assert!(error.to_string().contains("display-only"), "{error}");
}

fn refused(result: Result<(), ModelError>, why: &str, expected: &str) {
    assert!(
        matches!(&result, Err(ModelError::Invalid(message)) if message.contains(expected)),
        "{why}: expected `{expected}`, got {result:?}"
    );
}
fn shapes(f: &Fixture) -> Result<(), ModelError> {
    for invariant in [
        &lctx_model::domain::validation::invariants_for::<TypeTerm>()[0],
        &lctx_model::domain::validation::invariants_for::<TypeSequence>()[0],
        &lctx_model::domain::validation::invariants_for::<CallableParameterList>()[0],
        &lctx_model::domain::validation::invariants_for::<TypePresentationSupport>()[0],
    ] {
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
    assert_ne!(id("args"), id("kwargs"));
    assert_ne!(id("args"), id("p"));
    assert_ne!(id("type_is"), id("type_guard"));
    assert_ne!(
        id("type_form"),
        TypeTerm::TypeOf {
            target: match &v.terms["type_form"] {
                TypeTerm::TypeForm { target } => *target,
                _ => unreachable!(),
            }
        }
        .id()
    );
    assert_ne!(id("alias"), id("alias_ref"));
    assert_ne!(id("typed_dict"), id("anonymous"));
    let renamed = match v.terms["callable"].clone() {
        TypeTerm::Callable {
            form,
            parameters,
            param_spec,
            returns,
            ..
        } => TypeTerm::Callable {
            function: Some(
                lctx_model::domain::calls::ProviderSymbol {
                    native_key: "g".into(),
                    ..f.base
                        .rows::<lctx_model::domain::calls::ProviderSymbol>()
                        .into_iter()
                        .find(|s| s.kind == lctx_model::domain::calls::SymbolKind::Function)
                        .unwrap()
                }
                .id(),
            ),
            form,
            parameters,
            param_spec,
            returns,
        },
        _ => unreachable!(),
    };
    assert_ne!(
        id("callable"),
        renamed.id(),
        "a def's type names its function"
    );
    // A test operand is a type role of its own.
    let observation = TypeObservation {
        role: TypeRole::TestOperand,
        term: id("literal_string"),
        ..f.base.rows::<TypeObservation>()[0].clone()
    };
    let support = TypeSupport {
        assertion: observation.id(),
        ..f.base.rows::<TypeSupport>()[0].clone()
    };
    let mut observations = f.base.rows::<TypeObservation>();
    observations.push(observation);
    f.base.put(observations);
    let mut supports = f.base.rows::<TypeSupport>();
    supports.push(support);
    f.base.put(supports);
    f.base
        .check(&lctx_model::domain::validation::invariants_for::<TypeSupport>()[0])
        .unwrap();
}

#[test]
fn nested_truncated_structure_is_still_display_only() {
    use lctx_model::domain::{
        attribution::{Fidelity, Provider, ProviderRun},
        calls::ParameterKind,
    };
    for fidelity in [Fidelity::NativeStructural, Fidelity::DisplayOnly] {
        let mut f = Fixture::new(false);
        f.vocabulary();
        let run = f
            .base
            .rows::<ProviderRun>()
            .into_iter()
            .find(|r| r.id() == f.base.rows::<TypeSupport>()[0].run)
            .unwrap();
        let _ = Provider::NAME;
        let cut = TypeTerm::Truncated {
            provider: run.provider,
            context: run.context,
            reason: ObligationKind::BudgetReached,
            display: "Deep[...]".into(),
        };
        // Truncation two levels down: a callable's parameter, inside an overload, inside a generic.
        let (list, slots) = CallableParameterList::new(&[Slot {
            name: Some("deep".into()),
            kind: ParameterKind::PositionalOrKeyword,
            required: Some(true),
            term: cut.id(),
        }])
        .unwrap();
        let callable = TypeTerm::Callable {
            function: None,
            form: CallableForm::List,
            parameters: list.id(),
            param_spec: None,
            returns: TypeTerm::None.id(),
        };
        let (signatures, members) =
            TypeSequence::new(&[(TypeChildRole::Signature, callable.id())]).unwrap();
        let overload = TypeTerm::Overload {
            function: f
                .base
                .rows::<lctx_model::domain::calls::ProviderSymbol>()
                .into_iter()
                .find(|s| s.kind == lctx_model::domain::calls::SymbolKind::Function)
                .unwrap()
                .id(),
            signatures: signatures.id(),
        };
        let bound = TypeTerm::BoundMethod {
            receiver: TypeTerm::None.id(),
            function: overload.id(),
        };
        let mut terms = f.base.rows::<TypeTerm>();
        terms.extend([cut, callable, overload, bound.clone()]);
        f.base.put(terms);
        let mut sequences = f.base.rows::<TypeSequence>();
        sequences.push(signatures);
        f.base.put(sequences);
        let mut all = f.base.rows::<TypeSequenceMember>();
        all.extend(members);
        f.put_members(all);
        let mut lists = f.base.rows::<CallableParameterList>();
        lists.push(list);
        f.base.put(lists);
        let mut all = f.base.rows::<CallableParameter>();
        all.extend(slots);
        f.put_slots(all);
        f.present(&[&bound], fidelity);
        let result = f
            .base
            .check(&lctx_model::domain::validation::invariants_for::<TypePresentationSupport>()[0]);
        if fidelity == Fidelity::DisplayOnly {
            result.expect("display-only support of truncated structure");
        } else {
            refused(
                result,
                "structural support of a truncated closure",
                "display-only",
            );
        }
    }
}

#[test]
fn every_form_keeps_its_shape() {
    use lctx_model::domain::calls::ParameterKind;
    let mutate =
        |name: &str, change: &dyn Fn(&Vocabulary, &TypeTerm) -> TypeTerm, expected: &str| {
            let mut f = Fixture::new(false);
            let v = f.vocabulary();
            let changed = change(&v, &v.terms[name]);
            let mut terms = f.base.rows::<TypeTerm>();
            terms.push(changed);
            f.base.put(terms);
            refused(shapes(&f), name, expected);
        };
    mutate(
        "overload",
        &|v, t| TypeTerm::Overload {
            function: match t {
                TypeTerm::Overload { function, .. } => *function,
                _ => unreachable!(),
            },
            signatures: match &v.terms["generic"] {
                TypeTerm::Generic { parameters, .. } => *parameters,
                _ => unreachable!(),
            },
        },
        "other role",
    );
    mutate(
        "generic",
        &|v, t| match t {
            TypeTerm::Generic { parameters, .. } => TypeTerm::Generic {
                parameters: *parameters,
                body: v.terms["none"].id(),
            },
            _ => unreachable!(),
        },
        "a generic binds type variables",
    );
    mutate(
        "variadic",
        &|v, t| match t {
            TypeTerm::Callable { returns, .. } => TypeTerm::Callable {
                function: None,
                form: CallableForm::Ellipsis,
                parameters: match &v.terms["callable"] {
                    TypeTerm::Callable { parameters, .. } => *parameters,
                    _ => unreachable!(),
                },
                param_spec: None,
                returns: *returns,
            },
            _ => unreachable!(),
        },
        "differ from its form",
    );
    mutate(
        "forwarding",
        &|_, t| match t {
            TypeTerm::Callable {
                parameters,
                returns,
                ..
            } => TypeTerm::Callable {
                function: None,
                form: CallableForm::ParamSpec,
                parameters: *parameters,
                param_spec: None,
                returns: *returns,
            },
            _ => unreachable!(),
        },
        "differ from its form",
    );
    mutate(
        "anonymous",
        &|_, _| TypeTerm::AnonymousTypedDict {
            fields: TypedDictFieldList::new(&[]).unwrap().0.id(),
            partial: false,
        },
        "field list absent",
    );
    mutate(
        "args",
        &|_, _| TypeTerm::VariableForm {
            variable: Fixture::new(false).variable.id(),
            form: VariableFormKind::Args,
        },
        "needs its variable's kind",
    );
    mutate(
        "bound",
        &|v, _| TypeTerm::BoundMethod {
            receiver: v.terms["none"].id(),
            function: v.terms["none"].id(),
        },
        "binds a callable",
    );
    // Row contracts.
    assert_ne!(
        TypeTerm::SpecialForm {
            form: TypingForm::Literal
        }
        .id(),
        TypeTerm::SpecialForm {
            form: TypingForm::Ellipsis
        }
        .id()
    );
    assert!(
        CallableParameterList::new(&[Slot {
            name: Some("args".into()),
            kind: ParameterKind::VarPositional,
            required: Some(false),
            term: TypeTerm::None.id()
        }])
        .is_err()
    );
    assert!(
        CallableParameterList::new(&[Slot {
            name: None,
            kind: ParameterKind::KeywordOnly,
            required: Some(true),
            term: TypeTerm::None.id()
        }])
        .is_err()
    );
    // A list missing a member.
    let mut f = Fixture::new(false);
    f.vocabulary();
    let mut slots = f.base.rows::<CallableParameter>();
    slots.pop();
    f.put_slots(slots);
    refused(
        shapes(&f),
        "a parameter list missing a slot",
        "callable parameter",
    );
}

#[test]
fn a_module_type_is_its_providers() {
    use lctx_model::domain::{attribution::Provider, calls::ModuleBundle};
    let mut f = Fixture::new(false);
    let v = f.vocabulary();
    let alien = f
        .base
        .rows::<Provider>()
        .into_iter()
        .find(|p| p.tool == "other-type-provider")
        .unwrap();
    let foreign = ProviderModule::Bundled {
        provider: alien.id(),
        bundle: ModuleBundle::Typeshed,
        name: "os".into(),
    };
    let module = TypeTerm::Module {
        module: foreign.id(),
    };
    let mut modules = f.base.rows::<ProviderModule>();
    modules.push(foreign);
    f.base.put(modules);
    let mut terms = f.base.rows::<TypeTerm>();
    terms.push(module.clone());
    f.base.put(terms);
    f.present(
        &[&module],
        lctx_model::domain::attribution::Fidelity::NativeStructural,
    );
    refused(
        f.base
            .check(&lctx_model::domain::validation::invariants_for::<TypePresentationSupport>()[0]),
        "a module another provider bundles",
        "type module belongs to another provider",
    );
    assert!(v.terms.contains_key("module"));
}

#[test]
fn record_fields_state_their_models_flags_in_field_order() {
    let mut f = Fixture::new(false);
    let (fields, _) = f.records();
    let stored = |f: &Fixture| -> Result<(), ModelError> {
        for invariant in [
            &lctx_model::domain::validation::invariants_for::<RecordFieldObservation>()[0],
            &lctx_model::domain::validation::invariants_for::<RecordFieldSupport>()[0],
            &lctx_model::domain::validation::invariants_for::<FunctionBodyObservation>()[0],
            &lctx_model::domain::validation::invariants_for::<FunctionBodySupport>()[0],
        ] {
            f.base.check(invariant)?;
        }
        Ok(())
    };
    stored(&f).unwrap();
    // Each record model states exactly its own flags.
    let dataclass = fields[1].clone();
    for (row, why) in [
        (
            RecordFieldObservation {
                record: RecordKind::TypedDict,
                ..dataclass.clone()
            },
            "a typed-dict field with dataclass flags",
        ),
        (
            RecordFieldObservation {
                init: None,
                ..dataclass.clone()
            },
            "a dataclass field without init",
        ),
        (
            RecordFieldObservation {
                record: RecordKind::NamedTuple,
                init: None,
                ..dataclass.clone()
            },
            "a named-tuple field with an alias",
        ),
        (
            RecordFieldObservation {
                record: RecordKind::TypedDict,
                has_default: None,
                init: None,
                alias: None,
                kw_only: None,
                required: Some(true),
                ..dataclass.clone()
            },
            "a typed-dict field without read-only",
        ),
    ] {
        assert!(
            matches!(row.validate(), Err(ModelError::Invalid(ref m)) if m.contains("exactly the flags")),
            "{why}"
        );
    }
    RecordFieldObservation {
        record: RecordKind::TypedDict,
        has_default: None,
        init: None,
        alias: None,
        kw_only: None,
        required: Some(false),
        read_only: Some(true),
        ..dataclass.clone()
    }
    .validate()
    .expect("a typed-dict field's own flags");
    RecordFieldObservation {
        record: RecordKind::NamedTuple,
        init: None,
        alias: None,
        kw_only: None,
        ..dataclass.clone()
    }
    .validate()
    .expect("a named-tuple field's default");
    // Field order is 0..n, and one class has one record model.
    let mut gap = Fixture::new(false);
    let (mut rows, body) = gap.records();
    rows[1].ordinal = 2;
    gap.set_records(rows, body);
    refused(stored(&gap), "fields at ordinals 0 and 2", "ordinals 0..n");
    let mut mixed = Fixture::new(false);
    let (mut rows, body) = mixed.records();
    rows[1] = RecordFieldObservation {
        record: RecordKind::Attrs,
        ..rows[1].clone()
    };
    mixed.set_records(rows, body);
    refused(
        stored(&mixed),
        "a dataclass field beside an attrs field",
        "one record model",
    );
    // A function body belongs to a def.
    let mut module = Fixture::new(false);
    let (rows, body) = module.records();
    let root = module
        .base
        .rows::<lctx_model::domain::source::Occurrence>()
        .into_iter()
        .find(|o| o.syntax_kind == lctx_model::domain::source::SyntaxKind::ModModule)
        .unwrap();
    module.set_records(
        rows,
        FunctionBodyObservation {
            declaration: root.id(),
            ..body.clone()
        },
    );
    refused(
        stored(&module),
        "a module's body",
        "a function body belongs to a def",
    );
    assert_eq!(body.body, FunctionBodyKind::Ellipsis);
}

#[test]
fn an_inherited_field_refers_to_its_declaration_in_another_module() {
    use lctx_model::domain::source::{Occurrence, SyntaxKind};
    let stored = |f: &Fixture| -> Result<(), ModelError> {
        f.base.check(
            &lctx_model::domain::validation::invariants_for::<RecordFieldObservation>()[0],
        )?;
        f.base
            .check(&lctx_model::domain::validation::invariants_for::<RecordFieldSupport>()[0])
    };
    // The base class sits in `other.py`; the record class's claim stays under `example.py`'s scope.
    let declare = |f: &mut Fixture, inside_class: bool| {
        let (mut fields, body) = f.records();
        let declaration = f.base.foreign.clone();
        let mut occurrences = f.base.rows::<Occurrence>();
        if inside_class {
            occurrences.push(Occurrence {
                syntax_kind: SyntaxKind::StmtClassDef,
                structural_path: vec![7],
                ..declaration.clone()
            });
        }
        f.base.put(occurrences);
        fields[0].declaration = Some(declaration.id());
        f.set_records(fields, body);
    };
    let mut inherited = Fixture::new(false);
    declare(&mut inherited, true);
    stored(&inherited).expect("an inherited field declared in another module of the input");
    let mut loose = Fixture::new(false);
    declare(&mut loose, false);
    refused(
        stored(&loose),
        "a declaration outside any class",
        "lies inside a class statement",
    );
}

#[test]
fn dictionary_membership_retains_string_keys_and_refuses_corrupt_members() {
    for fault in ["none", "missing", "ordinal", "name"] {
        let mut f = Fixture::new(false);
        let term = f.base.rows::<TypeTerm>()[0].id();
        let slots = [("".into(), false, term), (" ".into(), true, term)];
        let (list, mut fields) = TypedDictFieldList::new(&slots).unwrap();
        if fault == "missing" {
            fields.pop();
        }
        if fault == "ordinal" {
            fields[1].ordinal = 3;
        }
        if fault == "name" {
            fields[1].name = "".into();
        }
        f.base.put(vec![list]);
        f.base.put(fields);
        let result = f
            .base
            .check(&lctx_model::domain::validation::invariants_for::<TypedDictFieldList>()[0]);
        assert_eq!(result.is_ok(), fault == "none", "{fault}: {result:?}");
        assert!(
            TypedDictFieldList::new(&[("".into(), true, term), ("".into(), false, term)]).is_err()
        );
    }
    let term = TypeTerm::None.id();
    assert!(
        CallableParameterList::new(&[Slot {
            name: Some("".into()),
            kind: lctx_model::domain::calls::ParameterKind::KeywordOnly,
            required: Some(false),
            term
        }])
        .is_ok()
    );
}

#[test]
fn unavailable_expanded_type_slots_require_display_only_and_lists_refuse_empty_names() {
    use lctx_model::domain::{attribution::Fidelity, calls::ParameterKind};
    for form in [
        CallableForm::List,
        CallableForm::Partial,
        CallableForm::NativeUnavailable,
    ] {
        for fidelity in [Fidelity::NativeStructural, Fidelity::DisplayOnly] {
            let mut f = Fixture::new(false);
            let term = f.term.id();
            let (list, slots) = CallableParameterList::new(&[Slot {
                name: Some("".into()),
                kind: ParameterKind::KeywordOnly,
                required: Some(false),
                term,
            }])
            .unwrap();
            let callable = TypeTerm::Callable {
                function: None,
                form,
                parameters: list.id(),
                param_spec: None,
                returns: term,
            };
            let mut lists = f.base.rows::<CallableParameterList>();
            lists.push(list);
            f.base.put(lists);
            let mut members = f.base.rows::<CallableParameter>();
            members.extend(slots);
            f.put_slots(members);
            let mut terms = f.base.rows::<TypeTerm>();
            terms.push(callable.clone());
            f.base.put(terms);
            f.present(&[&callable], fidelity);
            let shape = f
                .base
                .check(&lctx_model::domain::validation::invariants_for::<TypeTerm>()[0]);
            assert_eq!(
                shape.is_ok(),
                form == CallableForm::NativeUnavailable,
                "{form:?}: {shape:?}"
            );
            let closure = f.base.check(
                &lctx_model::domain::validation::invariants_for::<TypePresentationSupport>()[0],
            );
            assert_eq!(
                closure.is_ok(),
                form != CallableForm::NativeUnavailable || fidelity == Fidelity::DisplayOnly,
                "{form:?}/{fidelity:?}: {closure:?}"
            );
        }
    }
}

#[test]
fn opaque_children_keep_typed_ports_without_structural_support() {
    for port in ["bound", "overload", "generic_parameter", "generic_body"] {
        for truncated in [false, true] {
            for (fidelity, foreign) in [
                (Fidelity::DisplayOnly, false),
                (Fidelity::NativeStructural, false),
                (Fidelity::DisplayOnly, true),
            ] {
                let mut fixture = Fixture::new(false);
                fixture.residual_envelope(port, truncated, fidelity, foreign);
                fixture
                    .base
                    .check(
                        fixture
                            .base
                            .model
                            .invariants()
                            .iter()
                            .find(|i| i.name == "structural_type_shapes")
                            .unwrap(),
                    )
                    .unwrap();
                assert_eq!(
                    fixture
                        .base
                        .check(
                            &lctx_model::domain::validation::invariants_for::<
                                TypePresentationSupport,
                            >()[0]
                        )
                        .is_ok(),
                    fidelity == Fidelity::DisplayOnly && !foreign,
                    "{port}/{truncated}/{fidelity:?}/{foreign}"
                );
            }
        }
    }
}

#[test]
fn native_signature_ports_keep_role_origin_and_qualification() {
    for mismatch in [
        "none",
        "role",
        "term",
        "metadata_origin",
        "not_deprecated_message",
        "deprecated_none",
    ] {
        let mut f = Fixture::new(false);
        f.native_signature_ports(mismatch);
        let check = lctx_model::domain::validation::invariants_for::<NativeSignatureObservation>()
            .into_iter()
            .find(|i| i.name == "native_signature_ports")
            .unwrap();
        assert_eq!(
            f.base.check(&check).is_ok(),
            matches!(mismatch, "none" | "deprecated_none"),
            "{mismatch}"
        );
    }
}

#[test]
fn expected_argument_locations_require_exact_call_membership_and_context() {
    use lctx_model::domain::{
        assertion::AssertionQualification,
        calls::{Actual, ArgumentKind, CallArgument, CallSyntax},
        source::{Occurrence, SyntaxKind},
        syntax::SyntaxPlacement,
    };
    for case in ["exact", "absent", "foreign-context", "declared"] {
        let mut f = Fixture::new(false);
        let q = f.base.rows::<AssertionQualification>()[0].clone();
        let original = f.base.rows::<Occurrence>()[0].clone();
        let argument = Occurrence {
            syntax_kind: SyntaxKind::ExprList,
            ..original.clone()
        };
        let callsite = Occurrence {
            syntax_kind: SyntaxKind::ExprCall,
            ..original
        };
        let mut occurrences = f.base.rows::<Occurrence>();
        occurrences.extend([argument.clone(), callsite.clone()]);
        f.base.put(occurrences);
        let mut callq = q.clone();
        if case == "foreign-context" {
            callq.context =
                serde_json::from_value(serde_json::to_value([255u8; 16]).unwrap()).unwrap();
            let mut qualifications = f.base.rows::<AssertionQualification>();
            qualifications.push(callq.clone());
            f.base.put(qualifications);
        }
        let (call, arguments) = CallSyntax::new(
            callq.id(),
            callsite.id(),
            callsite.id(),
            false,
            &[Actual {
                occurrence: argument.id(),
                kind: ArgumentKind::Positional,
                keyword: None,
            }],
        )
        .unwrap();
        f.base.put(vec![call]);
        f.base.put(if case == "absent" {
            Vec::<CallArgument>::new()
        } else {
            arguments
        });
        f.base.put(Vec::<SyntaxPlacement>::new());
        f.base.put(vec![TypeObservation {
            qualification: q.id(),
            subject: argument.id(),
            role: TypeRole::Expected,
            declared: case == "declared",
            term: f.term.id(),
        }]);
        let result = f
            .base
            .check(&lctx_model::domain::validation::invariants_for::<TypeObservation>()[0]);
        assert_eq!(result.is_ok(), case == "exact", "{case}: {result:?}");
    }
}
