//! Native class metadata controls through the shared source and provider validators.
#[path = "typed_driver/mod.rs"]
mod typed_driver;
use lctx_model::domain::{
    assertion::AssertionQualification, calls::*, class_metadata::*, normalized::Rows, types::*, *,
};
use typed_driver::{files, rows, run};
inspector!(
    Metadata,
    ClassMetadataObservation,
    ClassMetadataSupport,
    ClassMemberObservation,
    ClassMemberSupport,
    RecordOptions,
    RecordTransformDefaults,
    RecordTransformFieldSpecifier,
    ProviderSymbol,
    TypeObservation,
    TypeTerm,
    AssertionQualification,
    RecordFieldObservation,
    lctx_model::domain::source::Occurrence,
    lctx_model::domain::syntax::SyntaxPlacement
);
#[tokio::test]
async fn native_class_metadata_keeps_effective_options_and_unknown_abstract_absence() {
    let tables = typed_driver::Tables::default();
    run(&files("class_metadata"), Metadata(tables.clone()))
        .await
        .unwrap();
    let symbols = rows::<ProviderSymbol>(&tables);
    let metadata = rows::<ClassMetadataObservation>(&tables);
    let named = |name: &str| {
        let class = symbols
            .iter()
            .find(|s| s.name == name && s.kind == SymbolKind::Class)
            .unwrap()
            .id();
        metadata.iter().find(|m| m.class == class).unwrap()
    };
    let custom = named("Custom");
    assert_eq!(
        symbols
            .iter()
            .find(|s| Some(s.id()) == custom.custom_metaclass)
            .unwrap()
            .name,
        "Meta"
    );
    assert_eq!(custom.slots.as_deref(), Some(["stored".into()].as_slice()));
    let closed = named("Closed");
    assert!(closed.final_declaration);
    assert!(closed.deprecated);
    assert_eq!(
        closed.deprecation_message.as_deref(),
        Some("use the typed interface")
    );
    let reader = named("Reader");
    assert!(reader.protocol && reader.runtime_checkable);
    assert!(reader.protocol_members.contains(&"read".into()));
    assert!(named("Abstract").abstract_members.contains(&"read".into()));
    assert!(named("Concrete").abstract_members.is_empty());
    assert!(metadata.iter().all(|m| !m.abstract_absence_known));
    let config = named("Config");
    let options = rows::<RecordOptions>(&tables);
    let opt = options
        .iter()
        .find(|r| Some(r.id()) == record_options(config, MetadataBasis::NativeEffective))
        .unwrap();
    assert!(opt.init && opt.frozen && opt.kw_only && opt.slots);
    assert_eq!(
        record_options(config, MetadataBasis::SourceDeclaration),
        None
    );
    let plain = named("Plain");
    assert!(!plain.protocol && !plain.final_declaration && !plain.enumeration);
    assert_eq!(plain.record_options, None);
    assert!(named("Choice").enumeration);
    let transform = rows::<RecordTransformDefaults>(&tables)
        .into_iter()
        .find(|t| Some(t.id()) == named("RecordBase").transform)
        .unwrap();
    assert_eq!(transform.field_specifier_count, 1);
    assert!(
        rows::<RecordTransformFieldSpecifier>(&tables)
            .iter()
            .any(|s| s.transform == transform.id() && s.kind == FieldSpecifierKind::Function)
    );
    assert_eq!(
        transform.field_specifier_identity_reason,
        Some(lctx_model::domain::obligation::ObligationKind::NativeUnavailable)
    );
    assert!(named("Transformed").record_options.is_some());
    let members = rows::<ClassMemberObservation>(&tables);
    let inherited = members
        .iter()
        .find(|m| m.class == named("Concrete").class && m.name == "read")
        .unwrap();
    assert_eq!(inherited.origin, MemberOrigin::Source);
    assert_eq!(inherited.basis, MetadataBasis::NativeEffective);
    assert!(!inherited.abstract_declaration);
    let method = members
        .iter()
        .find(|m| m.class == named("Abstract").class && m.name == "read")
        .unwrap();
    assert!(method.abstract_declaration);
    let inherited_value = members
        .iter()
        .find(|m| m.class == named("Derived").class && m.name == "value")
        .unwrap();
    assert_eq!(inherited_value.origin, MemberOrigin::Inherited);
    assert_eq!(inherited_value.defining_class, named("Access").class);
    let override_value = members
        .iter()
        .find(|m| m.class == named("Derived").class && m.name == "computed")
        .unwrap();
    assert_eq!(override_value.origin, MemberOrigin::Source);
    assert_eq!(override_value.defining_class, named("Derived").class);
    let generated = members
        .iter()
        .find(|m| m.class == named("Config").class && m.name == "__init__")
        .unwrap();
    assert_eq!(generated.origin, MemberOrigin::Synthesized);
    let ordinary = members
        .iter()
        .find(|m| m.class == named("Access").class && m.name == "value")
        .unwrap();
    let property = members
        .iter()
        .find(|m| m.class == named("PropertyAccess").class && m.name == "value")
        .unwrap();
    assert_ne!(ordinary.id(), property.id());
    assert_ne!(ordinary.kind, MemberKind::Property);
    assert_eq!(property.kind, MemberKind::Property);

    assert!(
        members.iter().any(|m| m.class == named("Choice").class
            && m.name == "FIRST"
            && m.enum_value.is_some())
    );
}

