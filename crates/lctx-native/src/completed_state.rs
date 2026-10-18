//! Installation-local physical completion. Serialized data never grants freeze,
//! semantic admission or protected read authority; native factories earn those separately.
pub(crate) mod earning;
mod product_replay;
mod uniform;
pub use earning::{
    ViewSelection, complete_frozen_content, complete_frozen_content_from_source,
    complete_frozen_content_scoped, complete_retained_content,
    complete_retained_content_from_source, complete_retained_content_scoped,
};
use lctx_model::domain::{self as d, ContentHash, Key, KeySink, ModelError};
pub use product_replay::check_retained_product_replay;
use serde::{Deserialize, Serialize};
use surrealdb::types::{Object, RecordId, SurrealValue, Value};
pub use uniform::PublishedUniformOwners;

pub const VERSION: u32 = 2;
/// Shared with the owning native fixture's EXPLAIN control; bindings grant no authority.
#[doc(hidden)]
pub const COMPOUND_BINDING_POINTERS_SQL: &str = "SELECT object AS id FROM native_hold WITH INDEX owner_holds WHERE owner=$compound_parent AND record::table(object)='compiler_binding_descriptor' ORDER BY object";
/// Shared with the owning native fixture's EXPLAIN control; complete EOF remains required.
#[doc(hidden)]
pub const COMPOUND_PARENT_HOLDS_SQL: &str =
    "SELECT * FROM native_hold WITH INDEX owner_holds WHERE owner=$compound_parent ORDER BY id";
#[cfg(test)]
#[derive(Clone)]
pub(crate) struct FinalParentObservationBarrier {
    pub(crate) observed: std::sync::Arc<tokio::sync::Barrier>,
    pub(crate) resume: std::sync::Arc<tokio::sync::Barrier>,
}
#[cfg(test)]
tokio::task_local! {
    pub(crate) static FINAL_PARENT_OBSERVATION_BARRIER: FinalParentObservationBarrier;
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RealizationInventory {
    pub rows: u64,
    pub content: ContentHash,
}
/// Full completion data. Physical validity is installation-local, while `completed_state`
/// is the separate logical identity over the six complete normalized family roots.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FrozenCompletion {
    pub version: u32,
    pub generation: ContentHash,
    pub attempt: ContentHash,
    pub attempt_epoch: i64,
    /// Source revision captured for this exact boundary. Zero is valid for an
    /// open-source retained boundary; aggregate freeze authority is earned separately.
    pub completed_revision: i64,
    pub validity_epoch: i64,
    pub completed_state: ContentHash,
    pub roots: d::completed_identity::CompletedFamilyRoots,
    pub bindings: ContentHash,
    pub realization_inventory: RealizationInventory,
    pub contributor_inventory: RealizationInventory,
}
impl FrozenCompletion {
    pub(crate) fn fence(&self) -> FrozenContentFence {
        FrozenContentFence {
            generation: self.generation,
            attempt: self.attempt,
            attempt_epoch: self.attempt_epoch,
            completed_revision: self.completed_revision,
            validity_epoch: self.validity_epoch,
            bindings: self.bindings,
        }
    }
    pub fn validate(&self) -> Result<(), ModelError> {
        if self.version != VERSION
            || self.attempt_epoch <= 0
            || self.completed_revision < 0
            || self.validity_epoch <= 0
            || self.roots.identity()? != self.completed_state
        {
            return Err(ModelError::Conflict("frozen completion structure"));
        }
        Ok(())
    }
    pub fn identity(&self) -> Result<ContentHash, ModelError> {
        self.validate()?;
        let mut sink = KeySink::new("native-frozen-completion/v2");
        sink.part(
            b"canonical",
            &serde_json::to_vec(self).map_err(ModelError::codec)?,
        );
        Ok(sink.finish())
    }
    pub fn record(&self) -> Result<RecordId, ModelError> {
        Ok(RecordId::new(
            "native_completed_state",
            self.identity()?.hex(),
        ))
    }
    pub fn value(&self, completed: bool) -> Result<Value, ModelError> {
        let mut row = Object::new();
        row.insert("id", self.record()?);
        row.insert(
            "descriptor",
            surrealdb::types::Bytes::from(serde_json::to_vec(self).map_err(ModelError::codec)?),
        );
        row.insert("completed", completed);
        Ok(Value::Object(row))
    }
    pub fn from_row(value: &Value, require_completed: bool) -> Result<Self, ModelError> {
        let Value::Object(row) = value else {
            return Err(ModelError::Schema("frozen completion row"));
        };
        let Some(Value::Bytes(bytes)) = row.get("descriptor") else {
            return Err(ModelError::Schema("frozen completion descriptor"));
        };
        let result: Self = serde_json::from_slice(bytes).map_err(ModelError::codec)?;
        result.validate()?;
        let Some(Value::Bool(completed)) = row.get("completed") else {
            return Err(ModelError::Schema("frozen completion state"));
        };
        if (require_completed && !*completed) || result.value(*completed)? != *value {
            return Err(ModelError::Conflict(
                "frozen completion exact persisted envelope",
            ));
        }
        Ok(result)
    }
}
/// One row of the completed inventory; the count/digest lives in FrozenCompletion.
/// Rows are ordered by logical view and retain the exact installed namespace/incarnation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InstalledViewRef {
    pub logical_view: ContentHash,
    pub selector: crate::installed_view::InstalledViewCompletion,
}
impl InstalledViewRef {
    pub fn validate(&self) -> Result<(), ModelError> {
        self.selector.validate()?;
        if self.logical_view != self.selector.view.identity {
            return Err(ModelError::Conflict("compound selector logical view"));
        }
        Ok(())
    }
    pub fn record(&self, completion: ContentHash) -> RecordId {
        let mut selector = self.selector.clone();
        selector.parent_completion = completion;
        selector.record().expect("validated compound selector")
    }
    pub fn value(&self, completion: ContentHash) -> Result<Value, ModelError> {
        self.validate()?;
        let mut selector = self.selector.clone();
        selector.parent_completion = completion;
        selector.value()
    }
    pub fn from_row(value: &Value, completion: ContentHash) -> Result<Self, ModelError> {
        let selector = crate::installed_view::InstalledViewCompletion::from_row(value)?;
        if selector.parent_completion != completion {
            return Err(ModelError::Conflict("compound selector parent"));
        }
        let result = Self {
            logical_view: selector.view.identity,
            selector,
        };
        result.validate()?;
        Ok(result)
    }
}
/// Full immutable selected copies, separately indexed from per-view incidence.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ContributorCopy {
    pub physical: ContentHash,
    pub incarnation: i64,
    pub descriptor: d::completed::CompletedContribution,
}
impl ContributorCopy {
    pub fn validate(&self) -> Result<(), ModelError> {
        if self.incarnation <= 0 {
            return Err(ModelError::Conflict("compound contributor incarnation"));
        }
        self.descriptor.identity()?;
        Ok(())
    }
    pub fn record(&self, completion: ContentHash) -> RecordId {
        contributor_copy_record(completion, self.physical)
    }
    pub fn value(&self, completion: ContentHash) -> Result<Value, ModelError> {
        self.validate()?;
        let mut row = Object::new();
        row.insert("id", self.record(completion));
        row.insert(
            "completion",
            RecordId::new("native_completed_state", completion.hex()),
        );
        row.insert("physical", self.physical.hex());
        row.insert(
            "descriptor",
            surrealdb::types::Bytes::from(serde_json::to_vec(self).map_err(ModelError::codec)?),
        );
        Ok(Value::Object(row))
    }
    pub fn from_row(row: &Value, completion: ContentHash) -> Result<Self, ModelError> {
        let Some(Value::Bytes(bytes)) = row.as_object().and_then(|row| row.get("descriptor"))
        else {
            return Err(ModelError::Schema("compound contributor descriptor"));
        };
        let result: Self = serde_json::from_slice(bytes).map_err(ModelError::codec)?;
        if result.value(completion)? != *row {
            return Err(ModelError::Conflict("compound contributor exact envelope"));
        }
        Ok(result)
    }
}
/// Streaming inventory digest excludes parent links and refuses duplicates/order.
pub struct RealizationHasher {
    rows: u64,
    last: Option<ContentHash>,
    sink: KeySink,
}
impl Default for RealizationHasher {
    fn default() -> Self {
        Self {
            rows: 0,
            last: None,
            sink: KeySink::new("native-completed-selector-inventory/v2"),
        }
    }
}
impl RealizationHasher {
    pub fn row(&mut self, row: &InstalledViewRef) -> Result<(), ModelError> {
        row.validate()?;
        if self.last.is_some_and(|last| last >= row.logical_view) {
            return Err(ModelError::Conflict(
                "compound selector inventory order/duplicate",
            ));
        }
        row.logical_view.encode(&mut self.sink);
        row.selector.identity()?.encode(&mut self.sink);
        self.rows = self
            .rows
            .checked_add(1)
            .ok_or(ModelError::Schema("compound selector count"))?;
        self.last = Some(row.logical_view);
        Ok(())
    }
    pub fn finish(mut self) -> RealizationInventory {
        self.sink.part(b"rows", &self.rows.to_le_bytes());
        RealizationInventory {
            rows: self.rows,
            content: self.sink.finish(),
        }
    }
}
pub struct ContributorHasher {
    rows: u64,
    last: Option<ContentHash>,
    sink: KeySink,
}
impl Default for ContributorHasher {
    fn default() -> Self {
        Self {
            rows: 0,
            last: None,
            sink: KeySink::new("native-completed-contributor-inventory/v2"),
        }
    }
}
impl ContributorHasher {
    pub fn row(&mut self, row: &ContributorCopy) -> Result<(), ModelError> {
        row.validate()?;
        if self.last.is_some_and(|last| last >= row.physical) {
            return Err(ModelError::Conflict(
                "compound contributor inventory order/duplicate",
            ));
        }
        self.sink.part(
            b"copy",
            &serde_json::to_vec(row).map_err(ModelError::codec)?,
        );
        self.rows = self
            .rows
            .checked_add(1)
            .ok_or(ModelError::Schema("compound contributor count"))?;
        self.last = Some(row.physical);
        Ok(())
    }
    pub fn finish(mut self) -> RealizationInventory {
        self.sink.part(b"rows", &self.rows.to_le_bytes());
        RealizationInventory {
            rows: self.rows,
            content: self.sink.finish(),
        }
    }
}
/// Only a native checked constructor can create this fence. It admits no arbitrary SQL.
pub struct FrozenContentFence {
    pub(crate) generation: ContentHash,
    pub(crate) attempt: ContentHash,
    pub(crate) attempt_epoch: i64,
    pub(crate) completed_revision: i64,
    pub(crate) validity_epoch: i64,
    pub(crate) bindings: ContentHash,
}
impl FrozenContentFence {
    pub fn attempt(&self) -> ContentHash {
        self.attempt
    }
    pub fn generation(&self) -> ContentHash {
        self.generation
    }
    pub fn attempt_epoch(&self) -> i64 {
        self.attempt_epoch
    }
    pub fn completed_revision(&self) -> i64 {
        self.completed_revision
    }
    pub fn validity_epoch(&self) -> i64 {
        self.validity_epoch
    }
    pub fn bindings(&self) -> ContentHash {
        self.bindings
    }
}
/// The native hold is on the exact independently read completion incarnation.
/// A DTO or matching hash cannot construct this owner.
pub struct HeldFrozenCompletedState {
    pub(crate) client: std::sync::Arc<surrealdb::Surreal<surrealdb::engine::remote::grpc::Client>>,
    pub(crate) completion: FrozenCompletion,
    pub(crate) held: crate::control::HeldRecord,
    pub(crate) admission: crate::control::HeldRecord,
    pub(crate) inventory: std::sync::Arc<InstalledViewInventory>,
    pub(crate) _charge: Box<dyn d::resources::Reservation>,
}
impl HeldFrozenCompletedState {
    pub fn completion(&self) -> &FrozenCompletion {
        &self.completion
    }
    pub fn owner(&self) -> &RecordId {
        self.held.owner()
    }
    pub fn admission_record(&self) -> &RecordId {
        self.admission.record()
    }
    pub fn admission_incarnation(&self) -> i64 {
        self.admission.incarnation()
    }
    pub fn inventory(&self) -> &InstalledViewInventory {
        &self.inventory
    }
    pub fn incarnation(&self) -> i64 {
        self.held.incarnation()
    }
}

