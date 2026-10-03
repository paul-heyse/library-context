//! Captured native twins challenge the normalized/Local/uncertain Summary ownership boundary.
#[path = "fixtures/transfer_composition.rs"]
mod fixture;
use lctx_model::domain::{
    analysis,
    execution::summary_production::SummaryData,
    local_semantics,
    normalized::{self, callable_aspects::*},
    *,
};

fn retain<R: Record>(
    rows: &mut normalized::Rows<R>,
    budget: &resources::ResourceBudget,
    keep: impl Fn(&R) -> bool,
) {
    let retained = rows.iter().filter(|r| keep(r)).cloned().collect::<Vec<_>>();
    *rows = normalized::Rows::new(budget);
    for row in retained {
        rows.insert(row).unwrap();
    }
}

fn aspects(f: &fixture::NativeFixture) -> AspectOutput {
    let mut d = AspectData::new(&f.budget);
    macro_rules! raw{($($field:ident:$ty:ty,)*)=>{$(for row in f.rows::<$ty>(){d.$field.insert(row).unwrap();})*};}
    lctx_model::callable_aspect_inputs!(raw);
    macro_rules! normalized{($($field:ident:$ty:ty,)*)=>{$(d.visit(<$ty>::NAME,&<$ty as Record>::encode(&f.data.$field.iter().cloned().collect::<Vec<_>>()).unwrap()).unwrap();)*};}
    lctx_model::normalized_binding_inputs!(normalized);
    let mut entities = normalized::entity_normalization::EntityData::new(&f.budget);
    macro_rules! facts{($($field:ident:$ty:ty=>$family:ident,)*)=>{$(for row in f.rows::<$ty>(){entities.$field.insert(row).unwrap();})*};}
    lctx_model::normalized_entity_inputs!(facts);
    let entities =
        normalized::entity_normalization::normalize(entities.inputs(), &f.budget).unwrap();
    macro_rules! entities{($($field:ident:$ty:ty,)*)=>{$(d.visit(<$ty>::NAME,&<$ty as Record>::encode(&entities.$field.iter().cloned().collect::<Vec<_>>()).unwrap()).unwrap();)*};}
    lctx_model::normalized_entity_outputs!(entities);
    normalize(&d, &f.budget).unwrap()
}

