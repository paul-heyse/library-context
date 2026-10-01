//! Complete Local publication envelope; later unscheduled owners are outside this fixture.
use lctx_model::domain::*;
pub fn model()->ValidatedModel {
    let mut relations=normalized_relations();
    relations.extend(analysis::early_relations());
    relations.extend(analysis::dispatch::relations());
    relations.extend(analysis::local::relations());
    relations.extend(transfer::local::relations());
    relations.extend(local_semantics::relations());relations.extend(local_theory::relations());relations.extend(local_fields::relations());
    macro_rules! declared {($($field:ident:$ty:ty,)*)=>{$(relations.push(Relation::of::<$ty>());)*};}
    lctx_model::local_semantic_outputs!(declared);
    relations.sort_by_key(Relation::name);relations.dedup_by_key(|r|r.name());
    ValidatedModel::validate(relations).unwrap()
}
