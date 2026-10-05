//! Provider invocation and coverage contracts, independent of execution machinery.
use super::charged::{ChargedMap, ChargedSet, StateCharge};
use super::input::InputRevision;
use super::source::CoverageScope;
use super::{ContentHash, Id, Key, KeySink, ModelError, Record};
use crate::{Domain, DomainCode};
use std::collections::BTreeSet;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, DomainCode)]
#[repr(i16)]
/// The families providers report coverage for. Codes 0, 4, 5, 6, 11 and 12 named publication,
/// graph, findings, cache and coverage units that are not coverage families; they are retired and
/// reserved, never reused (codebooks are append-only).
pub enum FactFamily {
    Exports = 1,
    Signatures = 2,
    Calls = 3,
    /// C2: syntax nodes the passes read (DESIGN §3.2).
    Syntax = 7,
    /// C3: scopes, bindings, references and their resolution (DESIGN §3.2).
    Lexical = 8,
    /// C4: type terms, their structure, type observations and record fields (DESIGN §3.2).
    Types = 9,
    /// C5: documents, passages, code blocks, links and mentions (DESIGN §3.2).
    Docs = 10,
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

#[derive(Debug, Clone, PartialEq, Eq, Domain, serde::Serialize, serde::Deserialize)]
#[model(name = "providers")]
pub struct Provider {
    #[model(key)]
    pub tool: String,
    #[model(key)]
    pub revision: String,
    #[model(key)]
    pub build_digest: ContentHash,
}
#[derive(Debug, Clone, PartialEq, Eq, Domain, serde::Serialize, serde::Deserialize)]
#[model(name = "analysis_contexts")]
pub struct AnalysisContext {
    #[model(key)]
    pub python_version: String,
    #[model(key)]
    pub python_platform: String,
    #[model(key)]
    pub search_path: Vec<String>,
    #[model(key)]
    pub site_package_path: Vec<String>,
    #[model(key)]
    pub config_digest: ContentHash,
    #[model(key)]
    pub environment_digest: ContentHash,
    #[model(key)]
    pub lock_digest: Option<ContentHash>,
}
#[derive(Debug, Clone, PartialEq, Eq, Domain, serde::Serialize, serde::Deserialize)]
#[model(name = "provider_runs", invariant_refs = invocation_invariants_refs)]
pub struct ProviderRun {
    #[model(key, provenance)]
    pub provider: Id<Provider>,
    #[model(key, provenance)]
    pub context: Id<AnalysisContext>,
    #[model(key)]
    pub input: Id<InputRevision>,
    #[model(key)]
    pub configuration: ContentHash,
    #[model(key)]
    pub requested_families: ContentHash,
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
#[derive(Debug, Clone, PartialEq, Eq, Domain, serde::Serialize, serde::Deserialize)]
#[model(name = "provider_coverage", validate = validate_coverage, invariant_refs = coverage_ownership_invariants_refs)]
pub struct ProviderCoverage {
    #[model(key)]
    pub scope: Id<CoverageScope>,
    #[model(key, provenance)]
    pub provider: Option<Id<Provider>>,
    #[model(key, provenance)]
    pub context: Id<AnalysisContext>,
    #[model(key)]
    pub family: FactFamily,
    #[model(key, provenance)]
    pub run: Option<Id<ProviderRun>>,
    pub status: CoverageStatus,
    pub reason: Option<ObligationKind>,
    pub diagnostic: Option<String>,
}
fn validate_coverage(row: &ProviderCoverage) -> Result<(), ModelError> {
    if (row.status == CoverageStatus::NotRequested) != row.provider.is_none() {
        return Err(ModelError::Invalid(
            "only not-requested coverage has no provider".into(),
        ));
    }
    match row.status {
        CoverageStatus::NotRequested if row.run.is_some() || row.reason.is_some() => {
            Err(ModelError::Invalid(
                "not-requested coverage has no invocation or failure reason".into(),
            ))
        }
        CoverageStatus::CompleteUnderStatedModel if row.run.is_none() || row.reason.is_some() => {
            Err(ModelError::Invalid(
                "complete coverage needs an invocation and no boundary".into(),
            ))
        }
        CoverageStatus::Partial | CoverageStatus::Unavailable | CoverageStatus::Failed
            if row.run.is_none() || row.reason.is_none() =>
        {
            Err(ModelError::Invalid(
                "attempted incomplete coverage needs an invocation and structured reason".into(),
            ))
        }
        _ => Ok(()),
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Domain, serde::Serialize, serde::Deserialize)]
#[model(name = "run_families")]
pub struct RunFamily {
    #[model(key, provenance)]
    pub run: Id<ProviderRun>,
    #[model(key)]
    pub family: FactFamily,
}
fn family_digest(families: &BTreeSet<FactFamily>) -> Result<ContentHash, ModelError> {
    if families.is_empty() {
        return Err(ModelError::Invalid(
            "invocation needs requested coverage families".into(),
        ));
    }
    let mut sink = KeySink::new("requested-families");
    for family in families {
        family.encode(&mut sink);
    }
    Ok(sink.finish())
}
impl ProviderRun {
    pub fn new(
        provider: Id<Provider>,
        context: Id<AnalysisContext>,
        input: Id<InputRevision>,
        configuration: ContentHash,
        families: impl IntoIterator<Item = FactFamily>,
    ) -> Result<(Self, Vec<RunFamily>), ModelError> {
        let families = families.into_iter().collect::<BTreeSet<_>>();
        let run = Self {
            provider,
            context,
            input,
            configuration,
            requested_families: family_digest(&families)?,
        };
        let memberships = families
            .into_iter()
            .map(|family| RunFamily {
                run: run.id(),
                family,
            })
            .collect();
        Ok((run, memberships))
    }
}

/// Derived from the selected providers and actual source/document/artifact scopes by assembly.
/// The expected invocation is exact, including configuration and requested family set.
#[derive(Debug, Clone)]
pub struct CoverageExpectation {
    pub input: Id<InputRevision>,
    pub scope: Id<CoverageScope>,
    pub provider: Id<Provider>,
    pub context: Id<AnalysisContext>,
    pub family: FactFamily,
    pub run: Option<Id<ProviderRun>>,
}

/// Missing, extra, contradictory or failed outcomes prevent publication. Empty complete scopes
/// are valid; omitted scopes and not-requested scopes cannot stand in for them.
pub fn validate_coverage_contract(
    expected: &[CoverageExpectation],
    observed: &[ProviderCoverage],
    runs: &[ProviderRun],
    requested: &[RunFamily],
    budget: &super::resources::ResourceBudget,
) -> Result<(), ModelError> {
    let mut invocation = InvocationCheck {
        charge: StateCharge::new(budget, "coverage_contract"),
        ..Default::default()
    };
    for run in runs {
        invocation.add_run(run.clone())?;
    }
    for member in requested {
        invocation.add_family(member)?;
    }
    invocation.validate_families()?;
    let mut outcomes = ChargedMap::default();
    for outcome in observed {
        outcome.validate()?;
        invocation.check_outcome(outcome)?;
        let key = (
            outcome.scope,
            outcome.provider,
            outcome.context,
            outcome.family,
            outcome.run,
        );
        if outcomes
            .insert(&mut invocation.charge, key, outcome)?
            .is_some()
        {
            return Err(ModelError::Conflict(ProviderCoverage::NAME));
        }
    }
    let mut expected_keys = ChargedSet::default();
    for item in expected {
        let key = (
            item.scope,
            item.run.map(|_| item.provider),
            item.context,
            item.family,
            item.run,
        );
        if !expected_keys.insert(&mut invocation.charge, key)? {
            return Err(ModelError::Invalid("duplicate coverage expectation".into()));
        }
        let outcome = outcomes
            .remove(&mut invocation.charge, &key)
            .ok_or_else(|| ModelError::Invalid("missing required coverage outcome".into()))?;
        if let Some(id) = item.run {
            let run = invocation
                .runs
                .get(&id)
                .ok_or_else(|| ModelError::Invalid("coverage names missing invocation".into()))?;
            if (run.input, run.provider, run.context) != (item.input, item.provider, item.context)
                || !invocation
                    .families
                    .get(&id)
                    .is_some_and(|members| members.contains(&item.family))
            {
                return Err(ModelError::Invalid(
                    "coverage disagrees with invocation".into(),
                ));
            }
            if matches!(
                outcome.status,
                CoverageStatus::NotRequested | CoverageStatus::Failed
            ) {
                return Err(ModelError::Invalid(
                    "required provider did not complete successfully".into(),
                ));
            }
        } else if outcome.status != CoverageStatus::NotRequested {
            return Err(ModelError::Invalid(
                "unrequested family has an attempted outcome".into(),
            ));
        }
    }
    if !outcomes.is_empty() {
        return Err(ModelError::Invalid(
            "coverage has undeclared scopes or invocations".into(),
        ));
    }
    Ok(())
}

/// Stored invocation membership is checked independently of compiler-provided scope expectations.
/// The latter are still required to prove that no scheduled scope was omitted.
pub(crate) fn invocation_invariants() -> Vec<super::Invariant> {
    vec![super::Invariant {
        revision: 1,
        name: "provider_invocation_membership",
        inputs: vec![
            super::ValidationInput::of::<ProviderRun>(&["id"]),
            super::ValidationInput::of::<RunFamily>(&["run", "family"]),
            super::ValidationInput::of::<ProviderCoverage>(&["id"]),
        ],
        create: std::sync::Arc::new(|budget| {
            Box::new(InvocationCheck {
                charge: StateCharge::new(budget, "provider_invocation_membership"),
                ..Default::default()
            })
        }),
    }]
}
#[derive(Default)]
struct InvocationCheck {
    charge: StateCharge,
    runs: ChargedMap<Id<ProviderRun>, ProviderRun>,
    families: ChargedMap<Id<ProviderRun>, BTreeSet<FactFamily>>,
}
impl InvocationCheck {
    fn add_run(&mut self, row: ProviderRun) -> Result<(), ModelError> {
        if self.runs.insert(&mut self.charge, row.id(), row)?.is_some() {
            return Err(ModelError::Conflict(ProviderRun::NAME));
        }
        Ok(())
    }
    fn add_family(&mut self, row: &RunFamily) -> Result<(), ModelError> {
        if !self.runs.contains_key(&row.run)
            || !self
                .families
                .update(&mut self.charge, row.run, |families| {
                    families.insert(row.family)
                })?
        {
            return Err(ModelError::Invalid(
                "invalid invocation family membership".into(),
            ));
        }
        Ok(())
    }
    fn check_outcome(&self, row: &ProviderCoverage) -> Result<(), ModelError> {
        if let Some(id) = row.run {
            let run = self
                .runs
                .get(&id)
                .ok_or_else(|| ModelError::Invalid("coverage invocation absent".into()))?;
            if (Some(run.provider), run.context) != (row.provider, row.context)
                || !self
                    .families
                    .get(&id)
                    .is_some_and(|f| f.contains(&row.family))
            {
                return Err(ModelError::Invalid(
                    "coverage disagrees with invocation".into(),
                ));
            }
        }
        if row.status == CoverageStatus::Failed {
            return Err(ModelError::Invalid(
                "failed provider invocation cannot publish".into(),
            ));
        }
        Ok(())
    }
    fn validate_families(&self) -> Result<(), ModelError> {
        for (id, run) in self.runs.iter() {
            let members = self
                .families
                .get(id)
                .ok_or_else(|| ModelError::Invalid("invocation lacks requested families".into()))?;
            if family_digest(members)? != run.requested_families {
                return Err(ModelError::Invalid(
                    "invocation family digest differs".into(),
                ));
            }
        }
        Ok(())
    }
}
impl super::InvariantCheck for InvocationCheck {
    fn visit(
        &mut self,
        relation: &str,
        batch: &arrow_array::RecordBatch,
    ) -> Result<(), ModelError> {
        if relation == ProviderRun::NAME {
            for row in ProviderRun::decode(batch)? {
                self.add_run(row)?;
            }
        } else if relation == RunFamily::NAME {
            for row in RunFamily::decode(batch)? {
                self.add_family(&row)?;
            }
        } else if relation == ProviderCoverage::NAME {
            for row in ProviderCoverage::decode(batch)? {
                self.check_outcome(&row)?;
            }
        } else {
            return Err(ModelError::Invalid(
                "undeclared invocation validation input".into(),
            ));
        }
        Ok(())
    }
    fn finish(self: Box<Self>) -> Result<(), ModelError> {
        self.validate_families()
    }
}

pub(crate) fn coverage_ownership_invariants() -> Vec<super::Invariant> {
    let mut inputs = super::ownership::ScopeIndex::inputs();
    inputs.extend([
        super::ValidationInput::of::<ProviderRun>(&["id"]),
        super::ValidationInput::of::<ProviderCoverage>(&["id"]),
    ]);
    vec![super::Invariant {
        revision: 1,
        name: "coverage_scope_ownership",
        inputs,
        create: std::sync::Arc::new(|budget| {
            Box::new(CoverageOwnership {
                charge: StateCharge::new(budget, "coverage_scope_ownership"),
                ownership: super::ownership::ScopeIndex::new(budget, "coverage_scope_ownership"),
                runs: Default::default(),
            })
        }),
    }]
}
#[derive(Default)]
struct CoverageOwnership {
    charge: StateCharge,
    ownership: super::ownership::ScopeIndex,
    runs: ChargedMap<Id<ProviderRun>, Id<InputRevision>>,
}
impl super::InvariantCheck for CoverageOwnership {
    fn visit(
        &mut self,
        relation: &str,
        batch: &arrow_array::RecordBatch,
    ) -> Result<(), ModelError> {
        if self.ownership.visit(relation, batch)? {
            return Ok(());
        }
        if relation == ProviderRun::NAME {
            for row in ProviderRun::decode(batch)? {
                self.runs.insert(&mut self.charge, row.id(), row.input)?;
            }
        } else if relation == ProviderCoverage::NAME {
            for row in ProviderCoverage::decode(batch)? {
                let scope = self.ownership.scope(row.scope)?;
                if let Some(run) = row.run {
                    let input = self
                        .runs
                        .get(&run)
                        .ok_or_else(|| ModelError::Invalid("coverage invocation missing".into()))?;
                    if !self.ownership.owns_scope(*input, scope)? {
                        return Err(ModelError::Invalid(
                            "coverage invocation does not own its scope".into(),
                        ));
                    }
                }
            }
        } else {
            return Err(ModelError::Invalid(
                "undeclared coverage ownership input".into(),
            ));
        }
        Ok(())
    }
    fn finish(self: Box<Self>) -> Result<(), ModelError> {
        Ok(())
    }
}

pub(crate) fn invocation_invariants_refs() -> Vec<&'static str> {
    vec!["provider_invocation_membership"]
}
pub(crate) fn coverage_ownership_invariants_refs() -> Vec<&'static str> {
    vec!["coverage_scope_ownership"]
}
