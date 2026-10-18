//! Lower one sealed private compiler result into genuine native contributions.
//!
//! Descriptors select actual protected private rows; they never construct native
//! installation or admission capabilities. Every native descriptor and composed
//! view is independently earned and compared before a final binding is installed.
use super::{Workspace, private_source::PrivateRelation};
use futures::{FutureExt, StreamExt, future::BoxFuture, stream::FuturesUnordered};
use lctx_compiler_store::compiler::NativeCompilerStore;
use lctx_model::domain::{
    ContentHash, ModelError, ValidatedModel,
    charged::StateCharge,
    completed::{CompletedBinding, CompletedContribution, CompletedView},
    resources::ResourceBudget,
    stages::ProviderOutcome,
};
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::Arc,
};

pub(super) struct PrivateProducer {
    pub descriptor: CompletedContribution,
    pub outputs: BTreeMap<String, Arc<PrivateRelation>>,
}
struct LoweringPlan {
    required: BTreeSet<usize>,
    graph: ReadyGraph,
    views: BTreeMap<ContentHash, CompletedView>,
    ids: Vec<ContentHash>,
    _charge: StateCharge,
}
impl LoweringPlan {
    fn new(
        producers: &[PrivateProducer],
        bindings: &[CompletedBinding],
        dependency_views: Vec<CompletedView>,
        qualification_views: &[CompletedView],
        model: &ValidatedModel,
        budget: &ResourceBudget,
    ) -> Result<Self, ModelError> {
        let mut charge = StateCharge::new(budget, "private native materialization plan");
        charge.grow(producers.len().saturating_mul(512))?;
        let mut owners = BTreeMap::new();
        let mut specs = BTreeSet::new();
        let mut ids = Vec::with_capacity(producers.len());
        let mut views = BTreeMap::new();
        for (index, producer) in producers.iter().enumerate() {
            let descriptor = &producer.descriptor;
            let id = descriptor.identity()?;
            if descriptor.spec.model != model.digest()
                || !specs.insert(descriptor.spec.identity()?)
                || owners.insert(id, index).is_some()
                || producer.outputs.keys().ne(descriptor.outputs.keys())
            {
                return Err(ModelError::Conflict(
                    "private materialization producer inventory",
                ));
            }
            ids.push(id);
            for (name, output) in &descriptor.outputs {
                let source = producer.outputs.get(name).ok_or(ModelError::Conflict(
                    "private materialization missing output",
                ))?;
                let relation = model
                    .relation(name)
                    .ok_or(ModelError::Schema("private materialization relation"))?;
                if source.model() != model.digest()
                    || source.relation().type_id() != relation.type_id()
                    || source.rows() != output.rows
                    || source.content() != output.content
                {
                    return Err(ModelError::Conflict(
                        "private materialization output source",
                    ));
                }
                Self::insert_view(
                    &mut views,
                    CompletedView::new(name.clone(), BTreeSet::from([id]), output.rows)?,
                    &mut charge,
                )?;
            }
        }
        for view in dependency_views {
            Self::insert_view(&mut views, view, &mut charge)?;
        }
        for view in qualification_views {
            Self::insert_view(&mut views, view.clone(), &mut charge)?;
        }
        for binding in bindings {
            binding.validate()?;
            if binding.source.model() != model.digest() {
                return Err(ModelError::Conflict(
                    "private materialization binding model",
                ));
            }
            Self::insert_view(&mut views, binding.view.clone(), &mut charge)?;
        }
        for view in views.values() {
            model
                .relation(&view.relation)
                .ok_or(ModelError::Schema("private materialization view relation"))?;
            for owner in &view.contributions {
                if !owners
                    .get(owner)
                    .is_some_and(|index| producers[*index].outputs.contains_key(&view.relation))
                {
                    return Err(ModelError::Conflict(
                        "private materialization view contributor",
                    ));
                }
            }
        }
        // Current/frozen bindings are semantic roots, not hints based on stage
        // names. Follow complete exact input views and retain every output of
        // each required producer, including declared empty outputs.
        let mut required = BTreeSet::new();
        let mut needed_views = BTreeSet::new();
        let mut pending = Vec::new();
        for binding in bindings {
            needed_views.insert(binding.view.identity);
            pending.extend(binding.view.contributions.iter().copied());
        }
        for view in qualification_views {
            needed_views.insert(view.identity);
            pending.extend(view.contributions.iter().copied());
        }
        let mut dependencies = vec![BTreeSet::new(); producers.len()];
        while let Some(id) = pending.pop() {
            let index = *owners
                .get(&id)
                .ok_or(ModelError::Conflict("private materialization root owner"))?;
            if !required.insert(index) {
                continue;
            }
            for name in producers[index].outputs.keys() {
                needed_views.insert(
                    CompletedView::new(
                        name.clone(),
                        BTreeSet::from([id]),
                        producers[index].descriptor.outputs[name].rows,
                    )?
                    .identity,
                );
            }
            for source in &producers[index].descriptor.spec.inputs {
                let view = views.get(&source.view()).ok_or(ModelError::Conflict(
                    "private materialization missing dependency view",
                ))?;
                if source.model() != model.digest()
                    || source.relation() != view.relation
                    || source.rows() < 0
                    || source.rows() as u64 != view.rows
                {
                    return Err(ModelError::Conflict(
                        "private materialization exact dependency",
                    ));
                }
                needed_views.insert(view.identity);
                for owner in &view.contributions {
                    let dependency = *owners.get(owner).ok_or(ModelError::Conflict(
                        "private materialization dependency owner",
                    ))?;
                    if dependencies[index].insert(dependency) {
                        charge.grow(128)?;
                        pending.push(*owner);
                    }
                }
            }
        }
        views.retain(|id, _| needed_views.contains(id));
        let graph = ReadyGraph::new(&dependencies, &required)?;
        Ok(Self {
            required,
            graph,
            views,
            ids,
            _charge: charge,
        })
    }
    fn insert_view(
        views: &mut BTreeMap<ContentHash, CompletedView>,
        view: CompletedView,
        charge: &mut StateCharge,
    ) -> Result<(), ModelError> {
        view.validate()?;
        if let Some(previous) = views.get(&view.identity) {
            if previous != &view {
                return Err(ModelError::Conflict(
                    "private materialization view identity",
                ));
            }
        } else {
            charge.grow(lctx_native::installed_view::view_heap(&view).saturating_add(256))?;
            views.insert(view.identity, view);
        }
        Ok(())
    }
}
async fn ensure_view(
    store: &Arc<NativeCompilerStore>,
    expected: &CompletedView,
    singletons: &BTreeMap<(ContentHash, String), CompletedView>,
    installed: &mut BTreeMap<ContentHash, CompletedView>,
    model: &ValidatedModel,
    budget: &ResourceBudget,
    retained: &mut StateCharge,
) -> Result<(), ModelError> {
    if let Some(actual) = installed.get(&expected.identity) {
        if actual != expected {
            return Err(ModelError::Conflict(
                "native materialization installed view",
            ));
        }
        return Ok(());
    }
    let _scratch = budget.reserve(
        "native materialization composition descriptors",
        expected.contributions.len().saturating_mul(
            size_of::<CompletedView>()
                .saturating_add(expected.relation.len())
                .saturating_add(256),
        ),
    )?;
    retained.grow(
        lctx_native::installed_view::view_heap(expected)
            .saturating_add(size_of::<CompletedView>())
            .saturating_add(128),
    )?;
    let ingredients = expected
        .contributions
        .iter()
        .map(|id| {
            singletons
                .get(&(*id, expected.relation.clone()))
                .cloned()
                .ok_or(ModelError::Conflict(
                    "native materialization incomplete dependency",
                ))
        })
        .collect::<Result<Vec<_>, _>>()?;
    let relation = model
        .relation(&expected.relation)
        .ok_or(ModelError::Schema(
            "native materialization compose relation",
        ))?;
    let actual = store
        .compose_completed_views(relation, &ingredients, budget)
        .await?;
    if actual != *expected {
        return Err(ModelError::Conflict(
            "native materialization independently composed view",
        ));
    }
    installed.insert(actual.identity, actual);
    Ok(())
}