/// Resolve the exact persisted physical completion selected by this admitted frozen
/// attempt. A logical manifest/completed-state digest is never a physical completion key.
pub async fn hold_admitted_completed_state_for_attempt(
    client: std::sync::Arc<surrealdb::Surreal<surrealdb::engine::remote::grpc::Client>>,
    attempt: ContentHash,
    budget: &d::resources::ResourceBudget,
) -> Result<HeldFrozenCompletedState, ModelError> {
    let reader = crate::NativeReader::private(client.clone()).with_budget(budget);
    let result = async {
        let _page = budget.reserve("admitted attempt exact completion lookup", 4096)?;
        let mut variables = surrealdb::types::Variables::new();
        variables.insert("attempt", RecordId::new("native_attempt", attempt.hex()));
        let rows: Vec<Object> = reader
            .query_native(
                "SELECT id,state,admitted,completed_identity FROM $attempt",
                variables,
            )
            .await?;
        let [row] = rows.as_slice() else {
            return Err(ModelError::Conflict(
                "admitted completed attempt cardinality",
            ));
        };
        if row.get("id")
            != Some(&Value::RecordId(RecordId::new(
                "native_attempt",
                attempt.hex(),
            )))
            || row.get("state") != Some(&Value::String("frozen".into()))
            || row.get("admitted") != Some(&Value::Bool(true))
        {
            return Err(ModelError::Conflict("admitted frozen completion attempt"));
        }
        let Some(Value::String(identity)) = row.get("completed_identity") else {
            return Err(ModelError::Schema("admitted physical completion identity"));
        };
        let identity = ContentHash(
            hex::decode(identity)
                .map_err(ModelError::codec)?
                .try_into()
                .map_err(|_| ModelError::Schema("admitted physical completion identity width"))?,
        );
        // The exact hold factory independently re-reads owner/epoch/bytes and inventory;
        // a raced or stale first lookup cannot grant authority.
        hold_completed_state(client.clone(), attempt, identity, budget).await
    }
    .await;
    let mut completion = d::completion::Completion::default();
    completion.step(
        "admitted completion lookup reader close",
        reader.close().await,
    );
    d::completion::complete(result, completion)
}

/// Acquires protected authority only from admitted persisted native state, then checks
/// every exact installed witness and the complete inventory EOF under the same epoch.
/// A decoded DTO is never accepted as input authority to this constructor.
pub async fn hold_completed_state(
    client: std::sync::Arc<surrealdb::Surreal<surrealdb::engine::remote::grpc::Client>>,
    attempt: ContentHash,
    identity: ContentHash,
    budget: &d::resources::ResourceBudget,
) -> Result<HeldFrozenCompletedState, ModelError> {
    let model = d::model()?;
    hold_completed_state_with_model(client, attempt, identity, &model, budget).await
}
/// Fresh admission may use an explicitly supplied validated model. This does not make
/// an undeclared executable factory eligible for later durable reuse.
pub async fn hold_completed_state_with_model(
    client: std::sync::Arc<surrealdb::Surreal<surrealdb::engine::remote::grpc::Client>>,
    attempt: ContentHash,
    identity: ContentHash,
    model: &d::ValidatedModel,
    budget: &d::resources::ResourceBudget,
) -> Result<HeldFrozenCompletedState, ModelError> {
    hold_completed_state_inner(client, attempt, identity, model, budget, None).await
}
/// Borrow the exact inventory earned by a still-owned physical completion. This is
/// not a data-only route: the actual client/pool and original parent incarnation
/// must correspond, and fresh frozen-owner/admission CAS checks still run.
pub async fn hold_completed_state_from_source_with_model(
    client: std::sync::Arc<surrealdb::Surreal<surrealdb::engine::remote::grpc::Client>>,
    source: &NativeCompletedState,
    model: &d::ValidatedModel,
    budget: &d::resources::ResourceBudget,
) -> Result<HeldFrozenCompletedState, ModelError> {
    if !std::sync::Arc::ptr_eq(&client, &source.client) || !source.budget.shares_pool(budget) {
        return Err(ModelError::Conflict(
            "completed source client/resource owner",
        ));
    }
    hold_completed_state_inner(
        client,
        source.completion.attempt,
        source.completion.identity()?,
        model,
        budget,
        Some(source),
    )
    .await
}
async fn hold_completed_state_inner(
    client: std::sync::Arc<surrealdb::Surreal<surrealdb::engine::remote::grpc::Client>>,
    attempt: ContentHash,
    identity: ContentHash,
    model: &d::ValidatedModel,
    budget: &d::resources::ResourceBudget,
    source: Option<&NativeCompletedState>,
) -> Result<HeldFrozenCompletedState, ModelError> {
    use surrealdb::types::Variables;
    let reader = crate::NativeReader::private(client.clone()).with_budget(budget);
    let result = async {
        let expected = crate::control::inspect_expected_read(
            &reader,
            RecordId::new("native_completed_state", identity.hex()),
            budget,
        )
        .await?
        .ok_or(ModelError::Conflict("native frozen completion missing"))?;
        let completion_decode = budget.reserve(
            "native frozen completion decoding",
            crate::loader::native_bytes(expected.row())
                .saturating_mul(32)
                .saturating_add(65536),
        )?;
        let data = FrozenCompletion::from_row(expected.row(), true)?;
        if data.identity()? != identity || data.attempt != attempt {
            return Err(ModelError::Conflict("native completed owner/identity"));
        }
        if let Some(source) = source
            && (data != source.completion || expected.incarnation() != source.incarnation())
        {
            return Err(ModelError::Conflict(
                "completed source exact parent changed",
            ));
        }
        drop(completion_decode);
        let held = crate::control::hold_frozen_completed_expected(&client, &data, expected, budget)
            .await?;
        let inventory = match source {
            Some(source) => source.inventory.clone(),
            None => inspect_inventory(&reader, &data, budget).await?,
        };
        let mut vars = Variables::new();
        vars.insert("attempt", RecordId::new("native_attempt", attempt.hex()));
        let admissions: Vec<RecordId> = reader
            .query_native("SELECT VALUE completed_admission FROM $attempt", vars)
            .await?;
        let [address] = admissions.as_slice() else {
            return Err(ModelError::Conflict(
                "frozen semantic admission pointer cardinality",
            ));
        };
        let admission = crate::control::inspect_expected_read(&reader, address.clone(), budget)
            .await?
            .ok_or(ModelError::Conflict("frozen semantic admission missing"))?;
        let _decode = budget.reserve(
            "frozen semantic admission decoding",
            crate::loader::native_bytes(admission.row())
                .saturating_mul(32)
                .saturating_add(65536),
        )?;
        let semantic = CompletedAdmission::from_row(admission.row())?;
        semantic.validate(&data, held.incarnation(), model, &inventory, budget)?;
        require_hold(&reader, address, &data.record()?).await?;
        let admission = crate::control::hold_frozen_admission_expected(
            &client, &data, &semantic, admission, budget,
        )
        .await?;
        drop(semantic);
        drop(_decode);
        // Recheck owner/installation after all reads. The first hold remains retained,
        // while this fixed exact effect confirms no epoch/admission change at return.
        let expected = crate::control::inspect_expected_read(&reader, data.record()?, budget)
            .await?
            .ok_or(ModelError::Conflict("native completion disappeared"))?;
        if expected.incarnation() != held.incarnation() {
            return Err(ModelError::Conflict("native completed incarnation changed"));
        }
        let repeated =
            crate::control::hold_frozen_completed_expected(&client, &data, expected, budget)
                .await?;
        drop(held);
        let completion_charge = budget.reserve(
            "native frozen completion retained owner",
            std::mem::size_of::<HeldFrozenCompletedState>()
                .saturating_add(2 * std::mem::size_of::<usize>())
                .saturating_add(crate::selection::node_bytes(repeated.owner()))
                .saturating_add(crate::selection::node_bytes(admission.owner())),
        )?;
        Ok(HeldFrozenCompletedState {
            client: client.clone(),
            completion: data,
            held: repeated,
            admission,
            inventory,
            _charge: completion_charge,
        })
    }
    .await;
    let mut completion = d::completion::Completion::default();
    completion.step("native completion reader close", reader.close().await);
    d::completion::complete(result, completion)
}
pub(crate) async fn require_hold(
    reader: &crate::NativeReader<()>,
    owner: &RecordId,
    object: &RecordId,
) -> Result<(), ModelError> {
    let mut bindings = surrealdb::types::Variables::new();
    bindings.insert("owner", owner.clone());
    bindings.insert("object", object.clone());
    let rows:Vec<RecordId>=reader.query_native("SELECT VALUE id FROM native_hold WITH INDEX owner_holds WHERE owner=$owner AND object=$object LIMIT 2",bindings).await?;
    if rows.len() != 1 {
        return Err(ModelError::Conflict(
            "completed exact hold incidence missing/duplicate",
        ));
    }
    Ok(())
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct SelectorCopyRequirement {
    physical: ContentHash,
    incarnation: i64,
    logical: ContentHash,
}
struct InstalledRefRequests<'a> {
    selectors: Vec<(RecordId, &'a crate::installed_view::InstalledViewCompletion)>,
    copies: Vec<SelectorCopyRequirement>,
    _charge: d::charged::StateCharge,
}
fn installed_ref_requests<'a>(
    data: &FrozenCompletion,
    references: &'a [InstalledViewRef],
    budget: &d::resources::ResourceBudget,
) -> Result<InstalledRefRequests<'a>, ModelError> {
    let completion = data.identity()?;
    let mut selectors = std::collections::BTreeMap::new();
    let mut targets = std::collections::BTreeMap::new();
    let mut requests =
        d::charged::StateCharge::new(budget, "compound selector exact observation requests");
    for reference in references {
        let _validation = inventory_typed_charge(
            std::mem::size_of::<InstalledViewRef>()
                .saturating_add(selector_heap(&reference.selector)),
            budget,
        )?;
        reference.validate()?;
        let selector = &reference.selector;
        if selector.parent_completion != completion
            || selector.generation != data.generation
            || selector.validity_epoch != data.validity_epoch
        {
            return Err(ModelError::Conflict(
                "compound selector exact parent/context",
            ));
        }
        let address = selector.record()?;
        requests.grow(crate::selection::node_bytes(&address).saturating_add(256))?;
        if selectors.insert(address, selector).is_some() {
            return Err(ModelError::Conflict("compound duplicate exact selector"));
        }
        for mapping in &selector.mappings {
            if let Some(previous) = targets.get(&mapping.physical) {
                if *previous != (mapping.incarnation, mapping.logical) {
                    return Err(ModelError::Conflict(
                        "compound selected physical mapping contradiction",
                    ));
                }
            } else {
                requests.grow(
                    std::mem::size_of::<crate::installed_view::ContributorRealization>()
                        .saturating_add(256),
                )?;
                targets.insert(mapping.physical, (mapping.incarnation, mapping.logical));
            }
        }
    }
    requests.grow(selectors.len().saturating_mul(std::mem::size_of::<(
        RecordId,
        &crate::installed_view::InstalledViewCompletion,
    )>()))?;
    let selectors = selectors.into_iter().collect::<Vec<_>>();
    requests.grow(
        targets
            .len()
            .saturating_mul(std::mem::size_of::<SelectorCopyRequirement>()),
    )?;
    let copies = targets
        .into_iter()
        .map(
            |(physical, (incarnation, logical))| SelectorCopyRequirement {
                physical,
                incarnation,
                logical,
            },
        )
        .collect();
    Ok(InstalledRefRequests {
        selectors,
        copies,
        _charge: requests,
    })
}
pub(crate) async fn verify_installed_refs(
    reader: &crate::NativeReader<()>,
    data: &FrozenCompletion,
    references: &[InstalledViewRef],
    budget: &d::resources::ResourceBudget,
) -> Result<(), ModelError> {
    prepare_installed_reference_verification(reader, data, references, budget)?.await
}
// Validate one exact reference inventory beneath the caller's existing hold.
// This boundary is shared by physical, frozen and published transport consumers.
fn prepare_installed_reference_verification<'a>(
    reader: &'a crate::NativeReader<()>,
    data: &'a FrozenCompletion,
    references: &'a [InstalledViewRef],
    budget: &'a d::resources::ResourceBudget,
) -> Result<futures::future::BoxFuture<'a, Result<(), ModelError>>, ModelError> {
    let body = async move {
        let parent = data.record()?;
        let completion = data.identity()?;
        let requests = installed_ref_requests(data, references, budget)?;
        let selectors = &requests.selectors;
        let mut offset = 0;
        let mut width = crate::loader::NATIVE_WINDOW_ROWS;
        while offset < selectors.len() {
            let count = width.min(selectors.len() - offset);
            let observed = async {
                let _page = if count > 1 {
                    Some(budget.reserve(
                        "compound selector observation page identities",
                        count.saturating_mul(1024),
                    )?)
                } else {
                    None
                };
                let page = &selectors[offset..offset + count];
                let ids = page.iter().map(|(id, _)| id.clone()).collect::<Vec<_>>();
                let mut actual =
                    crate::control::inspect_expected_window_read(reader, &ids, budget).await?;
                for (id, selector) in page {
                    let expected = actual.take_required(id)?;
                    if expected.row() != selector.value()? {
                        return Err(ModelError::Conflict("compound exact selector bytes"));
                    }
                }
                actual.finish()?;
                if count == 1 {
                    // Preserve the original scalar hold observation at the final width.
                    require_hold(reader, &parent, &page[0].0).await
                } else {
                    let pairs = ids
                        .into_iter()
                        .map(|id| (parent.clone(), id))
                        .collect::<Vec<_>>();
                    let mut holds =
                        crate::control::inspect_hold_window_read(reader, &pairs, budget).await?;
                    for (owner, object) in &pairs {
                        holds.take_required(owner, object)?;
                    }
                    holds.finish()
                }
            }
            .await;
            match observed {
                Err(ModelError::Resource { .. }) if count > 1 => {
                    width = (count / 2).max(1);
                }
                Err(error) => return Err(error),
                Ok(()) => {
                    offset += count;
                    width = crate::loader::NATIVE_WINDOW_ROWS;
                }
            }
        }
        let targets = &requests.copies;
        let mut offset = 0;
        let mut width = (crate::loader::NATIVE_WINDOW_ROWS / 2).max(1);
        while offset < targets.len() {
            let count = width.min(targets.len() - offset);
            // Keep the original scalar capacity/refusal route after a batch cannot fit.
            // There is no receiving effect or accepted partial selector to roll back.
            if count == 1 {
                verify_installed_copy(reader, data, &targets[offset], budget).await?;
                offset += 1;
                continue;
            }
            let observed = async {
                let _page = budget.reserve(
                    "compound selector observation page identities",
                    count.saturating_mul(2048),
                )?;
                let page = &targets[offset..offset + count];
                let mut ids = Vec::with_capacity(count * 2);
                let mut pairs = Vec::with_capacity(count * 2);
                for mapping in page {
                    for object in [
                        contributor_copy_record(completion, mapping.physical),
                        RecordId::new("compiler_contribution", mapping.physical.hex()),
                    ] {
                        pairs.push((parent.clone(), object.clone()));
                        ids.push(object);
                    }
                }
                let mut actual =
                    crate::control::inspect_expected_window_read(reader, &ids, budget).await?;
                for mapping in page {
                    let copy = actual
                        .take_required(&contributor_copy_record(completion, mapping.physical))?;
                    let copy = ContributorCopy::from_row(copy.row(), completion)?;
                    let physical = actual.take_required(&RecordId::new(
                        "compiler_contribution",
                        mapping.physical.hex(),
                    ))?;
                    verify_mapped_copy(mapping, &copy, physical.incarnation(), physical.row())?;
                }
                actual.finish()?;
                let mut holds =
                    crate::control::inspect_hold_window_read(reader, &pairs, budget).await?;
                for (owner, object) in &pairs {
                    holds.take_required(owner, object)?;
                }
                holds.finish()
            }
            .await;
            match observed {
                Err(ModelError::Resource { .. }) => {
                    width = (count / 2).max(1);
                }
                Err(error) => return Err(error),
                Ok(()) => {
                    offset += count;
                    width = (crate::loader::NATIVE_WINDOW_ROWS / 2).max(1);
                }
            }
        }
        Ok(())
    };
    crate::continuation::admit(
        budget,
        "native installed reference verification continuation",
        body,
    )
}

