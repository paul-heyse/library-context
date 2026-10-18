//! Format6 write admission and exact physical expectations. No cross-owner epoch ordering.
use super::*;
use authorization::int_field;
use lctx_model::domain::resources::TRANSFER_BYTES;

pub const ADMISSION_STRIPES: usize = 64;
pub(super) const STRIPE_FORMAT: i64 = 1;
pub(super) fn stripe_id(operation: ContentHash) -> RecordId {
    let hash = ContentHash::of(&operation.0);
    RecordId::new("native_admission_stripe", i64::from(hash.0[0] & 63))
}
pub(super) fn stripe_ids() -> Vec<RecordId> {
    (0..ADMISSION_STRIPES)
        .map(|index| RecordId::new("native_admission_stripe", index as i64))
        .collect()
}
pub(super) fn operation_owner(request: NativeRequest) -> RecordId {
    let mut sink = KeySink::new("native-installation-operation-owner/v6");
    request.issuance.generation.encode(&mut sink);
    request.operation.encode(&mut sink);
    RecordId::new("native_operation_owner", sink.finish().hex())
}
pub(super) fn bind_stripe(bindings: &mut Variables, operation: ContentHash) {
    bindings.insert("__stripe", stripe_id(operation));
}
pub(super) fn bind_all_stripes(bindings: &mut Variables) {
    bindings.insert("__stripes", stripe_ids());
}
/// Attempt admission uses the same finite width, but separate exact child IDs.
/// Persist the operation's slot in its durable participant so payload/recovery
/// never choose a different conflict address for the same original request.
pub(super) fn attempt_slot(operation: ContentHash) -> i64 {
    i64::from(ContentHash::of(&operation.0).0[0] & 63)
}
#[cfg(test)]
pub(super) fn attempt_stripe_ids(attempt: &RecordId) -> Vec<RecordId> {
    (0..ADMISSION_STRIPES)
        .map(|slot| {
            RecordId::new(
                "native_attempt_stripe",
                surrealdb::types::RecordIdKey::Array(
                    vec![
                        Value::RecordId(attempt.clone()),
                        surrealdb::types::SurrealValue::into_value(slot as i64),
                    ]
                    .into(),
                ),
            )
        })
        .collect()
}
// The marker read carries configuration only. Ordinary effects WRITE the selected
// stripe so both commit orders exclude an overlapping installation cut. A terminal
// recovery-reference DELETE only removes protection; its locked read must fail
// after a cut commits, but release-first cannot invalidate the cut's drain proof.
// Keep one exact check literal for both routes.
macro_rules! issuance_stripe_checks {
    () => { "LET $__installation=SELECT * FROM ONLY native_installation:current; LET $__admission=SELECT * FROM ONLY $__stripe FOR UPDATE; IF $__installation=NONE OR $__installation.schema_version!=11 OR $__installation.generation!=$__generation OR $__admission=NONE OR $__admission.format!=1 OR $__admission.generation!=$__generation OR $__installation.era!=$__admission.era OR $__installation.closed_through!=$__admission.closed_through OR $__installation.admission_open!=$__admission.admission_open OR $__admission.era!=$__era OR $__era<=$__admission.closed_through OR $__admission.revision>=9223372036854775807 { THROW 'native issuance stripe fenced'; };" };
}
pub(super) const ERA_GUARD: &str = concat!(
    issuance_stripe_checks!(),
    " UPDATE $__stripe SET revision+=1 RETURN NONE;"
);
pub(super) const TERMINAL_RELEASE_ERA_GUARD: &str = issuance_stripe_checks!();
pub(super) const ALL_STRIPES_GUARD: &str = "IF array::len($__stripes)!=64 { THROW 'native stripe set width'; }; FOR $__stripe_id IN $__stripes { LET $__state=SELECT * FROM ONLY $__stripe_id FOR UPDATE; IF $__state=NONE OR $__state.format!=1 OR $__state.generation!=$__generation OR $__state.era!=$__era OR $__state.closed_through!=$__installation.closed_through OR $__state.admission_open!=$__installation.admission_open OR $__state.revision>=9223372036854775807 { THROW 'native all-stripe authority'; }; UPDATE $__stripe_id SET revision+=1 RETURN NONE; };";
// One literal serves the actual production drain guard and its isolated native
// control, so unrelated attempt/effect borrowers cannot mask pin exclusion.
macro_rules! live_pin_drain_predicate {
    () => { "array::len(SELECT VALUE id FROM native_pin WITH INDEX live_pins WHERE released=false LIMIT 1)>0" };
}
pub(super) const DRAINED_PREDICATE: &str = concat!(
    "IF array::len(SELECT VALUE id FROM native_effect WITH INDEX live_effects WHERE resolved=false AND id!=$__effect LIMIT 1)>0 OR array::len(SELECT VALUE id FROM native_operation WITH INDEX operation_history WHERE state='open' LIMIT 1)>0 OR array::len(SELECT VALUE id FROM native_operation_page WITH INDEX live_operation_pages WHERE resolved=false LIMIT 1)>0 OR array::len(SELECT VALUE id FROM native_attempt WITH INDEX live_attempts WHERE state IN ['open','frozen','closing'] LIMIT 1)>0 OR ",
    live_pin_drain_predicate!(),
    " OR array::len(SELECT VALUE id FROM native_backup_hold WITH INDEX live_backups WHERE active=true LIMIT 1)>0 OR array::len(SELECT VALUE id FROM native_retirement WITH INDEX live_retirements WHERE state='registering' LIMIT 1)>0 { THROW 'native installation borrowers remain'; };"
);
#[cfg(test)]
pub(super) fn live_pin_drain_predicate_for_control() -> &'static str {
    live_pin_drain_predicate!()
}

/// Bounded hold-page lowering, shared by cleanup and pin release. Callers supply
/// distinct exact `__page_incidence_targets`, excluding only targets already cut
/// by their checked owner. Attempts keep their complete cut and predicate write;
/// ordinary predicate records use an exact point-set read and split bulk mutations.
/// These arrays are derived inside the same bounded native packet, not a host cache.
pub(super) const PREPARE_PAGE_INCIDENCE: &str = r#"
FOR $__page_attempt IN $__page_incidence_targets.filter(|$record| record::table($record)='native_attempt') { fn::native_touch($__page_attempt); };
LET $__page_predicate_rows=$__page_incidence_targets.filter(|$record| record::table($record)!='native_attempt').map(|$record| {id:type::record('native_predicate',[$record]),revision:1});
LET $__page_predicate_ids=$__page_predicate_rows.map(|$row| $row.id);
LET $__page_actual_predicates=SELECT id,revision FROM $__page_predicate_ids;
LET $__page_existing_predicate_ids=$__page_actual_predicates.map(|$row|$row.id);
LET $__page_missing_predicate_rows=$__page_predicate_rows.filter(|$row|$row.id NOT IN $__page_existing_predicate_ids);
IF array::len(SELECT * FROM $__page_actual_predicates WHERE revision>=9223372036854775807)>0 { THROW 'native predicate revision exhausted'; };
"#;
/// Execute only after all page-specific checks; every endpoint changes once and
/// all item creation/hold deletion remains in this same owner effect transaction.
// Every exact predicate is really written. Backend write conflicts preserve
// exclusion; duplicate inserts must not create per-row rollback savepoints.
pub(super) const WRITE_PAGE_INCIDENCE: &str = "IF array::len($__page_existing_predicate_ids)>0 { UPDATE $__page_existing_predicate_ids SET revision+=1 RETURN NONE; }; IF array::len($__page_missing_predicate_rows)>0 { INSERT INTO native_predicate $__page_missing_predicate_rows RETURN NONE; };";

