//! Installation-local completed view data and held read authority.
//! Deserializing completion data never establishes a native hold or cold admission.
use lctx_model::domain::{self as d, ContentHash, Key, KeySink, ModelError};
use serde::{Deserialize, Serialize};
use surrealdb::types::{RecordId, Value};

pub const MEMBER_CODEC: u32 = 2;
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ContributorRealization {
    pub logical: ContentHash,
    pub physical: ContentHash,
    pub incarnation: i64,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InstalledViewCompletion {
    pub version: u32,
    pub view: d::completed::CompletedView,
    pub realization: ContentHash,
    pub member_codec: u32,
    pub mappings: Vec<ContributorRealization>,
    pub closure: d::completed_identity::CompletedFamilyRoots,
    pub membership_rows: u64,
    pub membership_content: ContentHash,
    pub frozen_revision: i64,
}
impl InstalledViewCompletion {
    /// Structural validation only. Native completion additionally requires exact
    /// backing/closure checks, checked EOF, mutation freeze and a durable final effect.
    pub fn validate(&self) -> Result<(), ModelError> {
        self.view.validate()?;
        if self.version != 1 || self.member_codec != MEMBER_CODEC || self.frozen_revision < 0
            || self.membership_rows != self.view.rows
            || self.mappings.windows(2).any(|pair| pair[0].logical >= pair[1].logical)
            || self.mappings.iter().any(|mapping| mapping.incarnation <= 0)
            || self.mappings.iter().map(|mapping| mapping.physical).collect::<std::collections::BTreeSet<_>>().len() != self.mappings.len()
            || self.view.contributions.iter().any(|logical| !self.mappings.iter().any(|mapping| mapping.logical == *logical))
            || realization_id(&self.view, &self.mappings)? != self.realization {
            return Err(ModelError::Conflict("installed view completion structure"));
        }
        self.closure.identity()?;
        Ok(())
    }
}
pub fn realization_id(view: &d::completed::CompletedView, mappings: &[ContributorRealization]) -> Result<ContentHash, ModelError> {
    view.validate()?;
    if mappings.windows(2).any(|pair| pair[0].logical >= pair[1].logical)
        || mappings.iter().any(|mapping| mapping.incarnation <= 0) {
        return Err(ModelError::Conflict("view realization mapping order/incarnation"));
    }
    let mut sink = KeySink::new("native-installed-view-realization/v1");
    view.identity.encode(&mut sink);
    sink.part(b"member-codec", &MEMBER_CODEC.to_le_bytes());
    for mapping in mappings {
        mapping.logical.encode(&mut sink); mapping.physical.encode(&mut sink);
        sink.part(b"incarnation", &mapping.incarnation.to_le_bytes());
    }
    Ok(sink.finish())
}
pub fn member_id(realization: ContentHash, relation: &str, key: &[u8; 16]) -> RecordId {
    let mut sink = KeySink::new("native-view-member/v2");
    realization.encode(&mut sink); relation.to_owned().encode(&mut sink); sink.part(b"key", key);
    RecordId::new("compiler_view_member", sink.finish().hex())
}

/// The only construction route must hold the exact completion row/incarnation
/// through the owning native attempt. There is deliberately no public constructor.
pub struct HeldInstalledView {
    completion: InstalledViewCompletion,
    held: crate::control::HeldRecord,
}
impl HeldInstalledView {
    pub fn completion(&self) -> &InstalledViewCompletion { &self.completion }
    pub fn owner(&self) -> &RecordId { self.held.owner() }
    pub fn incarnation(&self) -> i64 { self.held.incarnation() }
    pub fn address(&self, relation: &str, key: &[u8; 16]) -> Result<RecordId, ModelError> {
        if relation != self.completion.view.relation { return Err(ModelError::Conflict("held view selected relation")); }
        Ok(member_id(self.completion.realization, relation, key))
    }
}

pub async fn hold_completion(
    client: &surrealdb::Surreal<surrealdb::engine::remote::grpc::Client>,
    attempt: ContentHash,
    owner: RecordId,
    expected: crate::control::ExpectedRecord,
) -> Result<HeldInstalledView, ModelError> {
    let Value::Object(row) = expected.row() else { return Err(ModelError::Schema("installed completion object")); };
    let Some(Value::Bytes(bytes)) = row.get("descriptor") else { return Err(ModelError::Schema("installed completion descriptor")); };
    let completion: InstalledViewCompletion = serde_json::from_slice(bytes.as_ref()).map_err(ModelError::codec)?;
    completion.validate()?;
    if expected.record() != &RecordId::new("compiler_view_realization", completion.realization.hex())
        || row.get("completed") != Some(&Value::Bool(true)) {
        return Err(ModelError::Conflict("installed completion exact physical namespace"));
    }
    let held = crate::control::hold_expected(client, attempt, owner, expected).await?;
    Ok(HeldInstalledView { completion, held })
}

#[cfg(test)]
mod tests {
    use super::*;
    fn view() -> d::completed::CompletedView {
        d::completed::CompletedView::new("packages".into(), [ContentHash::of(b"logical")].into(), 0).unwrap()
    }
    #[test]
    fn realization_namespace_changes_with_physical_copy_incarnation_and_relation() {
        let view = view();
        let logical = *view.contributions.first().unwrap();
        let mut mappings = vec![ContributorRealization { logical, physical: ContentHash::of(b"copy-a"), incarnation: 1 }];
        let first = realization_id(&view, &mappings).unwrap();
        let key = [1u8;16];
        assert_ne!(member_id(first, "packages", &key), member_id(first, "releases", &key));
        mappings[0].physical = ContentHash::of(b"copy-b");
        assert_ne!(first, realization_id(&view, &mappings).unwrap());
        mappings[0].physical = ContentHash::of(b"copy-a"); mappings[0].incarnation += 1;
        assert_ne!(first, realization_id(&view, &mappings).unwrap());
        mappings[0].incarnation = 0; assert!(realization_id(&view, &mappings).is_err());
    }
    #[test]
    fn data_only_completion_refuses_same_count_substitution_and_incomplete_inventory() {
        use d::recovery_closure::COMPLETED_FAMILIES;
        let view = view();
        let mappings = vec![ContributorRealization { logical: *view.contributions.first().unwrap(), physical: ContentHash::of(b"copy"), incarnation: 1 }];
        let mut completion = InstalledViewCompletion {
            version: 1, realization: realization_id(&view, &mappings).unwrap(), view,
            member_codec: MEMBER_CODEC, mappings,
            closure: d::completed_identity::CompletedFamilyRoots::new(COMPLETED_FAMILIES.map(|family| d::completed_identity::CompletedFamilyHasher::new(family).unwrap().finish())).unwrap(),
            membership_rows: 0, membership_content: ContentHash::of(b"empty exact member stream"), frozen_revision: 0,
        };
        completion.validate().unwrap();
        completion.mappings[0].physical = ContentHash::of(b"same-count-other-copy");
        assert!(completion.validate().is_err());
        completion.mappings.clear();
        assert!(completion.validate().is_err());
    }
}
