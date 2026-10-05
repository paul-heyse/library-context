use lctx_model::{
    Domain,
    domain::{
        analysis::{
            self,
            local::{Invocation, SourceReceipt},
            sources::CapturedSources,
        },
        input::{Package, Release},
        memory::MemoryGeneration,
        resources::ResourceBudget,
        stages::*,
        *,
    },
};
use std::{
    future::Future,
    task::{Context, Poll, Waker},
};
fn budget() -> ResourceBudget {
    ResourceBudget::fixed(32 << 20).unwrap()
}
fn nominal<T>(v: u8) -> Id<T> {
    serde::Deserialize::deserialize(serde::de::value::SeqDeserializer::<
        _,
        serde::de::value::Error,
    >::new([v; 16].into_iter()))
    .unwrap()
}
fn ready<T>(future: impl Future<Output = T>) -> T {
    match std::pin::pin!(future)
        .as_mut()
        .poll(&mut Context::from_waker(Waker::noop()))
    {
        Poll::Ready(value) => value,
        Poll::Pending => panic!("memory effect pending"),
    }
}
fn stage(name: &'static str, inputs: Vec<RelationUse>, outputs: Vec<RelationUse>) -> Stage {
    Stage {
        name,
        inputs,
        outputs,
        contributes: vec![],
        coverage: vec![],
        profiles: vec![Profile::Catalog],
        effect: Effect::Pure,
        code: ContentHash::of(b"capture-control"),
        configuration: ContentHash::of(b"capture-control"),
    }
}
fn capture_attempt(
    schedule: &Schedule,
    model: &ValidatedModel,
    budget: &ResourceBudget,
) -> (CapturedSources, Vec<CompletedRelation>) {
    let mut execution = schedule.execute();
    let sink = MemoryGeneration::bind(model, budget, &mut execution).unwrap();
    let mut producer = execution.begin("producer").unwrap();
    let batch = Batch::new(model, vec![Package { name: "p".into() }], budget).unwrap();
    ready(producer.write::<Package, _>(async |permit| sink.copy(permit, &batch).await)).unwrap();
    ready(producer.complete(&sink, ProviderOutcome::Complete)).unwrap();
    let consumer = execution.begin("consumer").unwrap();
    let sources = consumer.completed_sources().unwrap();
    let captured = CapturedSources::capture(&consumer, budget).unwrap();
    let mut typed = CapturedSources::new(budget);
    typed.include(&consumer.read::<Package>().unwrap()).unwrap();
    assert_eq!(typed.digest(), captured.digest());
    (captured, sources)
}
fn publication(
    inv: &Invocation,
    receipts: &[SourceReceipt],
    sources: &[CompletedRelation],
    budget: &ResourceBudget,
) -> Result<(), ModelError> {
    let descriptor = lctx_model::domain::validation::publication_checks_for::<Invocation>().remove(0);
    let mut check = (descriptor.create)(budget);
    check.visit(
        Invocation::NAME,
        &Invocation::encode(std::slice::from_ref(inv))?,
    )?;
    check.visit(SourceReceipt::NAME, &SourceReceipt::encode(receipts)?)?;
    let snapshots = sources.iter().map(analysis::sources::SourceSnapshot::from_source).collect::<Result<Vec<_>, _>>()?;
    check.finish(&snapshots, Profile::Catalog)
}
#[test]
fn capture_binds_exact_sealed_sources_and_refuses_coupled_omission() {
    let model = model().unwrap();
    let budget = budget();
    let schedule = Schedule::build(
        &model,
        vec![
            stage("producer", vec![], vec![RelationUse::of::<Package>()]),
            stage(
                "consumer",
                vec![RelationUse::stored::<Package>()],
                vec![RelationUse::of::<Release>()],
            ),
        ],
        &[],
        Profile::Catalog,
    )
    .unwrap();
    let (captured, sources) = capture_attempt(&schedule, &model, &budget);
    let (inv, _, receipts, _) = Invocation::admitted(
        nominal(1),
        nominal(2),
        nominal(3),
        None,
        [],
        &captured,
        [],
        &budget,
    )
    .unwrap();
    assert_eq!(receipts[0].source().rows(), 1);
    publication(&inv, &receipts, &sources, &budget).unwrap();
    // Decode must reject an unknown persisted boundary before receipts enter replay.
    let encoded = SourceReceipt::encode(&receipts).unwrap();
    assert_eq!(SourceReceipt::decode(&encoded).unwrap(), receipts);
    let mut columns = encoded.columns().to_vec();
    let prefix = encoded.schema().index_of("prefix").unwrap();
    columns[prefix] = std::sync::Arc::new(arrow_array::StringArray::from(vec![Some("Unknown")]));
    let malformed = arrow_array::RecordBatch::try_new(encoded.schema(), columns).unwrap();
    assert!(
        matches!(SourceReceipt::decode(&malformed), Err(ModelError::Invalid(message))
        if message.contains("source receipt has invalid metadata"))
    );
    assert!(publication(&inv, &[], &sources, &budget).is_err());
    let empty = CapturedSources::new(&budget);
    let (forged, _, no_receipts, _) = Invocation::admitted(
        nominal(1),
        nominal(2),
        nominal(3),
        None,
        [],
        &empty,
        [],
        &budget,
    )
    .unwrap();
    assert!(publication(&forged, &no_receipts, &sources, &budget).is_err());
    assert!(publication(&inv, &receipts, &[], &budget).is_err());
    let (retry, _) = capture_attempt(&schedule, &model, &budget);
    assert_eq!(
        retry.digest(),
        captured.digest(),
        "semantic source identity excludes attempt nonce"
    );
    let (retry_inv, _, _, _) = Invocation::admitted(
        nominal(1),
        nominal(2),
        nominal(3),
        None,
        [],
        &retry,
        [],
        &budget,
    )
    .unwrap();
    assert_eq!(retry_inv.id(), inv.id());
    let tiny = ResourceBudget::fixed(1).unwrap();
    assert!(
        Invocation::admitted(
            nominal(1),
            nominal(2),
            nominal(3),
            None,
            [],
            &captured,
            [],
            &tiny
        )
        .is_err()
    );
    assert_eq!(tiny.reserved(), 0);
}
#[test]
fn projection_meaning_is_early_exact_and_identity_bearing() {
    use lctx_model::domain::projection::ProjectionName;
    let budget = budget();
    let empty = CapturedSources::new(&budget);
    let projection = analysis::ProjectionDefinition::builtin(ProjectionName::CallableInvocation);
    projection.validate().unwrap();
    let mut forged = projection.clone();
    forged.policy = ContentHash::of(b"producer-policy");
    assert!(forged.validate().is_err());
    let (base, _, _, _) = Invocation::admitted(
        nominal(1),
        nominal(2),
        nominal(3),
        None,
        [],
        &empty,
        [],
        &budget,
    )
    .unwrap();
    let (with, _, _, members) = Invocation::admitted(
        nominal(1),
        nominal(2),
        nominal(3),
        None,
        [],
        &empty,
        [projection.id()],
        &budget,
    )
    .unwrap();
    assert_ne!(base.id(), with.id());
    assert_eq!(members.len(), 1);
    assert!(
        Invocation::admitted(
            nominal(1),
            nominal(2),
            nominal(3),
            None,
            [],
            &empty,
            [projection.id(), projection.id()],
            &budget
        )
        .is_err()
    );
}
#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(name="publication_inventory_control",publication_refs=invalid_inventory_refs,semantic_source=include_bytes!("analysis_sources.rs"))]
struct InventoryControl {
    #[model(key)]
    value: i64,
}
fn invalid_inventory() -> Vec<PublicationInvariant> {
    vec![PublicationInvariant {
        revision: 1,
        name: "unknown_source_control",
        inputs: vec![ValidationInput::of::<Package>(&["id"])],
        create: std::sync::Arc::new(|_| {
            panic!("invalid inventory must be refused before callback")
        }),
    }]
}
#[test]
fn model_refuses_unknown_publication_input_before_activation() {
    assert!(ValidatedModel::validate(vec![Relation::of::<InventoryControl>()], ValidationDefinitions { invariants: vec![], publication_checks: invalid_inventory() }).is_err());
}

fn invalid_inventory_refs() -> Vec<&'static str> {vec!["unknown_source_control"]}
