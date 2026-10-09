use lctx_model::domain::{ContentHash, Relation, analysis::sources::SourceSnapshot,
    completed::*, input::Package, stages::Profile};
use std::collections::{BTreeMap, BTreeSet};

fn spec(producer:&str)->ContributionSpec {
    ContributionSpec {captured_binding: None, producer:producer.into(),profile:Profile::Catalog,
        model:ContentHash::of(b"model"),implementation:ContentHash::of(b"implementation"),
        configuration:None,inputs:vec![],outputs:BTreeSet::from(["packages".into()])}
}
#[test]
fn completed_empty_is_explicit_and_failed_or_missing_outputs_are_not_completed() {
    let mut contribution=CompletedContribution {spec:spec("provider"),outcome:0,
        outputs:BTreeMap::from([("packages".into(),OutputContent {rows:0,content:ContentHash::of(b"empty")})])};
    let complete=contribution.identity().unwrap();
    contribution.outcome=2;
    assert_ne!(complete,contribution.identity().unwrap());
    contribution.outcome=3;
    assert!(contribution.identity().is_err());
    contribution.outcome=0;contribution.outputs.clear();
    assert!(contribution.identity().is_err());
}
#[test]
fn union_identity_and_source_do_not_depend_on_completion_order() {
    let a=spec("a").identity().unwrap();let b=spec("b").identity().unwrap();
    let relation=Relation::of::<Package>();
    let first=CompletedView::new(relation.name().into(),BTreeSet::from([a,b]),0).unwrap();
    let second=CompletedView::new(relation.name().into(),BTreeSet::from([b,a]),0).unwrap();
    assert_eq!(first,second);
    assert_eq!(SourceSnapshot::of_completed_view(&relation,ContentHash::of(b"model"),&first).unwrap(),
        SourceSnapshot::of_completed_view(&relation,ContentHash::of(b"model"),&second).unwrap());
    let mut forged=first.clone();forged.rows=1;
    assert!(forged.validate().is_err());
}
#[test]
fn binding_validates_exact_view_and_freezes_boundary_identity() {
    let relation=Relation::of::<Package>();
    let view=CompletedView::new(relation.name().into(),BTreeSet::from([spec("a").identity().unwrap()]),0).unwrap();
    let source=SourceSnapshot::of_completed_view(&relation,ContentHash::of(b"model"),&view).unwrap();
    let mut binding=CompletedBinding {boundary:None,source,view,configuration:None};
    binding.validate().unwrap();let current=binding.key();
    binding.boundary=Some("facts".into());binding.validate().unwrap();assert_ne!(current,binding.key());
    binding.view.rows=1;assert!(binding.validate().is_err());
}
