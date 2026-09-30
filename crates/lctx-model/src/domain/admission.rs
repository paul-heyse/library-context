//! The facts frontier and its admission (DESIGN §15.8, §15.11; ADR-0087).
//!
//! A facts generation publishes only facts relations. For every family its profile requests, it
//! states the coverage of every scope the family is stated over; for every other family it states
//! `NotRequested`. Preflight refuses a schedule that cannot produce that frontier before any store
//! effect. Admission reconciles the sealed coverage rows with the stage outcomes; it is the only
//! way to obtain a [`FrontierAdmission`].
use super::{
    attribution::*,
    charged::{ChargedMap, ChargedSet, ChargedVec, StateCharge},
    input::{
        ArtifactOwnership, ArtifactUse, DerivedArtifact, InputRevision, SourceRole, UnownedArtifact,
    },
    record::FieldValue,
    resources::ResourceBudget,
    source::{CoverageScope, SourceArtifact},
    stages::{ExecutionReceipt, Profile, ProviderOutcome, Schedule},
    *,
};
use std::collections::{BTreeMap, BTreeSet};
mod availability;
pub use availability::{CoverageEvidence, ScopedAvailability};

/// What a generation can answer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Frontier {
    Conformance,
    Facts,
    Normalized,
}
impl Frontier {
    pub const ALL: [Self; 3] = [Self::Conformance, Self::Facts, Self::Normalized];
    pub fn name(self) -> &'static str {
        match self {
            Self::Conformance => "conformance",
            Self::Facts => "facts",
            Self::Normalized => "normalized",
        }
    }
    /// The model is the sole owner of frontier closure and admission semantics. A frontier is
    /// added here only when its complete producer and validator envelope is implemented.
    pub fn descriptor(self) -> FrontierDescriptor {
        match self {
            Self::Conformance => FrontierDescriptor {
                frontier: self,
                declared: None,
                requirements: &[],
                checkpoints: &[],
                selectable: false,
                admission: false,
            },
            Self::Facts => FrontierDescriptor {
                frontier: self,
                declared: Some(facts_relations),
                requirements: FACTS_REQUIREMENTS,
                checkpoints: &[],
                selectable: true,
                admission: true,
            },
            Self::Normalized => FrontierDescriptor {
                frontier: self,
                declared: Some(normalized_relations),
                requirements: FACTS_REQUIREMENTS,
                checkpoints: &[Self::Facts],
                selectable: true,
                admission: true,
            },
        }
    }
}

