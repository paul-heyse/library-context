//! Installation-local completed view data and held read authority.
//! Deserializing completion data never establishes a native hold or cold admission.
pub(crate) mod audit;
pub use audit::InstallationAudit;
mod live;
use lctx_model::domain::{self as d, ContentHash, Key, KeySink, ModelError};
pub(crate) use live::recovery_view;
pub use live::{
    InstalledMember, SelectedMembers, attach_live_view, attach_live_views, hold_frozen_view,
    hold_physical_view, hold_published_view, install_live_view, install_live_view_scoped,
};
use serde::{Deserialize, Serialize};
use surrealdb::types::{RecordId, Value};

pub const MEMBER_CODEC: u32 = 2;
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ContributorRealization {
    /// Logical view whose contributor incidence selects this exact physical copy.
    pub view: ContentHash,
    pub logical: ContentHash,
    pub physical: ContentHash,
    pub incarnation: i64,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InstalledViewCompletion {
    pub generation: ContentHash,
    pub validity_epoch: i64,
    pub realization_incarnation: i64,
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
        if self.version != 1
            || self.member_codec != MEMBER_CODEC
            || self.frozen_revision < 0
            || self.validity_epoch <= 0
            || self.realization_incarnation <= 0
            || self.membership_rows != self.view.rows
            || self
                .mappings
                .windows(2)
                .any(|pair| (pair[0].view, pair[0].logical) >= (pair[1].view, pair[1].logical))
            || self.mappings.iter().any(|mapping| mapping.incarnation <= 0)
            || self.view.contributions.iter().any(|logical| {
                !self.mappings.iter().any(|mapping| {
                    mapping.view == self.view.identity && mapping.logical == *logical
                })
            })
            || realization_id(&self.view, &self.mappings)? != self.realization
        {
            return Err(ModelError::Conflict("installed view completion structure"));
        }
        self.closure.identity()?;
        Ok(())
    }
    pub fn identity(&self) -> Result<ContentHash, ModelError> {
        self.validate()?;
        let mut sink = KeySink::new("native-installed-view-completion/v1");
        sink.part(
            b"canonical",
            &serde_json::to_vec(self).map_err(ModelError::codec)?,
        );
        Ok(sink.finish())
    }
    pub fn record(&self) -> Result<RecordId, ModelError> {
        Ok(RecordId::new(
            "compiler_view_completion",
            self.identity()?.hex(),
        ))
    }
    pub fn value(&self) -> Result<Value, ModelError> {
        let mut row = surrealdb::types::Object::new();
        row.insert("id", self.record()?);
        row.insert(
            "descriptor",
            surrealdb::types::Bytes::from(serde_json::to_vec(self).map_err(ModelError::codec)?),
        );
        row.insert("completed", true);
        Ok(Value::Object(row))
    }
    pub fn from_row(row: &Value) -> Result<Self, ModelError> {
        let Value::Object(object) = row else {
            return Err(ModelError::Schema("installed completion row"));
        };
        let Some(Value::Bytes(bytes)) = object.get("descriptor") else {
            return Err(ModelError::Schema("installed completion descriptor"));
        };
        let result: Self = serde_json::from_slice(bytes).map_err(ModelError::codec)?;
        if result.value()? != *row {
            return Err(ModelError::Conflict("installed completion exact envelope"));
        }
        Ok(result)
    }
    /// Stable realization declaration excludes installation-local completion validity.
    pub fn realization_value(&self) -> Result<Value, ModelError> {
        self.validate()?;
        realization_declaration(&self.view, &self.mappings, true)
    }
}
pub fn realization_id(
    view: &d::completed::CompletedView,
    mappings: &[ContributorRealization],
) -> Result<ContentHash, ModelError> {
    view.validate()?;
    if mappings
        .windows(2)
        .any(|pair| (pair[0].view, pair[0].logical) >= (pair[1].view, pair[1].logical))
        || mappings.iter().any(|mapping| mapping.incarnation <= 0)
    {
        return Err(ModelError::Conflict(
            "view realization mapping order/incarnation",
        ));
    }
    let mut sink = KeySink::new("native-installed-view-realization/v1");
    view.identity.encode(&mut sink);
    sink.part(b"member-codec", &MEMBER_CODEC.to_le_bytes());
    for mapping in mappings {
        mapping.view.encode(&mut sink);
        mapping.logical.encode(&mut sink);
        mapping.physical.encode(&mut sink);
        sink.part(b"incarnation", &mapping.incarnation.to_le_bytes());
    }
    Ok(sink.finish())
}
pub fn member_id(realization: ContentHash, relation: &str, key: &[u8; 16]) -> RecordId {
    let mut sink = KeySink::new("native-view-member/v2");
    realization.encode(&mut sink);
    relation.to_owned().encode(&mut sink);
    sink.part(b"key", key);
    RecordId::new("compiler_view_member", sink.finish().hex())
}

