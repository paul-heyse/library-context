//! Normalization completion and evidence availability are distinct, scoped contracts.
//! Expected outcomes come from admitted facts coverage, never from observed normalized rows.
use super::*;
use crate::domain::{
    admission::{Availability, CoverageEvidence, ScopedAvailability},
    attribution::*,
    source::*,
    stages::*,
};
use crate::{Domain, DomainCode};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, DomainCode)]
#[repr(i16)]
/// These are evidence capabilities, not aliases for every output of their producer.
/// Entities covers the source/entity universe and ownership; Symbols covers correspondence;
/// PublicExposure owns export paths; FlowLinks owns N2 place/test-operand links and FlowEvents
/// owns N4 flow-call links. Output receipts attest completion of each owning stage as a whole.
pub enum Capability {
    Entities = 0,
    Symbols = 1,
    References = 2,
    Imports = 3,
    Ancestry = 4,
    Types = 5,
    Mentions = 6,
    Callables = 7,
    Calls = 8,
    Bindings = 9,
    FlowLinks = 10,
    InvocationProjection = 11,
    DefinitionProjection = 12,
    ReferenceProjection = 13,
    ExposureProjection = 14,
    PublicExposure = 15,
    FlowEvents = 16,
}
impl Capability {
    pub const ALL: [Self; 17] = [
        Self::Entities,
        Self::Symbols,
        Self::References,
        Self::Imports,
        Self::Ancestry,
        Self::Types,
        Self::Mentions,
        Self::Callables,
        Self::Calls,
        Self::Bindings,
        Self::FlowLinks,
        Self::InvocationProjection,
        Self::DefinitionProjection,
        Self::ReferenceProjection,
        Self::ExposureProjection,
        Self::PublicExposure,
        Self::FlowEvents,
    ];
    pub fn anchor(self) -> FactFamily {
        match self {
            Self::Entities => FactFamily::Syntax,
            Self::Imports | Self::PublicExposure => FactFamily::Exports,
            Self::Symbols | Self::Ancestry | Self::Callables => FactFamily::Signatures,
            Self::References => FactFamily::Lexical,
            Self::Types => FactFamily::Types,
            Self::Mentions => FactFamily::Docs,
            Self::Calls | Self::Bindings => FactFamily::Calls,
            Self::FlowLinks | Self::FlowEvents => FactFamily::Flow,
            _ => FactFamily::Artifacts,
        }
    }
    pub fn families(self) -> &'static [FactFamily] {
        use FactFamily::*;
        match self {
            Self::Entities => &[Artifacts, Syntax, Signatures, Types, Lexical, Flow],
            Self::Symbols => &[Artifacts, Syntax, Signatures, Types, Lexical],
            Self::Callables => &[Artifacts, Syntax, Signatures, Types, Lexical],
            Self::References => &[Artifacts, Syntax, Signatures, Lexical, Types],
            Self::Imports => &[Artifacts, Syntax, Signatures, Exports],
            Self::PublicExposure => &[Artifacts, Syntax, Signatures, Exports, Types, Lexical],
            Self::Ancestry => &[Artifacts, Syntax, Signatures],
            Self::Types => &[Artifacts, Syntax, Signatures, Types, Lexical],
            Self::Mentions => &[Artifacts, Docs, Exports, Signatures, Syntax],
            Self::Calls => &[Artifacts, Syntax, Signatures, Calls],
            Self::Bindings => &[Artifacts, Syntax, Signatures, Calls, Types, Lexical],
            Self::FlowLinks => &[Artifacts, Syntax, Signatures, Types, Flow, Lexical],
            Self::FlowEvents => &[Artifacts, Syntax, Signatures, Calls, Flow],
            Self::InvocationProjection | Self::DefinitionProjection => {
                projection::ProjectionSpec::builtin(projection::ProjectionName::CallableInvocation)
                    .families()
            }
            Self::ReferenceProjection => {
                projection::ProjectionSpec::builtin(projection::ProjectionName::ImportReference)
                    .families()
            }
            Self::ExposureProjection => {
                projection::ProjectionSpec::builtin(projection::ProjectionName::PublicExposure)
                    .families()
            }
        }
    }
    pub fn producer(self, profile: Profile) -> Stage {
        match self {
            Self::Entities | Self::Symbols | Self::PublicExposure => entity_normalization::stage(),
            Self::References
            | Self::Imports
            | Self::Ancestry
            | Self::Types
            | Self::Mentions
            | Self::FlowLinks => relation_normalization::stage(profile),
            Self::Callables => callable_normalization::stage(profile),
            Self::Calls | Self::FlowEvents => event_normalization::stage(profile),
            Self::Bindings => binding_normalization::stage(profile),
            _ => projection::normalization::stage(profile),
        }
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, DomainCode)]
#[repr(i16)]
pub enum EvidenceAvailability {
    Complete = 0,
    Partial = 1,
    Unavailable = 2,
    NotRequested = 3,
    NoScope = 4,
}
#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(name = "normalization_computations")]
pub struct NormalizationComputation {
    #[model(key)]
    pub capability: Capability,
    #[model(key)]
    pub policy: ContentHash,
    pub producer: String,
    pub declaration: ContentHash,
    pub profile: String,
    /// Aggregate only; narrower claims must inspect NormalizationCoverage and its premises.
    pub availability: EvidenceAvailability,
}
#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(name = "normalization_output_receipts")]
pub struct NormalizationOutputReceipt {
    #[model(key)]
    pub computation: Id<NormalizationComputation>,
    #[model(key)]
    pub relation: String,
    pub rows: i64,
    pub content: ContentHash,
}
#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(name = "normalization_coverage")]
pub struct NormalizationCoverage {
    #[model(key)]
    pub computation: Id<NormalizationComputation>,
    #[model(key)]
    pub scope: Id<CoverageScope>,
    #[model(key)]
    pub context: Id<AnalysisContext>,
    pub availability: EvidenceAvailability,
}
#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(name = "normalization_coverage_premises")]
pub struct NormalizationPremise {
    #[model(key)]
    pub outcome: Id<NormalizationCoverage>,
    #[model(key)]
    pub premise: Id<NormalizationEvidenceSet>,
}
#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(name = "normalization_evidence_sets")]
pub struct NormalizationEvidenceSet {
    #[model(key)]
    pub input: Id<input::InputRevision>,
    #[model(key)]
    pub context: Id<AnalysisContext>,
    #[model(key)]
    pub family: FactFamily,
}
#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(name = "normalization_evidence_members")]
pub struct NormalizationEvidenceMember {
    #[model(key)]
    pub premise: Id<NormalizationEvidenceSet>,
    #[model(key)]
    pub coverage: Id<ProviderCoverage>,
}
pub fn relations() -> Vec<Relation> {
    vec![
        Relation::of::<NormalizationComputation>(),
        Relation::of::<NormalizationOutputReceipt>(),
        Relation::of::<NormalizationCoverage>(),
        Relation::of::<NormalizationPremise>(),
        Relation::of::<NormalizationEvidenceSet>(),
        Relation::of::<NormalizationEvidenceMember>(),
    ]
}
pub struct CoverageOutput {
    pub computations: Rows<NormalizationComputation>,
    pub receipts: Rows<NormalizationOutputReceipt>,
    pub outcomes: Rows<NormalizationCoverage>,
    pub premises: Rows<NormalizationPremise>,
    pub sets: Rows<NormalizationEvidenceSet>,
    pub members: Rows<NormalizationEvidenceMember>,
}
impl CoverageOutput {
    pub fn new(budget: &resources::ResourceBudget) -> Self {
        Self {
            computations: Rows::new(budget),
            receipts: Rows::new(budget),
            outcomes: Rows::new(budget),
            premises: Rows::new(budget),
            sets: Rows::new(budget),
            members: Rows::new(budget),
        }
    }
    pub fn visit(
        &mut self,
        name: &str,
        batch: &arrow_array::RecordBatch,
    ) -> Result<bool, ModelError> {
        if name == NormalizationComputation::NAME {
            self.computations.decode(batch)?;
        } else if name == NormalizationOutputReceipt::NAME {
            self.receipts.decode(batch)?;
        } else if name == NormalizationCoverage::NAME {
            self.outcomes.decode(batch)?;
        } else if name == NormalizationPremise::NAME {
            self.premises.decode(batch)?;
        } else if name == NormalizationEvidenceSet::NAME {
            self.sets.decode(batch)?;
        } else if name == NormalizationEvidenceMember::NAME {
            self.members.decode(batch)?;
        } else {
            return Ok(false);
        }
        Ok(true)
    }
    pub fn validation_inputs() -> Vec<ValidationInput> {
        vec![
            ValidationInput::of::<NormalizationComputation>(&["id"]),
            ValidationInput::of::<NormalizationOutputReceipt>(&["id"]),
            ValidationInput::of::<NormalizationCoverage>(&["id"]),
            ValidationInput::of::<NormalizationPremise>(&["id"]),
            ValidationInput::of::<NormalizationEvidenceSet>(&["id"]),
            ValidationInput::of::<NormalizationEvidenceMember>(&["id"]),
        ]
    }
}
fn invalid(message: &str) -> ModelError {
    ModelError::Frontier(message.into())
}
fn input_of(
    scope: &CoverageScope,
    artifacts: &Rows<SourceArtifact>,
) -> Option<Id<input::InputRevision>> {
    match scope {
        CoverageScope::Input { input } => Some(*input),
        CoverageScope::Artifact { artifact } => artifacts.get(*artifact).map(|a| a.input),
        _ => None,
    }
}
/// The same operation is available to completed-stage consumers before the final writer.
/// Facts admission established the exact captured scope universe in ScopedAvailability.
pub struct ScopedOutcome {
    pub availability: EvidenceAvailability,
    pub premises: Vec<NormalizationEvidenceSet>,
    _reservation: Box<dyn resources::Reservation>,
}
type ScopeKey = (Id<AnalysisContext>, Id<CoverageScope>);
type FamilyKey = (Id<AnalysisContext>, Id<input::InputRevision>, FactFamily);
struct EvidenceIndex<'a> {
    by_scope: charged::ChargedMap<ScopeKey, Vec<&'a CoverageEvidence>>,
    families: charged::ChargedMap<FamilyKey, bool>,
    _charge: charged::StateCharge,
}
impl<'a> EvidenceIndex<'a> {
    fn new(
        evidence: &'a ScopedAvailability,
        artifacts: &Rows<SourceArtifact>,
        budget: &resources::ResourceBudget,
    ) -> Result<Self, ModelError> {
        let mut index = Self {
            by_scope: Default::default(),
            families: Default::default(),
            _charge: charged::StateCharge::new(budget, "normalized-coverage-index"),
        };
        for row in evidence.evidence() {
            let scope = evidence
                .scope(row.scope)
                .ok_or_else(|| invalid("coverage scope absent"))?;
            let input =
                input_of(scope, artifacts).ok_or_else(|| invalid("coverage input absent"))?;
            index
                .by_scope
                .update(&mut index._charge, (row.context, row.scope), |v| {
                    v.push(row)
                })?;
            let key = (row.context, input, row.family);
            let complete = index.families.get(&key).copied().unwrap_or(true)
                && row.availability == Availability::Complete;
            index.families.insert(&mut index._charge, key, complete)?;
        }
        Ok(index)
    }
    #[allow(
        clippy::too_many_arguments,
        reason = "Scoped premise selection carries the capability, nominal scope/context and admitted source owners"
    )]
    fn outcome(
        &self,
        capability: Capability,
        scope: Id<CoverageScope>,
        context: Id<AnalysisContext>,
        evidence: &ScopedAvailability,
        artifacts: &Rows<SourceArtifact>,
        budget: &resources::ResourceBudget,
    ) -> Result<ScopedOutcome, ModelError> {
        let target = evidence
            .scope(scope)
            .ok_or_else(|| invalid("normalization scope absent"))?;
        let input = input_of(target, artifacts)
            .ok_or_else(|| invalid("normalization scope has no captured input"))?;
        let local = self
            .by_scope
            .get(&(context, scope))
            .map(Vec::as_slice)
            .unwrap_or(&[]);
        let reservation = budget.reserve(
            "normalization-scoped-premises",
            local.len().saturating_mul(8) + capability.families().len().saturating_mul(128),
        )?;
        let premises: Vec<_> = capability
            .families()
            .iter()
            .filter(|f| self.families.contains_key(&(context, input, **f)))
            .map(|family| NormalizationEvidenceSet {
                input,
                context,
                family: *family,
            })
            .collect();
        let anchor: Vec<_> = local
            .iter()
            .filter(|r| r.family == capability.anchor())
            .collect();
        if anchor.is_empty() {
            return Err(invalid("normalization anchor coverage absent"));
        }
        let status = if anchor
            .iter()
            .all(|r| r.availability == Availability::NotRequested)
        {
            EvidenceAvailability::NotRequested
        } else if anchor
            .iter()
            .all(|r| r.availability == Availability::Unavailable)
        {
            EvidenceAvailability::Unavailable
        } else if capability
            .families()
            .iter()
            .all(|family| self.families.get(&(context, input, *family)) == Some(&true))
        {
            EvidenceAvailability::Complete
        } else {
            EvidenceAvailability::Partial
        };
        Ok(ScopedOutcome {
            availability: status,
            premises,
            _reservation: reservation,
        })
    }
}
pub fn scoped_outcome(
    capability: Capability,
    scope: Id<CoverageScope>,
    context: Id<AnalysisContext>,
    evidence: &ScopedAvailability,
    artifacts: &Rows<SourceArtifact>,
    budget: &resources::ResourceBudget,
) -> Result<ScopedOutcome, ModelError> {
    EvidenceIndex::new(evidence, artifacts, budget)?
        .outcome(capability, scope, context, evidence, artifacts, budget)
}
pub fn assemble(
    profile: Profile,
    evidence: &ScopedAvailability,
    artifacts: &Rows<SourceArtifact>,
    sources: &[CompletedRelation],
    budget: &resources::ResourceBudget,
) -> Result<CoverageOutput, ModelError> {
    let _temporary = budget.reserve(
        "normalization-coverage-index",
        evidence.evidence().len().saturating_mul(256),
    )?;
    let index = EvidenceIndex::new(evidence, artifacts, budget)?;
    let mut out = CoverageOutput::new(budget);
    for row in evidence.evidence() {
        let input = input_of(
            evidence
                .scope(row.scope)
                .ok_or_else(|| invalid("evidence scope absent"))?,
            artifacts,
        )
        .ok_or_else(|| invalid("evidence input absent"))?;
        let premise = out.sets.insert(NormalizationEvidenceSet {
            input,
            context: row.context,
            family: row.family,
        })?;
        out.members.insert(NormalizationEvidenceMember {
            premise,
            coverage: row.coverage,
        })?;
    }
    for capability in Capability::ALL {
        let stage = capability.producer(profile);
        let mut computation = NormalizationComputation {
            capability,
            policy: policy_revision(),
            producer: stage.name.into(),
            declaration: stage.digest(),
            profile: profile.name().into(),
            availability: EvidenceAvailability::NoScope,
        };
        // Keys are anchored by coverage admission, including wholly unavailable/empty observations.
        let mut scopes = BTreeMap::new();
        for row in evidence
            .evidence()
            .iter()
            .filter(|r| r.family == capability.anchor())
        {
            scopes.insert((row.scope, row.context), ());
        }
        let mut statuses = Vec::new();
        for ((scope, context), ()) in scopes {
            let scoped = index.outcome(capability, scope, context, evidence, artifacts, budget)?;
            let availability = scoped.availability;
            statuses.push(availability);
            let outcome = out.outcomes.insert(NormalizationCoverage {
                computation: computation.id(),
                scope,
                context,
                availability,
            })?;
            for premise in &scoped.premises {
                out.premises.insert(NormalizationPremise {
                    outcome,
                    premise: premise.id(),
                })?;
            }
        }
        computation.availability = if statuses.is_empty() {
            if matches!(capability, Capability::FlowLinks | Capability::FlowEvents)
                && profile == Profile::Catalog
            {
                EvidenceAvailability::NotRequested
            } else {
                EvidenceAvailability::NoScope
            }
        } else if statuses.iter().all(|s| *s == statuses[0]) {
            statuses[0]
        } else {
            EvidenceAvailability::Partial
        };
        let computation = out.computations.insert(computation)?;
        for relation in &stage.outputs {
            let source = sources
                .iter()
                .find(|s| s.relation() == relation.name() && s.producer() == stage.name)
                .ok_or_else(|| invalid("normalization output has no completed producer receipt"))?;
            let receipt = source.receipt();
            out.receipts.insert(NormalizationOutputReceipt {
                computation,
                relation: relation.name().into(),
                rows: i64::try_from(receipt.rows)
                    .map_err(|_| invalid("normalization receipt row overflow"))?,
                content: receipt.content,
            })?;
        }
    }
    Ok(out)
}
pub fn validate(expected: &CoverageOutput, stored: &CoverageOutput) -> Result<(), ModelError> {
    if expected.computations.same(&stored.computations)
        && expected.receipts.same(&stored.receipts)
        && expected.outcomes.same(&stored.outcomes)
        && expected.premises.same(&stored.premises)
        && expected.sets.same(&stored.sets)
        && expected.members.same(&stored.members)
    {
        Ok(())
    } else {
        Err(invalid(
            "normalized scoped outcomes or frozen receipt premises differ",
        ))
    }
}
/// One final writer consumes frozen completion receipts and the admitted lower coverage.
pub fn stage(profile: Profile) -> Stage {
    let mut inputs = Vec::new();
    for capability in Capability::ALL {
        let producer = capability.producer(profile);
        inputs.extend(producer.inputs);
        for output in producer.outputs {
            inputs.push(output.completed_store());
        }
    }
    inputs.push(RelationUse::stored::<SourceArtifact>());
    inputs.sort_by_key(|r| r.name());
    inputs.dedup_by_key(|r| r.name());
    Stage {
        name: "normalize_coverage",
        inputs: super::facts_stage_inputs(inputs),
        outputs: relations().iter().map(RelationUse::of_relation).collect(),
        contributes: vec![],
        coverage: vec![],
        profiles: vec![profile],
        effect: Effect::Pure,
        code: policy_revision(),
        configuration: ContentHash::of(b"normalization-coverage/v1"),
    }
}
