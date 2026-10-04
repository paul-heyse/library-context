//! A requested name denotes captured first-party releases and their associated corpora.
//! Catalog member counts and dependency acquisition never supply admission authority.
use super::*;
use crate::domain::{normalized::Rows, resources::{ResourceBudget, Reservation}, *};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("requested library is not admitted to this generation")]
pub struct LibraryAdmissionError;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct LibraryCoveragePacket {
    pub coverage: Id<attribution::ProviderCoverage>,
    pub input: Id<input::InputRevision>,
    pub scope: Id<source::CoverageScope>,
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
    /// Missing receipts remain Unavailable; NotRequested and Partial receipts retain their status.
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
macro_rules! serving_library_inputs {($m:ident)=>{$m!{
 inputs:$crate::domain::input::InputRevision,
 packages:$crate::domain::input::Package,
 releases:$crate::domain::input::Release,
 distributions:$crate::domain::input::InputDistribution,
 corpora:$crate::domain::input::CorpusLibrary,
 coverage:$crate::domain::attribution::ProviderCoverage,
 runs:$crate::domain::attribution::ProviderRun,
 scopes:$crate::domain::source::CoverageScope,
 modules:$crate::domain::source::Module,
 artifacts:$crate::domain::source::SourceArtifact,
}};}
macro_rules! data {($($field:ident:$ty:ty,)*)=>{
    pub struct LibraryAdmissionData {$(pub $field:Rows<$ty>,)*}
    impl LibraryAdmissionData {
        pub fn new(b:&ResourceBudget)->Self {Self{$($field:Rows::new(b),)*}}
        pub fn inputs()->Vec<ValidationInput> {vec![$(ValidationInput::of::<$ty>(&["id"]),)*]}
        pub fn visit(&mut self,relation:&str,batch:&arrow_array::RecordBatch)->Result<(),ModelError> {
            $(if relation==<$ty>::NAME {return self.$field.decode(batch);})*
            Err(ModelError::Invalid("undeclared library admission input".into()))
        }
    }
};}
crate::serving_library_inputs!(data);

/// Borrowed request resolution; one exact domain or the generation's complete admitted union.
#[derive(Clone, Copy)]
pub struct ResolvedLibraryDomain<'a> { domains: &'a [LibraryDomainPacket] }
impl ResolvedLibraryDomain<'_> {
    pub fn domains(&self)->&[LibraryDomainPacket] {self.domains}
    pub fn contains_capture(&self,input:Id<input::InputRevision>,release:Id<input::Release>)->bool {
        self.domains.iter().flat_map(|d| &d.captures).any(|c| c.release.input==input && c.release.release==release)
    }
    pub fn contains_release(&self,release:Id<input::Release>)->bool {
        self.domains.iter().flat_map(|d| &d.captures).any(|c| c.release.release==release)
    }
    pub fn contains_corpus(&self,input:Id<input::InputRevision>)->bool {
        self.domains.iter().flat_map(|d| &d.captures).any(|c| c.corpora.contains(&input))
    }
    pub fn metadata(&self,b:&ResourceBudget)->Result<AdmittedDomainMetadata,ModelError> {
        let bytes=self.domains.iter().map(domain_bytes).sum::<usize>().saturating_mul(2)
            .saturating_add(size_of::<LibraryDomainPacket>().saturating_mul(self.domains.len()));
        let charge=b.reserve("requested-library-domain-metadata",bytes)?;
        Ok(AdmittedDomainMetadata {domains:self.domains.to_vec(),_charge:charge})
    }
}
/// Retains request accounting through DTO construction and response handoff.
pub struct AdmittedDomainMetadata {
    pub domains:Vec<LibraryDomainPacket>,
    pub _charge:Box<dyn Reservation>,
}
pub struct PreparedLibraryDomains {
    domains:Vec<LibraryDomainPacket>,
    _charge:charged::StateCharge,
}
fn bounded_name(s:&str)->Result<Name,ModelError> {
    Name::new(s.to_owned()).map_err(|_|ModelError::Invalid("library release metadata exceeds wire bounds".into()))
}
fn domain_bytes(d:&LibraryDomainPacket)->usize {
    d.name.as_str().len().saturating_add(d.captures.iter().map(|c| {
        size_of::<LibraryCapturePacket>()+c.release.distribution.as_str().len()+c.release.version.as_str().len()
        +c.corpora.len()*size_of::<Id<input::InputRevision>>()
        +c.coverage.len()*size_of::<LibraryCoveragePacket>()
        +c.unavailable_inputs.len()*size_of::<Id<input::InputRevision>>()+512
    }).sum::<usize>())
}
impl LibraryAdmissionData {
    fn coverage_input(&self,row:&attribution::ProviderCoverage,scope:&source::CoverageScope,capture:&input::InputDistribution)->Result<Id<input::InputRevision>,ModelError> {
        if let Some(run)=row.run {
            return self.runs.get(run).map(|r|r.input).ok_or_else(||ModelError::Invalid("library coverage invocation is absent".into()));
        }
        Ok(match scope {
            source::CoverageScope::Input{input}=>*input,
            source::CoverageScope::Release{..}=>capture.input,
            source::CoverageScope::Module{module}=> {
                let m=self.modules.get(*module).ok_or_else(||ModelError::Invalid("library coverage module is absent".into()))?;
                self.artifacts.get(m.source).ok_or_else(||ModelError::Invalid("library coverage artifact is absent".into()))?.input
            },
            source::CoverageScope::Artifact{artifact}=>self.artifacts.get(*artifact).ok_or_else(||ModelError::Invalid("library coverage artifact is absent".into()))?.input,
        })
    }
    fn relevant(&self,row:&attribution::ProviderCoverage,scope:&source::CoverageScope,capture:&input::InputDistribution)->Result<bool,ModelError> {
        let input_matches=|input|input==capture.input || self.corpora.iter().any(|c| c.library==capture.input && c.corpus==input);
        if let Some(run)=row.run {
            let run=self.runs.get(run).ok_or_else(||ModelError::Invalid("library coverage invocation is absent".into()))?;
            if !input_matches(run.input) {return Ok(false);}
        }
        Ok(match scope {
            source::CoverageScope::Input{input}=>input_matches(*input),
            source::CoverageScope::Release{release}=>*release==capture.release,
            source::CoverageScope::Module{module}=> {
                let module=self.modules.get(*module).ok_or_else(||ModelError::Invalid("library coverage module is absent".into()))?;
                let artifact=self.artifacts.get(module.source).ok_or_else(||ModelError::Invalid("library coverage artifact is absent".into()))?;
                input_matches(artifact.input)
            },
            source::CoverageScope::Artifact{artifact}=> input_matches(self.artifacts.get(*artifact).ok_or_else(||ModelError::Invalid("library coverage artifact is absent".into()))?.input),
        })
    }
}
impl PreparedLibraryDomains {
    pub fn prepare(data:&LibraryAdmissionData,b:&ResourceBudget)->Result<Self,ModelError> {
        let mut charge=charged::StateCharge::new(b,"prepared-library-domain-index");
        charge.grow(size_of::<Self>())?;
        let mut domains:Vec<LibraryDomainPacket>=Vec::new();
        for capture in data.distributions.iter().filter(|d| d.role==input::DistributionRole::FirstParty) {
            if data.inputs.get(capture.input).is_none() {return Err(ModelError::Invalid("admitted library input is absent".into()));}
            let release=data.releases.get(capture.release).ok_or_else(||ModelError::Invalid("admitted library release is absent".into()))?;
            let package=data.packages.get(release.package).ok_or_else(||ModelError::Invalid("admitted library package is absent".into()))?;
            let mut coverage_count=0usize;
            for row in data.coverage.iter() {
                let scope=data.scopes.get(row.scope).ok_or_else(||ModelError::Invalid("library coverage scope is absent".into()))?;
                coverage_count+=usize::from(data.relevant(row,scope,capture)?);
            }
            let corpus_count=data.corpora.iter().filter(|c|c.library==capture.input).count();
            charge.grow((package.name.len()*2+release.version.len()+1024)
                .saturating_add(2*size_of::<LibraryCapturePacket>())
                .saturating_add(coverage_count.saturating_mul(size_of::<LibraryCoveragePacket>()))
                .saturating_add((2*corpus_count+1).saturating_mul(size_of::<Id<input::InputRevision>>())) )?;
            let mut corpora=Vec::with_capacity(corpus_count);
            for corpus in data.corpora.iter().filter(|c|c.library==capture.input) {
                if data.inputs.get(corpus.corpus).is_none() {return Err(ModelError::Invalid("admitted corpus input is absent".into()));}
                corpora.push(corpus.corpus);
            }
            corpora.sort();corpora.dedup();
            let mut coverage=Vec::with_capacity(coverage_count);
            for row in data.coverage.iter() {
                let scope=data.scopes.get(row.scope).ok_or_else(||ModelError::Invalid("library coverage scope is absent".into()))?;
                if data.relevant(row,scope,capture)? {
                    coverage.push(LibraryCoveragePacket {coverage:row.id(),input:data.coverage_input(row,scope,capture)?,scope:row.scope,context:row.context,provider:Nullable(row.provider),run:Nullable(row.run),family:row.family,status:row.status,reason:Nullable(row.reason)});
                }
            }
            let mut unavailable_inputs=Vec::with_capacity(corpus_count+1);
            for input in std::iter::once(capture.input).chain(corpora.iter().copied()) {
                if !coverage.iter().any(|c|c.input==input) {unavailable_inputs.push(input);}
            }
            let collection=if coverage.is_empty() {
                Availability::Unavailable{reason:bounded_name("provider coverage not captured")?}
            } else if unavailable_inputs.is_empty() && coverage.iter().all(|c|c.status==attribution::CoverageStatus::CompleteUnderStatedModel) {
                Availability::Available{}
            } else if unavailable_inputs.is_empty() && coverage.iter().all(|c|c.status==attribution::CoverageStatus::NotRequested) {
                Availability::NotRequested{}
            } else if coverage.iter().all(|c|matches!(c.status,attribution::CoverageStatus::Unavailable | attribution::CoverageStatus::Failed)) {
                Availability::Unavailable{reason:bounded_name("provider collection is unavailable under its stated models")?}
            } else {
                Availability::Partial{reason:bounded_name("provider collection is incomplete under its stated models")?}
            };
            let capture=LibraryCapturePacket {release:ReleaseIdentity {input:capture.input,release:capture.release,distribution:bounded_name(&package.name)?,version:bounded_name(&release.version)?},corpora,collection,coverage,unavailable_inputs};
            if let Some(domain)=domains.iter_mut().find(|d|d.name.as_str()==package.name) {
                domain.captures.reserve_exact(1);domain.captures.push(capture);
            } else {
                if domains.len()==domains.capacity() {
                    let additional=domains.capacity().max(4);
                    charge.grow(additional.saturating_mul(size_of::<LibraryDomainPacket>()))?;
                    domains.reserve_exact(additional);
                }
                domains.push(LibraryDomainPacket{name:bounded_name(&package.name)?,captures:vec![capture]});
            }
        }
        domains.sort_by(|a,b|a.name.as_str().cmp(b.name.as_str()));
        for domain in &mut domains {domain.captures.sort_by_key(|c|(c.release.input,c.release.release));}
        Ok(Self{domains,_charge:charge})
    }
    pub fn resolve<'a>(&'a self,name:Option<&Name>)->Result<ResolvedLibraryDomain<'a>,LibraryAdmissionError> {
        let domains=match name {
            None=>self.domains.as_slice(),
            Some(name)=> {
                let i=self.domains.binary_search_by(|d|d.name.as_str().cmp(name.as_str())).map_err(|_|LibraryAdmissionError)?;
                &self.domains[i..i+1]
            },
        };
        Ok(ResolvedLibraryDomain{domains})
    }
}