async fn verify_installed_copy(
    reader: &crate::NativeReader<()>,
    data: &FrozenCompletion,
    mapping: &SelectorCopyRequirement,
    budget: &d::resources::ResourceBudget,
) -> Result<(), ModelError> {
    let completion = data.identity()?;
    let copy_address = contributor_copy_record(completion, mapping.physical);
    let mut vars = surrealdb::types::Variables::new();
    vars.insert("copy", copy_address.clone());
    let copies: Vec<Value> = reader.query_native("SELECT * FROM $copy", vars).await?;
    let [copy] = copies.as_slice() else {
        return Err(ModelError::Conflict(
            "compound selected copy inventory cardinality",
        ));
    };
    let copy = ContributorCopy::from_row(copy, completion)?;
    require_hold(reader, &data.record()?, &copy_address).await?;
    let physical = RecordId::new("compiler_contribution", mapping.physical.hex());
    let actual = crate::control::inspect_expected_read(reader, physical.clone(), budget)
        .await?
        .ok_or(ModelError::Conflict("compound mapped contributor missing"))?;
    verify_mapped_copy(mapping, &copy, actual.incarnation(), actual.row())?;
    require_hold(reader, &data.record()?, &physical).await
}
fn verify_mapped_copy(
    mapping: &SelectorCopyRequirement,
    copy: &ContributorCopy,
    incarnation: i64,
    row: &Value,
) -> Result<(), ModelError> {
    if copy.physical != mapping.physical
        || copy.incarnation != mapping.incarnation
        || copy.descriptor.identity()? != mapping.logical
    {
        return Err(ModelError::Conflict(
            "compound exact selected copy incidence",
        ));
    }
    crate::completed_rows::validate_state_row("compiler_contribution", row)?;
    if incarnation != copy.incarnation
        || crate::installed_view::audit::descriptor::<d::completed::CompletedContribution>(row)?
            != copy.descriptor
    {
        return Err(ModelError::Conflict(
            "compound mapped full descriptor/incarnation",
        ));
    }
    Ok(())
}
fn contributor_copy_record(completion: ContentHash, physical: ContentHash) -> RecordId {
    let mut sink = KeySink::new("native-completed-contributor/v2");
    completion.encode(&mut sink);
    physical.encode(&mut sink);
    RecordId::new("native_completed_contributor", sink.finish().hex())
}
/// Installed only by the format coordinator together with target codecs and recovery.
pub fn schema_target() -> &'static str {
    "DEFINE TABLE native_completed_admission SCHEMAFULL; DEFINE FIELD descriptor ON native_completed_admission TYPE bytes; DEFINE FIELD completed_admission ON native_attempt TYPE option<record<native_completed_admission>>; DEFINE TABLE native_completed_state SCHEMAFULL; DEFINE FIELD descriptor ON native_completed_state TYPE bytes; DEFINE FIELD completed ON native_completed_state TYPE bool; DEFINE TABLE native_completed_view SCHEMAFULL; DEFINE FIELD completion ON native_completed_view TYPE record<native_completed_state>; DEFINE FIELD logical_view ON native_completed_view TYPE string; DEFINE FIELD descriptor ON native_completed_view TYPE bytes; DEFINE INDEX completion_view ON native_completed_view FIELDS completion,logical_view UNIQUE; DEFINE TABLE native_completed_contributor SCHEMAFULL; DEFINE FIELD completion ON native_completed_contributor TYPE record<native_completed_state>; DEFINE FIELD physical ON native_completed_contributor TYPE string; DEFINE FIELD descriptor ON native_completed_contributor TYPE bytes; DEFINE INDEX completion_contributor ON native_completed_contributor FIELDS completion,physical UNIQUE;"
}

