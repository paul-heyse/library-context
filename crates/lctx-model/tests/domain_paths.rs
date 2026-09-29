use std::collections::BTreeMap;
use lctx_model::domain::{Record,place_composition::*,value::*,transfer::TransferKind};
#[test]
fn paths_extend_only_on_the_identity_side_of_composition() {
    let timeout = PathSegment::Attribute { name: "timeout".into() }; let text = PathSegment::Attribute { name: "text".into() };
    let segments = BTreeMap::from([(timeout.id(),timeout.clone()),(text.id(),text.clone())]); let literals = BTreeMap::new();
    let catalog = PathCatalog { segments: &segments,literals: &literals };
    let empty = AccessPath::empty(); let timeout_path = empty.extend(timeout.id()); let input = empty.extend(text.id());
    assert_eq!(compose(&input,&empty,&timeout_path,&empty,TransferKind::Identity,TransferKind::Identity,&catalog).unwrap(),
        ComposedPaths::Flow { input: input.append(&timeout_path),output: empty.clone(),kind: TransferKind::Identity });
    assert_eq!(compose(&input,&empty,&timeout_path,&empty,TransferKind::Derived,TransferKind::Identity,&catalog).unwrap(),
        ComposedPaths::Flow { input: input.clone(),output: empty.clone(),kind: TransferKind::Derived },"parse(s.text) must not invent s.text.timeout");
    assert_eq!(compose(&input,&timeout_path,&empty,&empty,TransferKind::Identity,TransferKind::Identity,&catalog).unwrap(),
        ComposedPaths::Flow { input: input.clone(),output: timeout_path.clone(),kind: TransferKind::Identity });
    assert_eq!(compose(&input,&timeout_path,&empty,&empty,TransferKind::Identity,TransferKind::Derived,&catalog).unwrap(),
        ComposedPaths::Flow { input,output: empty,kind: TransferKind::Derived },"json.dumps(cfg) must not acquire an output.timeout path");
    assert_eq!(relation(&timeout_path,&AccessPath::empty().extend(text.id()),&catalog).unwrap(),PathRelation::Disjoint);
}
#[test]
fn wildcard_unknown_suffix_and_numeric_aliases_do_not_establish_false_disjointness() {
    let values = [Literal::Bool { value: true },Literal::Integer { decimal: "1".into() },Literal::Float { bits: 1.0f64.to_bits() as i64 },
        Literal::Float { bits: f64::NAN.to_bits() as i64 },Literal::Float { bits: 0.0f64.to_bits() as i64 },Literal::Float { bits: (-0.0f64).to_bits() as i64 }];
    let literals = values.iter().map(|row| (row.id(),row.clone())).collect();
    let items: Vec<_> = values.iter().map(|row| PathSegment::Item { key: row.id() }).collect();
    let mut segments: BTreeMap<_,_> = items.iter().map(|row| (row.id(),row.clone())).collect();
    segments.insert(PathSegment::AnyItem.id(),PathSegment::AnyItem);
    let catalog = PathCatalog { segments: &segments,literals: &literals };
    let path = |i: usize| AccessPath::empty().extend(items[i].id());
    assert_eq!(relation(&path(0),&path(1),&catalog).unwrap(),PathRelation::Rest(AccessPath::empty()));
    assert_eq!(relation(&path(1),&path(2),&catalog).unwrap(),PathRelation::Unknown);
    assert_eq!(relation(&path(3),&path(3),&catalog).unwrap(),PathRelation::Unknown);
    assert_eq!(relation(&path(4),&path(5),&catalog).unwrap(),PathRelation::Rest(AccessPath::empty()));
    let wildcard = AccessPath::empty().extend(PathSegment::AnyItem.id());
    assert_eq!(relation(&wildcard,&wildcard,&catalog).unwrap(),PathRelation::Unknown);
    assert_eq!(relation(&path(0),&AccessPath { unknown_suffix: true,..AccessPath::empty() },&catalog).unwrap(),PathRelation::Unknown);
    let mut forged = segments.clone(); forged.insert(items[0].id(),PathSegment::AnyItem);
    assert!(relation(&path(0),&path(0),&PathCatalog { segments: &forged,literals: &literals }).is_err());
}

#[test]
fn access_paths_saturate_with_an_unknown_suffix() {
    let [a, k, z] = [PathSegment::Attribute { name: "a".into() }, PathSegment::Item { key: Literal::String { value: "k".into() }.id() },
        PathSegment::Attribute { name: "z".into() }].map(|segment| segment.id());
    let two = AccessPath::empty().extend(a).extend(k);
    assert!(!two.unknown_suffix);
    let saturated = two.extend(PathSegment::AnyItem.id());
    assert_eq!((saturated.first, saturated.second, saturated.unknown_suffix), (Some(a), Some(k), true), "two explicit segments, then unknown");
    assert_eq!(saturated.extend(z), saturated, "a saturated path stays put");
    assert_eq!(AccessPath::empty().extend(a).append(&saturated), AccessPath { first: Some(a), second: Some(a), unknown_suffix: true });
    assert!(AccessPath { first: None, second: Some(a), unknown_suffix: false }.validate().is_err(), "no second segment without a first");
}

#[test]
fn places_are_identified_by_root_and_path() {
    let input = lctx_model::domain::input::InputRevision::from_entries(vec![]).unwrap();
    let source = lctx_model::domain::source::SourceArtifact::from_bytes(input.id(), "m.py".into(), b"def f(a): pass").unwrap();
    let at = |start: i64| lctx_model::domain::source::Occurrence { source: source.id(), start, end: start + 1,
        syntax_kind: lctx_model::domain::source::SyntaxKind::Parameter, role: lctx_model::domain::source::OccurrenceRole::Parameter, structural_path: vec![0, start as i32] };
    let (parameter, def) = (at(6), at(0));
    let x = AccessPath::empty().extend(PathSegment::Attribute { name: "x".into() }.id());
    let place = |root: PlaceRoot, path: &AccessPath| Place { root: root.id(), path: path.id() }.id();
    let formal = PlaceRoot::Formal { declaration: parameter.id() };
    let module = lctx_model::domain::source::Module { source: source.id(), qualified_name: "m".into() };
    let places = [place(formal.clone(), &AccessPath::empty()), place(formal.clone(), &x), place(PlaceRoot::Entry { declaration: parameter.id() }, &AccessPath::empty()),
        place(PlaceRoot::Return { callable: def.id() }, &AccessPath::empty()), place(PlaceRoot::Field { class: def.id(), name: "x".into() }, &AccessPath::empty()),
        place(PlaceRoot::Global { module: module.id(), name: "x".into() }, &AccessPath::empty())];
    assert_eq!(places.iter().collect::<std::collections::BTreeSet<_>>().len(), places.len(), "the variable, its field, the entry value, the return, a class field and a global are distinct");
    assert_eq!(place(formal.clone(), &x), place(formal, &x));
}
