//! One ownership policy for qualified assertions and provider coverage.
//! Corpus membership is explicit; paths and display names never establish ownership.
use super::charged::{ChargedMap, ChargedSet, StateCharge};
use super::{
    input::{CorpusLibrary, InputDistribution, InputRevision, Release},
    source::{CoverageScope, Module, SourceArtifact},
    *,
};

#[derive(Default)]
pub(crate) struct ScopeIndex {
    charge: StateCharge,
    sources: ChargedMap<Id<SourceArtifact>, Id<InputRevision>>,
    modules: ChargedMap<Id<Module>, Id<SourceArtifact>>,
    corpus: ChargedSet<(Id<InputRevision>, Id<InputRevision>)>,
    distributions: ChargedSet<(Id<InputRevision>, Id<Release>)>,
    corpus_keys: ChargedMap<Id<CorpusLibrary>, (Id<InputRevision>, Id<InputRevision>)>,
    distribution_keys: ChargedMap<Id<InputDistribution>, (Id<InputRevision>, Id<Release>)>,
    scopes: ChargedMap<Id<CoverageScope>, CoverageScope>,
}
fn invalid(message: &str) -> ModelError {
    ModelError::Invalid(message.into())
}
#[macro_export]
macro_rules! ownership_scope_inputs {
    ($apply:ident) => {
        $apply! {
            sources:$crate::domain::source::SourceArtifact,
            modules:$crate::domain::source::Module,
            corpus:$crate::domain::input::CorpusLibrary,
            distributions:$crate::domain::input::InputDistribution,
            scopes:$crate::domain::source::CoverageScope,
        }
    };
}
impl ScopeIndex {
    pub fn inputs() -> Vec<ValidationInput> {
        macro_rules! inputs {($($field:ident:$ty:ty,)*)=>{vec![$(ValidationInput::of::<$ty>(&["id"]),)*]};}
        crate::ownership_scope_inputs!(inputs)
    }
    pub fn new(budget: &super::resources::ResourceBudget, owner: &'static str) -> Self {
        Self {
            charge: StateCharge::new(budget, owner),
            ..Self::default()
        }
    }
    pub fn visit(
        &mut self,
        relation: &str,
        batch: &arrow_array::RecordBatch,
    ) -> Result<bool, ModelError> {
        if relation == SourceArtifact::NAME {
            for row in SourceArtifact::decode(batch)? {
                self.sources.insert(&mut self.charge, row.id(), row.input)?;
            }
        } else if relation == Module::NAME {
            for row in Module::decode(batch)? {
                self.modules
                    .insert(&mut self.charge, row.id(), row.source)?;
            }
        } else if relation == CorpusLibrary::NAME {
            for row in CorpusLibrary::decode(batch)? {
                self.insert_corpus(row.corpus, row.library)?;
            }
        } else if relation == InputDistribution::NAME {
            for row in InputDistribution::decode(batch)? {
                self.insert_distribution(row.input, row.release)?;
            }
        } else if relation == CoverageScope::NAME {
            for row in CoverageScope::decode(batch)? {
                self.scopes.insert(&mut self.charge, row.id(), row)?;
            }
        } else {
            return Ok(false);
        }
        Ok(true)
    }
    fn insert_corpus(&mut self, corpus: Id<InputRevision>, library: Id<InputRevision>) -> Result<(), ModelError> {
        let id = CorpusLibrary { corpus, library }.id();
        let pair = (corpus, library);
        if self.corpus_keys.get(&id).is_some_and(|prior| *prior != pair) { return Err(ModelError::Conflict(CorpusLibrary::NAME)); }
        self.corpus_keys.insert(&mut self.charge, id, pair)?;
        self.corpus.insert(&mut self.charge, pair)?;
        Ok(())
    }
    fn insert_distribution(&mut self, input: Id<InputRevision>, release: Id<Release>) -> Result<(), ModelError> {
        let pair = (input, release);
        // Ownership deliberately collapses the two distribution roles. Both nominal
        // IDs therefore select the same existing pair, as before indexed copying.
        for role in [super::input::DistributionRole::FirstParty, super::input::DistributionRole::Dependency] {
            let id = InputDistribution { input, release, role }.id();
            if self.distribution_keys.get(&id).is_some_and(|prior| *prior != pair) { return Err(ModelError::Conflict(InputDistribution::NAME)); }
            self.distribution_keys.insert(&mut self.charge, id, pair)?;
        }
        self.distributions.insert(&mut self.charge, pair)?;
        Ok(())
    }
    pub(crate) fn copy_selected_into<I: Iterator<Item = [u8; 16]>>(
        &self,
        target: &mut Self,
        inputs: &[ValidationInput],
        keys: &mut dyn FnMut(usize) -> Result<I, ModelError>,
    ) -> Result<(), ModelError> {
        use super::normalized::visit_selected_ids;
        visit_selected_ids::<SourceArtifact, I>(inputs, None, keys, &mut |id| {
            if let Some(value) = self.sources.get(&id) { target.sources.insert(&mut target.charge, id, *value)?; }
            Ok(())
        })?;
        visit_selected_ids::<Module, I>(inputs, None, keys, &mut |id| {
            if let Some(value) = self.modules.get(&id) { target.modules.insert(&mut target.charge, id, *value)?; }
            Ok(())
        })?;
        visit_selected_ids::<CorpusLibrary, I>(inputs, None, keys, &mut |id| {
            if let Some((corpus, library)) = self.corpus_keys.get(&id) { target.insert_corpus(*corpus, *library)?; }
            Ok(())
        })?;
        visit_selected_ids::<InputDistribution, I>(inputs, None, keys, &mut |id| {
            if let Some((input, release)) = self.distribution_keys.get(&id) { target.insert_distribution(*input, *release)?; }
            Ok(())
        })?;
        visit_selected_ids::<CoverageScope, I>(inputs, None, keys, &mut |id| {
            if let Some(row) = self.scopes.get(&id) {
                let _scratch = target.charge.budget().expect("scope budget").reserve(
                    "selected-scope-clone",
                    size_of::<CoverageScope>().saturating_add(row.heap_bytes()),
                )?;
                target.scopes.insert(&mut target.charge, id, row.clone())?;
            }
            Ok(())
        })?;
        Ok(())
    }
    pub fn scope(&self, id: Id<CoverageScope>) -> Result<&CoverageScope, ModelError> {
        self.scopes
            .get(&id)
            .ok_or_else(|| invalid("coverage scope missing"))
    }
    pub fn module_source(&self, id: Id<Module>) -> Result<Id<SourceArtifact>, ModelError> {
        self.modules
            .get(&id)
            .copied()
            .ok_or_else(|| invalid("scope module missing"))
    }
    fn input(&self, source: Id<SourceArtifact>) -> Result<Id<InputRevision>, ModelError> {
        self.sources
            .get(&source)
            .copied()
            .ok_or_else(|| invalid("scope source missing"))
    }
    pub fn acquired(
        &self,
        input: Id<InputRevision>,
        source: Id<SourceArtifact>,
    ) -> Result<bool, ModelError> {
        let owner = self.input(source)?;
        Ok(input == owner || self.corpus.contains(&(input, owner)))
    }
    pub fn within(
        &self,
        source: Id<SourceArtifact>,
        scope: &CoverageScope,
    ) -> Result<bool, ModelError> {
        Ok(match scope {
            CoverageScope::Artifact { artifact } => *artifact == source,
            CoverageScope::Module { module } => self.module_source(*module)? == source,
            CoverageScope::Input { input } => self.acquired(*input, source)?,
            CoverageScope::Release { release } => self
                .distributions
                .contains(&(self.input(source)?, *release)),
        })
    }
    pub fn owns_scope(
        &self,
        input: Id<InputRevision>,
        scope: &CoverageScope,
    ) -> Result<bool, ModelError> {
        Ok(match scope {
            CoverageScope::Input { input: owner } => *owner == input,
            CoverageScope::Artifact { artifact } => self.acquired(input, *artifact)?,
            CoverageScope::Module { module } => {
                self.acquired(input, self.module_source(*module)?)?
            }
            CoverageScope::Release { release } => {
                self.distributions.contains(&(input, *release))
                    || self.corpus.iter().any(|(corpus, member)| {
                        *corpus == input && self.distributions.contains(&(*member, *release))
                    })
            }
        })
    }
}