/// Only genuine dependency edges delay native jobs. A completed producer releases
/// its dependents immediately; unrelated slow jobs do not impose a layer barrier.
#[derive(Clone)]
struct ReadyGraph {
    ready: BTreeSet<usize>,
    remaining: Vec<usize>,
    dependents: Vec<Vec<usize>>,
}
impl ReadyGraph {
    fn new(
        dependencies: &[BTreeSet<usize>],
        required: &BTreeSet<usize>,
    ) -> Result<Self, ModelError> {
        let mut graph = Self {
            ready: BTreeSet::new(),
            remaining: vec![0; dependencies.len()],
            dependents: vec![Vec::new(); dependencies.len()],
        };
        for index in required {
            graph.remaining[*index] = dependencies[*index].len();
            if graph.remaining[*index] == 0 {
                graph.ready.insert(*index);
            }
            for dependency in &dependencies[*index] {
                if !required.contains(dependency) {
                    return Err(ModelError::Conflict(
                        "private materialization incomplete closure",
                    ));
                }
                graph.dependents[*dependency].push(*index);
            }
        }
        let mut checked = graph.clone();
        let mut count = 0;
        while let Some(index) = checked.ready.pop_first() {
            count += 1;
            checked.complete(index);
        }
        if count != required.len() {
            return Err(ModelError::Conflict(
                "private materialization dependency cycle",
            ));
        }
        Ok(graph)
    }
    fn complete(&mut self, index: usize) {
        for dependent in &self.dependents[index] {
            self.remaining[*dependent] -= 1;
            if self.remaining[*dependent] == 0 {
                self.ready.insert(*dependent);
            }
        }
    }
}
pub(super) struct LoweredContributions {
    pub contributions: Vec<CompletedContribution>,
    _charge: StateCharge,
}
struct EarnedProducer {
    views: BTreeMap<String, CompletedView>,
    _charge: StateCharge,
}
async fn materialize_producer(
    store: Arc<NativeCompilerStore>,
    producer: &PrivateProducer,
    expected_id: ContentHash,
    model: &ValidatedModel,
    budget: &ResourceBudget,
    cancellation: &lctx_compiler_store::admission::Cancellation,
) -> Result<EarnedProducer, ModelError> {
    let _producer_scratch = budget.reserve(
        "native materialization typed producer descriptors",
        lctx_native::installed_view::contribution_heap(&producer.descriptor)
            .saturating_mul(3)
            .saturating_add(
                producer
                    .outputs
                    .len()
                    .saturating_mul(size_of::<lctx_model::domain::Relation>() + 256),
            )
            .saturating_add(1024),
    )?;
    let mut charge = StateCharge::new(budget, "earned native producer views");
    // Reserve result metadata before accepting native effects. The returned views
    // keep this grant until the coordinator has transferred their ownership.
    for name in producer.outputs.keys() {
        let view = CompletedView::new(
            name.clone(),
            BTreeSet::from([expected_id]),
            producer.descriptor.outputs[name].rows,
        )?;
        charge.grow(
            lctx_native::installed_view::view_heap(&view)
                .saturating_mul(2)
                .saturating_add(name.len())
                .saturating_add(512),
        )?;
    }
    cancellation.check()?;
    let physical = store
        .begin_contribution(producer.descriptor.spec.clone())
        .await?;
    let relations = producer
        .outputs
        .keys()
        .map(|name| {
            model
                .relation(name)
                .cloned()
                .ok_or(ModelError::Schema("native materialization output relation"))
        })
        .collect::<Result<Vec<_>, _>>()?;
    for relation in &relations {
        let source = producer
            .outputs
            .get(relation.name())
            .ok_or(ModelError::Conflict("native materialization output source"))?;
        for batch in source.read_batches()? {
            cancellation.check()?;
            let checked = relation.checked_batch(model, batch?, budget)?;
            // This remains the real original-ingress/completion route; pages in
            // one contributor keep their acknowledged order.
            store
                .write_checked_batch(&physical, &checked, budget)
                .await?;
        }
    }
    let outcome = ProviderOutcome::ALL
        .into_iter()
        .find(|outcome| outcome.code() == producer.descriptor.outcome)
        .ok_or(ModelError::Conflict("native materialization outcome"))?;
    let earned = store
        .complete_contribution(physical, outcome, &relations, &BTreeMap::new(), budget)
        .await?;
    let actual = store.completed_contribution(physical).await?;
    if actual != producer.descriptor {
        return Err(ModelError::Conflict(
            "native materialization independent completed descriptor",
        ));
    }
    for (name, view) in &earned {
        let expected = CompletedView::new(
            name.clone(),
            BTreeSet::from([expected_id]),
            producer.descriptor.outputs[name].rows,
        )?;
        if *view != expected {
            return Err(ModelError::Conflict(
                "native materialization singleton view",
            ));
        }
    }
    Ok(EarnedProducer {
        views: earned,
        _charge: charge,
    })
}
type PendingProducer = BoxFuture<'static, (usize, Result<EarnedProducer, ModelError>)>;
async fn settle_failed_jobs(
    pending: &mut FuturesUnordered<PendingProducer>,
    mut failures: BTreeMap<usize, ModelError>,
) -> ModelError {
    // Never drop accepted peers at the first failure. Each result observer joins
    // its actual registered task, while NativeCalls independently retains effects.
    while let Some((index, result)) = pending.next().await {
        if let Err(error) = result {
            failures.insert(index, error);
        }
    }
    let primary_index = failures.iter().find(|(_, error)| !matches!(error, ModelError::Invalid(message) if message == "compilation cancelled"))
        .or_else(|| failures.first_key_value()).map(|(index, _)| *index)
        .expect("failed materialization has a primary failure");
    let primary = failures
        .remove(&primary_index)
        .expect("selected materialization failure");
    let mut terminal = lctx_model::domain::completion::Completion::default();
    for (_, error) in failures {
        terminal.step("native materialization peer", Err(error));
    }
    lctx_model::domain::completion::complete::<()>(Err(primary), terminal).unwrap_err()
}

