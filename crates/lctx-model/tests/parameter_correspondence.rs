//! Independently specified source/container/native identity controls.
use lctx_model::domain::{
    assertion::{Approximation, AssertionQualification},
    attribution::{AnalysisContext, Modality},
    calls::{ParameterKind, ParameterShape, Signature, SignatureForm},
    declarations::{ParameterDeclaration, SymbolDeclaration},
    lexical::SyntaxField,
    normalized::{binding_normalization::BindingData, parameter_correspondence::source_parameter},
    resources::ResourceBudget,
    source::{Occurrence, OccurrenceRole, SyntaxKind},
    syntax::{ParameterSyntaxObservation, SyntaxPlacement},
    *,
};
fn id<T>(n: u8) -> Id<T> { serde_json::from_value(serde_json::json!(vec![n; 16])).unwrap() }
fn qualification(context: Id<AnalysisContext>) -> AssertionQualification {
    AssertionQualification {
        assumptions: lctx_model::domain::assumptions::AssumptionSet::empty_id(), context, scope: id(2), condition: id(3), modality: Modality::Definite, approximation: Approximation::Exact }
}
fn occurrence(kind: SyntaxKind, path: i32) -> Occurrence {
    Occurrence { source: id(4), start: 10, end: 30, syntax_kind: kind, role: OccurrenceRole::Parameter, structural_path: vec![path] }
}
fn syntax(q: &AssertionQualification, parameter: &Occurrence, kind: ParameterKind) -> ParameterSyntaxObservation {
    ParameterSyntaxObservation { qualification: q.id(), function: id(5), parameter: parameter.id(), ordinal: 0, kind, default: None, default_literal: None, annotation: None }
}
fn attach(data: &mut BindingData, q: &AssertionQualification, parent: &Occurrence, child: &Occurrence) {
    data.occurrences.insert(child.clone()).unwrap();
    data.placements.insert(SyntaxPlacement { qualification: q.id(), occurrence: child.id(), parent: Some(parent.id()), field: SyntaxField::Child, ordinal: 0 }).unwrap();
}
fn native(data: &mut BindingData, q: &AssertionQualification, p: &ParameterSyntaxObservation, formal: &Occurrence, variant: i64, kind: ParameterKind, function: Id<Occurrence>) -> Id<calls::SignatureParameter> {
    let shape = ParameterShape { name: Some("same_name".into()), kind, required: !matches!(kind, ParameterKind::VarPositional | ParameterKind::VarKeyword) };
    let (signature, slots) = Signature::new(q, id(6), variant, SignatureForm::List, &[shape.clone()]).unwrap();
    data.shapes.insert(shape).unwrap();
    data.signatures.insert(signature.clone()).unwrap();
    data.parameters.insert(slots[0].clone()).unwrap();
    data.parameter_declarations.insert(ParameterDeclaration { qualification: q.id(), parameter: slots[0].id(), declaration: formal.id() }).unwrap();
    data.entity_declarations.insert(SymbolDeclaration { qualification: q.id(), symbol: signature.symbol, declaration: function }).unwrap();
    assert_eq!(p.ordinal, 0);
    slots[0].id()
}
#[test]
fn formal_child_not_equal_span_container_owns_the_native_slots() {
    let b = ResourceBudget::fixed(1 << 20).unwrap();
    let mut d = BindingData::new(&b);
    let q = qualification(id(1));
    d.qualifications.insert(q.clone()).unwrap();
    let container = occurrence(SyntaxKind::ParameterWithDefault, 0);
    let formal = occurrence(SyntaxKind::Parameter, 1);
    d.occurrences.insert(container.clone()).unwrap();
    attach(&mut d, &q, &container, &formal);
    let p = syntax(&q, &container, ParameterKind::PositionalOrKeyword);
    let slot = native(&mut d, &q, &p, &formal, 0, p.kind, p.function);
    let wrong_kind = native(&mut d, &q, &p, &formal, 1, ParameterKind::KeywordOnly, p.function);
    let found = source_parameter(&d, &p, q.context, &b).unwrap().unwrap();
    assert_eq!(found.formal, formal.id());
    assert_ne!(found.formal, container.id());
    assert!(found.parameters.contains(&slot));
    assert!(!found.parameters.contains(&wrong_kind));
    assert!(source_parameter(&d, &p, id(9), &b).unwrap().is_none());
}
#[test]
fn variadic_is_its_own_formal_but_a_name_does_not_attach_an_unrelated_function() {
    let b = ResourceBudget::fixed(1 << 20).unwrap();
    let mut d = BindingData::new(&b);
    let q = qualification(id(1));
    d.qualifications.insert(q.clone()).unwrap();
    let formal = occurrence(SyntaxKind::Parameter, 0);
    d.occurrences.insert(formal.clone()).unwrap();
    let p = syntax(&q, &formal, ParameterKind::VarPositional);
    native(&mut d, &q, &p, &formal, 0, p.kind, id(9));
    let found = source_parameter(&d, &p, q.context, &b).unwrap().unwrap();
    assert_eq!(found.formal, formal.id());
    assert!(found.parameters.is_empty());
}
#[test]
fn unavailable_ambiguous_and_cross_source_children_are_not_guessed() {
    let b = ResourceBudget::fixed(1 << 20).unwrap();
    let mut d = BindingData::new(&b);
    let q = qualification(id(1));
    d.qualifications.insert(q.clone()).unwrap();
    let container = occurrence(SyntaxKind::ParameterWithDefault, 0);
    d.occurrences.insert(container.clone()).unwrap();
    let p = syntax(&q, &container, ParameterKind::PositionalOrKeyword);
    assert!(source_parameter(&d, &p, q.context, &b).unwrap().is_none());
    attach(&mut d, &q, &container, &occurrence(SyntaxKind::Parameter, 1));
    attach(&mut d, &q, &container, &occurrence(SyntaxKind::Parameter, 2));
    assert!(source_parameter(&d, &p, q.context, &b).unwrap().is_none());
    let mut other = BindingData::new(&b);
    other.qualifications.insert(q.clone()).unwrap();
    other.occurrences.insert(container.clone()).unwrap();
    let mut child = occurrence(SyntaxKind::Parameter, 1);
    child.source = id(8);
    attach(&mut other, &q, &container, &child);
    assert!(source_parameter(&other, &p, q.context, &b).is_err());
}