#[tokio::test]
async fn located_receiver_queries_keep_properties_separate_and_refuse_any_and_wrong_roles() {
    let tables = typed_driver::Tables::default();
    run(&files("class_metadata"), Metadata(tables.clone()))
        .await
        .unwrap();
    let budget = typed_driver::budget();
    let mut qualifications = Rows::new(&budget);
    let mut terms = Rows::new(&budget);
    let mut members = Rows::new(&budget);
    for row in rows::<AssertionQualification>(&tables) {
        qualifications.insert(row).unwrap();
    }
    for row in rows::<TypeTerm>(&tables) {
        terms.insert(row).unwrap();
    }
    for row in rows::<ClassMemberObservation>(&tables) {
        members.insert(row).unwrap();
    }
    let observations = rows::<TypeObservation>(&tables);
    let receiver = observations
        .iter()
        .find(|o| {
            o.role == TypeRole::AttributeBase
                && matches!(terms.get(o.term), Some(TypeTerm::ClassInstance { .. }))
                && matches!(
                    receiver_members(
                        o,
                        o.subject,
                        qualifications.get(o.qualification).unwrap().context,
                        "computed",
                        &qualifications,
                        &terms,
                        &members
                    ),
                    MemberQuery::TypedCandidates(_)
                )
        })
        .unwrap();
    let context = qualifications.get(receiver.qualification).unwrap().context;
    let MemberQuery::TypedCandidates(properties) = receiver_members(
        receiver,
        receiver.subject,
        context,
        "computed",
        &qualifications,
        &terms,
        &members,
    ) else {
        panic!("property candidates missing")
    };
    assert!(
        properties
            .iter()
            .all(|id| members.get(*id).unwrap().kind == MemberKind::Property)
    );
    let MemberQuery::TypedCandidates(fields) = receiver_members(
        receiver,
        receiver.subject,
        context,
        "value",
        &qualifications,
        &terms,
        &members,
    ) else {
        panic!("ordinary member candidates missing")
    };
    assert!(
        fields
            .iter()
            .all(|id| members.get(*id).unwrap().kind != MemberKind::Property)
    );
    let other_source = observations
        .iter()
        .find(|o| o.subject != receiver.subject)
        .unwrap()
        .subject;
    assert_eq!(
        receiver_members(
            receiver,
            other_source,
            context,
            "value",
            &qualifications,
            &terms,
            &members
        ),
        MemberQuery::Unresolved
    );
    let mut expected = receiver.clone();
    expected.role = TypeRole::Expected;
    assert_eq!(
        receiver_members(
            &expected,
            expected.subject,
            context,
            "value",
            &qualifications,
            &terms,
            &members
        ),
        MemberQuery::Unresolved
    );
    let any = observations
        .iter()
        .find(|o| {
            o.role == TypeRole::AttributeBase
                && matches!(terms.get(o.term), Some(TypeTerm::Any { .. }))
        })
        .unwrap();
    assert_eq!(
        receiver_members(
            any,
            any.subject,
            context,
            "value",
            &qualifications,
            &terms,
            &members
        ),
        MemberQuery::Unresolved
    );
    assert!(
        observations
            .iter()
            .any(|o| o.role == TypeRole::AssignmentValue)
    );
    assert!(
        observations
            .iter()
            .filter(|o| o.role == TypeRole::AssignmentValue)
            .any(
                |inferred| observations
                    .iter()
                    .any(|expected| expected.subject == inferred.subject
                        && expected.qualification == inferred.qualification
                        && expected.role == TypeRole::Expected
                        && expected.term != inferred.term
                        && !expected.declared)
            ),
        "a contextual int expectation must not replace the inferred string assignment value"
    );

    assert!(
        observations
            .iter()
            .any(|o| o.role == TypeRole::ReturnExpression)
    );
    assert!(
        rows::<RecordFieldObservation>(&tables)
            .iter()
            .any(|f| f.name.as_str() == "size" && f.default_term.is_some())
    );
    let invariant = lctx_model::domain::validation::invariants_for::<TypeObservation>()
        .into_iter()
        .find(|i| i.name == "selected_type_location_shapes")
        .unwrap();
    let mut forged = receiver.clone();
    forged.subject = rows::<lctx_model::domain::source::Occurrence>(&tables)
        .into_iter()
        .find(|o| o.syntax_kind == lctx_model::domain::source::SyntaxKind::StmtClassDef)
        .unwrap()
        .id();
    let mut checked = (invariant.create)(&budget);
    for input in &invariant.inputs {
        if input.name() == TypeObservation::NAME {
            checked
                .visit(
                    input.name(),
                    &TypeObservation::encode(&[forged.clone()]).unwrap(),
                )
                .unwrap();
        } else {
            let guard = tables.lock().unwrap();
            checked
                .visit(input.name(), guard.get(input.name()).unwrap())
                .unwrap();
        }
    }
    assert!(
        matches!(checked.finish(), Err(ModelError::Invalid(message)) if message.contains("wrong source role or context"))
    );
}