/// Size the complete lowered query with the pinned native protobuf codec. Context
/// uses the maximum encoded widths of UUID/timeout/batch fields, so this is a wire
/// bound, independently of resident construction/serialization workspace admission.
/// The atomic request must pass before its first remote effect.
fn query_wire_bound(sql: &str, bindings: &Variables) -> Result<usize, ModelError> {
    use prost::Message;
    use surrealdb_protocol::proto::{rpc::v1 as rpc, v1 as proto};
    let variables = bindings
        .iter()
        .map(|(key, value)| {
            Ok(proto::KeyValue {
                key: key.clone(),
                value: Some(proto::Value::try_from(value.clone()).map_err(ModelError::codec)?),
            })
        })
        .collect::<Result<Vec<_>, ModelError>>()?;
    let uuid = proto::Uuid {
        bytes: vec![0; 16].into(),
    };
    let request = rpc::QueryRequest {
        context: Some(rpc::RequestContext {
            session: Some(uuid.clone()),
            transaction: Some(uuid),
            timeout: Some(proto::Duration {
                seconds: i64::MIN,
                nanos: 999_999_999,
            }),
        }),
        query: sql.into(),
        variables: Some(proto::Variables { variables }),
        accepted_encodings: Vec::new(),
        max_batch_records: u32::MAX,
        max_batch_bytes: u32::MAX,
    };
    Ok(request.encoded_len())
}
pub(crate) fn preflight(sql: &str, bindings: &Variables) -> Result<(), ModelError> {
    let bytes = query_wire_bound(sql, bindings)?;
    if bytes > TRANSFER_BYTES {
        return Err(ModelError::Limit {
            owner: "native transaction",
            limit: "complete control and payload bytes",
            observed: bytes,
            bound: TRANSFER_BYTES,
        });
    }
    Ok(())
}
pub(crate) fn initial_stripes(
    generation: ContentHash,
    era: i64,
    closed: i64,
    open: bool,
) -> Result<Vec<Value>, ModelError> {
    if era <= closed || closed < 0 || ADMISSION_STRIPES + 2 > crate::loader::NATIVE_WINDOW_ROWS {
        return Err(ModelError::Schema("native admission stripe initialization"));
    }
    let mut rows = Vec::with_capacity(ADMISSION_STRIPES);
    for id in stripe_ids() {
        let mut row = Object::new();
        row.insert("id", id);
        row.insert("format", STRIPE_FORMAT);
        row.insert("generation", generation.hex());
        row.insert("era", era);
        row.insert("closed_through", closed);
        row.insert("admission_open", open);
        row.insert("revision", 0i64);
        let value = Value::Object(row);
        if serde_json::to_vec(&value).map_err(ModelError::codec)?.len() > 1024
            || crate::loader::native_bytes(&value) > 1024
        {
            return Err(ModelError::Schema("native admission stripe row width"));
        }
        rows.push(value);
    }
    Ok(rows)
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExpectedObject {
    pub record: RecordId,
    pub incarnation: i64,
    pub phase: String,
    pub tombstone_revision_if_vacant: Option<i64>,
}
impl ExpectedObject {
    fn from_guard(record: RecordId, row: Option<&Object>) -> Result<Self, ModelError> {
        let Some(row) = row else {
            return Ok(Self {
                record,
                incarnation: 0,
                phase: "vacant".into(),
                tombstone_revision_if_vacant: Some(0),
            });
        };
        let incarnation = int_field(row, "incarnation")?;
        let Some(Value::String(phase)) = row.get("phase") else {
            return Err(ModelError::Schema("native expected phase"));
        };
        if incarnation <= 0
            || !matches!(phase.as_str(), "active" | "retired")
            || row.get("retired") != Some(&Value::Bool(phase == "retired"))
            || int_field(row, "revision")? < 0
            || int_field(row, "tombstone_revision")? < 0
        {
            return Err(ModelError::Conflict("native expected object not writable"));
        }
        let tombstone = if phase == "retired" {
            Some(int_field(row, "tombstone_revision")?)
        } else {
            None
        };
        if tombstone.is_some_and(|revision| revision < 0) {
            return Err(ModelError::Schema("native expected tombstone"));
        }
        Ok(Self {
            record,
            incarnation,
            phase: phase.clone(),
            tombstone_revision_if_vacant: tombstone,
        })
    }
    pub(super) fn binding(&self, guard: RecordId, creation: Option<&Value>) -> Value {
        let mut row = Object::new();
        row.insert("record", self.record.clone());
        row.insert("guard", guard);
        row.insert("incarnation", self.incarnation);
        row.insert("phase", self.phase.clone());
        row.insert(
            "tombstone_revision",
            self.tombstone_revision_if_vacant
                .map(surrealdb::types::SurrealValue::into_value)
                .unwrap_or(Value::None),
        );
        row.insert("creation", creation.cloned().unwrap_or(Value::None));
        row.insert("deferred_incidence", false);
        Value::Object(row)
    }
}
/// Generic effects have no WindowWriter complete-workspace grant. Reserve the
/// additional host expectation preparation from their owning recovery budget.
pub(super) fn reserve_expectation_preparation(
    bindings: &Variables,
    budget: &lctx_model::domain::resources::ResourceBudget,
) -> Result<Box<dyn lctx_model::domain::resources::Reservation>, ModelError> {
    budget.reserve(
        "native expectation preparation",
        WindowWriter::expectation_workspace_bytes(bindings)?,
    )
}
/// Internal plans declare physical guard IDs. Capture once before request hashing;
/// neither transaction retries nor execute_request refresh these observations.
pub(super) async fn prepare_expectations(
    client: &Surreal<Client>,
    bindings: &mut Variables,
) -> Result<(), ModelError> {
    if bindings.get("__object_expectations").is_some() {
        return classify_expectations(bindings);
    }
    let mut guards = std::collections::BTreeSet::new();
    for key in ["guards", "hold_guards", "__write_guards"] {
        if let Some(Value::Array(ids)) = bindings.get(key) {
            for value in ids {
                match value {
                    Value::RecordId(id) if id.table.as_str() == "native_guard" => {
                        guards.insert(id.clone());
                    }
                    _ => return Err(ModelError::Schema("native expected physical guard")),
                }
            }
        }
    }
    if guards.is_empty() {
        bindings.insert("__object_expectations", Vec::<Value>::new());
        return Ok(());
    }
    // Guard IDs are exact physical identities; existing callers already own their record
    // mapping. Do not reconstruct IDs from debug/string representations.
    let mut actual = std::collections::BTreeMap::new();
    let ids = guards.iter().cloned().collect::<Vec<_>>();
    for window in ids.chunks(crate::loader::NATIVE_WINDOW_ROWS) {
        let mut response = client
            .query("SELECT * FROM $guards")
            .bind(("guards", window.to_vec()))
            .await
            .map_err(ModelError::codec)?
            .check()
            .map_err(ModelError::codec)?;
        let rows: Vec<Value> = response.take(0).map_err(ModelError::codec)?;
        let mut admitted = crate::loader::NativeWindows::new(rows);
        let rows = admitted.next().transpose()?.unwrap_or_default();
        if admitted.next().is_some() {
            return Err(ModelError::Schema("native expected guard window width"));
        }
        for value in rows {
            let Value::Object(row) = value else {
                return Err(ModelError::Schema("native expected guard row"));
            };
            let Some(Value::RecordId(id)) = row.get("id") else {
                return Err(ModelError::Schema("native expected guard identity"));
            };
            if !guards.contains(id) || actual.insert(id.clone(), row.clone()).is_some() {
                return Err(ModelError::Conflict("native expected guard cardinality"));
            }
        }
    }
    let Some(Value::Array(records)) = bindings.get("__guard_records") else {
        return Err(ModelError::Schema("native expected record map missing"));
    };
    let mut mapping = std::collections::BTreeMap::new();
    for value in records {
        let Value::Object(row) = value else {
            return Err(ModelError::Schema("native expected record map"));
        };
        let (Some(Value::RecordId(guard)), Some(Value::RecordId(record))) =
            (row.get("guard"), row.get("record"))
        else {
            return Err(ModelError::Schema("native expected record identity"));
        };
        if guard_id(&Value::RecordId(record.clone()))? != *guard {
            return Err(ModelError::Conflict("native expected guard correspondence"));
        }
        if let Some(prior) = mapping.insert(guard.clone(), record.clone())
            && prior != *record
        {
            return Err(ModelError::Conflict("native expected record collision"));
        }
    }
    let creations = creation_indices(bindings)?;
    let expected = guards
        .into_iter()
        .map(|guard| {
            let record = mapping
                .get(&guard)
                .ok_or(ModelError::Schema("native expected record absent"))?;
            ExpectedObject::from_guard(record.clone(), actual.get(&guard))
                .map(|expected| expected.binding(guard, creations.get(record)))
        })
        .collect::<Result<Vec<_>, _>>()?;
    bindings.insert("__object_expectations", expected);
    classify_expectations(bindings)
}
/// Attach packet-local incidence metadata without refreshing physical observations.
/// The source arrays retain every candidate; compound native numeric equality
/// supplies no established lawful generic hash or representative-collapse contract.
pub(super) fn classify_expectations(bindings: &mut Variables) -> Result<(), ModelError> {
    struct Membership<'a> {
        values: &'a surrealdb::types::Array,
        field: Option<&'static str>,
        index: exact_index::ExactRecordIndex,
    }
    impl<'a> Membership<'a> {
        fn new(
            bindings: &'a Object,
            name: &str,
            field: Option<&'static str>,
        ) -> Result<Self, ModelError> {
            static EMPTY: std::sync::LazyLock<surrealdb::types::Array> =
                std::sync::LazyLock::new(Default::default);
            let values = match bindings.get(name) {
                Some(Value::Array(values)) => values,
                None => &EMPTY,
                _ => return Err(ModelError::Schema("native incidence membership array")),
            };
            let mut index = exact_index::ExactRecordIndex::default();
            for (position, value) in values.iter().enumerate() {
                if field == Some("blob")
                    && exact_index::field(value, "id")
                        .is_none_or(|id| id.table.as_str() != "original_chunk")
                {
                    continue;
                }
                let id = field.map_or_else(
                    || exact_index::record(value),
                    |name| exact_index::field(value, name),
                );
                index.insert(id, position);
            }
            Ok(Self {
                values,
                field,
                index,
            })
        }
        fn contains(&self, id: &RecordId) -> bool {
            self.index.candidates(Some(id)).iter().any(|&position| {
                let value = &self.values[position];
                self.field.map_or_else(
                    || exact_index::record(value),
                    |name| exact_index::field(value, name),
                ) == Some(id)
            })
        }
    }
    // Variables exposes immutable access only. Object conversion moves the same
    // BTreeMap backing, allowing independent expectation/source borrows without
    // cloning the packet or reallocating its nodes.
    let mut packet: Object = std::mem::take(bindings).into();
    let mut expectations = match packet.remove("__object_expectations") {
        Some(Value::Array(values)) => values,
        Some(value) => {
            packet.insert("__object_expectations", value);
            *bindings = packet.into();
            return Err(ModelError::Schema("native object expectations"));
        }
        None => {
            *bindings = packet.into();
            return Err(ModelError::Schema("native object expectations absent"));
        }
    };
    let result = (|| {
        let mutable = Membership::new(&packet, "__mutable_write_guards", None)?;
        let holds = Membership::new(&packet, "hold_guards", None)?;
        let guards = Membership::new(&packet, "guards", None)?;
        let writes = Membership::new(&packet, "__write_guards", None)?;
        let blobs = Membership::new(&packet, "rows", Some("blob"))?;
        for value in expectations.iter_mut() {
            let Value::Object(expected) = value else {
                return Err(ModelError::Schema("native expected incidence envelope"));
            };
            let (Some(Value::RecordId(guard)), Some(Value::RecordId(record))) =
                (expected.get("guard"), expected.get("record"))
            else {
                return Err(ModelError::Schema("native expected incidence identity"));
            };
            let deferred = !mutable.contains(guard)
                && (expected
                    .get("creation")
                    .is_some_and(|value| *value != Value::None)
                    || blobs.contains(record)
                    || (holds.contains(guard)
                        && !guards.contains(guard)
                        && !writes.contains(guard)));
            expected.insert("deferred_incidence", deferred);
        }
        Ok(())
    })();
    packet.insert("__object_expectations", expectations);
    *bindings = packet.into();
    result
}
// Only immutable plans declare these alternatives. Candidate indexes retain all
// rows and confirm original equality. Full payloads never enter an ordered key.
#[derive(Default)]
struct CreationReferences {
    values: Vec<(RecordId, Value)>,
    index: exact_index::ExactRecordIndex,
}
impl CreationReferences {
    fn get(&self, record: &RecordId) -> Option<&Value> {
        self.index
            .candidates(Some(record))
            .iter()
            .find_map(|&position| {
                let (candidate, value) = &self.values[position];
                (candidate == record).then_some(value)
            })
    }
}
fn creation_indices(bindings: &Variables) -> Result<CreationReferences, ModelError> {
    let Some(values) = bindings.get("__immutable_creations") else {
        return Ok(CreationReferences::default());
    };
    let Value::Array(values) = values else {
        return Err(ModelError::Schema("native immutable creations"));
    };
    let mut sources = Vec::new();
    for name in ["rows", "original_blobs"] {
        let rows = match bindings.get(name) {
            Some(Value::Array(rows)) => Some(rows),
            _ => None,
        };
        let mut index = exact_index::ExactRecordIndex::default();
        if let Some(rows) = rows {
            for (position, row) in rows.iter().enumerate() {
                index.insert(exact_index::field(row, "id"), position);
            }
        }
        sources.push((rows, index));
    }
    let mut output = CreationReferences::default();
    for value in values {
        let Value::Object(creation) = value else {
            return Err(ModelError::Schema("native immutable creation"));
        };
        let (Some(Value::String(kind)), Some(Value::RecordId(record))) =
            (creation.get("kind"), creation.get("record"))
        else {
            return Err(ModelError::Schema("native immutable creation envelope"));
        };
        let source = if kind == "immutable" {
            0
        } else if kind == "original_blob" && record.table.as_str() == "original_blob" {
            1
        } else {
            return Err(ModelError::Schema("native immutable creation kind"));
        };
        let (rows, index) = &sources[source];
        let rows = rows.ok_or(ModelError::Schema("native immutable creation payload"))?;
        let mut matches = index
            .candidates(Some(record))
            .iter()
            .copied()
            .filter(|&position| exact_index::field(&rows[position], "id") == Some(record));
        let first = matches
            .next()
            .ok_or(ModelError::Schema("native immutable creation row missing"))?;
        if matches.any(|position| rows[position] != rows[first]) {
            return Err(ModelError::Conflict("native immutable creation collision"));
        }
        let mut reference = Object::new();
        reference.insert("kind", ["immutable", "original_blob"][source]);
        reference.insert("index", first as i64);
        let reference = Value::Object(reference);
        if let Some(previous) = output.get(record)
            && *previous != reference
        {
            return Err(ModelError::Conflict("native immutable creation collision"));
        }
        // Keep every declaration as a candidate, including equal compound keys.
        output.index.insert(Some(record), output.values.len());
        output.values.push((record.clone(), reference));
    }
    Ok(output)
}

