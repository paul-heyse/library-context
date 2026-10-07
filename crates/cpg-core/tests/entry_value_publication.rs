//! Native entry and stability derive from completed compiler inputs; shared replay rejects forgery.
#[path = "fixtures/catalog_runtime.rs"]
mod catalog_runtime;
use lctx_model::domain::{
    admission::Frontier,
    conditions::{
        entry::{self, *},
        stability::{self, *},
    },
    normalized::entities::*,
    source::*,
    stages::Profile,
    value::*,
    *,
};
async fn run(forged: bool) {
    let fixture = catalog_runtime::compile(
        "entry_value_witnesses",
        Profile::Behavioral,
        Frontier::Normalized,
        catalog_runtime::settings("cases"),
        None,
    )
    .await;
    let budget = fixture.workspace.budget();
    let bytes =
        std::fs::read(catalog_runtime::root("entry_value_witnesses").join("cases.py")).unwrap();
    let mut data = EntryData::new(budget);
    macro_rules! load {($($field:ident:$ty:ty,)*)=>{$(for batch in fixture.workspace.completed::<$ty>().unwrap().batches().unwrap(){data.$field.decode(&batch.unwrap()).unwrap();})*};}
    lctx_model::entry_value_inputs!(load);
    let text = |id: Id<Occurrence>| {
        let o = data.occurrences.get(id).unwrap();
        std::str::from_utf8(&bytes[o.start as usize..o.end as usize]).unwrap()
    };
    let callable=data.callables.iter().find(|c|matches!(c,CallableEntity::Source{declaration,..} if text(*declaration).starts_with("def parameter("))).unwrap();
    let owner = EntityRef::Callable {
        callable: callable.id(),
    };
    let read = data
        .uses
        .iter()
        .find(|u| {
            data.owners
                .iter()
                .any(|o| o.occurrence == u.occurrence && o.entity == owner.id())
                && data.occurrences.iter().any(|e| {
                    e.syntax_kind == SyntaxKind::ExprCompare
                        && data
                            .occurrences
                            .get(u.occurrence)
                            .unwrap()
                            .structural_path
                            .len()
                            > e.structural_path.len()
                        && data
                            .occurrences
                            .get(u.occurrence)
                            .unwrap()
                            .structural_path
                            .starts_with(&e.structural_path)
                })
        })
        .unwrap();
    let PlaceRoot::Formal { declaration } = data
        .roots
        .get(data.places.get(read.place).unwrap().root)
        .unwrap()
    else {
        panic!("formal")
    };
    let formal = ParameterEntity::Source {
        declaration: *declaration,
    };
    let support = data
        .use_observations
        .iter()
        .filter(|o| o.use_ == read.id())
        .find_map(|o| data.use_supports.iter().find(|s| s.assertion == o.id()))
        .unwrap();
    let run = data.runs.get(support.run).unwrap();
    let request = EntryRequest {
        owner: owner.id(),
        formal: formal.id(),
        access: read.occurrence,
        context: run.context,
        run: run.id(),
    };
    let _use_entry = EntryValueWitness::derive(&data, request, budget)
        .unwrap()
        .unwrap();
    let occurrence = data.occurrences.get(request.access).unwrap();
    let atom = data
        .atoms
        .iter()
        .find(|a| {
            data.predicates.get(a.predicate) == Some(&Predicate::IsNone)
                && data.occurrences.get(a.evaluation).is_some_and(|e| {
                    occurrence.structural_path.len() > e.structural_path.len()
                        && occurrence.structural_path.starts_with(&e.structural_path)
                })
        })
        .unwrap();
    let leaf = data
        .leaves
        .iter()
        .find(|leaf| leaf.atom == atom.id())
        .unwrap();
    let support = data
        .leaf_supports
        .iter()
        .find(|support| support.assertion == leaf.id())
        .unwrap();
    let source = EntryAccessSource::guard(&data, request, leaf.id(), support.id()).unwrap();
    let entry = EntryValueWitness::derive_for(&data, request, &source, budget)
        .unwrap()
        .unwrap();
    let stability = StabilityWitness::derive(&data, atom.id(), &entry).unwrap();
    let mut row = entry.witness().clone();
    if forged {
        row.use_observation = data
            .use_observations
            .iter()
            .find(|o| o.id() != row.use_observation)
            .unwrap()
            .id();
    }

    let invariant = entry::entry_invariants()
        .into_iter()
        .find(|i| i.name == "entry_value_witness_replay")
        .unwrap();
    let mut check = (invariant.create)(budget);
    for input in &invariant.inputs {
        if input.name() == EntryValueWitness::NAME {
            check
                .visit(
                    input.name(),
                    &EntryValueWitness::encode(&[row.clone()]).unwrap(),
                )
                .unwrap();
        } else if input.name() == EntryAccessSource::NAME {
            check
                .visit(
                    input.name(),
                    &<EntryAccessSource as Record>::encode(&[entry.source().clone()]).unwrap(),
                )
                .unwrap();
        } else {
            for batch in fixture
                .workspace
                .relation(input.name())
                .unwrap()
                .batches()
                .unwrap()
            {
                check.visit(input.name(), &batch.unwrap()).unwrap();
            }
        }
    }
    let result = check.finish();
    if forged {
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("stored entry witness differs from replay")
        );
    } else {
        result.unwrap();
        let invariant = stability::stability_invariants()
            .into_iter()
            .next()
            .unwrap();
        let mut check = (invariant.create)(budget);
        for input in &invariant.inputs {
            if input.name() == EntryValueWitness::NAME {
                check
                    .visit(
                        input.name(),
                        &EntryValueWitness::encode(&[row.clone()]).unwrap(),
                    )
                    .unwrap();
            } else if input.name() == EntryAccessSource::NAME {
                check
                    .visit(
                        input.name(),
                        &<EntryAccessSource as Record>::encode(&[entry.source().clone()]).unwrap(),
                    )
                    .unwrap();
            } else if input.name() == StabilityWitness::NAME {
                check
                    .visit(
                        input.name(),
                        &StabilityWitness::encode(&[stability.witness().clone()]).unwrap(),
                    )
                    .unwrap();
            } else {
                for batch in fixture
                    .workspace
                    .relation(input.name())
                    .unwrap()
                    .batches()
                    .unwrap()
                {
                    check.visit(input.name(), &batch.unwrap()).unwrap();
                }
            }
        }
        check.finish().unwrap();
    }
}
#[tokio::test]
async fn local_entry_and_stability_derive_from_native_completed_sources() {
    run(false).await;
}
#[tokio::test]
async fn local_replay_refuses_forged_existing_native_entry_premise() {
    run(true).await;
}

#[path = "fixtures/native.rs"]
mod native_fixture;
