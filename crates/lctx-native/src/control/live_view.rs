//! Fixed per-view witness effects while unrelated attempt producers remain open.
//! This authority cannot create aggregate completion, admission or canonical content.
use super::*;
use lctx_model::domain::resources::ResourceBudget;

pub(crate) struct LiveViewOwner {
    pub(super) attempt: ContentHash,
    generation: ContentHash,
    epoch: i64,
    validity_epoch: i64,
    revision: i64,
}
impl LiveViewOwner {
    pub(crate) fn revision(&self) -> i64 {
        self.revision
    }
    pub(crate) fn epoch(&self) -> i64 {
        self.epoch
    }
    pub(crate) fn attempt(&self) -> ContentHash {
        self.attempt
    }
    pub(crate) fn generation(&self) -> ContentHash {
        self.generation
    }
    pub(crate) fn validity_epoch(&self) -> i64 {
        self.validity_epoch
    }
}
pub(super) const GUARD: &str = "IF $__owner.state!='open' OR $__owner.admitted OR $__owner.generation!=$live_generation OR $__owner.epoch!=$live_epoch OR $__installation.generation!=$live_generation OR $__installation.validity_epoch!=$live_validity_epoch { THROW 'live installed view owner changed'; };";
pub(super) fn bind(owner: &LiveViewOwner, bindings: &mut Variables) {
    bindings.insert("live_generation", owner.generation.hex());
    bindings.insert("live_epoch", owner.epoch);
    bindings.insert("live_validity_epoch", owner.validity_epoch);
}
pub(crate) async fn capture(
    reader: &crate::NativeReader<()>,
    attempt: ContentHash,
    budget: &ResourceBudget,
) -> Result<LiveViewOwner, ModelError> {
    let _charge = budget.reserve("live installed view owner inspection", 131072)?;
    let mut bindings = Variables::new();
    bindings.insert("attempt", RecordId::new("native_attempt", attempt.hex()));
    let owners: Vec<Object> = reader
        .query_native("SELECT * FROM $attempt", bindings)
        .await?;
    let installations: Vec<Object> = reader
        .query_native(
            "SELECT generation,validity_epoch FROM native_installation:current",
            Variables::new(),
        )
        .await?;
    let [owner] = owners.as_slice() else {
        return Err(ModelError::Conflict("live view attempt cardinality"));
    };
    let [installation] = installations.as_slice() else {
        return Err(ModelError::Conflict("live view installation cardinality"));
    };
    let result = LiveViewOwner {
        attempt,
        generation: authorization::hash_field(installation, "generation")?,
        epoch: authorization::int_field(owner, "epoch")?,
        validity_epoch: authorization::int_field(installation, "validity_epoch")?,
        revision: authorization::int_field(owner, "revision")?,
    };
    if owner.get("state") != Some(&Value::String("open".into()))
        || owner.get("admitted") != Some(&Value::Bool(false))
        || result.epoch <= 0
        || result.validity_epoch <= 0
        || authorization::hash_field(owner, "generation")? != result.generation
    {
        return Err(ModelError::Conflict(
            "live view requires actual open current attempt",
        ));
    }
    Ok(result)
}
pub(crate) fn witness_writer<'a>(
    client: &'a Surreal<Client>,
    owner: &LiveViewOwner,
    budget: &'a ResourceBudget,
) -> Result<super::completed::WitnessWriter<'a>, ModelError> {
    let mut scope = Variables::new();
    bind(owner, &mut scope);
    super::completed::WitnessWriter::new(
        client,
        budget,
        EffectOwner::OpenAttempt(owner.attempt),
        GUARD,
        scope,
    )
}
pub(crate) async fn hold_contributors(
    client: &Surreal<Client>,
    owner: &LiveViewOwner,
    root: &RecordId,
    expected: &[std::sync::Arc<ExpectedRecord>],
    budget: &ResourceBudget,
) -> Result<(), ModelError> {
    let mut scope = Variables::new();
    bind(owner, &mut scope);
    super::completed::hold_boundary_contributors(
        client,
        EffectOwner::OpenAttempt(owner.attempt),
        GUARD,
        scope,
        root,
        expected,
        budget,
    )
    .await
}
/// Transfer protected immutable contributor authority by an exact receiving CAS.
/// A source pin precheck cannot substitute for the guard/bytes comparison here.
pub(crate) async fn transfer_expected(
    pin: &ReaderPin,
    expected: &ExpectedRecord,
    budget: &ResourceBudget,
) -> Result<(), ModelError> {
    pin.ensure_live()?;
    let owner = RecordId::new("native_pin", pin.identity.hex());
    let (_, holds, mut variables) =
        hold_plan(None, vec![(owner.clone(), expected.record().clone())])?
            .ok_or(ModelError::Schema("selected reader exact transfer plan"))?;
    let _charge = budget.reserve(
        "selected reader exact transfer",
        crate::loader::native_bytes(expected.row())
            .saturating_mul(32)
            .saturating_add(131072),
    )?;
    variables.insert("selected_record", expected.record().clone());
    variables.insert(
        "selected_guard",
        guard_id(&Value::RecordId(expected.record().clone()))?,
    );
    variables.insert("selected_incarnation", expected.incarnation());
    variables.insert("selected_bytes", expected.row().clone());
    let sql = format!(
        "LET $guard=SELECT * FROM ONLY $selected_guard FOR UPDATE; LET $actual=SELECT * FROM ONLY $selected_record FOR UPDATE; IF $guard.phase!='active' OR $guard.incarnation!=$selected_incarnation OR $actual!=$selected_bytes {{ THROW 'selected reader exact transfer changed'; }}; {holds}"
    );
    effect_for_owner(&pin.client, EffectOwner::Pin(owner), &sql, variables).await?;
    pin.ensure_live()
}

