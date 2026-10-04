#[path = "fixtures/declarations.rs"]
mod fixture;
use fixture::Fixture;
use lctx_model::domain::{calls::*, *};

fn refused(fixture: &Fixture, expected: &str) {
    let error = fixture.validate().unwrap_err().to_string();
    assert!(
        error.contains(expected),
        "expected `{expected}`, got `{error}`"
    );
}

#[test]
fn symbol_and_parameter_declarations_link_to_their_defining_occurrences() {
    let fixture = Fixture::new();
    fixture.validate().unwrap();
    let mut f = Fixture::new();
    f.set_parameters(vec![
        (f.members[0].id(), f.stray.id()),
        (f.members[1].id(), f.b.id()),
    ]);
    refused(&f, "outside its symbol's declaration");
    let mut f = Fixture::new();
    f.set_parameters(vec![
        (f.members[0].id(), f.a.id()),
        (f.members[0].id(), f.b.id()),
        (f.members[1].id(), f.b.id()),
    ]);
    refused(&f, "one-to-one");
    let mut f = Fixture::new();
    f.set_parameters(vec![
        (f.members[0].id(), f.a.id()),
        (f.members[1].id(), f.a.id()),
    ]);
    refused(&f, "one-to-one");
    let mut f = Fixture::new();
    f.set_symbols(vec![
        (f.function.id(), f.definition.id()),
        (f.class_symbol.id(), f.definition.id()),
    ]);
    refused(&f, "does not define the symbol's kind");
    let mut f = Fixture::new();
    let (run, surface) = f.alien();
    let qualification = f.qualification.id();
    f.set_symbols_with(
        vec![
            (f.function.id(), f.definition.id()),
            (f.class_symbol.id(), f.class.id()),
        ],
        qualification,
        run,
        surface,
    );
    refused(&f, "another provider");
    let mut f = Fixture::new();
    let other = f.foreign_qualification();
    f.set_symbols_with(
        vec![
            (f.function.id(), f.definition.id()),
            (f.class_symbol.id(), f.class.id()),
        ],
        other,
        f.run.id(),
        f.surface.id(),
    );
    refused(&f, "differs from its symbol's context");
}

#[test]
fn call_syntax_fixes_complete_ordered_arguments_inside_the_call() {
    let fixture = Fixture::new();
    assert_eq!(fixture.arguments.len(), 2);
    assert_eq!(fixture.call.actuals(&fixture.arguments).unwrap().len(), 2);
    assert!(
        fixture.call.actuals(&fixture.arguments[..1]).is_err(),
        "a partial argument set cannot stand for the call"
    );
    let mut foreign = fixture.arguments.clone();
    foreign[1].call = CallSyntax {
        in_annotation: true,
        ..fixture.call.clone()
    }
    .id();
    assert!(
        fixture.call.actuals(&foreign).is_err(),
        "arguments of another call are refused"
    );
    assert!(
        CallArgument {
            keyword: Some("b".into()),
            kind: ArgumentKind::Positional,
            ..fixture.arguments[0].clone()
        }
        .validate()
        .is_err(),
        "a keyword needs keyword kind"
    );
    let mut f = Fixture::new();
    f.put(vec![f.arguments[0].clone()]);
    refused(&f, "differ from the call's declared membership");
    let mut f = Fixture::new();
    let mut outside = f.arguments.clone();
    outside[0].value = f.definition.id();
    f.put(outside);
    refused(&f, "outside its call site");
}

#[test]
fn declaration_native_qualifications_require_the_explicit_empty_assumption_basis() {
    let mut f = Fixture::new();
    f.put::<assumptions::AssumptionSet>(vec![]);
    refused(
        &f,
        "assertion_qualifications.assumptions references an absent or wrong-subtype assumption_sets",
    );
}
