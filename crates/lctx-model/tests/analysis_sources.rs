use lctx_model::{
    Domain,
    domain::{
        analysis::{
            self,
            local::{Invocation, SourceReceipt},
            sources::{CapturedSources,CompletedInput,SourceSnapshot},
        },
        input::Package,
        resources::ResourceBudget,
        stages::*,
        *,
    },
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
fn capture_inputs(model:&ValidatedModel,budget:&ResourceBudget)->(CapturedSources,Vec<SourceSnapshot>){let input=CompletedInput::<Package>::new("producer",model.digest(),ContentHash::of(b"fixture implementation"),ContentHash::of(b"package p"),1).unwrap();let sources=vec![input.snapshot()];let captured=CapturedSources::capture(Profile::Catalog,sources.clone(),budget).unwrap();let mut typed=CapturedSources::new(budget);typed.include(&input).unwrap();assert_eq!(typed.digest(),captured.digest());(captured,sources)}
fn publication(
    inv: &Invocation,
    receipts: &[SourceReceipt],
    sources: &[SourceSnapshot],
    budget: &ResourceBudget,
) -> Result<(), ModelError> {
    let descriptor = lctx_model::domain::validation::publication_checks_for::<Invocation>()
        .into_iter()
        .find(|check| check.name == "local_invocation_sources")
        .expect("canonical source-consistency definition");
    let mut check = (descriptor.create)(budget);
    check.visit(
        Invocation::NAME,
        &Invocation::encode(std::slice::from_ref(inv))?,
    )?;
    check.visit(SourceReceipt::NAME, &SourceReceipt::encode(receipts)?)?;
    check.finish(sources, Profile::Catalog)
}
#[test]
fn capture_binds_exact_completed_sources_and_refuses_coupled_omission() {
    let model = model().unwrap();
    let budget = budget();
    let (captured,sources)=capture_inputs(&model,&budget);
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
    // Source observations reject malformed declared producer metadata.
    let encoded = SourceReceipt::encode(&receipts).unwrap();
    assert_eq!(SourceReceipt::decode(&encoded).unwrap(), receipts);
    let mut columns = encoded.columns().to_vec();
    let prefix = encoded.schema().index_of("producer").unwrap();
    columns[prefix] = std::sync::Arc::new(arrow_array::StringArray::from(vec![""]));
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
    let (retry, _) = capture_inputs(&model,&budget);
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
    assert!(
        ValidatedModel::validate(
            vec![Relation::of::<InventoryControl>()],
            ValidationDefinitions {
                invariants: vec![],
                publication_checks: invalid_inventory()
            }
        )
        .is_err()
    );
}

fn invalid_inventory_refs() -> Vec<&'static str> {
    vec!["unknown_source_control"]
}
