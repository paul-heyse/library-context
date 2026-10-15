//! Installation-local completed view data and held read authority.
//! Deserializing completion data never establishes a native hold or cold admission.
pub(crate) mod audit;
pub use audit::{InstallationAudit, ProtectedInspectionSource, contribution_heap, view_heap};
mod earned;
mod live;
mod receiving;
pub use earned::{EarnedContributionSet, SelectedViewDescriptor, select_live_view_scoped};
use lctx_model::domain::{self as d, ContentHash, Key, KeySink, ModelError};
pub(crate) use live::recovery_view;
pub use live::{
    InstalledMember, SelectedMembers, attach_live_view, attach_live_views,
    attach_live_views_retaining, hold_frozen_view, hold_physical_view, hold_published_view,
};
pub(crate) use receiving::protect_receiving_views;
use serde::{Deserialize, Serialize};
use surrealdb::types::{RecordId, Value};

pub const MEMBER_CODEC: u32 = 3;
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ContributorRealization {
    pub view: ContentHash,
    pub logical: ContentHash,
    pub physical: ContentHash,
    pub incarnation: i64,
}
/// An exact view selector inside one compound completion. Its identity excludes
/// the parent link, so the parent inventory can bind selectors without a hash cycle.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InstalledViewCompletion {
    pub version: u32,
    pub parent_completion: ContentHash,
    pub generation: ContentHash,
    pub validity_epoch: i64,
    /// One immutable typed payload, shared by its inventory and retained caps.
    /// Sharing this data alone conveys neither a native pin nor resource ownership.
    #[serde(flatten)]
    pub selection: std::sync::Arc<SelectedViewDescriptor>,
    pub member_codec: u32,
    pub membership_rows: u64,
    pub membership_content: ContentHash,
}
impl std::ops::Deref for InstalledViewCompletion {
    type Target = SelectedViewDescriptor;
    fn deref(&self) -> &Self::Target {
        &self.selection
    }
}
impl InstalledViewCompletion {
    pub fn validate(&self) -> Result<(), ModelError> {
        self.view.validate()?;
        if self.version != 2
            || self.member_codec != MEMBER_CODEC
            || self.validity_epoch <= 0
            || self.membership_rows != self.view.rows
        {
            return Err(ModelError::Conflict("compound view selector structure"));
        }
        realization_id(&self.view, &self.mappings)?;
        if self.view.contributions.iter().any(|logical| {
            !self
                .mappings
                .iter()
                .any(|mapping| mapping.view == self.view.identity && mapping.logical == *logical)
        }) {
            return Err(ModelError::Conflict(
                "compound view selector missing root incidence",
            ));
        }
        Ok(())
    }
    pub fn identity(&self) -> Result<ContentHash, ModelError> {
        self.validate()?;
        let mut descriptor = self.clone();
        descriptor.parent_completion = ContentHash([0; 32]);
        let mut sink = KeySink::new("native-compound-view-selector/v2");
        sink.part(
            b"descriptor",
            &serde_json::to_vec(&descriptor).map_err(ModelError::codec)?,
        );
        Ok(sink.finish())
    }
    pub fn record(&self) -> Result<RecordId, ModelError> {
        self.validate()?;
        let mut sink = KeySink::new("native-completed-view/v2");
        self.parent_completion.encode(&mut sink);
        self.view.identity.encode(&mut sink);
        Ok(RecordId::new("native_completed_view", sink.finish().hex()))
    }
    pub fn value(&self) -> Result<Value, ModelError> {
        let mut row = surrealdb::types::Object::new();
        row.insert("id", self.record()?);
        row.insert(
            "completion",
            RecordId::new("native_completed_state", self.parent_completion.hex()),
        );
        row.insert("logical_view", self.view.identity.hex());
        row.insert(
            "descriptor",
            surrealdb::types::Bytes::from(serde_json::to_vec(self).map_err(ModelError::codec)?),
        );
        Ok(Value::Object(row))
    }
    pub fn from_row(row: &Value) -> Result<Self, ModelError> {
        let Some(Value::Bytes(bytes)) = row.as_object().and_then(|row| row.get("descriptor"))
        else {
            return Err(ModelError::Schema("compound selector descriptor"));
        };
        let result: Self = serde_json::from_slice(bytes).map_err(ModelError::codec)?;
        if result.value()? != *row {
            return Err(ModelError::Conflict(
                "compound selector exact persisted envelope",
            ));
        }
        Ok(result)
    }
}
/// Exact selector key, independent of any physical per-view namespace.
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
        return Err(ModelError::Conflict("selector mapping order/incarnation"));
    }
    let mut sink = KeySink::new("native-exact-selector/v2");
    view.identity.encode(&mut sink);
    for mapping in mappings {
        mapping.view.encode(&mut sink);
        mapping.logical.encode(&mut sink);
        mapping.physical.encode(&mut sink);
        sink.part(b"incarnation", &mapping.incarnation.to_le_bytes());
    }
    Ok(sink.finish())
}
/// Temporary canonical selector member address used only for ordered validation.
/// No rows with this family are installed or read as authority.
pub fn member_id(selector: ContentHash, relation: &str, key: &[u8; 16]) -> RecordId {
    let mut sink = KeySink::new("native-selected-member/v3");
    selector.encode(&mut sink);
    relation.to_owned().encode(&mut sink);
    sink.part(b"key", key);
    RecordId::new("selected_member", sink.finish().hex())
}