pub(super) const COMPILER_LOCATOR_PREPARATION: &str = r#"LET $parent=SELECT * FROM ONLY $locator_parent FOR UPDATE;
IF $parent=NONE OR $parent.completed OR $parent.attempt!=$__attempt OR $__owner.admitted { THROW 'compiler locator parent fenced'; };
LET $locator_ids=$locator_rows.map(|$row|$row.id);
LET $old_locators=SELECT * FROM $locator_ids;
LET $old_by_id=object::from_entries($old_locators.map(|$row|[<string>$row.id,$row]));
LET $new_blocks_by_id=object::from_entries($rows.map(|$row|[<string>$row.id,$row]));
FOR $row IN $locator_rows {
 IF $row.contribution!=$locator_parent OR $row.scope_lookup!=$row.scope_keys.map(|$token|[$row.contribution,$row.relation,$token]) { THROW 'compiler qualified scope lookup correspondence'; };
 LET $prior=$old_by_id[<string>$row.id] ?? NONE;
 IF $prior!=NONE {
  IF $prior.contribution!=$row.contribution OR $prior.relation!=$row.relation OR $prior.semantic_key!=$row.semantic_key OR $prior.node!=$row.node OR $prior.content!=$row.content OR $prior.scope_keys!=$row.scope_keys OR $prior.scope_lookup!=$row.scope_lookup OR $prior.storage!=$row.storage { THROW 'compiler logical member collision'; };
 };
};
// Group only bounded locator metadata. Each loop owns one old container and
// compares its requested full rows immediately; old containers never accumulate.
LET $new_locators_by_id=object::from_entries($locator_rows.map(|$row|[<string>$row.id,$row]));
LET $old_groups=SELECT block,array::group({id:id,slot:slot}) AS members FROM $old_locators WHERE storage='block' GROUP BY block;
FOR $group IN $old_groups {
 LET $old_id=$group.block;
 LET $before=SELECT * FROM ONLY $old_id;
 IF $before=NONE { THROW 'compiler old block missing'; };
 FOR $prior IN $group.members {
  LET $row=$new_locators_by_id[<string>$prior.id] ?? NONE;
  LET $after=$new_blocks_by_id[<string>$row.block] ?? NONE;
  IF $row=NONE OR $after=NONE OR $before.rows[$prior.slot]!=$after.rows[$row.slot] { THROW 'compiler logical member full envelope collision'; };
 };
};
LET $new_locators=$locator_rows.filter(|$row|($old_by_id[<string>$row.id] ?? NONE)=NONE);
LET $new_block_ids=array::distinct($new_locators.filter(|$row|$row.storage='block').map(|$row|$row.block));
LET $__compiler_original_rows=$rows;
LET $rows=$rows.filter(|$row|$row.id IN $new_block_ids);
LET $holds=$holds.filter(|$hold|$hold.object IN $new_block_ids);
LET $__object_expectations=($__object_expectations ?? []).filter(|$entry| record::table($entry.record)!='compiler_block' OR $entry.record IN $new_block_ids);
"#;
pub(super) fn expected_object_guard(bindings: &Variables) -> std::borrow::Cow<'static, str> {
    if bindings.get("__compiler_locator_packet") == Some(&Value::Bool(true)) {
        std::borrow::Cow::Owned(format!(
            "{COMPILER_LOCATOR_PREPARATION} {EXPECTED_OBJECT_GUARD}"
        ))
    } else {
        std::borrow::Cow::Borrowed(EXPECTED_OBJECT_GUARD)
    }
}
pub(super) const EXPECTED_OBJECT_GUARD: &str = r#"
IF array::len(($__object_expectations ?? []).filter(|$entry|record::table($entry.record) IN ['compiler_record','compiler_membership']))>0 { THROW 'logical compiler identity is not physical expectation'; };
// Immutable creation and hold-only guards are advanced by actual changed
// incidence inside the installation function. Explicit mutable write guards
// keep their original exclusion/revision behavior.
LET $__deferred_guards=($__object_expectations ?? []).filter(|$expected| $expected.deferred_incidence=true);
FOR $__expected IN ($__object_expectations ?? []) {
    LET $__guard_id=$__expected.guard;
    LET $__before=SELECT * FROM ONLY $__guard_id FOR UPDATE;
    IF $__before!=NONE AND $__before.revision>=9223372036854775807 { THROW 'native guard revision exhausted'; };
    IF $__expected.phase='active' {
        IF $__before=NONE OR $__before.phase!='active' OR $__before.incarnation!=$__expected.incarnation { THROW 'native expected active incarnation changed'; };
        IF ($__attempt_context=NONE OR $__expected.record!=$__attempt_context.attempt) AND $__expected.deferred_incidence!=true {
            UPDATE $__guard_id SET revision+=1 RETURN NONE;
        };
    } ELSE {
        IF $__expected.phase='vacant' {
            IF $__expected.incarnation!=0 OR $__expected.tombstone_revision!=0 { THROW 'native expected vacant identity invalid'; };
        } ELSE IF $__expected.phase='retired' {
            IF $__expected.incarnation<=0 OR $__expected.tombstone_revision<0 { THROW 'native expected tombstone identity invalid'; };
        } ELSE { THROW 'native expected phase invalid'; };
        IF $__expected.incarnation>=9223372036854775807 { THROW 'native incarnation exhausted'; };
        IF $__expected.creation!=NONE AND $__before.phase='active' AND $__before.incarnation=$__expected.incarnation+1 AND $__before.tombstone_revision=$__expected.tombstone_revision AND $__before.retired=false {
            LET $__creation=$__expected.creation;
            LET $__record=$__expected.record;
            LET $__actual=SELECT * FROM ONLY $__record FOR UPDATE;
            LET $__wanted=IF $__creation.kind='immutable' { ($__compiler_original_rows ?? $rows)[$__creation.index] } ELSE { $original_blobs[$__creation.index] };
            IF $__wanted.id!=$__record OR $__actual=NONE { THROW 'native immutable creation identity changed'; };
            IF $__creation.kind='immutable' {
                IF $__actual!=$__wanted { THROW 'native immutable address collision'; };
            } ELSE IF $__creation.kind='original_blob' AND record::table($__record)='original_blob' {
                IF $__actual.id!=$__wanted.id OR $__actual.content!=$__wanted.content OR $__actual.byte_len!=$__wanted.byte_len OR $__actual.format!=$__wanted.format OR type::is_bool($__actual.completed)=false OR array::len(object::keys($__actual))!=5 { THROW 'original blob declaration collision'; };
            } ELSE { THROW 'native immutable creation kind invalid'; };
            // Equal successor bytes alone do not change their physical guard.
        } ELSE {
            IF $__expected.phase='vacant' {
                IF $__before!=NONE { THROW 'native expected vacant guard changed'; };
            } ELSE IF $__before=NONE OR $__before.phase!='retired' OR $__before.incarnation!=$__expected.incarnation OR $__before.tombstone_revision!=$__expected.tombstone_revision { THROW 'native expected tombstone changed'; };
            UPSERT $__guard_id SET phase='active',incarnation=$__expected.incarnation+1,tombstone_revision=($__before.tombstone_revision ?? 0),revision=($__before.revision ?? 0)+1,retired=false,retiring_item=NONE RETURN NONE;
        };
    };
};"#;

