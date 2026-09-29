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
