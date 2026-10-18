//! Nongraph immutable containers and contribution-qualified physical locators.
//! Logical row IDs/envelopes remain unchanged. Packing confers no membership,
//! completion or semantic-admission authority, including for unselected slots.
use lctx_model::domain::resources::{Reservation, ResourceBudget};
use lctx_model::domain::{ContentHash, ContentHasher, ModelError};
use std::{
    collections::{BTreeMap, BTreeSet},
    io::Write,
    sync::Arc,
};
use surrealdb::types::{Object, RecordId, Value};

pub const FORMAT: &str = "CompilerBlock/v1;canonical-native-envelope/v1;packing-256k/v1";
/// A packing target, not a new maximum supported row size.
pub const TARGET_BYTES: usize = 256 * 1024;

/// Only actual stored objects may enter hold/incarnation/retirement protocols.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PhysicalLocation(Location);
#[derive(Clone, Debug, PartialEq, Eq)]
enum Location {
    Graph(RecordId),
    Block { record: RecordId, slot: usize },
}
impl PhysicalLocation {
    pub fn graph(record: RecordId) -> Result<Self, ModelError> {
        if !matches!(record.table.as_str(), "entity" | "assertion") {
            return Err(ModelError::Schema("physical graph address family"));
        }
        Ok(Self(Location::Graph(record)))
    }
    pub fn block(record: RecordId, slot: usize) -> Result<Self, ModelError> {
        if record.table.as_str() != "compiler_block" || slot > i64::MAX as usize {
            return Err(ModelError::Schema("physical compiler block address"));
        }
        Ok(Self(Location::Block { record, slot }))
    }
    pub fn record(&self) -> &RecordId {
        match &self.0 {
            Location::Graph(record) | Location::Block { record, .. } => record,
        }
    }
    pub fn from_locator(row: &Value) -> Result<Self, ModelError> {
        let row = object(row)?;
        match row.get("storage") {
            Some(Value::String(storage)) if storage == "graph" => {
                if row.contains_key("block") || row.contains_key("slot") {
                    return Err(ModelError::Schema("graph locator contains block address"));
                }
                Self::graph(record(row, "node")?.clone())
            }
            Some(Value::String(storage)) if storage == "block" => {
                if record(row, "node")?.table.as_str() != "compiler_record" {
                    return Err(ModelError::Schema("block locator logical family"));
                }
                let slot = match row.get("slot") {
                    Some(Value::Number(surrealdb::types::Number::Int(slot))) => {
                        usize::try_from(*slot).map_err(ModelError::codec)?
                    }
                    _ => return Err(ModelError::Schema("block locator slot")),
                };
                Self::block(record(row, "block")?.clone(), slot)
            }
            _ => Err(ModelError::Schema("compiler locator storage")),
        }
    }
}

