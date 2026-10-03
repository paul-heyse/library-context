//! Native class metadata controls through the shared source and provider validators.
#[path = "typed_driver/mod.rs"] mod typed_driver;
use lctx_model::domain::{class_metadata::*, calls::*, types::*, assertion::AssertionQualification, normalized::Rows, *};
use typed_driver::{files, rows, run};
inspector!(Metadata, ClassMetadataObservation, ClassMetadataSupport, ClassMemberObservation, ClassMemberSupport, RecordOptions, RecordTransformDefaults, ProviderSymbol, TypeObservation, TypeTerm, AssertionQualification, RecordFieldObservation);
#[tokio::test]
async fn native_class_metadata_keeps_effective_options_and_unknown_abstract_absence() {
    let tables = typed_driver::Tables::default();
    run(&files("class_metadata"), Metadata(tables.clone())).await.unwrap();
    let symbols = rows::<ProviderSymbol>(&tables);
    let metadata = rows::<ClassMetadataObservation>(&tables);
    let named = |name: &str| {
        let class = symbols.iter().find(|s| s.name == name && s.kind == SymbolKind::Class).unwrap().id();
        metadata.iter().find(|m| m.class == class).unwrap()
    };
    let custom = named("Custom");
    assert_eq!(symbols.iter().find(|s| Some(s.id()) == custom.custom_metaclass).unwrap().name, "Meta");
    assert_eq!(custom.slots.as_deref(), Some(["stored".into()].as_slice()));
    let closed = named("Closed");
    assert!(closed.final_declaration);
    assert!(closed.deprecated);
    assert_eq!(closed.deprecation_message.as_deref(), Some("use the typed interface"));
    let reader = named("Reader");
    assert!(reader.protocol && reader.runtime_checkable);
    assert!(reader.protocol_members.contains(&"read".into()));
    assert!(named("Abstract").abstract_members.contains(&"read".into()));
    assert!(named("Concrete").abstract_members.is_empty());
    assert!(metadata.iter().all(|m| !m.abstract_absence_known));
    let config = named("Config");
    let options = rows::<RecordOptions>(&tables);
    let opt = options.iter().find(|r| Some(r.id()) == record_options(config, MetadataBasis::NativeEffective)).unwrap();
    assert!(opt.init && opt.frozen && opt.kw_only && opt.slots);
    assert_eq!(record_options(config, MetadataBasis::SourceDeclaration), None);
    let plain = named("Plain");
    assert!(!plain.protocol && !plain.final_declaration && !plain.enumeration);
    assert_eq!(plain.record_options, None);
    assert!(named("Choice").enumeration);
    assert!(named("RecordBase").transform.is_some());
    assert!(named("Transformed").record_options.is_some());
    let members = rows::<ClassMemberObservation>(&tables);
    let inherited = members.iter().find(|m| m.class == named("Concrete").class && m.name == "read").unwrap();
    assert_eq!(inherited.basis, MetadataBasis::SourceDeclaration);
    assert!(!inherited.abstract_declaration);
    let method = members.iter().find(|m| m.class == named("Abstract").class && m.name == "read").unwrap();
    assert!(method.abstract_declaration);

}

#[tokio::test]
async fn located_receiver_queries_keep_properties_separate_and_refuse_any_and_wrong_roles() {
    let tables = typed_driver::Tables::default();
    run(&files("class_metadata"), Metadata(tables.clone())).await.unwrap();
    let budget = typed_driver::budget();
    let mut qualifications = Rows::new(&budget);
    let mut terms = Rows::new(&budget);
    let mut members = Rows::new(&budget);
    for row in rows::<AssertionQualification>(&tables) { qualifications.insert(row).unwrap(); }
    for row in rows::<TypeTerm>(&tables) { terms.insert(row).unwrap(); }
    for row in rows::<ClassMemberObservation>(&tables) { members.insert(row).unwrap(); }
    let observations = rows::<TypeObservation>(&tables);
    let receiver = observations.iter().find(|o| o.role == TypeRole::AttributeBase
        && matches!(terms.get(o.term), Some(TypeTerm::ClassInstance { .. }))
        && matches!(receiver_members(o, o.subject, qualifications.get(o.qualification).unwrap().context, "computed", &qualifications, &terms, &members), MemberQuery::TypedCandidates(_))).unwrap();
    let context = qualifications.get(receiver.qualification).unwrap().context;
    let MemberQuery::TypedCandidates(properties) = receiver_members(receiver, receiver.subject, context, "computed", &qualifications, &terms, &members) else { panic!("property candidates missing") };
    assert!(properties.iter().all(|id| members.get(*id).unwrap().kind == MemberKind::Property));
    let MemberQuery::TypedCandidates(fields) = receiver_members(receiver, receiver.subject, context, "value", &qualifications, &terms, &members) else { panic!("ordinary member candidates missing") };
    assert!(fields.iter().all(|id| members.get(*id).unwrap().kind != MemberKind::Property));
    let other_source = observations.iter().find(|o| o.subject != receiver.subject).unwrap().subject;
    assert_eq!(receiver_members(receiver, other_source, context, "value", &qualifications, &terms, &members), MemberQuery::Unresolved);
    let mut expected = receiver.clone(); expected.role = TypeRole::Expected;
    assert_eq!(receiver_members(&expected, expected.subject, context, "value", &qualifications, &terms, &members), MemberQuery::Unresolved);
    let any = observations.iter().find(|o| o.role == TypeRole::AttributeBase && matches!(terms.get(o.term), Some(TypeTerm::Any { .. }))).unwrap();
    assert_eq!(receiver_members(any, any.subject, context, "value", &qualifications, &terms, &members), MemberQuery::Unresolved);
    assert!(observations.iter().any(|o| o.role == TypeRole::AssignmentValue));
    assert!(observations.iter().any(|o| o.role == TypeRole::ReturnExpression));
    assert!(rows::<RecordFieldObservation>(&tables).iter().any(|f| f.name.as_str() == "size" && f.default_term.is_some()));
}
