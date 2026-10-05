//! Independent controls for canonical definitions, complete premises and exact replay requirements.
use lctx_model::{
    Domain,
    domain::{
        execution::source_call_records::*, memory::MemoryGeneration, resources::ResourceBudget,
        stages::*, *,
    },
};
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(name="validation_premises", semantic_source=include_bytes!("validation_definitions.rs"))]
struct Premise {
    #[model(key)]
    ordinal: i64,
    accepted: bool,
}
#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(name="validation_left", invariant_refs=shared_refs, semantic_source=include_bytes!("validation_definitions.rs"))]
struct Left {
    #[model(key)]
    ordinal: i64,
}
#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(name="validation_right", invariant_refs=shared_refs, semantic_source=include_bytes!("validation_definitions.rs"))]
struct Right {
    #[model(key)]
    ordinal: i64,
}
#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(name="validation_checkpoint_only", invariant_refs=checkpoint_only_refs, semantic_source=include_bytes!("validation_definitions.rs"))]
struct CheckpointOnly {
    #[model(key)]
    ordinal: i64,
}
fn checkpoint_only_refs() -> Vec<&'static str> {
    vec!["checkpoint_only_rule"]
}
fn shared_refs() -> Vec<&'static str> {
    vec!["accepted_validation_premise"]
}
struct Accept;
impl InvariantCheck for Accept {
    fn visit(
        &mut self,
        relation: &str,
        batch: &arrow_array::RecordBatch,
    ) -> Result<(), ModelError> {
        assert_eq!(relation, Premise::NAME);
        if Premise::decode(batch)?.iter().any(|p| !p.accepted) {
            return Err(ModelError::Invalid("authored rejected premise".into()));
        }
        Ok(())
    }
    fn finish(self: Box<Self>) -> Result<(), ModelError> {
        Ok(())
    }
}
fn shared(revision: u32, executions: Arc<AtomicUsize>) -> ValidationDefinitions {
    ValidationDefinitions {
        invariants: vec![Invariant {
            name: "accepted_validation_premise",
            revision,
            inputs: vec![ValidationInput::of::<Premise>(&["id"])],
            create: Arc::new(move |_| {
                executions.fetch_add(1, Ordering::Relaxed);
                Box::new(Accept)
            }),
        }],
        publication_checks: vec![],
    }
}
fn budget() -> ResourceBudget {
    ResourceBudget::fixed(8 << 20).unwrap()
}
fn stage(inputs: Vec<RelationUse>) -> Stage {
    Stage {
        name: "read_validation",
        inputs,
        outputs: vec![],
        contributes: vec![],
        coverage: vec![],
        profiles: vec![Profile::Catalog],
        effect: Effect::Pure,
        code: ContentHash::of(b"known-control"),
        configuration: ContentHash::of(b"known-control"),
    }
}
#[test]
fn any_referrer_retains_the_check_and_missing_definition_or_premise_refuses() {
    let count = Arc::new(AtomicUsize::new(0));
    for referring in [Relation::of::<Left>(), Relation::of::<Right>()] {
        let declarations = vec![referring.clone(), Relation::of::<Premise>()];
        assert!(
            ValidatedModel::validate(declarations.clone(), ValidationDefinitions::default())
                .is_err()
        );
        assert!(ValidatedModel::validate(vec![referring], shared(1, count.clone())).is_err());
        let model = ValidatedModel::validate(declarations, shared(1, count.clone())).unwrap();
        assert_eq!(model.invariants().len(), 1);
        let use_ = RelationUse::of_relation(
            model
                .relations()
                .iter()
                .find(|r| r.name() != Premise::NAME)
                .unwrap(),
        )
        .completed_store();
        assert_eq!(stage(vec![use_]).read_invariants(&model).unwrap().len(), 1);
        assert!(
            model
                .invariants_for_scope(&[use_.name()].into_iter().collect())
                .is_err()
        );
    }
    assert_eq!(
        count.load(Ordering::Relaxed),
        0,
        "model admission must never execute semantic callbacks"
    );
}
#[test]
fn acknowledged_premises_complete_inputs_without_selecting_their_checks() {
    let count = Arc::new(AtomicUsize::new(0));
    let mut definitions = shared(1, count.clone());
    let mut checkpoint_only = definitions.invariants[0].clone();
    checkpoint_only.name = "checkpoint_only_rule";
    definitions.invariants.push(checkpoint_only);
    let model = ValidatedModel::validate(
        vec![
            Relation::of::<Left>(),
            Relation::of::<Premise>(),
            Relation::of::<CheckpointOnly>(),
        ],
        definitions,
    )
    .unwrap();
    let referrers = [Left::NAME].into_iter().collect();
    let premises = [Premise::NAME, CheckpointOnly::NAME].into_iter().collect();
    assert!(model.invariants_for_scope(&referrers).is_err());
    assert!(
        model
            .invariants_for_scope_with_premises(
                &referrers,
                &[CheckpointOnly::NAME].into_iter().collect(),
            )
            .is_err()
    );
    let checks = model
        .invariants_for_scope_with_premises(&referrers, &premises)
        .unwrap();
    assert_eq!(checks.len(), 1);
    assert_eq!(checks[0].name, "accepted_validation_premise");
    assert_eq!(
        checks[0].digest(),
        model.invariant(checks[0].name).unwrap().digest()
    );
    assert!(
        model
            .invariants_for_scope_with_premises(
                &referrers,
                &[Premise::NAME, "undeclared_relation"].into_iter().collect(),
            )
            .is_err()
    );
    assert!(
        model
            .invariants_for_scope_with_premises(
                &["undeclared_relation"].into_iter().collect(),
                &premises,
            )
            .is_err()
    );
    assert!(
        model
            .invariants_for_scope_with_premises(&Default::default(), &premises)
            .unwrap()
            .is_empty()
    );
    assert_eq!(count.load(Ordering::Relaxed), 0);
}
#[test]
fn local_stability_scope_requires_the_complete_acknowledged_normalized_premises() {
    use conditions::{
        entry::{EntryAccessSource, EntryValueWitness},
        stability::StabilityWitness,
    };
    let model = model().unwrap();
    let referrers = [
        StabilityWitness::NAME,
        EntryValueWitness::NAME,
        EntryAccessSource::NAME,
    ]
    .into_iter()
    .collect();
    let normalized = normalized_relations();
    let mut premises: std::collections::BTreeSet<_> =
        normalized.iter().map(Relation::name).collect();
    assert!(model.invariants_for_scope(&referrers).is_err());
    let checks = model
        .invariants_for_scope_with_premises(&referrers, &premises)
        .unwrap();
    let stability = checks
        .iter()
        .find(|check| check.name == "entry_guard_stability_replay")
        .unwrap();
    assert_eq!(
        stability.digest(),
        model.invariant(stability.name).unwrap().digest()
    );
    for check in &checks {
        assert!(
            check.inputs.iter().all(|input| {
                referrers.contains(input.name()) || premises.contains(input.name())
            })
        );
    }
    assert!(premises.remove(flow_inventory::FlowUseInventoryObservation::NAME));
    assert!(
        model
            .invariants_for_scope_with_premises(&referrers, &premises)
            .is_err()
    );
}
#[test]
fn two_references_execute_once_and_an_authored_negative_still_refuses() {
    for accepted in [true, false] {
        let count = Arc::new(AtomicUsize::new(0));
        let model = ValidatedModel::validate(
            vec![
                Relation::of::<Left>(),
                Relation::of::<Right>(),
                Relation::of::<Premise>(),
            ],
            shared(1, count.clone()),
        )
        .unwrap();
        let memory = MemoryGeneration::conformance(&model, &budget());
        memory
            .put(
                &Batch::new(
                    &model,
                    vec![Premise {
                        ordinal: 0,
                        accepted,
                    }],
                    &budget(),
                )
                .unwrap(),
            )
            .unwrap();
        assert_eq!(memory.validate(&model, &budget()).is_ok(), accepted);
        assert_eq!(count.load(Ordering::Relaxed), 1);
    }
}
#[test]
fn revision_identity_input_order_and_kind_are_explicit_and_conflicts_refuse() {
    let count = Arc::new(AtomicUsize::new(0));
    let declarations = vec![Relation::of::<Left>(), Relation::of::<Premise>()];
    let model1 = ValidatedModel::validate(declarations.clone(), shared(1, count.clone())).unwrap();
    let model2 = ValidatedModel::validate(declarations.clone(), shared(2, count.clone())).unwrap();
    assert_ne!(model1.digest(), model2.digest());
    assert_ne!(
        model1.invariants()[0].digest(),
        model2.invariants()[0].digest()
    );
    let original = model1.invariants()[0].identity();
    let contextual = ValidationIdentity {
        kind: ValidationKind::Publication,
        ..original
    };
    assert_ne!(original.digest(), contextual.digest());
    let mut changed_order = model1.invariants()[0].clone();
    changed_order.inputs = vec![ValidationInput::of::<Premise>(&["ordinal", "id"])];
    assert_ne!(changed_order.digest(), model1.invariants()[0].digest());
    let mut conflict = shared(1, count.clone());
    conflict
        .invariants
        .extend(shared(2, count.clone()).invariants);
    assert!(ValidatedModel::validate(declarations.clone(), conflict).is_err());
    assert!(ValidatedModel::validate(declarations.clone(), shared(0, count.clone())).is_err());
    let mut reserved = shared(1, count.clone());
    reserved.invariants[0].name = "derivation_acyclic";
    assert!(ValidatedModel::validate(vec![Relation::of::<Premise>()], reserved).is_err());
    let mut wrong_order = shared(1, count);
    wrong_order.invariants[0].inputs = vec![ValidationInput::of::<Premise>(&["missing", "id"])];
    assert!(ValidatedModel::validate(declarations, wrong_order).is_err());
}
#[test]
fn canonical_model_and_shared_source_call_requirements_execute_one_actual_replay() {
    let model = model()
        .expect("every production relation reference resolves to a complete canonical definition");
    for references in [
        SourceCallHeader::invariant_refs(),
        SourceInvocation::invariant_refs(),
        SourceCallRun::invariant_refs(),
    ] {
        assert_eq!(references, vec!["source_call_replay"]);
    }
    let requirements = stage(vec![
        RelationUse::stored::<SourceCallHeader>(),
        RelationUse::stored::<SourceInvocation>(),
        RelationUse::stored::<SourceCallRun>(),
    ])
    .read_invariants(&model)
    .unwrap();
    assert_eq!(requirements.len(), 1);
    assert_eq!(requirements[0].name, "source_call_replay");
    let count = Arc::new(AtomicUsize::new(0));
    let mut definition = requirements[0].clone();
    let original = definition.create.clone();
    let executions = count.clone();
    definition.create = Arc::new(move |budget| {
        executions.fetch_add(1, Ordering::Relaxed);
        original(budget)
    });
    let mut check = (definition.create)(&budget());
    for input in &definition.inputs {
        let relation = model.relation(input.name()).unwrap();
        check
            .visit_input(
                input,
                &arrow_array::RecordBatch::new_empty(relation.schema().clone()),
            )
            .unwrap();
    }
    check.finish().unwrap();
    assert_eq!(count.load(Ordering::Relaxed), 1);
}
fn nominal<T>(value: u8) -> Id<T> {
    serde::Deserialize::deserialize(serde::de::value::SeqDeserializer::<
        _,
        serde::de::value::Error,
    >::new([value; 16].into_iter()))
    .unwrap()
}
#[test]
fn actual_source_replay_refuses_orphan_run_malformed_header_and_extra_argument_inventory() {
    let definition = validation::invariants_for::<SourceCallRun>().remove(0);
    let run = SourceCallRun {
        invocation: nominal(1),
        requested: true,
        bound: 0,
        refused: 0,
    };
    let header = SourceCallHeader {
        invocation: nominal(1),
        event: nominal(2),
        attempt: nominal(3),
        owner: nominal(4),
        callee: nominal(5),
        declaration: nominal(6),
        qualification: nominal(7),
        status: analysis::policy::EvidenceStatus::StructurallyObserved,
        premises: ContentHash::of(b"malformed header"),
    };
    let argument = SourceFrameArgument {
        release: nominal(1),
        ordinal: 0,
        formal: nominal(2),
        actual: nominal(3),
        evaluation: nominal(4),
    };
    for (name, batch) in [
        (SourceCallRun::NAME, SourceCallRun::encode(&[run]).unwrap()),
        (
            SourceCallHeader::NAME,
            SourceCallHeader::encode(&[header]).unwrap(),
        ),
        (
            SourceFrameArgument::NAME,
            SourceFrameArgument::encode(&[argument]).unwrap(),
        ),
    ] {
        let mut check = (definition.create)(&budget());
        check.visit(name, &batch).unwrap();
        assert!(
            check.finish().is_err(),
            "orphan source-call inventory must refuse: {name}"
        );
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(name="scoped_proofs", rule="scoped_proof", semantic_source=include_bytes!("validation_definitions.rs"))]
struct Proof {
    #[model(key)]
    ordinal: i64,
    #[model(premise)]
    parent: Option<Id<Proof>>,
}
#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(name="other_scoped_proofs", rule="other_scoped_proof", semantic_source=include_bytes!("validation_definitions.rs"))]
struct OtherProof {
    #[model(key)]
    ordinal: i64,
    #[model(premise)]
    parent: Id<Proof>,
}
#[test]
fn lower_scope_derivation_is_required_and_exactly_scoped_and_cycles_refuse() {
    let model = ValidatedModel::validate(
        vec![Relation::of::<Proof>(), Relation::of::<OtherProof>()],
        ValidationDefinitions::default(),
    )
    .unwrap();
    let scope = [Proof::NAME].into_iter().collect();
    let checks = model.invariants_for_scope(&scope).unwrap();
    let with_upper_premise = model
        .invariants_for_scope_with_premises(&scope, &[OtherProof::NAME].into_iter().collect())
        .unwrap();
    assert_eq!(with_upper_premise.len(), 1);
    assert_eq!(with_upper_premise[0].digest(), checks[0].digest());
    assert_eq!(checks.len(), 1);
    assert_eq!(checks[0].name, "derivation_acyclic");
    assert_eq!(
        checks[0]
            .inputs
            .iter()
            .map(ValidationInput::name)
            .collect::<Vec<_>>(),
        vec![Proof::NAME]
    );
    assert_ne!(
        checks[0].digest(),
        model.invariant("derivation_acyclic").unwrap().digest()
    );
    assert_eq!(
        checks[0].digest(),
        model
            .generated_invariant_for_scope(&scope)
            .unwrap()
            .unwrap()
            .digest()
    );
    assert!(
        model
            .generated_invariant_for_scope(&["undeclared_relation"].into_iter().collect())
            .is_err()
    );
    for cyclic in [false, true] {
        let mut row = Proof {
            ordinal: 0,
            parent: None,
        };
        if cyclic {
            row.parent = Some(row.id());
        }
        let mut check = (checks[0].create)(&budget());
        check
            .visit(Proof::NAME, &Proof::encode(&[row]).unwrap())
            .unwrap();
        assert_eq!(
            check.finish().is_err(),
            cyclic,
            "authored self-cycle must fail the lower-scope generated checker"
        );
    }
}

#[test]
fn source_snapshot_wire_is_auditable_metadata_and_rejects_unknown_fields() {
    let wire = serde_json::json!({ "relation": Premise::NAME, "producer": "control", "model": ContentHash::of(b"model"), "schedule": ContentHash::of(b"schedule"), "content": ContentHash::of(b"content"), "rows": 0, "physical": "control_prefix", "prefix": "Facts" });
    let snapshot: analysis::sources::SourceSnapshot = serde_json::from_value(wire.clone()).unwrap();
    assert_eq!(snapshot.relation(), Premise::NAME);
    assert_eq!(snapshot.rows(), 0);
    assert_eq!(snapshot.prefix(), Some("Facts"));
    assert_eq!(serde_json::to_value(&snapshot).unwrap(), wire);
    let mut malformed = wire;
    malformed
        .as_object_mut()
        .unwrap()
        .insert("read_authority".into(), serde_json::json!(true));
    assert!(serde_json::from_value::<analysis::sources::SourceSnapshot>(malformed).is_err());
}