/// One admitted installation packet. Backing charges survive native issue and
/// acknowledgement; the packet consumes existing canonical row buffers.
pub struct PreparedBlocks {
    blocks: Vec<Value>,
    locators: Vec<Value>,
    _charge: Box<dyn Reservation>,
}
impl PreparedBlocks {
    /// Preserving lowering only. Independent receiving uses validate_block and
    /// its ordinary typed batch/admission owner before installation.
    pub fn prepare(
        contribution: ContentHash,
        mut rows: Vec<Value>,
        budget: &ResourceBudget,
    ) -> Result<Self, ModelError> {
        let bytes = rows
            .iter()
            .map(|row| {
                crate::loader::native_bytes(row)
                    .saturating_mul(8)
                    .saturating_add(8192)
            })
            .fold(4096usize, usize::saturating_add);
        let charge = budget.reserve("compiler block installation packet", bytes)?;
        for row in &rows {
            frame(row)?;
        }
        rows.sort_by(|left, right| {
            let left = left.as_object().expect("checked row frame");
            let right = right.as_object().expect("checked row frame");
            (
                text(left, "semantic_type").expect("checked type"),
                text(left, "semantic_key").expect("checked key"),
            )
                .cmp(&(
                    text(right, "semantic_type").expect("checked type"),
                    text(right, "semantic_key").expect("checked key"),
                ))
        });
        let mut blocks = Vec::new();
        let mut locators = Vec::new();
        let mut pending = Vec::new();
        let mut pending_bytes = 0usize;
        let mut relation = String::new();
        for row in rows {
            let object = object(&row)?;
            let name = text(object, "semantic_type")?;
            if let Some(previous) = pending.last() {
                let earlier = object_ref(previous)?;
                if text(earlier, "semantic_type")? == name
                    && text(earlier, "semantic_key")? == text(object, "semantic_key")?
                {
                    if *previous != row {
                        return Err(ModelError::Conflict(
                            "compiler packet conflicting nominal row",
                        ));
                    }
                    continue;
                }
            }
            let size = encoded_size(&row)?;
            if !pending.is_empty()
                && (relation != name || pending_bytes.saturating_add(size) > TARGET_BYTES)
            {
                finish_block(
                    contribution,
                    &relation,
                    std::mem::take(&mut pending),
                    &mut blocks,
                    &mut locators,
                )?;
                pending_bytes = 0;
            }
            relation = name.to_owned();
            pending_bytes = pending_bytes.saturating_add(size);
            pending.push(row);
        }
        if !pending.is_empty() {
            finish_block(contribution, &relation, pending, &mut blocks, &mut locators)?;
        }
        Ok(Self {
            blocks,
            locators,
            _charge: charge,
        })
    }
    pub fn blocks(&self) -> &[Value] {
        &self.blocks
    }
    pub fn locators(&self) -> &[Value] {
        &self.locators
    }
    pub fn into_parts(self) -> (Vec<Value>, Vec<Value>, Box<dyn Reservation>) {
        (self.blocks, self.locators, self._charge)
    }
}
pub fn graph_locator(contribution: ContentHash, row: &Value) -> Result<Value, ModelError> {
    let object = object(row)?;
    PhysicalLocation::graph(record(object, "id")?.clone())?;
    let relation = text(object, "semantic_type")?;
    let key = crate::completed_rows::decode_nominal_key(text(object, "semantic_key")?)?;
    let mut locator = Object::new();
    locator.insert(
        "id",
        crate::completed_rows::membership_id(contribution, relation, &key),
    );
    locator.insert(
        "contribution",
        RecordId::new("compiler_contribution", contribution.hex()),
    );
    locator.insert("relation", relation.to_owned());
    locator.insert("semantic_key", hex::encode(key));
    locator.insert("node", record(object, "id")?.clone());
    locator.insert("content", text(object, "content")?.to_owned());
    locator.insert("storage", "graph");
    locator.insert(
        "scope_keys",
        object
            .get("scope_keys")
            .cloned()
            .ok_or(ModelError::Schema("graph locator scope"))?,
    );
    locator.insert("scope_lookup", locator_scope_lookup(&locator)?);
    Ok(Value::Object(locator))
}
pub struct LocatorWindow {
    rows: BTreeMap<RecordId, Option<Value>>,
    _charge: lctx_model::domain::charged::StateCharge,
}
impl LocatorWindow {
    pub fn take_optional(&mut self, id: &RecordId) -> Result<Option<Value>, ModelError> {
        self.rows
            .remove(id)
            .ok_or(ModelError::Conflict("unrequested logical locator"))
    }
    pub fn finish(self) -> Result<(), ModelError> {
        if self.rows.is_empty() {
            Ok(())
        } else {
            Err(ModelError::Conflict("unconsumed logical locators"))
        }
    }
}
/// Locator lifetime follows its protected contribution inventory, not a per-row
/// generic guard/hold. Missing requested keys remain explicit optional observations.
pub async fn read_locator_window_read<Context>(
    reader: &crate::NativeReader<Context>,
    ids: &[RecordId],
    budget: &ResourceBudget,
) -> Result<LocatorWindow, ModelError> {
    if ids.len() > crate::loader::NATIVE_WINDOW_ROWS {
        return Err(ModelError::Schema("locator request window"));
    }
    let mut charge =
        lctx_model::domain::charged::StateCharge::new(budget, "logical locator observations");
    charge.grow(ids.len().saturating_mul(2048))?;
    let mut rows = BTreeMap::new();
    for id in ids {
        if id.table.as_str() != "compiler_membership" || rows.insert(id.clone(), None).is_some() {
            return Err(ModelError::Schema("locator exact requested identities"));
        }
    }
    if ids.is_empty() {
        return Ok(LocatorWindow {
            rows,
            _charge: charge,
        });
    }
    let mut variables = surrealdb::types::Variables::new();
    variables.insert("locator_ids", ids.to_vec());
    let mut stream = reader.stream_prepared(crate::prepared::PreparedQuery::new(
        variables,
        vec![],
        vec!["SELECT * FROM $locator_ids".into()],
    )?)?;
    let result = async {
        while let Some(row) = stream.next().await? {
            validate_locator(&row)?;
            let id = record(object(&row)?, "id")?;
            let prior = rows
                .get_mut(id)
                .ok_or(ModelError::Conflict("foreign logical locator"))?;
            if prior.is_some() {
                return Err(ModelError::Conflict("duplicate logical locator"));
            }
            charge.grow(
                crate::loader::native_bytes(&row)
                    .saturating_mul(8)
                    .saturating_add(512),
            )?;
            *prior = Some(row);
        }
        Ok(())
    }
    .await;
    let mut terminal = lctx_model::domain::completion::Completion::default();
    terminal.step(
        "logical locator set drainage",
        stream.drain_transport().await,
    );
    lctx_model::domain::completion::complete(result, terminal)?;
    Ok(LocatorWindow {
        rows,
        _charge: charge,
    })
}
fn finish_block(
    contribution: ContentHash,
    relation: &str,
    rows: Vec<Value>,
    blocks: &mut Vec<Value>,
    locators: &mut Vec<Value>,
) -> Result<(), ModelError> {
    let content = block_content(relation, &rows)?;
    let id = RecordId::new("compiler_block", content.hex());
    for (slot, row) in rows.iter().enumerate() {
        let object = object(row)?;
        let key = crate::completed_rows::decode_nominal_key(text(object, "semantic_key")?)?;
        let mut locator = Object::new();
        locator.insert(
            "id",
            crate::completed_rows::membership_id(contribution, relation, &key),
        );
        locator.insert(
            "contribution",
            RecordId::new("compiler_contribution", contribution.hex()),
        );
        locator.insert("relation", relation.to_owned());
        locator.insert("semantic_key", hex::encode(key));
        locator.insert("node", record(object, "id")?.clone());
        locator.insert("content", text(object, "content")?.to_owned());
        locator.insert("storage", "block");
        locator.insert("block", id.clone());
        locator.insert("slot", i64::try_from(slot).map_err(ModelError::codec)?);
        locator.insert(
            "scope_keys",
            object
                .get("scope_keys")
                .cloned()
                .ok_or(ModelError::Schema("compiler row scopes"))?,
        );
        locator.insert("scope_lookup", locator_scope_lookup(&locator)?);
        locators.push(Value::Object(locator));
    }
    let mut block = Object::new();
    block.insert("id", id);
    block.insert("format", FORMAT);
    block.insert("relation", relation.to_owned());
    block.insert("content", content.hex());
    block.insert("rows", rows);
    let block = Value::Object(block);
    crate::loader::validate_native_row(&block)?;
    blocks.push(block);
    Ok(())
}
/// Independently validate the full container framing, content and each canonical
/// row. The receiving semantic owner still validates typed rows and membership.
pub fn validate_block(block: &Value, budget: &ResourceBudget) -> Result<(), ModelError> {
    let object = object(block)?;
    if object.len() != 5 || text(object, "format")? != FORMAT {
        return Err(ModelError::Schema("compiler block format/envelope"));
    }
    crate::loader::validate_native_row(block)?;
    let relation = text(object, "relation")?;
    let rows = match object.get("rows") {
        Some(Value::Array(rows)) if !rows.is_empty() => rows,
        _ => return Err(ModelError::Schema("compiler block nonempty rows")),
    };
    // Container hashing and canonical-byte comparison borrow existing buffers.
    // Only the current row's small projection/field workspace is transient; a
    // second whole-container expansion would double-charge the retained read.
    let largest = rows
        .iter()
        .map(crate::loader::native_bytes)
        .max()
        .unwrap_or(0);
    let _decode = budget.reserve(
        "compiler block independent framing",
        largest.saturating_mul(2).saturating_add(65536),
    )?;
    let content = block_content(relation, rows)?;
    if text(object, "content")? != content.hex()
        || record(object, "id")? != &RecordId::new("compiler_block", content.hex())
    {
        return Err(ModelError::Conflict("compiler block content identity"));
    }
    let mut previous = None;
    for row in rows {
        let object = frame(row)?;
        crate::completed_rows::validate_state_row("compiler_record", row)?;
        let key = text(object, "semantic_key")?;
        if text(object, "semantic_type")? != relation
            || previous.is_some_and(|before| before >= key)
        {
            return Err(ModelError::Conflict("compiler block row relation/order"));
        }
        previous = Some(key);
    }
    Ok(())
}
/// Reconstruct only named logical members, retaining virtual IDs in their complete
/// envelopes. Other slots are valid shared container contents, not selected members.
pub fn selected_row<'a>(block: &'a Value, locator: &Value) -> Result<&'a Value, ModelError> {
    let member = object(locator)?;
    let Location::Block { record: id, slot } = PhysicalLocation::from_locator(locator)?.0 else {
        return Err(ModelError::Schema("selected block locator"));
    };
    let block = object(block)?;
    if record(block, "id")? != &id {
        return Err(ModelError::Conflict("foreign compiler block locator"));
    }
    let Some(Value::Array(rows)) = block.get("rows") else {
        return Err(ModelError::Schema("compiler block rows"));
    };
    let row = rows.get(slot).ok_or(ModelError::Conflict(
        "compiler block locator slot outside container",
    ))?;
    let actual = object(row)?;
    if member.get("node") != actual.get("id")
        || member.get("relation") != actual.get("semantic_type")
        || member.get("semantic_key") != actual.get("semantic_key")
        || member.get("content") != actual.get("content")
        || member.get("scope_keys") != actual.get("scope_keys")
    {
        return Err(ModelError::Conflict(
            "compiler block selected correspondence",
        ));
    }
    Ok(row)
}
/// Strip physical locator properties for the unchanged v4 logical membership.
pub fn logical_membership(locator: &Value) -> Result<Value, ModelError> {
    let locator = object(locator)?;
    let mut logical = Object::new();
    for name in [
        "id",
        "contribution",
        "relation",
        "semantic_key",
        "node",
        "content",
    ] {
        logical.insert(
            name,
            locator
                .get(name)
                .cloned()
                .ok_or(ModelError::Schema("logical locator field"))?,
        );
    }
    Ok(Value::Object(logical))
}
pub struct LocatedBacking {
    physical: Arc<crate::control::ExpectedRecord>,
    location: PhysicalLocation,
}
impl LocatedBacking {
    pub fn row(&self) -> &Value {
        match &self.location.0 {
            Location::Graph(_) => self.physical.row(),
            Location::Block { slot, .. } => {
                let block = self
                    .physical
                    .row()
                    .as_object()
                    .expect("checked physical block");
                let Some(Value::Array(rows)) = block.get("rows") else {
                    unreachable!("checked block rows");
                };
                &rows[*slot]
            }
        }
    }
    pub fn physical_record(&self) -> &RecordId {
        self.location.record()
    }
    pub fn incarnation(&self) -> i64 {
        self.physical.incarnation()
    }
    pub fn expected_physical(&self) -> &crate::control::ExpectedRecord {
        &self.physical
    }
    pub fn location(&self) -> &PhysicalLocation {
        &self.location
    }
    pub fn into_owned_row(
        self,
        budget: &ResourceBudget,
    ) -> Result<(Value, Box<dyn Reservation>), ModelError> {
        // This boundary transfers one native Value, not another decoder/transaction
        // arena. Downstream Arrow and wire consumers admit their own workspaces.
        let charge = budget.reserve(
            "selected logical block row transfer",
            crate::loader::native_bytes(self.row())
                .saturating_mul(2)
                .saturating_add(4096),
        )?;
        let Self { physical, location } = self;
        let row = match Arc::try_unwrap(physical) {
            Ok(physical) => {
                let (value, original_arena) = physical.into_owned_row();
                let row = match location.0 {
                    Location::Graph(_) => value,
                    Location::Block { slot, .. } => {
                        let Value::Object(mut block) = value else {
                            unreachable!("checked physical block");
                        };
                        let Some(Value::Array(rows)) = block.remove("rows") else {
                            unreachable!("checked block rows");
                        };
                        rows.into_iter().nth(slot).expect("checked selected slot")
                    }
                };
                drop(original_arena);
                row
            }
            Err(physical) => Self { physical, location }.row().clone(),
        };
        Ok((row, charge))
    }
}
pub struct BackingWindow {
    rows: BTreeMap<RecordId, LocatedBacking>,
    _charge: Box<dyn Reservation>,
}
impl BackingWindow {
    pub fn get(&self, logical: &RecordId) -> Result<&LocatedBacking, ModelError> {
        self.rows
            .get(logical)
            .ok_or(ModelError::Conflict("selected logical backing missing"))
    }
    pub fn take_required(&mut self, logical: &RecordId) -> Result<LocatedBacking, ModelError> {
        self.rows
            .remove(logical)
            .ok_or(ModelError::Conflict("selected logical backing missing"))
    }
}
/// Logical membership and physical placement are separate codec contracts.
/// Portable logical imports deliberately have no placement; installation earns one.
pub fn validate_locator(row: &Value) -> Result<PhysicalLocation, ModelError> {
    let logical = logical_membership(row)?;
    crate::completed_rows::validate_state_row("compiler_membership", &logical)?;
    let location = PhysicalLocation::from_locator(row)?;
    let object = object(row)?;
    let fields = if matches!(location.0, Location::Graph(_)) {
        9
    } else {
        11
    };
    if object.len() != fields
        || !matches!(object.get("scope_keys"),Some(Value::Array(keys)) if keys.iter().all(|key|matches!(key,Value::String(_))))
    {
        return Err(ModelError::Schema(
            "compiler locator exact physical envelope",
        ));
    }
    let owner = record(object, "contribution")?;
    let relation = text(object, "relation")?;
    let (Some(Value::Array(scopes)), Some(Value::Array(lookup))) =
        (object.get("scope_keys"), object.get("scope_lookup"))
    else {
        return Err(ModelError::Schema("compiler locator scope lookup"));
    };
    if scopes.len() != lookup.len()
        || scopes.iter().zip(lookup.iter()).any(|(scope, entry)| {
            let Value::Array(entry) = entry else {
                return true;
            };
            entry.len() != 3
                || !matches!(&entry[0],Value::RecordId(actual) if actual==owner)
                || !matches!(&entry[1],Value::String(actual) if actual==relation)
                || entry[2] != *scope
        })
    {
        return Err(ModelError::Conflict(
            "compiler locator exact qualified scope lookup",
        ));
    }
    Ok(location)
}
/// A single native array-index key retains the complete qualification tuple.
/// The pinned planner supports containment on the leading indexed column only;
/// tuple elements avoid a broad scalar-prefix scan followed by a token filter.
fn locator_scope_lookup(row: &Object) -> Result<Value, ModelError> {
    let owner = record(row, "contribution")?;
    let relation = text(row, "relation")?;
    let Some(Value::Array(scopes)) = row.get("scope_keys") else {
        return Err(ModelError::Schema("compiler locator scope keys"));
    };
    let entries = scopes
        .iter()
        .map(|scope| {
            let Value::String(scope) = scope else {
                return Err(ModelError::Schema("compiler locator scope token"));
            };
            Ok(Value::Array(
                vec![
                    Value::RecordId(owner.clone()),
                    Value::String(relation.to_owned()),
                    Value::String(scope.clone()),
                ]
                .into(),
            ))
        })
        .collect::<Result<Vec<_>, ModelError>>()?;
    Ok(Value::Array(entries.into()))
}
pub async fn inspect_backing_window_read<Context>(
    reader: &crate::NativeReader<Context>,
    members: &[Value],
    budget: &ResourceBudget,
) -> Result<BackingWindow, ModelError> {
    inspect_backing_page_read(reader, members, budget)
        .await
        .map_err(crate::control::held::PageAcquireFailure::into_error)
}
pub(crate) async fn inspect_backing_page_read<Context>(
    reader: &crate::NativeReader<Context>,
    members: &[Value],
    budget: &ResourceBudget,
) -> Result<BackingWindow, crate::control::held::PageAcquireFailure> {
    use crate::control::held::{PageAcquireFailure, inspect_expected_page_read};
    if members.len() > crate::loader::NATIVE_WINDOW_ROWS {
        return Err(ModelError::Schema("logical backing locator window").into());
    }
    let charge = budget
        .reserve(
            "logical backing physical correspondence",
            members.len().saturating_mul(4096),
        )
        .map_err(PageAcquireFailure::reducible_resource)?;
    let mut ids = BTreeSet::new();
    let mut locations = Vec::with_capacity(members.len());
    for member in members {
        let location = validate_locator(member)?;
        ids.insert(location.record().clone());
        locations.push(location);
    }
    let ids = ids.into_iter().collect::<Vec<_>>();
    let mut physical = inspect_expected_page_read(reader, &ids, budget).await?;
    let mut retained = BTreeMap::new();
    for id in ids {
        let actual = physical.take_required(&id)?;
        if id.table.as_str() == "compiler_block" {
            validate_block(actual.row(), budget).map_err(PageAcquireFailure::reducible_resource)?;
        }
        retained.insert(id, Arc::new(actual));
    }
    physical.finish()?;
    let mut rows = BTreeMap::new();
    for (member, location) in members.iter().zip(locations) {
        let physical = retained
            .get(location.record())
            .ok_or(ModelError::Conflict("located physical backing absent"))?
            .clone();
        let located = LocatedBacking { physical, location };
        let object = object(member)?;
        let logical = record(object, "node")?;
        if matches!(located.location.0, Location::Block { .. }) {
            selected_row(located.physical.row(), member)?;
        } else {
            let actual = located
                .row()
                .as_object()
                .ok_or(ModelError::Schema("logical graph backing object"))?;
            if actual.get("id") != Some(&Value::RecordId(logical.clone()))
                || actual.get("semantic_type") != object.get("relation")
                || actual.get("semantic_key") != object.get("semantic_key")
                || actual.get("content") != object.get("content")
                || actual.get("scope_keys") != object.get("scope_keys")
            {
                return Err(ModelError::Conflict("logical graph locator correspondence").into());
            }
        }
        if let Some(previous) = rows.get(logical) {
            let previous: &LocatedBacking = previous;
            if previous.row() != located.row() {
                return Err(
                    ModelError::Conflict("conflicting logical backing across blocks").into(),
                );
            }
        } else {
            rows.insert(logical.clone(), located);
        }
    }
    Ok(BackingWindow {
        rows,
        _charge: charge,
    })
}
fn frame(row: &Value) -> Result<&Object, ModelError> {
    crate::loader::validate_native_row(row)?;
    let object = object(row)?;
    if record(object, "id")?.table.as_str() != "compiler_record"
        || !matches!(object.get("canonical"), Some(Value::Bytes(_)))
        || !matches!(object.get("body"), Some(Value::Object(_)))
        || !matches!(object.get("scope_keys"), Some(Value::Array(_)))
    {
        return Err(ModelError::Schema("compiler block canonical envelope"));
    }
    text(object, "semantic_type")?;
    crate::completed_rows::decode_nominal_key(text(object, "semantic_key")?)?;
    text(object, "content")?;
    Ok(object)
}
fn object_ref(row: &Value) -> Result<&Object, ModelError> {
    object(row)
}
fn object(row: &Value) -> Result<&Object, ModelError> {
    row.as_object()
        .ok_or(ModelError::Schema("compiler block object"))
}
fn text<'a>(object: &'a Object, name: &str) -> Result<&'a str, ModelError> {
    match object.get(name) {
        Some(Value::String(value)) => Ok(value.as_str()),
        _ => Err(ModelError::Schema("compiler block string")),
    }
}
fn record<'a>(object: &'a Object, name: &str) -> Result<&'a RecordId, ModelError> {
    match object.get(name) {
        Some(Value::RecordId(id)) => Ok(id),
        _ => Err(ModelError::Schema("compiler block record")),
    }
}
#[derive(Default)]
struct EncodingSize(usize);
impl Write for EncodingSize {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        self.0 = self
            .0
            .checked_add(bytes.len())
            .ok_or_else(|| std::io::Error::other("compiler block encoding overflow"))?;
        Ok(bytes.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}
fn block_content(relation: &str, rows: &[Value]) -> Result<ContentHash, ModelError> {
    struct Digest(ContentHasher);
    impl Write for Digest {
        fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
            self.0.update(bytes);
            Ok(bytes.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    let mut content = Digest(ContentHasher::default());
    serde_json::to_writer(&mut content, &(FORMAT, relation, rows)).map_err(ModelError::codec)?;
    Ok(content.0.finish())
}
fn encoded_size(row: &Value) -> Result<usize, ModelError> {
    let mut size = EncodingSize::default();
    serde_json::to_writer(&mut size, row).map_err(ModelError::codec)?;
    Ok(size.0)
}

#[cfg(test)]
mod controls;