/// A closed model-owned frontier declaration (ADR-0101). Physical stores consume this contract;
/// they do not decide which semantic layer a relation, validator or admission belongs to.
#[derive(Clone, Copy)]
pub struct FrontierDescriptor {
    frontier: Frontier,
    declared: Option<fn() -> Vec<Relation>>,
    requirements: &'static [FamilyRequirement],
    checkpoints: &'static [Frontier],
    selectable: bool,
    admission: bool,
}
impl FrontierDescriptor {
    pub fn frontier(self) -> Frontier {
        self.frontier
    }
    pub fn selectable(self) -> bool {
        self.selectable
    }
    pub fn requires_admission(self) -> bool {
        self.admission
    }
    pub fn checkpoints(self) -> &'static [Frontier] {
        self.checkpoints
    }
    fn relations_from_declaration(self) -> Result<BTreeSet<&'static str>, ModelError> {
        let declare = self
            .declared
            .ok_or_else(|| refuse("conformance cannot be a product checkpoint"))?;
        Ok(declare().iter().map(Relation::name).collect())
    }
    pub fn requirements(self) -> &'static [FamilyRequirement] {
        self.requirements
    }
    /// Refuse an incomplete model or a reference above the declared frontier. Conformance alone
    /// takes the caller's entire validated model, including deliberately small fixture models.
    pub fn relations(self, model: &ValidatedModel) -> Result<BTreeSet<&'static str>, ModelError> {
        let relations: BTreeSet<_> = match self.declared {
            Some(declare) => declare().iter().map(Relation::name).collect(),
            None => model.relations().iter().map(Relation::name).collect(),
        };
        for name in &relations {
            let relation = model
                .relations()
                .iter()
                .find(|r| r.name() == *name)
                .ok_or_else(|| {
                    refuse(format!(
                        "the model lacks {} relation {name}",
                        self.frontier.name()
                    ))
                })?;
            for field in relation.fields() {
                if let Some((_, target)) = field.target()
                    && !relations.contains(target)
                {
                    return Err(refuse(format!(
                        "{name}.{} references {target}, above the {} frontier",
                        field.name(),
                        self.frontier.name()
                    )));
                }
            }
        }
        Ok(relations)
    }
    pub fn invariants(self, model: &ValidatedModel) -> Result<Vec<&Invariant>, ModelError> {
        let relations = self.relations(model)?;
        Ok(model
            .invariants()
            .iter()
            .filter(|i| {
                i.inputs
                    .iter()
                    .all(|input| relations.contains(input.name()))
            })
            .collect())
    }
}
/// The captured artifacts a family is stated over.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ArtifactClass {
    PythonSource,
    Document,
}
impl ArtifactClass {
    /// The one classification of captured artifacts for coverage; producers select their inputs by it.
    pub fn of(path: &str) -> Option<Self> {
        match path.rsplit_once('.').map(|(_, extension)| extension) {
            Some("py" | "pyi") => Some(Self::PythonSource),
            Some("md" | "mdx" | "rst") => Some(Self::Document),
            _ => None,
        }
    }
}
/// Select the analyzed artifact universe from declared uses. Captured dependency files remain
/// supporting lookup context. A use is authoritative; a filename alone never requests analysis.
/// This operation is shared by producers and exact coverage admission.
pub fn analysis_roots(
    artifacts: &[SourceArtifact],
    uses: &[ArtifactUse],
) -> Result<BTreeSet<Id<SourceArtifact>>, ModelError> {
    let by_id: BTreeMap<_, _> = artifacts.iter().map(|a| (a.id(), a)).collect();
    let mut selected = BTreeSet::new();
    for usage in uses {
        let artifact = by_id
            .get(&usage.artifact)
            .ok_or_else(|| refuse("an artifact use names an absent artifact"))?;
        let requested = match ArtifactClass::of(&artifact.path) {
            Some(ArtifactClass::PythonSource) => matches!(
                usage.role,
                SourceRole::Release | SourceRole::Example | SourceRole::Test | SourceRole::DocBlock
            ),
            Some(ArtifactClass::Document) => usage.role == SourceRole::Document,
            None => false,
        };
        if requested {
            selected.insert(artifact.id());
        }
    }
    Ok(selected)
}
/// The scope a family's coverage is stated over: each input, or each input artifact of a class.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Grain {
    Input,
    Artifact(ArtifactClass),
}
/// A family of the facts frontier: its grain, the profiles that request it, and whether facts
/// are admissible when it is entirely unavailable.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FamilyRequirement {
    pub family: FactFamily,
    pub grains: &'static [Grain],
    pub requested_in: &'static [Profile],
    pub required: bool,
}

const BOTH: &[Profile] = &[Profile::Catalog, Profile::Behavioral];
const PYTHON: Grain = Grain::Artifact(ArtifactClass::PythonSource);
/// The facts frontier, one row per family.
pub const FACTS_REQUIREMENTS: &[FamilyRequirement] = &[
    FamilyRequirement {
        family: FactFamily::Artifacts,
        grains: &[Grain::Input],
        requested_in: BOTH,
        required: true,
    },
    FamilyRequirement {
        family: FactFamily::Syntax,
        grains: &[PYTHON],
        requested_in: BOTH,
        required: true,
    },
    FamilyRequirement {
        family: FactFamily::Lexical,
        grains: &[PYTHON],
        requested_in: BOTH,
        required: false,
    },
    FamilyRequirement {
        family: FactFamily::Signatures,
        grains: &[PYTHON, Grain::Input],
        requested_in: BOTH,
        required: false,
    },
    FamilyRequirement {
        family: FactFamily::Calls,
        grains: &[PYTHON],
        requested_in: BOTH,
        required: false,
    },
    FamilyRequirement {
        family: FactFamily::Types,
        grains: &[PYTHON],
        requested_in: BOTH,
        required: false,
    },
    FamilyRequirement {
        family: FactFamily::Exports,
        grains: &[PYTHON],
        requested_in: BOTH,
        required: false,
    },
    FamilyRequirement {
        family: FactFamily::Docs,
        grains: &[Grain::Artifact(ArtifactClass::Document)],
        requested_in: BOTH,
        required: false,
    },
    FamilyRequirement {
        family: FactFamily::Deployment,
        grains: &[Grain::Input],
        requested_in: BOTH,
        required: false,
    },
    FamilyRequirement {
        family: FactFamily::Flow,
        grains: &[PYTHON],
        requested_in: &[Profile::Behavioral],
        required: false,
    },
];
fn refuse(message: impl Into<String>) -> ModelError {
    ModelError::Frontier(message.into())
}

