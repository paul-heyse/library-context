//! Recoverable completed content, independent of a store layout or runtime authority.
//! Adapters supply exact descriptors; selecting them is never semantic admission.
use super::{ContentHash, ModelError, completed::{CompletedBinding, CompletedContribution, CompletedView, binding_inventory_identity}};
use std::collections::{BTreeMap, BTreeSet};

/// One declaration generates both the exhaustive type and its complete dispatch inventory.
/// Adding content therefore enters every adapter's family match automatically.
macro_rules! recovery_families {
    ($($family:ident),+ $(,)?) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        pub enum RecoveryFamily { $($family),+ }
        pub const RECOVERY_FAMILIES: &[RecoveryFamily] = &[$(RecoveryFamily::$family),+];
    };
}
recovery_families! {
    Contributions, Memberships, Views, TypedBacking, Bindings, Aliases,
    GraphPayloads, GraphRoles, ExternalEndpoints, OriginalHeaders, OriginalChunks,
    SearchOccurrences, SearchDocuments, SearchVectors,
}
impl RecoveryFamily {
    /// Claims are retained as actual source rows and compared against independent cold lowering.
    /// They confer no import or semantic-admission authority.
    pub const fn is_integrity_claim(self) -> bool {
        match self {
            Self::GraphRoles | Self::ExternalEndpoints | Self::SearchOccurrences |
            Self::SearchDocuments | Self::SearchVectors => true,
            Self::Contributions | Self::Memberships | Self::Views | Self::TypedBacking |
            Self::Bindings | Self::Aliases | Self::GraphPayloads | Self::OriginalHeaders |
            Self::OriginalChunks => false,
        }
    }
}
/// The transport order is semantic format authority. Native names belong to the adapter.
pub const COMPLETED_FAMILIES: [RecoveryFamily; 6] = [
    RecoveryFamily::Contributions, RecoveryFamily::Memberships, RecoveryFamily::Views,
    RecoveryFamily::TypedBacking, RecoveryFamily::Bindings, RecoveryFamily::Aliases,
];

