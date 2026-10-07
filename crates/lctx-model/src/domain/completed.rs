//! Exact immutable compiler products and dependencies. These describe completed state, not
//! runtime grants, execution history or a cross-run scheduler.
use super::{ContentHash, Key, KeySink, ModelError, analysis::sources::SourceSnapshot, stages::Profile};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TypedRowKey { pub relation: String, pub key: [u8; 16] }

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ContributionSpec {
    pub producer: String,
    pub profile: Profile,
    pub model: ContentHash,
    pub implementation: ContentHash,
    pub configuration: Option<ContentHash>,
    pub inputs: Vec<SourceSnapshot>,
    pub outputs: BTreeSet<String>,
}
impl ContributionSpec {
    pub fn identity(&self) -> Result<ContentHash, ModelError> {
        if self.producer.is_empty() { return Err(ModelError::Invalid("empty contribution producer".into())); }
        let mut input_ids = self.inputs.iter().map(SourceSnapshot::identity).collect::<Vec<_>>();
        input_ids.sort(); input_ids.dedup();
        let mut sink = KeySink::new("compiler-contribution-spec/v1");
        self.producer.encode(&mut sink); self.profile.name().to_string().encode(&mut sink);
        self.model.encode(&mut sink); self.implementation.encode(&mut sink);
        self.configuration.encode(&mut sink); input_ids.encode(&mut sink);
        for output in &self.outputs { output.encode(&mut sink); }
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
    pub fn new(relation: String, contributions: BTreeSet<ContentHash>, rows: u64) -> Result<Self, ModelError> {
        if relation.is_empty() || contributions.is_empty() {
            return Err(ModelError::Invalid("completed view needs relation and completed contributions".into()));
        }
        let mut sink = KeySink::new("compiler-completed-view/v1");
        relation.encode(&mut sink);
        for contribution in &contributions { contribution.encode(&mut sink); }
        // Count is deduplicated metadata, not a substitute for exact membership.
        sink.part(b"rows",&rows.to_le_bytes());
        Ok(Self { relation, contributions, rows, identity: sink.finish() })
    }
    pub fn validate(&self) -> Result<(), ModelError> {
        if Self::new(self.relation.clone(), self.contributions.clone(), self.rows)?.identity != self.identity {
            return Err(ModelError::Conflict("completed view identity"));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OutputContent { pub rows: u64, pub content: ContentHash }

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CompletedContribution {
    pub spec: ContributionSpec,
    pub outcome: i16,
    pub outputs: BTreeMap<String, OutputContent>,
}
impl CompletedContribution {
    pub fn identity(&self) -> Result<ContentHash, ModelError> {
        if !(0..=4).contains(&self.outcome) || self.outcome == 3
            || self.outputs.keys().cloned().collect::<BTreeSet<_>>() != self.spec.outputs
        { return Err(ModelError::Invalid("incomplete contribution output inventory/outcome".into())); }
        let mut sink = KeySink::new("compiler-completed-contribution/v1");
        self.spec.identity()?.encode(&mut sink); self.outcome.encode(&mut sink);
        for (name, output) in &self.outputs {
            name.encode(&mut sink); sink.part(b"rows",&output.rows.to_le_bytes()); output.content.encode(&mut sink);
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

pub const STATE_FORMAT_VERSION: u32 = 1;
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
    pub fn validate(&self) -> Result<(), ModelError> {
        if self.format_version != STATE_FORMAT_VERSION {
            return Err(ModelError::Invalid("unsupported completed-state format".into()));
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
    pub fn validate(&self)->Result<(),ModelError> {
        self.view.validate()?;
        if self.source.relation()!=self.view.relation || self.source.view()!=self.view.identity
            || self.source.rows()<0 || self.source.rows() as u64 != self.view.rows
            || self.boundary.as_ref().is_some_and(|boundary|boundary.is_empty()) {
            return Err(ModelError::Conflict("completed binding exact view"));
        }
        Ok(())
    }
    pub fn key(&self)->ContentHash {
        let mut sink=KeySink::new("compiler-binding-key/v1");
        self.source.relation().to_string().encode(&mut sink);self.boundary.encode(&mut sink);sink.finish()
    }
}