/// The facts frontier of one profile over one model.
#[derive(Debug, Clone)]
pub struct FrontierContract {
    frontier: Frontier,
    profile: Profile,
    model: ContentHash,
    relations: BTreeSet<&'static str>,
    requirements: Vec<FamilyRequirement>,
    digest: ContentHash,
    /// The family whose coverage states each assertion relation's completeness.
    families: BTreeMap<&'static str, FactFamily>,
}
impl FrontierContract {
    /// Facts relations may reference only facts relations: nothing at the facts frontier depends
    /// on a relation a later layer derives.
    pub fn facts(model: &ValidatedModel, profile: Profile) -> Result<Self, ModelError> {
        Self::for_frontier(model, profile, Frontier::Facts)
    }
    pub fn for_frontier(
        model: &ValidatedModel,
        profile: Profile,
        frontier: Frontier,
    ) -> Result<Self, ModelError> {
        let descriptor = frontier.descriptor();
        if !descriptor.requires_admission() {
            return Err(refuse("conformance has no product admission contract"));
        }
        let relations = descriptor.relations(model)?;
        let mut families = BTreeMap::new();
        for name in &relations {
            let relation = model
                .relations()
                .iter()
                .find(|r| r.name() == *name)
                .ok_or_else(|| refuse(format!("the model lacks facts relation {name}")))?;
            if let Some(family) = relation.family() {
                families.insert(*name, family);
            }
        }
        let requirements = descriptor.requirements();
        let stated: BTreeSet<_> = requirements.iter().map(|r| r.family as i16).collect();
        if stated.len() != requirements.len()
            || stated.len() != <FactFamily as FieldValue>::codes().len()
        {
            return Err(refuse(
                "the facts frontier states every fact family exactly once",
            ));
        }
        let mut digest = KeySink::new("frontier-contract");
        digest.part(b"frontier", frontier.name().as_bytes());
        digest.part(b"selectable", &[u8::from(descriptor.selectable())]);
        for checkpoint in descriptor.checkpoints() {
            digest.part(b"checkpoint", checkpoint.name().as_bytes());
        }
        digest.part(b"profile", profile.name().as_bytes());
        digest.part(b"model", &model.digest().0);
        for name in &relations {
            digest.part(b"relation", name.as_bytes());
        }
        for requirement in requirements {
            Key::encode(&requirement.family, &mut digest);
            for grain in requirement.grains {
                let grain = match grain {
                    Grain::Input => "input",
                    Grain::Artifact(ArtifactClass::PythonSource) => "python",
                    Grain::Artifact(ArtifactClass::Document) => "document",
                };
                digest.part(b"grain", grain.as_bytes());
            }
            digest.part(
                b"requested",
                &[u8::from(requirement.requested_in.contains(&profile))],
            );
            digest.part(b"required", &[u8::from(requirement.required)]);
        }
        Ok(Self {
            frontier,
            profile,
            model: model.digest(),
            relations,
            requirements: requirements.to_vec(),
            digest: digest.finish(),
            families,
        })
    }
    pub fn frontier(&self) -> Frontier {
        self.frontier
    }
    pub fn profile(&self) -> Profile {
        self.profile
    }
    pub fn model(&self) -> ContentHash {
        self.model
    }
    pub fn digest(&self) -> ContentHash {
        self.digest
    }
    pub fn requirements(&self) -> &[FamilyRequirement] {
        &self.requirements
    }
    pub fn contains(&self, relation: &str) -> bool {
        self.relations.contains(relation)
    }
    pub fn requested(&self, family: FactFamily) -> bool {
        self.requirements
            .iter()
            .any(|r| r.family == family && r.requested_in.contains(&self.profile))
    }
    /// Refuse, before any store effect, a schedule that cannot produce this frontier: one that
    /// reads or writes above it, attempts a family its profile does not request, leaves a
    /// requested family uncovered, writes a family's assertions from a stage that does not report
    /// that family's coverage, or has no coverage writer.
    pub fn preflight(&self, schedule: &Schedule) -> Result<Preflight, ModelError> {
        self.preflight_scope(schedule, false)
    }
    /// Admission for a frozen prefix of a cumulative schedule. A stage cannot straddle the
    /// checkpoint boundary; later writers are excluded, while the full schedule digest is bound.
    pub fn checkpoint_preflight(&self, schedule: &Schedule) -> Result<Preflight, ModelError> {
        self.preflight_scope(schedule, true)
    }
    fn preflight_scope(
        &self,
        schedule: &Schedule,
        checkpoint: bool,
    ) -> Result<Preflight, ModelError> {
        if schedule.model() != self.model || schedule.profile() != self.profile {
            return Err(refuse(
                "the schedule's model or profile differs from the frontier contract",
            ));
        }
        let mut coverers: BTreeMap<FactFamily, BTreeSet<Id<Provider>>> = BTreeMap::new();
        let mut stages = BTreeMap::new();
        let mut written = BTreeSet::new();
        for stage in schedule.stages() {
            if checkpoint && stage.outputs.iter().all(|r| !self.contains(r.name())) {
                continue;
            }
            for relation in stage
                .inputs
                .iter()
                .chain(&stage.outputs)
                .chain(&stage.contributes)
            {
                if !self.contains(relation.name()) {
                    return Err(refuse(format!(
                        "stage {} uses {}, above the facts frontier",
                        stage.name,
                        relation.name()
                    )));
                }
            }
            written.extend(stage.outputs.iter().map(|r| r.name()));
            let Some(provider) = stage.provider else {
                continue;
            };
            for family in &stage.coverage {
                if !self.requested(*family) {
                    return Err(refuse(format!(
                        "stage {} attempts {family:?}, which the {} profile does not request",
                        stage.name,
                        self.profile.name()
                    )));
                }
                coverers.entry(*family).or_default().insert(provider);
            }
            stages.insert(
                stage.name,
                (
                    provider,
                    stage.coverage.iter().copied().collect::<BTreeSet<_>>(),
                ),
            );
        }
        for requirement in self
            .requirements
            .iter()
            .filter(|r| r.requested_in.contains(&self.profile))
        {
            if !coverers.contains_key(&requirement.family) {
                return Err(refuse(format!(
                    "no scheduled stage covers requested {:?}",
                    requirement.family
                )));
            }
        }
        // Coverage states the completeness of a family's assertions only when the stage that
        // reports it also writes them; an unrequested family's assertions are never written.
        for (relation, family) in &self.families {
            let writer = schedule.stages().iter().find(|stage| {
                stage
                    .outputs
                    .iter()
                    .any(|output| output.name() == *relation)
            });
            match (self.requested(*family), writer) {
                (true, Some(stage)) if stage.coverage.contains(family) => {}
                (true, _) => {
                    return Err(refuse(format!(
                        "{relation} is not written by a stage that reports {family:?} coverage"
                    )));
                }
                (false, Some(stage)) => {
                    return Err(refuse(format!(
                        "stage {} writes {relation} of unrequested {family:?}",
                        stage.name
                    )));
                }
                (false, None) => {}
            }
        }
        for relation in [ProviderCoverage::NAME, CoverageScope::NAME] {
            if !written.contains(relation) {
                return Err(refuse(format!("no scheduled stage writes {relation}")));
            }
        }
        // Every relation above the checkpoint closure needs a scheduled writer, including
        // vocabulary, total assessments and graph snapshot chunks that may be explicitly empty.
        for checkpoint in self.frontier.descriptor().checkpoints() {
            let lower = checkpoint.descriptor().relations_from_declaration()?;
            for relation in self.relations.difference(&lower) {
                if !written.contains(relation) {
                    return Err(refuse(format!(
                        "no scheduled stage writes normalized relation {relation}"
                    )));
                }
            }
        }
        if self.frontier == Frontier::Normalized {
            for capability in super::normalized::coverage::Capability::ALL {
                let required = capability.producer(self.profile);
                if !schedule
                    .stages()
                    .iter()
                    .any(|s| s.name == required.name && s.digest() == required.digest())
                {
                    return Err(refuse("normalized capability producer declaration differs"));
                }
            }
        }
        Ok(Preflight {
            contract: self.clone(),
            schedule: schedule.digest(),
            coverers,
            stages,
        })
    }
}