pub struct InstalledViewInventory {
    references: Vec<InstalledViewRef>,
    copies: std::collections::BTreeMap<ContentHash, ContributorCopy>,
    bindings: Vec<d::completed::CompletedBinding>,
    binding_incarnations: std::collections::BTreeMap<ContentHash, i64>,
    _charge: d::charged::StateCharge,
}
impl InstalledViewInventory {
    /// Check actual retained backing ownership before borrowing it in an operation.
    pub fn require_pool(&self, budget: &d::resources::ResourceBudget) -> Result<(), ModelError> {
        if self
            ._charge
            .budget()
            .is_some_and(|owner| owner.shares_pool(budget))
        {
            Ok(())
        } else {
            Err(ModelError::Conflict(
                "completed inventory foreign resource pool",
            ))
        }
    }
    fn new(budget: &d::resources::ResourceBudget) -> Result<Self, ModelError> {
        let mut charge =
            d::charged::StateCharge::new(budget, "exact completed installed inventory");
        charge.grow(std::mem::size_of::<Self>() + 2 * std::mem::size_of::<usize>())?;
        Ok(Self {
            references: Vec::new(),
            copies: std::collections::BTreeMap::new(),
            bindings: Vec::new(),
            binding_incarnations: std::collections::BTreeMap::new(),
            _charge: charge,
        })
    }
    fn retain_reference(&mut self, reference: InstalledViewRef) -> Result<(), ModelError> {
        let additional = vector_capacity_growth(&self.references);
        self._charge.grow(
            additional
                .saturating_mul(std::mem::size_of::<InstalledViewRef>())
                .saturating_add(selector_heap(&reference.selector)),
        )?;
        self.references.reserve_exact(additional);
        self.references.push(reference);
        Ok(())
    }
    fn retain_copy(&mut self, copy: ContributorCopy) -> Result<(), ModelError> {
        if self.copies.contains_key(&copy.physical) {
            return Err(ModelError::Conflict(
                "completed duplicate physical copy inventory",
            ));
        }
        self._charge.grow(
            std::mem::size_of::<(ContentHash, ContributorCopy)>()
                .saturating_add(32)
                .saturating_add(crate::installed_view::audit::contribution_heap(
                    &copy.descriptor,
                )),
        )?;
        self.copies.insert(copy.physical, copy);
        Ok(())
    }
    fn retain_binding(
        &mut self,
        binding: d::completed::CompletedBinding,
        incarnation: i64,
    ) -> Result<(), ModelError> {
        let identity = binding.descriptor_identity()?;
        if self.binding_incarnations.contains_key(&identity) {
            return Err(ModelError::Conflict(
                "completed duplicate canonical binding incidence",
            ));
        }
        let additional = vector_capacity_growth(&self.bindings);
        self._charge.grow(
            additional
                .saturating_mul(std::mem::size_of::<d::completed::CompletedBinding>())
                .saturating_add(binding_heap(&binding))
                .saturating_add(std::mem::size_of::<(ContentHash, i64)>() + 32),
        )?;
        self.bindings.reserve_exact(additional);
        self.binding_incarnations.insert(identity, incarnation);
        self.bindings.push(binding);
        Ok(())
    }
    pub fn references(&self) -> &[InstalledViewRef] {
        &self.references
    }
    pub fn copies(&self) -> &std::collections::BTreeMap<ContentHash, ContributorCopy> {
        &self.copies
    }
    pub fn bindings(&self) -> &[d::completed::CompletedBinding] {
        &self.bindings
    }
    pub fn binding_incarnation(&self, binding: ContentHash) -> Result<i64, ModelError> {
        self.binding_incarnations
            .get(&binding)
            .copied()
            .ok_or(ModelError::Conflict(
                "canonical binding absent from completed inventory",
            ))
    }
    pub fn view(&self, logical: ContentHash) -> Result<&InstalledViewRef, ModelError> {
        self.references
            .binary_search_by_key(&logical, |row| row.logical_view)
            .map(|index| &self.references[index])
            .map_err(|_| ModelError::Conflict("view absent from exact completed inventory"))
    }
    pub fn contributors(
        &self,
        logical: ContentHash,
    ) -> Result<&[crate::installed_view::ContributorRealization], ModelError> {
        Ok(&self.installed(logical)?.mappings)
    }
    pub fn installed(
        &self,
        logical: ContentHash,
    ) -> Result<&crate::installed_view::InstalledViewCompletion, ModelError> {
        self.references
            .binary_search_by_key(&logical, |reference| reference.logical_view)
            .map(|index| &self.references[index].selector)
            .map_err(|_| {
                ModelError::Conflict("installed view absent from exact completed inventory")
            })
    }
}
fn vector_capacity_growth<T>(values: &Vec<T>) -> usize {
    if values.len() == values.capacity() {
        values.capacity().max(4)
    } else {
        0
    }
}
fn completed_view_heap(view: &d::completed::CompletedView) -> usize {
    use d::HeapSize;
    view.relation
        .heap_bytes()
        .saturating_add(view.contributions.heap_bytes())
}
fn selector_heap(selector: &crate::installed_view::InstalledViewCompletion) -> usize {
    std::mem::size_of::<crate::installed_view::SelectedViewDescriptor>()
        .saturating_add(2 * std::mem::size_of::<usize>())
        .saturating_add(completed_view_heap(&selector.view))
        .saturating_add(
            selector
                .mappings
                .capacity()
                .saturating_mul(std::mem::size_of::<
                    crate::installed_view::ContributorRealization,
                >()),
        )
}
fn binding_heap(binding: &d::completed::CompletedBinding) -> usize {
    use d::HeapSize;
    binding
        .boundary
        .heap_bytes()
        .saturating_add(binding.source.heap_bytes())
        .saturating_add(completed_view_heap(&binding.view))
}
/// Deserialization, validation, exact-envelope reconstruction and hashing need
/// scratch while a physical row is live; none of that workspace is retained by
/// the completed inventory after its typed value has been admitted.
fn inventory_decode_charge(
    row: &Value,
    budget: &d::resources::ResourceBudget,
) -> Result<Box<dyn d::resources::Reservation>, ModelError> {
    budget.reserve(
        "exact completed inventory row decode",
        crate::loader::native_bytes(row)
            .saturating_mul(32)
            .saturating_add(2048),
    )
}
/// Typed validation may clone a view's set/relation; selector addressing also
/// clones the full mappings, and hashing/serialization may coexist with both.
/// Keep the established conservative workspace allowance, sized from the typed
/// containers and scoped to the actual operation rather than retained as data.
fn inventory_typed_charge(
    typed_bytes: usize,
    budget: &d::resources::ResourceBudget,
) -> Result<Box<dyn d::resources::Reservation>, ModelError> {
    budget.reserve(
        "exact completed inventory typed validation",
        typed_bytes.saturating_mul(32).saturating_add(4096),
    )
}
pub(crate) async fn inspect_inventory(
    reader: &crate::NativeReader<()>,
    data: &FrozenCompletion,
    budget: &d::resources::ResourceBudget,
) -> Result<std::sync::Arc<InstalledViewInventory>, ModelError> {
    let parent = data.record()?;
    let mut result = InstalledViewInventory::new(budget)?;
    let mut variables = surrealdb::types::Variables::new();
    // `$parent` is row-scoped in the pinned planner, even when client-bound.
    // Use an ordinary bind name so exact owner equality can become an index prefix.
    variables.insert("compound_parent", parent.clone());
    let _window = budget.reserve(
        "compound exact inventory pointer windows",
        crate::loader::NATIVE_WINDOW_ROWS.saturating_mul(2048),
    )?;
    let mut copies=reader.stream_prepared_observed(crate::prepared::PreparedQuery::new(variables.clone(),vec![],vec!["SELECT id,physical FROM native_completed_contributor WITH INDEX completion_contributor WHERE completion=$compound_parent ORDER BY physical,id".into()])?,Some("compound_contributor_pointers"),None)?;
    let observed = async {
        let mut hasher = ContributorHasher::default();
        let mut previous = None;
        let mut window = Vec::with_capacity(crate::loader::NATIVE_WINDOW_ROWS / 2);
        while let Some(row) = copies.next().await? {
            let pointer = InventoryPointer::from_row(&row, "physical", previous.as_ref())?;
            previous = Some(pointer.key.clone());
            window.push(pointer);
            if window.len() >= crate::loader::NATIVE_WINDOW_ROWS / 2 {
                audit_copy_window(reader, data, &window, &mut result, &mut hasher, budget).await?;
                window.clear();
            }
        }
        if !window.is_empty() {
            audit_copy_window(reader, data, &window, &mut result, &mut hasher, budget).await?;
        }
        if hasher.finish() != data.contributor_inventory {
            return Err(ModelError::Conflict(
                "compound contributor complete EOF/inventory",
            ));
        }
        Ok(())
    }
    .await;
    let mut terminal = d::completion::Completion::default();
    terminal.step(
        "compound contributor inventory EOF/drain",
        copies.drain_transport().await,
    );
    d::completion::complete(observed, terminal)?;
    let mut selectors=reader.stream_prepared_observed(crate::prepared::PreparedQuery::new(variables.clone(),vec![],vec!["SELECT id,logical_view FROM native_completed_view WITH INDEX completion_view WHERE completion=$compound_parent ORDER BY logical_view,id".into()])?,Some("compound_selector_pointers"),None)?;
    let observed = async {
        let mut hasher = RealizationHasher::default();
        let mut previous = None;
        let mut window = Vec::with_capacity(crate::loader::NATIVE_WINDOW_ROWS / 2);
        while let Some(row) = selectors.next().await? {
            let pointer = InventoryPointer::from_row(&row, "logical_view", previous.as_ref())?;
            previous = Some(pointer.key.clone());
            window.push(pointer);
            if window.len() >= crate::loader::NATIVE_WINDOW_ROWS / 2 {
                audit_selector_window(reader, data, &window, &mut result, &mut hasher, budget)
                    .await?;
                window.clear();
            }
        }
        if !window.is_empty() {
            audit_selector_window(reader, data, &window, &mut result, &mut hasher, budget).await?;
        }
        if hasher.finish() != data.realization_inventory {
            return Err(ModelError::Conflict(
                "exact completed inventory actual EOF/digest",
            ));
        }
        Ok(())
    }
    .await;
    let mut terminal = d::completion::Completion::default();
    terminal.step(
        "compound selector inventory EOF/drain",
        selectors.drain_transport().await,
    );
    d::completion::complete(observed, terminal)?;
    let _selected = budget.reserve(
        "compound selected exact physical incidence",
        result
            .references
            .iter()
            .map(|reference| reference.selector.mappings.len())
            .sum::<usize>()
            .saturating_mul(128)
            .saturating_add(1024),
    )?;
    let selected = result
        .references
        .iter()
        .flat_map(|reference| {
            reference
                .selector
                .mappings
                .iter()
                .map(|mapping| mapping.physical)
        })
        .collect::<std::collections::BTreeSet<_>>();
    if !selected.iter().eq(result.copies.keys()) {
        return Err(ModelError::Conflict(
            "compound contributor inventory extra/missing selector incidence",
        ));
    }
    let mut bindings = reader.stream_prepared_observed(
        crate::prepared::PreparedQuery::new(
            variables,
            vec![],
            vec![COMPOUND_BINDING_POINTERS_SQL.into()],
        )?,
        Some("compound_binding_pointers"),
        None,
    )?;
    let observed = async {
        let mut window = Vec::with_capacity(crate::loader::NATIVE_WINDOW_ROWS);
        let mut previous = None;
        while let Some(row) = bindings.next().await? {
            let Some(Value::RecordId(id)) = row.as_object().and_then(|row| row.get("id")) else {
                return Err(ModelError::Schema("compound binding pointer"));
            };
            if previous.as_ref().is_some_and(|previous| previous >= id) {
                return Err(ModelError::Conflict(
                    "compound binding pointer order/duplicate",
                ));
            }
            previous = Some(id.clone());
            window.push(id.clone());
            if window.len() >= crate::loader::NATIVE_WINDOW_ROWS {
                audit_binding_window(reader, &window, &mut result, budget).await?;
                window.clear();
            }
        }
        if !window.is_empty() {
            audit_binding_window(reader, &window, &mut result, budget).await?;
        }
        Ok(())
    }
    .await;
    let mut terminal = d::completion::Completion::default();
    terminal.step(
        "compound binding pointer EOF/drain",
        bindings.drain_transport().await,
    );
    d::completion::complete(observed, terminal)?;
    let binding_identity = {
        // The model stably orders borrowed pointers, then validates one binding
        // at a time. Admit its vector plus up to one element-array sort workspace.
        let largest = result
            .bindings
            .iter()
            .map(|binding| {
                std::mem::size_of::<d::completed::CompletedBinding>()
                    .saturating_add(binding_heap(binding))
            })
            .max()
            .unwrap_or(0);
        let _pointers = budget.reserve(
            "exact completed binding identity pointers",
            result
                .bindings
                .len()
                .saturating_mul(std::mem::size_of::<&d::completed::CompletedBinding>())
                .saturating_mul(2),
        )?;
        let _validation = inventory_typed_charge(largest, budget)?;
        d::completed::binding_inventory_identity(&result.bindings)?
    };
    if binding_identity != data.bindings {
        return Err(ModelError::Conflict(
            "completed canonical binding actual EOF/inventory",
        ));
    }
    // The identity check already refused duplicate keys; their relative order
    // needs no stability and in-place sorting needs no second binding array.
    result
        .bindings
        .sort_unstable_by_key(|binding| binding.key());
    audit_parent_holds(reader, data, &result, budget).await?;
    Ok(std::sync::Arc::new(result))
}
pub(crate) fn validate_selector_copies(
    data: &FrozenCompletion,
    selector: &crate::installed_view::InstalledViewCompletion,
    copies: &std::collections::BTreeMap<ContentHash, ContributorCopy>,
) -> Result<(), ModelError> {
    selector.validate()?;
    if selector.parent_completion != data.identity()?
        || selector.generation != data.generation
        || selector.validity_epoch != data.validity_epoch
    {
        return Err(ModelError::Conflict(
            "compound selector exact parent/context",
        ));
    }
    for mapping in &selector.mappings {
        let copy = copies
            .get(&mapping.physical)
            .ok_or(ModelError::Conflict("compound selector exact copy missing"))?;
        if copy.incarnation != mapping.incarnation || copy.descriptor.identity()? != mapping.logical
        {
            return Err(ModelError::Conflict(
                "compound selector exact copy descriptor/incarnation",
            ));
        }
    }
    Ok(())
}
/// Narrow discovery retains stream order; exact guarded children establish meaning.
struct InventoryPointer {
    id: RecordId,
    key: String,
}
impl InventoryPointer {
    fn from_row(row: &Value, key: &str, previous: Option<&String>) -> Result<Self, ModelError> {
        let object = row
            .as_object()
            .ok_or(ModelError::Schema("compound inventory pointer"))?;
        let (Some(Value::RecordId(id)), Some(Value::String(value))) =
            (object.get("id"), object.get(key))
        else {
            return Err(ModelError::Schema("compound inventory pointer fields"));
        };
        let table = match key {
            "physical" => "native_completed_contributor",
            "logical_view" => "native_completed_view",
            _ => return Err(ModelError::Schema("compound inventory pointer key")),
        };
        let hex = |value: &str| {
            value.len() == 64
                && value
                    .bytes()
                    .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        };
        let surrealdb::types::RecordIdKey::String(id_key) = &id.key else {
            return Err(ModelError::Schema("compound inventory pointer identity"));
        };
        if object.len() != 2
            || id.table.as_str() != table
            || !hex(id_key)
            || !hex(value)
            || previous.is_some_and(|previous| previous >= value)
        {
            return Err(ModelError::Conflict(
                "compound inventory pointer envelope/order/duplicate",
            ));
        }
        Ok(Self {
            id: id.clone(),
            key: value.clone(),
        })
    }
    fn copy(&self, row: &Value, completion: ContentHash) -> Result<ContributorCopy, ModelError> {
        let copy = ContributorCopy::from_row(row, completion)?;
        if self.id != copy.record(completion) || self.key != copy.physical.hex() {
            return Err(ModelError::Conflict("compound copy pointer exact child"));
        }
        Ok(copy)
    }
    fn selector(
        &self,
        row: &Value,
        completion: ContentHash,
    ) -> Result<InstalledViewRef, ModelError> {
        let reference = InstalledViewRef::from_row(row, completion)?;
        if self.id != reference.record(completion) || self.key != reference.logical_view.hex() {
            return Err(ModelError::Conflict(
                "compound selector pointer exact child",
            ));
        }
        Ok(reference)
    }
}
async fn audit_copy_window(
    reader: &crate::NativeReader<()>,
    data: &FrozenCompletion,
    pointers: &[InventoryPointer],
    inventory: &mut InstalledViewInventory,
    hasher: &mut ContributorHasher,
    budget: &d::resources::ResourceBudget,
) -> Result<(), ModelError> {
    let identity = data.identity()?;
    let mut ids = Vec::with_capacity(pointers.len() * 2);
    for pointer in pointers {
        ids.push(pointer.id.clone());
        ids.push(RecordId::new("compiler_contribution", pointer.key.clone()));
    }
    let mut actual = crate::control::held::inspect_expected_window_read_observed(
        reader,
        &ids,
        budget,
        Some([
            "compound_contributor_records",
            "compound_contributor_guards",
        ]),
    )
    .await?;
    for pointer in pointers {
        let child = actual.take_required(&pointer.id)?;
        let _child_decode = inventory_decode_charge(child.row(), budget)?;
        let copy = pointer.copy(child.row(), identity)?;
        let physical = RecordId::new("compiler_contribution", copy.physical.hex());
        let expected = actual.take_required(&physical)?;
        let _physical_decode = inventory_decode_charge(expected.row(), budget)?;
        crate::completed_rows::validate_state_row("compiler_contribution", expected.row())?;
        if expected.incarnation() != copy.incarnation
            || crate::installed_view::audit::descriptor::<d::completed::CompletedContribution>(
                expected.row(),
            )? != copy.descriptor
        {
            return Err(ModelError::Conflict(
                "compound contributor actual full descriptor/incarnation",
            ));
        }
        hasher.row(&copy)?;
        inventory.retain_copy(copy)?;
    }
    actual.finish()
}
async fn audit_selector_window(
    reader: &crate::NativeReader<()>,
    data: &FrozenCompletion,
    pointers: &[InventoryPointer],
    inventory: &mut InstalledViewInventory,
    hasher: &mut RealizationHasher,
    budget: &d::resources::ResourceBudget,
) -> Result<(), ModelError> {
    let identity = data.identity()?;
    let mut ids = Vec::with_capacity(pointers.len() * 2);
    for pointer in pointers {
        ids.push(pointer.id.clone());
        ids.push(RecordId::new("compiler_view", pointer.key.clone()));
    }
    let mut actual = crate::control::held::inspect_expected_window_read_observed(
        reader,
        &ids,
        budget,
        Some(["compound_selector_records", "compound_selector_guards"]),
    )
    .await?;
    for pointer in pointers {
        let selector = actual.take_required(&pointer.id)?;
        let _selector_decode = inventory_decode_charge(selector.row(), budget)?;
        let reference = pointer.selector(selector.row(), identity)?;
        let semantic = actual.take_required(&RecordId::new(
            "compiler_view",
            reference.logical_view.hex(),
        ))?;
        let _semantic_decode = inventory_decode_charge(semantic.row(), budget)?;
        crate::completed_rows::validate_state_row("compiler_view", semantic.row())?;
        if crate::installed_view::audit::descriptor::<d::completed::CompletedView>(semantic.row())?
            != reference.selector.view
        {
            return Err(ModelError::Conflict(
                "compound semantic view descriptor changed",
            ));
        }
        validate_selector_copies(data, &reference.selector, &inventory.copies)?;
        hasher.row(&reference)?;
        inventory.retain_reference(reference)?;
    }
    actual.finish()
}
async fn audit_binding_window(
    reader: &crate::NativeReader<()>,
    ids: &[RecordId],
    inventory: &mut InstalledViewInventory,
    budget: &d::resources::ResourceBudget,
) -> Result<(), ModelError> {
    let mut actual = crate::control::held::inspect_expected_window_read_observed(
        reader,
        ids,
        budget,
        Some(["compound_binding_records", "compound_binding_guards"]),
    )
    .await?;
    let mut pairs = Vec::with_capacity(ids.len());
    for id in ids {
        let expected = actual.take_required(id)?;
        let _decode = inventory_decode_charge(expected.row(), budget)?;
        let binding = crate::binding_descriptor::decode(expected.row())?;
        if inventory.installed(binding.view.identity)?.view != binding.view {
            return Err(ModelError::Conflict(
                "completed canonical binding exact installed view",
            ));
        }
        pairs.push((
            id.clone(),
            RecordId::new("compiler_view", binding.view.identity.hex()),
        ));
        inventory.retain_binding(binding, expected.incarnation())?;
    }
    actual.finish()?;
    let mut holds = crate::control::held::inspect_hold_window_read_observed(
        reader,
        &pairs,
        budget,
        Some("compound_binding_holds"),
    )
    .await?;
    for (owner, object) in pairs {
        holds.take_required(&owner, &object)?;
    }
    holds.finish()
}

