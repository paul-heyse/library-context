//! Preparation lifetime follows the remaining selected consumers, without predicting cache hits.
use super::UpperStage;
use lctx_model::domain::{projection::ProjectionName, stages::Profile};
use std::collections::BTreeSet;

pub(super) struct Needs {
    pub graphs: BTreeSet<ProjectionName>,
    pub bindings: bool,
    pub local: bool,
    pub evaluations: bool,
    pub bodies: bool,
    pub source_calls: bool,
}
impl Needs {
    pub fn remaining(
        stages: impl IntoIterator<Item = UpperStage>,
        profile: Profile,
        analytics_requested: bool,
    ) -> Self {
        let mut needs = Self {
            graphs: BTreeSet::new(),
            bindings: false,
            local: false,
            evaluations: false,
            bodies: false,
            source_calls: false,
        };
        for stage in stages {
            if stage != UpperStage::Analytic || analytics_requested {
                needs.graphs.extend(stage.graphs(profile));
            }
            needs.bindings |= stage.bindings(profile);
            if profile == Profile::Behavioral {
                needs.local |= matches!(
                    stage,
                    UpperStage::Base | UpperStage::Models | UpperStage::Summary
                );
                needs.evaluations |= matches!(
                    stage,
                    UpperStage::Completion
                        | UpperStage::SourceCalls
                        | UpperStage::Enriched
                        | UpperStage::Models
                );
                needs.bodies |= stage == UpperStage::SourceCalls;
                needs.source_calls |= stage == UpperStage::Enriched;
            }
        }
        needs
    }
}
pub(super) fn release_unused<T>(owner: &mut Option<T>, needed: bool) {
    if !needed {
        *owner = None;
    }
}

#[cfg(test)]
mod controls {
    use super::*;
    use lctx_model::domain::{charged::StateCharge, resources::ResourceBudget};

    #[test]
    fn selected_consumers_release_graphs_and_bindings_before_later_work() {
        use ProjectionName::*;
        use UpperStage::*;
        let selected = [
            SourceCalls,
            Enriched,
            Models,
            Summary,
            Structural,
            Analytic,
            Synthesis,
        ];
        let needs = Needs::remaining(selected, Profile::Behavioral, false);
        assert_eq!(
            needs.graphs,
            BTreeSet::from([CallableInvocation, DefinitionContainment])
        );
        assert!(needs.bindings);
        let budget = ResourceBudget::fixed(4096).unwrap();
        let mut bindings = Some(StateCharge::new(&budget, "binding-lifetime-control"));
        bindings.as_mut().unwrap().grow(1024).unwrap();
        let mut graphs = Some(StateCharge::new(&budget, "graph-lifetime-control"));
        graphs.as_mut().unwrap().grow(2048).unwrap();
        let graph_bytes = graphs.as_ref().unwrap().accounted_bytes();
        // Summary can be a cache hit: retiring its demand still releases the binding owner.
        let after_summary = Needs::remaining(
            [Structural, Analytic, Synthesis],
            Profile::Behavioral,
            false,
        );
        release_unused(&mut bindings, after_summary.bindings);
        release_unused(&mut graphs, !after_summary.graphs.is_empty());
        assert_eq!(graphs.as_ref().unwrap().reserved(), 2048);
        assert_eq!(budget.reserved(), graph_bytes);
        let after_structural = Needs::remaining([Analytic, Synthesis], Profile::Behavioral, false);
        release_unused(&mut graphs, !after_structural.graphs.is_empty());
        assert_eq!(budget.reserved(), 0);
        assert!(
            Needs::remaining([Synthesis], Profile::Behavioral, true)
                .graphs
                .is_empty()
        );
    }

    #[test]
    fn optional_analytic_retains_only_invocation_and_catalog_has_no_binding_owner() {
        use ProjectionName::*;
        use UpperStage::*;
        let remaining = Needs::remaining([Analytic, Retrieval], Profile::Behavioral, true);
        assert_eq!(remaining.graphs, BTreeSet::from([CallableInvocation]));
        assert!(!remaining.bindings);
        assert!(
            Needs::remaining([Summary], Profile::Catalog, false)
                .graphs
                .is_empty()
        );
        assert!(
            !Needs::remaining(
                [SourceCalls, Enriched, Models, Summary],
                Profile::Catalog,
                true
            )
            .bindings
        );
        // An absent Structural stage cannot keep its containment projection alive.
        assert_eq!(
            Needs::remaining([Summary], Profile::Behavioral, false).graphs,
            BTreeSet::from([CallableInvocation])
        );
    }
    #[test]
    fn out_of_order_completion_keeps_running_and_remaining_consumers() {
        use UpperStage::*;
        let selected = [SourceCalls, Enriched, Models, Summary, Structural, Analytic];
        let mut completed = BTreeSet::new();
        let needs = |completed: &BTreeSet<&'static str>| {
            Needs::remaining(
                selected
                    .into_iter()
                    .filter(|stage| !completed.contains(stage.name())),
                Profile::Behavioral,
                true,
            )
        };
        // Structural completes while Summary runs and Analytic remains unstarted.
        completed.insert(Structural.name());
        let retained = needs(&completed);
        assert_eq!(
            retained.graphs,
            BTreeSet::from([ProjectionName::CallableInvocation])
        );
        assert!(
            retained.local
                && retained.bindings
                && retained.evaluations
                && retained.bodies
                && retained.source_calls
        );
        completed.extend([Models.name(), SourceCalls.name()]);
        let retained = needs(&completed);
        assert!(!retained.bodies);
        assert!(retained.evaluations && retained.source_calls && retained.local);
        completed.insert(Enriched.name());
        let retained = needs(&completed);
        assert!(!retained.evaluations && !retained.source_calls);
        assert!(retained.local && retained.bindings);
        completed.insert(Summary.name());
        let retained = needs(&completed);
        assert!(!retained.local && !retained.bindings);
        assert!(!retained.graphs.is_empty());
        completed.insert(Analytic.name());
        assert!(needs(&completed).graphs.is_empty());
    }
}
