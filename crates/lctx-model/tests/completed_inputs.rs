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
