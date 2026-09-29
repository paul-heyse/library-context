//! One ownership policy for qualified assertions and provider coverage.
//! Corpus membership is explicit; paths and display names never establish ownership.
use super::charged::{ChargedMap, ChargedSet, StateCharge};
use super::{*,input::{CorpusLibrary,InputDistribution,InputRevision,Release},source::{CoverageScope,Module,SourceArtifact}};

#[derive(Default)]
pub(crate) struct ScopeIndex { charge: StateCharge,
    sources: ChargedMap<Id<SourceArtifact>,Id<InputRevision>>,
    modules: ChargedMap<Id<Module>,Id<SourceArtifact>>,
    corpus: ChargedSet<(Id<InputRevision>,Id<InputRevision>)>,
    distributions: ChargedSet<(Id<InputRevision>,Id<Release>)>,
    scopes: ChargedMap<Id<CoverageScope>,CoverageScope>,
}
fn invalid(message: &str) -> ModelError { ModelError::Invalid(message.into()) }
impl ScopeIndex {
    pub fn inputs() -> Vec<ValidationInput> { vec![
        ValidationInput::of::<SourceArtifact>(&["id"]),ValidationInput::of::<Module>(&["id"]),
        ValidationInput::of::<CorpusLibrary>(&["id"]),ValidationInput::of::<InputDistribution>(&["id"]),
        ValidationInput::of::<CoverageScope>(&["id"]),
    ] }
    pub fn new(budget: &super::resources::ResourceBudget, owner: &'static str) -> Self { Self { charge: StateCharge::new(budget,owner),..Self::default() } }
    pub fn visit(&mut self, relation: &str, batch: &arrow_array::RecordBatch) -> Result<bool,ModelError> {
        if relation == SourceArtifact::NAME { for row in SourceArtifact::decode(batch)? { self.sources.insert(&mut self.charge, row.id(),row.input)?; } }
        else if relation == Module::NAME { for row in Module::decode(batch)? { self.modules.insert(&mut self.charge, row.id(),row.source)?; } }
        else if relation == CorpusLibrary::NAME { for row in CorpusLibrary::decode(batch)? { self.corpus.insert(&mut self.charge, (row.corpus,row.library))?; } }
        else if relation == InputDistribution::NAME { for row in InputDistribution::decode(batch)? { self.distributions.insert(&mut self.charge, (row.input,row.release))?; } }
        else if relation == CoverageScope::NAME { for row in CoverageScope::decode(batch)? { self.scopes.insert(&mut self.charge, row.id(),row)?; } }
        else { return Ok(false); }
        Ok(true)
    }
    pub fn scope(&self, id: Id<CoverageScope>) -> Result<&CoverageScope,ModelError> {
        self.scopes.get(&id).ok_or_else(|| invalid("coverage scope missing"))
    }
    pub fn module_source(&self, id: Id<Module>) -> Result<Id<SourceArtifact>,ModelError> {
        self.modules.get(&id).copied().ok_or_else(|| invalid("scope module missing"))
    }
    fn input(&self, source: Id<SourceArtifact>) -> Result<Id<InputRevision>,ModelError> {
        self.sources.get(&source).copied().ok_or_else(|| invalid("scope source missing"))
    }
    pub fn acquired(&self, input: Id<InputRevision>, source: Id<SourceArtifact>) -> Result<bool,ModelError> {
        let owner = self.input(source)?; Ok(input == owner || self.corpus.contains(&(input,owner)))
    }
    pub fn within(&self, source: Id<SourceArtifact>, scope: &CoverageScope) -> Result<bool,ModelError> {
        Ok(match scope {
            CoverageScope::Artifact { artifact } => *artifact == source,
            CoverageScope::Module { module } => self.module_source(*module)? == source,
            CoverageScope::Input { input } => self.acquired(*input,source)?,
            CoverageScope::Release { release } => self.distributions.contains(&(self.input(source)?,*release)),
        })
    }
    pub fn owns_scope(&self, input: Id<InputRevision>, scope: &CoverageScope) -> Result<bool,ModelError> {
        Ok(match scope {
            CoverageScope::Input { input: owner } => *owner == input,
            CoverageScope::Artifact { artifact } => self.acquired(input,*artifact)?,
            CoverageScope::Module { module } => self.acquired(input,self.module_source(*module)?)?,
            CoverageScope::Release { release } => self.distributions.contains(&(input,*release))
                || self.corpus.iter().any(|(corpus,member)| *corpus == input && self.distributions.contains(&(*member,*release))),
        })
    }
}
