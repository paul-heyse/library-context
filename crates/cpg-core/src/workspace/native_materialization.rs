//! Lower one sealed private compiler result into genuine native contributions.
//!
//! Descriptors select actual protected private rows; they never construct native
//! installation or admission capabilities. Every native descriptor and composed
//! view is independently earned and compared before a final binding is installed.
use super::private_source::PrivateRelation;
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
    order: Vec<usize>,
    views: BTreeMap<ContentHash, CompletedView>,
    ids: Vec<ContentHash>,
    _charge: StateCharge,
}
impl LoweringPlan {
    fn new(
        producers: &[PrivateProducer],
        bindings: &[CompletedBinding],
        dependency_views: Vec<CompletedView>,
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
        let mut dependents = vec![Vec::new(); producers.len()];
        let mut remaining = vec![0_usize; producers.len()];
        for (index, producer) in producers.iter().enumerate() {
            let mut dependencies = BTreeSet::new();
            for source in &producer.descriptor.spec.inputs {
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
                for owner in &view.contributions {
                    let dependency = *owners.get(owner).ok_or(ModelError::Conflict(
                        "private materialization dependency owner",
                    ))?;
                    if !dependencies.contains(&dependency) {
                        charge.grow(128)?;
                        dependencies.insert(dependency);
                    }
                }
            }
            remaining[index] = dependencies.len();
            for dependency in dependencies {
                dependents[dependency].push(index);
            }
        }
        let mut ready = remaining
            .iter()
            .enumerate()
            .filter_map(|(index, count)| (*count == 0).then_some(index))
            .collect::<BTreeSet<_>>();
        let mut order = Vec::with_capacity(producers.len());
        while let Some(index) = ready.pop_first() {
            order.push(index);
            for dependent in &dependents[index] {
                remaining[*dependent] -= 1;
                if remaining[*dependent] == 0 {
                    ready.insert(*dependent);
                }
            }
        }
        if order.len() != producers.len() {
            return Err(ModelError::Conflict(
                "private materialization dependency cycle",
            ));
        }
        Ok(Self {
            order,
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

pub(super) async fn materialize(
    store: Arc<NativeCompilerStore>,
    producers: Vec<PrivateProducer>,
    bindings: Vec<CompletedBinding>,
    dependency_views: Vec<CompletedView>,
    model: &ValidatedModel,
    budget: &ResourceBudget,
    cancellation: &lctx_compiler_store::admission::Cancellation,
) -> Result<(), ModelError> {
    // Closure, exact source metadata and cycles are checked before native effects.
    let plan = LoweringPlan::new(&producers, &bindings, dependency_views, model, budget)?;
    let mut retained = StateCharge::new(budget, "earned native materialization views");
    let mut singletons = BTreeMap::new();
    let mut installed = BTreeMap::new();
    for index in &plan.order {
        cancellation.check()?;
        let producer = &producers[*index];
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
        for source in &producer.descriptor.spec.inputs {
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
                // The model-owned constructor checks the actual immutable typed rows.
                // Raw Arrow/schema/digest metadata cannot forge this witness.
                let checked = relation.checked_batch(model, batch?, budget)?;
                // SourceArtifact headers and ArtifactChunk originals use the native
                // writer's existing real original-ingress/completion path.
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
        for (name, view) in earned {
            let expected = CompletedView::new(
                name.clone(),
                BTreeSet::from([plan.ids[*index]]),
                producer.descriptor.outputs[&name].rows,
            )?;
            if view != expected {
                return Err(ModelError::Conflict(
                    "native materialization singleton view",
                ));
            }
            retained.grow(
                lctx_native::installed_view::view_heap(&view)
                    .saturating_mul(2)
                    .saturating_add(name.len())
                    .saturating_add(512),
            )?;
            installed.insert(view.identity, view.clone());
            singletons.insert((plan.ids[*index], name), view);
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
    for binding in bindings {
        cancellation.check()?;
        ensure_view(
            &store,
            &binding.view,
            &singletons,
            &mut installed,
            model,
            budget,
            &mut retained,
        )
        .await?;
        store.bind(binding).await?;
    }
    Ok(())
}