/// Physical completion precedes semantic admission. This private-field owner can only
/// be earned by the independent native completion writer under a durable frozen fence.
pub struct NativeCompletedState {
    // Actual source provenance, never reconstructed from serialized identities.
    pub(crate) client: std::sync::Arc<surrealdb::Surreal<surrealdb::engine::remote::grpc::Client>>,
    pub(crate) budget: d::resources::ResourceBudget,
    pub(crate) completion: FrozenCompletion,
    pub(crate) held: crate::control::HeldRecord,
    pub(crate) inventory: std::sync::Arc<InstalledViewInventory>,
    pub(crate) _charge: d::charged::StateCharge,
}
/// Re-hold protected physical completion without granting artifact admission.
/// Used by lifecycle operations on a frozen producer, including warm-only targets.
pub(crate) async fn hold_physical_state_for_attempt(
    client: std::sync::Arc<surrealdb::Surreal<surrealdb::engine::remote::grpc::Client>>,
    attempt: ContentHash,
    budget: &d::resources::ResourceBudget,
) -> Result<NativeCompletedState, ModelError> {
    let reader = crate::NativeReader::private(client.clone()).with_budget(budget);
    let result = async {
        let _delivery = budget.reserve(
            "frozen physical owner delivery",
            d::resources::MAX_ROW_BYTES,
        )?;
        let mut variables = surrealdb::types::Variables::new();
        variables.insert(
            "physical_attempt",
            RecordId::new("native_attempt", attempt.hex()),
        );
        let owners: Vec<Object> = reader
            .query_native("SELECT * FROM $physical_attempt", variables)
            .await?;
        let [owner] = owners.as_slice() else {
            return Err(ModelError::Conflict(
                "physical lifecycle attempt cardinality",
            ));
        };
        if owner.get("state") != Some(&Value::String("frozen".into())) {
            return Err(ModelError::Conflict(
                "physical lifecycle requires frozen producer",
            ));
        }
        let Some(Value::String(identity)) = owner.get("completed_identity") else {
            return Err(ModelError::Conflict(
                "frozen lifecycle has no physical completion",
            ));
        };
        let expected = crate::control::inspect_expected_read(
            &reader,
            RecordId::new("native_completed_state", identity.clone()),
            budget,
        )
        .await?
        .ok_or(ModelError::Conflict(
            "frozen lifecycle physical completion absent",
        ))?;
        let decode = budget.reserve(
            "frozen lifecycle physical completion decode",
            crate::loader::native_bytes(expected.row())
                .saturating_mul(32)
                .saturating_add(65536),
        )?;
        let data = FrozenCompletion::from_row(expected.row(), true)?;
        if data.identity()?.hex() != *identity || data.attempt != attempt {
            return Err(ModelError::Conflict(
                "frozen lifecycle physical owner identity",
            ));
        }
        drop(decode);
        let held =
            crate::control::hold_frozen_physical_expected(&client, &data, expected, budget).await?;
        let inventory = inspect_inventory(&reader, &data, budget).await?;
        // Repeat the exact freeze/epoch/bytes CAS after the complete inventory EOF.
        let expected = crate::control::inspect_expected_read(&reader, data.record()?, budget)
            .await?
            .ok_or(ModelError::Conflict(
                "frozen lifecycle completion disappeared",
            ))?;
        if expected.incarnation() != held.incarnation() {
            return Err(ModelError::Conflict("frozen lifecycle completion ABA"));
        }
        #[cfg(test)]
        if let Ok(barrier) = FINAL_PARENT_OBSERVATION_BARRIER.try_with(Clone::clone) {
            barrier.observed.wait().await;
            barrier.resume.wait().await;
        }
        let repeated =
            crate::control::hold_frozen_physical_expected(&client, &data, expected, budget).await?;
        drop(held);
        let mut charge = d::charged::StateCharge::new(budget, "frozen lifecycle physical owner");
        charge.grow(
            std::mem::size_of::<NativeCompletedState>()
                .saturating_add(2 * std::mem::size_of::<usize>())
                .saturating_add(crate::selection::node_bytes(repeated.owner())),
        )?;
        Ok(NativeCompletedState {
            client: client.clone(),
            budget: budget.clone(),
            completion: data,
            held: repeated,
            inventory,
            _charge: charge,
        })
    }
    .await;
    let mut terminal = d::completion::Completion::default();
    terminal.step("frozen lifecycle reader close", reader.close().await);
    d::completion::complete(result, terminal)
}
impl NativeCompletedState {
    pub fn completion(&self) -> &FrozenCompletion {
        &self.completion
    }
    pub fn incarnation(&self) -> i64 {
        self.held.incarnation()
    }
    pub fn inventory(&self) -> &InstalledViewInventory {
        &self.inventory
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CompletedAdmission {
    pub version: u32,
    pub completion: ContentHash,
    pub completion_incarnation: i64,
    pub generation: ContentHash,
    pub validity_epoch: i64,
    pub evidence: d::artifact_admission::ArtifactAdmissionEvidence,
}
/// Retained typed admission backing, independent of serialized envelope and
/// decoding/validation workspace. ExpectedRecord owns its own native bytes.
fn admission_heap(admission: &CompletedAdmission) -> usize {
    fn vector<T>(values: &Vec<T>) -> usize {
        values.capacity().saturating_mul(std::mem::size_of::<T>())
    }
    fn execution(values: &Vec<d::artifact_admission::ValidationExecution>) -> usize {
        use d::HeapSize;
        vector(values).saturating_add(
            values
                .iter()
                .map(|value| {
                    value
                        .name
                        .capacity()
                        .saturating_add(vector(&value.inputs))
                        .saturating_add(
                            value
                                .inputs
                                .iter()
                                .map(|input| match input {
                                    d::artifact_admission::SelectedInput::Completed {
                                        source,
                                        ..
                                    } => source.heap_bytes(),
                                    d::artifact_admission::SelectedInput::NotRequested {
                                        ..
                                    }
                                    | d::artifact_admission::SelectedInput::EmptyDomain => 0,
                                })
                                .sum::<usize>(),
                        )
                })
                .sum::<usize>(),
        )
    }
    let manifest = &admission.evidence.manifest;
    vector(&manifest.captures)
        .saturating_add(vector(&manifest.producers))
        .saturating_add(
            manifest
                .producers
                .iter()
                .map(|producer| producer.producer.capacity())
                .sum::<usize>(),
        )
        .saturating_add(vector(&manifest.families))
        .saturating_add(vector(&manifest.required_outcomes))
        .saturating_add(
            manifest
                .required_outcomes
                .iter()
                .map(|outcome| outcome.producer.capacity())
                .sum::<usize>(),
        )
        .saturating_add(vector(&manifest.outcomes))
        .saturating_add(
            manifest
                .outcomes
                .iter()
                .map(|outcome| {
                    outcome
                        .key
                        .producer
                        .capacity()
                        .saturating_add(outcome.detail.as_ref().map_or(0, String::capacity))
                })
                .sum::<usize>(),
        )
        .saturating_add(vector(&manifest.originals))
        .saturating_add(vector(&manifest.projections))
        .saturating_add(
            manifest
                .projections
                .iter()
                .map(|projection| {
                    projection
                        .name
                        .capacity()
                        .saturating_add(vector(&projection.declared_losses))
                        .saturating_add(
                            projection
                                .declared_losses
                                .iter()
                                .map(String::capacity)
                                .sum::<usize>(),
                        )
                })
                .sum::<usize>(),
        )
        .saturating_add(vector(&manifest.embeddings))
        .saturating_add(execution(&admission.evidence.invariants))
        .saturating_add(execution(&admission.evidence.frontier))
}
impl CompletedAdmission {
    pub(crate) fn validate(
        &self,
        data: &FrozenCompletion,
        incarnation: i64,
        model: &d::ValidatedModel,
        inventory: &InstalledViewInventory,
        budget: &d::resources::ResourceBudget,
    ) -> Result<(), ModelError> {
        if self.version != 2
            || self.completion != data.identity()?
            || self.completion_incarnation != incarnation
            || incarnation <= 0
            || self.generation != data.generation
            || self.validity_epoch != data.validity_epoch
        {
            return Err(ModelError::Conflict(
                "semantic admission exact physical completion/epoch",
            ));
        }
        self.evidence
            .validate_current(model, inventory.bindings(), &data.roots, budget)
    }
    pub fn identity(&self) -> Result<ContentHash, ModelError> {
        let mut sink = KeySink::new("native-completed-admission/v2");
        sink.part(
            b"canonical",
            &serde_json::to_vec(self).map_err(ModelError::codec)?,
        );
        Ok(sink.finish())
    }
    pub fn record(&self) -> Result<RecordId, ModelError> {
        Ok(RecordId::new(
            "native_completed_admission",
            self.identity()?.hex(),
        ))
    }
    pub fn value(&self) -> Result<Value, ModelError> {
        let mut row = Object::new();
        row.insert("id", self.record()?);
        row.insert(
            "descriptor",
            surrealdb::types::Bytes::from(serde_json::to_vec(self).map_err(ModelError::codec)?),
        );
        Ok(Value::Object(row))
    }
    pub(crate) fn from_row(row: &Value) -> Result<Self, ModelError> {
        let Some(Value::Bytes(bytes)) = row.as_object().and_then(|row| row.get("descriptor"))
        else {
            return Err(ModelError::Schema("native completed admission descriptor"));
        };
        let data: Self = serde_json::from_slice(bytes).map_err(ModelError::codec)?;
        if data.value()? != *row {
            return Err(ModelError::Conflict(
                "native completed admission canonical envelope",
            ));
        }
        Ok(data)
    }
}

#[derive(Clone)]
pub struct PublishedCompletedState {
    inner: std::sync::Arc<PublishedCompletedInner>,
}
// A publication retains the exact reader's pin/session/resource ownership. Keep
// that execution owner behind a private capability: installed selectors can retain
// publications whose readers themselves retain installed selectors.
trait PublishedReadRetention: Send + Sync {
    fn reader(&self) -> &crate::NativeReader<d::serving::SnapshotHandle>;
}
struct RetainedPublishedReader(crate::NativeReader<d::serving::SnapshotHandle>);
impl PublishedReadRetention for RetainedPublishedReader {
    fn reader(&self) -> &crate::NativeReader<d::serving::SnapshotHandle> {
        &self.0
    }
}
struct PublishedCompletedInner {
    completion: FrozenCompletion,
    completion_incarnation: i64,
    admission: CompletedAdmission,
    admission_incarnation: i64,
    inventory: std::sync::Arc<InstalledViewInventory>,
    marker: crate::reader::ProtectedPublicationMarker,
    // This is a retained borrowed scope/pin, never a separately owned session to close.
    reader: Box<dyn PublishedReadRetention>,
    _charge: Box<dyn d::resources::Reservation>,
}
impl PublishedCompletedState {
    pub(crate) fn shared_inventory(&self) -> std::sync::Arc<InstalledViewInventory> {
        self.inner.inventory.clone()
    }
    pub fn completion(&self) -> &FrozenCompletion {
        &self.inner.completion
    }
    pub fn incarnation(&self) -> i64 {
        self.inner.completion_incarnation
    }
    pub fn admission_record(&self) -> Result<RecordId, ModelError> {
        self.inner.admission.record()
    }
    pub fn admission_incarnation(&self) -> i64 {
        self.inner.admission_incarnation
    }
    pub fn inventory(&self) -> &InstalledViewInventory {
        &self.inner.inventory
    }
    /// Require this protected inventory to belong to the borrowing native session.
    pub fn require_session(
        &self,
        client: &surrealdb::Surreal<surrealdb::engine::remote::grpc::Client>,
    ) -> Result<(), ModelError> {
        if !std::ptr::eq(client, self.retained_reader().client()) {
            return Err(ModelError::Conflict(
                "published inventory foreign native session",
            ));
        }
        Ok(())
    }
    /// Capability identity includes the retained publication marker and its pin scope.
    pub fn same_capability(&self, other: &Self) -> bool {
        std::sync::Arc::ptr_eq(&self.inner, &other.inner)
    }
    pub(crate) fn marker(&self) -> &Object {
        self.inner.marker.row()
    }
    pub(crate) fn admission(&self) -> &CompletedAdmission {
        &self.inner.admission
    }
    pub(crate) fn retained_reader(&self) -> &crate::NativeReader<d::serving::SnapshotHandle> {
        self.inner.reader.reader()
    }
}
/// Marker/pin reachability and persisted semantic admission survive legitimate origin
/// history compaction. No origin attempt lookup is made on this publication read path.
pub async fn published_completed_state(
    reader: &crate::NativeReader<d::serving::SnapshotHandle>,
    budget: &d::resources::ResourceBudget,
) -> Result<PublishedCompletedState, ModelError> {
    let recovery = reader.attached_recovery_scope(reader.handle().service_generation)?;
    crate::control::with_recovery_scope(
        recovery,
        prepare_published_completed_state(reader, budget)?,
    )
    .await
}
// Reader/serving callers share one complete protected preparation, without
// inheriting its inventory/eligibility/query execution future graph.
fn prepare_published_completed_state<'a>(
    reader: &'a crate::NativeReader<d::serving::SnapshotHandle>,
    budget: &'a d::resources::ResourceBudget,
) -> Result<futures::future::BoxFuture<'a, Result<PublishedCompletedState, ModelError>>, ModelError>
{
    let body = async move {
        let marker = reader.protected_publication_marker(budget).await?;
        let record = |field: &str, table: &str| -> Result<RecordId, ModelError> {
            match marker.row().get(field) {
                Some(Value::RecordId(id)) if id.table.as_str() == table => Ok(id.clone()),
                _ => Err(ModelError::Schema("published completed pointer")),
            }
        };
        let completion_id = record("completion", "native_completed_state")?;
        let admission_id = record("admission", "native_completed_admission")?;
        let transport = reader.transport_reader().with_budget(budget);
        let expected =
            crate::control::inspect_expected_read(&transport, completion_id.clone(), budget)
                .await?
                .ok_or(ModelError::Conflict(
                    "published physical completion missing",
                ))?;
        let _scratch = budget.reserve(
            "published completion decode",
            crate::loader::native_bytes(expected.row())
                .saturating_mul(32)
                .saturating_add(65536),
        )?;
        let data = FrozenCompletion::from_row(expected.row(), true)?;
        if data.record()? != completion_id
            || data.generation != reader.handle().service_generation
            || marker.row().get("completion_incarnation")
                != Some(&expected.incarnation().into_value())
        {
            return Err(ModelError::Conflict(
                "published physical completion identity/incarnation",
            ));
        }
        drop(_scratch);
        let admission =
            crate::control::inspect_expected_read(&transport, admission_id.clone(), budget)
                .await?
                .ok_or(ModelError::Conflict("published semantic admission missing"))?;
        let admission_decode = budget.reserve(
            "published semantic admission decode",
            crate::loader::native_bytes(admission.row())
                .saturating_mul(32)
                .saturating_add(65536),
        )?;
        let semantic = CompletedAdmission::from_row(admission.row())?;
        let model = d::model()?;
        if semantic.record()? != admission_id
            || marker.row().get("admission_incarnation")
                != Some(&admission.incarnation().into_value())
        {
            return Err(ModelError::Conflict(
                "published semantic admission identity/incarnation",
            ));
        }
        let charge = budget.reserve(
            "published completed retained typed data",
            std::mem::size_of::<PublishedCompletedInner>()
                .saturating_add(std::mem::size_of::<RetainedPublishedReader>())
                .saturating_add(2 * std::mem::size_of::<usize>())
                .saturating_add(admission_heap(&semantic)),
        )?;
        drop(admission_decode);
        require_hold(&transport, &admission_id, &completion_id).await?;
        let inventory = inspect_inventory(&transport, &data, budget).await?;
        {
            let _validation = inventory_typed_charge(
                std::mem::size_of::<CompletedAdmission>().saturating_add(admission_heap(&semantic)),
                budget,
            )?;
            semantic.validate(&data, expected.incarnation(), &model, &inventory, budget)?;
        }
        let repeated = reader.protected_publication_marker(budget).await?;
        if repeated.row() != marker.row() {
            return Err(ModelError::Conflict(
                "published marker changed during completed factory",
            ));
        }
        let result = PublishedCompletedState {
            inner: std::sync::Arc::new(PublishedCompletedInner {
                completion: data,
                completion_incarnation: expected.incarnation(),
                admission: semantic,
                admission_incarnation: admission.incarnation(),
                inventory,
                marker,
                reader: Box::new(RetainedPublishedReader(reader.clone())),
                _charge: charge,
            }),
        };
        if crate::completed_transport::eligibility_published(&result, budget).await?
            != reader.handle().eligibility
        {
            return Err(ModelError::Conflict("published exact selector eligibility"));
        }
        Ok(result)
    };
    // Admit before allocating the continuation or beginning native observations.
    // The same pool owns the continuation until all preparation checks finish;
    // the returned capability retains its own separately admitted typed backing.
    crate::continuation::admit(
        budget,
        "published completion preparation continuation",
        body,
    )
}

// Semantic execution belongs to compiler storage (ADR-0151). Native completion
// independently audits the full selected typed backing and physical holds; the read
// factories above then check the exact immutable persisted production evidence against
// current applicability, bindings and the complete six-family identity. There is no
// native DTO-to-semantic-admission constructor.

/// Independent temporary owner for an explicitly retained product boundary.
/// Receiving product effects must compare expected_record() before closing this pin.
pub struct NativeRetainedState {
    pub(crate) inner: NativeCompletedState,
    pub(crate) pin: std::sync::Arc<crate::control::ReaderPin>,
    pub(crate) closed: bool,
}
impl Drop for NativeRetainedState {
    fn drop(&mut self) {
        if !self.closed {
            self.pin.retain_unknown();
        }
    }
}
impl NativeRetainedState {
    pub fn completion(&self) -> &FrozenCompletion {
        self.inner.completion()
    }
    pub fn inventory(&self) -> &InstalledViewInventory {
        self.inner.inventory()
    }
    pub fn expected_record(&self) -> &crate::control::HeldRecord {
        &self.inner.held
    }
    pub fn incarnation(&self) -> i64 {
        self.inner.incarnation()
    }
    pub fn retain_unknown(&self) {
        self.pin.retain_unknown();
    }
    pub async fn close(mut self) -> Result<(), ModelError> {
        self.pin.release().await?;
        self.closed = true;
        Ok(())
    }
}

#[cfg(test)]
mod compound_controls {
    use super::*;
    fn contributor() -> d::completed::CompletedContribution {
        d::completed::CompletedContribution {
            spec: d::completed::ContributionSpec {
                captured_binding: None,
                producer: "p".into(),
                profile: d::completed::OutputProfile::Specific(d::stages::Profile::Catalog),
                model: ContentHash([1; 32]),
                implementation: ContentHash([2; 32]),
                configuration: None,
                inputs: vec![],
                outputs: ["packages".into()].into(),
            },
            assurance: d::producer_assurance::ProducerAssurance::CurrentProviderInterface,
            outcome: 0,
            outputs: [(
                "packages".into(),
                d::completed::OutputContent {
                    rows: 0,
                    content: ContentHash([3; 32]),
                },
            )]
            .into(),
        }
    }
    fn selector() -> InstalledViewRef {
        let logical = contributor().identity().unwrap();
        let view =
            d::completed::CompletedView::new("packages".into(), [logical].into(), 0).unwrap();
        InstalledViewRef {
            logical_view: view.identity,
            selector: crate::installed_view::InstalledViewCompletion {
                version: 2,
                parent_completion: ContentHash([0; 32]),
                generation: ContentHash([4; 32]),
                validity_epoch: 1,
                selection: std::sync::Arc::new(crate::installed_view::SelectedViewDescriptor {
                    mappings: vec![crate::installed_view::ContributorRealization {
                        view: view.identity,
                        logical,
                        physical: ContentHash([5; 32]),
                        incarnation: 1,
                    }],
                    view,
                }),
                member_codec: crate::installed_view::MEMBER_CODEC,
                membership_rows: 0,
                membership_content: ContentHash([6; 32]),
            },
        }
    }
    fn completion() -> FrozenCompletion {
        let roots = d::completed_identity::CompletedFamilyRoots::new(
            d::recovery_closure::COMPLETED_FAMILIES.map(|family| {
                d::completed_identity::CompletedFamilyHasher::new(family)
                    .unwrap()
                    .finish()
            }),
        )
        .unwrap();
        let mut selectors = RealizationHasher::default();
        selectors.row(&selector()).unwrap();
        let mut copies = ContributorHasher::default();
        copies
            .row(&ContributorCopy {
                physical: ContentHash([5; 32]),
                incarnation: 1,
                descriptor: contributor(),
            })
            .unwrap();
        FrozenCompletion {
            version: VERSION,
            generation: ContentHash([4; 32]),
            attempt: ContentHash([7; 32]),
            attempt_epoch: 1,
            completed_revision: 3,
            validity_epoch: 1,
            completed_state: roots.identity().unwrap(),
            roots,
            bindings: d::completed::binding_inventory_identity(&[]).unwrap(),
            realization_inventory: selectors.finish(),
            contributor_inventory: copies.finish(),
        }
    }
    #[test]
    fn selector_inventory_retains_one_typed_copy_and_releases_decode_work_per_row() {
        // This is a typed allocation/lookup control, not native receiving authority.
        // Its inventory crosses two complete 64-selector receiving windows.
        let budget = d::resources::ResourceBudget::fixed(16 << 20).unwrap();
        let parent = ContentHash([7; 32]);
        let logical = (0..128u8)
            .map(|index| ContentHash([index; 32]))
            .collect::<std::collections::BTreeSet<_>>();
        let mut fixtures = (0..129)
            .map(|index| {
                let mut reference = selector();
                std::sync::Arc::make_mut(&mut reference.selector.selection).view =
                    d::completed::CompletedView::new(
                        format!("selector_{index:03}"),
                        logical.clone(),
                        0,
                    )
                    .unwrap();
                reference.logical_view = reference.selector.view.identity;
                reference.selector.parent_completion = parent;
                std::sync::Arc::make_mut(&mut reference.selector.selection).mappings = logical
                    .iter()
                    .map(|logical| crate::installed_view::ContributorRealization {
                        view: reference.logical_view,
                        logical: *logical,
                        physical: *logical,
                        incarnation: 1,
                    })
                    .collect();
                reference
            })
            .collect::<Vec<_>>();
        fixtures.sort_by_key(|reference| reference.logical_view);
        let mut inventory = InstalledViewInventory::new(&budget).unwrap();
        let base = inventory._charge.reserved();
        let mut former_workspace = 0usize;
        let mut hasher = RealizationHasher::default();
        for fixture in fixtures {
            let row = fixture.value(parent).unwrap();
            let before = inventory._charge.reserved();
            {
                let decode = inventory_decode_charge(&row, &budget).unwrap();
                former_workspace = former_workspace.saturating_add(decode.size());
                let reference = InstalledViewRef::from_row(&row, parent).unwrap();
                hasher.row(&reference).unwrap();
                inventory.retain_reference(reference).unwrap();
                assert!(
                    budget.reserved() > inventory._charge.accounted_bytes(),
                    "decode scratch remains admitted while physical decoding and hashing are live"
                );
            }
            assert!(inventory._charge.reserved() > before);
            assert_eq!(
                budget.reserved(),
                inventory._charge.accounted_bytes(),
                "finished row workspace must not accumulate in retained inventory"
            );
        }
        assert_eq!(hasher.finish().rows, 129);
        assert!(
            former_workspace > budget.limit(),
            "cumulative dead decode work cannot fit this actual typed-state allowance"
        );
        let typed = base
            + inventory.references.capacity() * std::mem::size_of::<InstalledViewRef>()
            + inventory
                .references
                .iter()
                .map(|reference| selector_heap(&reference.selector))
                .sum::<usize>();
        assert_eq!(inventory._charge.reserved(), typed);
        let accounted = inventory._charge.accounted_bytes();
        for reference in &inventory.references {
            {
                let _validation = inventory_typed_charge(
                    std::mem::size_of::<InstalledViewRef>()
                        .saturating_add(selector_heap(&reference.selector)),
                    &budget,
                )
                .unwrap();
                let child = reference.record(parent);
                let projection = parent_hold(
                    &RecordId::new("native_completed_state", parent.hex()),
                    child.clone(),
                )
                .unwrap();
                assert_eq!(
                    projection.as_object().unwrap().get("object"),
                    Some(&Value::RecordId(child))
                );
                assert!(
                    budget.reserved() > accounted,
                    "post-decode selector addressing and hold projection need their own live workspace"
                );
            }
            assert_eq!(
                budget.reserved(),
                accounted,
                "typed validation scratch ends after each actual projection"
            );
        }
        for index in [0, 63, 64, 127, 128] {
            let expected = &inventory.references[index];
            let installed = inventory.installed(expected.logical_view).unwrap();
            assert!(
                std::ptr::eq(installed, &expected.selector),
                "lookup borrows the sole canonical selector, including across receiving windows"
            );
            assert_eq!(
                inventory.contributors(expected.logical_view).unwrap(),
                expected.selector.mappings
            );
            let held_descriptor = installed.clone();
            assert!(
                std::sync::Arc::ptr_eq(&held_descriptor.selection, &installed.selection),
                "retained capabilities share the inventory payload rather than clone its transitive mappings"
            );
            assert_eq!(budget.reserved(), accounted);
        }
        assert!(inventory.require_pool(&budget).is_ok());
        assert!(
            inventory
                .require_pool(&d::resources::ResourceBudget::fixed(16 << 20).unwrap())
                .is_err()
        );
        assert!(matches!(
            inventory.installed(ContentHash([255; 32])),
            Err(ModelError::Conflict(
                "installed view absent from exact completed inventory"
            ))
        ));
        let owner = std::sync::Arc::new(inventory);
        let borrower = owner.clone();
        drop(owner);
        assert_eq!(
            budget.reserved(),
            accounted,
            "live borrowers retain actual typed allocation ownership"
        );
        drop(borrower);
        assert_eq!(budget.reserved(), 0);
    }
    #[test]
    fn shared_selector_keeps_exact_flat_wire_and_rejects_unknown_fields() {
        #[derive(Serialize)]
        struct PreviousWire<'a> {
            version: u32,
            parent_completion: ContentHash,
            generation: ContentHash,
            validity_epoch: i64,
            view: &'a d::completed::CompletedView,
            mappings: &'a [crate::installed_view::ContributorRealization],
            member_codec: u32,
            membership_rows: u64,
            membership_content: ContentHash,
        }
        let selector = selector().selector;
        let prior = PreviousWire {
            version: selector.version,
            parent_completion: selector.parent_completion,
            generation: selector.generation,
            validity_epoch: selector.validity_epoch,
            view: &selector.view,
            mappings: &selector.mappings,
            member_codec: selector.member_codec,
            membership_rows: selector.membership_rows,
            membership_content: selector.membership_content,
        };
        let bytes = serde_json::to_vec(&selector).unwrap();
        assert_eq!(
            bytes,
            serde_json::to_vec(&prior).unwrap(),
            "sharing changes no persisted byte or field order"
        );
        let decoded: crate::installed_view::InstalledViewCompletion =
            serde_json::from_slice(&bytes).unwrap();
        assert_eq!(decoded, selector);
        assert_eq!(decoded.identity().unwrap(), selector.identity().unwrap());
        let mut unknown: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        unknown
            .as_object_mut()
            .unwrap()
            .insert("extra".into(), true.into());
        assert!(
            serde_json::from_value::<crate::installed_view::InstalledViewCompletion>(unknown)
                .is_err()
        );
    }
    #[test]
    fn installed_ref_requests_preserve_each_exact_selector_and_share_only_equal_copy_requirements()
    {
        let budget = d::resources::ResourceBudget::fixed(1 << 20).unwrap();
        let data = completion();
        let mut first = selector();
        first.selector.parent_completion = data.identity().unwrap();
        let mut second = first.clone();
        std::sync::Arc::make_mut(&mut second.selector.selection).view =
            d::completed::CompletedView::new(
                "releases".into(),
                first.selector.view.contributions.clone(),
                0,
            )
            .unwrap();
        second.logical_view = second.selector.view.identity;
        std::sync::Arc::make_mut(&mut second.selector.selection).mappings[0].view =
            second.logical_view;
        let references = vec![first, second];
        let requests = installed_ref_requests(&data, &references, &budget).unwrap();
        assert_eq!(requests.selectors.len(), 2);
        assert_eq!(requests.copies.len(), 1);
        assert_eq!(
            requests.copies[0].physical,
            references[0].selector.mappings[0].physical
        );
        assert_eq!(
            requests.copies[0].incarnation,
            references[0].selector.mappings[0].incarnation
        );
        assert_eq!(
            requests.copies[0].logical,
            references[0].selector.mappings[0].logical
        );
        for (id, expected) in &requests.selectors {
            let original = references
                .iter()
                .find(|reference| reference.selector.record().unwrap() == *id)
                .unwrap();
            assert!(std::ptr::eq(*expected, &original.selector));
            assert_eq!(
                expected.value().unwrap(),
                original.value(data.identity().unwrap()).unwrap()
            );
        }
        let mut changed = references.clone();
        std::sync::Arc::make_mut(&mut changed[1].selector.selection).mappings[0].incarnation += 1;
        assert!(installed_ref_requests(&data, &changed, &budget).is_err());
    }
    #[test]
    fn installed_ref_requests_refuse_duplicate_foreign_context_and_unfunded_targets() {
        let budget = d::resources::ResourceBudget::fixed(1 << 20).unwrap();
        let data = completion();
        let mut exact = selector();
        exact.selector.parent_completion = data.identity().unwrap();
        assert!(installed_ref_requests(&data, &[exact.clone(), exact.clone()], &budget).is_err());
        for foreign in 0..3 {
            let mut changed = exact.clone();
            match foreign {
                0 => changed.selector.parent_completion = ContentHash([9; 32]),
                1 => changed.selector.generation = ContentHash([9; 32]),
                _ => changed.selector.validity_epoch += 1,
            }
            assert!(installed_ref_requests(&data, &[changed], &budget).is_err());
        }
        assert!(matches!(
            installed_ref_requests(
                &data,
                &[exact],
                &d::resources::ResourceBudget::fixed(1).unwrap()
            ),
            Err(ModelError::Resource { .. })
        ));
    }
    #[test]
    fn compound_parent_link_is_acyclic_but_exact_persisted_selector_is_parent_bound() {
        let mut one = selector();
        let unlinked = one.selector.identity().unwrap();
        let parent = completion().identity().unwrap();
        one.selector.parent_completion = parent;
        assert_eq!(one.selector.identity().unwrap(), unlinked);
        let persisted = one.value(parent).unwrap();
        assert_eq!(InstalledViewRef::from_row(&persisted, parent).unwrap(), one);
        assert!(InstalledViewRef::from_row(&persisted, ContentHash([9; 32])).is_err());
        let mut another = one.clone();
        another.selector.parent_completion = ContentHash([9; 32]);
        assert_ne!(
            another.selector.record().unwrap(),
            one.selector.record().unwrap()
        );
        let mut before = RealizationHasher::default();
        before.row(&selector()).unwrap();
        let mut after = RealizationHasher::default();
        after.row(&one).unwrap();
        assert_eq!(before.finish(), after.finish());
        another.selector.membership_content = ContentHash([8; 32]);
        assert_ne!(another.selector.identity().unwrap(), unlinked);
        std::sync::Arc::make_mut(&mut another.selector.selection).mappings[0].incarnation = 0;
        assert!(another.validate().is_err());
    }
    #[test]
    fn complete_copy_inventory_binds_full_descriptor_incarnation_order_and_eof() {
        let copy = ContributorCopy {
            physical: ContentHash([5; 32]),
            incarnation: 1,
            descriptor: contributor(),
        };
        let hash = |copy: &ContributorCopy| {
            let mut hasher = ContributorHasher::default();
            hasher.row(copy).unwrap();
            hasher.finish()
        };
        let first = hash(&copy);
        assert_ne!(first, ContributorHasher::default().finish());
        let mut changed = copy.clone();
        changed.incarnation = 2;
        assert_ne!(first, hash(&changed));
        changed = copy.clone();
        changed.descriptor.outcome = 2;
        assert_ne!(first, hash(&changed));
        changed = copy.clone();
        changed
            .descriptor
            .outputs
            .get_mut("packages")
            .unwrap()
            .content = ContentHash([12; 32]);
        assert_ne!(first, hash(&changed));
        let mut duplicate = ContributorHasher::default();
        duplicate.row(&copy).unwrap();
        assert!(duplicate.row(&copy).is_err());
        let parent = completion().identity().unwrap();
        let row = copy.value(parent).unwrap();
        assert_eq!(ContributorCopy::from_row(&row, parent).unwrap(), copy);
        assert!(ContributorCopy::from_row(&row, ContentHash([8; 32])).is_err());
        changed.incarnation = 0;
        assert!(changed.validate().is_err());
    }
    #[test]
    fn inventory_projection_pointers_require_exact_guarded_children_and_stream_order() {
        let parent = completion().identity().unwrap();
        let copy = ContributorCopy {
            physical: ContentHash([5; 32]),
            incarnation: 1,
            descriptor: contributor(),
        };
        let copy_row = copy.value(parent).unwrap();
        let mut reference = selector();
        reference.selector.parent_completion = parent;
        let selector_row = reference.value(parent).unwrap();
        let projection = |id: RecordId, key: &str, value: String| {
            let mut row = Object::new();
            row.insert("id", id);
            row.insert(key, value);
            Value::Object(row)
        };
        for (row, key, value) in [
            (&copy_row, "physical", copy.physical.hex()),
            (&selector_row, "logical_view", reference.logical_view.hex()),
        ] {
            let id =
                RecordId::from_value(row.as_object().unwrap().get("id").unwrap().clone()).unwrap();
            let projected = projection(id, key, value.clone());
            let pointer = InventoryPointer::from_row(&projected, key, None).unwrap();
            if key == "physical" {
                assert_eq!(pointer.copy(row, parent).unwrap(), copy);
                assert!(pointer.copy(row, ContentHash([9; 32])).is_err());
                let wrong_key = InventoryPointer {
                    id: pointer.id.clone(),
                    key: ContentHash([8; 32]).hex(),
                };
                assert!(wrong_key.copy(row, parent).is_err());
                let wrong_id = InventoryPointer {
                    id: RecordId::new("native_completed_contributor", "other"),
                    key: value.clone(),
                };
                assert!(wrong_id.copy(row, parent).is_err());
            } else {
                assert_eq!(pointer.selector(row, parent).unwrap(), reference);
                assert!(pointer.selector(row, ContentHash([9; 32])).is_err());
                let wrong_key = InventoryPointer {
                    id: pointer.id.clone(),
                    key: ContentHash([8; 32]).hex(),
                };
                assert!(wrong_key.selector(row, parent).is_err());
                let wrong_id = InventoryPointer {
                    id: RecordId::new("native_completed_view", "other"),
                    key: value.clone(),
                };
                assert!(wrong_id.selector(row, parent).is_err());
            }
            assert!(InventoryPointer::from_row(&projected, key, Some(&value)).is_err());
            assert!(InventoryPointer::from_row(&projected, key, Some(&"f".repeat(64))).is_err());
            assert!(InventoryPointer::from_row(&projected, key, Some(&"0".repeat(64))).is_ok());
            assert!(InventoryPointer::from_row(row, key, None).is_err());
            assert!(
                InventoryPointer::from_row(
                    &projection(pointer.id.clone(), key, "bad".into()),
                    key,
                    None
                )
                .is_err()
            );
            assert!(
                InventoryPointer::from_row(
                    &projection(RecordId::new("foreign", "0".repeat(64)), key, value),
                    key,
                    None
                )
                .is_err()
            );
        }
    }
    #[test]
    fn retained_boundary_preserves_zero_source_revision_without_claiming_aggregate_freeze() {
        let mut retained = completion();
        retained.completed_revision = 0;
        retained.validate().unwrap();
        let row = retained.value(true).unwrap();
        assert_eq!(FrozenCompletion::from_row(&row, true).unwrap(), retained);
        let positive = completion();
        assert_eq!(retained.completed_state, positive.completed_state);
        assert_ne!(retained.identity().unwrap(), positive.identity().unwrap());
        retained.completed_revision = -1;
        assert!(retained.validate().is_err());
        retained.completed_revision = 0;
        retained.attempt_epoch = 0;
        assert!(retained.validate().is_err());
        retained.attempt_epoch = 1;
        retained.validity_epoch = 0;
        assert!(retained.validate().is_err());
    }
    #[test]
    fn physical_v2_envelope_refuses_legacy_unfinished_extra_and_same_count_substitution() {
        let one = completion();
        let row = one.value(true).unwrap();
        assert_eq!(FrozenCompletion::from_row(&row, true).unwrap(), one);
        assert!(FrozenCompletion::from_row(&one.value(false).unwrap(), true).is_err());
        let mut legacy = one.clone();
        legacy.version = 1;
        assert!(legacy.validate().is_err());
        let mut extra = row.clone();
        if let Value::Object(row) = &mut extra {
            row.insert("unexpected", true);
        }
        assert!(FrozenCompletion::from_row(&extra, true).is_err());
        let mut changed = one.clone();
        changed.contributor_inventory.content = ContentHash([13; 32]);
        assert_ne!(one.identity().unwrap(), changed.identity().unwrap());
        changed = one.clone();
        changed.realization_inventory.content = ContentHash([14; 32]);
        assert_ne!(one.identity().unwrap(), changed.identity().unwrap());
    }
}