/// Independent receiving CAS pages over an already deduplicated opaque inventory.
/// Each page compares every exact source row before installing any of its holds.
pub(crate) async fn transfer_expected_many(
    pin: &ReaderPin,
    records: &[std::sync::Arc<ExpectedRecord>],
    budget: &ResourceBudget,
) -> Result<(), ModelError> {
    pin.ensure_live()?;
    if records
        .windows(2)
        .any(|pair| pair[0].record() >= pair[1].record())
        || records.iter().any(|record| record.incarnation() <= 0)
    {
        return Err(ModelError::Conflict(
            "receiving exact contributor order/incarnation",
        ));
    }
    if records.is_empty() {
        return Ok(());
    }
    let owner = RecordId::new("native_pin", pin.identity.hex());
    let authority = EffectOwner::Pin(owner.clone());
    let _scope = budget.reserve("receiving CAS fixed owner context", 8192)?;
    let issuance = authorization::owner_issuance(&pin.client, &authority).await?;
    let mut offset = 0;
    while offset < records.len() {
        let mut count = (records.len() - offset).min(crate::loader::NATIVE_WINDOW_ROWS);
        loop {
            let page = &records[offset..offset + count];
            let bytes = page
                .iter()
                .map(|expected| crate::loader::native_bytes(expected.row()).saturating_add(2048))
                .fold(0usize, usize::saturating_add)
                .saturating_mul(32)
                .saturating_add(131072);
            let _workspace = match budget.reserve("receiving exact CAS construction", bytes) {
                Ok(charge) => charge,
                Err(ModelError::Resource { .. }) if count > 1 => {
                    count = (count / 2).max(1);
                    continue;
                }
                Err(error) => return Err(error),
            };
            pin.ensure_live()?;
            let mut selected = Vec::with_capacity(count);
            let mut edges = Vec::with_capacity(count);
            for expected in page {
                let mut row = Object::new();
                row.insert("record", expected.record().clone());
                row.insert(
                    "guard",
                    guard_id(&Value::RecordId(expected.record().clone()))?,
                );
                row.insert("incarnation", expected.incarnation());
                row.insert("bytes", expected.row().clone());
                selected.push(Value::Object(row));
                edges.push((owner.clone(), expected.record().clone()));
            }
            let (_, holds, mut variables) =
                hold_plan(None, edges)?.ok_or(ModelError::Schema("receiving checked hold page"))?;
            variables.insert("selected", selected);
            let sql = format!(
                "FOR $expected IN $selected {{ LET $record=$expected.record; LET $guard_id=$expected.guard; LET $guard=SELECT * FROM ONLY $guard_id FOR UPDATE; LET $actual=SELECT * FROM ONLY $record FOR UPDATE; IF $guard.phase!='active' OR $guard.incarnation!=$expected.incarnation OR $actual!=$expected.bytes {{ THROW 'selected reader exact transfer changed'; }}; }}; {holds}"
            );
            let prepared = match authorization::prepare_owner_effect(
                &pin.client,
                &authority,
                issuance,
                &sql,
                variables,
            )
            .await
            {
                Ok(prepared) => prepared,
                Err(ModelError::Limit {
                    owner: "native transaction",
                    limit: "complete control and payload bytes",
                    ..
                }) if count > 1 => {
                    count = (count / 2).max(1);
                    continue;
                }
                Err(error) => return Err(error),
            };
            if prepared.bytes()? > _workspace.size() {
                return Err(ModelError::Schema("receiving exact CAS construction bound"));
            }
            authorization::execute_prepared_owner_effect(&pin.client, prepared).await?;
            pin.ensure_live()?;
            offset += count;
            break;
        }
    }
    Ok(())
}

