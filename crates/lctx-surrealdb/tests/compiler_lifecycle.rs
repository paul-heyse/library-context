//! Actual native admission closure races a cold view registration and held transport.
use lctx_model::domain::{
    ContentHash, Record, Relation,
    admission::Frontier,
    completed::ContributionSpec,
    input::Package,
    stages::{Profile, ProviderOutcome},
};
use lctx_surrealdb::{RuntimeConfig, compiler::NativeCompilerStore};
use std::collections::{BTreeMap, BTreeSet};

async fn fixture_admin(
    store: &NativeCompilerStore,
    config: &RuntimeConfig,
) -> std::sync::Arc<
    lctx_surrealdb::surrealdb::Surreal<lctx_surrealdb::surrealdb::engine::remote::grpc::Client>,
> {
    lctx_surrealdb::reader::connect(
        &config.endpoint,
        &config.root_credentials(),
        store.namespace().as_str(),
        store.database().as_str(),
    )
    .await
    .unwrap()
}

#[tokio::test(flavor = "multi_thread")]
async fn closing_admission_waits_for_cold_registration_and_held_native_rows() {
    let path = std::path::PathBuf::from(
        std::env::var_os("LCTX_COMPILER_RUNTIME_CONFIG").expect("owned persistent native fixture"),
    );
    let config = RuntimeConfig::read(&path).unwrap();
    let store = NativeCompilerStore::begin(&config, Frontier::Facts)
        .await
        .unwrap();
    let relation = Relation::of::<Package>();
    let specification = ContributionSpec {
        captured_binding: None,
        producer: "lifecycle".into(),
        profile: Profile::Catalog,
        model: ContentHash::of(b"lifecycle-model"),
        implementation: ContentHash::of(b"lifecycle-code"),
        configuration: None,
        inputs: vec![],
        outputs: BTreeSet::from([relation.name().into()]),
    };
    let id = store
        .begin_contribution(specification.clone())
        .await
        .unwrap();
    store
        .write_batch(
            &id,
            &relation,
            &Package::encode(&[Package {
                name: "retained".into(),
            }])
            .unwrap(),
        )
        .await
        .unwrap();
    let views = store
        .complete_contribution(
            id,
            ProviderOutcome::Complete,
            std::slice::from_ref(&relation),
            &BTreeMap::new(),
        )
        .await
        .unwrap();
    let admin = fixture_admin(&store, &config).await;
    let cold = NativeCompilerStore::from_existing(
        admin,
        store.namespace().clone(),
        store.database().clone(),
    );
    let budget = lctx_model::domain::resources::ResourceBudget::fixed(32 << 20).unwrap();
    let mut registration =
        Box::pin(cold.scan_rows(&views[relation.name()], &relation, None, None, &budget));
    assert!(
        futures::poll!(&mut registration).is_pending(),
        "cold registration reaches its SDK metadata await"
    );
    let final_owner = cold.clone();
    let finalization = tokio::spawn(async move { final_owner.end_writes().await });
    tokio::time::sleep(std::time::Duration::from_millis(5)).await;
    assert!(
        !finalization.is_finished(),
        "admission was recorded before metadata await"
    );
    assert!(
        cold.begin_contribution(specification).await.is_err(),
        "closure refuses new work"
    );
    let rows = registration.await.unwrap();
    assert!(
        !finalization.is_finished(),
        "returned transport retains admission through terminal drain"
    );
    drop(rows);
    tokio::time::timeout(std::time::Duration::from_secs(10), finalization)
        .await
        .unwrap()
        .unwrap()
        .unwrap();
    assert!(cold.check().is_err());
    store.abandon().await.unwrap();
}

