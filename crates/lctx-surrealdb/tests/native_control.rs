//! Focused controls for the shared content, exact-view and durable ownership cutover.
use lctx_model::domain::{
    ContentHash, FiniteF64, ModelError, Record, Relation,
    admission::Frontier,
    analysis::sources::SourceSnapshot,
    analytics::QualityStep,
    completed::{CompletedBinding, ContributionSpec},
    resources::ResourceBudget,
    stages::{Profile, ProviderOutcome},
};
use lctx_surrealdb::{RuntimeConfig, compiler::NativeCompilerStore, control};
use std::collections::{BTreeMap, BTreeSet};
use surrealdb::types::{Object, RecordId, Value, Variables};
fn config() -> RuntimeConfig {
    RuntimeConfig::read(std::path::Path::new(
        &std::env::var_os("LCTX_COMPILER_RUNTIME_CONFIG").expect("stable validation runtime"),
    ))
    .unwrap()
}
fn row(run: [u8; 16], value: f64) -> QualityStep {
    QualityStep {
        run: serde_json::from_value(serde_json::json!(run)).unwrap(),
        ordinal: 0,
        value: FiniteF64::new(value).unwrap(),
    }
}
fn specification(producer: String) -> ContributionSpec {
    ContributionSpec {
        captured_binding: None,
        producer,
        profile: Profile::Catalog,
        model: lctx_model::domain::model().unwrap().digest(),
        implementation: ContentHash::of(b"native-control/v1"),
        configuration: None,
        inputs: vec![],
        outputs: BTreeSet::from([QualityStep::NAME.into()]),
    }
}
async fn contribute(
    store: &std::sync::Arc<NativeCompilerStore>,
    spec: ContributionSpec,
    row: &QualityStep,
    previous: &BTreeMap<String, lctx_model::domain::completed::CompletedView>,
) -> (
    ContentHash,
    BTreeMap<String, lctx_model::domain::completed::CompletedView>,
) {
    let relation = Relation::of::<QualityStep>();
    let id = store.begin_contribution(spec).await.unwrap();
    store
        .write_batch(
            &id,
            &relation,
            &QualityStep::encode(std::slice::from_ref(row)).unwrap(),
        )
        .await
        .unwrap();
    let views = store
        .complete_contribution(id, ProviderOutcome::Complete, &[relation], previous)
        .await
        .unwrap();
    (id, views)
}
#[tokio::test(flavor = "multi_thread")]
async fn indexed_abandonment_removes_only_unadmitted_products_and_owned_holds() {
    let cfg = config();
    let private = NativeCompilerStore::begin(&cfg, Frontier::Facts)
        .await
        .unwrap();
    let admitted = NativeCompilerStore::begin(&cfg, Frontier::Facts)
        .await
        .unwrap();
    let nonce = control::fresh_identity("indexed-abandonment").unwrap();
    let run = nonce.0[..16].try_into().unwrap();
    let (private_contribution, _) = contribute(
        &private,
        specification(format!("private-{}", nonce.hex())),
        &row(run, 1.0),
        &BTreeMap::new(),
    )
    .await;
    let (admitted_contribution, views) = contribute(
        &admitted,
        specification(format!("admitted-{}", nonce.hex())),
        &row(run, 2.0),
        &BTreeMap::new(),
    )
    .await;
    private
        .retain_product_identity(nonce, private_contribution)
        .await
        .unwrap();
    admitted
        .retain_product_identity(nonce, admitted_contribution)
        .await
        .unwrap();
    let view = views[QualityStep::NAME].clone();
    admitted
        .bind(CompletedBinding {
            boundary: None,
            source: SourceSnapshot::of_completed_view(
                &Relation::of::<QualityStep>(),
                specification(String::new()).model,
                &view,
            )
            .unwrap(),
            view,
            configuration: None,
        })
        .await
        .unwrap();
    admitted.mark_attempt_admitted().await.unwrap();
    let client = lctx_surrealdb::compiler::check_installation(&cfg)
        .await
        .unwrap();
    let unrelated = control::ReaderPin::acquire(client.clone(), &[])
        .await
        .unwrap();
    let result=async {
        unrelated.protect(RecordId::new("compiler_contribution",private_contribution.hex())).await?;
        let private_owner = RecordId::new("native_attempt", private.attempt().hex());
        let private_product = RecordId::new("native_product",format!("{}_{}",nonce.hex(),private.attempt().hex()));
        // Exercise many transaction windows for both attempt and unadmitted product roots.
        // The immutable targets survive cleanup; only these two owners lose their holds.
        let targets = (0..1025).map(|ordinal| RecordId::new("original",ContentHash::of(format!("cleanup_{}_{}",nonce.hex(),ordinal).as_bytes()).hex())).collect::<Vec<_>>();
        for window in targets.chunks(128) {
            let rows = window.iter().map(|id| {
                let mut row = Object::new();
                row.insert("id",id.clone());
                row.insert("content",ContentHash::of(b"").hex());
                row.insert("byte_len",0i64);
                Value::Object(row)
            }).collect();
            control::ensure_rows(&client,Some(private.attempt()),rows).await?;
            control::hold(&client,Some(private.attempt()),private_owner.clone(),window.to_vec()).await?;
            control::hold(&client,Some(private.attempt()),private_product.clone(),window.to_vec()).await?;
        }
        let mut partial = Variables::new();
        partial.insert("attempt",private_owner.clone());
        partial.insert("target",targets[0].clone());
        partial.insert("admitted_attempt",RecordId::new("native_attempt",admitted.attempt().hex()));
        let mut response = client.query("SELECT VALUE id FROM native_hold WITH INDEX owner_holds WHERE owner=$attempt AND object=$target LIMIT 1").bind(partial.clone()).await.map_err(ModelError::codec)?.check().map_err(ModelError::codec)?;
        let first:Vec<RecordId> = response.take(0).map_err(ModelError::codec)?;
        partial.insert("first",first);
        // Simulate restart after a confirmed fence and one confirmed partial release.
        control::effect(&client,None,"LET $owner=SELECT * FROM ONLY $attempt FOR UPDATE; UPDATE $attempt SET state='frozen',revision+=1 RETURN NONE; UPDATE $admitted_attempt SET state='maintenance_fenced',revision+=1 RETURN NONE; LET $owned=SELECT VALUE id FROM $first WHERE owner=$attempt; DELETE $owned RETURN NONE",partial).await?;
        let post_fence_refused = control::effect(&client,Some(private.attempt()),"RETURN NONE",Variables::new()).await.is_err();
        let mut plans=Vec::new();
        for (sql,index,variable,value) in [
            ("SELECT VALUE id FROM native_hold WITH INDEX owner_holds WHERE owner=$owner","owner_holds","owner",RecordId::new("native_attempt",private.attempt().hex())),
            ("SELECT VALUE id FROM compiler_contribution WITH INDEX attempt_contributions WHERE attempt=$attempt","attempt_contributions","attempt",RecordId::new("native_attempt",private.attempt().hex())),
            ("SELECT VALUE id FROM native_product WITH INDEX contribution_products WHERE contribution=$contribution","contribution_products","contribution",RecordId::new("compiler_contribution",private_contribution.hex())),
        ] {let mut vars=Variables::new();vars.insert(variable,value);let plan:String=lctx_surrealdb::NativeReader::private(client.clone()).query(format!("EXPLAIN {sql}"),vars).await?;plans.push((index,plan));}
        private.abandon().await?;admitted.abandon().await?;
        let private_product=RecordId::new("native_product",format!("{}_{}",nonce.hex(),private.attempt().hex()));let admitted_product=RecordId::new("native_product",format!("{}_{}",nonce.hex(),admitted.attempt().hex()));
        let mut response=client.query("SELECT VALUE id FROM $products; SELECT VALUE object FROM native_hold WITH INDEX owner_holds WHERE owner=$private; SELECT VALUE object FROM native_hold WITH INDEX owner_holds WHERE owner=$admitted; SELECT VALUE object FROM native_hold WITH INDEX owner_holds WHERE owner=$unrelated; SELECT VALUE state FROM $attempts; SELECT VALUE id FROM native_hold WITH INDEX owner_holds WHERE owner=$attempt; SELECT VALUE id FROM $targets").bind(("products",vec![private_product.clone(),admitted_product.clone()])).bind(("private",private_product)).bind(("admitted",admitted_product.clone())).bind(("unrelated",RecordId::new("native_pin",unrelated.identity.hex()))).bind(("attempts",vec![private_owner.clone(),RecordId::new("native_attempt",admitted.attempt().hex())])).bind(("attempt",private_owner)).bind(("targets",targets)).await.map_err(lctx_model::domain::ModelError::codec)?.check().map_err(lctx_model::domain::ModelError::codec)?;
        let products:Vec<RecordId>=response.take(0).map_err(lctx_model::domain::ModelError::codec)?;let private_holds:Vec<RecordId>=response.take(1).map_err(lctx_model::domain::ModelError::codec)?;let admitted_holds:Vec<RecordId>=response.take(2).map_err(lctx_model::domain::ModelError::codec)?;let unrelated_holds:Vec<RecordId>=response.take(3).map_err(lctx_model::domain::ModelError::codec)?;
        let states:Vec<String>=response.take(4).map_err(ModelError::codec)?;
        let attempt_holds:Vec<RecordId>=response.take(5).map_err(ModelError::codec)?;
        let targets:Vec<RecordId>=response.take(6).map_err(ModelError::codec)?;
        Ok::<_,lctx_model::domain::ModelError>((plans,products,admitted_product,private_holds,admitted_holds,unrelated_holds,post_fence_refused,states,attempt_holds,targets))
    }.await;
    let mut completion = lctx_model::domain::completion::Completion::default();
    completion.step(
        "indexed cleanup unrelated pin release",
        unrelated.release().await,
    );
    completion.step("indexed cleanup private local drain", private.drain().await);
    completion.step(
        "indexed cleanup private owner",
        control::close_attempt(&client, private.attempt(), "abandoned").await,
    );
    completion.step(
        "indexed cleanup admitted local drain",
        admitted.drain().await,
    );
    completion.step(
        "indexed cleanup admitted owner",
        control::close_attempt(&client, admitted.attempt(), "abandoned").await,
    );
    let (
        plans,
        products,
        admitted_product,
        private_holds,
        admitted_holds,
        unrelated_holds,
        post_fence_refused,
        states,
        attempt_holds,
        targets,
    ) = lctx_model::domain::completion::complete(result, completion).unwrap();
    assert!(post_fence_refused);
    assert_eq!(
        states.into_iter().collect::<BTreeSet<_>>(),
        BTreeSet::from(["frozen".to_string(), "maintenance_fenced".to_string()])
    );
    assert!(attempt_holds.is_empty());
    assert_eq!(targets.len(), 1025);
    for (index, plan) in plans {
        assert!(
            plan.contains("IndexScan ")
                && plan.contains(&format!("index: {index},"))
                && !plan.contains("TableScan"),
            "{index}: {plan}"
        );
    }
    assert_eq!(products, vec![admitted_product]);
    assert!(private_holds.is_empty());
    assert!(admitted_holds.contains(&RecordId::new(
        "compiler_contribution",
        admitted_contribution.hex()
    )));
    assert_eq!(
        unrelated_holds,
        vec![RecordId::new(
            "compiler_contribution",
            private_contribution.hex()
        )]
    );
}
#[tokio::test(flavor = "multi_thread")]
async fn shared_revision_addresses_coexist_and_exact_views_select_their_payload() {
    let cfg = config();
    let a = NativeCompilerStore::begin(&cfg, Frontier::Facts)
        .await
        .unwrap();
    let b = NativeCompilerStore::begin(&cfg, Frontier::Facts)
        .await
        .unwrap();
    assert_eq!(a.database(), b.database());
    assert_eq!(a.database(), &cfg.database);
    let nonce = control::fresh_identity("revision-control").unwrap();
    let run = nonce.0[..16].try_into().unwrap();
    let (_, av) = contribute(
        &a,
        specification(format!("a-{}", nonce.hex())),
        &row(run, 1.0),
        &BTreeMap::new(),
    )
    .await;
    let (_, bv) = contribute(
        &b,
        specification(format!("b-{}", nonce.hex())),
        &row(run, 2.0),
        &BTreeMap::new(),
    )
    .await;
    let budget = ResourceBudget::fixed(64 << 20).unwrap();
    let relation = Relation::of::<QualityStep>();
    let mut ar = a
        .scan_rows(&av[QualityStep::NAME], &relation, None, None, &budget)
        .await
        .unwrap();
    let ab = ar.next().await.unwrap().unwrap();
    assert_eq!(
        QualityStep::decode(
            &lctx_surrealdb::codec::decode_bodies(&relation, vec![ab], &budget).unwrap()
        )
        .unwrap()[0]
            .value,
        FiniteF64::new(1.0).unwrap()
    );
    assert!(ar.next().await.unwrap().is_none());
    drop(ar);
    let mut br = b
        .scan_rows(&bv[QualityStep::NAME], &relation, None, None, &budget)
        .await
        .unwrap();
    let bb = br.next().await.unwrap().unwrap();
    assert_eq!(
        QualityStep::decode(
            &lctx_surrealdb::codec::decode_bodies(&relation, vec![bb], &budget).unwrap()
        )
        .unwrap()[0]
            .value,
        FiniteF64::new(2.0).unwrap()
    );
    assert!(br.next().await.unwrap().is_none());
    drop(br);
    a.abandon().await.unwrap();
    b.abandon().await.unwrap();
}
#[tokio::test(flavor = "multi_thread")]
async fn atomic_field_reads_exclude_foreign_same_key_revisions_before_sorting() {
    use lctx_model::domain::normalized::links::ImportModuleCandidate;
    use lctx_surrealdb::compiler::NativePredicate;
    let cfg = config();
    let selected = NativeCompilerStore::begin(&cfg, Frontier::Normalized)
        .await
        .unwrap();
    let foreign = NativeCompilerStore::begin(&cfg, Frontier::Normalized)
        .await
        .unwrap();
    let nonce = control::fresh_identity("atomic-revision-control").unwrap();
    let key = |tag: u8| {
        let mut bytes = [tag; 16];
        bytes[..8].copy_from_slice(&nonce.0[..8]);
        bytes
    };
    let a = ImportModuleCandidate {
        assessment: serde_json::from_value(serde_json::json!(key(1))).unwrap(),
        observation: serde_json::from_value(serde_json::json!(key(2))).unwrap(),
        module: serde_json::from_value(serde_json::json!(key(3))).unwrap(),
    };
    let mut b = a.clone();
    b.module = serde_json::from_value(serde_json::json!(key(4))).unwrap();
    assert_eq!(a.id(), b.id());
    let relation = Relation::of::<ImportModuleCandidate>();
    for field in ["assessment", "module"] {
        assert!(lctx_surrealdb::schema::atomic_scope_field(
            relation.name(),
            field
        ));
    }
    let mut selected_view = None;
    for (store, record, producer) in [(&selected, &a, "selected"), (&foreign, &b, "foreign")] {
        let mut spec = specification(format!("{producer}-{}", nonce.hex()));
        spec.outputs = BTreeSet::from([relation.name().into()]);
        let owner = store.begin_contribution(spec).await.unwrap();
        store
            .write_batch(
                &owner,
                &relation,
                &ImportModuleCandidate::encode(std::slice::from_ref(record)).unwrap(),
            )
            .await
            .unwrap();
        let views = store
            .complete_contribution(
                owner,
                ProviderOutcome::Complete,
                std::slice::from_ref(&relation),
                &BTreeMap::new(),
            )
            .await
            .unwrap();
        if producer == "selected" {
            selected_view = Some(views[relation.name()].clone());
        }
    }
    let budget = ResourceBudget::fixed(64 << 20).unwrap();
    let result = async {
        // Predicates use the native body representation (reference byte arrays),
        // rather than the nominal key's display encoding.
        let bodies = lctx_surrealdb::codec::batch_bodies(
            &relation,
            &ImportModuleCandidate::encode(&[a.clone(), b.clone()])?,
        )?;
        let field = |row: usize, name: &str| {
            let Value::Object(body) = &bodies[row] else {
                panic!("native record body")
            };
            body.get(name).unwrap().clone()
        };
        let predicates = [
            NativePredicate::Field {
                field: "assessment".into(),
                values: vec![field(0, "assessment")],
            },
            NativePredicate::Field {
                field: "module".into(),
                values: vec![field(0, "module"), field(1, "module")],
            },
            NativePredicate::FieldSql {
                field: "module".into(),
                values: vec![field(1, "module")],
                sql: "$atomic_residual".into(),
                bindings: Variables::new(),
                preparation: vec!["LET $atomic_residual = true".into()],
            },
        ];
        let mut results = vec![];
        for predicate in predicates {
            let mut rows = selected
                .scan_rows(
                    selected_view.as_ref().unwrap(),
                    &relation,
                    None,
                    Some(predicate),
                    &budget,
                )
                .await?;
            let mut bodies = vec![];
            while let Some(body) = rows.next().await? {
                bodies.push(body);
            }
            results.push(ImportModuleCandidate::decode(
                &lctx_surrealdb::codec::decode_bodies(&relation, bodies, &budget)?,
            )?);
        }
        Ok::<_, lctx_model::domain::ModelError>(results)
    }
    .await;
    let selected_cleanup = selected.abandon().await;
    let foreign_cleanup = foreign.abandon().await;
    let results = result.unwrap();
    selected_cleanup.unwrap();
    foreign_cleanup.unwrap();
    assert_eq!(results, vec![vec![a.clone()], vec![a], vec![]]);
}
#[tokio::test(flavor = "multi_thread")]
async fn only_admitted_retention_attaches_to_fresh_attempt_without_payload_replay() {
    let cfg = config();
    let source = NativeCompilerStore::begin(&cfg, Frontier::Facts)
        .await
        .unwrap();
    let target = NativeCompilerStore::begin(&cfg, Frontier::Facts)
        .await
        .unwrap();
    let nonce = control::fresh_identity("retention-control").unwrap();
    let spec = specification(nonce.hex());
    let run = nonce.0[..16].try_into().unwrap();
    let (id, views) = contribute(&source, spec.clone(), &row(run, 3.0), &BTreeMap::new()).await;
    source.retain_product_identity(nonce, id).await.unwrap();
    assert!(
        target
            .attach_retained_product(nonce, &spec)
            .await
            .unwrap()
            .is_none()
    );
    let view = views[QualityStep::NAME].clone();
    let relation = Relation::of::<QualityStep>();
    source
        .bind(CompletedBinding {
            boundary: None,
            source: SourceSnapshot::of_completed_view(&relation, spec.model, &view).unwrap(),
            view,
            configuration: None,
        })
        .await
        .unwrap();
    source.mark_attempt_admitted().await.unwrap();
    let mut changed = spec.clone();
    changed.model = ContentHash::of(b"changed model");
    assert!(
        target
            .attach_retained_product(nonce, &changed)
            .await
            .unwrap()
            .is_none()
    );
    changed = spec.clone();
    changed.implementation = ContentHash::of(b"changed implementation");
    assert!(
        target
            .attach_retained_product(nonce, &changed)
            .await
            .unwrap()
            .is_none()
    );
    changed = spec.clone();
    changed.producer.push_str("changed source");
    assert!(
        target
            .attach_retained_product(nonce, &changed)
            .await
            .unwrap()
            .is_none()
    );
    changed = spec.clone();
    changed.inputs.push(
        SourceSnapshot::of_completed_view(&relation, spec.model, &views[QualityStep::NAME])
            .unwrap(),
    );
    assert!(
        target
            .attach_retained_product(nonce, &changed)
            .await
            .unwrap()
            .is_none()
    );
    let client = lctx_surrealdb::compiler::check_installation(&cfg)
        .await
        .unwrap();
    let mut response=client.query("SELECT VALUE count() FROM compiler_record WHERE semantic_type=$relation AND semantic_key=$key GROUP ALL").bind(("relation",QualityStep::NAME)).bind(("key",row(run,3.0).id().hex())).await.unwrap().check().unwrap();
    let before: Vec<u64> = response.take(0).unwrap();
    let (attached, attached_views, descriptor) = target
        .attach_retained_product(nonce, &spec)
        .await
        .unwrap()
        .unwrap();
    let mut response=client.query("SELECT VALUE count() FROM compiler_record WHERE semantic_type=$relation AND semantic_key=$key GROUP ALL").bind(("relation",QualityStep::NAME)).bind(("key",row(run,3.0).id().hex())).await.unwrap().check().unwrap();
    let after: Vec<u64> = response.take(0).unwrap();
    assert_eq!(before, vec![1]);
    assert_eq!(
        after, before,
        "attachment does not replay or duplicate backing payloads"
    );
    assert_ne!(id, attached);
    assert_eq!(attached_views[QualityStep::NAME], views[QualityStep::NAME]);
    assert_eq!(descriptor.spec, spec);
    let current = target
        .complete_contribution(
            attached,
            ProviderOutcome::Complete,
            &[relation],
            &BTreeMap::new(),
        )
        .await
        .unwrap();
    assert_eq!(current, attached_views);
    source.abandon().await.unwrap();
    target.abandon().await.unwrap();
}
#[tokio::test(flavor = "multi_thread")]
async fn guarded_reachability_retains_shared_children() {
    let client = lctx_surrealdb::compiler::check_installation(&config())
        .await
        .unwrap();
    let nonce = control::fresh_identity("retirement-control").unwrap();
    let parent = RecordId::new("native_guard", format!("p{}", nonce.hex()));
    let other = RecordId::new("native_guard", format!("o{}", nonce.hex()));
    let child = RecordId::new("native_guard", format!("c{}", nonce.hex()));
    let rows = [&parent, &other, &child]
        .into_iter()
        .map(|id| {
            let mut row = Object::new();
            row.insert("id", id.clone());
            row.insert("revision", 0i64);
            row.insert("retired", false);
            Value::Object(row)
        })
        .collect();
    control::ensure_rows(&client, None, rows).await.unwrap();
    control::hold(&client, None, parent.clone(), vec![child.clone()])
        .await
        .unwrap();
    control::hold(&client, None, other.clone(), vec![child.clone()])
        .await
        .unwrap();
    let pass = control::retire_reachable(&client, vec![parent], 16)
        .await
        .unwrap();
    assert_eq!(pass.retired, 1);
    assert_eq!(pass.retained.len(), 1);
    assert!(
        !control::retirement_eligibility(&client, child.clone())
            .await
            .unwrap()
            .eligible
    );
    let pass = control::retire_reachable(&client, vec![other], 16)
        .await
        .unwrap();
    assert_eq!(pass.retired, 2);
    control::ensure_rows(
        &client,
        None,
        vec![{
            let mut row = Object::new();
            row.insert("id", child.clone());
            row.insert("revision", 0i64);
            row.insert("retired", false);
            Value::Object(row)
        }],
    )
    .await
    .unwrap();
    control::retire(&client, child).await.unwrap();
}
#[tokio::test(flavor = "multi_thread")]
async fn known_statement_abort_is_reconciled_and_attempt_fence_rejects_late_effect() {
    let cfg = config();
    let store = NativeCompilerStore::begin(&cfg, Frontier::Facts)
        .await
        .unwrap();
    let client = lctx_surrealdb::compiler::check_installation(&cfg)
        .await
        .unwrap();
    assert!(
        control::effect(
            &client,
            Some(store.attempt()),
            "THROW 'expected native control abort'",
            Variables::new()
        )
        .await
        .is_err()
    );
    {
        let mut response = client
            .query("SELECT VALUE id FROM native_effect WHERE attempt=$attempt AND resolved=false")
            .bind((
                "attempt",
                RecordId::new("native_attempt", store.attempt().hex()),
            ))
            .await
            .unwrap()
            .check()
            .unwrap();
        let pending: Vec<RecordId> = response.take(0).unwrap();
        assert!(pending.is_empty());
    }
    control::close_attempt(&client, store.attempt(), "frozen")
        .await
        .unwrap();
    assert!(
        control::effect(
            &client,
            Some(store.attempt()),
            "RETURN NONE",
            Variables::new()
        )
        .await
        .is_err()
    );
    {
        let mut response = client
            .query("SELECT VALUE id FROM native_effect WHERE attempt=$attempt AND resolved=false")
            .bind((
                "attempt",
                RecordId::new("native_attempt", store.attempt().hex()),
            ))
            .await
            .unwrap()
            .check()
            .unwrap();
        let pending: Vec<RecordId> = response.take(0).unwrap();
        assert!(pending.is_empty());
    }
    store.abandon().await.unwrap();
}

