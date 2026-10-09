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
                self.corpus
                    .insert(&mut self.charge, (row.corpus, row.library))?;
            }
        } else if relation == InputDistribution::NAME {
            for row in InputDistribution::decode(batch)? {
                self.distributions
                    .insert(&mut self.charge, (row.input, row.release))?;
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
    pub(crate) fn copy_selected_into(
        &self,
        target: &mut Self,
        selected: &mut dyn FnMut(std::any::TypeId, [u8; 16]) -> Result<bool, ModelError>,
    ) -> Result<(), ModelError> {
        use std::any::TypeId;
        for (id, value) in self.sources.iter() {
            if selected(TypeId::of::<SourceArtifact>(), *id.bytes())? {
                target.sources.insert(&mut target.charge, *id, *value)?;
            }
        }
        for (id, value) in self.modules.iter() {
            if selected(TypeId::of::<Module>(), *id.bytes())? {
                target.modules.insert(&mut target.charge, *id, *value)?;
            }
        }
        for &(corpus, library) in self.corpus.iter() {
            let id = CorpusLibrary { corpus, library }.id();
            if selected(TypeId::of::<CorpusLibrary>(), *id.bytes())? {
                target
                    .corpus
                    .insert(&mut target.charge, (corpus, library))?;
            }
        }
        for &(input, release) in self.distributions.iter() {
            let ids = [
                super::input::DistributionRole::FirstParty,
                super::input::DistributionRole::Dependency,
            ]
            .map(|role| {
                InputDistribution {
                    input,
                    release,
                    role,
                }
                .id()
            });
            if selected(TypeId::of::<InputDistribution>(), *ids[0].bytes())?
                || selected(TypeId::of::<InputDistribution>(), *ids[1].bytes())?
            {
                target
                    .distributions
                    .insert(&mut target.charge, (input, release))?;
            }
        }
        for (id, row) in self.scopes.iter() {
            if selected(TypeId::of::<CoverageScope>(), *id.bytes())? {
                let _scratch = target.charge.budget().expect("scope budget").reserve(
                    "selected-scope-clone",
                    size_of::<CoverageScope>().saturating_add(row.heap_bytes()),
                )?;
                target.scopes.insert(&mut target.charge, *id, row.clone())?;
            }
        }
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