/// Native factories retain exact completion/realization authority through a live
/// attempt or a protected publication pin. There is no public data-only constructor.
pub struct HeldInstalledView {
    client: std::sync::Arc<surrealdb::Surreal<surrealdb::engine::remote::grpc::Client>>,
    selector: HeldSelector,
    owner: InstalledViewOwner,
    _charge: d::charged::StateCharge,
}
enum HeldSelector {
    // Scalar physical context is cheap; the sole variable-size payload is owned
    // by the exact inventory retained through InstalledViewOwner.
    Retained(InstalledViewCompletion),
    Earned(SelectedViewDescriptor),
}
impl std::ops::Deref for HeldSelector {
    type Target = SelectedViewDescriptor;
    fn deref(&self) -> &Self::Target {
        match self {
            Self::Retained(completion) => &completion.selection,
            Self::Earned(selector) => selector,
        }
    }
}
enum InstalledViewOwner {
    Earned {
        set: std::sync::Arc<EarnedContributionSet>,
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
        inventory: std::sync::Arc<crate::completed_state::InstalledViewInventory>,
        pin: std::sync::Arc<crate::control::ReaderPin>,
        owner: RecordId,
    },
}
impl HeldInstalledView {
    /// Check the original backing pool; a matching descriptor cannot transfer its charge.
    pub fn require_pool(&self, budget: &d::resources::ResourceBudget) -> Result<(), ModelError> {
        let inventory = match &self.owner {
            InstalledViewOwner::Earned { set } => {
                return if set
                    ._charge
                    .budget()
                    .is_some_and(|owner| owner.shares_pool(budget))
                {
                    Ok(())
                } else {
                    Err(ModelError::Conflict(
                        "earned selector foreign resource pool",
                    ))
                };
            }
            InstalledViewOwner::Recovery { inventory, .. } => inventory.as_ref(),
            InstalledViewOwner::Physical { completed, .. } => completed.inventory(),
            InstalledViewOwner::Frozen { completed, .. } => completed.inventory(),
            InstalledViewOwner::Published { completed, .. } => completed.inventory(),
        };
        inventory.require_pool(budget)
    }
    pub(crate) fn earned_source(&self) -> Option<&std::sync::Arc<EarnedContributionSet>> {
        match &self.owner {
            InstalledViewOwner::Earned { set } => Some(set),
            _ => None,
        }
    }
    pub(crate) fn validation_client(
        &self,
    ) -> std::sync::Arc<surrealdb::Surreal<surrealdb::engine::remote::grpc::Client>> {
        self.client.clone()
    }
    /// Preserve a borrowed terminal-product pin after unconfirmed transport. Other
    /// installed owners already retain their independent durable source lifetime.
    pub fn retain_unknown(&self) {
        match &self.owner {
            InstalledViewOwner::Recovery { pin, .. } => pin.retain_unknown(),
            InstalledViewOwner::Earned { set } => set.retain_unknown(),
            _ => {}
        }
    }
    pub fn completion(&self) -> &SelectedViewDescriptor {
        &self.selector
    }
    pub fn selector(&self) -> &SelectedViewDescriptor {
        &self.selector
    }
    pub fn retained_completion(&self) -> Result<&InstalledViewCompletion, ModelError> {
        match &self.selector {
            HeldSelector::Retained(completion) => Ok(completion),
            HeldSelector::Earned(_) => Err(ModelError::Conflict(
                "internal selection has no retained boundary",
            )),
        }
    }
    pub fn owner(&self) -> &RecordId {
        match &self.owner {
            InstalledViewOwner::Earned { set } => set.owner(),
            InstalledViewOwner::Published { owner, .. }
            | InstalledViewOwner::Frozen { owner, .. }
            | InstalledViewOwner::Physical { owner, .. }
            | InstalledViewOwner::Recovery { owner, .. } => owner,
        }
    }
    pub fn incarnation(&self) -> i64 {
        self.completion_incarnation()
    }
    pub fn completion_incarnation(&self) -> i64 {
        match &self.owner {
            InstalledViewOwner::Earned { .. } => 0,
            InstalledViewOwner::Published { expected, .. }
            | InstalledViewOwner::Frozen { expected, .. }
            | InstalledViewOwner::Physical { expected, .. }
            | InstalledViewOwner::Recovery { expected, .. } => expected.incarnation(),
        }
    }
    fn expected_bytes(&self) -> &Value {
        match &self.owner {
            InstalledViewOwner::Earned { .. } => {
                panic!("internal selection has no durable completion row")
            }
            InstalledViewOwner::Published { expected, .. }
            | InstalledViewOwner::Frozen { expected, .. }
            | InstalledViewOwner::Physical { expected, .. }
            | InstalledViewOwner::Recovery { expected, .. } => expected.row(),
        }
    }
    pub fn address(&self, relation: &str, key: &[u8; 16]) -> Result<RecordId, ModelError> {
        if relation != self.selector.view.relation {
            return Err(ModelError::Conflict("held view selected relation"));
        }
        Ok(member_id(
            realization_id(&self.selector.view, &self.selector.mappings)?,
            relation,
            key,
        ))
    }
}

/// Discovery only. This reads one explicitly selected physical contributor under the
/// caller's tracked/cancellable reader; only checked selector factories can mint a held view.
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

/// The physical schema lives with the compound completion inventory.
pub fn schema_target() -> &'static str {
    ""
}