#[cfg(test)]
mod selected_ownership_controls {
    use super::*;
    fn nominal<T>(byte: u8) -> Id<T> {
        serde::Deserialize::deserialize(serde::de::value::SeqDeserializer::<_, serde::de::value::Error>::new([byte;16].into_iter())).unwrap()
    }
    #[test]
    fn selected_compact_ownership_keeps_collapsed_distribution_roles_and_omits_absence() {
        let budget = resources::ResourceBudget::fixed(1 << 20).unwrap();
        let mut source = ScopeIndex::new(&budget,"selected scope source");
        let pair = InputDistribution { input: nominal(1), release: nominal(2), role: input::DistributionRole::FirstParty };
        source.visit(InputDistribution::NAME,&InputDistribution::encode(std::slice::from_ref(&pair)).unwrap()).unwrap();
        let corpus = CorpusLibrary { corpus: nominal(3), library: pair.input };
        source.visit(CorpusLibrary::NAME,&CorpusLibrary::encode(std::slice::from_ref(&corpus)).unwrap()).unwrap();
        let dependency = InputDistribution { role: input::DistributionRole::Dependency, ..pair.clone() };
        let inputs = [ValidationInput::of::<InputDistribution>(&["id"]),ValidationInput::of::<CorpusLibrary>(&["id"])];
        let selected = [
            [*dependency.id().bytes(),[255;16]].into_iter().collect::<std::collections::BTreeSet<_>>(),
            [*corpus.id().bytes(),[255;16]].into_iter().collect::<std::collections::BTreeSet<_>>(),
        ];
        let mut target = ScopeIndex::new(&budget,"selected scope target");
        source.copy_selected_into(&mut target,&inputs,&mut |port| Ok(selected[port].iter().copied())).unwrap();
        assert_eq!(target.distributions.len(),1);
        assert_eq!(target.corpus.len(),1);
        assert_eq!(target.distribution_keys.len(),2,"both roles retain the existing collapsed ownership meaning");
        assert!(target.owns_scope(corpus.corpus,&CoverageScope::Release { release: pair.release }).unwrap());
        let first_party = [*pair.id().bytes()];
        let mut again = ScopeIndex::new(&budget,"selected scope overlap");
        target.copy_selected_into(&mut again,&inputs[..1],&mut |_| Ok(first_party.iter().copied())).unwrap();
        assert_eq!(again.distributions.len(),1);
        drop((source,target,again));
        assert_eq!(budget.reserved(),0);
    }
}
