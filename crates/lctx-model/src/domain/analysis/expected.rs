//! Finite admitted capability contracts. Unbound method/capability pairs refuse activation.
//! Input scopes come from captured artifact roles; result observations never select the universe.
use super::{AnalysisCapability, AnalysisMethod, invalid};
use crate::domain::{
    attribution::{AnalysisContext, FactFamily, ProviderCoverage},
    input::{ArtifactUse, InputRevision},
    normalized::coverage::{Capability, NormalizationComputation, NormalizationCoverage},
    source::{CoverageScope, SourceArtifact},
    stages::{Profile, ReadPermit},
    *,
};
/// The recognized expected-domain row inventory and typed decoder dispatch share this owner.
#[macro_export]
macro_rules! expected_domain_inputs {
    ($apply:ident) => {
        $apply! {
         inputs:$crate::domain::input::InputRevision,
         artifacts:$crate::domain::source::SourceArtifact,
         uses:$crate::domain::input::ArtifactUse,
         scopes:$crate::domain::source::CoverageScope,
         computations:$crate::domain::normalized::coverage::NormalizationComputation,
         normalized:$crate::domain::normalized::coverage::NormalizationCoverage,
         native:$crate::domain::attribution::ProviderCoverage,
         text:$crate::domain::embedding::text::TextDefinition,
         analytics:$crate::domain::analysis::settings::AnalyticsConfiguration,
        }
    };
}
#[derive(Clone, Copy)]
struct ScopeContract {
    grain: admission::Grain,
    native: &'static [FactFamily],
    normalized: &'static [Capability],
}
#[derive(Clone, Copy)]
pub(crate) struct Contract {
    pub capability: AnalysisCapability,
    method: AnalysisMethod,
    scopes: &'static [ScopeContract],
    behavioral: bool,
}
impl Contract {
    fn needs_native(self) -> bool {
        self.scopes.iter().any(|s| !s.native.is_empty())
    }
}
pub(crate) fn contract(
    method: AnalysisMethod,
    capability: AnalysisCapability,
) -> Result<Contract, ModelError> {
    use admission::{ArtifactClass::*, Grain::*};
    let (scopes, behavioral): (&'static [ScopeContract], bool) = match (method, capability) {
        (AnalysisMethod::LocalTransfers, AnalysisCapability::Transfers) => (
            &[ScopeContract {
                grain: Artifact(PythonSource),
                native: &[FactFamily::Flow],
                normalized: &[
                    Capability::FlowLinks,
                    Capability::FlowEvents,
                    Capability::Bindings,
                ],
            }],
            true,
        ),
        (AnalysisMethod::Execution, AnalysisCapability::Execution) => (
            &[ScopeContract {
                grain: Artifact(PythonSource),
                native: &[
                    FactFamily::Syntax,
                    FactFamily::Signatures,
                    FactFamily::Lexical,
                    FactFamily::Flow,
                    FactFamily::Calls,
                ],
                normalized: &[Capability::Symbols, Capability::Callables],
            }],
            true,
        ),
        (AnalysisMethod::Completion, AnalysisCapability::Completion) => (
            &[ScopeContract {
                grain: Artifact(PythonSource),
                native: &[
                    FactFamily::Syntax,
                    FactFamily::Signatures,
                    FactFamily::Lexical,
                    FactFamily::Flow,
                ],
                normalized: &[Capability::Symbols, Capability::Callables],
            }],
            true,
        ),
        (AnalysisMethod::Models, AnalysisCapability::Models)
        | (AnalysisMethod::SourceCalls, AnalysisCapability::Execution)
        | (AnalysisMethod::EnrichedExecution, AnalysisCapability::Execution) => (
            &[ScopeContract {
                grain: Artifact(PythonSource),
                native: &[
                    FactFamily::Syntax,
                    FactFamily::Signatures,
                    FactFamily::Lexical,
                    FactFamily::Flow,
                ],
                normalized: &[
                    Capability::Symbols,
                    Capability::Callables,
                    Capability::Calls,
                    Capability::Bindings,
                    Capability::FlowEvents,
                ],
            }],
            true,
        ),
        (AnalysisMethod::Summaries, AnalysisCapability::Summaries) => (
            &[
                ScopeContract {
                    grain: Artifact(PythonSource),
                    native: &[
                        FactFamily::Syntax,
                        FactFamily::Signatures,
                        FactFamily::Lexical,
                        FactFamily::Flow,
                    ],
                    normalized: &[
                        Capability::Symbols,
                        Capability::Callables,
                        Capability::Calls,
                        Capability::Bindings,
                        Capability::FlowLinks,
                        Capability::FlowEvents,
                    ],
                },
                ScopeContract {
                    grain: Input,
                    native: &[],
                    normalized: &[Capability::InvocationProjection],
                },
            ],
            true,
        ),
        (AnalysisMethod::Delegation, AnalysisCapability::Delegation) => (
            &[
                ScopeContract {
                    grain: Artifact(PythonSource),
                    native: &[],
                    normalized: &[
                        Capability::Symbols,
                        Capability::PublicExposure,
                        Capability::Callables,
                    ],
                },
                ScopeContract {
                    grain: Input,
                    native: &[],
                    normalized: &[
                        Capability::InvocationProjection,
                        Capability::DefinitionProjection,
                    ],
                },
            ],
            false,
        ),
        (AnalysisMethod::Controls, AnalysisCapability::Controls) => (
            &[ScopeContract {
                grain: Artifact(PythonSource),
                native: &[FactFamily::Syntax, FactFamily::Flow],
                normalized: &[
                    Capability::Calls,
                    Capability::Bindings,
                    Capability::Callables,
                    Capability::FlowEvents,
                ],
            }],
            true,
        ),
        (AnalysisMethod::Handoffs, AnalysisCapability::Handoffs) => (
            &[ScopeContract {
                grain: Artifact(PythonSource),
                native: &[FactFamily::Syntax],
                normalized: &[
                    Capability::Calls,
                    Capability::Bindings,
                    Capability::Callables,
                    Capability::PublicExposure,
                ],
            }],
            false,
        ),
        (AnalysisMethod::DirectUsage, AnalysisCapability::DirectUsage) => (
            &[ScopeContract {
                grain: Artifact(PythonSource),
                native: &[],
                normalized: &[
                    Capability::Calls,
                    Capability::Callables,
                    Capability::PublicExposure,
                ],
            }],
            false,
        ),
        (AnalysisMethod::PageRank, AnalysisCapability::PageRank)
        | (AnalysisMethod::Communities, AnalysisCapability::Communities)
        | (AnalysisMethod::Concepts, AnalysisCapability::Concepts)
        | (AnalysisMethod::RelationalConcepts, AnalysisCapability::RelationalConcepts)
        | (AnalysisMethod::Neighbours, AnalysisCapability::Neighbours) => (
            &[
                ScopeContract {
                    grain: Artifact(PythonSource),
                    native: &[],
                    normalized: &[
                        Capability::Symbols,
                        Capability::Callables,
                        Capability::PublicExposure,
                    ],
                },
                ScopeContract {
                    grain: Input,
                    native: &[],
                    normalized: &[Capability::InvocationProjection],
                },
            ],
            false,
        ),
        (AnalysisMethod::Catalog, AnalysisCapability::Catalog) => (
            &[ScopeContract {
                grain: Artifact(PythonSource),
                native: &[],
                normalized: &[
                    Capability::PublicExposure,
                    Capability::Symbols,
                    Capability::Ancestry,
                    Capability::Types,
                    Capability::Callables,
                ],
            }],
            false,
        ),
        (AnalysisMethod::CatalogEvidence, AnalysisCapability::CatalogEvidence) => (
            &[
                ScopeContract {
                    grain: Artifact(PythonSource),
                    native: &[FactFamily::Syntax],
                    normalized: &[
                        Capability::Symbols,
                        Capability::References,
                        Capability::Callables,
                        Capability::Calls,
                        Capability::Bindings,
                    ],
                },
                ScopeContract {
                    grain: Artifact(Document),
                    native: &[FactFamily::Docs],
                    normalized: &[Capability::Mentions],
                },
                ScopeContract {
                    grain: Input,
                    native: &[FactFamily::Deployment],
                    normalized: &[],
                },
            ],
            false,
        ),
        (AnalysisMethod::CatalogSelection, AnalysisCapability::CatalogSelection)
        | (AnalysisMethod::Synthesis, AnalysisCapability::Synthesis)
        | (AnalysisMethod::Retrieval, AnalysisCapability::Retrieval) => (
            &[
                ScopeContract {
                    grain: Artifact(PythonSource),
                    native: &[FactFamily::Syntax],
                    normalized: &[
                        Capability::PublicExposure,
                        Capability::Symbols,
                        Capability::Ancestry,
                        Capability::Types,
                        Capability::References,
                        Capability::Callables,
                        Capability::Calls,
                        Capability::Bindings,
                    ],
                },
                ScopeContract {
                    grain: Artifact(Document),
                    native: &[FactFamily::Docs],
                    normalized: &[Capability::Mentions],
                },
                ScopeContract {
                    grain: Input,
                    native: &[FactFamily::Deployment],
                    normalized: &[],
                },
            ],
            false,
        ),
        (AnalysisMethod::AnalyticEmbedding, AnalysisCapability::AnalyticEmbedding) => (
            &[
                ScopeContract {
                    grain: Artifact(PythonSource),
                    native: &[FactFamily::Syntax, FactFamily::Signatures],
                    normalized: &[Capability::Symbols, Capability::Callables],
                },
                ScopeContract {
                    grain: Artifact(Document),
                    native: &[FactFamily::Docs],
                    normalized: &[],
                },
            ],
            false,
        ),
        _ => return Err(invalid("analysis method/capability contract is not bound")),
    };
    Ok(Contract {
        capability,
        method,
        scopes,
        behavioral,
    })
}
pub(crate) fn method_contract(method: AnalysisMethod) -> Result<Contract, ModelError> {
    let capability = match method {
        AnalysisMethod::LocalTransfers => AnalysisCapability::Transfers,
        AnalysisMethod::Execution => AnalysisCapability::Execution,
        AnalysisMethod::SourceCalls | AnalysisMethod::EnrichedExecution => {
            AnalysisCapability::Execution
        }
        AnalysisMethod::Completion => AnalysisCapability::Completion,
        AnalysisMethod::Models => AnalysisCapability::Models,
        AnalysisMethod::Summaries => AnalysisCapability::Summaries,
        AnalysisMethod::Catalog => AnalysisCapability::Catalog,
        AnalysisMethod::Delegation => AnalysisCapability::Delegation,
        AnalysisMethod::DirectUsage => AnalysisCapability::DirectUsage,
        AnalysisMethod::Handoffs => AnalysisCapability::Handoffs,
        AnalysisMethod::Controls => AnalysisCapability::Controls,
        AnalysisMethod::CatalogEvidence => AnalysisCapability::CatalogEvidence,
        AnalysisMethod::CatalogSelection => AnalysisCapability::CatalogSelection,
        AnalysisMethod::Synthesis => AnalysisCapability::Synthesis,
        AnalysisMethod::Retrieval => AnalysisCapability::Retrieval,
        AnalysisMethod::AnalyticEmbedding => AnalysisCapability::AnalyticEmbedding,
        AnalysisMethod::PageRank => AnalysisCapability::PageRank,
        AnalysisMethod::Communities => AnalysisCapability::Communities,
        AnalysisMethod::Concepts => AnalysisCapability::Concepts,
        AnalysisMethod::RelationalConcepts => AnalysisCapability::RelationalConcepts,
        AnalysisMethod::Neighbours => AnalysisCapability::Neighbours,
        _ => {
            return Err(invalid(
                "analysis method has no admitted capability contract",
            ));
        }
    };
    contract(method, capability)
}
pub(crate) fn inputs(method: AnalysisMethod) -> Vec<ValidationInput> {
    let contract = method_contract(method).expect("bound method has a finite expected contract");
    let mut inputs = vec![
        ValidationInput::of::<InputRevision>(&["id"]),
        ValidationInput::of::<SourceArtifact>(&["id"]),
        ValidationInput::of::<ArtifactUse>(&["id"]),
        ValidationInput::of::<CoverageScope>(&["id"]),
        ValidationInput::of::<NormalizationComputation>(&["id"]),
        ValidationInput::of::<NormalizationCoverage>(&["id"]),
    ];
    if contract.needs_native() {
        inputs.push(ValidationInput::of::<ProviderCoverage>(&["id"]));
    }
    if method == AnalysisMethod::AnalyticEmbedding {
        inputs.push(ValidationInput::of::<embedding::text::TextDefinition>(&[
            "id",
        ]));
    }
    if crate::domain::analytics::build::METHODS.contains(&method) {
        inputs.push(ValidationInput::of::<
            crate::domain::analysis::settings::AnalyticsConfiguration,
        >(&["id"]));
    }
    inputs
}
pub(crate) struct FrontierIndex {
    charge: charged::StateCharge,
    profile: Profile,
    inputs: charged::ChargedMap<Id<InputRevision>, InputRevision>,
    artifacts: charged::ChargedMap<Id<SourceArtifact>, SourceArtifact>,
    uses: charged::ChargedMap<Id<ArtifactUse>, ArtifactUse>,
    scopes: charged::ChargedMap<Id<CoverageScope>, CoverageScope>,
    computations: charged::ChargedMap<Id<NormalizationComputation>, NormalizationComputation>,
    normalized: charged::ChargedMap<Id<NormalizationCoverage>, NormalizationCoverage>,
    native: charged::ChargedMap<Id<ProviderCoverage>, ProviderCoverage>,
    analytics: charged::ChargedMap<
        Id<crate::domain::analysis::settings::AnalyticsConfiguration>,
        crate::domain::analysis::settings::AnalyticsConfiguration,
    >,
    text: charged::ChargedMap<Id<embedding::text::TextDefinition>, embedding::text::TextDefinition>,
}
impl FrontierIndex {
    pub(crate) fn set_profile(&mut self, profile: Profile) {
        self.profile = profile;
    }
    pub(crate) fn new(profile: Profile, budget: &resources::ResourceBudget) -> Self {
        Self {
            charge: charged::StateCharge::new(budget, "analysis_expected_frontier"),
            profile,
            inputs: Default::default(),
            artifacts: Default::default(),
            uses: Default::default(),
            scopes: Default::default(),
            computations: Default::default(),
            normalized: Default::default(),
            native: Default::default(),
            analytics: Default::default(),
            text: Default::default(),
        }
    }
    pub(crate) fn visit(
        &mut self,
        relation: &str,
        batch: &arrow_array::RecordBatch,
    ) -> Result<bool, ModelError> {
        self.visit_with_check(relation, batch, || Ok(()))
    }
    fn visit_with_check(
        &mut self,
        relation: &str,
        batch: &arrow_array::RecordBatch,
        check: impl FnOnce() -> Result<(), ModelError>,
    ) -> Result<bool, ModelError> {
        macro_rules! insert {
            ($r:ty,$field:ident) => {
                if relation == <$r>::NAME {
                    check()?;
                    for row in <$r>::decode(batch)? {
                        if self
                            .$field
                            .insert(&mut self.charge, row.id(), row)?
                            .is_some()
                        {
                            return Err(ModelError::Conflict(<$r>::NAME));
                        }
                    }
                    return Ok(true);
                }
            };
        }
        macro_rules! dispatch {($($field:ident:$ty:ty,)*)=>{$(insert!($ty,$field);)*};}
        crate::expected_domain_inputs!(dispatch);
        Ok(false)
    }
    pub(crate) fn domain(
        &self,
        input: Id<InputRevision>,
        context: Id<AnalysisContext>,
        contract: Contract,
    ) -> Result<ExpectedDomain, ModelError> {
        if !self.inputs.contains_key(&input) {
            return Err(invalid("expected analysis input absent"));
        }
        let budget = self
            .charge
            .budget()
            .ok_or_else(|| invalid("expected frontier budget absent"))?;
        let bytes = self
            .artifacts
            .values()
            .try_fold(0usize, |n, r| {
                n.checked_add(size_of::<SourceArtifact>() + r.heap_bytes() + 256)
            })
            .and_then(|n| {
                n.checked_add(
                    self.uses
                        .len()
                        .checked_mul(size_of::<ArtifactUse>() + 256)?,
                )
            })
            .ok_or_else(|| invalid("expected root allocation overflow"))?;
        let source_bytes = self
            .native
            .values()
            .try_fold(0usize, |n, r| {
                n.checked_add(size_of::<ProviderCoverage>() + r.heap_bytes() + 32)
            })
            .and_then(|n| {
                n.checked_add(
                    self.normalized
                        .len()
                        .checked_mul(size_of::<NormalizationCoverage>() + 32)?,
                )
            })
            .and_then(|n| {
                n.checked_add(
                    self.computations
                        .len()
                        .checked_mul(size_of::<&NormalizationComputation>())?,
                )
            })
            .ok_or_else(|| invalid("expected lower allocation overflow"))?;
        let reservation = budget.reserve(
            "analysis_expected_roots",
            bytes
                .checked_add(source_bytes)
                .and_then(|n| {
                    n.checked_add(
                        self.artifacts
                            .len()
                            .checked_mul(size_of::<ExpectedScope>() + 128)?,
                    )
                })
                .and_then(|n| n.checked_add(size_of::<ExpectedScope>()))
                .ok_or_else(|| invalid("expected domain allocation overflow"))?,
        )?;
        let artifacts = self.artifacts.values().cloned().collect::<Vec<_>>();
        let uses = self.uses.values().cloned().collect::<Vec<_>>();
        let roots = admission::analysis_roots(&artifacts, &uses)?;
        let mut selected = charged::ChargedMap::default();
        let mut charge = charged::StateCharge::new(budget, "analysis_expected_scopes");
        for (ordinal, scope_contract) in contract.scopes.iter().enumerate() {
            let mut add = |scope: CoverageScope| -> Result<(), ModelError> {
                if self.scopes.get(&scope.id()) != Some(&scope) {
                    return Err(invalid("expected captured scope absent"));
                }
                if selected
                    .insert(&mut charge, scope.id(), (scope, ordinal))?
                    .is_some()
                {
                    return Err(invalid("overlapping expected scope contracts"));
                }
                Ok(())
            };
            match scope_contract.grain {
                admission::Grain::Input => add(CoverageScope::Input { input })?,
                admission::Grain::Artifact(class) => {
                    for artifact in self.artifacts.values().filter(|r| {
                        r.input == input
                            && roots.contains(&r.id())
                            && admission::ArtifactClass::of(&r.path) == Some(class)
                    }) {
                        add(CoverageScope::Artifact {
                            artifact: artifact.id(),
                        })?;
                    }
                }
            }
        }
        let requested = if contract.method == AnalysisMethod::AnalyticEmbedding {
            if self.text.len() != 1 {
                return Err(invalid(
                    "analytic embedding requires one completed text definition",
                ));
            }
            self.text
                .values()
                .next()
                .expect("one text definition")
                .requested
        } else if crate::domain::analytics::build::METHODS.contains(&contract.method) {
            if self.analytics.len() != 1 {
                return Err(invalid(
                    "optional analytics requires one immutable configuration",
                ));
            }
            crate::domain::analytics::build::selected(
                self.analytics
                    .values()
                    .next()
                    .expect("one analytics configuration"),
                contract.method,
            )
        } else {
            !contract.behavioral || self.profile == Profile::Behavioral
        };
        if selected.is_empty() {
            let scope = CoverageScope::Input { input };
            if self.scopes.get(&scope.id()) != Some(&scope) {
                return Err(invalid("empty input analysis scope absent"));
            }
            return Ok(ExpectedDomain {
                scopes: vec![ExpectedScope {
                    scope: scope.id(),
                    context,
                    requested,
                    no_scope: requested,
                    native: vec![],
                    normalized: vec![],
                }],
                _reservation: reservation,
            });
        }
        let _outputs = budget.reserve(
            "analysis_expected_domain",
            selected
                .len()
                .checked_mul(size_of::<ExpectedScope>() + 256)
                .ok_or_else(|| invalid("expected scope output allocation overflow"))?,
        )?;
        let mut out = Vec::with_capacity(selected.len());
        for (scope, (_, ordinal)) in selected.iter() {
            let scope_contract = &contract.scopes[*ordinal];
            let mut native = Vec::new();
            let mut normalized = Vec::new();
            for family in scope_contract.native {
                let rows = self
                    .native
                    .values()
                    .filter(|r| r.scope == *scope && r.context == context && r.family == *family)
                    .collect::<Vec<_>>();
                if rows.is_empty() {
                    return Err(invalid("expected native family scope absent"));
                }
                for row in rows {
                    row.validate()?;
                    native.push(row.clone());
                }
            }
            for capability in scope_contract.normalized {
                let computations = self
                    .computations
                    .values()
                    .filter(|r| r.capability == *capability)
                    .collect::<Vec<_>>();
                if computations.len() != 1 {
                    return Err(invalid(
                        "expected normalized capability declaration absent or ambiguous",
                    ));
                }
                let computation = computations[0];
                if computation.profile != self.profile.name() {
                    return Err(invalid("normalized capability has another profile"));
                }
                let rows = self
                    .normalized
                    .values()
                    .filter(|r| {
                        r.scope == *scope
                            && r.context == context
                            && r.computation == computation.id()
                    })
                    .collect::<Vec<_>>();
                if rows.len() != 1 {
                    return Err(invalid(
                        "expected normalized capability scope absent or ambiguous",
                    ));
                }
                normalized.push(rows[0].clone());
            }
            out.push(ExpectedScope {
                scope: *scope,
                context,
                requested,
                no_scope: false,
                native,
                normalized,
            });
        }
        Ok(ExpectedDomain {
            scopes: out,
            _reservation: reservation,
        })
    }
}
pub(crate) struct ExpectedDomain {
    pub scopes: Vec<ExpectedScope>,
    _reservation: Box<dyn resources::Reservation>,
}
pub(crate) struct ExpectedScope {
    pub scope: Id<CoverageScope>,
    pub context: Id<AnalysisContext>,
    pub requested: bool,
    pub no_scope: bool,
    pub native: Vec<ProviderCoverage>,
    pub normalized: Vec<NormalizationCoverage>,
}
/// Conditional expected-domain routing never treats an unrelated relation as evidence.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VisitResult {
    Handled,
    Skipped,
}
/// Capture binds the expected-domain operation to actual R0 inputs. Batch membership is independently
/// rechecked from effect-owner physical inputs before publication; this is not a new read authority.
pub struct CoverageAdmission<'a> {
    sources: &'a super::sources::CapturedSources,
    index: FrontierIndex,
}
impl<'a> CoverageAdmission<'a> {
    pub fn new(
        sources: &'a super::sources::CapturedSources,
        budget: &resources::ResourceBudget,
    ) -> Result<Self, ModelError> {
        let profile = sources
            .profile()
            .ok_or_else(|| invalid("coverage admission requires a captured stage profile"))?;
        Ok(Self {
            sources,
            index: FrontierIndex::new(profile, budget),
        })
    }
    /// Route expected-domain rows through the owned dispatch. A skipped relation does not
    /// establish completeness; required method inputs are checked against the capture in domain.
    pub fn visit_if_expected<R: Record>(
        &mut self,
        permit: &ReadPermit<'_, R>,
        batch: &arrow_array::RecordBatch,
    ) -> Result<VisitResult, ModelError> {
        // The dispatch checks source identity for handled inputs before decoding or mutation.
        Ok(
            if self
                .index
                .visit_with_check(R::NAME, batch, || self.sources.accepts(permit))?
            {
                VisitResult::Handled
            } else {
                VisitResult::Skipped
            },
        )
    }
    pub fn visit<R: Record>(
        &mut self,
        permit: &ReadPermit<'_, R>,
        batch: &arrow_array::RecordBatch,
    ) -> Result<(), ModelError> {
        self.sources.accepts(permit)?;
        match self.visit_if_expected(permit, batch)? {
            VisitResult::Handled => Ok(()),
            VisitResult::Skipped => Err(invalid("relation is not an expected-domain input")),
        }
    }
    pub(crate) fn domain(
        &self,
        input: Id<InputRevision>,
        context: Id<AnalysisContext>,
        method: AnalysisMethod,
        capability: AnalysisCapability,
        source_digest: ContentHash,
    ) -> Result<ExpectedDomain, ModelError> {
        if source_digest != self.sources.digest() {
            return Err(invalid("expected domain belongs to another source capture"));
        }
        let contract = contract(method, capability)?;
        for input in inputs(method) {
            if !self.sources.has_relation(input.name()) {
                return Err(invalid(
                    "expected-domain relation was not declared and completed",
                ));
            }
        }
        self.index.domain(input, context, contract)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{
        attribution::{CoverageStatus, Provider, ProviderRun},
        embedding::text::TextDefinition,
        input::{ManifestEntry, SourceRole},
        normalized::coverage::EvidenceAvailability,
    };
    struct Fixture {
        index: FrontierIndex,
        input: Id<InputRevision>,
        context: Id<AnalysisContext>,
        python: Id<CoverageScope>,
        document: Id<CoverageScope>,
        root: Id<CoverageScope>,
        budget: resources::ResourceBudget,
    }
    fn visit<R: Record>(index: &mut FrontierIndex, rows: &[R]) {
        assert!(index.visit(R::NAME, &R::encode(rows).unwrap()).unwrap());
    }
    fn fixture() -> Fixture {
        fixture_profile(Profile::Catalog)
    }
    fn fixture_profile(profile: Profile) -> Fixture {
        let budget = resources::ResourceBudget::fixed(1 << 24).unwrap();
        let mut index = FrontierIndex::new(profile, &budget);
        let input = InputRevision::from_entries(
            [
                ("api.py", b"x=1\n".as_slice()),
                ("guide.md", b"# Guide\n".as_slice()),
                ("pyproject.toml", b"[project]\n".as_slice()),
            ]
            .into_iter()
            .map(|(path, bytes)| ManifestEntry {
                path: path.into(),
                content: ContentHash::of(bytes),
                byte_len: bytes.len() as i64,
            })
            .collect(),
        )
        .unwrap();
        let context = AnalysisContext {
            python_version: "3.14.7".into(),
            python_platform: "linux".into(),
            search_path: vec![],
            site_package_path: vec![],
            config_digest: ContentHash::of(b"finite-grain-control"),
            environment_digest: input.manifest,
            lock_digest: None,
        };
        let provider = Provider {
            tool: "contract-fixture".into(),
            revision: "1".into(),
            build_digest: ContentHash::of(b"contract"),
        };
        let (run, _) = ProviderRun::new(
            provider.id(),
            context.id(),
            input.id(),
            context.config_digest,
            [
                FactFamily::Syntax,
                FactFamily::Signatures,
                FactFamily::Docs,
                FactFamily::Deployment,
            ],
        )
        .unwrap();
        let artifacts = [
            ("api.py", b"x=1\n".as_slice(), SourceRole::Release),
            ("guide.md", b"# Guide\n".as_slice(), SourceRole::Document),
            (
                "pyproject.toml",
                b"[project]\n".as_slice(),
                SourceRole::Configuration,
            ),
        ]
        .map(|(path, bytes, role)| {
            (
                SourceArtifact::from_bytes(input.id(), path.into(), bytes).unwrap(),
                role,
            )
        });
        visit(&mut index, std::slice::from_ref(&input));
        for (artifact, role) in &artifacts {
            visit(&mut index, std::slice::from_ref(artifact));
            visit(
                &mut index,
                &[ArtifactUse {
                    input: input.id(),
                    artifact: artifact.id(),
                    role: *role,
                }],
            );
            visit(
                &mut index,
                &[CoverageScope::Artifact {
                    artifact: artifact.id(),
                }],
            );
        }
        let python = CoverageScope::Artifact {
            artifact: artifacts[0].0.id(),
        }
        .id();
        let document = CoverageScope::Artifact {
            artifact: artifacts[1].0.id(),
        }
        .id();
        let root = CoverageScope::Input { input: input.id() };
        visit(&mut index, std::slice::from_ref(&root));
        for (scope, family) in [
            (python, FactFamily::Syntax),
            (python, FactFamily::Signatures),
            (document, FactFamily::Docs),
            (root.id(), FactFamily::Deployment),
        ] {
            visit(
                &mut index,
                &[ProviderCoverage {
                    scope,
                    provider: Some(provider.id()),
                    context: context.id(),
                    family,
                    run: Some(run.id()),
                    status: CoverageStatus::CompleteUnderStatedModel,
                    reason: None,
                    diagnostic: None,
                }],
            );
        }
        for (scope, capability) in [
            (python, Capability::Symbols),
            (python, Capability::References),
            (python, Capability::Callables),
            (python, Capability::Calls),
            (python, Capability::Bindings),
            (document, Capability::Mentions),
        ] {
            let computation = NormalizationComputation {
                capability,
                policy: ContentHash::of(b"normalization-contract"),
                producer: "contract".into(),
                declaration: ContentHash::of(b"normalization-contract"),
                profile: profile.name().into(),
                availability: EvidenceAvailability::Complete,
            };
            visit(&mut index, std::slice::from_ref(&computation));
            visit(
                &mut index,
                &[NormalizationCoverage {
                    computation: computation.id(),
                    scope,
                    context: context.id(),
                    availability: EvidenceAvailability::Complete,
                }],
            );
        }
        Fixture {
            index,
            input: input.id(),
            context: context.id(),
            python,
            document,
            root: root.id(),
            budget,
        }
    }
    #[test]
    fn base_execution_requires_exact_native_families_and_retains_catalog_not_requested() {
        for profile in [Profile::Catalog, Profile::Behavioral] {
            let mut f = fixture_profile(profile);
            for method in [AnalysisMethod::Execution, AnalysisMethod::Completion] {
                assert!(
                    f.index
                        .domain(f.input, f.context, method_contract(method).unwrap())
                        .is_err()
                );
            }
            let template = f
                .index
                .native
                .values()
                .find(|r| r.scope == f.python && r.family == FactFamily::Syntax)
                .unwrap()
                .clone();
            for family in [FactFamily::Lexical, FactFamily::Calls, FactFamily::Flow] {
                let mut coverage = ProviderCoverage {
                    family,
                    ..template.clone()
                };
                if family == FactFamily::Flow && profile == Profile::Catalog {
                    coverage.status = CoverageStatus::NotRequested;
                    coverage.run = None;
                    coverage.provider = None;
                }
                visit(&mut f.index, &[coverage]);
            }
            for method in [AnalysisMethod::Execution, AnalysisMethod::Completion] {
                let domain = f
                    .index
                    .domain(f.input, f.context, method_contract(method).unwrap())
                    .unwrap();
                assert_eq!(domain.scopes.len(), 1);
                let scope = &domain.scopes[0];
                assert_eq!(scope.scope, f.python);
                assert_eq!(scope.requested, profile == Profile::Behavioral);
                assert_eq!(
                    scope.native.len(),
                    if method == AnalysisMethod::Execution {
                        5
                    } else {
                        4
                    }
                );
                assert_eq!(scope.normalized.len(), 2);
                assert!(scope.normalized.iter().all(|r| matches!(
                    f.index.computations.get(&r.computation).unwrap().capability,
                    Capability::Symbols | Capability::Callables
                )));
                let flow = scope
                    .native
                    .iter()
                    .find(|r| r.family == FactFamily::Flow)
                    .unwrap();
                assert_eq!(
                    flow.status == CoverageStatus::NotRequested,
                    profile == Profile::Catalog
                );
                assert!(contract(method, AnalysisCapability::Catalog).is_err());
            }
            drop(f.index);
            assert_eq!(f.budget.reserved(), 0);
        }
    }
    #[test]
    fn catalog_evidence_uses_document_and_input_grains_without_fabricated_python_receipts() {
        let mut f = fixture();
        let contract = method_contract(AnalysisMethod::CatalogEvidence).unwrap();
        let domain = f.index.domain(f.input, f.context, contract).unwrap();
        assert_eq!(domain.scopes.len(), 3);
        let doc = domain
            .scopes
            .iter()
            .find(|r| r.scope == f.document)
            .unwrap();
        assert_eq!(doc.native.len(), 1);
        assert_eq!(doc.native[0].family, FactFamily::Docs);
        assert_eq!(doc.normalized.len(), 1);
        let root = domain.scopes.iter().find(|r| r.scope == f.root).unwrap();
        assert_eq!(root.native[0].family, FactFamily::Deployment);
        assert!(root.normalized.is_empty());
        let python = domain.scopes.iter().find(|r| r.scope == f.python).unwrap();
        assert_eq!(python.native[0].family, FactFamily::Syntax);
        assert_eq!(python.normalized.len(), 5);
        drop(domain);
        let deployment = f
            .index
            .native
            .values()
            .find(|r| r.family == FactFamily::Deployment)
            .unwrap()
            .id();
        f.index.native.remove(&mut f.index.charge, &deployment);
        assert!(f.index.domain(f.input, f.context, contract).is_err());
        drop(f.index);
        assert_eq!(f.budget.reserved(), 0);
    }
    #[test]
    fn catalog_selection_requires_both_catalog_lower_contracts_without_optional_analytics() {
        let mut f = fixture();
        let contract = method_contract(AnalysisMethod::CatalogSelection).unwrap();
        assert!(
            f.index.domain(f.input, f.context, contract).is_err(),
            "C1 lower receipts alone cannot close C0 exposure/type domains"
        );
        for capability in [
            Capability::PublicExposure,
            Capability::Ancestry,
            Capability::Types,
        ] {
            let computation = NormalizationComputation {
                capability,
                policy: ContentHash::of(b"normalization-contract"),
                producer: "contract".into(),
                declaration: ContentHash::of(b"normalization-contract"),
                profile: Profile::Catalog.name().into(),
                availability: EvidenceAvailability::Complete,
            };
            visit(&mut f.index, std::slice::from_ref(&computation));
            let coverage = NormalizationCoverage {
                computation: computation.id(),
                scope: f.python,
                context: f.context,
                availability: EvidenceAvailability::Complete,
            };
            visit(&mut f.index, &[coverage]);
        }
        let domain = f.index.domain(f.input, f.context, contract).unwrap();
        assert_eq!(domain.scopes.len(), 3);
        assert!(domain.scopes.iter().all(|r| r.requested));
        let python = domain.scopes.iter().find(|r| r.scope == f.python).unwrap();
        assert_eq!(python.normalized.len(), 8);
        assert_eq!(python.native.len(), 1);
        let inputs = crate::domain::analysis::selection::Invocation::invariants()
            .remove(0)
            .inputs;
        assert!(
            inputs
                .iter()
                .any(|i| i.name() == crate::domain::analysis::catalog_evidence::Invocation::NAME)
        );
        assert!(!inputs.iter().any(|i| {
            [
                crate::domain::analysis::structural::Invocation::NAME,
                crate::domain::analysis::analytic::Invocation::NAME,
            ]
            .contains(&i.name())
        }));
        drop(domain);
        drop(f.index);
        assert_eq!(f.budget.reserved(), 0);
    }
    #[test]
    fn analytic_selection_is_explicit_and_dependencies_follow_each_artifact_class() {
        for requested in [false, true] {
            let mut f = fixture();
            let contract = method_contract(AnalysisMethod::AnalyticEmbedding).unwrap();
            assert!(
                f.index.domain(f.input, f.context, contract).is_err(),
                "missing immutable selection cannot choose a request"
            );
            visit(
                &mut f.index,
                &[TextDefinition {
                    requested,
                    ..TextDefinition::builtin()
                }],
            );
            let domain = f.index.domain(f.input, f.context, contract).unwrap();
            assert_eq!(domain.scopes.len(), 2);
            assert!(domain.scopes.iter().all(|r| r.requested == requested));
            let doc = domain
                .scopes
                .iter()
                .find(|r| r.scope == f.document)
                .unwrap();
            assert_eq!(doc.native[0].family, FactFamily::Docs);
            assert!(doc.normalized.is_empty());
            let python = domain.scopes.iter().find(|r| r.scope == f.python).unwrap();
            assert_eq!(python.native.len(), 2);
            assert_eq!(python.normalized.len(), 2);
            drop(domain);
            drop(f.index);
            assert_eq!(f.budget.reserved(), 0);
        }
    }
}
