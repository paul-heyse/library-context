//! Operation-owned direct recovery ingredients. These are typed in-memory objects,
//! never serialized authority; adapters independently protect and reconcile native roots.
use super::{CompletedContribution, CompletedView, ContentHash, ModelError};
use crate::domain::{HeapSize, charged::StateCharge, resources::ResourceBudget};
use std::{collections::BTreeMap, mem::size_of, sync::Arc};

const NODE: usize = 32;
fn view_heap(view: &CompletedView) -> usize {
    view.relation.capacity().saturating_add(
        view.contributions
            .len()
            .saturating_mul(size_of::<ContentHash>() + NODE),
    )
}
fn validation_bytes(value: &CompletedContribution) -> usize {
    value
        .spec
        .inputs
        .heap_bytes()
        .saturating_add(value.spec.outputs.heap_bytes())
        .saturating_add(
            value
                .spec
                .captured_binding
                .as_ref()
                .map_or(0, |binding| binding.heap_bytes()),
        )
        .saturating_add(
            value
                .outputs
                .keys()
                .map(|key| size_of::<String>() + NODE + key.capacity())
                .fold(0, usize::saturating_add),
        )
        .saturating_mul(4)
        .saturating_add(4096)
}

/// One checked contributor and its direct input edges. Handles retain the adapter's
/// typed backing and charges. Edges point to view handles, not recursively to ingredients.
pub struct SharedRecoveryContribution<V, C> {
    identity: ContentHash,
    data: Arc<C>,
    dependencies: BTreeMap<ContentHash, Arc<V>>,
    _charge: StateCharge,
}
impl<V: AsRef<CompletedView>, C: AsRef<CompletedContribution>> SharedRecoveryContribution<V, C> {
    pub fn new(
        data: Arc<C>,
        budget: &ResourceBudget,
        mut dependency: impl FnMut(ContentHash) -> Result<Arc<V>, ModelError>,
    ) -> Result<Self, ModelError> {
        let value = data.as_ref().as_ref();
        let _scratch = budget.reserve(
            "shared recovery contributor validation",
            validation_bytes(value),
        )?;
        let identity = value.identity()?;
        drop(_scratch);
        let mut charge = StateCharge::new(budget, "shared recovery direct contributor");
        charge.grow(size_of::<Self>() + 2 * size_of::<usize>())?;
        let mut dependencies: BTreeMap<ContentHash, Arc<V>> = BTreeMap::new();
        for input in &value.spec.inputs {
            let view = if let Some(view) = dependencies.get(&input.view()) {
                view.clone()
            } else {
                let view = dependency(input.view())?;
                let _scratch = budget.reserve(
                    "shared recovery dependency validation",
                    view_heap(view.as_ref().as_ref()),
                )?;
                view.as_ref().as_ref().validate()?;
                charge.grow(size_of::<(ContentHash, Arc<V>)>() + NODE)?;
                dependencies.insert(input.view(), view.clone());
                view
            };
            let view = view.as_ref().as_ref();
            if view.identity != input.view()
                || view.relation != input.relation()
                || input.rows() < 0
                || view.rows != input.rows() as u64
            {
                return Err(ModelError::Conflict("recovery dependency metadata"));
            }
        }
        Ok(Self {
            identity,
            data,
            dependencies,
            _charge: charge,
        })
    }
    pub fn identity(&self) -> ContentHash {
        self.identity
    }
    pub fn data(&self) -> &Arc<C> {
        &self.data
    }
    pub fn dependencies(&self) -> &BTreeMap<ContentHash, Arc<V>> {
        &self.dependencies
    }
}