pub(super) async fn materialize(
    workspace: &Arc<Workspace>,
    store: Arc<NativeCompilerStore>,
    calls: Arc<crate::native_calls::NativeCalls>,
    producers: Vec<PrivateProducer>,
    bindings: Vec<CompletedBinding>,
    dependency_views: Vec<CompletedView>,
    qualification_views: Vec<CompletedView>,
) -> Result<LoweredContributions, ModelError> {
    let model = &workspace.model;
    let budget = workspace.budget();
    let cancellation = &workspace.cancellation;
    // Closure, exact source metadata and cycles are checked before native effects.
    let mut plan = LoweringPlan::new(
        &producers,
        &bindings,
        dependency_views,
        &qualification_views,
        model,
        budget,
    )?;
    let producers = Arc::new(producers);
    let mut retained = StateCharge::new(budget, "earned native materialization views");
    let mut singletons = BTreeMap::new();
    let mut installed = BTreeMap::new();
    let mut pending = FuturesUnordered::<PendingProducer>::new();
    let mut failures = BTreeMap::new();
    loop {
        while let Some(index) = plan.graph.ready.pop_first() {
            let launched = async {
                cancellation.check()?;
                for source in &producers[index].descriptor.spec.inputs {
                    ensure_view(
                        &store,
                        &plan.views[&source.view()],
                        &singletons,
                        &mut installed,
                        model,
                        budget,
                        &mut retained,
                    )
                    .await?;
                }
                let producer_sources = producers.clone();
                let native = store.clone();
                let expected_id = plan.ids[index];
                let model = workspace.model.clone();
                let budget = budget.clone();
                let cancellation = cancellation.clone();
                let calls = calls.clone();
                workspace.spawn_stage(async move {
                    calls
                        .call(async move {
                            materialize_producer(
                                native,
                                &producer_sources[index],
                                expected_id,
                                &model,
                                &budget,
                                &cancellation,
                            )
                            .await
                        })
                        .await
                })
            }
            .await;
            match launched {
                Ok(job) => pending.push(async move { (index, job.await) }.boxed()),
                Err(error) => {
                    failures.insert(index, error);
                    break;
                }
            }
        }
        if !failures.is_empty() {
            cancellation.cancel();
            store.fail();
            return Err(settle_failed_jobs(&mut pending, failures).await);
        }
        let Some((index, result)) = pending.next().await else {
            break;
        };
        let integrated = (|| {
            let earned = result?;
            for (name, view) in earned.views {
                retained.grow(
                    lctx_native::installed_view::view_heap(&view)
                        .saturating_mul(2)
                        .saturating_add(name.len())
                        .saturating_add(512),
                )?;
                installed.insert(view.identity, view.clone());
                singletons.insert((plan.ids[index], name), view);
            }
            Ok::<(), ModelError>(())
        })();
        match integrated {
            Ok(()) => plan.graph.complete(index),
            Err(error) => {
                failures.insert(index, error);
            }
        }
        if !failures.is_empty() {
            cancellation.cancel();
            store.fail();
            return Err(settle_failed_jobs(&mut pending, failures).await);
        }
    }
    for view in plan.views.values() {
        cancellation.check()?;
        ensure_view(
            &store,
            view,
            &singletons,
            &mut installed,
            model,
            budget,
            &mut retained,
        )
        .await?;
    }
    cancellation.check()?;
    let native = store.clone();
    let binding_budget = budget.clone();
    calls
        .call(async move { native.bind_set(bindings, &binding_budget).await })
        .await?;
    store
        .retain_admission_views(qualification_views, budget)
        .await?;
    let mut charge = StateCharge::new(budget, "materialized completed descriptor transfer");
    for index in &plan.required {
        // The result and AdmissionRuntime's input clone overlap until the runtime
        // has installed its own retained inventory grant.
        charge.grow(
            lctx_native::installed_view::contribution_heap(&producers[*index].descriptor)
                .saturating_mul(2)
                .saturating_add(256),
        )?;
    }
    let contributions = plan
        .required
        .iter()
        .map(|index| producers[*index].descriptor.clone())
        .collect();
    Ok(LoweredContributions {
        contributions,
        _charge: charge,
    })
}

