//! Provider invocation and coverage contracts, independent of execution machinery.
use std::collections::{BTreeMap, BTreeSet};
use crate::{Domain, DomainCode};
use super::{ContentHash, Id, Key, KeySink, ModelError, Record};
use super::input::InputRevision;
use super::source::CoverageScope;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, DomainCode)]
#[repr(i16)]
pub enum FactFamily {
        Provenance = 0,
        Exports = 1,
        Signatures = 2,
        Calls = 3,
        Coverage = 4,
        /// The `snapshots` table: the publication act, not a fact family (DESIGN §6.1).
        Publication = 5,
        /// The derived `nodes`/`edges` catalogs (DESIGN §3.8): not a coverage unit.
        Graph = 6,
        /// C2: syntax nodes the passes read (DESIGN §3.2).
        Syntax = 7,
        /// C3: scopes, bindings, references and their resolution (DESIGN §3.2).
        Lexical = 8,
        /// C4: type terms, their structure, type observations and record fields (DESIGN §3.2).
        Types = 9,
        /// C5: documents, passages, code blocks, links and mentions (DESIGN §3.2).
        Docs = 10,
        /// Analysis results: invocations, findings, witnesses, evidence, assertions and briefs
        /// (ADR-0019). Not a coverage unit: no run declares it.
        Findings = 11,
        /// The global embedding cache (DESIGN §3.2, §11.1): not snapshot-qualified.
        EmbeddingCache = 12,
        /// The flow IR (ADR-0022 §The flow provider): definitions, uses, reaching definitions,
        /// statement regions, value sources and their conditions, from `cpg-flow`.
        Flow = 13,
        Artifacts = 14,
        Deployment = 15,
}

pub use super::obligation::ObligationKind;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, DomainCode)]
#[repr(i16)]
pub enum Origin {
        InputContext = 0,
        SourceObservation = 1,
        AnalyzerAssertion = 2,
        DerivedAnalysis = 3,
        SyntheticModel = 4,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, DomainCode)]
#[repr(i16)]
pub enum ExtractionMode {
        NativeTraversal = 0,
        ReportDecode = 1,
        RelationalDerivation = 2,
        GraphAnalysis = 3,
        Recognizer = 4,
        StatisticalAnalysis = 5,
        TemplateSynthesis = 6,
        FixtureExecution = 7,
        ManualReview = 8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, DomainCode)]
#[repr(i16)]
pub enum Fidelity {
        Raw = 0,
        NativeStructural = 1,
        NormalizedStructural = 2,
        ReportProjection = 3,
        DisplayOnly = 4,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, DomainCode)]
#[repr(i16)]
pub enum Modality {
        Definite = 0,
        Candidate = 1,
        Potential = 2,
}