/// A schedule admitted to a frontier, with the provider covering each requested family.
#[derive(Debug, Clone)]
pub struct Preflight {
    contract: FrontierContract,
    schedule: ContentHash,
    coverers: BTreeMap<FactFamily, BTreeSet<Id<Provider>>>,
    stages: BTreeMap<&'static str, (Id<Provider>, BTreeSet<FactFamily>)>,
}
/// One coverage row a facts generation must state: a requested family names its provider; an
/// unrequested one is a single `NotRequested` row.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Expected {
    pub scope: Id<CoverageScope>,
    pub family: FactFamily,
    pub provider: Option<Id<Provider>>,
}
impl Preflight {
    pub fn contract(&self) -> &FrontierContract {
        &self.contract
    }
    pub fn schedule(&self) -> ContentHash {
        self.schedule
    }
    /// The exact coverage rows over these inputs and their captured artifacts.
    pub fn expected_coverage(
        &self,
        inputs: &[InputRevision],
        artifacts: &[SourceArtifact],
        uses: &[ArtifactUse],
    ) -> Result<BTreeMap<Expected, CoverageScope>, ModelError> {
        let owned: BTreeSet<_> = inputs.iter().map(Record::id).collect();
        if let Some(artifact) = artifacts.iter().find(|a| !owned.contains(&a.input)) {
            return Err(refuse(format!(
                "artifact {} belongs to no admitted input",
                artifact.path
            )));
        }
        let mut expected = BTreeMap::new();
        let selected = analysis_roots(artifacts, uses)?;
        for requirement in &self.contract.requirements {
            let scopes: Vec<CoverageScope> = requirement
                .grains
                .iter()
                .flat_map(|grain| match grain {
                    Grain::Input => inputs
                        .iter()
                        .map(|input| CoverageScope::Input { input: input.id() })
                        .collect::<Vec<_>>(),
                    Grain::Artifact(class) => artifacts
                        .iter()
                        .filter(|a| {
                            ArtifactClass::of(&a.path) == Some(*class) && selected.contains(&a.id())
                        })
                        .map(|artifact| CoverageScope::Artifact {
                            artifact: artifact.id(),
                        })
                        .collect::<Vec<_>>(),
                })
                .collect();
            let providers: Vec<Option<Id<Provider>>> = match self.coverers.get(&requirement.family)
            {
                Some(providers) if self.contract.requested(requirement.family) => {
                    providers.iter().copied().map(Some).collect()
                }
                _ => vec![None],
            };
            for scope in scopes {
                for provider in &providers {
                    expected.insert(
                        Expected {
                            scope: scope.id(),
                            family: requirement.family,
                            provider: *provider,
                        },
                        scope.clone(),
                    );
                }
            }
        }
        Ok(expected)
    }
}