#[cfg(test)]
mod controls {
    use super::*;
    use lctx_model::domain::{
        Record,
        analysis::sources::SourceSnapshot,
        completed::{ContributionSpec, OutputContent, OutputProfile},
        input::Package,
        producer_assurance::ProducerAssurance,
        source::SourceArtifact,
        stages::Profile,
    };
    fn producer(
        name: &str,
        outputs: &[&str],
        inputs: Vec<SourceSnapshot>,
        model: &ValidatedModel,
        budget: &ResourceBudget,
        directory: &Arc<tempfile::TempDir>,
    ) -> PrivateProducer {
        let outputs = outputs
            .iter()
            .map(|name| {
                let source = PrivateRelation::empty(
                    model.digest(),
                    model.relation(name).unwrap().clone(),
                    budget.clone(),
                    directory.clone(),
                )
                .unwrap();
                ((*name).to_owned(), source)
            })
            .collect::<BTreeMap<_, _>>();
        PrivateProducer {
            descriptor: CompletedContribution {
                spec: ContributionSpec {
                    captured_binding: None,
                    producer: name.into(),
                    profile: OutputProfile::Specific(Profile::Catalog),
                    model: model.digest(),
                    implementation: ContentHash::of(name.as_bytes()),
                    configuration: None,
                    inputs,
                    outputs: outputs.keys().cloned().collect(),
                },
                assurance: ProducerAssurance::CurrentProviderInterface,
                outcome: ProviderOutcome::Complete.code(),
                outputs: outputs
                    .iter()
                    .map(|(name, source)| {
                        (
                            name.clone(),
                            OutputContent {
                                rows: source.rows(),
                                content: source.content(),
                            },
                        )
                    })
                    .collect(),
            },
            outputs,
        }
    }
    fn binding(view: CompletedView, model: &ValidatedModel) -> CompletedBinding {
        CompletedBinding {
            boundary: None,
            source: SourceSnapshot::of_completed_view(
                model.relation(&view.relation).unwrap(),
                model.digest(),
                &view,
            )
            .unwrap(),
            view,
            configuration: None,
        }
    }
    #[test]
    fn requested_closure_keeps_dependencies_and_full_empty_outputs_but_not_unrooted_helpers() {
        let model = lctx_model::domain::model().unwrap();
        let budget = ResourceBudget::fixed(16 << 20).unwrap();
        let directory = Arc::new(tempfile::tempdir().unwrap());
        let dependency = producer(
            "dependency",
            &[Package::NAME],
            vec![],
            &model,
            &budget,
            &directory,
        );
        let dependency_view = CompletedView::new(
            Package::NAME.into(),
            BTreeSet::from([dependency.descriptor.identity().unwrap()]),
            0,
        )
        .unwrap();
        let root = producer(
            "root",
            &[Package::NAME, SourceArtifact::NAME],
            vec![binding(dependency_view.clone(), &model).source],
            &model,
            &budget,
            &directory,
        );
        let root_id = root.descriptor.identity().unwrap();
        let helper = producer(
            "unrooted-helper",
            &[Package::NAME],
            vec![],
            &model,
            &budget,
            &directory,
        );
        let helper_id = helper.descriptor.identity().unwrap();
        let roots = vec![binding(
            CompletedView::new(SourceArtifact::NAME.into(), BTreeSet::from([root_id]), 0).unwrap(),
            &model,
        )];
        let plan = LoweringPlan::new(
            &[dependency, root, helper],
            &roots,
            vec![dependency_view],
            &[],
            &model,
            &budget,
        )
        .unwrap();
        assert_eq!(plan.required, BTreeSet::from([0, 1]));
        assert!(
            plan.views
                .values()
                .any(|view| view.relation == Package::NAME
                    && view.contributions == BTreeSet::from([root_id]))
        );
        assert!(
            plan.views
                .values()
                .all(|view| !view.contributions.contains(&helper_id))
        );
    }
    #[test]
    fn qualification_roots_retain_exact_singletons_without_adopting_unrooted_helpers() {
        let model = lctx_model::domain::model().unwrap();
        let budget = ResourceBudget::fixed(16 << 20).unwrap();
        let directory = Arc::new(tempfile::tempdir().unwrap());
        let root = producer(
            "published-root",
            &[SourceArtifact::NAME],
            vec![],
            &model,
            &budget,
            &directory,
        );
        let qualification = producer(
            "compact-qualification",
            &[Package::NAME],
            vec![],
            &model,
            &budget,
            &directory,
        );
        let helper = producer(
            "unrooted-helper",
            &[Package::NAME],
            vec![],
            &model,
            &budget,
            &directory,
        );
        let root_view = CompletedView::new(
            SourceArtifact::NAME.into(),
            BTreeSet::from([root.descriptor.identity().unwrap()]),
            0,
        )
        .unwrap();
        let qualification_view = CompletedView::new(
            Package::NAME.into(),
            BTreeSet::from([qualification.descriptor.identity().unwrap()]),
            0,
        )
        .unwrap();
        let helper_id = helper.descriptor.identity().unwrap();
        let plan = LoweringPlan::new(
            &[root, qualification, helper],
            &[binding(root_view, &model)],
            vec![],
            std::slice::from_ref(&qualification_view),
            &model,
            &budget,
        )
        .unwrap();
        assert_eq!(plan.required, BTreeSet::from([0, 1]));
        assert_eq!(
            plan.views.get(&qualification_view.identity),
            Some(&qualification_view)
        );
        assert!(
            plan.views
                .values()
                .all(|view| !view.contributions.contains(&helper_id))
        );
    }
    #[test]
    fn all_output_current_roots_retain_every_actual_contributor() {
        let model = lctx_model::domain::model().unwrap();
        let budget = ResourceBudget::fixed(16 << 20).unwrap();
        let directory = Arc::new(tempfile::tempdir().unwrap());
        let producers = ["first", "second"]
            .into_iter()
            .map(|name| producer(name, &[Package::NAME], vec![], &model, &budget, &directory))
            .collect::<Vec<_>>();
        let contributors = producers
            .iter()
            .map(|producer| producer.descriptor.identity().unwrap())
            .collect();
        let roots = vec![binding(
            CompletedView::new(Package::NAME.into(), contributors, 0).unwrap(),
            &model,
        )];
        let plan = LoweringPlan::new(&producers, &roots, vec![], &[], &model, &budget).unwrap();
        assert_eq!(plan.required, BTreeSet::from([0, 1]));
    }
    #[test]
    fn ready_dag_releases_dependents_without_waiting_for_unrelated_slow_peers() {
        let mut graph = ReadyGraph::new(
            &[
                BTreeSet::new(),
                BTreeSet::new(),
                BTreeSet::from([0]),
                BTreeSet::from([0, 1]),
            ],
            &BTreeSet::from([0, 1, 2, 3]),
        )
        .unwrap();
        assert_eq!(graph.ready.pop_first(), Some(0));
        assert_eq!(graph.ready.pop_first(), Some(1));
        graph.complete(0);
        assert_eq!(graph.ready.pop_first(), Some(2));
        assert!(!graph.ready.contains(&3));
        graph.complete(2);
        graph.complete(1);
        assert_eq!(graph.ready.pop_first(), Some(3));
        assert!(
            ReadyGraph::new(
                &[BTreeSet::from([1]), BTreeSet::from([0])],
                &BTreeSet::from([0, 1])
            )
            .is_err()
        );
    }
    #[tokio::test]
    async fn failure_settlement_waits_for_started_peers_and_retains_their_errors() {
        let (release, held) = tokio::sync::oneshot::channel();
        let mut pending = FuturesUnordered::<PendingProducer>::new();
        pending.push(
            async move {
                held.await.unwrap();
                (1, Err(ModelError::Schema("accepted peer error")))
            }
            .boxed(),
        );
        let settling = settle_failed_jobs(
            &mut pending,
            BTreeMap::from([(0, ModelError::Conflict("first actual failure"))]),
        );
        tokio::pin!(settling);
        assert!(futures::poll!(&mut settling).is_pending());
        release.send(()).unwrap();
        let error = settling.await;
        assert!(matches!(
            error.primary(),
            Some(ModelError::Conflict("first actual failure"))
        ));
        assert!(format!("{error:?}").contains("accepted peer error"));
    }
}
