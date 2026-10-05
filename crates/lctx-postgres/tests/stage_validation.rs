//! A frozen relation is not eligible until its owned cross-relation checks pass.
use lctx_model::{
    Domain,
    domain::{input::Package, resources::ResourceBudget, stages::*, *},
};
use lctx_postgres::{generations::GenerationStore, testing::DisposableDatabase};
use std::{collections::BTreeMap, sync::Arc};

#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(name = "stage_anchors", semantic_source = include_bytes!("stage_validation.rs"))]
struct Anchor {
    #[model(key)]
    name: String,
    value: i64,
}
#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(name = "stage_checked", invariant_refs = checks_refs, semantic_source = include_bytes!("stage_validation.rs"))]
struct Checked {
    #[model(key)]
    anchor: Id<Anchor>,
    value: i64,
}
fn checks() -> Vec<Invariant> {
    vec![Invariant {
        revision: 1,
        name: "checked_matches_anchor",
        inputs: vec![
            ValidationInput::of::<Anchor>(&["id"]),
            ValidationInput::of::<Checked>(&["id"]),
        ],
        create: Arc::new(|_| {
            Box::new(Check {
                anchors: BTreeMap::new(),
            })
        }),
    }]
}
struct Check {
    anchors: BTreeMap<Id<Anchor>, i64>,
}
impl InvariantCheck for Check {
    fn visit(
        &mut self,
        relation: &str,
        batch: &arrow_array::RecordBatch,
    ) -> Result<(), ModelError> {
        if relation == Anchor::NAME {
            for row in Anchor::decode(batch)? {
                self.anchors.insert(row.id(), row.value);
            }
        } else {
            for row in Checked::decode(batch)? {
                if self.anchors.get(&row.anchor) != Some(&row.value) {
                    return Err(ModelError::Invalid(
                        "checked value disagrees with anchor".into(),
                    ));
                }
            }
        }
        Ok(())
    }
    fn finish(self: Box<Self>) -> Result<(), ModelError> {
        Ok(())
    }
}

#[tokio::test]
async fn owned_invariants_and_nominal_closure_refuse_before_a_read_capability_exists() {
    let db = DisposableDatabase::start().await;
    let model = Arc::new(
        ValidatedModel::validate(
            vec![
                Relation::of::<Anchor>(),
                Relation::of::<Checked>(),
                Relation::of::<Package>(),
            ],
            lctx_model::domain::ValidationDefinitions {
                invariants: checks(),
                publication_checks: vec![],
            },
        )
        .unwrap(),
    );
    let store = GenerationStore::install(db.owner.clone(), model.clone())
        .await
        .unwrap();
    let stage = |name, inputs, outputs| Stage {
        name,
        inputs,
        outputs,
        contributes: vec![],
        coverage: vec![],
        profiles: vec![Profile::Catalog],
        effect: Effect::Store,
        code: ContentHash::of(b"reader checks"),
        configuration: ContentHash::of(b"test"),
    };
    let schedule = Schedule::build(
        &model,
        vec![
            stage(
                "source",
                vec![],
                vec![RelationUse::of::<Anchor>(), RelationUse::of::<Checked>()],
            ),
            stage(
                "consumer",
                vec![
                    RelationUse::stored::<Anchor>(),
                    RelationUse::stored::<Checked>().validated_by(&[]),
                ],
                vec![RelationUse::of::<Package>()],
            ),
        ],
        &[],
        Profile::Catalog,
    )
    .unwrap();
    for variant in 0..3 {
        let budget = ResourceBudget::fixed(8 << 20).unwrap();
        let mut execution = schedule.execute();
        let attempt = store
            .begin_conformance(db.writer.clone(), &mut execution, budget.clone())
            .await
            .unwrap();
        let id = attempt.generation();
        let anchor = Anchor {
            name: "valid".into(),
            value: 7,
        };
        let mut output = StageOutput::new(
            execution.begin("source").unwrap(),
            &attempt,
            &model,
            budget,
            Default::default(),
        )
        .unwrap();
        output.declare::<Anchor>().unwrap();
        output.declare::<Checked>().unwrap();
        output
            .push(Checked {
                anchor: if variant == 2 {
                    Anchor {
                        name: "missing".into(),
                        value: 7,
                    }
                    .id()
                } else {
                    anchor.id()
                },
                value: if variant == 1 { 8 } else { 7 },
            })
            .await
            .unwrap();
        output.push(anchor).await.unwrap();
        output.finish(ProviderOutcome::Complete).await.unwrap();
        let consumer = execution.begin("consumer").unwrap();
        let eligible = attempt.read_contract(&consumer).await;
        assert_eq!(
            eligible.is_ok(),
            variant == 0,
            "variant {variant}: {eligible:?}"
        );
        let count: i64 = sqlx::query_scalar("SELECT count(*) FROM lctx_model_store.stage_read_checks WHERE generation_id=decode($1,'hex')")
            .bind(id.hex()).fetch_one(db.owner.pool()).await.unwrap();
        assert_eq!(
            count,
            if variant == 0 { 2 } else { 0 },
            "refusal cannot acknowledge a partial check set"
        );
        drop(consumer);
        drop(eligible);
        attempt.abort().await.unwrap();
    }
}