/// Native factories retain exact completion/realization authority through a live
/// attempt or a protected publication pin. There is no public data-only constructor.
pub struct HeldInstalledView {
    client: std::sync::Arc<surrealdb::Surreal<surrealdb::engine::remote::grpc::Client>>,
    completion: InstalledViewCompletion,
    owner: InstalledViewOwner,
    _charge: d::charged::StateCharge,
}
enum InstalledViewOwner {
    Live {
        held: crate::control::HeldRecord,
        epoch: i64,
    },
    Physical {
        expected: crate::control::ExpectedRecord,
        completed: std::sync::Arc<crate::completed_state::NativeCompletedState>,
        owner: RecordId,
    },
    Frozen {
        expected: crate::control::ExpectedRecord,
        completed: std::sync::Arc<crate::completed_state::HeldFrozenCompletedState>,
        owner: RecordId,
    },
    Published {
        expected: crate::control::ExpectedRecord,
        completed: crate::completed_state::PublishedCompletedState,
        owner: RecordId,
    },
    Recovery {
        expected: crate::control::ExpectedRecord,
        pin: std::sync::Arc<crate::control::ReaderPin>,
        owner: RecordId,
    },
}
impl HeldInstalledView {
    pub(crate) fn validation_client(
        &self,
    ) -> std::sync::Arc<surrealdb::Surreal<surrealdb::engine::remote::grpc::Client>> {
        self.client.clone()
    }
    /// Preserve a borrowed terminal-product pin after unconfirmed transport. Other
    /// installed owners already retain their independent durable source lifetime.
    pub fn retain_unknown(&self) {
        if let InstalledViewOwner::Recovery { pin, .. } = &self.owner {
            pin.retain_unknown();
        }
    }
    pub fn completion(&self) -> &InstalledViewCompletion {
        &self.completion
    }
    pub fn owner(&self) -> &RecordId {
        match &self.owner {
            InstalledViewOwner::Live { held, .. } => held.owner(),
            InstalledViewOwner::Published { owner, .. }
            | InstalledViewOwner::Frozen { owner, .. }
            | InstalledViewOwner::Physical { owner, .. }
            | InstalledViewOwner::Recovery { owner, .. } => owner,
        }
    }
    pub fn incarnation(&self) -> i64 {
        self.completion.realization_incarnation
    }
    pub fn completion_incarnation(&self) -> i64 {
        match &self.owner {
            InstalledViewOwner::Live { held, .. } => held.incarnation(),
            InstalledViewOwner::Published { expected, .. }
            | InstalledViewOwner::Frozen { expected, .. }
            | InstalledViewOwner::Physical { expected, .. }
            | InstalledViewOwner::Recovery { expected, .. } => expected.incarnation(),
        }
    }
    fn expected_record(&self) -> &RecordId {
        match &self.owner {
            InstalledViewOwner::Live { held, .. } => held.record(),
            InstalledViewOwner::Published { expected, .. }
            | InstalledViewOwner::Frozen { expected, .. }
            | InstalledViewOwner::Physical { expected, .. }
            | InstalledViewOwner::Recovery { expected, .. } => expected.record(),
        }
    }
    fn expected_bytes(&self) -> &Value {
        match &self.owner {
            InstalledViewOwner::Live { held, .. } => held.row(),
            InstalledViewOwner::Published { expected, .. }
            | InstalledViewOwner::Frozen { expected, .. }
            | InstalledViewOwner::Physical { expected, .. }
            | InstalledViewOwner::Recovery { expected, .. } => expected.row(),
        }
    }
    pub fn address(&self, relation: &str, key: &[u8; 16]) -> Result<RecordId, ModelError> {
        if relation != self.completion.view.relation {
            return Err(ModelError::Conflict("held view selected relation"));
        }
        Ok(member_id(self.completion.realization, relation, key))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn view() -> d::completed::CompletedView {
        d::completed::CompletedView::new("packages".into(), [ContentHash::of(b"logical")].into(), 0)
            .unwrap()
    }
    #[test]
    fn realization_namespace_changes_with_physical_copy_incarnation_and_relation() {
        let view = view();
        let logical = *view.contributions.first().unwrap();
        let mut mappings = vec![ContributorRealization {
            view: view.identity,
            logical,
            physical: ContentHash::of(b"copy-a"),
            incarnation: 1,
        }];
        let first = realization_id(&view, &mappings).unwrap();
        let key = [1u8; 16];
        assert_ne!(
            member_id(first, "packages", &key),
            member_id(first, "releases", &key)
        );
        mappings[0].physical = ContentHash::of(b"copy-b");
        assert_ne!(first, realization_id(&view, &mappings).unwrap());
        mappings[0].physical = ContentHash::of(b"copy-a");
        mappings[0].incarnation += 1;
        assert_ne!(first, realization_id(&view, &mappings).unwrap());
        mappings[0].incarnation = 0;
        assert!(realization_id(&view, &mappings).is_err());
    }
    #[test]
    fn data_only_completion_refuses_same_count_substitution_and_incomplete_inventory() {
        use d::recovery_closure::COMPLETED_FAMILIES;
        let view = view();
        let mappings = vec![ContributorRealization {
            view: view.identity,
            logical: *view.contributions.first().unwrap(),
            physical: ContentHash::of(b"copy"),
            incarnation: 1,
        }];
        let mut completion = InstalledViewCompletion {
            version: 1,
            generation: ContentHash::of(b"generation"),
            validity_epoch: 1,
            realization_incarnation: 1,
            realization: realization_id(&view, &mappings).unwrap(),
            view,
            member_codec: MEMBER_CODEC,
            mappings,
            closure: d::completed_identity::CompletedFamilyRoots::new(COMPLETED_FAMILIES.map(
                |family| {
                    d::completed_identity::CompletedFamilyHasher::new(family)
                        .unwrap()
                        .finish()
                },
            ))
            .unwrap(),
            membership_rows: 0,
            membership_content: ContentHash::of(b"empty exact member stream"),
            frozen_revision: 0,
        };
        completion.validate().unwrap();
        completion.mappings[0].physical = ContentHash::of(b"same-count-other-copy");
        assert!(completion.validate().is_err());
        completion.mappings.clear();
        assert!(completion.validate().is_err());
    }
    #[test]
    fn validity_completion_is_epoch_qualified_and_does_not_rename_stable_realization() {
        use d::recovery_closure::COMPLETED_FAMILIES;
        let view = view();
        let mappings = vec![ContributorRealization {
            view: view.identity,
            logical: *view.contributions.first().unwrap(),
            physical: ContentHash::of(b"copy"),
            incarnation: 1,
        }];
        let original = InstalledViewCompletion {
            generation: ContentHash::of(b"generation"),
            validity_epoch: 1,
            realization_incarnation: 4,
            version: 1,
            realization: realization_id(&view, &mappings).unwrap(),
            view,
            member_codec: MEMBER_CODEC,
            mappings,
            closure: d::completed_identity::CompletedFamilyRoots::new(COMPLETED_FAMILIES.map(
                |family| {
                    d::completed_identity::CompletedFamilyHasher::new(family)
                        .unwrap()
                        .finish()
                },
            ))
            .unwrap(),
            membership_rows: 0,
            membership_content: ContentHash::of(b"empty"),
            frozen_revision: 2,
        };
        let mut rechecked = original.clone();
        rechecked.validity_epoch += 1;
        assert_ne!(
            original.record().unwrap(),
            rechecked.record().unwrap(),
            "new epoch earns distinct immutable completion"
        );
        assert_eq!(
            original.realization_value().unwrap(),
            rechecked.realization_value().unwrap(),
            "epoch recheck does not overwrite stable namespace"
        );
        rechecked.realization_incarnation += 1;
        assert_ne!(
            original.identity().unwrap(),
            rechecked.identity().unwrap(),
            "actual realization recreation invalidates completion"
        );
        let row = original.value().unwrap();
        assert_eq!(InstalledViewCompletion::from_row(&row).unwrap(), original);
        let Value::Object(mut wrong) = row else {
            unreachable!()
        };
        wrong.insert(
            "id",
            RecordId::new("compiler_view_realization", original.realization.hex()),
        );
        assert!(InstalledViewCompletion::from_row(&Value::Object(wrong)).is_err());
    }
}

pub(crate) fn realization_declaration(
    view: &d::completed::CompletedView,
    mappings: &[ContributorRealization],
    sealed: bool,
) -> Result<Value, ModelError> {
    #[derive(Serialize)]
    struct Declaration<'a> {
        version: u32,
        view: &'a d::completed::CompletedView,
        member_codec: u32,
        mappings: &'a [ContributorRealization],
    }
    let realization = realization_id(view, mappings)?;
    let descriptor = Declaration {
        version: 1,
        view,
        member_codec: MEMBER_CODEC,
        mappings,
    };
    let mut row = surrealdb::types::Object::new();
    row.insert(
        "id",
        RecordId::new("compiler_view_realization", realization.hex()),
    );
    row.insert("sealed", sealed);
    row.insert(
        "descriptor",
        surrealdb::types::Bytes::from(serde_json::to_vec(&descriptor).map_err(ModelError::codec)?),
    );
    Ok(Value::Object(row))
}
/// Target format only, installed by the format coordinator. No legacy member namespace
/// is consulted after cutover; the stable realization and epoch-qualified witness differ.
pub fn schema_target() -> &'static str {
    "DEFINE TABLE compiler_view_realization SCHEMAFULL; DEFINE FIELD descriptor ON compiler_view_realization TYPE bytes; DEFINE FIELD sealed ON compiler_view_realization TYPE bool; DEFINE TABLE compiler_view_completion SCHEMAFULL; DEFINE FIELD descriptor ON compiler_view_completion TYPE bytes; DEFINE FIELD completed ON compiler_view_completion TYPE bool; DEFINE TABLE compiler_view_member SCHEMAFULL; DEFINE FIELD realization ON compiler_view_member TYPE record<compiler_view_realization>; DEFINE FIELD relation ON compiler_view_member TYPE string; DEFINE FIELD semantic_key ON compiler_view_member TYPE string; DEFINE FIELD node ON compiler_view_member TYPE record<entity | assertion | compiler_record>; DEFINE FIELD content ON compiler_view_member TYPE string; DEFINE INDEX realization_key ON compiler_view_member FIELDS realization,relation,semantic_key UNIQUE; DEFINE INDEX realization_nodes ON compiler_view_member FIELDS realization,node;"
}

/// Discovery only. This reads one explicitly selected physical contributor under the
/// caller's tracked/cancellable reader; only install_live_view can mint a held view.
pub async fn inspect_contributor(
    reader: &crate::NativeReader<()>,
    physical: ContentHash,
    budget: &d::resources::ResourceBudget,
) -> Result<Option<crate::control::ExpectedRecord>, ModelError> {
    crate::control::inspect_expected_read(
        reader,
        RecordId::new("compiler_contribution", physical.hex()),
        budget,
    )
    .await
}