#[tokio::test(flavor = "multi_thread")]
async fn retained_membership_deletion_refuses_attachment_and_same_view_revision_conflicts() {
    let cfg = config();
    let source = NativeCompilerStore::begin(&cfg, Frontier::Facts)
        .await
        .unwrap();
    let nonce = control::fresh_identity("deleted-membership-control").unwrap();
    let run = nonce.0[..16].try_into().unwrap();
    let spec = specification(nonce.hex());
    let (id, views) = contribute(&source, spec.clone(), &row(run, 1.0), &BTreeMap::new()).await;
    let relation = Relation::of::<QualityStep>();
    let view = views[QualityStep::NAME].clone();
    source
        .bind(CompletedBinding {
            boundary: None,
            source: SourceSnapshot::of_completed_view(&relation, spec.model, &view).unwrap(),
            view,
            configuration: None,
        })
        .await
        .unwrap();
    source.retain_product_identity(nonce, id).await.unwrap();
    source.mark_attempt_admitted().await.unwrap();
    let client = lctx_surrealdb::compiler::check_installation(&cfg)
        .await
        .unwrap();
    client
        .query("DELETE compiler_membership WHERE contribution=$owner")
        .bind(("owner", RecordId::new("compiler_contribution", id.hex())))
        .await
        .unwrap()
        .check()
        .unwrap();
    let target = NativeCompilerStore::begin(&cfg, Frontier::Facts)
        .await
        .unwrap();
    assert!(
        target.attach_retained_product(nonce, &spec).await.is_err(),
        "retained descriptor alone cannot admit missing compact membership"
    );
    source.abandon().await.unwrap();
    target.abandon().await.unwrap();
    let union = NativeCompilerStore::begin(&cfg, Frontier::Facts)
        .await
        .unwrap();
    let (_, prior) = contribute(
        &union,
        specification(format!("first{}", nonce.hex())),
        &row(run, 1.0),
        &BTreeMap::new(),
    )
    .await;
    let id = union
        .begin_contribution(specification(format!("second{}", nonce.hex())))
        .await
        .unwrap();
    union
        .write_batch(
            &id,
            &relation,
            &QualityStep::encode(&[row(run, 2.0)]).unwrap(),
        )
        .await
        .unwrap();
    assert!(
        union
            .complete_contribution(id, ProviderOutcome::Complete, &[relation], &prior)
            .await
            .is_err(),
        "one exact view cannot select two revisions of a nominal row"
    );
    let _ = union.abandon().await;
}
#[tokio::test(flavor = "multi_thread")]
async fn retained_empty_output_keeps_exact_zero_cardinality() {
    let cfg = config();
    let source = NativeCompilerStore::begin(&cfg, Frontier::Facts)
        .await
        .unwrap();
    let target = NativeCompilerStore::begin(&cfg, Frontier::Facts)
        .await
        .unwrap();
    let nonce = control::fresh_identity("empty-output-control").unwrap();
    let spec = specification(nonce.hex());
    let relation = Relation::of::<QualityStep>();
    let id = source.begin_contribution(spec.clone()).await.unwrap();
    let views = source
        .complete_contribution(
            id,
            ProviderOutcome::Complete,
            std::slice::from_ref(&relation),
            &BTreeMap::new(),
        )
        .await
        .unwrap();
    let view = views[QualityStep::NAME].clone();
    assert_eq!(view.rows, 0);
    source
        .bind(CompletedBinding {
            boundary: None,
            source: SourceSnapshot::of_completed_view(&relation, spec.model, &view).unwrap(),
            view,
            configuration: None,
        })
        .await
        .unwrap();
    source.retain_product_identity(nonce, id).await.unwrap();
    source.mark_attempt_admitted().await.unwrap();
    let (_, attached, descriptor) = target
        .attach_retained_product(nonce, &spec)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(attached[QualityStep::NAME].rows, 0);
    assert_eq!(descriptor.outputs[QualityStep::NAME].rows, 0);
    source.abandon().await.unwrap();
    target.abandon().await.unwrap();
}
#[tokio::test(flavor = "multi_thread")]
async fn reconciliation_fence_excludes_late_remote_commit_and_retirement_resumes_after_unpin() {
    let cfg = config();
    let client = lctx_surrealdb::compiler::check_installation(&cfg)
        .await
        .unwrap();
    let operation = control::fresh_identity("unknown-effect-control").unwrap();
    let effect = RecordId::new("native_effect", operation.hex());
    let object = RecordId::new("native_guard", format!("unknown{}", operation.hex()));
    client.query("CREATE $effect SET attempt=NONE,request='control',committed=false,resolved=false,revision=0,epoch=1 RETURN NONE").bind(("effect",effect.clone())).await.unwrap().check().unwrap();
    let pending = client.as_ref().clone().begin().await.unwrap();
    pending.query("LET $fence=SELECT * FROM ONLY $effect FOR UPDATE; CREATE $object SET revision=0,retired=false RETURN NONE; UPDATE $effect SET committed=true,resolved=true,revision+=1 RETURN NONE").bind(("effect",effect)).bind(("object",object.clone())).await.unwrap().check().unwrap();
    assert!(!control::reconcile_effect(&client, operation).await.unwrap());
    assert!(
        pending.commit().await.is_err(),
        "fenced unknown transaction cannot commit after reconciliation"
    );
    let mut response = client
        .query("SELECT VALUE id FROM $object")
        .bind(("object", object.clone()))
        .await
        .unwrap()
        .check()
        .unwrap();
    let absent: Vec<RecordId> = response.take(0).unwrap();
    assert!(absent.is_empty());
    let mut row = Object::new();
    row.insert("id", object.clone());
    row.insert("revision", 0i64);
    row.insert("retired", false);
    control::ensure_rows(&client, None, vec![Value::Object(row)])
        .await
        .unwrap();
    let pin = control::ReaderPin::acquire(client.clone(), &[])
        .await
        .unwrap();
    pin.protect(object.clone()).await.unwrap();
    let first = control::retire_reachable(&client, vec![object], 1)
        .await
        .unwrap();
    assert_eq!(first.retired, 0);
    assert_eq!(first.retained.len(), 1);
    pin.release().await.unwrap();
    let next = control::resume_retirement(&client, first.identity, 4)
        .await
        .unwrap();
    assert_eq!(next.retired, 1);
    assert!(next.remaining.is_empty());
    assert!(next.retained.is_empty());
    drop(pin);

    // Controlled application acknowledgment discard after a real guarded native COMMIT.
    // This exercises reconnect/reconciliation, without claiming network failure injection.
    let committed = control::fresh_identity("committed-effect-ack-discard").unwrap();
    let receipt = RecordId::new("native_effect", committed.hex());
    let payload = RecordId::new("native_guard", format!("committed{}", committed.hex()));
    let guard = RecordId::new(
        "native_guard",
        ContentHash::of(&serde_json::to_vec(&Value::RecordId(payload.clone())).unwrap()).hex(),
    );
    let mut expected = Object::new();
    expected.insert("id", payload.clone());
    expected.insert("revision", 17i64);
    expected.insert("retired", false);
    let mut bindings = Variables::new();
    bindings.insert("effect", receipt.clone());
    bindings.insert("object", payload.clone());
    bindings.insert("guard", guard);
    bindings.insert("row", expected.clone());
    bindings.insert("request", committed.hex());
    client.query("BEGIN; LET $installation=SELECT * FROM ONLY native_installation:current FOR UPDATE; UPDATE native_installation:current SET admission_revision=(admission_revision ?? 0)+1 RETURN NONE; CREATE $effect SET attempt=NONE,request=$request,committed=false,resolved=false,revision=0,epoch=($installation.admission_revision ?? 0)+1 RETURN NONE; COMMIT;").bind(bindings.clone()).await.unwrap().check().unwrap();
    let discarded = client.query("BEGIN; LET $operation=SELECT * FROM ONLY $effect FOR UPDATE; IF $operation=NONE OR $operation.resolved OR $operation.epoch<=0 { THROW 'control committed effect fenced'; }; LET $before=SELECT * FROM ONLY $guard FOR UPDATE; IF $operation.epoch<=($before.retired_through ?? 0) { THROW 'control original effect epoch retired'; }; UPSERT $guard SET revision=(revision ?? 0)+1,retired=false RETURN NONE; CREATE $object CONTENT $row RETURN NONE; UPDATE $effect SET committed=true,resolved=true,revision+=1 RETURN NONE; COMMIT;").bind(bindings).await;
    drop(discarded); // The application learns success only from the fresh-session receipt.
    client.invalidate().await.unwrap();
    drop(client);
    let client = lctx_surrealdb::compiler::check_installation(&cfg)
        .await
        .unwrap();
    let reader = lctx_surrealdb::NativeReader::private(client.clone());
    let result = async {
        let first = control::reconcile_effect(&client, committed).await?;
        let mut bindings = Variables::new();
        bindings.insert("object", payload.clone());
        bindings.insert("effect", receipt.clone());
        let payloads: Vec<Object> = reader
            .query_native("SELECT * FROM $object", bindings.clone())
            .await?;
        let before: Vec<Object> = reader
            .query_native("SELECT * FROM $effect", bindings.clone())
            .await?;
        let second = control::reconcile_effect(&client, committed).await?;
        let after: Vec<Object> = reader
            .query_native("SELECT * FROM $effect", bindings)
            .await?;
        Ok::<_, ModelError>((first, second, payloads, before, after))
    }
    .await;
    let mut completion = lctx_model::domain::completion::Completion::default();
    completion.step(
        "committed acknowledgment control payload retirement",
        control::retire(&client, payload).await,
    );
    completion.step(
        "committed acknowledgment control reader close",
        reader.close().await,
    );
    completion.step(
        "committed acknowledgment control session invalidation",
        client
            .invalidate()
            .await
            .map_err(|error| ModelError::Cause(Box::new(error))),
    );
    let (first, second, payloads, before, after) =
        lctx_model::domain::completion::complete(result, completion).unwrap();
    assert!(
        first && second,
        "committed acknowledgment reconciliation is repeatable after session loss"
    );
    assert_eq!(
        payloads,
        vec![expected],
        "one exact committed payload survives session loss"
    );
    assert_eq!(
        before, after,
        "receipt reconciliation cannot replay the committed decision"
    );
    let [receipt_row] = before.as_slice() else {
        panic!("one exact committed effect receipt");
    };
    assert_eq!(receipt_row.get("id"), Some(&Value::RecordId(receipt)));
    assert_eq!(
        receipt_row.get("request"),
        Some(&Value::String(committed.hex()))
    );
    assert_eq!(receipt_row.get("committed"), Some(&Value::Bool(true)));
    assert_eq!(receipt_row.get("resolved"), Some(&Value::Bool(true)));
    assert_eq!(
        receipt_row.get("revision"),
        Some(&Value::Number(surrealdb::types::Number::Int(1)))
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn completed_state_is_exact_binding_closure_in_current_and_cold_owners() {
    let cfg = config();
    let store = NativeCompilerStore::begin(&cfg, Frontier::Facts)
        .await
        .unwrap();
    let nonce = control::fresh_identity("state-closure-control").unwrap();
    let mut run = nonce.0[..16].try_into().unwrap();
    let (_, first) = contribute(
        &store,
        specification(format!("first{}", nonce.hex())),
        &row(run, 1.0),
        &BTreeMap::new(),
    )
    .await;
    run[0] ^= 1;
    let (_, second) = contribute(
        &store,
        specification(format!("second{}", nonce.hex())),
        &row(run, 2.0),
        &first,
    )
    .await;
    let relation = Relation::of::<QualityStep>();
    let view = second[QualityStep::NAME].clone();
    store
        .bind(CompletedBinding {
            boundary: None,
            source: SourceSnapshot::of_completed_view(
                &relation,
                specification(String::new()).model,
                &view,
            )
            .unwrap(),
            view,
            configuration: None,
        })
        .await
        .unwrap();
    assert_eq!(
        store.views().await.unwrap().len(),
        1,
        "unbound singleton view is not portable state authority"
    );
    let identity = store.completed_state().await.unwrap();
    let bindings = store.bindings().await.unwrap();
    let client = lctx_surrealdb::compiler::check_installation(&cfg)
        .await
        .unwrap();
    let cold = NativeCompilerStore::from_publication(
        client,
        cfg.namespace.clone(),
        cfg.database.clone(),
        bindings,
    )
    .await
    .unwrap();
    cold.verify_state().await.unwrap();
    assert_eq!(cold.completed_state().await.unwrap(), identity);
    run[0] ^= 2;
    let (_, unbound) = contribute(
        &store,
        specification(format!("unbound{}", nonce.hex())),
        &row(run, 3.0),
        &BTreeMap::new(),
    )
    .await;
    assert_eq!(
        store.completed_state().await.unwrap(),
        identity,
        "unbound completed rows remain outside captured state"
    );
    let detached = tempfile::tempdir().unwrap();
    assert_eq!(
        store
            .export_state(&detached.path().join("before.jsonl"))
            .await
            .unwrap(),
        identity
    );
    let view = unbound[QualityStep::NAME].clone();
    store
        .bind(CompletedBinding {
            boundary: None,
            source: SourceSnapshot::of_completed_view(
                &relation,
                specification(String::new()).model,
                &view,
            )
            .unwrap(),
            view,
            configuration: None,
        })
        .await
        .unwrap();
    let changed = store.completed_state().await.unwrap();
    assert_ne!(
        changed, identity,
        "a fresh operation captures changed bindings"
    );
    assert_eq!(
        store
            .export_state(&detached.path().join("after.jsonl"))
            .await
            .unwrap(),
        changed
    );
    assert_eq!(
        cold.completed_state().await.unwrap(),
        identity,
        "published binding inventory remains exact"
    );
    store.abandon().await.unwrap();
}
#[tokio::test(flavor = "multi_thread")]
async fn retirement_uses_exact_guards_without_blocking_on_unrelated_live_effects() {
    let client = lctx_surrealdb::compiler::check_installation(&config())
        .await
        .unwrap();
    let nonce = control::fresh_identity("parallel-retirement-control").unwrap();
    let unrelated = RecordId::new("native_effect", nonce.hex());
    let object = RecordId::new("native_guard", format!("late{}", nonce.hex()));
    let guard = RecordId::new(
        "native_guard",
        ContentHash::of(&serde_json::to_vec(&Value::RecordId(object.clone())).unwrap()).hex(),
    );
    client.query("CREATE $effect SET attempt=NONE,request='unrelated-active-control',committed=false,resolved=false,revision=0,epoch=1 RETURN NONE").bind(("effect",unrelated)).await.unwrap().check().unwrap();
    let pending = client.as_ref().clone().begin().await.unwrap();
    pending.query("UPSERT $guard SET revision=(revision ?? 0)+1,retired=(retired ?? false) RETURN NONE; CREATE $object SET revision=0,retired=false RETURN NONE").bind(("guard",guard)).bind(("object",object.clone())).await.unwrap().check().unwrap();
    assert!(
        control::retirement_eligibility(&client, object.clone())
            .await
            .unwrap()
            .eligible,
        "an unrelated active operation is not an object dependency"
    );
    control::retire(&client, object.clone()).await.unwrap();
    assert!(
        pending.commit().await.is_err(),
        "late immutable insertion must conflict with retirement's exact guard"
    );
    control::reconcile_effect(&client, nonce).await.unwrap();
    let mut response = client
        .query("SELECT VALUE id FROM $object")
        .bind(("object", object))
        .await
        .unwrap()
        .check()
        .unwrap();
    let rows: Vec<RecordId> = response.take(0).unwrap();
    assert!(rows.is_empty());
}

#[tokio::test(flavor = "multi_thread")]
async fn fresh_attempt_reactivates_content_but_original_attempt_effect_and_pin_epochs_stay_fenced()
{
    let cfg = config();
    let old = NativeCompilerStore::begin(&cfg, Frontier::Facts)
        .await
        .unwrap();
    let client = lctx_surrealdb::compiler::check_installation(&cfg)
        .await
        .unwrap();
    let nonce = control::fresh_identity("reactivation-control").unwrap();
    let object = RecordId::new("native_guard", format!("reactivate{}", nonce.hex()));
    let guard = RecordId::new(
        "native_guard",
        ContentHash::of(&serde_json::to_vec(&Value::RecordId(object.clone())).unwrap()).hex(),
    );
    let mut row = Object::new();
    row.insert("id", object.clone());
    row.insert("revision", 0i64);
    row.insert("retired", false);
    let value = Value::Object(row);
    control::ensure_rows(&client, Some(old.attempt()), vec![value.clone()])
        .await
        .unwrap();
    let old_pin = control::ReaderPin::acquire(client.clone(), &[])
        .await
        .unwrap();
    let mut response = client
        .query("SELECT VALUE epoch FROM $attempt")
        .bind((
            "attempt",
            RecordId::new("native_attempt", old.attempt().hex()),
        ))
        .await
        .unwrap()
        .check()
        .unwrap();
    let epochs: Vec<i64> = response.take(0).unwrap();
    let epoch = epochs[0];
    let delayed = RecordId::new("native_effect", nonce.hex());
    client.query("CREATE $effect SET attempt=NONE,request='delayed-unowned-control',committed=false,resolved=false,revision=0,epoch=$epoch RETURN NONE").bind(("effect",delayed.clone())).bind(("epoch",epoch)).await.unwrap().check().unwrap();
    let first = control::retire_reachable(&client, vec![object.clone()], 1)
        .await
        .unwrap();
    assert_eq!(first.retired, 1);
    let fresh = NativeCompilerStore::begin(&cfg, Frontier::Facts)
        .await
        .unwrap();
    control::ensure_rows(&client, Some(fresh.attempt()), vec![value.clone()])
        .await
        .unwrap();
    assert!(
        control::ensure_rows(&client, Some(old.attempt()), vec![value])
            .await
            .is_err(),
        "an earlier attempt never borrows the reactivating attempt epoch"
    );
    assert!(
        old_pin.protect(object.clone()).await.is_err(),
        "pin protection retains the original pin epoch"
    );
    let delayed_result=client.query("BEGIN; LET $effect_state=SELECT * FROM ONLY $effect FOR UPDATE; LET $original_epoch=$effect_state.epoch; LET $guard_state=SELECT * FROM ONLY $guard FOR UPDATE; IF $original_epoch<=($guard_state.retired_through ?? 0) { THROW 'delayed original epoch fenced'; }; UPDATE $effect SET committed=true,resolved=true RETURN NONE; COMMIT;").bind(("effect",delayed)).bind(("guard",guard.clone())).await.unwrap().check();
    assert!(
        delayed_result.is_err(),
        "reactivation does not erase the retired-through watermark for a delayed unowned intent"
    );
    control::reconcile_effect(&client, nonce).await.unwrap();
    let mut response = client
        .query("SELECT retired,retired_through FROM $guard")
        .bind(("guard", guard))
        .await
        .unwrap()
        .check()
        .unwrap();
    let guards: Vec<Object> = response.take(0).unwrap();
    assert_eq!(guards[0].get("retired"), Some(&Value::Bool(false)));
    assert!(
        matches!(guards[0].get("retired_through"),Some(Value::Number(surrealdb::types::Number::Int(mark))) if *mark>=epoch)
    );
    old_pin.release().await.unwrap();
    let second = control::retire_reachable(&client, vec![object], 1)
        .await
        .unwrap();
    assert_eq!(second.identity, first.identity);
    assert_eq!(
        second.retired, 2,
        "explicit retirement revisits a legally reactivated root"
    );
    old.abandon().await.unwrap();
    fresh.abandon().await.unwrap();
}
#[tokio::test(flavor = "multi_thread")]
async fn retained_product_keeps_exact_prerequisite_closure_after_origin_owner_retirement() {
    let cfg = config();
    let origin = NativeCompilerStore::begin(&cfg, Frontier::Facts)
        .await
        .unwrap();
    let nonce = control::fresh_identity("retained-prerequisite-control").unwrap();
    let relation = Relation::of::<QualityStep>();
    let run = nonce.0[..16].try_into().unwrap();
    let (_, base) = contribute(
        &origin,
        specification(format!("base{}", nonce.hex())),
        &row(run, 1.0),
        &BTreeMap::new(),
    )
    .await;
    let input = SourceSnapshot::of_completed_view(
        &relation,
        specification(String::new()).model,
        &base[QualityStep::NAME],
    )
    .unwrap();
    let mut derived_spec = specification(format!("derived{}", nonce.hex()));
    derived_spec.inputs = vec![input];
    let mut derived_run = run;
    derived_run[0] ^= 1;
    let (derived, derived_views) = contribute(
        &origin,
        derived_spec.clone(),
        &row(derived_run, 2.0),
        &BTreeMap::new(),
    )
    .await;
    let view = derived_views[QualityStep::NAME].clone();
    origin
        .bind(CompletedBinding {
            boundary: None,
            source: SourceSnapshot::of_completed_view(&relation, derived_spec.model, &view)
                .unwrap(),
            view,
            configuration: None,
        })
        .await
        .unwrap();
    origin
        .retain_product_identity(nonce, derived)
        .await
        .unwrap();
    origin.mark_attempt_admitted().await.unwrap();
    let client = lctx_surrealdb::compiler::check_installation(&cfg)
        .await
        .unwrap();
    let owner = RecordId::new("native_guard", nonce.hex());
    let mut owner_row = Object::new();
    owner_row.insert("id", owner.clone());
    owner_row.insert("revision", 0i64);
    owner_row.insert("retired", false);
    control::ensure_rows(
        &client,
        Some(origin.attempt()),
        vec![Value::Object(owner_row)],
    )
    .await
    .unwrap();
    control::hold(
        &client,
        Some(origin.attempt()),
        owner.clone(),
        vec![
            RecordId::new(
                "compiler_view",
                derived_views[QualityStep::NAME].identity.hex(),
            ),
            RecordId::new("compiler_view", base[QualityStep::NAME].identity.hex()),
        ],
    )
    .await
    .unwrap();
    origin.abandon().await.unwrap();
    let retired = control::retire_reachable(&client, vec![owner], 64)
        .await
        .unwrap();
    assert!(retired.retired >= 1);
    let target = NativeCompilerStore::begin(&cfg, Frontier::Facts)
        .await
        .unwrap();
    let (attached, _, descriptor) = target
        .attach_retained_product(nonce, &derived_spec)
        .await
        .unwrap()
        .expect(
            "admitted product retains its prerequisite closure independently of the origin owner",
        );
    assert_eq!(descriptor.spec.inputs, derived_spec.inputs);
    let mut response = client
        .query("SELECT VALUE object FROM native_hold WHERE owner=$owner")
        .bind((
            "owner",
            RecordId::new("compiler_contribution", attached.hex()),
        ))
        .await
        .unwrap()
        .check()
        .unwrap();
    let targets: Vec<RecordId> = response.take(0).unwrap();
    assert!(targets.contains(&RecordId::new(
        "compiler_view",
        base[QualityStep::NAME].identity.hex()
    )));
    let mut rows = target
        .scan_rows(
            &base[QualityStep::NAME],
            &relation,
            None,
            None,
            &ResourceBudget::fixed(64 << 20).unwrap(),
        )
        .await
        .unwrap();
    assert!(rows.next().await.unwrap().is_some());
    assert!(rows.next().await.unwrap().is_none());
    drop(rows);
    target.abandon().await.unwrap();
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "database-wide backup exclusion requires explicit owned-service maintenance"]
async fn backup_hold_excludes_retirement_under_exclusive_maintenance() {
    let path = std::env::var_os("LCTX_SURREAL_INSTALLER_CONFIG")
        .expect("explicit validation maintenance installer configuration");
    let cfg = RuntimeConfig::read(std::path::Path::new(&path)).unwrap();
    assert_eq!(
        cfg.authentication,
        lctx_surrealdb::AuthenticationScope::Root
    );
    let client = lctx_surrealdb::compiler::check_installation(&cfg)
        .await
        .unwrap();
    let nonce = control::fresh_identity("backup-exclusion-control").unwrap();
    let object = RecordId::new("native_guard", nonce.hex());
    let mut row = Object::new();
    row.insert("id", object.clone());
    row.insert("revision", 0i64);
    row.insert("retired", false);
    control::ensure_rows(&client, None, vec![Value::Object(row)])
        .await
        .unwrap();
    let backup = control::acquire_backup_hold(&client).await.unwrap();
    let result = async {
        let eligibility = control::retirement_eligibility(&client, object.clone()).await?;
        let retirement = control::retire(&client, object.clone()).await;
        Ok::<_, lctx_model::domain::ModelError>((eligibility, retirement))
    }
    .await;
    let mut completion = lctx_model::domain::completion::Completion::default();
    completion.step(
        "backup control hold release",
        control::release_backup_hold(&client, backup).await,
    );
    completion.step(
        "backup control synthetic object retirement",
        control::retire(&client, object).await,
    );
    let (eligibility, retirement) =
        lctx_model::domain::completion::complete(result, completion).unwrap();
    assert!(!eligibility.eligible);
    assert!(
        eligibility
            .reasons
            .contains(&"active native backup hold".to_string())
    );
    assert!(retirement.is_err());
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "abandoned reader recovery requires explicit owned-service maintenance"]
async fn maintenance_reconciliation_requires_drain_proof_and_fences_only_named_clients() {
    assert!(
        std::env::var_os("LCTX_SURREAL_MAINTENANCE_TOKEN").is_some(),
        "run through explicit native-client maintenance"
    );
    let path = std::env::var_os("LCTX_SURREAL_INSTALLER_CONFIG")
        .expect("explicit validation maintenance installer configuration");
    let cfg = RuntimeConfig::read(std::path::Path::new(&path)).unwrap();
    assert_eq!(
        cfg.authentication,
        lctx_surrealdb::AuthenticationScope::Root
    );
    let client = lctx_surrealdb::compiler::check_installation(&cfg)
        .await
        .unwrap();
    let object = RecordId::new(
        "native_guard",
        control::fresh_identity("maintenance-reader-target")
            .unwrap()
            .hex(),
    );
    let mut row = Object::new();
    row.insert("id", object.clone());
    row.insert("revision", 0i64);
    row.insert("retired", false);
    control::ensure_rows(&client, None, vec![Value::Object(row)])
        .await
        .unwrap();
    let first = control::ReaderPin::acquire(client.clone(), &[])
        .await
        .unwrap();
    let unrelated = control::ReaderPin::acquire(client.clone(), &[])
        .await
        .unwrap();
    let backup = control::acquire_backup_hold(&client).await.unwrap();
    let result=async {
        first.protect(object.clone()).await?;
        let pins=[first.identity];let holds=[backup];
        let open_refused=lctx_surrealdb::compiler::reconcile_maintenance(&cfg,&pins,&holds,true).await.is_err();
        let mut ordinary=RuntimeConfig::read(std::path::Path::new(&path)).map_err(lctx_model::domain::ModelError::codec)?;ordinary.authentication=lctx_surrealdb::AuthenticationScope::Database;
        let authority_refused=lctx_surrealdb::compiler::reconcile_maintenance(&ordinary,&pins,&holds,true).await.is_err();
        lctx_surrealdb::compiler::close_admission(&cfg).await?;
        let live_refused=lctx_surrealdb::compiler::reconcile_maintenance(&cfg,&pins,&holds,false).await.is_err();
        let mut response=client.query("SELECT VALUE released FROM ONLY $pin").bind(("pin",RecordId::new("native_pin",first.identity.hex()))).await.map_err(lctx_model::domain::ModelError::codec)?.check().map_err(lctx_model::domain::ModelError::codec)?;
        let before:Option<bool>=response.take(0).map_err(lctx_model::domain::ModelError::codec)?;
        lctx_surrealdb::compiler::reconcile_maintenance(&cfg,&pins,&holds,true).await?;
        lctx_surrealdb::compiler::reconcile_maintenance(&cfg,&pins,&holds,true).await?;
        let unknown=control::fresh_identity("missing-maintenance-pin")?;
        let missing_refused=match lctx_surrealdb::compiler::reconcile_maintenance(&cfg,&[first.identity,unknown],&[],true).await {Err(lctx_model::domain::ModelError::Completion(outcome))=>outcome.completion.committed.iter().any(|effect|effect.kind=="reconciled native reader pin" && effect.identity==first.identity.hex()),_=>false};
        let fenced=first.protect(object.clone()).await.is_err();
        let mut response=client.query("SELECT VALUE released FROM ONLY $pin; SELECT VALUE active FROM ONLY $backup; SELECT VALUE released FROM ONLY $unrelated").bind(("pin",RecordId::new("native_pin",first.identity.hex()))).bind(("backup",RecordId::new("native_backup_hold",backup.hex()))).bind(("unrelated",RecordId::new("native_pin",unrelated.identity.hex()))).await.map_err(lctx_model::domain::ModelError::codec)?.check().map_err(lctx_model::domain::ModelError::codec)?;
        let released:Option<bool>=response.take(0).map_err(lctx_model::domain::ModelError::codec)?;let active:Option<bool>=response.take(1).map_err(lctx_model::domain::ModelError::codec)?;let unrelated_released:Option<bool>=response.take(2).map_err(lctx_model::domain::ModelError::codec)?;
        Ok::<_,lctx_model::domain::ModelError>((open_refused,authority_refused,live_refused,before,missing_refused,fenced,released,active,unrelated_released))
    }.await;
    let mut completion = lctx_model::domain::completion::Completion::default();
    completion.step(
        "maintenance control first pin cleanup",
        first.release().await,
    );
    completion.step(
        "maintenance control unrelated pin cleanup",
        unrelated.release().await,
    );
    completion.step(
        "maintenance control backup cleanup",
        control::release_backup_hold(&client, backup).await,
    );
    completion.step(
        "maintenance control target retirement",
        control::retire(&client, object).await,
    );
    completion.step(
        "maintenance control admission restoration",
        lctx_surrealdb::compiler::open_admission(&cfg).await,
    );
    let (
        open_refused,
        authority_refused,
        live_refused,
        before,
        missing_refused,
        fenced,
        released,
        active,
        unrelated_released,
    ) = lctx_model::domain::completion::complete(result, completion).unwrap();
    assert!(open_refused && authority_refused && live_refused && missing_refused && fenced);
    assert_eq!(before, Some(false));
    assert_eq!(released, Some(true));
    assert_eq!(active, Some(false));
    assert_eq!(unrelated_released, Some(false));
}