#[tokio::test]
async fn inferred_fact_premise_requires_checkpoint_proof_or_an_explicit_source() {
    use lctx_model::domain::{dependency_closure::*, input::Release};
    let db = DisposableDatabase::start().await;
    let model = Arc::new(
        ValidatedModel::declared(vec![
            Relation::of::<Package>(),
            Relation::of::<Release>(),
            Relation::of::<Anchor>(),
        ])
        .unwrap(),
    );
    let store = GenerationStore::install(db.owner.clone(), model.clone())
        .await
        .unwrap();
    let order = PublicationOrder::planning(&[PublicationGroup::new(
        PublicationBoundary::Facts,
        vec!["source"],
    )])
    .unwrap();
    let stage = |name, inputs, outputs| Stage {
        name,
        inputs,
        outputs,
        contributes: vec![],
        coverage: vec![],
        profiles: vec![Profile::Catalog],
        effect: Effect::Store,
        code: ContentHash::of(b"checkpoint premise control"),
        configuration: ContentHash::of(b"fixture"),
    };
    for explicit in [false, true] {
        let budget = ResourceBudget::fixed(8 << 20).unwrap();
        let closure = DependencyClosure::build(
            &model,
            vec![ValidationInput::of::<Release>(&["id"])],
            if explicit {
                vec![RelationUse::stored::<Package>()]
            } else {
                vec![]
            },
            &[],
            PublicationBoundary::Facts,
            LowerLayerPolicy::OmitInferredOrdinaryFacts,
            &order,
        )
        .unwrap();
        let schedule = Schedule::build(
            &model,
            vec![
                stage(
                    "source",
                    vec![],
                    vec![RelationUse::of::<Package>(), RelationUse::of::<Release>()],
                ),
                stage(
                    "consumer",
                    closure.grants,
                    vec![RelationUse::of::<Anchor>()],
                ),
            ],
            &[],
            Profile::Catalog,
        )
        .unwrap();
        let mut execution = schedule.execute();
        let attempt = store
            .begin_conformance(db.writer.clone(), &mut execution, budget.clone())
            .await
            .unwrap();
        let mut output = StageOutput::new(
            execution.begin("source").unwrap(),
            &attempt,
            &model,
            budget,
            Default::default(),
        )
        .unwrap();
        output.declare::<Package>().unwrap();
        output.declare::<Release>().unwrap();
        let package = Package {
            name: "checkpoint-probe".into(),
        };
        output
            .push(Release {
                package: package.id(),
                version: "1.0".into(),
            })
            .await
            .unwrap();
        output.push(package).await.unwrap();
        output.finish(ProviderOutcome::Complete).await.unwrap();
        let consumer = execution.begin("consumer").unwrap();
        assert_eq!(
            consumer.read::<Package>().is_ok(),
            explicit,
            "closure omission cannot authorize an actual fact read"
        );
        let admitted = attempt.read_contract(&consumer).await;
        assert_eq!(
            admitted.is_ok(),
            explicit,
            "no checkpoint was admitted, so the inferred fact must have an explicit acknowledged source: {admitted:?}"
        );
        drop(admitted);
        drop(consumer);
        attempt.abort().await.unwrap();
    }
}

fn checks_refs() -> Vec<&'static str> {
    vec!["checked_matches_anchor"]
}
