//! Exact immutable compiler products and dependencies. These describe completed state, not
//! runtime grants, execution history or a cross-run scheduler.
use super::{
    ContentHash, Key, KeySink, ModelError, analysis::sources::SourceSnapshot, stages::Profile,
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TypedRowKey {
    pub relation: String,
    pub key: [u8; 16],
}

/// Profile meaning of the completed output. Attempt/publication profile is separate authority.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    content = "profile",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum OutputProfile {
    Neutral,
    Specific(Profile),
}
impl OutputProfile {
    /// Search metadata only; the complete serialized Spec remains authoritative.
    pub fn name(self) -> &'static str {
        match self {
            Self::Neutral => "neutral",
            Self::Specific(Profile::Catalog) => "specific:catalog",
            Self::Specific(Profile::Behavioral) => "specific:behavioral",
        }
    }
    fn encode(self, sink: &mut KeySink) {
        match self {
            Self::Neutral => sink.part(b"output-profile", b"neutral"),
            Self::Specific(profile) => {
                sink.part(b"output-profile", b"specific");
                profile.name().to_string().encode(sink);
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ContributionSpec {
    #[serde(deserialize_with = "super::producer_contract::required_optional")]
    pub captured_binding: Option<super::producer_contract::CapturedProducerBinding>,
    pub producer: String,
    pub profile: OutputProfile,
    pub model: ContentHash,
    pub implementation: ContentHash,
    pub configuration: Option<ContentHash>,
    pub inputs: Vec<SourceSnapshot>,
    pub outputs: BTreeSet<String>,
}
impl ContributionSpec {
    /// Check output profile against the attempt without treating a neutral tag as authority.
    pub fn validate_profile(&self, attempt: Profile) -> Result<(), ModelError> {
        match self.profile {
            OutputProfile::Specific(profile) if profile == attempt => Ok(()),
            OutputProfile::Specific(_) => Err(ModelError::Conflict("contribution attempt profile")),
            OutputProfile::Neutral => self.validate_neutral_profile(),
        }
    }
    fn validate_neutral_profile(&self) -> Result<(), ModelError> {
        use super::producer_contract::{OutputProfilePolicy, facts};
        let contract = facts::facts()
            .into_iter()
            .find(|contract| contract.name == self.producer)
            .ok_or(ModelError::Conflict("unknown neutral producer contract"))?;
        let binding = self.captured_binding.as_ref().ok_or(ModelError::Conflict(
            "neutral contribution needs captured contract",
        ))?;
        if contract.output_profile != OutputProfilePolicy::Neutral
            || binding.contract != contract.identity()
            || binding.semantic_revision != contract.semantic_revision
            || self.outputs
                != contract
                    .outputs
                    .iter()
                    .chain(&contract.contributes)
                    .map(|relation| relation.name().to_owned())
                    .collect()
        {
            return Err(ModelError::Conflict("neutral output contract or inventory"));
        }
        contract.validate_context_dependency(binding, self.profile)
    }
    pub fn identity(&self) -> Result<ContentHash, ModelError> {
        if self.profile == OutputProfile::Neutral {
            self.validate_neutral_profile()?;
        }
        if self.producer.is_empty() {
            return Err(ModelError::Invalid("empty contribution producer".into()));
        }
        let input_ids = self
            .inputs
            .iter()
            .map(SourceSnapshot::identity)
            .collect::<Vec<_>>();
        let mut sink = KeySink::new("compiler-contribution-spec/v3");
        self.captured_binding
            .as_ref()
            .map(|binding| binding.identity())
            .transpose()?
            .encode(&mut sink);
        if self
            .captured_binding
            .as_ref()
            .is_some_and(|binding| self.configuration != Some(binding.producer_settings))
        {
            return Err(ModelError::Conflict("captured producer settings"));
        }
        self.producer.encode(&mut sink);
        self.profile.encode(&mut sink);
        self.model.encode(&mut sink);
        self.implementation.encode(&mut sink);
        self.configuration.encode(&mut sink);
        // Input roles are their declaration positions. Reordering or repeating a selected
        // immutable view is a different dependency binding, even when the set is unchanged.
        sink.part(b"input-roles", &(input_ids.len() as u64).to_le_bytes());
        input_ids.encode(&mut sink);
        for output in &self.outputs {
            output.encode(&mut sink);
        }
        Ok(sink.finish())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CompletedView {
    pub relation: String,
    pub contributions: BTreeSet<ContentHash>,
    pub rows: u64,
    pub identity: ContentHash,
}
impl CompletedView {
    pub fn new(
        relation: String,
        contributions: BTreeSet<ContentHash>,
        rows: u64,
    ) -> Result<Self, ModelError> {
        if relation.is_empty() || contributions.is_empty() {
            return Err(ModelError::Invalid(
                "completed view needs relation and completed contributions".into(),
            ));
        }
        let mut sink = KeySink::new("compiler-completed-view/v1");
        relation.encode(&mut sink);
        for contribution in &contributions {
            contribution.encode(&mut sink);
        }
        // Count is deduplicated metadata, not a substitute for exact membership.
        sink.part(b"rows", &rows.to_le_bytes());
        Ok(Self {
            relation,
            contributions,
            rows,
            identity: sink.finish(),
        })
    }
    pub fn validate(&self) -> Result<(), ModelError> {
        if Self::new(self.relation.clone(), self.contributions.clone(), self.rows)?.identity
            != self.identity
        {
            return Err(ModelError::Conflict("completed view identity"));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OutputContent {
    pub rows: u64,
    pub content: ContentHash,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CompletedContribution {
    pub spec: ContributionSpec,
    /// Required on the current wire. Historical absence never becomes a current
    /// provider interface or a reusable pure producer by deserialization default.
    pub assurance: super::producer_assurance::ProducerAssurance,
    pub outcome: i16,
    pub outputs: BTreeMap<String, OutputContent>,
}
impl CompletedContribution {
    pub fn identity(&self) -> Result<ContentHash, ModelError> {
        if !(0..=4).contains(&self.outcome)
            || self.outcome == 3
            || self.outputs.keys().cloned().collect::<BTreeSet<_>>() != self.spec.outputs
        {
            return Err(ModelError::Invalid(
                "incomplete contribution output inventory/outcome".into(),
            ));
        }
        self.assurance.validate_for(self)?;
        let mut sink = KeySink::new("compiler-completed-contribution/v2");
        self.spec.identity()?.encode(&mut sink);
        self.assurance.identity()?.encode(&mut sink);
        self.outcome.encode(&mut sink);
        for (name, output) in &self.outputs {
            name.encode(&mut sink);
            sink.part(b"rows", &output.rows.to_le_bytes());
            output.content.encode(&mut sink);
        }
        Ok(sink.finish())
    }
}

/// One exact retained dependency. Whole-view dependencies include absence/membership premises.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DependencySelection {
    pub input: SourceSnapshot,
    pub keys: Option<BTreeSet<TypedRowKey>>,
}

pub const STATE_FORMAT_VERSION: u32 = 4;

/// Physical addresses qualify the complete nominal identity and semantic payload with the
/// executable model and codec. Nominal identity alone is never a mutable storage address.
pub fn payload_address(
    model: ContentHash,
    relation: &str,
    nominal: &[u8],
    content: ContentHash,
) -> ContentHash {
    let mut sink = KeySink::new("native-immutable-payload/v1");
    model.encode(&mut sink);
    sink.part(b"codec", b"canonical-native-json/v1");
    relation.to_string().encode(&mut sink);
    sink.part(b"nominal", nominal);
    content.encode(&mut sink);
    sink.finish()
}
pub fn attempt_contribution(attempt: ContentHash, spec: ContentHash) -> ContentHash {
    let mut sink = KeySink::new("native-attempt-contribution/v1");
    attempt.encode(&mut sink);
    spec.encode(&mut sink);
    sink.finish()
}
pub fn attempt_binding(attempt: ContentHash, binding: ContentHash) -> ContentHash {
    let mut sink = KeySink::new("native-attempt-binding/v1");
    attempt.encode(&mut sink);
    binding.encode(&mut sink);
    sink.finish()
}
pub fn binding_inventory_identity(
    bindings: &[CompletedBinding],
) -> Result<ContentHash, ModelError> {
    let mut ordered = bindings.iter().collect::<Vec<_>>();
    ordered.sort_by_key(|binding| binding.key());
    if ordered
        .windows(2)
        .any(|pair| pair[0].key() == pair[1].key())
    {
        return Err(ModelError::Conflict("duplicate publication binding"));
    }
    let mut sink = KeySink::new("native-publication-view/v2");
    for binding in ordered {
        binding.validate()?;
        binding.descriptor_identity()?.encode(&mut sink);
    }
    Ok(sink.finish())
}

/// Native owner retirement decisions are explicit; the storage manager may consume these
/// reasons but cannot decide that authoritative content is disposable by age.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RetirementEligibility {
    pub eligible: bool,
    pub reasons: Vec<String>,
}
/// The first transport record is format authority, never a descriptor or state row.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CompletedStateHeader {
    pub format_version: u32,
}
impl CompletedStateHeader {
    pub fn current() -> Self {
        Self {
            format_version: STATE_FORMAT_VERSION,
        }
    }
    pub fn decode(bytes: &[u8]) -> Result<Self, ModelError> {
        let header: Self = serde_json::from_slice(bytes).map_err(ModelError::codec)?;
        if header.format_version != STATE_FORMAT_VERSION {
            return Err(ModelError::Invalid(
                "unsupported completed-state transport format".into(),
            ));
        }
        Ok(header)
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CompletedStateIdentity {
    pub format_version: u32,
    pub contributions: u64,
    pub memberships: u64,
    pub backing_rows: u64,
    pub content: ContentHash,
}
impl CompletedStateIdentity {
    pub fn from_roots(
        roots: &super::completed_identity::CompletedFamilyRoots,
    ) -> Result<Self, ModelError> {
        let families = roots.roots();
        Ok(Self {
            format_version: STATE_FORMAT_VERSION,
            contributions: families[0].rows,
            memberships: families[1].rows,
            backing_rows: families[3].rows,
            content: roots.identity()?,
        })
    }
    pub fn validate(&self) -> Result<(), ModelError> {
        if self.format_version != STATE_FORMAT_VERSION {
            return Err(ModelError::Invalid(
                "unsupported completed-state format".into(),
            ));
        }
        Ok(())
    }
}

/// Current relation and immutable semantic-boundary bindings needed for exact restore.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CompletedBinding {
    pub boundary: Option<String>,
    pub source: SourceSnapshot,
    pub view: CompletedView,
    pub configuration: Option<ContentHash>,
}
impl CompletedBinding {
    pub fn validate(&self) -> Result<(), ModelError> {
        self.view.validate()?;
        if self.source.relation() != self.view.relation
            || self.source.producer().is_empty()
            || self.source.view() != self.view.identity
            || self.source.rows() < 0
            || self.source.rows() as u64 != self.view.rows
            || self
                .boundary
                .as_ref()
                .is_some_and(|boundary| boundary.is_empty())
        {
            return Err(ModelError::Conflict("completed binding exact view"));
        }
        Ok(())
    }
    pub fn key(&self) -> ContentHash {
        let mut sink = KeySink::new("compiler-binding-key/v1");
        self.source.relation().to_string().encode(&mut sink);
        self.boundary.encode(&mut sink);
        sink.finish()
    }
    /// The mutable attempt slot is separate from the complete retained meaning.
    /// Exact source attribution, boundary role, configuration and the entire validated
    /// completed view participate. A view hash alone cannot replace this descriptor.
    pub fn descriptor_identity(&self) -> Result<ContentHash, ModelError> {
        self.validate()?;
        let mut sink = KeySink::new("compiler-binding-descriptor/v2");
        self.boundary.encode(&mut sink);
        self.source.identity().encode(&mut sink);
        self.view.relation.encode(&mut sink);
        self.view.identity.encode(&mut sink);
        sink.part(b"rows", &self.view.rows.to_le_bytes());
        for contribution in &self.view.contributions {
            contribution.encode(&mut sink);
        }
        self.configuration.encode(&mut sink);
        Ok(sink.finish())
    }
}

/// A bounded native reachability retirement pass; remaining roots can be retried explicitly.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RetirementProgress {
    pub identity: ContentHash,
    pub retired: u64,
    pub examined: u64,
    pub remaining: Vec<String>,
    pub retained: Vec<String>,
}
