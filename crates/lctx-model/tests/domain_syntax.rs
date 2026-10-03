//! Typed syntax contracts (cutover plan A3): placement, detail, declarations, decorators, imports,
//! `__all__`, parameters and class fields over one real source, with subject boundaries and
//! attachment outcomes. Texts are the source bytes at their occurrences. Every control states its
//! answer first.
#[path = "fixtures/syntax.rs"]
#[macro_use]
mod fixture;
use fixture::{Fixture, slice};
use lctx_model::domain::{attribution::FactFamily, source::*, syntax::*, value::*, *};

fn refused(fixture: &Fixture, why: &str, expected: &str) {
    let result = fixture.validate();
    assert!(
        matches!(&result, Err(ModelError::Invalid(message)) if message.contains(expected)),
        "{why}: expected `{expected}`, got {result:?}"
    );
}

#[test]
fn typed_syntax_validates_and_its_texts_are_the_bytes_at_its_occurrences() {
    let f = Fixture::new();
    f.validate().unwrap();
    for (name, bytes) in [
        ("f.name", "f"),
        ("C.name", "C"),
        ("decorator", "@dec"),
        ("import.alias", "os.path as p"),
        ("from.alias", "x"),
        ("docstring", "\"\"\"Doc.\"\"\""),
        ("c", "c: int = 2"),
        ("c.annotation", "int"),
        ("b.default", "1"),
        ("y", "y"),
        ("y.annotation", "int"),
        ("y.value", "3"),
        ("all", "__all__ = [\"f\"]"),
        ("import", "import os.path as p"),
        ("from", "from . import x"),
    ] {
        assert_eq!(
            slice(&f.occ[name]),
            bytes,
            "{name} is the source bytes at its span"
        );
    }
    assert!(
        slice(&f.occ["f"]).starts_with("@dec\ndef f(")
            && slice(&f.occ["C"]).starts_with("class C:")
    );
    // The literal default's value is what the bytes do not state: the parsed integer.
    assert_eq!(
        f.details[0].detail,
        SyntaxDetail::Literal {
            literal: Literal::Integer {
                decimal: "1".into()
            }
            .id()
        }
        .id()
    );
}

#[test]
fn declarations_decorators_parameters_and_fields_nest_as_the_parse_nests() {
    let mut f = Fixture::new();
    f.decorators[0].declaration = f.occ["C"].id();
    f.sync();
    refused(
        &f,
        "a decorator of another declaration",
        "a decorator lies outside its occurrence",
    );
    let mut f = Fixture::new();
    f.declarations[1].kind = DeclarationKind::Function;
    f.sync();
    refused(
        &f,
        "a class declared as a function",
        "have the declared kinds",
    );
    let mut f = Fixture::new();
    f.declarations[0].name = f.occ["a"].id();
    f.sync();
    refused(
        &f,
        "a declaration named by a parameter",
        "have the declared kinds",
    );
    let mut f = Fixture::new();
    f.parameters[0].function = f.occ["C"].id();
    f.sync();
    refused(&f, "a parameter of a class", "belongs to a def or lambda");
    let mut f = Fixture::new();
    f.parameters[2].annotation = Some(f.occ["y.annotation"].id());
    f.sync();
    refused(
        &f,
        "an annotation outside its function",
        "an annotation lies outside its occurrence",
    );
    let mut f = Fixture::new();
    f.fields[0].target = f.occ["a"].id();
    f.sync();
    refused(
        &f,
        "a field target outside its class",
        "a field target lies outside its occurrence",
    );
    let mut f = Fixture::new();
    f.imports[0].level = 1;
    f.sync();
    refused(&f, "a relative plain import", "a plain import is absolute");
    let mut f = Fixture::new();
    f.imports[1].alias = f.occ["import.alias"].id();
    f.sync();
    refused(
        &f,
        "an alias of another statement",
        "an import alias lies outside its occurrence",
    );
    let mut f = Fixture::new();
    f.dunder_all[0].statement = f.occ["b.default"].id();
    f.sync();
    refused(
        &f,
        "__all__ stated by an expression",
        "__all__ is stated by a statement",
    );
}

