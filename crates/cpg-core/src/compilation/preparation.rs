//! Preparation lifetime follows the remaining selected consumers, without predicting cache hits.
use super::UpperStage;
use lctx_model::domain::{projection::ProjectionName, stages::Profile};
use std::collections::BTreeSet;

pub(super) struct Needs {
    pub graphs: BTreeSet<ProjectionName>,
    pub bindings: bool,
}
impl Needs {
    pub fn remaining(stages: impl IntoIterator<Item = UpperStage>, profile: Profile,
        analytics_requested: bool) -> Self {
        let mut needs = Self { graphs: BTreeSet::new(), bindings: false };
        for stage in stages {
            if stage != UpperStage::Analytic || analytics_requested {
                needs.graphs.extend(stage.graphs(profile));
            }
            needs.bindings |= stage.bindings(profile);
        }
        needs
    }
}
pub(super) fn release_unused<T>(owner: &mut Option<T>, needed: bool) {
    if !needed { *owner = None; }
}

#[cfg(test)]
mod controls {
    use super::*;
    use lctx_model::domain::{charged::StateCharge, resources::ResourceBudget};

    #[test]
    fn selected_consumers_release_graphs_and_bindings_before_later_work() {
        use ProjectionName::*;
        use UpperStage::*;
        let selected = [SourceCalls, Enriched, Models, Summary, Structural, Analytic, Synthesis];
        let needs = Needs::remaining(selected, Profile::Behavioral, false);
        assert_eq!(needs.graphs, BTreeSet::from([CallableInvocation, DefinitionContainment]));
        assert!(needs.bindings);
        let budget = ResourceBudget::fixed(4096).unwrap();
        let mut bindings = Some(StateCharge::new(&budget, "binding-lifetime-control"));
        bindings.as_mut().unwrap().grow(1024).unwrap();
        let mut graphs = Some(StateCharge::new(&budget, "graph-lifetime-control"));
        graphs.as_mut().unwrap().grow(2048).unwrap();
        // Summary can be a cache hit: retiring its demand still releases the binding owner.
        let after_summary = Needs::remaining([Structural, Analytic, Synthesis], Profile::Behavioral, false);
        release_unused(&mut bindings, after_summary.bindings);
        release_unused(&mut graphs, !after_summary.graphs.is_empty());
        assert_eq!(budget.reserved(), 2048);
        let after_structural = Needs::remaining([Analytic, Synthesis], Profile::Behavioral, false);
        release_unused(&mut graphs, !after_structural.graphs.is_empty());
        assert_eq!(budget.reserved(), 0);
        assert!(Needs::remaining([Synthesis], Profile::Behavioral, true).graphs.is_empty());
    }

    #[test]
    fn optional_analytic_retains_only_invocation_and_catalog_has_no_binding_owner() {
        use ProjectionName::*;
        use UpperStage::*;
        let remaining = Needs::remaining([Analytic, Retrieval], Profile::Behavioral, true);
        assert_eq!(remaining.graphs, BTreeSet::from([CallableInvocation]));
        assert!(!remaining.bindings);
        assert!(Needs::remaining([Summary], Profile::Catalog, false).graphs.is_empty());
        assert!(!Needs::remaining([SourceCalls, Enriched, Models, Summary], Profile::Catalog, true).bindings);
        // An absent Structural stage cannot keep its containment projection alive.
        assert_eq!(Needs::remaining([Summary], Profile::Behavioral, false).graphs,
            BTreeSet::from([CallableInvocation]));
    }
}