/// One immutable logical closure. Whole exact views retain membership and absence premises,
/// including empty outputs; all contribution outputs remain required recovery content.
#[derive(Debug)]
pub struct RecoveryClosure {
    bindings: Vec<CompletedBinding>,
    views: BTreeMap<ContentHash, CompletedView>,
    contributions: BTreeMap<ContentHash, CompletedContribution>,
}
/// Incremental traversal lets a native adapter await reads and a dump adapter read local rows
/// while sharing dependency, identity, relation and cardinality semantics.
pub struct RecoveryTraversal {
    closure: RecoveryClosure,
    pending: Vec<CompletedView>,
}
impl RecoveryTraversal {
    pub fn new(bindings: Vec<CompletedBinding>) -> Result<Self, ModelError> {
        binding_inventory_identity(&bindings)?;
        Ok(Self { pending: bindings.iter().map(|b| b.view.clone()).collect(), closure: RecoveryClosure {
            bindings, views: BTreeMap::new(), contributions: BTreeMap::new(),
        } })
    }
    pub fn next_view(&mut self) -> Result<Option<CompletedView>, ModelError> {
        while let Some(view) = self.pending.pop() {
            view.validate()?;
            if let Some(old) = self.closure.views.get(&view.identity) {
                if old != &view { return Err(ModelError::Conflict("recovery view descriptor collision")); }
                continue;
            }
            return Ok(Some(view));
        }
        Ok(None)
    }
    /// The adapter independently reads the registered view and contributor descriptors.
    pub fn include_view(
        &mut self,
        requested: &CompletedView,
        actual: CompletedView,
        contributions: Vec<CompletedContribution>,
        mut dependency: impl FnMut(ContentHash) -> Result<CompletedView, ModelError>,
    ) -> Result<(), ModelError> {
        actual.validate()?;
        if actual != *requested { return Err(ModelError::Conflict("recovery exact dependency view")); }
        let mut received = BTreeSet::new();
        for contribution in contributions {
            let identity = contribution.identity()?;
            if !actual.contributions.contains(&identity) || !received.insert(identity) {
                return Err(ModelError::Conflict("recovery exact contributor inventory"));
            }
            if !contribution.outputs.contains_key(&actual.relation) {
                return Err(ModelError::Conflict("recovery contribution output ownership"));
            }
            if let Some(old) = self.closure.contributions.get(&identity) {
                if old != &contribution { return Err(ModelError::Conflict("recovery contributor collision")); }
                continue;
            }
            for input in &contribution.spec.inputs {
                let view = dependency(input.view())?;
                if view.identity != input.view() || view.relation != input.relation()
                    || input.rows() < 0 || view.rows != input.rows() as u64 {
                    return Err(ModelError::Conflict("recovery dependency metadata"));
                }
                self.pending.push(view);
            }
            self.closure.contributions.insert(identity, contribution);
        }
        if received != actual.contributions { return Err(ModelError::Conflict("recovery missing contributor")); }
        self.closure.views.insert(actual.identity, actual);
        Ok(())
    }
    pub fn finish(self) -> Result<RecoveryClosure, ModelError> {
        if !self.pending.is_empty() { return Err(ModelError::Conflict("unfinished recovery traversal")); }
        Ok(self.closure)
    }
}
impl RecoveryClosure {
    pub fn bindings(&self) -> &[CompletedBinding] { &self.bindings }
    pub fn views(&self) -> &BTreeMap<ContentHash, CompletedView> { &self.views }
    pub fn contributions(&self) -> &BTreeMap<ContentHash, CompletedContribution> { &self.contributions }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{Relation, analysis::sources::SourceSnapshot, completed::{ContributionSpec, OutputContent}, input::{Package, Release}, stages::Profile};
    fn contribution(inputs: Vec<SourceSnapshot>, outputs: &[(&str, u64)]) -> CompletedContribution {
        CompletedContribution {
            spec: ContributionSpec { captured_binding: None, producer: format!("closure:{}", outputs[0].0), profile: Profile::Catalog, model: ContentHash::of(b"model"), implementation: ContentHash::of(b"implementation"), configuration: None, inputs, outputs: outputs.iter().map(|(name, _)| name.to_string()).collect() },
            outcome: 0, outputs: outputs.iter().map(|(name, rows)| (name.to_string(), OutputContent { rows: *rows, content: ContentHash::of(name.as_bytes()) })).collect(),
        }
    }
    #[test]
    fn whole_empty_dependency_and_unbound_sibling_output_remain_recoverable() {
        let seed = contribution(vec![], &[("packages", 0), ("quality_steps", 17)]);
        let seed_view = CompletedView::new("packages".into(), [seed.identity().unwrap()].into(), 0).unwrap();
        let input = SourceSnapshot::of_completed_view(&Relation::of::<Package>(), seed.spec.model, &seed_view).unwrap();
        let produced = contribution(vec![input], &[("releases", 1)]);
        let output = CompletedView::new("releases".into(), [produced.identity().unwrap()].into(), 1).unwrap();
        let binding = CompletedBinding { boundary: None, source: SourceSnapshot::of_completed_view(&Relation::of::<Release>(), produced.spec.model, &output).unwrap(), view: output.clone(), configuration: None };
        let mut traversal = RecoveryTraversal::new(vec![binding]).unwrap();
        let requested = traversal.next_view().unwrap().unwrap();
        traversal.include_view(&requested, output, vec![produced], |id| { assert_eq!(id, seed_view.identity); Ok(seed_view.clone()) }).unwrap();
        let requested = traversal.next_view().unwrap().unwrap();
        traversal.include_view(&requested, seed_view.clone(), vec![seed.clone()], |_| unreachable!()).unwrap();
        assert!(traversal.next_view().unwrap().is_none());
        let closure = traversal.finish().unwrap();
        assert_eq!(closure.views()[&seed_view.identity].rows, 0);
        assert_eq!(closure.contributions()[&seed.identity().unwrap()].outputs["quality_steps"].rows, 17);
    }
    #[test]
    fn independently_supplied_missing_contributor_and_wrong_dependency_metadata_fail() {
        let seed = contribution(vec![], &[("packages", 0)]);
        let dependency = CompletedView::new("packages".into(), [seed.identity().unwrap()].into(), 0).unwrap();
        let mut input = SourceSnapshot::of_completed_view(&Relation::of::<Package>(), seed.spec.model, &dependency).unwrap(); input.rows = 1;
        let produced = contribution(vec![input], &[("releases", 0)]);
        let output = CompletedView::new("releases".into(), [produced.identity().unwrap()].into(), 0).unwrap();
        let binding = CompletedBinding { boundary: None, source: SourceSnapshot::of_completed_view(&Relation::of::<Release>(), produced.spec.model, &output).unwrap(), view: output.clone(), configuration: None };
        for descriptors in [vec![], vec![produced.clone()]] {
            let mut traversal = RecoveryTraversal::new(vec![binding.clone()]).unwrap(); let requested = traversal.next_view().unwrap().unwrap();
            assert!(traversal.include_view(&requested, output.clone(), descriptors, |_| Ok(dependency.clone())).is_err());
        }
    }
}
