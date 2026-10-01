//! Actual captured documentation roles and native normalized Usage-policy evidence.
#[path = "typed_driver/mod.rs"]
mod typed_driver;
use lctx_model::domain::{
    analysis::usage,
    input::{ArtifactUse, SourceRole},
    normalized::{Rows, entity_normalization, event_normalization::*},
    resources::ResourceBudget,
    *,
};
use typed_driver::{files, rows};
inspector!(Facts, CallTarget);
#[tokio::test]
async fn native_official_doc_sites_keep_alias_targets_and_source_receipts_without_flow() {
    let tables = typed_driver::Tables::default();
    typed_driver::run(&files("direct_usage"), Facts(tables.clone()))
        .await
        .unwrap();
    let budget = ResourceBudget::fixed(256 << 20).unwrap();
    let mut raw = entity_normalization::EntityData::new(&budget);
    macro_rules! facts {($($field:ident:$ty:ty=>$family:ident,)*)=>{$(for row in rows::<$ty>(&tables) {raw.$field.insert(row).unwrap();})*};}
    lctx_model::normalized_entity_inputs!(facts);
    let entities = entity_normalization::normalize(raw.inputs(), &budget).unwrap();
    let mut data = EventData::new(&budget);
    macro_rules! inputs {($($field:ident:$ty:ty,)*)=>{$(for row in rows::<$ty>(&tables) {data.$field.insert(row).unwrap();})*};}
    lctx_model::normalized_event_inputs!(inputs);
    macro_rules! normalized {($($field:ident:$ty:ty,)*)=>{$(data.visit(<$ty>::NAME,&<$ty as Record>::encode(&entities.$field.iter().cloned().collect::<Vec<_>>()).unwrap()).unwrap();)*};}
    lctx_model::normalized_entity_outputs!(normalized);
    let output = normalize(&data, &budget).unwrap();
    let mut uses = Rows::new(&budget);
    for row in rows::<ArtifactUse>(&tables) {
        uses.insert(row).unwrap();
    }
    assert!(uses.iter().any(|u| u.role == SourceRole::DocBlock));
    let symbol = data.symbols.iter().find(|s| s.name == "first").unwrap();
    let first = data
        .symbol_resolutions
        .iter()
        .find(|r| r.symbol == symbol.id())
        .unwrap()
        .entity
        .unwrap();
    let second_symbol = data.symbols.iter().find(|s| s.name == "second").unwrap();
    let second = data
        .symbol_resolutions
        .iter()
        .find(|r| r.symbol == second_symbol.id())
        .unwrap()
        .entity
        .unwrap();
    let input = data.artifacts.iter().next().unwrap().input;
    let inputs = usage::Inputs {
        input,
        context: symbol.context,
        events: &output,
        targets: &data.targets,
        artifacts: &data.artifacts,
        uses: &uses,
        occurrences: &data.occurrences,
        entities: &data.refs,
    };
    let result = inputs.count(&[first, second], &budget).unwrap();
    assert_eq!(
        result
            .scores()
            .iter()
            .find(|s| s.target == first)
            .unwrap()
            .share
            .get(),
        3.0
    );
    assert_eq!(
        result
            .scores()
            .iter()
            .find(|s| s.target == second)
            .unwrap()
            .share
            .get(),
        1.0
    );
    assert_eq!(
        result
            .evidence()
            .iter()
            .map(|e| e.site)
            .collect::<std::collections::BTreeSet<_>>()
            .len(),
        4
    );
    assert!(
        result
            .evidence()
            .iter()
            .all(|e| output.admissions.get(e.admission).is_some()
                && output.alternatives.get(e.alternative).is_some())
    );
    let first_only = inputs.count(&[first], &budget).unwrap();
    assert_eq!(first_only.scores().len(), 1);
    assert_eq!(first_only.scores()[0].share.get(), 3.0);
    drop(first_only);
    drop(result);
    drop(uses);
    drop(output);
    drop(data);
    drop(entities);
    drop(raw);
    assert_eq!(budget.reserved(), 0);
}
