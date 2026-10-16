//! Partial semantic graphs exercise scoped native kernels, never publication admission.
use lctx_compiler_store::compiler::NativeCompilerStore;
use lctx_model::domain::{
    ContentHash, ModelError, Relation,
    admission::Frontier,
    completed::ContributionSpec,
    graph::{Assertion, Entity},
    resources::ResourceBudget,
    stages::{Profile, ProviderOutcome},
};
use lctx_native::surrealdb;
use lctx_native::{NativeReader, RuntimeConfig};
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::Arc,
};

pub struct ScopedFixture {
    pub reader: NativeReader<()>,
    pub store: Arc<NativeCompilerStore>,
    #[allow(
        dead_code,
        reason = "Retains exact installed-view leases through all scoped fixture consumers"
    )]
    pub installed: Vec<Arc<lctx_native::installed_view::HeldInstalledView>>,
}
impl std::ops::Deref for ScopedFixture {
    type Target = NativeReader<()>;
    fn deref(&self) -> &Self::Target {
        &self.reader
    }
}
impl ScopedFixture {
    pub async fn close(&self) -> Result<(), ModelError> {
        self.reader.close().await?;
        self.store.abandon().await
    }
}
pub fn config() -> RuntimeConfig {
    RuntimeConfig::read(std::path::Path::new(
        &std::env::var_os("LCTX_COMPILER_RUNTIME_CONFIG").expect("stable validation runtime"),
    ))
    .unwrap()
}
pub async fn reader(
    config: &RuntimeConfig,
    entities: &[Entity],
    assertions: &[Assertion],
) -> Result<ScopedFixture, ModelError> {
    let store = NativeCompilerStore::begin(
        config,
        Frontier::Facts,
        &lctx_compiler_store::compiler::recovery_factory(
            config,
            (lctx_model::domain::resources::ResourceBudget::fixed(
                lctx_model::domain::resources::DEFAULT_MEMORY_BYTES,
            )
            .expect("owned native control recovery budget"))
            .clone(),
        )
        .expect("owned native control factory"),
    )
    .await?;
    let model = lctx_model::domain::model()?;
    let budget = ResourceBudget::fixed(64 << 20)?;
    let mut groups = BTreeMap::<String, Vec<surrealdb::types::Value>>::new();
    for view in lctx_native::codec::entity_views(entities)?
        .into_iter()
        .chain(lctx_native::codec::assertion_views(assertions)?)
    {
        let surrealdb::types::Value::Object(mut body) = view.body else {
            return Err(ModelError::Schema("scoped fixture body"));
        };
        body.insert("id", view.semantic_key);
        groups
            .entry(view.semantic_type)
            .or_default()
            .push(surrealdb::types::Value::Object(body));
    }
    let relations = groups
        .keys()
        .map(|name| lctx_native::adapter::select(name).map(|adapter| adapter.relation.clone()))
        .collect::<Result<Vec<Relation>, _>>()?;
    let contribution = store
        .begin_contribution(ContributionSpec {
            captured_binding: None,
            producer: "scoped-native-kernel-fixture".into(),
            profile: lctx_model::domain::completed::OutputProfile::Specific(Profile::Catalog),
            model: model.digest(),
            implementation: ContentHash::of(b"scoped-native-kernel-fixture/v1"),
            configuration: None,
            inputs: vec![],
            outputs: groups.keys().cloned().collect::<BTreeSet<_>>(),
        })
        .await?;
    for relation in &relations {
        let batch = lctx_native::codec::decode_bodies(
            relation,
            groups.remove(relation.name()).unwrap(),
            &budget,
        )?;
        store
            .write_batch(&contribution, relation, &batch, &budget)
            .await?;
    }
    let views = store
        .complete_contribution(
            contribution,
            ProviderOutcome::Complete,
            &relations,
            &BTreeMap::new(),
            &budget,
        )
        .await?;
    let client = lctx_compiler_store::compiler::check_installation(config).await?;
    let selected = views.values().map(|v| v.identity).collect::<Vec<_>>();
    let derivation = ContentHash::of(
        format!("scoped-native-kernel-fixture/{}", store.attempt().hex()).as_bytes(),
    );
    let loader =
        lctx_native::Loader::for_attempt_views(client.clone(), store.attempt(), selected.clone())
            .with_derivation(derivation)
            .with_budget(&budget);
    loader.entity_references(entities).await?;
    loader.assertion_references(assertions).await?;
    let descriptors = views.values().cloned().collect::<Vec<_>>();
    let reader = store
        .reader_for_views(&descriptors, &budget)
        .await?
        .with_derivation(derivation);
    let mut installed = Vec::new();
    for view in &descriptors {
        installed.push(store.installed_view(view, &budget).await?);
    }
    Ok(ScopedFixture {
        reader,
        store,
        installed,
    })
}