/// Exact direct contents of one view, independently checked against its descriptor.
/// This is reusable logical preparation within one adapter operation, not root admission.
pub struct SharedRecoveryIngredient<V, C> {
    view: Arc<V>,
    contributions: BTreeMap<ContentHash, Arc<SharedRecoveryContribution<V, C>>>,
    _charge: StateCharge,
}
impl<V: AsRef<CompletedView>, C: AsRef<CompletedContribution>> SharedRecoveryIngredient<V, C> {
    pub fn new(
        view: Arc<V>,
        selected: impl IntoIterator<Item = Arc<SharedRecoveryContribution<V, C>>>,
        budget: &ResourceBudget,
    ) -> Result<Self, ModelError> {
        let actual = view.as_ref().as_ref();
        let _scratch = budget.reserve("shared recovery view validation", view_heap(actual))?;
        actual.validate()?;
        drop(_scratch);
        let mut charge = StateCharge::new(budget, "shared recovery direct view");
        charge.grow(size_of::<Self>() + 2 * size_of::<usize>())?;
        let mut contributions = BTreeMap::new();
        for contribution in selected {
            let identity = contribution.identity;
            if !actual.contributions.contains(&identity) || contributions.contains_key(&identity) {
                return Err(ModelError::Conflict("recovery exact contributor inventory"));
            }
            if !contribution
                .data
                .as_ref()
                .as_ref()
                .outputs
                .contains_key(&actual.relation)
            {
                return Err(ModelError::Conflict(
                    "recovery contribution output ownership",
                ));
            }
            charge
                .grow(size_of::<(ContentHash, Arc<SharedRecoveryContribution<V, C>>)>() + NODE)?;
            contributions.insert(identity, contribution);
        }
        if !actual.contributions.iter().eq(contributions.keys()) {
            return Err(ModelError::Conflict("recovery missing contributor"));
        }
        Ok(Self {
            view,
            contributions,
            _charge: charge,
        })
    }
    pub fn view(&self) -> &Arc<V> {
        &self.view
    }
}