#[test]
fn all_source_kinds_and_descriptor_receivers_use_the_same_formal_mapping() {
    use lctx_model::domain::normalized::{callables::*, entities::*};
    for kind in [ParameterKind::PositionalOnly, ParameterKind::PositionalOrKeyword, ParameterKind::KeywordOnly, ParameterKind::VarPositional, ParameterKind::VarKeyword] {
        let b = ResourceBudget::fixed(1 << 20).unwrap();
        let mut d = BindingData::new(&b);
        let q = qualification(id(1));
        d.qualifications.insert(q.clone()).unwrap();
        let formal = occurrence(SyntaxKind::Parameter, 1);
        let container = if matches!(kind, ParameterKind::VarPositional | ParameterKind::VarKeyword) { formal.clone() } else { occurrence(SyntaxKind::ParameterWithDefault, 0) };
        d.occurrences.insert(container.clone()).unwrap();
        if formal.id() != container.id() { attach(&mut d, &q, &container, &formal); }
        let p = syntax(&q, &container, kind);
        let parameter = native(&mut d, &q, &p, &formal, 0, kind, p.function);
        let callable = CallableEntity::Source { declaration: p.function, kind: CallableKind::Function }.id();
        let mut variant = SignatureVariant { signature: d.parameters.get(parameter).unwrap().signature, context: q.context, resolution: id(10), callable: Some(callable), assessment: None, adjustment: SignatureAdjustment::None };
        d.callable_variants.insert(variant.clone()).unwrap();
        d.callable_slots.insert(SignatureSlot { parameter, variant: variant.id(), ordinal: 0, default: if matches!(kind, ParameterKind::VarPositional | ParameterKind::VarKeyword) { DefaultSlot::Collector } else { DefaultSlot::Required } }).unwrap();
        let found = source_parameter(&d, &p, q.context, &b).unwrap().unwrap();
        assert_eq!(found.formal, formal.id());
        assert!(found.parameters.contains(&parameter));
        assert!(!normalized::parameter_correspondence::is_bound_receiver(&d, &found, callable, q.context));
        d.callable_variants = normalized::Rows::new(&b);
        variant.adjustment = SignatureAdjustment::BindInstanceReceiver;
        d.callable_variants.insert(variant).unwrap();
        assert!(normalized::parameter_correspondence::is_bound_receiver(&d, &found, callable, q.context));
        assert!(!normalized::parameter_correspondence::is_bound_receiver(&d, &found, id(11), q.context));
        assert!(!normalized::parameter_correspondence::is_bound_receiver(&d, &found, callable, id(12)));
    }
}