#[test]
fn a_placement_is_one_per_occurrence_inside_its_parent() {
    let mut f = Fixture::new();
    f.placements[4].parent = Some(f.occ["c"].id());
    f.sync();
    refused(
        &f,
        "a child outside its parent",
        "a placed child lies outside its occurrence",
    );
    let mut f = Fixture::new();
    let mut second = f.placements[1].clone();
    second.ordinal = 5;
    f.placements.push(second);
    f.sync();
    refused(
        &f,
        "two placements of one occurrence",
        "one placement per qualification",
    );
    let mut f = Fixture::new();
    f.placements[1].parent = None;
    f.sync();
    refused(
        &f,
        "a statement placed without a parent",
        "only a module is placed without a parent",
    );
}

#[test]
fn a_foreign_optional_subject_is_refused() {
    let mut f = Fixture::new();
    let foreign = Occurrence {
        source: f.other.id(),
        start: 0,
        end: 1,
        syntax_kind: SyntaxKind::StmtExpr,
        role: OccurrenceRole::Syntax,
        structural_path: vec![9],
    };
    f.add("foreign", foreign.clone());
    f.declarations[0].docstring = Some(foreign.id());
    f.sync();
    assert!(
        f.validate().is_err(),
        "a docstring from another artifact, outside the qualification's scope, is refused"
    );
    let mut twin = Fixture::new();
    twin.add("foreign", foreign);
    twin.validate().unwrap();
}

#[test]
fn row_contracts_refuse_impossible_syntax() {
    let f = Fixture::new();
    assert!(
        DunderAllObservation {
            literal: true,
            names: None,
            ..f.dunder_all[0].clone()
        }
        .validate()
        .is_err(),
        "a literal __all__ states its names"
    );
    assert!(
        DunderAllObservation {
            literal: false,
            ..f.dunder_all[0].clone()
        }
        .validate()
        .is_ok(),
        "a computed __all__ retains a partial literal subset"
    );
    assert!(
        ParameterSyntaxObservation {
            default: None,
            ..f.parameters[1].clone()
        }
        .validate()
        .is_err(),
        "a literal default needs its default"
    );
    assert!(
        ParameterSyntaxObservation {
            kind: calls::ParameterKind::VarPositional,
            ..f.parameters[1].clone()
        }
        .validate()
        .is_err(),
        "*args has no default"
    );
    assert!(
        ClassFieldSyntaxObservation {
            annotation: None,
            value: None,
            ..f.fields[0].clone()
        }
        .validate()
        .is_err(),
        "a bare name is not a field"
    );
    assert!(
        SyntaxPlacement {
            ordinal: -1,
            ..f.placements[1].clone()
        }
        .validate()
        .is_err()
    );
    assert!(
        DeclarationObservation {
            name: f.occ["f"].id(),
            ..f.declarations[0].clone()
        }
        .validate()
        .is_err()
    );
    let boundary = SubjectBoundary {
        scope: f.scope.id(),
        provider: f.provider.id(),
        context: f.context.id(),
        family: FactFamily::Syntax,
        subject: None,
        reason: AttachmentKind::Unmatched.reason(),
        detail: None,
    };
    assert!(
        AttachmentOutcome {
            boundary: boundary.id(),
            source: f.source.id(),
            start: 3,
            end: 2,
            syntax_kind: Some(SyntaxKind::ExprName),
            role: Some(OccurrenceRole::Read),
            outcome: AttachmentKind::Unmatched
        }
        .validate()
        .is_err(),
        "an unordered span"
    );
    assert!(
        SubjectBoundary {
            detail: Some("x".repeat(4097)),
            ..boundary
        }
        .validate()
        .is_err(),
        "a bounded detail"
    );
}