async fn native_store_and_readers(compose: bool) {
    let f = fixture::native_from("transfer_alternatives").await;
    let aspects = aspects(&f);
    let mut local = local_semantics::LocalData::new(&f.budget);
    let mut summary = SummaryData::new(&f.budget);
    macro_rules! feed {
        ($ty:ty,$rows:expr) => {{
            let b = <$ty as Record>::encode(($rows).as_ref()).unwrap();
            local.visit(<$ty>::NAME, &b).unwrap();
            summary.visit(<$ty>::NAME, &b).unwrap();
        }};
    }
    for (name, batch) in f.tables.lock().unwrap().iter() {
        local.visit(name, batch).unwrap();
        summary.visit(name, batch).unwrap();
    }
    macro_rules! normalized{($($field:ident:$ty:ty,)*)=>{$(feed!($ty,f.data.$field.iter().cloned().collect::<Vec<_>>());)*};}
    lctx_model::normalized_binding_inputs!(normalized);
    macro_rules! aspect_rows{($($field:ident:$ty:ty,)*)=>{$(feed!($ty,aspects.$field.iter().cloned().collect::<Vec<_>>());)*};}
    lctx_model::callable_aspect_outputs!(aspect_rows);
    let mut inventory = analysis::native::NativeInventory::new(&f.budget);
    for (name, batch) in f.tables.lock().unwrap().iter() {
        if analysis::native::NativeInventory::inputs()
            .iter()
            .any(|i| i.name() == *name)
        {
            inventory.visit(name, batch).unwrap();
        }
    }
    let inventory = inventory.collect().unwrap();
    feed!(
        analysis::native::NativeQualification,
        inventory.qualifications.iter().cloned().collect::<Vec<_>>()
    );
    let input = f.rows::<input::InputRevision>()[0].id();
    let context = f.data.event_events.iter().next().unwrap().context;
    let (_, definition) = local_semantics::definition();
    let (invocation, _) =
        analysis::local::AnalysisInvocation::new(input, context, definition.id(), None, []);
    feed!(
        analysis::local::AnalysisInvocation,
        std::slice::from_ref(&invocation)
    );
    let rows = local_semantics::produce(&local, &invocation, &definition, &f.budget).unwrap();
    let record = aspects
        .symbolic_classes
        .iter()
        .find(|c| {
            f.rows::<symbols::ClassTraitObservation>().iter().any(|t| {
                t.id() == c.traits
                    && f.data
                        .symbols
                        .get(t.symbol)
                        .is_some_and(|s| s.name == "RecordHolder")
            })
        })
        .unwrap();
    assert!(record.supported_record);
    let store = aspects
        .symbolic_stores
        .iter()
        .find(|s| s.class == record.class)
        .unwrap();
    let receiver = local
        .fields
        .symbolic_parameter_syntax
        .get(store.receiver)
        .unwrap();
    let wrapper = local.entry.occurrences.get(receiver.parameter).unwrap();
    assert_eq!(
        (wrapper.syntax_kind, wrapper.role),
        (
            source::SyntaxKind::ParameterWithDefault,
            source::OccurrenceRole::Syntax
        )
    );
    let receipts = rows
        .fields
        .symbolic_stores
        .iter()
        .filter(|s| s.store == store.id())
        .collect::<Vec<_>>();
    assert_eq!(
        receipts.len(),
        1,
        "unchanged store needs exact value and receiver Entry proofs"
    );
    let receipt = receipts[0];
    let value_entry = rows.entries.get(receipt.value_entry).unwrap();
    let receiver_entry = rows.entries.get(receipt.receiver_entry).unwrap();
    assert_eq!(value_entry.access, store.value);
    let normalized::entities::ParameterEntity::Source {
        declaration: receiver_formal,
    } = local.entry.formals.get(receiver_entry.formal).unwrap()
    else {
        panic!("receiver needs a source formal")
    };
    assert_ne!(*receiver_formal, receiver.parameter);
    assert_eq!(
        local
            .entry
            .occurrences
            .get(*receiver_formal)
            .unwrap()
            .syntax_kind,
        source::SyntaxKind::Parameter
    );
    assert!(
        local
            .entry
            .placements
            .iter()
            .any(|p| p.parent == Some(receiver.parameter)
                && p.occurrence == *receiver_formal
                && p.field == lexical::SyntaxField::Child
                && p.ordinal == 0)
    );
    let source_value = local.entry.values.get(receipt.value).unwrap();
    assert_eq!(
        (
            source_value.sink,
            source_value.kind,
            source_value.transfer,
            source_value.through_call
        ),
        (
            store.value,
            flow::FlowSinkKind::Definition,
            transfer::TransferKind::Identity,
            false
        )
    );
    let source_support = local.entry.value_supports.get(receipt.support).unwrap();
    // Each Entry proof is independently necessary, as is the wrapper-to-formal source edge.
    for mutation in 0..5 {
        let mut entry = conditions::entry::EntryData::new(&f.budget);
        macro_rules! copy{($($field:ident:$ty:ty,)*)=>{$(for row in local.entry.$field.iter(){entry.$field.insert(row.clone()).unwrap();})*};}
        lctx_model::entry_value_inputs!(copy);
        match mutation {
            0 => retain(&mut entry.use_supports, &f.budget, |s| {
                s.assertion != value_entry.use_observation
            }),
            1 => retain(&mut entry.use_supports, &f.budget, |s| {
                s.assertion != receiver_entry.use_observation
            }),
            2 => retain(&mut entry.definition_supports, &f.budget, |s| {
                s.assertion != value_entry.definition
            }),
            3 => retain(&mut entry.definition_supports, &f.budget, |s| {
                s.assertion != receiver_entry.definition
            }),
            _ => {
                let child = entry
                    .placements
                    .iter()
                    .filter(|p| {
                        p.parent == Some(receiver.parameter) && p.occurrence == *receiver_formal
                    })
                    .map(Record::id)
                    .collect::<Vec<_>>();
                retain(&mut entry.placement_supports, &f.budget, |s| {
                    !child.contains(&s.assertion)
                });
            }
        }
        let field_data = local_fields::FieldData {
            entry: &entry,
            theory: &local.theory,
            inventory: &local.fields,
        };
        assert!(
            local_symbolic::derive(&field_data, &invocation, store, source_support, &f.budget)
                .unwrap()
                .is_err(),
            "native Entry/source premise removal {mutation}"
        );
    }
    if !compose {
        return;
    }
    feed!(
        local_symbolic::SymbolicFieldStore,
        rows.fields
            .symbolic_stores
            .iter()
            .cloned()
            .collect::<Vec<_>>()
    );
    let catalog = models::Catalog::committed().unwrap();
    let (_, definition) = execution::configuration::summaries(
        catalog.declaration().id(),
        execution::configuration::SummaryLimits {
            depth: 2,
            ..Default::default()
        },
    )
    .unwrap();
    let (invocation, _) =
        analysis::summary::AnalysisInvocation::new(input, context, definition.id(), None, []);
    let alternatives =
        execution::summary_symbolic::derive(&summary, &invocation, &f.budget).unwrap();
    let selected = alternatives
        .iter()
        .filter(|a| {
            summary.symbolic_links.get(a.link).is_some_and(|l| {
                summary
                    .symbolic_associations
                    .get(l.association)
                    .is_some_and(|a| a.class == record.id())
            })
        })
        .collect::<Vec<_>>();
    assert_eq!(
        selected.len(),
        4,
        "opaque argument and return are separate native value observations"
    );
    let mut accesses = selected
        .iter()
        .map(|a| {
            let link = summary.symbolic_links.get(a.link).unwrap();
            summary.symbolic_readers.get(link.reader).unwrap().access
        })
        .collect::<std::collections::BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    accesses.sort_by_key(|access| summary.entry.occurrences.get(*access).unwrap().start);
    assert_eq!(accesses.len(), 3);
    let expected = [
        vec![(
            flow::FlowSinkKind::Return,
            transfer::TransferKind::Identity,
            false,
        )],
        vec![(
            flow::FlowSinkKind::Return,
            transfer::TransferKind::Derived,
            false,
        )],
        vec![
            (
                flow::FlowSinkKind::Argument,
                transfer::TransferKind::Identity,
                false,
            ),
            (
                flow::FlowSinkKind::Return,
                transfer::TransferKind::Derived,
                true,
            ),
        ],
    ];
    let mut use_conditions = std::collections::BTreeSet::new();
    for (access, expected) in accesses.iter().zip(expected) {
        let values = selected
            .iter()
            .filter(|a| {
                let value = summary.entry.values.get(a.value).unwrap();
                summary.entry.uses.get(value.use_).unwrap().occurrence == *access
            })
            .collect::<Vec<_>>();
        let mut matrix = values
            .iter()
            .map(|a| (a.kind, a.transfer, a.through_call))
            .collect::<Vec<_>>();
        matrix.sort_by_key(|(kind, transfer, through)| (*kind as i16, *transfer as i16, *through));
        assert_eq!(matrix, expected);
        let value = summary.entry.values.get(values[0].value).unwrap();
        let observations = summary
            .entry
            .use_observations
            .iter()
            .filter(|o| o.use_ == value.use_)
            .collect::<Vec<_>>();
        assert_eq!(observations.len(), 1);
        use_conditions.insert(
            summary
                .entry
                .qualifications
                .get(observations[0].qualification)
                .unwrap()
                .condition,
        );
    }
    assert_eq!(
        use_conditions,
        std::collections::BTreeSet::from([conditions::Diagram::always().id()]),
        "native use observations are structural; path guards belong to their value observations"
    );
    let conditions = selected
        .iter()
        .map(|a| {
            summary
                .entry
                .qualifications
                .get(a.reader_qualification)
                .unwrap()
                .condition
        })
        .collect::<std::collections::BTreeSet<_>>();
    assert_eq!(
        conditions.len(),
        4,
        "the opaque return has its own native value qualification"
    );
    for a in &selected {
        let value = summary.entry.values.get(a.value).unwrap();
        assert_eq!(a.reader_qualification, value.qualification);
        assert_eq!(
            (a.sink, a.kind, a.transfer, a.through_call),
            (value.sink, value.kind, value.transfer, value.through_call)
        );
        assert_eq!(
            summary
                .entry
                .value_supports
                .get(a.support)
                .unwrap()
                .assertion,
            value.id()
        );
        if a.kind == flow::FlowSinkKind::Argument {
            assert_eq!(
                a.sink,
                summary.entry.uses.get(value.use_).unwrap().occurrence
            );
        } else {
            assert_eq!(
                summary.entry.occurrences.get(a.sink).unwrap().syntax_kind,
                source::SyntaxKind::ExprIf
            );
        }
        assert_eq!(
            a.constructor, receipt.constructor,
            "constructor is the callable body owner"
        );
        assert_eq!(a.depth, 2);
        assert_eq!(a.reason, obligation::ObligationKind::ScopeBoundary);
        assert_eq!(a.store, Some(receipt.id()));
        assert_ne!(a.constructor, a.reader);
        let rq = summary
            .entry
            .qualifications
            .get(a.reader_qualification)
            .unwrap();
        let cq = summary
            .entry
            .qualifications
            .get(a.constructor_qualification)
            .unwrap();
        assert_eq!(rq.scope, cq.scope, "same artifact scope must be preserved");
        // Qualification equality with the exact native value above preserves each row's guard.
        // This opaque argument is natively unguarded; its separate return carries a path guard.
        if a.kind == flow::FlowSinkKind::Argument {
            assert_eq!(
                rq.condition,
                conditions::Diagram::always().id(),
                "opaque argument keeps its original unguarded native value qualification"
            );
        }
    }
    assert!(
        alternatives.iter().all(
            |a| summary.symbolic_links.get(a.link).is_some_and(|l| summary
                .symbolic_associations
                .get(l.association)
                .is_some_and(|a| summary
                    .symbolic_classes
                    .get(a.class)
                    .is_some_and(|c| c.supported_record)))
        )
    );
    // Removing a completed Local premise preserves source association but changes the refusal.
    summary.symbolic_local_stores = normalized::Rows::new(&f.budget);
    let refused = execution::summary_symbolic::derive(&summary, &invocation, &f.budget).unwrap();
    assert!(
        refused
            .iter()
            .all(|a| a.reason == obligation::ObligationKind::MissingEvidence)
    );
}

#[tokio::test]
async fn native_local_store_requires_both_entry_proofs_and_receiver_source_edge() {
    native_store_and_readers(false).await;
}

#[tokio::test]
async fn exact_native_store_keeps_three_readers_and_four_native_values_without_finite_authority() {
    native_store_and_readers(true).await;
}