/// Root-local traversal keeps only shared handles and exact identity edges. Each
/// root still visits its whole closure; no prior root's success substitutes for it.
pub struct SharedRecoveryTraversal<V, C> {
    pending: Vec<Arc<V>>,
    awaiting: Option<ContentHash>,
    closure: SharedRecoveryClosure<V, C>,
    pending_charge: StateCharge,
}
pub struct SharedRecoveryClosure<V, C> {
    views: BTreeMap<ContentHash, Arc<V>>,
    contributions: BTreeMap<ContentHash, Arc<C>>,
    _charge: StateCharge,
}
/// Shared descriptors and the reservation transferred with their owning maps.
pub type SharedRecoveryParts<V, C> = (
    BTreeMap<ContentHash, Arc<V>>,
    BTreeMap<ContentHash, Arc<C>>,
    StateCharge,
);
impl<V, C> SharedRecoveryClosure<V, C> {
    pub fn views(&self) -> &BTreeMap<ContentHash, Arc<V>> {
        &self.views
    }
    pub fn contributions(&self) -> &BTreeMap<ContentHash, Arc<C>> {
        &self.contributions
    }
    pub fn into_parts(self) -> SharedRecoveryParts<V, C> {
        (self.views, self.contributions, self._charge)
    }
}
impl<V: AsRef<CompletedView>, C: AsRef<CompletedContribution>> SharedRecoveryTraversal<V, C> {
    pub fn for_view(view: Arc<V>, budget: &ResourceBudget) -> Result<Self, ModelError> {
        let _scratch = budget.reserve(
            "shared recovery root validation",
            view_heap(view.as_ref().as_ref()),
        )?;
        view.as_ref().as_ref().validate()?;
        drop(_scratch);
        let mut charge = StateCharge::new(budget, "shared recovery root handles");
        charge.grow(size_of::<SharedRecoveryClosure<V, C>>())?;
        let mut pending_charge = StateCharge::new(budget, "shared recovery pending handles");
        pending_charge.grow(4 * size_of::<Arc<V>>())?;
        let mut pending = Vec::with_capacity(4);
        pending.push(view);
        Ok(Self {
            pending,
            awaiting: None,
            pending_charge,
            closure: SharedRecoveryClosure {
                views: BTreeMap::new(),
                contributions: BTreeMap::new(),
                _charge: charge,
            },
        })
    }
    pub fn next_view(&mut self) -> Result<Option<Arc<V>>, ModelError> {
        if self.awaiting.is_some() {
            return Err(ModelError::Conflict("unconsumed recovery view"));
        }
        while let Some(view) = self.pending.pop() {
            let actual = view.as_ref().as_ref();
            if let Some(old) = self.closure.views.get(&actual.identity) {
                if old.as_ref().as_ref() != actual {
                    return Err(ModelError::Conflict("recovery view descriptor collision"));
                }
                continue;
            }
            self.awaiting = Some(actual.identity);
            return Ok(Some(view));
        }
        Ok(None)
    }
    pub fn include_view(
        &mut self,
        requested: &Arc<V>,
        ingredient: &SharedRecoveryIngredient<V, C>,
    ) -> Result<(), ModelError> {
        let actual = ingredient.view.as_ref().as_ref();
        if self.awaiting != Some(actual.identity) || actual != requested.as_ref().as_ref() {
            return Err(ModelError::Conflict("recovery exact dependency view"));
        }
        for (identity, contribution) in &ingredient.contributions {
            if let Some(old) = self.closure.contributions.get(identity) {
                if old.as_ref().as_ref() != contribution.data.as_ref().as_ref() {
                    return Err(ModelError::Conflict("recovery contributor collision"));
                }
                continue;
            }
            self.closure
                ._charge
                .grow(size_of::<(ContentHash, Arc<C>)>() + NODE)?;
            for view in contribution.dependencies.values() {
                if self.pending.len() == self.pending.capacity() {
                    let capacity = self.pending.capacity().max(4);
                    self.pending_charge
                        .grow(capacity.saturating_mul(size_of::<Arc<V>>()))?;
                    self.pending.reserve_exact(capacity);
                }
                self.pending.push(view.clone());
            }
            self.closure
                .contributions
                .insert(*identity, contribution.data.clone());
        }
        self.closure
            ._charge
            .grow(size_of::<(ContentHash, Arc<V>)>() + NODE)?;
        self.closure
            .views
            .insert(actual.identity, ingredient.view.clone());
        self.awaiting = None;
        Ok(())
    }
    pub fn finish(self) -> Result<SharedRecoveryClosure<V, C>, ModelError> {
        if self.awaiting.is_some() || !self.pending.is_empty() {
            return Err(ModelError::Conflict("unfinished recovery traversal"));
        }
        Ok(self.closure)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{
        Relation,
        analysis::sources::SourceSnapshot,
        completed::{ContributionSpec, OutputContent, OutputProfile},
        input::Package,
        producer_assurance::ProducerAssurance,
        stages::Profile,
    };
    // Deliberately not Clone: traversal must borrow the actual typed owner.
    struct Handle<T>(T);
    impl<T> AsRef<T> for Handle<T> {
        fn as_ref(&self) -> &T {
            &self.0
        }
    }
    type View = Handle<CompletedView>;
    type Contribution = Handle<CompletedContribution>;
    fn contribution(name: &str, inputs: &[Arc<View>]) -> Arc<Contribution> {
        let model = ContentHash::of(b"shared-recovery-model");
        Arc::new(Handle(CompletedContribution {
            assurance: ProducerAssurance::CurrentProviderInterface,
            spec: ContributionSpec {
                captured_binding: None,
                producer: name.into(),
                profile: OutputProfile::Specific(Profile::Catalog),
                model,
                implementation: ContentHash::of(name.as_bytes()),
                configuration: None,
                inputs: inputs
                    .iter()
                    .map(|view| {
                        SourceSnapshot::of_completed_view(
                            &Relation::of::<Package>(),
                            model,
                            view.as_ref().as_ref(),
                        )
                        .unwrap()
                    })
                    .collect(),
                outputs: ["packages".to_string()].into(),
            },
            outcome: 0,
            outputs: [(
                "packages".to_string(),
                OutputContent {
                    rows: 0,
                    content: ContentHash::of(name.as_bytes()),
                },
            )]
            .into(),
        }))
    }
    fn view(contribution: &Arc<Contribution>) -> Arc<View> {
        Arc::new(Handle(
            CompletedView::new(
                "packages".into(),
                [contribution.as_ref().as_ref().identity().unwrap()].into(),
                0,
            )
            .unwrap(),
        ))
    }
    fn ingredient(
        contribution: Arc<Contribution>,
        view: Arc<View>,
        dependencies: &[Arc<View>],
        budget: &ResourceBudget,
    ) -> Arc<SharedRecoveryIngredient<View, Contribution>> {
        let direct = Arc::new(
            SharedRecoveryContribution::new(contribution, budget, |identity| {
                dependencies
                    .iter()
                    .find(|view| view.as_ref().as_ref().identity == identity)
                    .cloned()
                    .ok_or(ModelError::Conflict("test missing dependency"))
            })
            .unwrap(),
        );
        Arc::new(SharedRecoveryIngredient::new(view, [direct], budget).unwrap())
    }
    #[test]
    fn diamond_reuses_nonclone_direct_owners_and_keeps_independent_complete_roots() {
        let budget = ResourceBudget::fixed(1 << 20).unwrap();
        let seed = contribution("seed", &[]);
        let seed_view = view(&seed);
        let left = contribution("left", std::slice::from_ref(&seed_view));
        let left_view = view(&left);
        let right = contribution("right", std::slice::from_ref(&seed_view));
        let right_view = view(&right);
        let root = contribution("root", &[left_view.clone(), right_view.clone()]);
        let root_view = view(&root);
        let ingredients = [
            ingredient(seed.clone(), seed_view.clone(), &[], &budget),
            ingredient(
                left,
                left_view.clone(),
                std::slice::from_ref(&seed_view),
                &budget,
            ),
            ingredient(
                right,
                right_view.clone(),
                std::slice::from_ref(&seed_view),
                &budget,
            ),
            ingredient(
                root,
                root_view.clone(),
                &[left_view.clone(), right_view.clone()],
                &budget,
            ),
        ]
        .into_iter()
        .map(|ingredient| (ingredient.view().as_ref().as_ref().identity, ingredient))
        .collect::<BTreeMap<_, _>>();
        let retained = budget.reserved();
        let mut completed = Vec::new();
        for _ in 0..2 {
            let mut traversal =
                SharedRecoveryTraversal::for_view(root_view.clone(), &budget).unwrap();
            let mut visited = 0;
            while let Some(requested) = traversal.next_view().unwrap() {
                visited += 1;
                traversal
                    .include_view(
                        &requested,
                        &ingredients[&requested.as_ref().as_ref().identity],
                    )
                    .unwrap();
            }
            assert_eq!(
                visited, 4,
                "each root traverses every unique reachable view"
            );
            let closure = traversal.finish().unwrap();
            assert_eq!(
                closure
                    .views()
                    .keys()
                    .copied()
                    .collect::<std::collections::BTreeSet<_>>(),
                [
                    seed_view.as_ref().as_ref().identity,
                    left_view.as_ref().as_ref().identity,
                    right_view.as_ref().as_ref().identity,
                    root_view.as_ref().as_ref().identity
                ]
                .into()
            );
            assert_eq!(closure.contributions().len(), 4);
            assert!(Arc::ptr_eq(
                &closure.contributions()[&seed.as_ref().as_ref().identity().unwrap()],
                &seed
            ));
            assert!(Arc::ptr_eq(
                &closure.views()[&seed_view.as_ref().as_ref().identity],
                &seed_view
            ));
            completed.push(closure);
        }
        drop(completed);
        assert_eq!(
            budget.reserved(),
            retained,
            "root handle and pending scratch charges release independently"
        );
        drop(ingredients);
        assert_eq!(budget.reserved(), 0);
    }
    #[test]
    fn direct_ingredients_refuse_missing_inventory_and_wrong_dependency_metadata() {
        let budget = ResourceBudget::fixed(1 << 20).unwrap();
        let seed = contribution("seed", &[]);
        let seed_view = view(&seed);
        assert!(matches!(
            SharedRecoveryIngredient::<View, Contribution>::new(seed_view.clone(), [], &budget),
            Err(ModelError::Conflict("recovery missing contributor"))
        ));
        let mut invalid =
            Arc::try_unwrap(contribution("invalid", std::slice::from_ref(&seed_view)))
                .ok()
                .unwrap()
                .0;
        invalid.spec.inputs[0].rows = 1;
        assert!(matches!(
            SharedRecoveryContribution::new(Arc::new(Handle(invalid)), &budget, |_| Ok(
                seed_view.clone()
            )),
            Err(ModelError::Conflict("recovery dependency metadata"))
        ));
        assert_eq!(budget.reserved(), 0);
    }
    #[test]
    fn shared_traversal_refuses_unfinished_root_and_effect_free_admission() {
        let budget = ResourceBudget::fixed(1 << 20).unwrap();
        let seed = contribution("seed", &[]);
        let seed_view = view(&seed);
        let before = Arc::strong_count(&seed_view);
        let tiny = ResourceBudget::fixed(1).unwrap();
        assert!(matches!(
            SharedRecoveryTraversal::<View, Contribution>::for_view(seed_view.clone(), &tiny),
            Err(ModelError::Resource { .. })
        ));
        assert_eq!(Arc::strong_count(&seed_view), before);
        let traversal =
            SharedRecoveryTraversal::<View, Contribution>::for_view(seed_view.clone(), &budget)
                .unwrap();
        assert!(matches!(
            traversal.finish(),
            Err(ModelError::Conflict("unfinished recovery traversal"))
        ));
        let mut traversal =
            SharedRecoveryTraversal::<View, Contribution>::for_view(seed_view, &budget).unwrap();
        let requested = traversal.next_view().unwrap().unwrap();
        assert!(matches!(
            traversal.next_view(),
            Err(ModelError::Conflict("unconsumed recovery view"))
        ));
        assert!(matches!(
            traversal.finish(),
            Err(ModelError::Conflict("unfinished recovery traversal"))
        ));
        drop(requested);
        assert_eq!(budget.reserved(), 0);
    }
}
