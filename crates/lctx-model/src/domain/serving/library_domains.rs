//! A requested name denotes captured first-party releases and their associated corpora.
//! Catalog member counts and dependency acquisition never supply admission authority.
use super::*;
use crate::domain::{
    normalized::Rows,
    resources::{Reservation, ResourceBudget},
    *,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("requested library is not admitted to this snapshot")]
pub struct LibraryAdmissionError;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct LibraryCoveragePacket {
    /// Representative scoped receipt or actual compact authority for the exact domain.
    pub coverage: uniform_outcome::CoverageAuthority,
    pub input: Id<input::InputRevision>,
    /// Representative scope; null only for an independently validated empty compact domain.
    pub scope: Nullable<Id<source::CoverageScope>>,
    pub observations: u64,
    pub context: Id<attribution::AnalysisContext>,
    pub provider: Nullable<Id<attribution::Provider>>,
    pub run: Nullable<Id<attribution::ProviderRun>>,
    pub family: attribution::FactFamily,
    pub status: attribution::CoverageStatus,
    pub reason: Nullable<obligation::ObligationKind>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct LibraryCapturePacket {
    pub release: ReleaseIdentity,
    pub corpora: Vec<Id<input::InputRevision>>,
    /// Collection coverage is separate from enumeration of canonical catalog members.
    pub collection: Availability,
    /// Exact groups by input/provider/run/context/family/status/reason, with representative references.
    /// Missing receipts remain Unavailable; NotRequested and Partial groups retain their status.
    pub coverage: Vec<LibraryCoveragePacket>,
    /// Captured inputs without any scoped provider collection receipt.
    pub unavailable_inputs: Vec<Id<input::InputRevision>>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct LibraryDomainPacket {
    pub name: Name,
    /// Finite union of captures; order never implies a latest-release selection.
    pub captures: Vec<LibraryCapturePacket>,
}

/// This inventory is shared by pure admission, prepared dependency declaration and PG hydration.
#[macro_export]
macro_rules! serving_library_inputs {
    ($m:ident) => {
        $m! {
         inputs:$crate::domain::input::InputRevision,
         packages:$crate::domain::input::Package,
         releases:$crate::domain::input::Release,
         distributions:$crate::domain::input::InputDistribution,
         corpora:$crate::domain::input::CorpusLibrary,
         coverage:$crate::domain::attribution::ProviderCoverage,
         uniform:$crate::domain::uniform_outcome::UniformNotRequested,
         contexts:$crate::domain::attribution::AnalysisContext,
         uses:$crate::domain::input::ArtifactUse,
         runs:$crate::domain::attribution::ProviderRun,
         scopes:$crate::domain::source::CoverageScope,
         modules:$crate::domain::source::Module,
         artifacts:$crate::domain::source::SourceArtifact,
        }
    };
}
macro_rules! data {($($field:ident:$ty:ty,)*)=>{
    pub struct LibraryAdmissionData {$(pub $field:Rows<$ty>,)* uniform_owners:uniform_outcome::UniformOwners}
    impl LibraryAdmissionData {
        pub fn new(b:&ResourceBudget)->Self {Self{$($field:Rows::new(b),)* uniform_owners:uniform_outcome::UniformOwners::new(b)}}
        pub fn admit_uniform_owner(&mut self,owner:&uniform_outcome::ValidatedUniformOwner)->Result<(),ModelError>{self.uniform_owners.include(owner)}
        pub fn inputs()->Vec<ValidationInput> {vec![$(ValidationInput::of::<$ty>(&["id"]),)*]}
        pub fn visit(&mut self,relation:&str,batch:&arrow_array::RecordBatch)->Result<(),ModelError> {
            $(if relation==<$ty>::NAME {return self.$field.decode(batch);})*
            Err(ModelError::Invalid("undeclared library admission input".into()))
        }
    }
};}
crate::serving_library_inputs!(data);

/// Borrowed request resolution; one exact domain or the snapshot's complete admitted union.
#[derive(Clone, Copy)]
pub struct ResolvedLibraryDomain<'a> {
    domains: &'a [LibraryDomainPacket],
}
impl ResolvedLibraryDomain<'_> {
    pub fn domains(&self) -> &[LibraryDomainPacket] {
        self.domains
    }
    pub fn contains_capture(
        &self,
        input: Id<input::InputRevision>,
        release: Id<input::Release>,
    ) -> bool {
        self.domains
            .iter()
            .flat_map(|d| &d.captures)
            .any(|c| c.release.input == input && c.release.release == release)
    }
    pub fn contains_release(&self, release: Id<input::Release>) -> bool {
        self.domains
            .iter()
            .flat_map(|d| &d.captures)
            .any(|c| c.release.release == release)
    }
    /// Subjectless evidence is admitted by a captured first-party input or its corpus.
    /// Explicit member/release subjects still require their narrower attribution.
    pub fn contains_input(&self, input: Id<input::InputRevision>) -> bool {
        self.domains
            .iter()
            .flat_map(|d| &d.captures)
            .any(|c| c.release.input == input || c.corpora.contains(&input))
    }
    pub fn contains_corpus(&self, input: Id<input::InputRevision>) -> bool {
        self.domains
            .iter()
            .flat_map(|d| &d.captures)
            .any(|c| c.corpora.contains(&input))
    }
    pub fn metadata(&self, b: &ResourceBudget) -> Result<AdmittedDomainMetadata, ModelError> {
        let bytes = self
            .domains
            .iter()
            .map(domain_bytes)
            .sum::<usize>()
            .saturating_mul(2)
            .saturating_add(size_of::<LibraryDomainPacket>().saturating_mul(self.domains.len()));
        let charge = b.reserve("requested-library-domain-metadata", bytes)?;
        Ok(AdmittedDomainMetadata {
            domains: self.domains.to_vec(),
            _charge: charge,
        })
    }
}
/// Retains request accounting through DTO construction and response handoff.
pub struct AdmittedDomainMetadata {
    pub domains: Vec<LibraryDomainPacket>,
    pub _charge: Box<dyn Reservation>,
}
pub struct PreparedLibraryDomains {
    domains: Vec<LibraryDomainPacket>,
    _charge: charged::StateCharge,
}
fn bounded_name(s: &str) -> Result<Name, ModelError> {
    Name::new(s.to_owned())
        .map_err(|_| ModelError::Invalid("library release metadata exceeds wire bounds".into()))
}
fn domain_bytes(d: &LibraryDomainPacket) -> usize {
    d.name.as_str().len().saturating_add(
        d.captures
            .iter()
            .map(|c| {
                size_of::<LibraryCapturePacket>()
                    + c.release.distribution.as_str().len()
                    + c.release.version.as_str().len()
                    + c.corpora.len() * size_of::<Id<input::InputRevision>>()
                    + c.coverage.len() * size_of::<LibraryCoveragePacket>()
                    + c.unavailable_inputs.len() * size_of::<Id<input::InputRevision>>()
                    + 512
            })
            .sum::<usize>(),
    )
}
impl LibraryAdmissionData {
    fn coverage_input(
        &self,
        row: &attribution::ProviderCoverage,
        scope: &source::CoverageScope,
        capture: &input::InputDistribution,
    ) -> Result<Id<input::InputRevision>, ModelError> {
        if let Some(run) = row.run {
            return self.runs.get(run).map(|r| r.input).ok_or_else(|| {
                ModelError::Invalid("library coverage invocation is absent".into())
            });
        }
        Ok(match scope {
            source::CoverageScope::Input { input } => *input,
            source::CoverageScope::Release { .. } => capture.input,
            source::CoverageScope::Module { module } => {
                let m = self.modules.get(*module).ok_or_else(|| {
                    ModelError::Invalid("library coverage module is absent".into())
                })?;
                self.artifacts
                    .get(m.source)
                    .ok_or_else(|| {
                        ModelError::Invalid("library coverage artifact is absent".into())
                    })?
                    .input
            }
            source::CoverageScope::Artifact { artifact } => {
                self.artifacts
                    .get(*artifact)
                    .ok_or_else(|| {
                        ModelError::Invalid("library coverage artifact is absent".into())
                    })?
                    .input
            }
        })
    }
    fn relevant(
        &self,
        row: &attribution::ProviderCoverage,
        scope: &source::CoverageScope,
        capture: &input::InputDistribution,
    ) -> Result<bool, ModelError> {
        let input_matches = |input| {
            input == capture.input
                || self
                    .corpora
                    .iter()
                    .any(|c| c.library == capture.input && c.corpus == input)
        };
        if let Some(run) = row.run {
            let run = self.runs.get(run).ok_or_else(|| {
                ModelError::Invalid("library coverage invocation is absent".into())
            })?;
            if !input_matches(run.input) {
                return Ok(false);
            }
        }
        Ok(match scope {
            source::CoverageScope::Input { input } => input_matches(*input),
            source::CoverageScope::Release { release } => *release == capture.release,
            source::CoverageScope::Module { module } => {
                let module = self.modules.get(*module).ok_or_else(|| {
                    ModelError::Invalid("library coverage module is absent".into())
                })?;
                let artifact = self.artifacts.get(module.source).ok_or_else(|| {
                    ModelError::Invalid("library coverage artifact is absent".into())
                })?;
                input_matches(artifact.input)
            }
            source::CoverageScope::Artifact { artifact } => input_matches(
                self.artifacts
                    .get(*artifact)
                    .ok_or_else(|| {
                        ModelError::Invalid("library coverage artifact is absent".into())
                    })?
                    .input,
            ),
        })
    }
}
impl PreparedLibraryDomains {
    pub fn prepare(data: &LibraryAdmissionData, b: &ResourceBudget) -> Result<Self, ModelError> {
        let mut charge = charged::StateCharge::new(b, "prepared-library-domain-index");
        charge.grow(size_of::<Self>())?;
        // Reconstruct each exact uniform domain once for this preparation. Scope membership
        // and request policy are checked independently; a deserialized descriptor alone refuses.
        let _uniform_inputs = b.reserve("library-uniform-domain-inputs",if data.uniform.is_empty(){0}else{
            data.artifacts.iter().fold(0usize,|bytes,row|bytes.saturating_add(size_of::<source::SourceArtifact>()+row.heap_bytes()))
                .saturating_add(data.uses.len().saturating_mul(size_of::<input::ArtifactUse>()))
        })?;
        let artifacts=if data.uniform.is_empty(){vec![]}else{data.artifacts.iter().cloned().collect::<Vec<_>>()};
        let uses=if data.uniform.is_empty(){vec![]}else{data.uses.iter().cloned().collect::<Vec<_>>()};
        charge.grow(data.uniform.len().saturating_mul(size_of::<(&uniform_outcome::UniformNotRequested,uniform_outcome::ValidatedUniformDomain)>()))?;
        let mut uniform_domains=Vec::with_capacity(data.uniform.len());
        for descriptor in data.uniform.iter(){
            let owner=data.uniform_owners.require(descriptor)?;
            // Analysis descriptors are independently admitted at publication and their
            // named frame/frontier consumers. They never represent native facts coverage.
            if descriptor.kind!=uniform_outcome::UniformKind::NativeFamily{continue;}
            let input=data.inputs.get(descriptor.input).ok_or(ModelError::Conflict("uniform library input absent"))?;
            let context=data.contexts.get(descriptor.context).ok_or(ModelError::Conflict("uniform library context absent"))?;
            let domain=uniform_outcome::ExpectedUniform::native_inputs(owner,input,context,&artifacts,&uses,owner.profile(),b)?;
            for (scope,_) in domain.members(){
                if data.scopes.get(*scope).is_none(){return Err(ModelError::Conflict("uniform library canonical scope absent"));}
            }
            uniform_domains.push((descriptor,domain));
        }
        let mut domains: Vec<LibraryDomainPacket> = Vec::new();
        for capture in data
            .distributions
            .iter()
            .filter(|d| d.role == input::DistributionRole::FirstParty)
        {
            if data.inputs.get(capture.input).is_none() {
                return Err(ModelError::Invalid(
                    "admitted library input is absent".into(),
                ));
            }
            let release = data
                .releases
                .get(capture.release)
                .ok_or_else(|| ModelError::Invalid("admitted library release is absent".into()))?;
            let package = data
                .packages
                .get(release.package)
                .ok_or_else(|| ModelError::Invalid("admitted library package is absent".into()))?;
            let corpus_count = data
                .corpora
                .iter()
                .filter(|c| c.library == capture.input)
                .count();
            charge.grow(
                (package.name.len() * 2 + release.version.len() + 1024)
                    .saturating_add(2 * size_of::<LibraryCapturePacket>())
                    .saturating_add(
                        (2 * corpus_count + 1)
                            .saturating_mul(size_of::<Id<input::InputRevision>>()),
                    ),
            )?;
            let mut corpora = Vec::with_capacity(corpus_count);
            for corpus in data.corpora.iter().filter(|c| c.library == capture.input) {
                if data.inputs.get(corpus.corpus).is_none() {
                    return Err(ModelError::Invalid(
                        "admitted corpus input is absent".into(),
                    ));
                }
                corpora.push(corpus.corpus);
            }
            corpora.sort_unstable();
            corpora.dedup();
            let mut coverage: Vec<LibraryCoveragePacket> = Vec::new();
            for row in data.coverage.iter() {
                let scope = data.scopes.get(row.scope).ok_or_else(|| {
                    ModelError::Invalid("library coverage scope is absent".into())
                })?;
                if data.relevant(row, scope, capture)? {
                    let input = data.coverage_input(row, scope, capture)?;
                    if let Some(group) = coverage.iter_mut().find(|c| {
                        c.input == input
                            && c.context == row.context
                            && c.provider.0 == row.provider
                            && c.run.0 == row.run
                            && c.family == row.family
                            && c.status == row.status
                            && c.reason.0 == row.reason
                    }) {
                        group.observations =
                            group.observations.checked_add(1).ok_or_else(|| {
                                ModelError::Invalid("library coverage count overflow".into())
                            })?;
                        if (uniform_outcome::CoverageAuthority::Scoped{coverage:row.id()}) < group.coverage {
                            group.coverage = uniform_outcome::CoverageAuthority::Scoped{coverage:row.id()};
                            group.scope = Nullable(Some(row.scope));
                        }
                    } else {
                        // Account the capacity step before allocating a new finite status group.
                        if coverage.len() == coverage.capacity() {
                            let additional = coverage.capacity().max(4);
                            charge.grow(
                                additional.saturating_mul(size_of::<LibraryCoveragePacket>()),
                            )?;
                            coverage.reserve_exact(additional);
                        }
                        coverage.push(LibraryCoveragePacket {
                            coverage: uniform_outcome::CoverageAuthority::Scoped{coverage:row.id()},
                            input,
                            scope: Nullable(Some(row.scope)),
                            observations: 1,
                            context: row.context,
                            provider: Nullable(row.provider),
                            run: Nullable(row.run),
                            family: row.family,
                            status: row.status,
                            reason: Nullable(row.reason),
                        });
                    }
                }
            }
            for (descriptor,domain) in &uniform_domains {
                if descriptor.input!=capture.input && !corpora.contains(&descriptor.input){continue;}
                let family=descriptor.family.ok_or(ModelError::Conflict("uniform library native family absent"))?;
                // Ordinary scoped rows and a total uniform outcome cannot both claim a family.
                if coverage.iter().any(|row|row.input==descriptor.input && row.context==descriptor.context && row.family==family){return Err(ModelError::Conflict("mixed uniform/scoped library family authority"));}
                let scope=domain.members().next().map(|(scope,definition)|{
                    if !matches!(domain.scoped(*scope,*definition),uniform_outcome::ScopedUniformOutcome::NotRequested(_)){return Err(ModelError::Conflict("uniform library scoped operation"));}Ok(*scope)
                }).transpose()?;
                if scope.is_none() && !domain.is_explicit_empty(){return Err(ModelError::Conflict("uniform library missing scope domain"));}
                if coverage.len()==coverage.capacity(){let additional=coverage.capacity().max(4);charge.grow(additional.saturating_mul(size_of::<LibraryCoveragePacket>()))?;coverage.reserve_exact(additional);}
                coverage.push(LibraryCoveragePacket{coverage:domain.authority(),input:descriptor.input,scope:Nullable(scope),observations:u64::try_from(descriptor.members).map_err(ModelError::codec)?,context:descriptor.context,provider:Nullable(None),run:Nullable(None),family,status:attribution::CoverageStatus::NotRequested,reason:Nullable(None)});
            }
            coverage.sort_unstable_by_key(|c| {
                (
                    c.input,
                    c.context,
                    c.provider.0,
                    c.run.0,
                    c.family as i16,
                    c.status as i16,
                    c.reason.0,
                )
            });
            let mut unavailable_inputs = Vec::with_capacity(corpus_count + 1);
            for input in std::iter::once(capture.input).chain(corpora.iter().copied()) {
                if !coverage.iter().any(|c| c.input == input) {
                    unavailable_inputs.push(input);
                }
            }
            let collection = if coverage.is_empty() {
                Availability::Unavailable {
                    reason: bounded_name("provider coverage not captured")?,
                }
            } else if unavailable_inputs.is_empty()
                && coverage
                    .iter()
                    .all(|c| c.status == attribution::CoverageStatus::CompleteUnderStatedModel)
            {
                Availability::Available {}
            } else if unavailable_inputs.is_empty()
                && coverage
                    .iter()
                    .all(|c| c.status == attribution::CoverageStatus::NotRequested)
            {
                Availability::NotRequested {}
            } else if coverage.iter().all(|c| {
                matches!(
                    c.status,
                    attribution::CoverageStatus::Unavailable | attribution::CoverageStatus::Failed
                )
            }) {
                Availability::Unavailable {
                    reason: bounded_name(
                        "provider collection is unavailable under its stated models",
                    )?,
                }
            } else {
                Availability::Partial {
                    reason: bounded_name(
                        "provider collection is incomplete under its stated models",
                    )?,
                }
            };
            let capture = LibraryCapturePacket {
                release: ReleaseIdentity {
                    input: capture.input,
                    release: capture.release,
                    distribution: bounded_name(&package.name)?,
                    version: bounded_name(&release.version)?,
                },
                corpora,
                collection,
                coverage,
                unavailable_inputs,
            };
            if let Some(domain) = domains.iter_mut().find(|d| d.name.as_str() == package.name) {
                domain.captures.reserve_exact(1);
                domain.captures.push(capture);
            } else {
                if domains.len() == domains.capacity() {
                    let additional = domains.capacity().max(4);
                    charge.grow(additional.saturating_mul(size_of::<LibraryDomainPacket>()))?;
                    domains.reserve_exact(additional);
                }
                domains.push(LibraryDomainPacket {
                    name: bounded_name(&package.name)?,
                    captures: vec![capture],
                });
            }
        }
        domains.sort_unstable_by(|a, b| a.name.as_str().cmp(b.name.as_str()));
        for domain in &mut domains {
            domain
                .captures
                .sort_unstable_by_key(|c| (c.release.input, c.release.release));
        }
        Ok(Self {
            domains,
            _charge: charge,
        })
    }
    pub fn resolve<'a>(
        &'a self,
        name: Option<&Name>,
    ) -> Result<ResolvedLibraryDomain<'a>, LibraryAdmissionError> {
        let domains = match name {
            None => self.domains.as_slice(),
            Some(name) => {
                let i = self
                    .domains
                    .binary_search_by(|d| d.name.as_str().cmp(name.as_str()))
                    .map_err(|_| LibraryAdmissionError)?;
                &self.domains[i..i + 1]
            }
        };
        Ok(ResolvedLibraryDomain { domains })
    }
}