/// A compound boundary owns only its declared identity-bearing roots. Pins,
/// publication/admission/product references are incoming edges; they add no
/// auxiliary outgoing authority to this inventory.
async fn audit_parent_holds(
    reader: &crate::NativeReader<()>,
    data: &FrozenCompletion,
    inventory: &InstalledViewInventory,
    budget: &d::resources::ResourceBudget,
) -> Result<(), ModelError> {
    let parent = data.record()?;
    let identity = data.identity()?;
    let mut expected = crate::ordered_rows::SortedRows::with_budget(budget)?;
    for copy in inventory.copies.values() {
        expected.push(parent_hold(&parent, copy.record(identity))?)?;
        expected.push(parent_hold(
            &parent,
            RecordId::new("compiler_contribution", copy.physical.hex()),
        )?)?;
    }
    for reference in &inventory.references {
        let _validation = inventory_typed_charge(
            std::mem::size_of::<InstalledViewRef>()
                .saturating_add(selector_heap(&reference.selector)),
            budget,
        )?;
        expected.push(parent_hold(&parent, reference.record(identity))?)?;
        expected.push(parent_hold(
            &parent,
            RecordId::new("compiler_view", reference.logical_view.hex()),
        )?)?;
    }
    for binding in &inventory.bindings {
        let _validation = inventory_typed_charge(
            std::mem::size_of::<d::completed::CompletedBinding>()
                .saturating_add(binding_heap(binding)),
            budget,
        )?;
        expected.push(parent_hold(
            &parent,
            crate::binding_descriptor::record(binding)?,
        )?)?;
    }
    let expected = expected.finish()?.into_prepared();
    let mut exact = ParentHoldInventoryCursor::new(&expected, budget)?;
    let mut variables = surrealdb::types::Variables::new();
    variables.insert("compound_parent", parent.clone());
    let mut rows = reader.stream_prepared_observed(
        crate::prepared::PreparedQuery::new(
            variables,
            vec![],
            vec![COMPOUND_PARENT_HOLDS_SQL.into()],
        )?,
        Some("compound_parent_holds"),
        None,
    )?;
    let result = async {
        while let Some(row) = rows.next().await? {
            exact.row(&parent, &row, budget)?;
        }
        exact.finish()
    }
    .await;
    let mut terminal = d::completion::Completion::default();
    terminal.step(
        "compound parent hold complete EOF/drain",
        rows.drain_transport().await,
    );
    d::completion::complete(result, terminal)
}
fn parent_hold(parent: &RecordId, object: RecordId) -> Result<Value, ModelError> {
    let mut row = Object::new();
    row.insert("id", crate::control::hold_record_id(parent, &object)?);
    row.insert("owner", parent.clone());
    row.insert("object", object);
    Ok(Value::Object(row))
}
fn check_parent_hold(
    parent: &RecordId,
    row: &Value,
    previous: Option<&RecordId>,
) -> Result<RecordId, ModelError> {
    let object = row
        .as_object()
        .ok_or(ModelError::Schema("compound parent hold object"))?;
    let Some(Value::RecordId(target)) = object.get("object") else {
        return Err(ModelError::Schema("compound parent hold target"));
    };
    let id = crate::control::hold_record_id(parent, target)?;
    if previous.is_some_and(|previous| previous >= &id)
        || row != parent_hold(parent, target.clone())?
    {
        return Err(ModelError::Conflict(
            "compound parent hold exact envelope/order/duplicate",
        ));
    }
    Ok(id)
}
struct ParentHoldInventoryCursor {
    expected: crate::ordered_rows::PreparedFrameRows,
    previous: Option<RecordId>,
}
impl ParentHoldInventoryCursor {
    fn new(
        expected: &crate::ordered_rows::PreparedRows,
        budget: &d::resources::ResourceBudget,
    ) -> Result<Self, ModelError> {
        Ok(Self {
            expected: expected.frame_cursor(budget)?,
            previous: None,
        })
    }
    fn row(
        &mut self,
        parent: &RecordId,
        row: &Value,
        budget: &d::resources::ResourceBudget,
    ) -> Result<(), ModelError> {
        let _scratch = budget.reserve(
            "compound actual hold inventory envelope",
            crate::loader::native_bytes(row)
                .saturating_mul(16)
                .saturating_add(4096),
        )?;
        let id = check_parent_hold(parent, row, self.previous.as_ref())?;
        let frame = serde_json::to_vec(row).map_err(ModelError::codec)?;
        match self.expected.next_frame()? {
            Some((expected_id, expected)) if expected_id == &id && expected == frame => {
                self.previous = Some(id);
                Ok(())
            }
            _ => Err(ModelError::Conflict(
                "compound parent actual hold inventory extra/missing/substitution",
            )),
        }
    }
    fn finish(mut self) -> Result<(), ModelError> {
        if self.expected.next_frame()?.is_some() {
            return Err(ModelError::Conflict(
                "compound parent actual hold inventory extra/missing/substitution",
            ));
        }
        Ok(())
    }
}
#[cfg(test)]
mod parent_hold_controls {
    use super::*;
    #[test]
    fn parent_hold_inventory_refuses_extra_copy_missing_selector_equal_count_substitution_and_duplicate()
     {
        let budget = d::resources::ResourceBudget::fixed(4 << 20).unwrap();
        let parent = RecordId::new("native_completed_state", "p");
        let copies = [
            RecordId::new("compiler_contribution", "a"),
            RecordId::new("native_completed_view", "v"),
        ];
        let prepare = |objects: &[RecordId]| {
            let mut rows = crate::ordered_rows::SortedRows::with_budget(&budget).unwrap();
            for object in objects {
                rows.push(parent_hold(&parent, object.clone()).unwrap())
                    .unwrap();
            }
            rows.finish().unwrap().into_prepared()
        };
        let expected = prepare(&copies);
        let compare = |actual: &crate::ordered_rows::PreparedRows| -> Result<(), ModelError> {
            let mut exact = ParentHoldInventoryCursor::new(&expected, &budget)?;
            let mut rows = actual.cursor_with_budget(&budget)?;
            while let Some(row) = rows.next_row()? {
                exact.row(&parent, &row, &budget)?;
            }
            exact.finish()
        };
        compare(&prepare(&copies)).unwrap();
        let mut extra = copies.to_vec();
        extra.push(RecordId::new("compiler_contribution", "extra"));
        assert!(compare(&prepare(&extra)).is_err());
        assert!(compare(&prepare(&copies[..1])).is_err());
        let substituted = [
            copies[0].clone(),
            RecordId::new("native_completed_view", "other"),
        ];
        assert!(compare(&prepare(&substituted)).is_err());
        let row = parent_hold(&parent, copies[0].clone()).unwrap();
        let previous = check_parent_hold(&parent, &row, None).unwrap();
        assert!(check_parent_hold(&parent, &row, Some(&previous)).is_err());
        assert!(
            check_parent_hold(
                &RecordId::new("native_completed_state", "foreign"),
                &row,
                None
            )
            .is_err()
        );
        let mut rows = expected.cursor_with_budget(&budget).unwrap();
        let first = rows.next_row().unwrap().unwrap();
        let second = rows.next_row().unwrap().unwrap();
        let mut duplicate = ParentHoldInventoryCursor::new(&expected, &budget).unwrap();
        duplicate.row(&parent, &first, &budget).unwrap();
        assert!(duplicate.row(&parent, &first, &budget).is_err());
        let mut reversed = ParentHoldInventoryCursor::new(&expected, &budget).unwrap();
        assert!(reversed.row(&parent, &second, &budget).is_err());
        let mut foreign = ParentHoldInventoryCursor::new(&expected, &budget).unwrap();
        assert!(
            foreign
                .row(
                    &RecordId::new("native_completed_state", "foreign"),
                    &first,
                    &budget
                )
                .is_err()
        );
        assert!(
            ParentHoldInventoryCursor::new(&expected, &budget)
                .unwrap()
                .finish()
                .is_err()
        );
        let empty = prepare(&[]);
        ParentHoldInventoryCursor::new(&empty, &budget)
            .unwrap()
            .finish()
            .unwrap();
        assert!(
            ParentHoldInventoryCursor::new(&empty, &budget)
                .unwrap()
                .row(&parent, &first, &budget)
                .is_err()
        );
    }
}
