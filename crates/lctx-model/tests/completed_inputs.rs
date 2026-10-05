use lctx_model::{Domain, domain::{*, analysis::sources::{CapturedSources, CompletedInput}, stages::Profile}};

#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(name = "completed_fixture")]
struct First {
    #[model(key)]
    name: String,
    value: i64,
}
#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(name = "completed_fixture")]
/// An unrelated declaration comment must not alter semantic compatibility.
struct WithDocumentation {
    #[model(key)]
    name: String,
    value: i64,
}
fn budget() -> resources::ResourceBudget {
    resources::ResourceBudget::fixed(16 * 1024 * 1024).unwrap()
}
#[test]
fn declaration_structure_not_documentation_defines_semantic_contract() {
    let first = ValidatedModel::validate(vec![Relation::of::<First>()], ValidationDefinitions::default()).unwrap();
    let commented = ValidatedModel::validate(vec![Relation::of::<WithDocumentation>()], ValidationDefinitions::default()).unwrap();
    assert_eq!(first.digest(), commented.digest());
    assert_eq!(First { name: "one".into(), value: 3 }.id().bytes(), WithDocumentation { name: "one".into(), value: 3 }.id().bytes());
}
#[test]
fn exact_completed_inputs_are_independent_of_physical_store_and_schedule() {
    let budget = budget();
    let input = CompletedInput::<First>::new("producer", ContentHash([1;32]), ContentHash([2;32]), ContentHash([3;32]), 2).unwrap();
    let mut capture = CapturedSources::capture(Profile::Catalog, [input.snapshot()], &budget).unwrap();
    let digest = capture.digest();
    capture.include(&input).unwrap();
    assert_eq!(capture.digest(), digest);
    let changed = CompletedInput::<First>::new("producer", ContentHash([1;32]), ContentHash([4;32]), ContentHash([3;32]), 2).unwrap();
    assert!(capture.include(&changed).is_err());
    let other = CapturedSources::capture(Profile::Catalog, [changed.snapshot()], &budget).unwrap();
    assert_ne!(other.digest(), digest);
    let wire = serde_json::to_value(input.snapshot()).unwrap();
    assert!(wire.get("physical").is_none());
    assert!(wire.get("schedule").is_none());
    assert!(wire.get("prefix").is_none());
    assert!(wire.get("implementation").is_some());
}
#[test]
fn completed_input_rejects_invalid_metadata() {
    assert!(CompletedInput::<First>::new("", ContentHash([1;32]), ContentHash([2;32]), ContentHash([3;32]), 0).is_err());
    assert!(CompletedInput::<First>::new("p", ContentHash([1;32]), ContentHash([2;32]), ContentHash([3;32]), u64::MAX).is_err());
}

#[test]
fn semantic_predecessor_selection_uses_declared_sources_without_runtime_metadata() {
    let model=lctx_model::domain::model().unwrap();
    let mut selectors=0;
    for profile in Profile::ALL {
        let mut inputs=local_semantics::LocalData::consumed_inputs(profile);
        inputs.extend(execution::summary_production::SummaryData::consumed_inputs(profile));
        inputs.extend(analytics::build::Data::consumed_inputs(profile));
        inputs.extend(synthesis::production::Data::consumed_inputs(profile));
        for input in inputs {assert!(model.relation(input.name()).is_some());selectors+=usize::from(input.prefix().is_some());}
    }
    assert!(selectors>0,"earlier semantic vocabulary must be selected explicitly");
}

#[test]
fn normalized_replay_and_production_select_the_same_native_vocabulary(){
 use normalized::*;
 let profile=Profile::Behavioral;
 let owners=[
  (entity_normalization::stage(),entity_normalization::EntityData::validation_inputs()),
  (relation_normalization::stage(profile),relation_normalization::RelationData::validation_inputs()),
  (callable_normalization::stage(profile),callable_normalization::CallableData::validation_inputs()),
  (event_normalization::stage(profile),event_normalization::EventData::validation_inputs()),
  (binding_normalization::stage(profile),binding_normalization::BindingData::validation_inputs()),
 ];
 let mut selected=0;
 for (stage,inputs) in owners {
  for input in inputs.into_iter().filter(|input|stages::is_vocabulary(input.name())){
   let producer=stage.inputs.iter().find(|producer|producer.name()==input.name()).expect("replayed producer source");
   assert_eq!(input.prefix(),Some(stages::PublicationBoundary::Facts),"{} {}",stage.name,input.name());
   assert_eq!(input.prefix(),producer.prefix(),"{} {}",stage.name,input.name());
   selected+=1;
  }
 }
 assert!(selected>0);
}