#[tokio::test(flavor = "multi_thread")]
async fn final_content_freeze_preserves_reads_and_reuses_complete_canonical_runs() {
    let path = std::path::PathBuf::from(
        std::env::var_os("LCTX_COMPILER_RUNTIME_CONFIG").expect("owned persistent native fixture"),
    );
    let config = RuntimeConfig::read(&path).unwrap();
    let store = NativeCompilerStore::begin(&config, Frontier::Facts)
        .await
        .unwrap();
    let relation = Relation::of::<Package>();
    let spec = ContributionSpec {
        captured_binding: None,
        producer: "frozen".into(),
        profile: Profile::Catalog,
        model: ContentHash::of(b"model"),
        implementation: ContentHash::of(b"code"),
        configuration: None,
        inputs: vec![],
        outputs: BTreeSet::from([relation.name().into()]),
    };
    let owner = store.begin_contribution(spec.clone()).await.unwrap();
    store
        .write_batch(
            &owner,
            &relation,
            &Package::encode(&[
                Package {
                    name: "first".into(),
                },
                Package {
                    name: "second".into(),
                },
            ])
            .unwrap(),
        )
        .await
        .unwrap();
    store
        .complete_contribution(
            owner,
            ProviderOutcome::Complete,
            std::slice::from_ref(&relation),
            &BTreeMap::new(),
        )
        .await
        .unwrap();
    let inventory = store.freeze_content().await.unwrap();
    assert_eq!(inventory.contributions.len(), 1);
    assert_eq!(
        inventory.contributions[0],
        store.completed_contribution(owner).await.unwrap()
    );
    assert!(store.begin_contribution(spec).await.is_err());
    assert!(store.set_frontier(Frontier::Facts).is_err());
    assert!(
        store
            .write_batch(
                &owner,
                &relation,
                &Package::encode(&[Package {
                    name: "late".into()
                }])
                .unwrap()
            )
            .await
            .is_err()
    );
    let budget = lctx_model::domain::resources::ResourceBudget::fixed(32 << 20).unwrap();
    store.prepare_canonical(&budget).await.unwrap();
    let run_charge = budget.reserved();
    assert!(run_charge > 0);
    store.prepare_canonical(&budget).await.unwrap();
    assert_eq!(budget.reserved(), run_charge);
    let mut first = store.scan_canonical(true, &budget).await.unwrap();
    let mut second = store.scan_canonical(true, &budget).await.unwrap();
    assert_eq!(first.next().await.unwrap(), second.next().await.unwrap());
    drop(first);
    assert!(second.next().await.unwrap().is_some());
    assert!(second.next().await.unwrap().is_none());
    drop(second);
    let mut empty = store.scan_canonical(false, &budget).await.unwrap();
    assert!(empty.next().await.unwrap().is_none());
    drop(empty);
    store.prepare_canonical(&budget).await.unwrap();
    store.check_publication_target(&config).unwrap();
    let wrong_target = RuntimeConfig {
        endpoint: "grpc://127.0.0.1:1".into(),
        ..config
    };
    assert!(store.check_publication_target(&wrong_target).is_err());
    store.abandon().await.unwrap();
    drop(store);
    assert_eq!(budget.reserved(), 0);
}
#[tokio::test(flavor = "multi_thread")]
async fn final_content_freeze_rejects_actual_pending_owner_on_cold_store() {
    let path = std::path::PathBuf::from(
        std::env::var_os("LCTX_COMPILER_RUNTIME_CONFIG").expect("owned persistent native fixture"),
    );
    let config = RuntimeConfig::read(&path).unwrap();
    let store = NativeCompilerStore::begin(&config, Frontier::Facts)
        .await
        .unwrap();
    let relation = Relation::of::<Package>();
    store
        .begin_contribution(ContributionSpec {
            captured_binding: None,
            producer: "pending".into(),
            profile: Profile::Catalog,
            model: ContentHash::of(b"model"),
            implementation: ContentHash::of(b"code"),
            configuration: None,
            inputs: vec![],
            outputs: BTreeSet::from([relation.name().into()]),
        })
        .await
        .unwrap();
    let cold = NativeCompilerStore::from_existing(
        fixture_admin(&store, &config).await,
        store.namespace().clone(),
        store.database().clone(),
    );
    assert!(cold.contributions().await.unwrap().is_empty());
    assert!(cold.freeze_content().await.is_err());
    assert!(
        cold.prepare_canonical(
            &lctx_model::domain::resources::ResourceBudget::fixed(1 << 20).unwrap()
        )
        .await
        .is_err()
    );
    store.abandon().await.unwrap();
}