/// A stage's reported outcome must agree with the coverage its provider stated.
pub fn reconcile(outcome: ProviderOutcome, rows: &[&ProviderCoverage]) -> Result<(), ModelError> {
    use CoverageStatus::*;
    let agrees = match outcome {
        ProviderOutcome::Complete => rows
            .iter()
            .all(|row| row.status == CompleteUnderStatedModel),
        ProviderOutcome::Partial => {
            rows.iter()
                .all(|row| matches!(row.status, CompleteUnderStatedModel | Partial | Unavailable))
                && rows
                    .iter()
                    .any(|row| row.status != CompleteUnderStatedModel)
        }
        ProviderOutcome::Unavailable => {
            !rows.is_empty() && rows.iter().all(|row| row.status == Unavailable)
        }
        ProviderOutcome::Failed | ProviderOutcome::NotRequested => false,
    };
    if agrees {
        Ok(())
    } else {
        Err(refuse(format!(
            "a stage reported {outcome:?} against coverage it does not state"
        )))
    }
}

/// A family's availability in an admitted generation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Availability {
    Complete,
    Partial,
    Unavailable,
    NotRequested,
    /// Requested, but the inputs have no scope of its grain.
    NoScope,
}
impl Availability {
    pub const ALL: [Self; 5] = [
        Self::Complete,
        Self::Partial,
        Self::Unavailable,
        Self::NotRequested,
        Self::NoScope,
    ];
    /// The stored code. Append-only: a new availability takes the next code; none is renumbered.
    pub fn code(self) -> i16 {
        match self {
            Self::Complete => 0,
            Self::Partial => 1,
            Self::Unavailable => 2,
            Self::NotRequested => 3,
            Self::NoScope => 4,
        }
    }
    pub fn from_code(code: i16) -> Option<Self> {
        Self::ALL.into_iter().find(|a| a.code() == code)
    }
}
/// Proof that a sealed generation's coverage and stage outcomes meet its facts frontier. Only
/// [`AdmissionCheck::finish`] constructs one.
///
/// ```compile_fail
/// use lctx_model::domain::admission::FrontierAdmission;
/// let forged = FrontierAdmission { contract: todo!(), model: todo!(), schedule: todo!(), coverage: todo!(),
///     content: todo!(), profile: todo!(), availability: todo!() };
/// ```
///
/// ```
/// use lctx_model::domain::{ContentHash, admission::FrontierAdmission};
/// fn recorded(admission: &FrontierAdmission) -> (ContentHash, ContentHash) { (admission.contract(), admission.coverage()) }
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FrontierAdmission {
    frontier: Frontier,
    contract: ContentHash,
    model: ContentHash,
    schedule: ContentHash,
    coverage: ContentHash,
    content: ContentHash,
    profile: Profile,
    availability: BTreeMap<FactFamily, Availability>,
    scoped: std::sync::Arc<ScopedAvailability>,
}
impl FrontierAdmission {
    pub fn frontier(&self) -> Frontier {
        self.frontier
    }
    pub fn contract(&self) -> ContentHash {
        self.contract
    }
    pub fn model(&self) -> ContentHash {
        self.model
    }
    pub fn schedule(&self) -> ContentHash {
        self.schedule
    }
    pub fn coverage(&self) -> ContentHash {
        self.coverage
    }
    pub fn content(&self) -> ContentHash {
        self.content
    }
    pub fn profile(&self) -> Profile {
        self.profile
    }
    pub fn availability(&self) -> &BTreeMap<FactFamily, Availability> {
        &self.availability
    }
    pub fn scoped(&self) -> &std::sync::Arc<ScopedAvailability> {
        &self.scoped
    }
}