pub(crate) async fn finish_boundary(
    client: &Surreal<Client>,
    owner: &LiveViewOwner,
    data: &crate::completed_state::FrozenCompletion,
    expected: &ExpectedRecord,
    revision: i64,
    budget: &ResourceBudget,
) -> Result<(), ModelError> {
    if expected.row() != &data.value(false)? {
        return Err(ModelError::Conflict("live boundary initial envelope"));
    }
    let _charge = budget.reserve(
        "live boundary checked finality",
        crate::loader::native_bytes(expected.row())
            .saturating_mul(32)
            .saturating_add(131072),
    )?;
    let mut variables = Variables::new();
    bind(owner, &mut variables);
    variables.insert("boundary", expected.record().clone());
    variables.insert(
        "boundary_guard",
        guard_id(&Value::RecordId(expected.record().clone()))?,
    );
    variables.insert("boundary_incarnation", expected.incarnation());
    variables.insert("boundary_bytes", expected.row().clone());
    variables.insert("boundary_revision", revision);
    let sql = format!(
        "{GUARD} fn::native_touch($boundary); LET $revision=SELECT VALUE revision FROM ONLY type::record('native_predicate',[$boundary]); LET $guard=SELECT * FROM ONLY $boundary_guard FOR UPDATE; LET $actual=SELECT * FROM ONLY $boundary FOR UPDATE; IF $revision!=$boundary_revision+1 OR $guard.phase!='active' OR $guard.incarnation!=$boundary_incarnation OR $actual!=$boundary_bytes {{ THROW 'live compound boundary EOF audit raced'; }}; UPDATE $boundary SET completed=true RETURN NONE;"
    );
    effect_for_owner(
        client,
        EffectOwner::OpenAttempt(owner.attempt),
        &sql,
        variables,
    )
    .await
}

/// One receiving effect compares the complete exact contributor inventory before
/// adding any receiver edge. Prevalidation never commits a receiver prefix.
pub(crate) async fn attach_contributors(
    client: &Surreal<Client>,
    owner: &LiveViewOwner,
    expected: &[ExpectedRecord],
    budget: &ResourceBudget,
) -> Result<(), ModelError> {
    let bytes = expected
        .iter()
        .map(|record| {
            crate::loader::native_bytes(record.row())
                .saturating_mul(32)
                .saturating_add(8192)
        })
        .fold(131072usize, usize::saturating_add);
    let _charge = budget.reserve("atomic receiving contributor inventory", bytes)?;
    if expected.iter().any(|record| {
        record.record().table.as_str() != "compiler_contribution" || record.incarnation() <= 0
    }) {
        return Err(ModelError::Conflict(
            "atomic receiving contributor family/incarnation",
        ));
    }
    let receiver = RecordId::new("native_attempt", owner.attempt().hex());
    let edges = expected
        .iter()
        .map(|record| (receiver.clone(), record.record().clone()))
        .collect();
    let Some((_, holds, mut variables)) = hold_plan(Some(owner.attempt()), edges)? else {
        return Ok(());
    };
    bind(owner, &mut variables);
    let mut comparisons = Vec::with_capacity(expected.len());
    for expected in expected {
        let mut row = Object::new();
        row.insert("record", expected.record().clone());
        row.insert(
            "guard",
            guard_id(&Value::RecordId(expected.record().clone()))?,
        );
        row.insert("incarnation", expected.incarnation());
        row.insert("bytes", expected.row().clone());
        comparisons.push(Value::Object(row));
    }
    variables.insert("attached", comparisons);
    let sql = format!(
        "{GUARD} FOR $expected IN $attached {{ LET $attached_record=$expected.record; LET $attached_guard=$expected.guard; fn::native_touch($attached_record); LET $guard=SELECT * FROM ONLY $attached_guard FOR UPDATE; LET $actual=SELECT * FROM ONLY $attached_record FOR UPDATE; IF $guard.phase!='active' OR $guard.incarnation!=$expected.incarnation OR $actual!=$expected.bytes OR $actual.completed!=true {{ THROW 'atomic receiving contributor changed'; }}; }}; {holds}"
    );
    effect_for_owner(
        client,
        EffectOwner::OpenAttempt(owner.attempt()),
        &sql,
        variables,
    )
    .await
}