#[tokio::test(flavor = "multi_thread")]
async fn acknowledged_setup_authentication_failure_has_a_failed_phase_terminal() {
    use std::{
        io::{self, Write},
        sync::{Arc, Mutex},
    };
    use tracing::instrument::WithSubscriber;
    #[derive(Clone, Default)]
    struct Output(Arc<Mutex<Vec<u8>>>);
    impl Write for Output {
        fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
            self.0.lock().unwrap().extend_from_slice(bytes);
            Ok(bytes.len())
        }
        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }
    let path = std::path::PathBuf::from(
        std::env::var_os("LCTX_COMPILER_RUNTIME_CONFIG").expect("owned persistent native fixture"),
    );
    let mut config = RuntimeConfig::read(&path).unwrap();
    config.password.push_str("-intentional-refusal");
    let output = Output::default();
    let writer = output.clone();
    let dispatch = tracing::Dispatch::new(
        tracing_subscriber::fmt()
            .without_time()
            .with_ansi(false)
            .with_writer(move || writer.clone())
            .finish(),
    );
    let result = NativeCompilerStore::begin(&config, Frontier::Facts)
        .with_subscriber(dispatch)
        .await;
    assert!(result.is_err());
    let retained = String::from_utf8(output.0.lock().unwrap().clone()).unwrap();
    let lines = retained
        .lines()
        .filter(|line| line.contains("phase=\"native_setup\""))
        .collect::<Vec<_>>();
    assert_eq!(lines.len(), 2, "{retained}");
    assert!(lines[0].contains("status=\"begin\""));
    assert!(lines[1].contains("status=\"failed\""));
    assert!(!retained.contains(&config.password));
}

#[tokio::test(flavor = "multi_thread")]
async fn native_producing_completion_waits_own_rows_without_waiting_unrelated_reader() {
    use lctx_model::domain::{analysis::sources::SourceSnapshot, resources::ResourceBudget};
    let path = std::path::PathBuf::from(std::env::var_os("LCTX_COMPILER_RUNTIME_CONFIG").expect("owned persistent native fixture"));
    let config = RuntimeConfig::read(&path).unwrap();
    let store = NativeCompilerStore::begin(&config, Frontier::Facts).await.unwrap();
    let relation = Relation::of::<Package>();
    let mut specification = ContributionSpec {
        captured_binding: None, producer: "scoped-source".into(), profile: Profile::Catalog,
        model: ContentHash::of(b"scoped-model"), implementation: ContentHash::of(b"scoped-code"),
        configuration: None, inputs: vec![], outputs: BTreeSet::from([relation.name().into()]),
    };
    let id = store.begin_contribution(specification.clone()).await.unwrap();
    store.write_batch(&id, &relation, &Package::encode(&[Package { name: "scoped-source-row".into() }]).unwrap()).await.unwrap();
    let views = store.complete_contribution(id, ProviderOutcome::Complete, std::slice::from_ref(&relation), &BTreeMap::new()).await.unwrap();
    let frozen = &views[relation.name()];
    let budget = ResourceBudget::fixed(32 << 20).unwrap();
    let unrelated = store.scan_rows(frozen, &relation, None, None, &budget).await.unwrap();

    specification.producer = "scoped-dependent".into();
    specification.inputs.push(SourceSnapshot::of_completed_view(&relation, specification.model, frozen).unwrap());
    let scope = store.producing_scope(specification.identity().unwrap(), &budget).unwrap();
    let id = scope.run(store.begin_contribution(specification)).await.unwrap();
    let mut own = scope.run(store.scan_rows(frozen, &relation, None, None, &budget)).await.unwrap();
    let mut completion = Box::pin(store.complete_contribution_scoped(&scope, id, ProviderOutcome::Complete, std::slice::from_ref(&relation), &views));
    assert!(futures::poll!(&mut completion).is_pending(), "own producing stream is not terminal");
    // Payload hydration starts after local completion closed root admission. The exact
    // returned stream carries its admitted scope rather than relying on task inheritance.
    assert!(own.next().await.unwrap().is_some());
    assert!(own.next().await.unwrap().is_none());
    let next = completion.await.unwrap();
    assert_eq!(next[relation.name()].rows, 1, "an unrelated retained reader cannot block local completion");
    let mut final_close = Box::pin(store.end_writes());
    assert!(futures::poll!(&mut final_close).is_pending(), "final closure still owns every retained native reader");
    drop(unrelated);
    final_close.await.unwrap();
    store.abandon().await.unwrap();
}