/// Reads a sealed generation's inputs, artifacts, their classes and coverage, charged to the
/// attempt budget.
pub struct AdmissionCheck {
    preflight: Preflight,
    charge: StateCharge,
    inputs: ChargedVec<InputRevision>,
    artifacts: ChargedVec<SourceArtifact>,
    uses: ChargedVec<ArtifactUse>,
    classified: ChargedSet<Id<SourceArtifact>>,
    scopes: ChargedMap<Id<CoverageScope>, CoverageScope>,
    rows: ChargedVec<ProviderCoverage>,
    normalized: super::normalized::coverage::CoverageOutput,
}
impl AdmissionCheck {
    pub fn new(preflight: Preflight, budget: &ResourceBudget) -> Self {
        Self {
            preflight,
            charge: StateCharge::new(budget, "facts_admission"),
            inputs: ChargedVec::default(),
            artifacts: ChargedVec::default(),
            uses: ChargedVec::default(),
            classified: ChargedSet::default(),
            scopes: ChargedMap::default(),
            rows: ChargedVec::default(),
            normalized: super::normalized::coverage::CoverageOutput::new(budget),
        }
    }
    /// The stored relations admission reads, in visiting order.
    pub fn inputs(&self) -> Vec<ValidationInput> {
        let mut inputs = vec![
            ValidationInput::of::<InputRevision>(&["id"]),
            ValidationInput::of::<SourceArtifact>(&["id"]),
            ValidationInput::of::<ArtifactOwnership>(&["id"]),
            ValidationInput::of::<UnownedArtifact>(&["id"]),
            ValidationInput::of::<DerivedArtifact>(&["id"]),
            ValidationInput::of::<ArtifactUse>(&["id"]),
            ValidationInput::of::<CoverageScope>(&["id"]),
            ValidationInput::of::<ProviderCoverage>(&["id"]),
        ];
        if self.preflight.contract.frontier == Frontier::Normalized {
            inputs.extend(super::normalized::coverage::CoverageOutput::validation_inputs());
        }
        inputs
    }
    pub fn visit(
        &mut self,
        relation: &str,
        batch: &arrow_array::RecordBatch,
    ) -> Result<(), ModelError> {
        let c = &mut self.charge;
        if relation == InputRevision::NAME {
            for row in InputRevision::decode(batch)? {
                self.inputs.push(c, row)?;
            }
        } else if relation == SourceArtifact::NAME {
            for row in SourceArtifact::decode(batch)? {
                self.artifacts.push(c, row)?;
            }
        } else if relation == ArtifactOwnership::NAME {
            for row in ArtifactOwnership::decode(batch)? {
                self.classified.insert(c, row.artifact)?;
            }
        } else if relation == UnownedArtifact::NAME {
            for row in UnownedArtifact::decode(batch)? {
                self.classified.insert(c, row.artifact)?;
            }
        } else if relation == DerivedArtifact::NAME {
            for row in DerivedArtifact::decode(batch)? {
                self.classified.insert(c, row.artifact())?;
            }
        } else if relation == ArtifactUse::NAME {
            for row in ArtifactUse::decode(batch)? {
                self.uses.push(c, row)?;
            }
        } else if relation == CoverageScope::NAME {
            for row in CoverageScope::decode(batch)? {
                self.scopes.insert(c, row.id(), row)?;
            }
        } else if relation == ProviderCoverage::NAME {
            for row in ProviderCoverage::decode(batch)? {
                self.rows.push(c, row)?;
            }
        } else if self.preflight.contract.frontier == Frontier::Normalized
            && self.normalized.visit(relation, batch)?
        {
        } else {
            return Err(ModelError::Invalid(format!(
                "admission does not read {relation}"
            )));
        }
        Ok(())
    }
    /// Admit the sealed generation whose validated content digest is `content`, produced by the
    /// execution `receipt`.
    pub fn finish(
        self,
        receipt: &ExecutionReceipt,
        content: ContentHash,
    ) -> Result<FrontierAdmission, ModelError> {
        let contract = &self.preflight.contract;
        if receipt.model() != contract.model || receipt.schedule() != self.preflight.schedule {
            return Err(refuse(
                "the execution receipt belongs to another model or schedule",
            ));
        }
        // The captured closure is complete only when every artifact has its ownership class.
        if let Some(artifact) = self
            .artifacts
            .iter()
            .find(|a| !self.classified.contains(&a.id()))
        {
            return Err(refuse(format!(
                "captured artifact {} has no ownership class",
                artifact.path
            )));
        }
        let expected =
            self.preflight
                .expected_coverage(&self.inputs, &self.artifacts, &self.uses)?;
        let mut stated: BTreeMap<Expected, &ProviderCoverage> = BTreeMap::new();
        for row in self.rows.iter() {
            row.validate()?;
            if !self.scopes.contains_key(&row.scope) {
                return Err(refuse("a coverage row states an absent scope"));
            }
            let requested = contract.requested(row.family);
            match (requested, row.status) {
                (_, CoverageStatus::Failed) => {
                    return Err(refuse(format!(
                        "{:?} coverage failed; a failed provider aborts the attempt",
                        row.family
                    )));
                }
                (true, CoverageStatus::NotRequested) => {
                    return Err(refuse(format!(
                        "requested {:?} is stated NotRequested",
                        row.family
                    )));
                }
                (false, status) if status != CoverageStatus::NotRequested => {
                    return Err(refuse(format!(
                        "{:?} was attempted although the {} profile does not request it",
                        row.family,
                        contract.profile.name()
                    )));
                }
                _ => {}
            }
            let key = Expected {
                scope: row.scope,
                family: row.family,
                provider: row.provider,
            };
            if !expected.contains_key(&key) {
                return Err(refuse(format!("unexpected {:?} coverage row", row.family)));
            }
            if stated.insert(key, row).is_some() {
                return Err(refuse(format!(
                    "duplicate {:?} coverage for one scope and provider",
                    row.family
                )));
            }
        }
        if let Some(missing) = expected.keys().find(|key| !stated.contains_key(key)) {
            return Err(refuse(format!(
                "missing {:?} coverage for a scope",
                missing.family
            )));
        }
        for (stage, (provider, families)) in &self.preflight.stages {
            let outcome = *receipt
                .outcomes()
                .get(stage)
                .ok_or_else(|| refuse(format!("stage {stage} has no outcome")))?;
            let rows: Vec<_> = stated
                .values()
                .copied()
                .filter(|row| row.provider == Some(*provider) && families.contains(&row.family))
                .collect();
            reconcile(outcome, &rows)?;
        }
        let mut availability = BTreeMap::new();
        for requirement in &contract.requirements {
            let statuses: Vec<_> = stated
                .iter()
                .filter(|(key, _)| key.family == requirement.family)
                .map(|(_, row)| row.status)
                .collect();
            let family = if !contract.requested(requirement.family) {
                Availability::NotRequested
            } else if statuses.is_empty() {
                Availability::NoScope
            } else if statuses
                .iter()
                .all(|s| *s == CoverageStatus::CompleteUnderStatedModel)
            {
                Availability::Complete
            } else if statuses.iter().all(|s| *s == CoverageStatus::Unavailable) {
                Availability::Unavailable
            } else {
                Availability::Partial
            };
            if requirement.required && family == Availability::Unavailable {
                return Err(refuse(format!(
                    "required {:?} is entirely unavailable",
                    requirement.family
                )));
            }
            availability.insert(requirement.family, family);
        }
        let mut coverage = KeySink::new("facts-coverage");
        for row in stated.values() {
            row.id().encode(&mut coverage);
        }
        let scoped = std::sync::Arc::new(ScopedAvailability::validated(
            &self.rows,
            &self.scopes,
            &availability,
            self.charge.budget().expect("bound admission"),
        )?);
        if contract.frontier == Frontier::Normalized {
            let budget = self.charge.budget().expect("bound admission");
            let mut artifacts = super::normalized::Rows::new(budget);
            for row in self.artifacts.iter() {
                artifacts.insert(row.clone())?;
            }
            let expected = super::normalized::coverage::assemble(
                contract.profile,
                &scoped,
                &artifacts,
                receipt.sources(),
                budget,
            )?;
            super::normalized::coverage::validate(&expected, &self.normalized)?;
            for row in self.normalized.outcomes.iter() {
                row.id().encode(&mut coverage);
            }
        }
        Ok(FrontierAdmission {
            frontier: contract.frontier,
            contract: contract.digest,
            model: contract.model,
            schedule: self.preflight.schedule,
            coverage: coverage.finish(),
            content,
            profile: contract.profile,
            scoped,
            availability,
        })
    }
}
