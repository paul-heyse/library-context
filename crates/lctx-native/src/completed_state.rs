//! Installation-local physical completion. Serialized data never grants freeze,
//! semantic admission or protected read authority; native factories earn those separately.
pub(crate) mod earning;
mod product_replay;
mod uniform;
pub use earning::{
    ViewSelection, complete_frozen_content, complete_frozen_content_scoped,
    complete_retained_content, complete_retained_content_scoped,
};
use lctx_model::domain::{self as d, ContentHash, Key, KeySink, ModelError};
pub use product_replay::check_retained_product_replay;
use serde::{Deserialize, Serialize};
use surrealdb::types::{Object, RecordId, SurrealValue, Value};
pub use uniform::PublishedUniformOwners;

pub const VERSION: u32 = 2;
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
        let mut sink = KeySink::new("native-completed-contributor/v2");
        completion.encode(&mut sink);
        self.physical.encode(&mut sink);
        RecordId::new("native_completed_contributor", sink.finish().hex())
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
        let completion_charge = budget.reserve(
            "native frozen completion decoding",
            crate::loader::native_bytes(expected.row())
                .saturating_mul(32)
                .saturating_add(65536),
        )?;
        let data = FrozenCompletion::from_row(expected.row(), true)?;
        if data.identity()? != identity || data.attempt != attempt {
            return Err(ModelError::Conflict("native completed owner/identity"));
        }
        let held = crate::control::hold_frozen_completed_expected(&client, &data, expected, budget)
            .await?;
        let inventory = inspect_inventory(&reader, &data, budget).await?;
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
        Ok(HeldFrozenCompletedState {
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
pub(crate) async fn verify_installed_ref(
    reader: &crate::NativeReader<()>,
    data: &FrozenCompletion,
    reference: &InstalledViewRef,
    budget: &d::resources::ResourceBudget,
) -> Result<crate::installed_view::InstalledViewCompletion, ModelError> {
    reference.validate()?;
    let selector = &reference.selector;
    if selector.parent_completion != data.identity()?
        || selector.generation != data.generation
        || selector.validity_epoch != data.validity_epoch
    {
        return Err(ModelError::Conflict(
            "compound selector exact parent/context",
        ));
    }
    let address = selector.record()?;
    let expected = crate::control::inspect_expected_read(reader, address.clone(), budget)
        .await?
        .ok_or(ModelError::Conflict("compound exact selector missing"))?;
    if expected.row() != &selector.value()? {
        return Err(ModelError::Conflict("compound exact selector bytes"));
    }
    require_hold(reader, &data.record()?, &address).await?;
    for mapping in &selector.mappings {
        let mut vars = surrealdb::types::Variables::new();
        let copy_address = {
            let mut sink = KeySink::new("native-completed-contributor/v2");
            data.identity()?.encode(&mut sink);
            mapping.physical.encode(&mut sink);
            RecordId::new("native_completed_contributor", sink.finish().hex())
        };
        vars.insert("copy", copy_address.clone());
        let copies: Vec<Value> = reader.query_native("SELECT * FROM $copy", vars).await?;
        let [copy] = copies.as_slice() else {
            return Err(ModelError::Conflict(
                "compound selected copy inventory cardinality",
            ));
        };
        let copy = ContributorCopy::from_row(copy, data.identity()?)?;
        if copy.physical != mapping.physical
            || copy.incarnation != mapping.incarnation
            || copy.descriptor.identity()? != mapping.logical
        {
            return Err(ModelError::Conflict(
                "compound exact selected copy incidence",
            ));
        }
        require_hold(reader, &data.record()?, &copy_address).await?;
        let physical = RecordId::new("compiler_contribution", mapping.physical.hex());
        let actual = crate::control::inspect_expected_read(reader, physical.clone(), budget)
            .await?
            .ok_or(ModelError::Conflict("compound mapped contributor missing"))?;
        crate::completed_rows::validate_state_row("compiler_contribution", actual.row())?;
        if actual.incarnation() != copy.incarnation
            || crate::installed_view::audit::descriptor::<d::completed::CompletedContribution>(
                actual.row(),
            )? != copy.descriptor
        {
            return Err(ModelError::Conflict(
                "compound mapped full descriptor/incarnation",
            ));
        }
        require_hold(reader, &data.record()?, &physical).await?;
    }
    Ok(selector.clone())
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
    installed:
        std::collections::BTreeMap<ContentHash, crate::installed_view::InstalledViewCompletion>,
    _charge: d::charged::StateCharge,
}
impl InstalledViewInventory {
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
        self.installed.get(&logical).ok_or(ModelError::Conflict(
            "installed view absent from exact completed inventory",
        ))
    }
}
pub(crate) async fn inspect_inventory(
    reader: &crate::NativeReader<()>,
    data: &FrozenCompletion,
    budget: &d::resources::ResourceBudget,
) -> Result<std::sync::Arc<InstalledViewInventory>, ModelError> {
    let identity = data.identity()?;
    let parent = data.record()?;
    let mut result = InstalledViewInventory {
        references: Vec::new(),
        copies: std::collections::BTreeMap::new(),
        bindings: Vec::new(),
        binding_incarnations: std::collections::BTreeMap::new(),
        installed: std::collections::BTreeMap::new(),
        _charge: d::charged::StateCharge::new(budget, "exact completed installed inventory"),
    };
    let mut variables = surrealdb::types::Variables::new();
    variables.insert("parent", parent.clone());
    let mut copies=reader.stream_prepared(crate::prepared::PreparedQuery::new(variables.clone(),vec![],vec!["SELECT * FROM native_completed_contributor WITH INDEX completion_contributor WHERE completion=$parent ORDER BY physical,id".into()])?)?;
    let observed = async {
        let mut hasher = ContributorHasher::default();
        while let Some(row) = copies.next().await? {
            result._charge.grow(
                crate::loader::native_bytes(&row)
                    .saturating_mul(32)
                    .saturating_add(2048),
            )?;
            let copy = ContributorCopy::from_row(&row, identity)?;
            hasher.row(&copy)?;
            result.copies.insert(copy.physical, copy);
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
    // Inspect each exact physical contributor once, in actual heterogeneous row/guard
    // windows with its canonical copy child. No selector repeats this acquisition.
    let _window = budget.reserve(
        "compound exact copy window handles",
        crate::loader::NATIVE_WINDOW_ROWS.saturating_mul(2048),
    )?;
    let mut windows = Vec::with_capacity(crate::loader::NATIVE_WINDOW_ROWS);
    for copy in result.copies.values() {
        windows.push(copy);
        if windows.len() >= crate::loader::NATIVE_WINDOW_ROWS / 2 {
            audit_copy_window(reader, data, &windows, budget).await?;
            windows.clear();
        }
    }
    if !windows.is_empty() {
        audit_copy_window(reader, data, &windows, budget).await?;
    }
    drop(windows);
    let mut selectors=reader.stream_prepared(crate::prepared::PreparedQuery::new(variables.clone(),vec![],vec!["SELECT * FROM native_completed_view WITH INDEX completion_view WHERE completion=$parent ORDER BY logical_view,id".into()])?)?;
    let observed = async {
        let mut hasher = RealizationHasher::default();
        while let Some(row) = selectors.next().await? {
            result._charge.grow(
                crate::loader::native_bytes(&row)
                    .saturating_mul(32)
                    .saturating_add(2048)
                    .saturating_add(2 * std::mem::size_of::<InstalledViewRef>()),
            )?;
            let reference = InstalledViewRef::from_row(&row, identity)?;
            hasher.row(&reference)?;
            validate_selector_copies(data, &reference.selector, &result.copies)?;
            result
                .installed
                .insert(reference.logical_view, reference.selector.clone());
            result.references.push(reference);
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
            .installed
            .values()
            .map(|selector| selector.mappings.len())
            .sum::<usize>()
            .saturating_mul(128)
            .saturating_add(1024),
    )?;
    let selected = result
        .installed
        .values()
        .flat_map(|selector| selector.mappings.iter().map(|mapping| mapping.physical))
        .collect::<std::collections::BTreeSet<_>>();
    if !selected.iter().eq(result.copies.keys()) {
        return Err(ModelError::Conflict(
            "compound contributor inventory extra/missing selector incidence",
        ));
    }
    let mut windows = Vec::with_capacity(crate::loader::NATIVE_WINDOW_ROWS / 2);
    for reference in &result.references {
        windows.push(reference);
        if windows.len() >= crate::loader::NATIVE_WINDOW_ROWS / 2 {
            audit_selector_window(reader, data, &windows, budget).await?;
            windows.clear();
        }
    }
    if !windows.is_empty() {
        audit_selector_window(reader, data, &windows, budget).await?;
    }
    drop(windows);
    let mut bindings=reader.stream_prepared(crate::prepared::PreparedQuery::new(variables,vec![],vec!["SELECT object AS id FROM native_hold WITH INDEX owner_holds WHERE owner=$parent AND record::table(object)='compiler_binding_descriptor' ORDER BY object".into()])?)?;
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
    if d::completed::binding_inventory_identity(&result.bindings)? != data.bindings {
        return Err(ModelError::Conflict(
            "completed canonical binding actual EOF/inventory",
        ));
    }
    result.bindings.sort_by_key(|binding| binding.key());
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
async fn audit_copy_window(
    reader: &crate::NativeReader<()>,
    data: &FrozenCompletion,
    copies: &[&ContributorCopy],
    budget: &d::resources::ResourceBudget,
) -> Result<(), ModelError> {
    let identity = data.identity()?;
    let mut ids = Vec::with_capacity(copies.len() * 2);
    for copy in copies {
        ids.push(copy.record(identity));
        ids.push(RecordId::new("compiler_contribution", copy.physical.hex()));
    }
    let mut actual = crate::control::inspect_expected_window_read(reader, &ids, budget).await?;
    for copy in copies {
        let child = actual.take_required(&copy.record(identity))?;
        if child.row() != &copy.value(identity)? {
            return Err(ModelError::Conflict(
                "compound guarded copy descriptor changed",
            ));
        }
        let physical = RecordId::new("compiler_contribution", copy.physical.hex());
        let expected = actual.take_required(&physical)?;
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
    }
    actual.finish()
}
async fn audit_selector_window(
    reader: &crate::NativeReader<()>,
    data: &FrozenCompletion,
    references: &[&InstalledViewRef],
    budget: &d::resources::ResourceBudget,
) -> Result<(), ModelError> {
    let identity = data.identity()?;
    let mut ids = Vec::with_capacity(references.len() * 2);
    for reference in references {
        ids.push(reference.record(identity));
        ids.push(RecordId::new("compiler_view", reference.logical_view.hex()));
    }
    let mut actual = crate::control::inspect_expected_window_read(reader, &ids, budget).await?;
    for reference in references {
        let selector = actual.take_required(&reference.record(identity))?;
        if selector.row() != &reference.value(identity)? {
            return Err(ModelError::Conflict(
                "compound guarded selector bytes changed",
            ));
        }
        let semantic = actual.take_required(&RecordId::new(
            "compiler_view",
            reference.logical_view.hex(),
        ))?;
        crate::completed_rows::validate_state_row("compiler_view", semantic.row())?;
        if crate::installed_view::audit::descriptor::<d::completed::CompletedView>(semantic.row())?
            != reference.selector.view
        {
            return Err(ModelError::Conflict(
                "compound semantic view descriptor changed",
            ));
        }
    }
    actual.finish()
}
async fn audit_binding_window(
    reader: &crate::NativeReader<()>,
    ids: &[RecordId],
    inventory: &mut InstalledViewInventory,
    budget: &d::resources::ResourceBudget,
) -> Result<(), ModelError> {
    let mut actual = crate::control::inspect_expected_window_read(reader, ids, budget).await?;
    let mut pairs = Vec::with_capacity(ids.len());
    for id in ids {
        let expected = actual.take_required(id)?;
        inventory._charge.grow(
            crate::loader::native_bytes(expected.row())
                .saturating_mul(32)
                .saturating_add(2048),
        )?;
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
        let identity = binding.descriptor_identity()?;
        if inventory
            .binding_incarnations
            .insert(identity, expected.incarnation())
            .is_some()
        {
            return Err(ModelError::Conflict(
                "completed duplicate canonical binding incidence",
            ));
        }
        inventory.bindings.push(binding);
    }
    actual.finish()?;
    let mut holds = crate::control::inspect_hold_window_read(reader, &pairs, budget).await?;
    for (owner, object) in pairs {
        holds.take_required(&owner, &object)?;
    }
    holds.finish()
}

/// Physical completion precedes semantic admission. This private-field owner can only
/// be earned by the independent native completion writer under a durable frozen fence.
pub struct NativeCompletedState {
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
        let mut charge =
            d::charged::StateCharge::new(budget, "frozen lifecycle physical completion");
        charge.grow(
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
        let repeated =
            crate::control::hold_frozen_physical_expected(&client, &data, expected, budget).await?;
        Ok(NativeCompletedState {
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
struct PublishedCompletedInner {
    completion: FrozenCompletion,
    completion_incarnation: i64,
    admission: CompletedAdmission,
    admission_incarnation: i64,
    inventory: std::sync::Arc<InstalledViewInventory>,
    marker: crate::reader::ProtectedPublicationMarker,
    // This is a retained borrowed scope/pin, never a separately owned session to close.
    reader: crate::NativeReader<d::serving::SnapshotHandle>,
    _charge: Box<dyn d::resources::Reservation>,
}
impl PublishedCompletedState {
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
        if !std::ptr::eq(client, self.inner.reader.client()) {
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
        &self.inner.reader
    }
}
/// Marker/pin reachability and persisted semantic admission survive legitimate origin
/// history compaction. No origin attempt lookup is made on this publication read path.
pub async fn published_completed_state(
    reader: &crate::NativeReader<d::serving::SnapshotHandle>,
    budget: &d::resources::ResourceBudget,
) -> Result<PublishedCompletedState, ModelError> {
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
    let expected = crate::control::inspect_expected_read(&transport, completion_id.clone(), budget)
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
        || marker.row().get("completion_incarnation") != Some(&expected.incarnation().into_value())
    {
        return Err(ModelError::Conflict(
            "published physical completion identity/incarnation",
        ));
    }
    let admission = crate::control::inspect_expected_read(&transport, admission_id.clone(), budget)
        .await?
        .ok_or(ModelError::Conflict("published semantic admission missing"))?;
    let mut charge = budget.reserve(
        "published completed retained data",
        crate::loader::native_bytes(expected.row())
            .saturating_add(crate::loader::native_bytes(admission.row()))
            .saturating_mul(32)
            .saturating_add(65536),
    )?;
    let semantic = CompletedAdmission::from_row(admission.row())?;
    let model = d::model()?;
    if semantic.record()? != admission_id
        || marker.row().get("admission_incarnation") != Some(&admission.incarnation().into_value())
    {
        return Err(ModelError::Conflict(
            "published semantic admission identity/incarnation",
        ));
    }
    require_hold(&transport, &semantic.record()?, &data.record()?).await?;
    let inventory = inspect_inventory(&transport, &data, budget).await?;
    semantic.validate(&data, expected.incarnation(), &model, &inventory, budget)?;
    let repeated = reader.protected_publication_marker(budget).await?;
    if repeated.row() != marker.row() {
        return Err(ModelError::Conflict(
            "published marker changed during completed factory",
        ));
    }
    charge.try_resize(
        crate::loader::native_bytes(&data.value(true)?)
            .saturating_add(crate::loader::native_bytes(&semantic.value()?))
            .saturating_mul(32)
            .saturating_add(65536),
    )?;
    let result = PublishedCompletedState {
        inner: std::sync::Arc::new(PublishedCompletedInner {
            completion: data,
            completion_incarnation: expected.incarnation(),
            admission: semantic,
            admission_incarnation: admission.incarnation(),
            inventory,
            marker,
            reader: reader.clone(),
            _charge: charge,
        }),
    };
    if crate::completed_transport::eligibility_published(&result, budget).await?
        != reader.handle().eligibility
    {
        return Err(ModelError::Conflict("published exact selector eligibility"));
    }
    Ok(result)
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
                mappings: vec![crate::installed_view::ContributorRealization {
                    view: view.identity,
                    logical,
                    physical: ContentHash([5; 32]),
                    incarnation: 1,
                }],
                view,
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
        another.selector.mappings[0].incarnation = 0;
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
        expected.push(parent_hold(&parent, reference.record(identity))?)?;
        expected.push(parent_hold(
            &parent,
            RecordId::new("compiler_view", reference.logical_view.hex()),
        )?)?;
    }
    for binding in &inventory.bindings {
        expected.push(parent_hold(
            &parent,
            crate::binding_descriptor::record(binding)?,
        )?)?;
    }
    let expected = expected.finish()?.into_prepared();
    let mut actual = crate::ordered_rows::SortedRows::with_budget(budget)?;
    let mut variables = surrealdb::types::Variables::new();
    variables.insert("parent", parent.clone());
    let mut rows = reader.stream_prepared(crate::prepared::PreparedQuery::new(
        variables,
        vec![],
        vec![
            "SELECT * FROM native_hold WITH INDEX owner_holds WHERE owner=$parent ORDER BY id"
                .into(),
        ],
    )?)?;
    let result = async {
        let mut previous = None;
        while let Some(row) = rows.next().await? {
            let _scratch = budget.reserve(
                "compound actual hold inventory envelope",
                crate::loader::native_bytes(&row)
                    .saturating_mul(16)
                    .saturating_add(4096),
            )?;
            previous = Some(check_parent_hold(&parent, &row, previous.as_ref())?);
            actual.push(row)?;
        }
        Ok(())
    }
    .await;
    let mut terminal = d::completion::Completion::default();
    terminal.step(
        "compound parent hold complete EOF/drain",
        rows.drain_transport().await,
    );
    d::completion::complete(result, terminal)?;
    exact_parent_hold_inventory(&expected, &actual.finish()?.into_prepared(), budget)
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
        || row != &parent_hold(parent, target.clone())?
    {
        return Err(ModelError::Conflict(
            "compound parent hold exact envelope/order/duplicate",
        ));
    }
    Ok(id)
}
fn exact_parent_hold_inventory(
    expected: &crate::ordered_rows::PreparedRows,
    actual: &crate::ordered_rows::PreparedRows,
    budget: &d::resources::ResourceBudget,
) -> Result<(), ModelError> {
    let mut expected = expected.frame_cursor(budget)?;
    let mut actual = actual.frame_cursor(budget)?;
    loop {
        match (expected.next_frame()?, actual.next_frame()?) {
            (None, None) => return Ok(()),
            (Some((left_id, left)), Some((right_id, right)))
                if left_id == right_id && left == right => {}
            _ => {
                return Err(ModelError::Conflict(
                    "compound parent actual hold inventory extra/missing/substitution",
                ));
            }
        }
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
        exact_parent_hold_inventory(&expected, &prepare(&copies), &budget).unwrap();
        let mut extra = copies.to_vec();
        extra.push(RecordId::new("compiler_contribution", "extra"));
        assert!(exact_parent_hold_inventory(&expected, &prepare(&extra), &budget).is_err());
        assert!(exact_parent_hold_inventory(&expected, &prepare(&copies[..1]), &budget).is_err());
        let substituted = [
            copies[0].clone(),
            RecordId::new("native_completed_view", "other"),
        ];
        assert!(exact_parent_hold_inventory(&expected, &prepare(&substituted), &budget).is_err());
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
    }
}