#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(name = "providers")]
pub struct Provider {
    #[model(key)] pub tool: String,
    #[model(key)] pub revision: String,
    #[model(key)] pub build_digest: ContentHash,
}
#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(name = "analysis_contexts")]
pub struct AnalysisContext {
    #[model(key)] pub python_version: String,
    #[model(key)] pub python_platform: String,
    #[model(key)] pub search_path: Vec<String>,
    #[model(key)] pub site_package_path: Vec<String>,
    #[model(key)] pub config_digest: ContentHash,
    #[model(key)] pub environment_digest: ContentHash,
    #[model(key)] pub lock_digest: Option<ContentHash>,
}
#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(name = "provider_runs", invariants = invocation_invariants)]
pub struct ProviderRun {
    #[model(key, provenance)] pub provider: Id<Provider>,
    #[model(key, provenance)] pub context: Id<AnalysisContext>,
    #[model(key)] pub input: Id<InputRevision>,
    #[model(key)] pub configuration: ContentHash,
    #[model(key)] pub requested_families: ContentHash,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, DomainCode)]
#[repr(i16)]
pub enum CoverageStatus {
    CompleteUnderStatedModel = 0,
    Partial = 1,
    NotRequested = 2,
    Unavailable = 3,
    Failed = 4,
}
#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(name = "provider_coverage", validate = validate_coverage)]
pub struct ProviderCoverage {
    #[model(key)] pub scope: Id<CoverageScope>,
    #[model(key, provenance)] pub provider: Id<Provider>,
    #[model(key, provenance)] pub context: Id<AnalysisContext>,
    #[model(key)] pub family: FactFamily,
    #[model(key, provenance)] pub run: Option<Id<ProviderRun>>,
    pub status: CoverageStatus,
    pub reason: Option<ObligationKind>,
    pub diagnostic: Option<String>,
}
fn validate_coverage(row: &ProviderCoverage) -> Result<(), ModelError> {
    if !row.family.is_coverage_family() { return Err(ModelError::Invalid("not a coverage family".into())); }
    match row.status {
        CoverageStatus::NotRequested if row.run.is_some() || row.reason.is_some() =>
            Err(ModelError::Invalid("not-requested coverage has no invocation or failure reason".into())),
        CoverageStatus::CompleteUnderStatedModel if row.run.is_none() || row.reason.is_some() =>
            Err(ModelError::Invalid("complete coverage needs an invocation and no boundary".into())),
        CoverageStatus::Partial | CoverageStatus::Unavailable | CoverageStatus::Failed if row.run.is_none() || row.reason.is_none() =>
            Err(ModelError::Invalid("attempted incomplete coverage needs an invocation and structured reason".into())),
        _ => Ok(()),
    }
}
impl FactFamily {
    pub fn is_coverage_family(self) -> bool {
        matches!(self, Self::Exports | Self::Signatures | Self::Calls | Self::Syntax | Self::Lexical | Self::Types | Self::Docs | Self::Flow | Self::Artifacts | Self::Deployment)
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(name = "run_families")]
pub struct RunFamily {
    #[model(key, provenance)] pub run: Id<ProviderRun>,
    #[model(key)] pub family: FactFamily,
}
fn family_digest(families: &BTreeSet<FactFamily>) -> Result<ContentHash, ModelError> {
    if families.is_empty() || families.iter().any(|f| !f.is_coverage_family()) {
        return Err(ModelError::Invalid("invocation needs requested coverage families".into()));
    }
    let mut sink = KeySink::new("requested-families");
    for family in families { family.encode(&mut sink); }
    Ok(sink.finish())
}
impl ProviderRun {
    pub fn new(provider: Id<Provider>, context: Id<AnalysisContext>, input: Id<InputRevision>, configuration: ContentHash,
        families: impl IntoIterator<Item = FactFamily>) -> Result<(Self, Vec<RunFamily>), ModelError> {
        let families = families.into_iter().collect::<BTreeSet<_>>();
        let run = Self { provider, context, input, configuration, requested_families: family_digest(&families)? };
        let memberships = families.into_iter().map(|family| RunFamily { run: run.id(), family }).collect();
        Ok((run, memberships))
    }
}

/// Derived from the selected providers and actual source/document/artifact scopes by assembly.
/// The expected invocation is exact, including configuration and requested family set.
#[derive(Debug, Clone)]
pub struct CoverageExpectation {
    pub input: Id<InputRevision>, pub scope: Id<CoverageScope>, pub provider: Id<Provider>,
    pub context: Id<AnalysisContext>, pub family: FactFamily, pub run: Option<Id<ProviderRun>>,
}

/// Missing, extra, contradictory or failed outcomes prevent publication. Empty complete scopes
/// are valid; omitted scopes and not-requested scopes cannot stand in for them.
pub fn validate_coverage_contract(expected: &[CoverageExpectation], observed: &[ProviderCoverage],
    runs: &[ProviderRun], requested: &[RunFamily]) -> Result<(), ModelError> {
    let mut invocation = InvocationCheck::default();
    for run in runs { invocation.add_run(run.clone())?; }
    for member in requested { invocation.add_family(member)?; }
    invocation.validate_families()?;
    let mut outcomes = BTreeMap::new();
    for outcome in observed {
        outcome.validate()?;
        invocation.check_outcome(outcome)?;
        let key = (outcome.scope, outcome.provider, outcome.context, outcome.family, outcome.run);
        if outcomes.insert(key, outcome).is_some() { return Err(ModelError::Conflict(ProviderCoverage::NAME)); }
    }
    let mut expected_keys = BTreeSet::new();
    for item in expected {
        let key = (item.scope, item.provider, item.context, item.family, item.run);
        if !expected_keys.insert(key) { return Err(ModelError::Invalid("duplicate coverage expectation".into())); }
        let outcome = outcomes.remove(&key).ok_or_else(|| ModelError::Invalid("missing required coverage outcome".into()))?;
        if let Some(id) = item.run {
            let run = invocation.runs.get(&id).ok_or_else(|| ModelError::Invalid("coverage names missing invocation".into()))?;
            if (run.input, run.provider, run.context) != (item.input, item.provider, item.context)
                || !invocation.families.get(&id).is_some_and(|members| members.contains(&item.family)) {
                return Err(ModelError::Invalid("coverage disagrees with invocation".into()));
            }
            if matches!(outcome.status, CoverageStatus::NotRequested | CoverageStatus::Failed) {
                return Err(ModelError::Invalid("required provider did not complete successfully".into()));
            }
        } else if outcome.status != CoverageStatus::NotRequested {
            return Err(ModelError::Invalid("unrequested family has an attempted outcome".into()));
        }
    }
    if !outcomes.is_empty() { return Err(ModelError::Invalid("coverage has undeclared scopes or invocations".into())); }
    Ok(())
}

/// Stored invocation membership is checked independently of compiler-provided scope expectations.
/// The latter are still required to prove that no scheduled scope was omitted.
fn invocation_invariants() -> Vec<super::Invariant> {
    vec![super::Invariant { name: "provider_invocation_membership", inputs: vec![
        super::ValidationInput::of::<ProviderRun>(&["id"]),
        super::ValidationInput::of::<RunFamily>(&["run", "family"]),
        super::ValidationInput::of::<ProviderCoverage>(&["id"]),
    ], create: || Box::new(InvocationCheck::default()) }]
}
#[derive(Default)]
struct InvocationCheck {
    runs: BTreeMap<Id<ProviderRun>, ProviderRun>,
    families: BTreeMap<Id<ProviderRun>, BTreeSet<FactFamily>>,
}
impl InvocationCheck {
    fn add_run(&mut self, row: ProviderRun) -> Result<(), ModelError> {
        if self.runs.len() >= 1_000_000 { return Err(ModelError::Invalid("invocation validation cardinality budget exceeded".into())); }
        if self.runs.insert(row.id(), row).is_some() { return Err(ModelError::Conflict(ProviderRun::NAME)); }
        Ok(())
    }
    fn add_family(&mut self, row: &RunFamily) -> Result<(), ModelError> {
        if !row.family.is_coverage_family() || !self.runs.contains_key(&row.run)
            || !self.families.entry(row.run).or_default().insert(row.family) {
            return Err(ModelError::Invalid("invalid invocation family membership".into()));
        }
        Ok(())
    }
    fn check_outcome(&self, row: &ProviderCoverage) -> Result<(), ModelError> {
        if let Some(id) = row.run {
            let run = self.runs.get(&id).ok_or_else(|| ModelError::Invalid("coverage invocation absent".into()))?;
            if (run.provider, run.context) != (row.provider, row.context)
                || !self.families.get(&id).is_some_and(|f| f.contains(&row.family)) {
                return Err(ModelError::Invalid("coverage disagrees with invocation".into()));
            }
        }
        if row.status == CoverageStatus::Failed {
            return Err(ModelError::Invalid("failed provider invocation cannot publish".into()));
        }
        Ok(())
    }
    fn validate_families(&self) -> Result<(), ModelError> {
        for (id, run) in &self.runs {
            let members = self.families.get(id).ok_or_else(|| ModelError::Invalid("invocation lacks requested families".into()))?;
            if family_digest(members)? != run.requested_families {
                return Err(ModelError::Invalid("invocation family digest differs".into()));
            }
        }
        Ok(())
    }
}
impl super::InvariantCheck for InvocationCheck {
    fn visit(&mut self, relation: &str, batch: &arrow_array::RecordBatch) -> Result<(), ModelError> {
        if relation == ProviderRun::NAME {
            for row in ProviderRun::decode(batch)? { self.add_run(row)?; }
        } else if relation == RunFamily::NAME {
            for row in RunFamily::decode(batch)? { self.add_family(&row)?; }
        } else if relation == ProviderCoverage::NAME {
            for row in ProviderCoverage::decode(batch)? { self.check_outcome(&row)?; }
        } else { return Err(ModelError::Invalid("undeclared invocation validation input".into())); }
        Ok(())
    }
    fn finish(self: Box<Self>) -> Result<(), ModelError> { self.validate_families() }
}