pub(crate) async fn set_admission(client: &Surreal<Client>, open: bool) -> Result<(), ModelError> {
    let mut bindings = Variables::new();
    bindings.insert("open", open);
    bind_all_stripes(&mut bindings);
    let sql = format!(
        "{ALL_STRIPES_GUARD} IF $open {{ IF array::len(SELECT VALUE id FROM native_effect WITH INDEX live_effects WHERE resolved=false AND id!=$__effect LIMIT 1)>0 OR array::len(SELECT VALUE id FROM native_operation WITH INDEX operation_history WHERE state='open' LIMIT 1)>0 OR array::len(SELECT VALUE id FROM native_operation_page WITH INDEX live_operation_pages WHERE resolved=false LIMIT 1)>0 {{ THROW 'native admission unresolved effects'; }}; }}; FOR $stripe IN $__stripes {{ UPDATE $stripe SET admission_open=$open RETURN NONE; }}; UPDATE native_installation:current SET admission_open=$open,control_revision+=1 RETURN NONE"
    );
    effect(client, None, &sql, bindings).await
}

pub(super) fn validate_control_records(
    marker: &Object,
    stripes: &[Object],
    gates: &[Object],
) -> Result<(), ModelError> {
    let expected = stripe_ids()
        .into_iter()
        .collect::<std::collections::BTreeSet<_>>();
    let mut seen = std::collections::BTreeSet::new();
    for stripe in stripes {
        let Some(Value::RecordId(id)) = stripe.get("id") else {
            return Err(ModelError::Schema("native stripe identity"));
        };
        if !expected.contains(id)
            || !seen.insert(id.clone())
            || int_field(stripe, "format")? != STRIPE_FORMAT
            || int_field(stripe, "revision")? < 0
            || ["generation", "era", "closed_through", "admission_open"]
                .into_iter()
                .any(|field| stripe.get(field) != marker.get(field))
            || serde_json::to_vec(stripe).map_err(ModelError::codec)?.len() > 1024
        {
            return Err(ModelError::Conflict("native stripe marker correspondence"));
        }
    }
    if seen != expected {
        return Err(ModelError::Conflict("native stripe set incomplete"));
    }
    let [gate] = gates else {
        return Err(ModelError::Conflict("native backup gate cardinality"));
    };
    if int_field(gate, "revision")? < 0 || int_field(gate, "active_count")? < 0 {
        return Err(ModelError::Schema("native backup gate counters"));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn opaque_wire_admission_is_distinct_from_resident_workspace_and_refuses_oversize() {
        let mut variables = Variables::new();
        variables.insert("payload", surrealdb::types::Bytes::from(vec![255; 1 << 20]));
        let sql = "RETURN $payload;";
        assert!(
            immutable_window::plan_bytes(sql, &variables).unwrap() > TRANSFER_BYTES,
            "JSON and resident coexistence work is not a protobuf message size"
        );
        let bound = query_wire_bound(sql, &variables).unwrap();
        assert!(
            bound > (1 << 20) && bound < (1 << 20) + 256,
            "opaque bytes remain native wire bytes: {bound}"
        );
        preflight(sql, &variables).unwrap();
        variables.insert(
            "payload",
            surrealdb::types::Bytes::from(vec![255; TRANSFER_BYTES + 1]),
        );
        assert!(matches!(preflight(sql,&variables),Err(ModelError::Limit {
            owner:"native transaction",limit:"complete control and payload bytes",observed,bound:TRANSFER_BYTES
        }) if observed>TRANSFER_BYTES));
    }
    #[test]
    fn complete_stripe_set_is_fixed_bounded_and_operation_stable() {
        let rows = initial_stripes(ContentHash([255; 32]), i64::MAX, i64::MAX - 1, false).unwrap();
        assert_eq!(rows.len(), 64);
        assert_eq!(
            stripe_ids()
                .into_iter()
                .collect::<std::collections::BTreeSet<_>>()
                .len(),
            64
        );
        for byte in 0..=255 {
            let operation = ContentHash([byte; 32]);
            assert_eq!(stripe_id(operation), stripe_id(operation));
            assert!(stripe_ids().contains(&stripe_id(operation)));
        }
        assert!(!ERA_GUARD.contains("UPDATE native_installation"));
        let mut bindings = Variables::new();
        bindings.insert("rows", rows);
        preflight(ALL_STRIPES_GUARD, &bindings).unwrap();
    }
    #[test]
    fn creation_lookup_preserves_first_equal_row_kind_and_declaration_order_refusal() {
        fn row(id: RecordId, payload: i64) -> Value {
            let mut row = Object::new();
            row.insert("id", id);
            row.insert("payload", payload);
            Value::Object(row)
        }
        fn declaration(id: RecordId, kind: &str) -> Value {
            let mut row = Object::new();
            row.insert("record", id);
            row.insert("kind", kind);
            Value::Object(row)
        }
        let a = RecordId::new("entity", "a");
        let b = RecordId::new("original_blob", "b");
        let unrelated = RecordId::new("entity", "unrelated");
        let mut bindings = Variables::new();
        bindings.insert(
            "rows",
            vec![
                row(unrelated.clone(), 0),
                row(a.clone(), 1),
                row(a.clone(), 1),
                row(unrelated, 2),
            ],
        );
        bindings.insert("original_blobs", vec![row(b.clone(), 3)]);
        bindings.insert(
            "__immutable_creations",
            vec![
                declaration(a.clone(), "immutable"),
                declaration(a.clone(), "immutable"),
                declaration(b.clone(), "original_blob"),
            ],
        );
        let indices = creation_indices(&bindings).unwrap();
        assert_eq!(indices.values.len(), 3);
        assert_eq!(
            indices.get(&a).unwrap().as_object().unwrap().get("index"),
            Some(&Value::Number(surrealdb::types::Number::Int(1)))
        );
        assert_eq!(
            indices.get(&b).unwrap().as_object().unwrap().get("index"),
            Some(&Value::Number(surrealdb::types::Number::Int(0)))
        );
        assert_eq!(
            indices.get(&b).unwrap().as_object().unwrap().get("kind"),
            Some(&Value::String("original_blob".into()))
        );
        bindings.insert("rows", vec![row(a.clone(), 1), row(a.clone(), 2)]);
        assert!(matches!(
            creation_indices(&bindings),
            Err(ModelError::Conflict("native immutable creation collision"))
        ));
        bindings.insert("rows", Vec::<Value>::new());
        bindings.insert(
            "__immutable_creations",
            vec![declaration(a, "immutable"), Value::None],
        );
        assert!(matches!(
            creation_indices(&bindings),
            Err(ModelError::Schema("native immutable creation row missing"))
        ));
        bindings.insert("rows", vec![row(b.clone(), 3)]);
        bindings.insert(
            "__immutable_creations",
            vec![
                declaration(b.clone(), "immutable"),
                declaration(b, "original_blob"),
            ],
        );
        assert!(matches!(
            creation_indices(&bindings),
            Err(ModelError::Conflict("native immutable creation collision"))
        ));
    }

    #[test]
    fn generic_preparation_reservation_refuses_before_growth_and_releases_with_owner() {
        let record = RecordId::new("entity", "generic-preparation");
        let guard = guard_id(&Value::RecordId(record.clone())).unwrap();
        let expected = ExpectedObject {
            record,
            incarnation: 3,
            phase: "active".into(),
            tombstone_revision_if_vacant: None,
        }
        .binding(guard.clone(), None);
        let mut bindings = Variables::new();
        bindings.insert("__object_expectations", vec![expected]);
        bindings.insert("hold_guards", vec![guard]);
        let original = bindings.clone();
        let tiny = lctx_model::domain::resources::ResourceBudget::fixed(1).unwrap();
        assert!(matches!(
            reserve_expectation_preparation(&bindings, &tiny),
            Err(ModelError::Resource { .. })
        ));
        assert_eq!(
            bindings, original,
            "reservation refuses before metadata mutation or candidate growth"
        );
        assert_eq!(tiny.reserved(), 0);
        let budget = lctx_model::domain::resources::ResourceBudget::fixed(65536).unwrap();
        let charge = reserve_expectation_preparation(&bindings, &budget).unwrap();
        classify_expectations(&mut bindings).unwrap();
        assert!(budget.reserved() > 0);
        assert_eq!(
            bindings
                .get("__object_expectations")
                .unwrap()
                .as_array()
                .unwrap()[0]
                .as_object()
                .unwrap()
                .get("deferred_incidence"),
            Some(&Value::Bool(true))
        );
        drop(bindings);
        assert!(
            budget.reserved() > 0,
            "grant outlives its prepared envelope"
        );
        drop(charge);
        assert_eq!(budget.reserved(), 0);
    }
    #[tokio::test]
    async fn supplied_expectations_get_packet_metadata_without_recapturing_guards() {
        let records = (0..7)
            .map(|index| {
                RecordId::new(
                    if index == 1 {
                        "original_blob"
                    } else {
                        "entity"
                    },
                    format!("classification-{index}"),
                )
            })
            .collect::<Vec<_>>();
        let guards = records
            .iter()
            .map(|id| guard_id(&Value::RecordId(id.clone())).unwrap())
            .collect::<Vec<_>>();
        let expected = records
            .iter()
            .zip(&guards)
            .enumerate()
            .map(|(index, (record, guard))| {
                ExpectedObject {
                    record: record.clone(),
                    incarnation: 17,
                    phase: ["active", "vacant", "retired"][index % 3].into(),
                    tombstone_revision_if_vacant: Some(23),
                }
                .binding(
                    guard.clone(),
                    matches!(index, 0 | 6).then_some(&Value::Bool(true)),
                )
            })
            .collect::<Vec<_>>();
        let mut bindings = Variables::new();
        bindings.insert("__object_expectations", expected.clone());
        bindings.insert("guards", vec![guards[3].clone()]);
        bindings.insert("__write_guards", vec![guards[4].clone()]);
        bindings.insert(
            "__mutable_write_guards",
            vec![guards[5].clone(), guards[6].clone()],
        );
        bindings.insert("hold_guards", guards[2..].to_vec());
        let mut chunk = Object::new();
        chunk.insert("id", RecordId::new("original_chunk", "classify"));
        chunk.insert("blob", records[1].clone());
        bindings.insert("rows", vec![Value::Object(chunk)]);
        // An unconnected client proves this supplied-envelope path does no query.
        prepare_expectations(&Surreal::init(), &mut bindings)
            .await
            .unwrap();
        let Some(Value::Array(classified)) = bindings.get("__object_expectations") else {
            panic!("classified");
        };
        for (index, value) in classified.iter().enumerate() {
            let mut observed = value.as_object().unwrap().clone();
            assert_eq!(
                observed.remove("deferred_incidence"),
                Some(Value::Bool(matches!(index, 0..=2)))
            );
            let mut original = expected[index].as_object().unwrap().clone();
            original.remove("deferred_incidence");
            assert_eq!(observed, original, "physical capture never refreshes");
        }
        let first = classified.clone();
        prepare_expectations(&Surreal::init(), &mut bindings)
            .await
            .unwrap();
        assert_eq!(
            bindings.get("__object_expectations"),
            Some(&Value::Array(first)),
            "repeat preparation is exact"
        );
    }
    #[test]
    fn active_expectations_ignore_hold_revision_but_tombstones_bind_revision() {
        let id = RecordId::new("original", "candidate");
        let mut guard = Object::new();
        guard.insert("phase", "active");
        guard.insert("incarnation", 4i64);
        guard.insert("revision", 8i64);
        guard.insert("retired", false);
        guard.insert("tombstone_revision", 0i64);
        let active = ExpectedObject::from_guard(id.clone(), Some(&guard)).unwrap();
        guard.insert("revision", 9i64);
        assert_eq!(
            active,
            ExpectedObject::from_guard(id.clone(), Some(&guard)).unwrap()
        );
        guard.insert("phase", "retired");
        guard.insert("retired", true);
        guard.insert("tombstone_revision", 2i64);
        let retired = ExpectedObject::from_guard(id.clone(), Some(&guard)).unwrap();
        guard.insert("tombstone_revision", 3i64);
        assert_ne!(
            retired,
            ExpectedObject::from_guard(id, Some(&guard)).unwrap()
        );
    }
    #[test]
    fn full_stripe_validation_refuses_missing_duplicate_and_divergent_control_rows() {
        let generation = ContentHash::of(b"stripe-generation");
        let mut marker = Object::new();
        marker.insert("generation", generation.hex());
        marker.insert("era", 3i64);
        marker.insert("closed_through", 2i64);
        marker.insert("admission_open", false);
        let rows = initial_stripes(generation, 3, 2, false)
            .unwrap()
            .into_iter()
            .map(|row| match row {
                Value::Object(row) => row,
                _ => unreachable!(),
            })
            .collect::<Vec<_>>();
        let mut gate = Object::new();
        gate.insert("revision", 0i64);
        gate.insert("active_count", 0i64);
        validate_control_records(&marker, &rows, std::slice::from_ref(&gate)).unwrap();
        assert!(
            validate_control_records(&marker, &rows[..63], std::slice::from_ref(&gate)).is_err()
        );
        let mut duplicate = rows.clone();
        duplicate[63] = duplicate[0].clone();
        assert!(
            validate_control_records(&marker, &duplicate, std::slice::from_ref(&gate)).is_err()
        );
        for field in ["generation", "era", "closed_through", "admission_open"] {
            let mut divergent = rows.clone();
            divergent[17].insert(field, Value::None);
            assert!(
                validate_control_records(&marker, &divergent, std::slice::from_ref(&gate)).is_err()
            );
        }
        gate.insert("active_count", -1i64);
        assert!(validate_control_records(&marker, &rows, &[gate]).is_err());
    }
    #[test]
    fn target_control_declarations_and_fixed_transaction_guards_parse() {
        surrealdb_syn::parse(schema()).unwrap();
        for sql in [
            ERA_GUARD,
            ALL_STRIPES_GUARD,
            DRAINED_PREDICATE,
            EXPECTED_OBJECT_GUARD,
        ] {
            surrealdb_syn::parse(&format!("BEGIN; {sql} COMMIT;")).unwrap();
        }
    }
    #[test]
    fn oversized_complete_transaction_refuses_before_effect() {
        let mut bindings = Variables::new();
        bindings.insert("payload", "x".repeat(TRANSFER_BYTES));
        assert!(preflight("BEGIN;COMMIT;", &bindings).is_err());
    }
}

#[cfg(test)]
mod attempt_cut_controls {
    use super::*;
    #[test]
    fn attempt_stripe_ids_are_complete_exact_and_scope_qualified() {
        let one = RecordId::new("native_attempt", "one");
        let two = RecordId::new("native_attempt", "two");
        let ids = attempt_stripe_ids(&one);
        assert_eq!(ids.len(), 64);
        let unique = ids.iter().collect::<std::collections::BTreeSet<_>>();
        assert_eq!(unique.len(), 64);
        assert!(
            attempt_stripe_ids(&two)
                .iter()
                .all(|id| !unique.contains(id))
        );
        for operation in 0..256u32 {
            let slot = attempt_slot(ContentHash::of(&operation.to_le_bytes()));
            assert!((0..64).contains(&slot));
        }
    }
    #[test]
    fn actual_attempt_owner_plans_parse_and_participate_once_without_shared_touch() {
        let request = NativeRequest {
            issuance: IssuanceEra {
                generation: ContentHash::of(b"cut-generation"),
                era: 1,
            },
            operation: ContentHash::of(b"cut-operation"),
            request: ContentHash::of(b"cut-request"),
        };
        for owner in [
            EffectOwner::OpenAttempt(ContentHash::of(b"cut-attempt")),
            EffectOwner::FrozenAttempt(ContentHash::of(b"cut-attempt")),
            EffectOwner::TerminalAttempt(TerminalAttempt {
                identity: ContentHash::of(b"cut-attempt"),
                epoch: 1,
            }),
        ] {
            let plan = authorization::owner_effect_plan(
                request,
                &owner,
                "RETURN true",
                Variables::new(),
                false,
            )
            .unwrap();
            {
                let sql = &plan.query;
                surrealdb_syn::parse(sql).unwrap();
                assert_eq!(
                    sql.matches("fn::native_attempt_participate($__attempt_context)")
                        .count(),
                    1
                );
                assert!(!sql.contains("fn::native_touch($__attempt)"));
                assert!(!sql.contains("fn::native_touch($__owner_id)"));
            }
            assert_eq!(
                plan.bindings.get("__attempt_slot"),
                Some(&surrealdb::types::SurrealValue::into_value(attempt_slot(
                    request.operation
                )))
            );
        }
        surrealdb_syn::parse(include_str!("schema.surql")).unwrap();
    }
    #[test]
    fn attempt_participant_lock_uses_record_parameter_not_field_idiom() {
        use surrealdb_sql::{Expr, TopLevelExpr, statements::DefineStatement};
        // Parsing alone accepts the former query. The pinned core's FOR UPDATE
        // source classifier rejects its Idiom before evaluating the field value.
        let old = surrealdb_syn::parse("SELECT * FROM ONLY $context.effect FOR UPDATE;").unwrap();
        let [TopLevelExpr::Expr(Expr::Select(old))] = old.expressions.as_slice() else {
            panic!("one revealing locked SELECT");
        };
        assert!(old.for_update);
        assert!(matches!(old.what.as_slice(), [Expr::Idiom(_)]));
        let schema = surrealdb_syn::parse(include_str!("schema.surql")).unwrap();
        let participant = schema
            .expressions
            .iter()
            .find_map(|expression| {
                if let TopLevelExpr::Expr(Expr::Define(statement)) = expression
                    && let DefineStatement::Function(function) = statement.as_ref()
                    && function.name.as_str() == "native_attempt_participate"
                {
                    Some(function)
                } else {
                    None
                }
            })
            .expect("actual participant definition");
        let binding = participant
            .block
            .0
            .iter()
            .find_map(|expression| match expression {
                Expr::Let(binding) if binding.name.as_str() == "effect_id" => Some(binding),
                _ => None,
            })
            .expect("exact witness binding");
        assert!(matches!(&binding.what, Expr::Idiom(_)));
        let locked = participant
            .block
            .0
            .iter()
            .find_map(|expression| match expression {
                Expr::Let(binding) if binding.name.as_str() == "effect" => match &binding.what {
                    Expr::Select(locked) => Some(locked),
                    _ => None,
                },
                _ => None,
            })
            .expect("actual participant witness read");
        assert!(locked.only && locked.for_update);
        assert!(
            matches!(locked.what.as_slice(), [Expr::Param(parameter)] if parameter.as_str() == "effect_id")
        );
    }
    #[tokio::test(flavor = "multi_thread")]
    #[ignore = "exclusive task-owned RocksDB campaign only"]
    async fn native_deferred_first_match_preserves_order_and_none_for_compound_ids() {
        use surrealdb::types::{Number, RecordIdKey};
        let cfg = crate::RuntimeConfig::read(std::path::Path::new(
            &std::env::var_os("LCTX_COMPILER_RUNTIME_CONFIG").expect("owned RocksDB campaign"),
        ))
        .unwrap();
        let client = crate::installation::check_installation(&cfg).await.unwrap();
        let compound = |number| {
            RecordId::new(
                "entity",
                RecordIdKey::Array(vec![Value::Number(number)].into()),
            )
        };
        let scalar = RecordId::new("entity", "scalar");
        let entry = |record: RecordId, label: &str| {
            let mut row = Object::new();
            row.insert("record", record);
            row.insert("label", label);
            Value::Object(row)
        };
        let entries = vec![
            entry(compound(Number::Int(1)), "first"),
            entry(compound(Number::Float(1.0)), "later"),
            entry(scalar.clone(), "scalar"),
        ];
        for (target, label) in [
            (compound(Number::Int(1)), Some("first")),
            (scalar, Some("scalar")),
            (RecordId::new("entity", "absent"), None),
        ] {
            let mut response = client.query("RETURN {found:array::find($entries, |$entry| $entry.record=$target), original:($entries.filter(|$entry| $entry.record=$target)[0] ?? NONE)};").bind(("entries", entries.clone())).bind(("target", target)).await.unwrap().check().unwrap();
            let result: Value = response.take(0).unwrap();
            let result = result
                .as_object()
                .expect("native first-match comparison object");
            assert_eq!(
                result.get("found"),
                result.get("original"),
                "native first-match and absence semantics unchanged"
            );
            let found = result
                .get("found")
                .and_then(Value::as_object)
                .and_then(|row| row.get("label"));
            assert_eq!(
                found,
                label.map(|label| Value::String(label.into())).as_ref()
            );
        }
        client.invalidate().await.unwrap();
    }
    // Uses direct protocol packets to control native transaction commit order;
    // never selected on the shared fixture service. The enclosing owned campaign
    // owns leftover logical participants and failed-commit sessions through discard.
    #[tokio::test(flavor = "multi_thread")]
    #[ignore = "exclusive task-owned RocksDB campaign only"]
    async fn native_atomic_disjoint_applies_overlap_and_complete_cut_fences_delayed_apply() {
        atomic_applies_and_complete_cut("close").await;
    }
    #[tokio::test(flavor = "multi_thread")]
    #[ignore = "exclusive task-owned RocksDB campaign only"]
    async fn native_atomic_disjoint_applies_overlap_and_complete_freeze_fences_delayed_apply() {
        atomic_applies_and_complete_cut("freeze").await;
    }
    async fn atomic_applies_and_complete_cut(action: &str) {
        let cfg = crate::RuntimeConfig::read(std::path::Path::new(
            &std::env::var_os("LCTX_COMPILER_RUNTIME_CONFIG").expect("owned RocksDB campaign"),
        ))
        .unwrap();
        let client = crate::installation::check_installation(&cfg).await.unwrap();
        let era = IssuanceEra::capture(&client).await.unwrap();
        let attempt = fresh_identity("attempt-stripe-overlap-control").unwrap();
        let budget = lctx_model::domain::resources::ResourceBudget::fixed(
            lctx_model::domain::resources::DEFAULT_MEMORY_BYTES,
        )
        .unwrap();
        let stop = RetryCancellation::default();
        let cancel = stop.clone();
        let signals = cfg
            .signal_owner("attempt stripe allocation control", move || cancel.cancel())
            .unwrap();
        let scope = RecoveryScope::new(
            client.clone(),
            budget,
            stop,
            signals.pending_recipient().unwrap(),
            RecoveryTarget {
                generation: cfg.service_generation,
                namespace: cfg.namespace.as_str().into(),
                database: cfg.database.as_str().into(),
                endpoint: Some(cfg.endpoint.clone()),
            },
        )
        .unwrap();
        with_recovery_scope(
            scope.clone(),
            authorization::begin_attempt_in(&client, attempt, era),
        )
        .await
        .unwrap();
        assert_eq!(scope.pending_count().unwrap(), 0);
        let first = fresh_identity("stripe-first-operation").unwrap();
        let mut second = fresh_identity("stripe-second-operation").unwrap();
        while attempt_slot(second) == attempt_slot(first) {
            second = fresh_identity("stripe-second-operation").unwrap();
        }
        let make = |operation| {
            let request = NativeRequest {
                issuance: era,
                operation,
                request: ContentHash::of(&operation.0),
            };
            authorization::owner_effect_plan(
                request,
                &EffectOwner::OpenAttempt(attempt),
                "RETURN true",
                Variables::new(),
                false,
            )
            .unwrap()
        };
        let a = make(first);
        let b = make(second);
        let body = |sql: &str| {
            sql.strip_prefix("BEGIN;")
                .unwrap()
                .strip_suffix("COMMIT;")
                .unwrap()
                .to_owned()
        };
        let left = client.as_ref().clone().begin().await.unwrap();
        let right = client.as_ref().clone().begin().await.unwrap();
        checked_transaction(
            left.query(body(&a.query))
                .bind(a.bindings.clone())
                .await
                .unwrap(),
        )
        .unwrap();
        checked_transaction(
            right
                .query(body(&b.query))
                .bind(b.bindings.clone())
                .await
                .unwrap(),
        )
        .unwrap();
        left.commit().await.unwrap().invalidate().await.unwrap();
        right.commit().await.unwrap().invalidate().await.unwrap();
        // Same-slot operations deliberately share the installation/attempt conflict
        // address. This is combined production-packet evidence, not attribution
        // to one of those two stripe families in isolation.
        for first_commit in [true, false] {
            let one = fresh_identity("same-slot-first").unwrap();
            let mut two = fresh_identity("same-slot-second").unwrap();
            while two == one || attempt_slot(two) != attempt_slot(one) {
                two = fresh_identity("same-slot-second").unwrap();
            }
            let one_plan = make(one);
            let two_plan = make(two);
            let left = client.as_ref().clone().begin().await.unwrap();
            let right = client.as_ref().clone().begin().await.unwrap();
            checked_transaction(
                left.query(body(&one_plan.query))
                    .bind(one_plan.bindings.clone())
                    .await
                    .unwrap(),
            )
            .unwrap();
            checked_transaction(
                right
                    .query(body(&two_plan.query))
                    .bind(two_plan.bindings.clone())
                    .await
                    .unwrap(),
            )
            .unwrap();
            let conflict = if first_commit {
                left.commit().await.unwrap().invalidate().await.unwrap();
                right
                    .commit()
                    .await
                    .expect_err("same-slot second commit conflicts")
            } else {
                right.commit().await.unwrap().invalidate().await.unwrap();
                left.commit()
                    .await
                    .expect_err("same-slot second commit conflicts")
            };
            assert!(
                retryable_conflict(&conflict),
                "expected same-slot native conflict: {conflict}"
            );
            let retry = if first_commit { &two_plan } else { &one_plan };
            checked_transaction(
                client
                    .query(retry.query.clone())
                    .bind(retry.bindings.clone())
                    .await
                    .unwrap(),
            )
            .unwrap();
            // Replay the original request again and compare its complete receipt.
            // No second epoch, participant or payload result may be manufactured.
            let operation = if first_commit { two } else { one };
            let request = NativeRequest {
                issuance: era,
                operation,
                request: ContentHash::of(&operation.0),
            };
            let effect = RecordId::new("native_effect", operation.hex());
            let mut observed = client
                .query("SELECT * FROM ONLY $effect;")
                .bind(("effect", effect.clone()))
                .await
                .unwrap()
                .check()
                .unwrap();
            let receipt: Option<Object> = observed.take(0).unwrap();
            assert_eq!(
                receipt.as_ref().and_then(|row| row.get("committed")),
                Some(&Value::Bool(true))
            );
            checked_transaction(
                client
                    .query(retry.query.clone())
                    .bind(retry.bindings.clone())
                    .await
                    .unwrap(),
            )
            .unwrap();
            let mut observed = client
                .query("SELECT * FROM ONLY $effect;")
                .bind(("effect", effect))
                .await
                .unwrap()
                .check()
                .unwrap();
            let replayed: Option<Object> = observed.take(0).unwrap();
            assert_eq!(
                with_recovery_scope(
                    scope.clone(),
                    authorization::reconcile_request(&client, request)
                )
                .await
                .unwrap(),
                EffectDisposition::Committed
            );
            assert_eq!(receipt, replayed, "same original outcome exactly once");
        }
        // Exact replay never executes payload again; an owner-context collision
        // refuses even when operation, request digest and original era agree.
        checked_transaction(
            client
                .query(a.query.clone())
                .bind(a.bindings.clone())
                .await
                .unwrap(),
        )
        .unwrap();
        let mut collision = a.bindings.clone();
        collision.insert("__owner_kind", "frozen_attempt");
        assert!(
            checked_transaction(client.query(a.query.clone()).bind(collision).await.unwrap())
                .is_err()
        );
        // Both bodies read an absent effect before either commits. The losing
        // commit must conflict, rather than inventing a second terminal result.
        for fence_first in [false, true] {
            let operation = fresh_identity("overlapping-absent-fence").unwrap();
            let request = NativeRequest {
                issuance: era,
                operation,
                request: ContentHash::of(&operation.0),
            };
            let apply = make(operation);
            let (fence_sql, fence_bindings) =
                authorization::reconciliation_plan(request, era).unwrap();
            let applying = client.as_ref().clone().begin().await.unwrap();
            let fencing = client.as_ref().clone().begin().await.unwrap();
            checked_transaction(
                applying
                    .query(body(&apply.query))
                    .bind(apply.bindings)
                    .await
                    .unwrap(),
            )
            .unwrap();
            checked_transaction(
                fencing
                    .query(body(&fence_sql))
                    .bind(fence_bindings)
                    .await
                    .unwrap(),
            )
            .unwrap();
            let conflict = if fence_first {
                fencing.commit().await.unwrap().invalidate().await.unwrap();
                applying
                    .commit()
                    .await
                    .expect_err("fence first excludes the original absent-snapshot apply")
            } else {
                applying.commit().await.unwrap().invalidate().await.unwrap();
                fencing
                    .commit()
                    .await
                    .expect_err("apply first excludes the original absent-snapshot fence")
            };
            assert!(
                retryable_conflict(&conflict),
                "expected exact absent-fence conflict: {conflict}"
            );
            assert_eq!(
                with_recovery_scope(
                    scope.clone(),
                    authorization::reconcile_request(&client, request)
                )
                .await
                .unwrap(),
                if fence_first {
                    EffectDisposition::FencedUncommitted
                } else {
                    EffectDisposition::Committed
                }
            );
            assert_eq!(scope.pending_count().unwrap(), 0);
        }
        let absent_operation = fresh_identity("atomic-absent-fence").unwrap();
        let absent = make(absent_operation);
        let absent_request = NativeRequest {
            issuance: era,
            operation: absent_operation,
            request: ContentHash::of(&absent_operation.0),
        };
        assert_eq!(
            with_recovery_scope(
                scope.clone(),
                authorization::reconcile_request(&client, absent_request)
            )
            .await
            .unwrap(),
            EffectDisposition::FencedUncommitted
        );
        assert!(
            checked_transaction(
                client
                    .query(absent.query)
                    .bind(absent.bindings)
                    .await
                    .unwrap()
            )
            .is_err(),
            "delayed original apply cannot pass exact absent-request fence"
        );
        let cut = client.as_ref().clone().begin().await.unwrap();
        cut.query("fn::native_attempt_cut($attempt,$generation,$era,1,$action); UPDATE $attempt SET state=$next_state,revision+=1 RETURN NONE;").bind(("attempt",RecordId::new("native_attempt",attempt.hex()))).bind(("generation",era.generation.hex())).bind(("era",era.era)).bind(("action",action.to_owned())).bind(("next_state",if action=="freeze" {"frozen"} else {"closing"})).await.unwrap().check().unwrap();
        let late = make(fresh_identity("stripe-late-intent").unwrap());
        checked_transaction(
            client
                .query(late.query.clone())
                .bind(late.bindings.clone())
                .await
                .unwrap(),
        )
        .unwrap();
        let conflict = cut
            .commit()
            .await
            .expect_err("older full cut must conflict with later committed atomic apply");
        assert!(
            retryable_conflict(&conflict),
            "expected conclusive native write conflict, not an unknown transport failure: {conflict}"
        );
        let delayed = make(fresh_identity("stripe-delayed-apply").unwrap());
        let pending = client.as_ref().clone().begin().await.unwrap();
        checked_transaction(
            pending
                .query(body(&delayed.query))
                .bind(delayed.bindings.clone())
                .await
                .unwrap(),
        )
        .unwrap();
        let cut = client.as_ref().clone().begin().await.unwrap();
        cut.query("fn::native_attempt_cut($attempt,$generation,$era,1,$action); UPDATE $attempt SET state=$next_state,revision+=1 RETURN NONE;").bind(("attempt",RecordId::new("native_attempt",attempt.hex()))).bind(("generation",era.generation.hex())).bind(("era",era.era)).bind(("action",action.to_owned())).bind(("next_state",if action=="freeze" {"frozen"} else {"closing"})).await.unwrap().check().unwrap();
        cut.commit().await.unwrap().invalidate().await.unwrap();
        let conflict = pending
            .commit()
            .await
            .expect_err("completed cut excludes an older uncommitted apply body");
        assert!(
            retryable_conflict(&conflict),
            "expected native commit conflict: {conflict}"
        );
        assert!(
            checked_transaction(
                client
                    .query(delayed.query)
                    .bind(delayed.bindings)
                    .await
                    .unwrap()
            )
            .is_err(),
            "prepared delayed atomic apply is fenced by the completed cut"
        );
        let after = make(fresh_identity("stripe-after-cut-intent").unwrap());
        assert!(
            checked_transaction(
                client
                    .query(after.query)
                    .bind(after.bindings)
                    .await
                    .unwrap()
            )
            .is_err(),
            "new atomic apply refuses closed eligibility"
        );
        client.invalidate().await.unwrap();
        signals
            .complete(&lctx_model::domain::completion::Completion::default())
            .await
            .unwrap();
    }
}
