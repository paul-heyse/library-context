//! Prospective schedule demand owns preparation registry entries; readers own their borrowed runs.
use super::*;
use lctx_model::domain::stages::{InputSelection, PublicationBoundary, Schedule};
use std::collections::BTreeSet;

pub(super) struct MembershipDemand {
    schedule: Arc<Schedule>,
    completed: BTreeSet<&'static str>,
    frozen: BTreeSet<PublicationBoundary>,
    _charge: StateCharge,
}
impl MembershipDemand {
    fn pending_outputs(&self) -> impl Iterator<Item = &'static str> + '_ {
        self.schedule
            .stages()
            .iter()
            .filter(|stage| !self.completed.contains(stage.name))
            .flat_map(|stage| stage.outputs.iter().map(|output| output.name()))
    }
    fn pending_selections(&self) -> impl Iterator<Item = &InputSelection> {
        let stage_inputs = self
            .schedule
            .stages()
            .iter()
            .filter(|stage| !self.completed.contains(stage.name))
            .flat_map(|stage| {
                self.schedule
                    .selections(stage.name)
                    .expect("validated schedule owns stage")
            });
        let prefix_inputs = self
            .schedule
            .publication_groups()
            .iter()
            .filter(|group| !self.frozen.contains(&group.epoch))
            .flat_map(|group| {
                self.schedule
                    .prefix_selections(group.epoch)
                    .expect("validated schedule owns prefix")
            });
        stage_inputs.chain(prefix_inputs)
    }
}
impl Workspace {
    /// Install actual future consumers before providers can complete their first shared output.
    pub(crate) fn install_membership_demand(
        &self,
        schedule: Schedule,
    ) -> Result<Arc<Schedule>, ModelError> {
        if schedule.model() != self.model.digest() {
            return Err(ModelError::Conflict("membership demand model"));
        }
        let mut owner = self.membership_demand.lock().map_err(|_| poisoned())?;
        if owner.is_some() {
            return Err(ModelError::Conflict("membership demand already installed"));
        }
        let mut charge =
            StateCharge::new(&self.budget, "compiler membership demand schedule owner");
        charge.grow(
            size_of::<MembershipDemand>()
                + size_of::<Schedule>()
                + schedule.heap_bytes()
                + 2 * size_of::<usize>()
                + schedule
                    .stages()
                    .len()
                    .saturating_mul(size_of::<&'static str>() + 32)
                + schedule
                    .publication_groups()
                    .len()
                    .saturating_mul(size_of::<PublicationBoundary>() + 32),
        )?;
        let schedule = Arc::new(schedule);
        *owner = Some(MembershipDemand {
            schedule: schedule.clone(),
            completed: BTreeSet::new(),
            frozen: BTreeSet::new(),
            _charge: charge,
        });
        drop(owner);
        self.refresh_membership_demand()?;
        Ok(schedule)
    }
    /// Current and actually bound frozen roots remain needed by final admission. Pending consumers
    /// additionally retain the actual completed singleton ingredients their recipes will select.
    pub(crate) fn refresh_membership_demand(&self) -> Result<(), ModelError> {
        if self.is_private() {
            return Ok(());
        }
        let owner = self.membership_demand.lock().map_err(|_| poisoned())?;
        let Some(demand) = owner.as_ref() else {
            return Ok(());
        };
        let producers = self.completed_by_producer.lock().map_err(|_| poisoned())?;
        let current = self.completed.lock().map_err(|_| poisoned())?;
        let frozen = self.frozen_shared.lock().map_err(|_| poisoned())?;
        let prospective = demand
            .pending_selections()
            .map(|selection| selection.producers.len())
            .sum::<usize>();
        let _handles = self.budget.reserve(
            "compiler membership demand borrowed handles",
            current
                .len()
                .saturating_add(frozen.len())
                .saturating_add(prospective)
                .saturating_mul(size_of::<Arc<CompletedRelation>>()),
        )?;
        let mut exact = Vec::with_capacity(current.len().saturating_add(frozen.len()));
        exact.extend(current.values().cloned());
        exact.extend(frozen.values().cloned());
        let mut future = Vec::with_capacity(prospective);
        for selection in demand.pending_selections() {
            for producer in &selection.producers {
                if let Some(source) = producers.get(&(*producer, selection.relation)) {
                    future.push(source.clone());
                }
            }
        }
        drop((current, frozen, producers));
        self.native()?.retain_membership_preparations(
            exact.iter().map(|source| &source.view),
            future.iter().map(|source| &source.view),
            demand.pending_outputs(),
            &self.budget,
        )
    }
    /// Only an accepted completed result retires stage demand; launch order and cache outcomes do not.
    pub(crate) fn membership_stage_completed(&self, name: &'static str) -> Result<(), ModelError> {
        let mut owner = self.membership_demand.lock().map_err(|_| poisoned())?;
        let Some(demand) = owner.as_mut() else {
            return Ok(());
        };
        if !demand
            .schedule
            .stages()
            .iter()
            .any(|stage| stage.name == name)
        {
            return Err(ModelError::Conflict("membership demand unknown stage"));
        }
        demand.completed.insert(name);
        drop(owner);
        self.refresh_membership_demand()
    }
    pub(super) fn finish_membership_prefix(
        &self,
        schedule: ContentHash,
        boundary: PublicationBoundary,
    ) -> Result<(), ModelError> {
        let mut owner = self.membership_demand.lock().map_err(|_| poisoned())?;
        let Some(demand) = owner.as_mut() else {
            return Ok(());
        };
        if demand.schedule.digest() != schedule {
            return Err(ModelError::Conflict(
                "membership demand foreign prefix schedule",
            ));
        }
        demand.frozen.insert(boundary);
        drop(owner);
        self.refresh_membership_demand()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use lctx_model::domain::{
        input::Package,
        stages::{Effect, PublicationGroup, RelationUse, Stage},
        value::Literal,
    };
    fn stage(name: &'static str, inputs: Vec<RelationUse>, outputs: Vec<RelationUse>) -> Stage {
        Stage {
            name,
            captured_binding: None,
            inputs,
            outputs,
            contributes: vec![],
            coverage: vec![],
            profiles: vec![Profile::Catalog],
            effect: Effect::Pure,
            code: ContentHash::of(b"demand-control"),
            configuration: ContentHash::of(b"demand-control"),
        }
    }
    #[test]
    fn remaining_and_running_consumers_keep_old_singletons_and_future_composition() {
        let model =
            ValidatedModel::declared(vec![Relation::of::<Literal>(), Relation::of::<Package>()])
                .unwrap();
        let schedule = Schedule::build_with_publications(
            &model,
            vec![
                stage("first", vec![], vec![RelationUse::of::<Literal>()]),
                stage(
                    "second",
                    vec![RelationUse::completed::<Literal>().at_epoch(PublicationBoundary::Facts)],
                    vec![RelationUse::of::<Literal>()],
                ),
                stage(
                    "old_reader",
                    vec![RelationUse::completed::<Literal>().at_epoch(PublicationBoundary::Facts)],
                    vec![RelationUse::of::<Package>()],
                ),
            ],
            &[],
            Profile::Catalog,
            vec![
                PublicationGroup::new(PublicationBoundary::Facts, vec!["first"]),
                PublicationGroup::new(PublicationBoundary::BaseEvaluation, vec!["second"]),
            ],
        )
        .unwrap();
        let budget = ResourceBudget::fixed(1 << 20).unwrap();
        let mut demand = MembershipDemand {
            schedule: Arc::new(schedule),
            completed: BTreeSet::new(),
            frozen: BTreeSet::new(),
            _charge: StateCharge::new(&budget, "pure-demand-recipe"),
        };
        demand.completed.insert("first");
        demand.frozen.insert(PublicationBoundary::Facts);
        assert!(
            demand
                .pending_outputs()
                .any(|relation| relation == Literal::NAME),
            "an unfinished writer retains useful union ingredients before its output exists"
        );
        assert!(
            demand
                .pending_selections()
                .any(|selection| selection.relation == Literal::NAME
                    && selection.producers.contains("first"))
        );
        demand.frozen.insert(PublicationBoundary::BaseEvaluation);
        demand.completed.insert("second");
        assert!(
            !demand
                .pending_outputs()
                .any(|relation| relation == Literal::NAME),
            "readers of final roots do not retain departed composition branches"
        );
        assert!(
            demand
                .pending_selections()
                .any(|selection| selection.producers.contains("first")),
            "the earlier frozen reader still needs its actual singleton"
        );
        // An accepted cached/optional result retires the same actual stage recipe as fresh work.
        demand.completed.insert("old_reader");
        assert_eq!(demand.pending_selections().count(), 0);
        assert_eq!(demand.pending_outputs().count(), 0);
    }
}