#[cfg(test)]
mod compact_controls {
    use super::*;
    #[test]
    fn compact_library_facts_coverage_keeps_analysis_owner_separate(){
        use crate::domain::{producer_contract::{self,acquisition::CapturedAcquisition},completed::*,uniform_outcome::*};
        let budget=ResourceBudget::fixed(64<<20).unwrap();
        let (input,context)=test_support::input_context();
        let analysis=test_support::empty_owner(analysis::owner::AnalysisOwner::Analytic,&input,&context,&budget);
        let mut data=LibraryAdmissionData::new(&budget);
        data.inputs.insert(input.clone()).unwrap();data.contexts.insert(context.clone()).unwrap();
        let package=data.packages.insert(input::Package{name:"compact-mixed".into()}).unwrap();
        let release=data.releases.insert(input::Release{package,version:"1".into()}).unwrap();
        data.distributions.insert(input::InputDistribution{input:input.id(),release,role:input::DistributionRole::FirstParty}).unwrap();
        data.uniform.insert(analysis.owner.descriptor().clone()).unwrap();
        assert!(PreparedLibraryDomains::prepare(&data,&budget).is_err(),"analysis inventory still requires its admitted owner");
        data.admit_uniform_owner(&analysis.owner).unwrap();
        let prepared=PreparedLibraryDomains::prepare(&data,&budget).unwrap();
        let resolved=prepared.resolve(None).unwrap();let capture=&resolved.domains()[0].captures[0];
        assert!(capture.coverage.is_empty(),"five analytic input subjects grant no native fact family coverage");
        assert!(matches!(capture.collection,Availability::Unavailable{..}));
        let capture=CapturedAcquisition::tree(input.clone(),"finite-mixed-uniform-control".into());
        let contract=producer_contract::facts::assembly();
        let mut stage=contract.bind(ContentHash::of(b"finite-assembly-control"),ContentHash::of(b"assemble"),[]);
        let binding=stage.captured_binding.as_mut().unwrap();
        binding.context_dependency=producer_contract::CapturedContextDependency::SemanticContext;
        binding.sources.push(producer_contract::CapturedSourceBinding{input:input.id(),context:Some(context.id()),configuration:Some(context.config_digest),acquisition:std::sync::Arc::new(capture)});
        let (_,domain)=ExpectedUniform::captured_family(&stage,stages::Profile::Catalog,input.id(),context.id(),attribution::FactFamily::Flow,&budget).unwrap();
        let descriptor=domain.declaration().unwrap();
        let outputs=contract.outputs.iter().chain(&contract.contributes).map(|relation|relation.name().to_owned()).collect::<std::collections::BTreeSet<_>>();
        let completed=CompletedContribution{spec:ContributionSpec{captured_binding:stage.captured_binding,producer:stage.name.into(),profile:OutputProfile::Specific(stages::Profile::Catalog),model:crate::domain::model().unwrap().digest(),implementation:stage.code,configuration:Some(stage.configuration),inputs:vec![],outputs:outputs.clone()},
            assurance:producer_assurance::ProducerAssurance::CurrentProviderInterface,outcome:0,
            outputs:outputs.into_iter().map(|name|{let rows=u64::from(name==UniformNotRequested::NAME);(name,OutputContent{rows,content:ContentHash::of(b"pure-kernel-output")})}).collect()};
        let owner=ValidatedUniformOwner::validate_semantic_binding(&completed,&descriptor,stages::Profile::Catalog,&budget).unwrap();
        data.uniform.insert(descriptor.clone()).unwrap();
        assert!(PreparedLibraryDomains::prepare(&data,&budget).is_err(),"an analysis owner cannot stand in for a missing native owner");
        data.admit_uniform_owner(&owner).unwrap();
        let prepared=PreparedLibraryDomains::prepare(&data,&budget).unwrap();let resolved=prepared.resolve(None).unwrap();
        let capture=&resolved.domains()[0].captures[0];assert!(matches!(capture.collection,Availability::NotRequested{}));
        assert_eq!(capture.coverage.len(),1);assert_eq!(capture.coverage[0].family,attribution::FactFamily::Flow);
        assert_eq!(capture.coverage[0].coverage,CoverageAuthority::Uniform{descriptor:descriptor.id()});
        assert_eq!(capture.coverage[0].observations,0);
    }
}
