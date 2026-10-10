//! Partial semantic graphs exercise scoped native kernels, never publication admission.
use lctx_model::domain::{
    ContentHash, ModelError, Relation,
    admission::Frontier,
    completed::ContributionSpec,
    graph::{Assertion, Entity},
    resources::ResourceBudget,
    stages::{Profile, ProviderOutcome},
};
use lctx_surrealdb::{NativeReader, RuntimeConfig, compiler::NativeCompilerStore};
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::Arc,
};

pub struct ScopedFixture {
    pub reader: NativeReader<()>,
    pub store: Arc<NativeCompilerStore>,
    #[allow(
        dead_code,
        reason = "Some kernel controls require the exact physical owner"
    )]
    pub contribution: ContentHash,
    #[allow(
        dead_code,
        reason = "Some kernel controls prepare additional scoped native operators"
    )]
    pub views: Vec<ContentHash>,
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
    let store = NativeCompilerStore::begin(config, Frontier::Facts).await?;
    let model = lctx_model::domain::model()?;
    let budget = ResourceBudget::fixed(64 << 20)?;
    let mut groups = BTreeMap::<String, Vec<surrealdb::types::Value>>::new();
    for view in lctx_surrealdb::codec::entity_views(entities)?
        .into_iter()
        .chain(lctx_surrealdb::codec::assertion_views(assertions)?)
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
        .map(|name| {
            model
                .relation(name)
                .cloned()
                .ok_or(ModelError::Schema("scoped fixture typed relation"))
        })
        .collect::<Result<Vec<Relation>, _>>()?;
    let contribution = store
        .begin_contribution(ContributionSpec {
            captured_binding: None,
            producer: "scoped-native-kernel-fixture".into(),
            profile: Profile::Catalog,
            model: model.digest(),
            implementation: ContentHash::of(b"scoped-native-kernel-fixture/v1"),
            configuration: None,
            inputs: vec![],
            outputs: groups.keys().cloned().collect::<BTreeSet<_>>(),
        })
        .await?;
    for relation in &relations {
        let batch = lctx_surrealdb::codec::decode_bodies(
            relation,
            groups.remove(relation.name()).unwrap(),
            &budget,
        )?;
        store.write_batch(&contribution, relation, &batch).await?;
    }
    let views = store
        .complete_contribution(contribution,
            ProviderOutcome::Complete,
            &relations,
            &BTreeMap::new(), &budget)
        .await?;
    let client = lctx_surrealdb::compiler::check_installation(config).await?;
    let loader = lctx_surrealdb::Loader::for_attempt_views(
        client.clone(),
        store.attempt(),
        views.values().map(|view| view.identity).collect(),
    ).with_budget(&budget);
    loader.entity_references(entities).await?;
    loader.assertion_references(assertions).await?;
    let views = views.values().map(|v| v.identity).collect::<Vec<_>>();
    Ok(ScopedFixture {
        reader: NativeReader::for_views(client, views.clone()).with_budget(&budget),
        store,
        contribution,
        views,
    })
}
