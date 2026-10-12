//! Independent exact selected mapping and dependency inspection for native installation.
//! This prepares a writer's audit; it does not grant semantic admission or completion.
use super::ContributorRealization;
use crate::{NativeReader, completed_rows::validate_state_row, control::ExpectedRecord};
use lctx_model::domain::{
    self as d, ContentHash, ModelError,
    charged::StateCharge,
    completed::{CompletedContribution, CompletedView},
    recovery_closure::{RecoveryClosure, RecoveryTraversal},
    resources::ResourceBudget,
};
use std::collections::BTreeMap;
use surrealdb::types::{RecordId, Value, Variables};
#[cfg(test)]
tokio::task_local! { static LATE_MEMBERSHIP_FAILURE:bool; }
#[cfg(test)]
#[derive(Clone)]
struct BlobObservationBarrier {
    observed: std::sync::Arc<tokio::sync::Barrier>,
    resume: std::sync::Arc<tokio::sync::Barrier>,
}
#[cfg(test)]
tokio::task_local! {static BLOB_OBSERVATION_BARRIER:BlobObservationBarrier;}

/// Operation-local independent observations beneath a real native pin. This is
/// neither portable proof nor semantic admission; each root still earns its own
/// descriptor, aliases, mappings, inventories and final checked native effect.
pub struct InstallationAudit {
    client: std::sync::Arc<surrealdb::Surreal<surrealdb::engine::remote::grpc::Client>>,
    pin: std::sync::Arc<crate::control::ReaderPin>,
    contributors: BTreeMap<RecordId, (i64, Value)>,
    blobs: BTreeMap<RecordId, i64>,
    budget: ResourceBudget,
    charge: StateCharge,
    blob_observations: u64,
    contribution_observations: u64,
    closed: bool,
}
impl InstallationAudit {
    pub async fn new(
        client: std::sync::Arc<surrealdb::Surreal<surrealdb::engine::remote::grpc::Client>>,
        budget: &ResourceBudget,
    ) -> Result<Self, ModelError> {
        Ok(Self {
            pin: crate::control::ReaderPin::acquire(client.clone(), &[]).await?,
            client,
            contributors: BTreeMap::new(),
            blobs: BTreeMap::new(),
            budget: budget.clone(),
            charge: StateCharge::new(budget, "installation audit retained observations"),
            blob_observations: 0,
            contribution_observations: 0,
            closed: false,
        })
    }
    fn check(&self, reader: &NativeReader<()>, budget: &ResourceBudget) -> Result<(), ModelError> {
        self.pin.ensure_live()?;
        if !std::ptr::eq(self.client.as_ref(), reader.client()) {
            return Err(ModelError::Conflict(
                "installation audit foreign native session",
            ));
        }
        if !self.budget.shares_pool(budget) {
            return Err(ModelError::Conflict(
                "installation audit foreign resource owner",
            ));
        }
        Ok(())
    }
    pub async fn close(&mut self) -> Result<(), ModelError> {
        self.pin.release().await?;
        self.closed = true;
        Ok(())
    }
    pub fn retain_unknown(&self) {
        self.pin.retain_unknown();
    }
    /// Structural observation counts; these are not timing or throughput claims.
    pub fn observations(&self) -> (u64, u64) {
        (self.contribution_observations, self.blob_observations)
    }
}
impl Drop for InstallationAudit {
    fn drop(&mut self) {
        if !self.closed {
            self.pin.retain_unknown();
        }
    }
}

/// Exact physical observations remain retained through the final checked installation
/// effect, which must compare these incarnations while touching each contributor.
pub(crate) struct InspectedClosure {
    pub(crate) logical: RecoveryClosure,
    pub(crate) contributors: BTreeMap<ContentHash, ExpectedRecord>,
    pub(crate) mappings: Vec<ContributorRealization>,
    _charge: StateCharge,
}
pub(crate) fn descriptor<T: serde::de::DeserializeOwned>(row: &Value) -> Result<T, ModelError> {
    let Some(Value::Bytes(bytes)) = row.as_object().and_then(|row| row.get("descriptor")) else {
        return Err(ModelError::Schema("installed audit descriptor"));
    };
    serde_json::from_slice(bytes).map_err(ModelError::codec)
}
/// Independently read evidence already retained by this exact native owner. The
/// immutable seal revision is not today's incidence predicate: incoming holds can
/// advance that predicate without changing one member of the completed namespace.
pub(crate) struct OriginalHeldCompletion {
    pub completion: super::InstalledViewCompletion,
    pub expected: crate::control::ExpectedRecord,
    _charge: StateCharge,
}
pub(crate) fn same_completion_metadata(
    expected: &super::InstalledViewCompletion,
    actual: &super::InstalledViewCompletion,
) -> bool {
    let mut metadata = actual.clone();
    metadata.frozen_revision = expected.frozen_revision;
    metadata == *expected
}
pub(crate) async fn original_held_completion(
    reader: &NativeReader<()>,
    owner: &RecordId,
    expected: &super::InstalledViewCompletion,
    current_revision: i64,
    budget: &ResourceBudget,
) -> Result<OriginalHeldCompletion, ModelError> {
    let mut after = Value::None;
    let mut selected = None;
    let mut charge = StateCharge::new(budget, "original held installed completion");
    loop {
        let mut variables = Variables::new();
        variables.insert("seal_owner", owner.clone());
        variables.insert("seal_after", after.clone());
        let rows:Vec<Value>=reader.query_native("SELECT object AS address,object.* AS stored FROM native_hold WITH INDEX owner_holds WHERE owner=$seal_owner AND record::table(object)='compiler_view_completion' AND ($seal_after=NONE OR object>$seal_after) ORDER BY object LIMIT 1",variables).await?;
        let [row] = rows.as_slice() else {
            if !rows.is_empty() {
                return Err(ModelError::Conflict("held completion bounded owner page"));
            }
            break;
        };
        let _page = budget.reserve(
            "held completion owner page decode",
            crate::loader::native_bytes(row)
                .saturating_mul(32)
                .saturating_add(65536),
        )?;
        let object = row
            .as_object()
            .ok_or(ModelError::Schema("held completion owner page"))?;
        let Some(Value::RecordId(address)) = object.get("address") else {
            return Err(ModelError::Schema("held completion exact owner address"));
        };
        after = Value::RecordId(address.clone());
        let actual = object
            .get("stored")
            .ok_or(ModelError::Schema("held completion owner target absent"))?;
        let completion = super::InstalledViewCompletion::from_row(actual)?;
        if completion.record()? != *address {
            return Err(ModelError::Conflict(
                "held completion owner target identity",
            ));
        }
        if same_completion_metadata(expected, &completion) {
            if completion.frozen_revision > current_revision {
                return Err(ModelError::Conflict(
                    "held original seal exceeds current incidence revision",
                ));
            }
            if selected.is_some() {
                return Err(ModelError::Conflict(
                    "multiple exact held installed completion seals",
                ));
            }
            charge.grow(
                crate::loader::native_bytes(actual)
                    .saturating_mul(32)
                    .saturating_add(65536),
            )?;
            selected = Some((completion, actual.clone()));
        }
    }
    let (completion, row) = selected.ok_or(ModelError::Conflict(
        "sealed namespace lacks exact owner-held completion",
    ))?;
    let expected = crate::control::inspect_expected_read(reader, completion.record()?, budget)
        .await?
        .ok_or(ModelError::Conflict("owner-held completion disappeared"))?;
    if expected.row() != &row {
        return Err(ModelError::Conflict(
            "owner-held original completion changed",
        ));
    }
    crate::completed_state::require_hold(reader, owner, expected.record()).await?;
    Ok(OriginalHeldCompletion {
        completion,
        expected,
        _charge: charge,
    })
}
fn exact_mappings(
    closure: &RecoveryClosure,
    mappings: &[ContributorRealization],
) -> Result<(), ModelError> {
    if mappings
        .windows(2)
        .any(|pair| (pair[0].view, pair[0].logical) >= (pair[1].view, pair[1].logical))
        || mappings.iter().any(|mapping| mapping.incarnation <= 0)
        || !closure
            .views()
            .values()
            .flat_map(|view| {
                view.contributions
                    .iter()
                    .map(move |logical| (view.identity, *logical))
            })
            .eq(mappings
                .iter()
                .map(|mapping| (mapping.view, mapping.logical)))
    {
        return Err(ModelError::Conflict(
            "installed exact dependency realization inventory",
        ));
    }
    let mut physical = BTreeMap::new();
    for mapping in mappings {
        let selected = (mapping.logical, mapping.incarnation);
        if physical
            .insert(mapping.physical, selected)
            .is_some_and(|previous| previous != selected)
        {
            return Err(ModelError::Conflict(
                "installed exact dependency realization inventory",
            ));
        }
    }
    Ok(())
}
/// Reads only explicitly supplied physical contributors. A matching logical copy elsewhere
/// in the installation cannot satisfy a missing, replaced or malformed selected contributor.
pub(crate) async fn inspect_closure(
    reader: &NativeReader<()>,
    root: &CompletedView,
    mappings: &[ContributorRealization],
    budget: &ResourceBudget,
) -> Result<InspectedClosure, ModelError> {
    use d::HeapSize;
    fn view_heap(view: &CompletedView) -> usize {
        view.relation.capacity().saturating_add(
            view.contributions
                .len()
                .saturating_mul(std::mem::size_of::<ContentHash>() + 32),
        )
    }
    fn contribution_heap(value: &CompletedContribution) -> usize {
        value
            .spec
            .producer
            .capacity()
            .saturating_add(value.spec.inputs.heap_bytes())
            .saturating_add(value.spec.outputs.heap_bytes())
            .saturating_add(
                value
                    .spec
                    .captured_binding
                    .as_ref()
                    .map_or(0, |binding| binding.heap_bytes()),
            )
            .saturating_add(
                value
                    .outputs
                    .iter()
                    .map(|(name, _)| {
                        std::mem::size_of::<(String, d::completed::OutputContent)>()
                            + 32
                            + name.capacity()
                    })
                    .fold(0, usize::saturating_add),
            )
    }
    fn pending_capacity(count: usize) -> Result<usize, ModelError> {
        count
            .max(4)
            .checked_next_power_of_two()
            .and_then(|capacity| capacity.checked_mul(std::mem::size_of::<CompletedView>()))
            .ok_or_else(|| {
                ModelError::Invalid("installed traversal pending capacity overflow".into())
            })
    }
    // Validation constructs an independent CompletedView, including its set clone.
    let mut validation_bytes = view_heap(root);
    let mut validation = budget.reserve(
        "installed closure view validation scratch",
        validation_bytes,
    )?;
    root.validate()?;
    let mut charge = StateCharge::new(budget, "installed exact closure descriptors");
    charge.grow(
        mappings
            .len()
            .saturating_mul(std::mem::size_of::<ContributorRealization>()),
    )?;
    if mappings
        .windows(2)
        .any(|pair| (pair[0].view, pair[0].logical) >= (pair[1].view, pair[1].logical))
    {
        return Err(ModelError::Conflict(
            "installed selected mapping order/duplicate",
        ));
    }
    let mut descriptor_charge = StateCharge::new(budget, "installed local contributor descriptors");
    let mut contributors = BTreeMap::new();
    let mut descriptors = BTreeMap::new();
    let mut identity_bytes = 0usize;
    for mapping in mappings {
        if mapping.incarnation <= 0 {
            return Err(ModelError::Conflict("installed selected incarnation"));
        }
        let expected = crate::control::inspect_expected_read(
            reader,
            RecordId::new("compiler_contribution", mapping.physical.hex()),
            budget,
        )
        .await?
        .ok_or(ModelError::Conflict(
            "installed selected contributor missing",
        ))?;
        if expected.incarnation() != mapping.incarnation {
            return Err(ModelError::Conflict(
                "installed selected contributor incarnation",
            ));
        }
        // ExpectedRecord already retains and charges its native row. Only this
        // iteration's descriptor decode is additional scratch.
        let decode_bytes = crate::loader::native_bytes(expected.row())
            .saturating_mul(32)
            .saturating_add(65536);
        let _decode = budget.reserve("installed selected contributor decode", decode_bytes)?;
        identity_bytes = identity_bytes.max(decode_bytes);
        validate_state_row("compiler_contribution", expected.row())?;
        let data: CompletedContribution = descriptor(expected.row())?;
        if data.identity()? != mapping.logical {
            return Err(ModelError::Conflict(
                "installed selected contributor logical identity",
            ));
        }
        if let Some(previous) = descriptors.get(&mapping.logical) {
            if previous != &data {
                return Err(ModelError::Conflict(
                    "installed equivalent copies differ in full descriptor",
                ));
            }
        } else {
            descriptor_charge.grow(
                std::mem::size_of::<(ContentHash, CompletedContribution)>()
                    + 32
                    + contribution_heap(&data),
            )?;
            descriptors.insert(mapping.logical, data);
        }
        if !contributors.contains_key(&mapping.physical) {
            // Row payload reservation moves with ExpectedRecord; this is only the map entry.
            charge.grow(std::mem::size_of::<(ContentHash, ExpectedRecord)>() + 32)?;
        }
        contributors.insert(mapping.physical, expected);
    }
    // RecoveryTraversal retains unique logical descriptors, but its private pending
    // Vec may contain repeated dependencies and keeps capacity after popping. Admit
    // a conservative single-traversal bound for every dependency queued by each new
    // contributor, including the root and power-of-two capacity highwater. This
    // working bound is released at traversal completion, not retained in its result.
    // include_view revalidates contribution identity after decode scratch ends:
    // output-key sets, input identity vectors and captured-binding validation are
    // additional allocations. Preserve the existing descriptor decode/validation
    // bound for the largest selected contributor, independently of retained copies.
    let _identity = budget.reserve(
        "installed closure contributor validation scratch",
        identity_bytes,
    )?;
    let mut working = StateCharge::new(budget, "installed closure traversal working state");
    let mut queued = 1usize;
    let mut pending_bytes = pending_capacity(queued)?;
    working.grow(pending_bytes.saturating_add(view_heap(root)))?;
    let mut scheduled = std::collections::BTreeSet::new();
    let mut traversal = RecoveryTraversal::for_views(vec![root.clone()])?;
    while let Some(requested) = traversal.next_view()? {
        let mut variables = Variables::new();
        variables.insert(
            "view",
            RecordId::new("compiler_view", requested.identity.hex()),
        );
        let _read = budget.reserve("installed exact view read", std::mem::size_of::<Value>())?;
        let rows: Vec<Value> = reader
            .query_native("SELECT * FROM $view", variables)
            .await?;
        let [actual] = rows.as_slice() else {
            return Err(ModelError::Conflict("installed selected view cardinality"));
        };
        let _decode = budget.reserve(
            "installed exact view decode",
            crate::loader::native_bytes(actual)
                .saturating_mul(32)
                .saturating_add(65536),
        )?;
        validate_state_row("compiler_view", actual)?;
        let actual: CompletedView = descriptor(actual)?;
        let actual_validation = view_heap(&actual);
        if actual_validation > validation_bytes {
            validation.try_resize(actual_validation)?;
            validation_bytes = actual_validation;
        }
        let mut local = StateCharge::new(budget, "installed closure local selected descriptors");
        local.grow(requested.contributions.len().saturating_mul(
            std::mem::size_of::<CompletedContribution>() + std::mem::size_of::<ContentHash>() + 32,
        ))?;
        let mut selected = Vec::with_capacity(requested.contributions.len());
        let mut dependencies = BTreeMap::new();
        for logical in &requested.contributions {
            let contribution = descriptors
                .get(logical)
                .ok_or(ModelError::Conflict("installed missing dependency mapping"))?;
            local.grow(contribution_heap(contribution))?;
            selected.push(contribution.clone());
            let mapping = &mappings[mappings
                .binary_search_by_key(&(requested.identity, *logical), |mapping| {
                    (mapping.view, mapping.logical)
                })
                .map_err(|_| ModelError::Conflict("installed selected mapping disappeared"))?];
            crate::completed_state::require_hold(
                reader,
                &RecordId::new("compiler_view", requested.identity.hex()),
                &RecordId::new("compiler_contribution", mapping.physical.hex()),
            )
            .await?;
            for input in &contribution.spec.inputs {
                // Reusing dependency metadata does not discharge another physical
                // contributor's edge. Check every selected source independently.
                crate::completed_state::require_hold(
                    reader,
                    &RecordId::new("compiler_contribution", mapping.physical.hex()),
                    &RecordId::new("compiler_view", input.view().hex()),
                )
                .await?;
                if dependencies.contains_key(&input.view()) {
                    continue;
                }
                let _page = budget.reserve(
                    "installed dependency view read",
                    std::mem::size_of::<Value>(),
                )?;
                let mut variables = Variables::new();
                variables.insert("view", RecordId::new("compiler_view", input.view().hex()));
                let rows: Vec<Value> = reader
                    .query_native("SELECT * FROM $view", variables)
                    .await?;
                let [row] = rows.as_slice() else {
                    return Err(ModelError::Conflict(
                        "installed dependency view cardinality",
                    ));
                };
                let _decode = budget.reserve(
                    "installed dependency view decode",
                    crate::loader::native_bytes(row)
                        .saturating_mul(32)
                        .saturating_add(65536),
                )?;
                validate_state_row("compiler_view", row)?;
                let dependency: CompletedView = descriptor(row)?;
                local.grow(
                    std::mem::size_of::<(ContentHash, CompletedView)>()
                        + 32
                        + view_heap(&dependency),
                )?;
                dependencies.insert(input.view(), dependency);
            }
            if !scheduled.contains(logical) {
                working.grow(std::mem::size_of::<ContentHash>() + 32)?;
                // include_view queues inputs only for a contributor first entering
                // its logical closure; repeated input roles still own separate clones.
                for input in &contribution.spec.inputs {
                    let dependency = dependencies
                        .get(&input.view())
                        .ok_or(ModelError::Conflict("installed undeclared dependency view"))?;
                    let dependency_validation = view_heap(dependency);
                    if dependency_validation > validation_bytes {
                        validation.try_resize(dependency_validation)?;
                        validation_bytes = dependency_validation;
                    }
                    queued = queued.checked_add(1).ok_or_else(|| {
                        ModelError::Invalid("installed traversal pending count overflow".into())
                    })?;
                    let capacity = pending_capacity(queued)?;
                    working.grow(
                        capacity
                            .saturating_sub(pending_bytes)
                            .saturating_add(view_heap(dependency)),
                    )?;
                    pending_bytes = capacity;
                }
                charge.grow(
                    std::mem::size_of::<(ContentHash, CompletedContribution)>()
                        + 32
                        + contribution_heap(contribution),
                )?;
                scheduled.insert(*logical);
            }
        }
        charge
            .grow(std::mem::size_of::<(ContentHash, CompletedView)>() + 32 + view_heap(&actual))?;
        traversal.include_view(&requested, actual, selected, |identity| {
            dependencies
                .get(&identity)
                .cloned()
                .ok_or(ModelError::Conflict("installed undeclared dependency view"))
        })?;
    }
    let logical = traversal.finish()?;
    let _mapping_check = budget.reserve(
        "installed exact mapping correspondence",
        mappings
            .len()
            .saturating_mul(std::mem::size_of::<(ContentHash, (ContentHash, i64))>() + 32),
    )?;
    exact_mappings(&logical, mappings)?;
    Ok(InspectedClosure {
        logical,
        contributors,
        mappings: mappings.to_vec(),
        _charge: charge,
    })
}

/// Complete output EOF is checked for every selected contribution, including outputs
/// unrelated to the root's relation and explicit empty outputs. Root member preparation
/// below may select only the requested relation after this independent audit succeeds.
pub(crate) async fn audit_memberships(
    reader: &NativeReader<()>,
    closure: &InspectedClosure,
    budget: &ResourceBudget,
) -> Result<(), ModelError> {
    let mut audit = InstallationAudit::new(reader.shared_client(), budget).await?;
    let result = audit_memberships_scoped(reader, closure, budget, &mut audit).await;
    if result
        .as_ref()
        .err()
        .is_some_and(|error| !error.permits_storage_cleanup())
    {
        audit.retain_unknown();
    }
    let mut terminal = d::completion::Completion::default();
    terminal.step(
        "installed audit observation pin release",
        audit.close().await,
    );
    d::completion::complete(result, terminal)
}
pub(crate) async fn audit_memberships_scoped(
    reader: &NativeReader<()>,
    closure: &InspectedClosure,
    budget: &ResourceBudget,
    audit: &mut InstallationAudit,
) -> Result<(), ModelError> {
    use d::{Key, KeySink, completed::OutputContent};
    audit.check(reader, budget)?;
    for mapping in &closure.mappings {
        let descriptor = &closure.logical.contributions()[&mapping.logical];
        let owner = RecordId::new("compiler_contribution", mapping.physical.hex());
        let expected = &closure.contributors[&mapping.physical];
        if let Some((incarnation, row)) = audit.contributors.get(&owner) {
            let current = crate::control::inspect_expected_read(reader, owner.clone(), budget)
                .await?
                .ok_or(ModelError::Conflict("reused installed contributor missing"))?;
            if current.incarnation() != *incarnation
                || current.row() != row
                || current.row() != expected.row()
            {
                return Err(ModelError::Conflict(
                    "reused installed contributor premise changed",
                ));
            }
            continue;
        }
        audit.pin.protect(owner.clone()).await?;
        let protected = crate::control::inspect_expected_read(reader, owner.clone(), budget)
            .await?
            .ok_or(ModelError::Conflict(
                "protected installed contributor missing",
            ))?;
        if protected.incarnation() != expected.incarnation() || protected.row() != expected.row() {
            return Err(ModelError::Conflict(
                "protected installed contributor changed",
            ));
        }
        let _outputs = budget.reserve(
            "installed output digest inventory",
            serde_json::to_vec(&descriptor.outputs)
                .map_err(ModelError::codec)?
                .len()
                .saturating_mul(8)
                .saturating_add(1024),
        )?;
        let mut outputs = descriptor
            .outputs
            .keys()
            .map(|name| {
                (
                    name.clone(),
                    (0u64, KeySink::new("native-contribution-output/v1")),
                )
            })
            .collect::<BTreeMap<_, _>>();
        let mut after_relation = String::new();
        let mut after_key = String::new();
        let mut variables = Variables::new();
        variables.insert("owner", owner.clone());
        let statements=vec!["SELECT * FROM compiler_membership WITH INDEX contribution_rows WHERE contribution=$owner ORDER BY relation,semantic_key".into()];
        #[cfg(test)]
        let statements = {
            let mut statements = statements;
            if LATE_MEMBERSHIP_FAILURE
                .try_with(|enabled| *enabled)
                .unwrap_or(false)
            {
                statements.push("THROW 'late independent membership audit control'".into());
            }
            statements
        };
        let mut stream = reader.stream_prepared(crate::prepared::PreparedQuery::new(
            variables,
            vec![],
            statements,
        )?)?;
        let observed = async {
            let mut window = Vec::new();
            let mut charge = StateCharge::new(budget, "installed membership window");
            while let Some(row) = stream.next().await? {
                let bytes = crate::loader::native_bytes(&row)
                    .saturating_mul(8)
                    .saturating_add(512);
                charge.grow(bytes)?;
                let row = &row;
                validate_state_row("compiler_membership", row)?;
                let object = row
                    .as_object()
                    .ok_or(ModelError::Schema("installed membership object"))?;
                let text = |field: &str| -> Result<String, ModelError> {
                    match object.get(field) {
                        Some(Value::String(value)) => Ok(value.clone()),
                        _ => Err(ModelError::Schema("installed membership text")),
                    }
                };
                let relation = text("relation")?;
                let key = text("semantic_key")?;
                let content = text("content")?;
                if object.get("contribution") != Some(&Value::RecordId(owner.clone()))
                    || (!after_relation.is_empty()
                        && (relation.as_str(), key.as_str())
                            <= (after_relation.as_str(), after_key.as_str()))
                {
                    return Err(ModelError::Conflict(
                        "installed membership owner/order/duplicate",
                    ));
                }
                let (count, sink) = outputs.get_mut(&relation).ok_or(ModelError::Conflict(
                    "installed undeclared output membership",
                ))?;
                key.encode(sink);
                content.encode(sink);
                *count = count
                    .checked_add(1)
                    .ok_or(ModelError::Schema("installed membership count"))?;
                let Some(Value::RecordId(node)) = object.get("node") else {
                    return Err(ModelError::Schema("installed membership node"));
                };
                let _ = node;
                after_relation = relation;
                after_key = key;
                window.push(row.clone());
                if window.len() >= crate::loader::NATIVE_WINDOW_ROWS
                    || charge.reserved() >= d::resources::TRANSFER_BYTES
                {
                    audit_membership_window(reader, &owner, &window, budget, audit).await?;
                    window.clear();
                    charge = StateCharge::new(budget, "installed membership window");
                }
            }
            if !window.is_empty() {
                audit_membership_window(reader, &owner, &window, budget, audit).await?;
            }
            Ok::<_, ModelError>(())
        }
        .await;
        let mut terminal = d::completion::Completion::default();
        terminal.step(
            "installed membership complete stream drain",
            stream.drain_transport().await,
        );
        d::completion::complete(observed, terminal)?;
        let actual = outputs
            .into_iter()
            .map(|(relation, (rows, sink))| {
                (
                    relation,
                    OutputContent {
                        rows,
                        content: sink.finish(),
                    },
                )
            })
            .collect::<BTreeMap<_, _>>();
        if actual != descriptor.outputs {
            return Err(ModelError::Conflict("installed complete output EOF/digest"));
        }
        audit.charge.grow(
            crate::loader::native_bytes(expected.row())
                .saturating_mul(4)
                .saturating_add(512),
        )?;
        audit
            .contributors
            .insert(owner, (expected.incarnation(), expected.row().clone()));
        audit.contribution_observations += 1;
    }
    Ok(())
}
/// Stream exact point sets and require one active guard for every requested key.
/// Canonical payloads remain streamed; only small correspondence metadata is retained.
async fn audit_active_guards(
    reader: &NativeReader<()>,
    ids: &std::collections::BTreeSet<RecordId>,
    budget: &ResourceBudget,
) -> Result<(), ModelError> {
    let mut charge = StateCharge::new(budget, "installed audit guard correspondence");
    let mut guards = std::collections::BTreeSet::new();
    for id in ids {
        let guard = crate::control::guard_id(&Value::RecordId(id.clone()))?;
        charge.grow(
            crate::selection::node_bytes(&guard)
                .saturating_mul(3)
                .saturating_add(128),
        )?;
        guards.insert(guard);
    }
    for window in guards
        .iter()
        .cloned()
        .collect::<Vec<_>>()
        .chunks(crate::loader::NATIVE_WINDOW_ROWS)
    {
        let mut variables = Variables::new();
        variables.insert("guards", window.to_vec());
        let mut stream = reader.stream_prepared(crate::prepared::PreparedQuery::new(
            variables,
            vec![],
            vec!["SELECT id,phase,incarnation FROM $guards".into()],
        )?)?;
        let result=async {
            let mut seen=std::collections::BTreeSet::new();
            while let Some(value)=stream.next().await? {
                let row=value.as_object().ok_or(ModelError::Schema("installed set guard object"))?;
                let Some(Value::RecordId(id))=row.get("id") else{return Err(ModelError::Schema("installed set guard identity"));};
                if !window.contains(id) || !seen.insert(id.clone()) || row.get("phase")!=Some(&Value::String("active".into()))
                    || !matches!(row.get("incarnation"),Some(Value::Number(surrealdb::types::Number::Int(value))) if *value>0) {
                    return Err(ModelError::Conflict("installed set guard missing/duplicate/foreign/inactive"));
                }
            }
            if seen.len()!=window.len(){return Err(ModelError::Conflict("installed set guard missing"));}Ok(())
        }.await;
        let mut terminal = d::completion::Completion::default();
        terminal.step(
            "installed set guard stream drain",
            stream.drain_transport().await,
        );
        d::completion::complete(result, terminal)?;
    }
    Ok(())
}
pub(crate) async fn audit_owned_keys<Context>(
    reader: &NativeReader<Context>,
    owner: &RecordId,
    ids: &std::collections::BTreeSet<RecordId>,
    budget: &ResourceBudget,
) -> Result<(), ModelError> {
    let _charge = budget.reserve(
        "installed set hold correspondence",
        ids.iter()
            .map(|id| {
                crate::selection::node_bytes(id)
                    .saturating_mul(6)
                    .saturating_add(256)
            })
            .sum(),
    )?;
    for window in ids
        .iter()
        .cloned()
        .collect::<Vec<_>>()
        .chunks(crate::loader::NATIVE_WINDOW_ROWS)
    {
        let mut variables = Variables::new();
        variables.insert("owner", owner.clone());
        variables.insert("objects", window.to_vec());
        let mut stream=reader.stream_prepared(crate::prepared::PreparedQuery::new(variables,vec![],vec!["SELECT id,owner,object FROM native_hold WITH INDEX owner_holds WHERE owner=$owner AND object IN $objects".into()])?)?;
        let result = async {
            let mut seen = std::collections::BTreeSet::new();
            while let Some(value) = stream.next().await? {
                let row = value
                    .as_object()
                    .ok_or(ModelError::Schema("installed set hold object"))?;
                let Some(Value::RecordId(object)) = row.get("object") else {
                    return Err(ModelError::Schema("installed set hold target"));
                };
                if row.get("owner") != Some(&Value::RecordId(owner.clone()))
                    || !window.contains(object)
                    || !seen.insert(object.clone())
                {
                    return Err(ModelError::Conflict(
                        "installed set hold owner/duplicate/foreign",
                    ));
                }
            }
            if seen.len() != window.len() {
                return Err(ModelError::Conflict("installed set hold missing"));
            }
            Ok(())
        }
        .await;
        let mut terminal = d::completion::Completion::default();
        terminal.step(
            "installed set hold stream drain",
            stream.drain_transport().await,
        );
        d::completion::complete(result, terminal)?;
    }
    Ok(())
}
async fn audit_endpoint_window(
    reader: &NativeReader<()>,
    owner: &RecordId,
    expected: &BTreeMap<RecordId, Value>,
    budget: &ResourceBudget,
) -> Result<(), ModelError> {
    if expected.is_empty() {
        return Ok(());
    }
    let ids = expected
        .keys()
        .cloned()
        .collect::<std::collections::BTreeSet<_>>();
    audit_active_guards(reader, &ids, budget).await?;
    audit_owned_keys(reader, owner, &ids, budget).await?;
    let mut variables = Variables::new();
    variables.insert("ids", ids.into_iter().collect::<Vec<_>>());
    let mut stream = reader
        .stream_prepared(crate::prepared::PreparedQuery::new(
            variables,
            vec![],
            vec!["SELECT * FROM $ids".into()],
        )?)?
        .with_row_bytes(d::resources::MAX_ROW_BYTES);
    let result = async {
        let mut seen = std::collections::BTreeSet::new();
        while let Some(row) = stream.next().await? {
            let Some(Value::RecordId(id)) = row.as_object().and_then(|row| row.get("id")) else {
                return Err(ModelError::Schema("installed set endpoint identity"));
            };
            if !seen.insert(id.clone()) || expected.get(id) != Some(&row) {
                return Err(ModelError::Conflict(
                    "installed set endpoint bytes/duplicate/foreign",
                ));
            }
        }
        if seen.len() != expected.len() {
            return Err(ModelError::Conflict(
                "installed canonical endpoint missing before freeze",
            ));
        }
        Ok(())
    }
    .await;
    let mut terminal = d::completion::Completion::default();
    terminal.step(
        "installed set endpoint stream drain",
        stream.drain_transport().await,
    );
    d::completion::complete(result, terminal)
}
async fn audit_membership_window(
    reader: &NativeReader<()>,
    owner: &RecordId,
    members: &[Value],
    budget: &ResourceBudget,
    audit: &mut InstallationAudit,
) -> Result<(), ModelError> {
    let mut metadata = BTreeMap::new();
    let mut charge = StateCharge::new(budget, "installed set backing correspondence");
    for row in members {
        let object = row
            .as_object()
            .ok_or(ModelError::Schema("installed set member object"))?;
        let Some(Value::RecordId(node)) = object.get("node") else {
            return Err(ModelError::Schema("installed set member backing"));
        };
        charge.grow(
            crate::selection::node_bytes(node)
                .saturating_mul(5)
                .saturating_add(512),
        )?;
        if let Some(previous) = metadata.insert(node.clone(), object) {
            if ["relation", "semantic_key", "content"]
                .iter()
                .any(|field| previous.get(*field) != object.get(*field))
            {
                return Err(ModelError::Conflict(
                    "installed membership ambiguous backing",
                ));
            }
        }
    }
    let ids = metadata
        .keys()
        .cloned()
        .collect::<std::collections::BTreeSet<_>>();
    audit_active_guards(reader, &ids, budget).await?;
    audit_owned_keys(reader, owner, &ids, budget).await?;
    let mut variables = Variables::new();
    variables.insert("ids", ids.into_iter().collect::<Vec<_>>());
    let mut stream = reader
        .stream_prepared(crate::prepared::PreparedQuery::new(
            variables,
            vec![],
            vec!["SELECT * FROM $ids".into()],
        )?)?
        .with_row_bytes(d::resources::MAX_ROW_BYTES);
    let result = async {
        let mut seen = std::collections::BTreeSet::new();
        let mut endpoints = BTreeMap::new();
        let mut endpoint_charge = StateCharge::new(budget, "installed endpoint set preparation");
        while let Some(row) = stream.next().await? {
            let backing = row
                .as_object()
                .ok_or(ModelError::Schema("installed set backing object"))?;
            let Some(Value::RecordId(node)) = backing.get("id") else {
                return Err(ModelError::Schema("installed set backing identity"));
            };
            let member = metadata
                .get(node)
                .ok_or(ModelError::Conflict("installed foreign backing"))?;
            if !seen.insert(node.clone())
                || backing.get("semantic_type") != member.get("relation")
                || backing.get("semantic_key") != member.get("semantic_key")
                || backing.get("content") != member.get("content")
            {
                return Err(ModelError::Conflict(
                    "installed membership backing metadata/duplicate",
                ));
            }
            let (expected, original, _decode) = audit_backing(&row, budget)?;
            if let Some(original) = original {
                audit_original(reader, &original, budget, audit).await?;
            }
            for endpoint in expected {
                let Some(Value::RecordId(id)) = endpoint.as_object().and_then(|row| row.get("id"))
                else {
                    return Err(ModelError::Schema("installed canonical endpoint id"));
                };
                let bytes = crate::loader::native_bytes(&endpoint)
                    .saturating_mul(8)
                    .saturating_add(512);
                if !endpoints.is_empty()
                    && (endpoints.len() >= crate::loader::NATIVE_WINDOW_ROWS
                        || endpoint_charge.reserved().saturating_add(bytes)
                            > d::resources::TRANSFER_BYTES)
                {
                    audit_endpoint_window(reader, owner, &endpoints, budget).await?;
                    endpoints.clear();
                    endpoint_charge =
                        StateCharge::new(budget, "installed endpoint set preparation");
                }
                if let Some(previous) = endpoints.get(id) {
                    if previous != &endpoint {
                        return Err(ModelError::Conflict(
                            "installed endpoint expected collision",
                        ));
                    }
                } else {
                    endpoint_charge.grow(bytes)?;
                    endpoints.insert(id.clone(), endpoint);
                }
            }
        }
        if seen.len() != metadata.len() {
            return Err(ModelError::Conflict("installed membership backing missing"));
        }
        audit_endpoint_window(reader, owner, &endpoints, budget).await
    }
    .await;
    let mut terminal = d::completion::Completion::default();
    terminal.step(
        "installed set backing stream drain",
        stream.drain_transport().await,
    );
    d::completion::complete(result, terminal)
}
fn audit_backing(
    row: &Value,
    budget: &ResourceBudget,
) -> Result<
    (
        Vec<Value>,
        Option<crate::completed_rows::OriginalBacking>,
        Box<dyn d::resources::Reservation>,
    ),
    ModelError,
> {
    use d::graph::{Assertion, Entity};
    let object = row
        .as_object()
        .ok_or(ModelError::Schema("installed backing object"))?;
    let Some(Value::RecordId(node)) = object.get("id") else {
        return Err(ModelError::Schema("installed backing id"));
    };
    let _decode = budget.reserve(
        "installed backing canonical decode",
        crate::loader::native_bytes(row)
            .saturating_mul(32)
            .saturating_add(65536),
    )?;
    let canonical = match object.get("canonical") {
        Some(Value::Bytes(bytes)) => bytes,
        _ => return Err(ModelError::Schema("installed canonical backing")),
    };
    let (expected, endpoints) = match node.table.as_str() {
        "compiler_record" => {
            let original = validate_state_row("compiler_record", row)?;
            crate::completed_rows::validate_backing_batch(std::slice::from_ref(row), budget)?;
            return Ok((vec![], original, _decode));
        }
        "entity" => {
            let entity: Entity = serde_json::from_slice(canonical).map_err(ModelError::codec)?;
            entity.validate()?;
            let view = crate::codec::entity_views(std::slice::from_ref(&entity))?
                .pop()
                .ok_or(ModelError::Schema("installed entity view"))?;
            (
                crate::adapter::physical_row(
                    crate::loader::entity_payload_id(&entity)?,
                    entity.content(),
                    serde_json::to_vec(&entity).map_err(ModelError::codec)?,
                    Some((entity.kind() as i64, entity.subtype())),
                    view,
                )?,
                crate::loader::entity_endpoint_values(&entity)?,
            )
        }
        "assertion" => {
            let assertion: Assertion =
                serde_json::from_slice(canonical).map_err(ModelError::codec)?;
            assertion.validate()?;
            let view = crate::codec::assertion_views(std::slice::from_ref(&assertion))?
                .pop()
                .ok_or(ModelError::Schema("installed assertion view"))?;
            (
                crate::adapter::physical_row(
                    crate::loader::assertion_payload_id(&assertion)?,
                    assertion.content(),
                    serde_json::to_vec(&assertion).map_err(ModelError::codec)?,
                    Some((assertion.kind as i64, None)),
                    view,
                )?,
                crate::loader::assertion_endpoint_values(&assertion)?,
            )
        }
        _ => return Err(ModelError::Conflict("installed undeclared backing family")),
    };
    if expected != *row {
        return Err(ModelError::Conflict(
            "installed canonical graph backing projection",
        ));
    }
    Ok((endpoints, None, _decode))
}
async fn audit_original(
    reader: &NativeReader<()>,
    original: &crate::completed_rows::OriginalBacking,
    budget: &ResourceBudget,
    audit: &mut InstallationAudit,
) -> Result<(), ModelError> {
    let source = d::graph::EntityId::of(original.artifact);
    let layout = crate::originals::layout(reader, &RecordId::new("original", source.0.hex()))
        .await?
        .ok_or(ModelError::Conflict("installed original layout"))?;
    crate::originals::validate_layout(reader, &layout).await?;
    if !crate::originals::header(reader, &layout.blob).await? {
        return Err(ModelError::Conflict("installed original incomplete blob"));
    }
    let remaining = layout
        .blob
        .byte_len
        .checked_sub(original.start)
        .ok_or(ModelError::Conflict("installed original coverage"))?;
    if original.len as u64 != remaining.min(d::artifact::ARTIFACT_CHUNK_BYTES as u64) {
        return Err(ModelError::Conflict("installed original coverage"));
    }
    let mut hash = d::ContentHasher::default();
    let mut offset = 0;
    while offset < original.len {
        let length = (original.len - offset).min(256 << 10);
        let bytes = reader
            .original_bytes(source, original.start + offset as u64, length)
            .await?;
        hash.update(&bytes);
        offset += length;
    }
    if hash.finish() != original.digest {
        return Err(ModelError::Conflict("installed original content digest"));
    }
    let blob = layout.blob.id();
    if !audit.blobs.contains_key(&blob) {
        audit.pin.protect(blob.clone()).await?;
    }
    let premise = crate::originals::active_blob_premise(reader, &layout.blob).await?;
    if audit
        .blobs
        .get(&blob)
        .is_some_and(|observed| *observed != premise)
    {
        return Err(ModelError::Conflict(
            "installed original protected incarnation changed",
        ));
    }
    if !audit.blobs.contains_key(&blob) {
        #[cfg(test)]
        if let Ok(barrier) = BLOB_OBSERVATION_BARRIER.try_with(Clone::clone) {
            barrier.observed.wait().await;
            barrier.resume.wait().await;
        }
        crate::originals::validate_blob(reader, &layout.blob, budget).await?;
        if crate::originals::active_blob_premise(reader, &layout.blob).await? != premise {
            return Err(ModelError::Conflict(
                "installed original observation premise changed",
            ));
        }
        if !audit.blobs.contains_key(&blob) {
            audit.charge.grow(
                crate::selection::node_bytes(&blob)
                    .saturating_mul(4)
                    .saturating_add(128),
            )?;
        }
        audit.blobs.insert(blob, premise);
        audit.blob_observations += 1;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use d::{
        Record, Relation,
        analysis::sources::SourceSnapshot,
        completed::{ContributionSpec, OutputContent, OutputProfile},
        input::Package,
        stages::Profile,
    };
    use surrealdb::types::SurrealValue;
    #[tokio::test(flavor = "multi_thread")]
    async fn completed_blob_observation_survives_peer_hold_and_equal_replay_under_exact_pin() {
        use d::{
            EvidenceBytes, artifact::ArtifactChunk, input::InputRevision, source::SourceArtifact,
        };
        use futures::FutureExt;
        use lctx_compiler_store::compiler::NativeCompilerStore;
        let config = lctx_compiler_store::RuntimeConfig::read(std::path::Path::new(
            &std::env::var_os("LCTX_COMPILER_RUNTIME_CONFIG").expect("fresh owned native campaign"),
        ))
        .unwrap();
        let store = NativeCompilerStore::begin(&config, d::admission::Frontier::Facts)
            .await
            .unwrap();
        let client = lctx_compiler_store::reader::connect(
            &config.endpoint,
            &config.writer_credentials(),
            store.namespace().as_str(),
            store.database().as_str(),
        )
        .await
        .unwrap();
        let budget = ResourceBudget::fixed(256 << 20).unwrap();
        let reader = NativeReader::private(client.clone()).with_budget(&budget);
        let mut audit = InstallationAudit::new(client.clone(), &budget)
            .await
            .unwrap();
        let peer = crate::control::ReaderPin::acquire(client.clone(), &[])
            .await
            .unwrap();
        let result = std::panic::AssertUnwindSafe(async {
            let sources = Relation::of::<SourceArtifact>();
            let chunks = Relation::of::<ArtifactChunk>();
            let relations = [sources.clone(), chunks.clone()];
            let mut specification = contribution("completed-blob-owner-replay", vec![]).spec;
            specification.outputs = relations
                .iter()
                .map(|relation| relation.name().into())
                .collect();
            let producer = store.begin_contribution(specification).await.unwrap();
            let bytes = [store.attempt().0.as_slice(), b"completed blob peer replay"].concat();
            let source = SourceArtifact::from_bytes(
                InputRevision::from_entries(vec![]).unwrap().id(),
                "peer.bin".into(),
                &bytes,
            )
            .unwrap();
            let chunk = ArtifactChunk {
                artifact: source.id(),
                ordinal: 0,
                body: EvidenceBytes(bytes.clone()),
            };
            store
                .write_batch(
                    &producer,
                    &sources,
                    &SourceArtifact::encode(std::slice::from_ref(&source)).unwrap(),
                    &budget,
                )
                .await
                .unwrap();
            store
                .write_batch(
                    &producer,
                    &chunks,
                    &ArtifactChunk::encode(&[chunk]).unwrap(),
                    &budget,
                )
                .await
                .unwrap();
            let views = store
                .complete_contribution(
                    producer,
                    d::stages::ProviderOutcome::Complete,
                    &relations,
                    &BTreeMap::new(),
                    &budget,
                )
                .await
                .unwrap();
            let view = &views[chunks.name()];
            let held = store.installed_view(view, &budget).await.unwrap();
            let mappings: Vec<ContributorRealization> =
                serde_json::from_value(serde_json::to_value(&held.completion().mappings).unwrap())
                    .unwrap();
            let closure = inspect_closure(&reader, view, &mappings, &budget)
                .await
                .unwrap();
            let layout = crate::originals::Layout::source(&source).unwrap();
            let barrier = BlobObservationBarrier {
                observed: std::sync::Arc::new(tokio::sync::Barrier::new(2)),
                resume: std::sync::Arc::new(tokio::sync::Barrier::new(2)),
            };
            let replay = async {
                barrier.observed.wait().await;
                peer.protect(layout.blob.id()).await.unwrap();
                for window in crate::control::ImmutableWindows::new(
                    vec![layout.blob.piece(0, &bytes).unwrap()],
                    Some(store.attempt()),
                    None,
                    None,
                )
                .with_budget(Some(&budget))
                {
                    crate::control::install_window(
                        &client,
                        Some(store.attempt()),
                        window.unwrap(),
                        Some(&budget),
                    )
                    .await
                    .unwrap();
                }
                barrier.resume.wait().await;
            };
            let (observed, ()) = tokio::join!(
                BLOB_OBSERVATION_BARRIER.scope(
                    barrier.clone(),
                    audit_memberships_scoped(&reader, &closure, &budget, &mut audit)
                ),
                replay
            );
            observed.unwrap();
            assert_eq!(audit.observations(), (1, 1));
            audit_memberships_scoped(&reader, &closure, &budget, &mut audit)
                .await
                .unwrap();
            assert_eq!(
                audit.observations(),
                (1, 1),
                "equal replay neither invalidates immutable bytes nor repeats the whole audit"
            );
        })
        .catch_unwind()
        .await;
        let mut cleanup = d::completion::Completion::default();
        cleanup.step("peer blob reader close", reader.close().await);
        cleanup.step("peer blob audit pin close", audit.close().await);
        cleanup.step("peer blob other pin close", peer.release().await);
        cleanup.step("peer blob attempt cleanup", store.abandon().await);
        cleanup.step(
            "peer blob observer invalidation",
            client.invalidate().await.map_err(ModelError::codec),
        );
        let cleaned = d::completion::complete(Ok(()), cleanup);
        if let Err(panic) = result {
            if let Err(error) = cleaned {
                eprintln!("peer blob cleanup: {error:?}");
            }
            std::panic::resume_unwind(panic);
        }
        cleaned.unwrap();
    }
    #[tokio::test(flavor = "multi_thread")]
    async fn late_membership_terminal_failure_cannot_publish_reusable_observation() {
        use futures::FutureExt;
        use lctx_compiler_store::compiler::NativeCompilerStore;
        let config = lctx_compiler_store::RuntimeConfig::read(std::path::Path::new(
            &std::env::var_os("LCTX_COMPILER_RUNTIME_CONFIG").expect("fresh owned native campaign"),
        ))
        .unwrap();
        let store = NativeCompilerStore::begin(&config, d::admission::Frontier::Facts)
            .await
            .unwrap();
        let client = lctx_compiler_store::reader::connect(
            &config.endpoint,
            &config.writer_credentials(),
            store.namespace().as_str(),
            store.database().as_str(),
        )
        .await
        .unwrap();
        let budget = ResourceBudget::fixed(256 << 20).unwrap();
        let reader = NativeReader::private(client.clone()).with_budget(&budget);
        let mut audit = InstallationAudit::new(client.clone(), &budget)
            .await
            .unwrap();
        let result=std::panic::AssertUnwindSafe(async{
            let relation=Relation::of::<Package>();let mut specification=contribution("late-terminal",vec![]).spec;specification.outputs=[relation.name().into()].into();
            let producer=store.begin_contribution(specification).await.unwrap();
            let rows=(0..129).map(|index|Package{name:format!("terminal-package-{index}")}).collect::<Vec<_>>();
            store.write_batch(&producer,&relation,&Package::encode(&rows).unwrap(),&budget).await.unwrap();
            let views=store.complete_contribution(producer,d::stages::ProviderOutcome::Complete,std::slice::from_ref(&relation),&BTreeMap::new(),&budget).await.unwrap();
            let view=&views[relation.name()];let held=store.installed_view(view,&budget).await.unwrap();
            let mappings:Vec<ContributorRealization>=serde_json::from_value(serde_json::to_value(&held.completion().mappings).unwrap()).unwrap();
            let closure=inspect_closure(&reader,view,&mappings,&budget).await.unwrap();
            let failed=LATE_MEMBERSHIP_FAILURE.scope(true,audit_memberships_scoped(&reader,&closure,&budget,&mut audit)).await.unwrap_err();
            assert!(format!("{failed:?}").contains("late independent membership audit control"),"{failed:?}");
            assert_eq!(audit.observations(),(0,0),"valid provisional membership never becomes a complete observation before its physical terminal");
            audit_memberships_scoped(&reader,&closure,&budget,&mut audit).await.unwrap();assert_eq!(audit.observations(),(1,0));
        }).catch_unwind().await;
        let mut cleanup = d::completion::Completion::default();
        cleanup.step("terminal audit reader close", reader.close().await);
        cleanup.step("terminal audit pin close", audit.close().await);
        cleanup.step("terminal audit attempt cleanup", store.abandon().await);
        cleanup.step(
            "terminal audit observer invalidation",
            client.invalidate().await.map_err(ModelError::codec),
        );
        let cleaned = d::completion::complete(Ok(()), cleanup);
        if let Err(panic) = result {
            if let Err(error) = cleaned {
                eprintln!("terminal audit cleanup: {error:?}");
            }
            std::panic::resume_unwind(panic);
        }
        cleaned.unwrap();
    }
    #[tokio::test(flavor = "multi_thread")]
    async fn set_audit_requires_exact_backing_guards_holds_and_endpoint_inventory() {
        use d::input::Release;
        use futures::FutureExt;
        use lctx_compiler_store::compiler::NativeCompilerStore;
        let config = lctx_compiler_store::RuntimeConfig::read(std::path::Path::new(
            &std::env::var_os("LCTX_COMPILER_RUNTIME_CONFIG").expect("fresh owned native campaign"),
        ))
        .unwrap();
        let store = NativeCompilerStore::begin(&config, d::admission::Frontier::Facts)
            .await
            .unwrap();
        let client = lctx_compiler_store::reader::connect(
            &config.endpoint,
            &config.writer_credentials(),
            store.namespace().as_str(),
            store.database().as_str(),
        )
        .await
        .unwrap();
        let budget = ResourceBudget::fixed(256 << 20).unwrap();
        let reader = NativeReader::private(client.clone()).with_budget(&budget);
        let result=std::panic::AssertUnwindSafe(async{
            let packages=Relation::of::<Package>();let releases=Relation::of::<Release>();let relations=[packages.clone(),releases.clone()];
            let mut specification=contribution("set-correspondence",vec![]).spec;specification.outputs=relations.iter().map(|relation|relation.name().into()).collect();
            let producer=store.begin_contribution(specification).await.unwrap();
            let package=Package{name:format!("set-correspondence-{}",store.attempt().hex())};let release=Release{package:package.id(),version:"1".into()};
            store.write_batch(&producer,&packages,&Package::encode(&[package]).unwrap(),&budget).await.unwrap();store.write_batch(&producer,&releases,&Release::encode(std::slice::from_ref(&release)).unwrap(),&budget).await.unwrap();
            let views=store.complete_contribution(producer,d::stages::ProviderOutcome::Complete,&relations,&BTreeMap::new(),&budget).await.unwrap();
            let view=&views[releases.name()];let held=store.installed_view(view,&budget).await.unwrap();
            let mappings:Vec<ContributorRealization>=serde_json::from_value(serde_json::to_value(&held.completion().mappings).unwrap()).unwrap();
            let closure=inspect_closure(&reader,view,&mappings,&budget).await.unwrap();let owner=RecordId::new("compiler_contribution",mappings[0].physical.hex());
            let backing=crate::loader::entity_payload_id(&d::graph::Entity::from(release.clone())).unwrap();
            let endpoints=crate::loader::entity_endpoint_values(&d::graph::Entity::from(release)).unwrap();
            let endpoint=RecordId::from_value(endpoints[0].as_object().unwrap().get("id").unwrap().clone()).unwrap();
            let mut ids=vec![("backing",backing.clone()),("backing guard",crate::control::guard_id(&Value::RecordId(backing.clone())).unwrap()),("endpoint",endpoint.clone()),("endpoint guard",crate::control::guard_id(&Value::RecordId(endpoint.clone())).unwrap())];
            for (kind,object) in [("backing hold",backing),("endpoint hold",endpoint)]{
                let mut variables=Variables::new();variables.insert("owner",owner.clone());variables.insert("object",object);
                let rows:Vec<RecordId>=reader.query_native("SELECT VALUE id FROM native_hold WITH INDEX owner_holds WHERE owner=$owner AND object=$object",variables).await.unwrap();assert_eq!(rows.len(),1);ids.push((kind,rows[0].clone()));
            }
            for (kind,id) in ids{
                let mut audit=InstallationAudit::new(client.clone(),&budget).await.unwrap();audit.pin.protect(owner.clone()).await.unwrap();
                let mut variables=Variables::new();variables.insert("id",id.clone());let original:Vec<Value>=reader.query_native("SELECT * FROM $id",variables.clone()).await.unwrap();assert_eq!(original.len(),1);
                client.query("DELETE $id RETURN NONE").bind(variables.clone()).await.unwrap().check().unwrap();
                let failed=audit_memberships_scoped(&reader,&closure,&budget,&mut audit).await;
                variables.insert("row",original[0].clone());client.query("UPSERT $id CONTENT $row RETURN NONE").bind(variables).await.unwrap().check().unwrap();
                let observations=audit.observations();audit.close().await.unwrap();
                assert!(failed.is_err(),"missing {kind} cannot be replaced by matching counts or nominal identity");assert_eq!(observations,(0,0),"failed {kind} audit cannot publish reuse");
            }
        }).catch_unwind().await;
        let mut cleanup = d::completion::Completion::default();
        cleanup.step("set correspondence reader close", reader.close().await);
        cleanup.step("set correspondence attempt cleanup", store.abandon().await);
        cleanup.step(
            "set correspondence observer invalidation",
            client.invalidate().await.map_err(ModelError::codec),
        );
        let cleaned = d::completion::complete(Ok(()), cleanup);
        if let Err(panic) = result {
            if let Err(error) = cleaned {
                eprintln!("set correspondence cleanup: {error:?}");
            }
            std::panic::resume_unwind(panic);
        }
        cleaned.unwrap();
    }
    fn contribution(name: &str, inputs: Vec<SourceSnapshot>) -> CompletedContribution {
        CompletedContribution {
            assurance:
                lctx_model::domain::producer_assurance::ProducerAssurance::CurrentProviderInterface,
            spec: ContributionSpec {
                captured_binding: None,
                producer: name.into(),
                profile: OutputProfile::Specific(Profile::Catalog),
                model: ContentHash::of(b"model"),
                implementation: ContentHash::of(b"implementation"),
                configuration: None,
                inputs,
                outputs: ["packages".into()].into(),
            },
            outcome: 0,
            outputs: [(
                "packages".into(),
                OutputContent {
                    rows: 0,
                    content: ContentHash::of(b"empty membership EOF"),
                },
            )]
            .into(),
        }
    }
    fn closure() -> (RecoveryClosure, Vec<ContributorRealization>) {
        let dependency = contribution("dependency", vec![]);
        let dep_id = dependency.identity().unwrap();
        let dependency_view = CompletedView::new("packages".into(), [dep_id].into(), 0).unwrap();
        let input = SourceSnapshot::of_completed_view(
            &Relation::of::<Package>(),
            dependency.spec.model,
            &dependency_view,
        )
        .unwrap();
        let root = contribution("root", vec![input]);
        let root_id = root.identity().unwrap();
        let root_view = CompletedView::new("packages".into(), [root_id].into(), 0).unwrap();
        let mut traversal = RecoveryTraversal::for_views(vec![root_view.clone()]).unwrap();
        let requested = traversal.next_view().unwrap().unwrap();
        traversal
            .include_view(&requested, root_view, vec![root], |_| {
                Ok(dependency_view.clone())
            })
            .unwrap();
        let requested = traversal.next_view().unwrap().unwrap();
        traversal
            .include_view(
                &requested,
                dependency_view,
                vec![dependency],
                |_| unreachable!(),
            )
            .unwrap();
        assert!(traversal.next_view().unwrap().is_none());
        let closure = traversal.finish().unwrap();
        let mappings = closure
            .views()
            .values()
            .flat_map(|view| {
                view.contributions
                    .iter()
                    .map(move |logical| (view.identity, *logical))
            })
            .enumerate()
            .map(|(i, (view, logical))| ContributorRealization {
                view,
                logical,
                physical: ContentHash::of(format!("exact-copy-{i}").as_bytes()),
                incarnation: 1,
            })
            .collect();
        (closure, mappings)
    }
    #[test]
    fn immutable_seal_metadata_survives_incidence_revision_changes_only() {
        let (closure, mappings) = closure();
        let view = closure.views().values().next().unwrap().clone();
        let actual = super::super::InstalledViewCompletion {
            generation: ContentHash::of(b"generation"),
            validity_epoch: 1,
            realization_incarnation: 7,
            version: 1,
            realization: super::super::realization_id(&view, &mappings).unwrap(),
            view,
            member_codec: super::super::MEMBER_CODEC,
            mappings,
            closure: d::completed_identity::CompletedFamilyRoots::new(
                d::recovery_closure::COMPLETED_FAMILIES.map(|family| {
                    d::completed_identity::CompletedFamilyHasher::new(family)
                        .unwrap()
                        .finish()
                }),
            )
            .unwrap(),
            membership_rows: 0,
            membership_content: ContentHash::of(b"audited EOF"),
            frozen_revision: 19,
        };
        let mut current = actual.clone();
        current.frozen_revision = 91;
        assert!(same_completion_metadata(&current, &actual));
        assert_eq!(actual.frozen_revision, 19);
        assert_ne!(
            actual.identity().unwrap(),
            current.identity().unwrap(),
            "incidence revision must not mint a replacement immutable seal"
        );
        current.membership_content = ContentHash::of(b"changed member");
        assert!(!same_completion_metadata(&current, &actual));
        current = actual.clone();
        current.realization_incarnation = 8;
        assert!(!same_completion_metadata(&current, &actual));
        current = actual.clone();
        current.mappings[0].physical = ContentHash::of(b"another physical copy");
        assert!(!same_completion_metadata(&current, &actual));
        current = actual.clone();
        current.validity_epoch = 2;
        assert!(!same_completion_metadata(&current, &actual));
    }
    #[test]
    fn exact_dependency_mapping_retains_empty_premise_and_refuses_missing_extra_duplicate_or_rebound_copy()
     {
        let (closure, mappings) = closure();
        assert_eq!(mappings.len(), 2);
        exact_mappings(&closure, &mappings).unwrap();
        assert!(
            closure.views().values().all(|view| view.rows == 0),
            "empty dependencies are still closure authority"
        );
        for defect in [
            "missing",
            "extra",
            "duplicate",
            "physical_alias",
            "incarnation",
        ] {
            let mut changed = mappings.clone();
            match defect {
                "missing" => {
                    changed.pop();
                }
                "extra" => {
                    changed.push(ContributorRealization {
                        view: ContentHash::of(b"unrequested view"),
                        logical: ContentHash::of(b"unrequested logical owner"),
                        physical: ContentHash::of(b"unrequested copy"),
                        incarnation: 1,
                    });
                    changed.sort_by_key(|mapping| (mapping.view, mapping.logical));
                }
                "duplicate" => {
                    changed.push(changed[0].clone());
                    changed.sort_by_key(|mapping| (mapping.view, mapping.logical));
                }
                "physical_alias" => {
                    changed[1].physical = changed[0].physical;
                }
                _ => {
                    changed[0].incarnation = 0;
                }
            }
            assert!(
                exact_mappings(&closure, &changed).is_err(),
                "exact closure rejects {defect}"
            );
        }
        let root = closure
            .views()
            .values()
            .find(|view| {
                view.contributions
                    .iter()
                    .any(|logical| !closure.contributions()[logical].spec.inputs.is_empty())
            })
            .unwrap();
        let before = super::super::realization_id(root, &mappings).unwrap();
        let mut alternate = mappings.clone();
        alternate[0].physical = ContentHash::of(b"another valid copy");
        exact_mappings(&closure, &alternate).unwrap();
        assert_ne!(
            before,
            super::super::realization_id(root, &alternate).unwrap(),
            "two exact valid copies have different physical namespaces"
        );
    }
    #[test]
    fn equivalent_copies_are_selected_per_view_instead_of_collapsing_the_closure() {
        let mut shared = contribution("shared", vec![]);
        shared.spec.outputs.insert("releases".into());
        shared
            .outputs
            .insert("releases".into(), shared.outputs["packages"].clone());
        let logical = shared.identity().unwrap();
        let views = ["packages", "releases"]
            .map(|relation| CompletedView::new(relation.into(), [logical].into(), 0).unwrap());
        let mut traversal = RecoveryTraversal::for_views(views.to_vec()).unwrap();
        while let Some(view) = traversal.next_view().unwrap() {
            traversal
                .include_view(
                    &view,
                    view.clone(),
                    vec![shared.clone()],
                    |_| unreachable!(),
                )
                .unwrap();
        }
        let closure = traversal.finish().unwrap();
        let mut mappings = views
            .iter()
            .enumerate()
            .map(|(index, view)| ContributorRealization {
                view: view.identity,
                logical,
                physical: ContentHash::of(format!("copy-{index}").as_bytes()),
                incarnation: 1,
            })
            .collect::<Vec<_>>();
        mappings.sort_by_key(|mapping| (mapping.view, mapping.logical));
        exact_mappings(&closure, &mappings).unwrap();
        assert_ne!(
            mappings[0].physical, mappings[1].physical,
            "each selected view retains its exact chosen copy"
        );
        mappings[1].physical = mappings[0].physical;
        exact_mappings(&closure, &mappings).unwrap();
        mappings[1].incarnation += 1;
        assert!(
            exact_mappings(&closure, &mappings).is_err(),
            "one physical record cannot claim two current incarnations"
        );
        mappings[1].incarnation = 1;
        mappings[1].view = mappings[0].view;
        assert!(
            exact_mappings(&closure, &mappings).is_err(),
            "duplicate incidence is never an alternate choice"
        );
    }
}

/// Only alias rows owned by the supplied completed contributor set are closure
/// authority. Global aliases with the same source are intentionally never queried.
pub(crate) async fn audit_aliases(
    reader: &NativeReader<()>,
    closure: &InspectedClosure,
    budget: &ResourceBudget,
) -> Result<crate::ordered_rows::PreparedRows, ModelError> {
    use crate::ordered_rows::SortedRows;
    use d::{
        KeySink,
        completion::{Completion, complete},
    };
    let mut seeds = SortedRows::with_budget(budget)?;
    let mut aliases = SortedRows::with_budget(budget)?;
    for mapping in &closure.mappings {
        let owner = RecordId::new("compiler_contribution", mapping.physical.hex());
        let mut variables = Variables::new();
        variables.insert("owner", owner.clone());
        let mut members=reader.stream_prepared(crate::prepared::PreparedQuery::new(variables,vec![],
            vec!["SELECT node AS id FROM compiler_membership WITH INDEX contribution_rows WHERE contribution=$owner".into()])?)?;
        let result = async {
            while let Some(row) = members.next().await? {
                seeds.push(row)?;
            }
            Ok::<_, ModelError>(())
        }
        .await;
        let mut completion = Completion::default();
        completion.step(
            "installed seed membership drainage",
            members.drain_transport().await,
        );
        complete(result, completion)?;
        let mut variables = Variables::new();
        variables.insert("owner", owner.clone());
        let mut holds = reader.stream_prepared(crate::prepared::PreparedQuery::new(
            variables,
            vec![],
            vec![
                "SELECT object AS id FROM native_hold WITH INDEX owner_holds WHERE owner=$owner"
                    .into(),
            ],
        )?)?;
        let result = async {
            while let Some(pointer) = holds.next().await? {
                let Some(Value::RecordId(id)) = pointer.as_object().and_then(|row| row.get("id"))
                else {
                    return Err(ModelError::Schema("installed held alias pointer"));
                };
                if id.table.as_str() != "compiler_alias" {
                    continue;
                }
                let expected = crate::control::inspect_expected_read(reader, id.clone(), budget)
                    .await?
                    .ok_or(ModelError::Conflict("installed held alias missing"))?;
                let object = expected
                    .row()
                    .as_object()
                    .ok_or(ModelError::Schema("installed alias object"))?;
                let Some(Value::RecordId(source)) = object.get("source") else {
                    return Err(ModelError::Schema("installed alias source"));
                };
                let Some(Value::RecordId(target)) = object.get("target") else {
                    return Err(ModelError::Schema("installed alias target"));
                };
                if source.table.as_str() != "entity" || target.table.as_str() != "entity" {
                    return Err(ModelError::Conflict("installed alias entity family"));
                }
                let mut sink = KeySink::new("native-place-alias/v1");
                sink.part(
                    b"source",
                    &serde_json::to_vec(source).map_err(ModelError::codec)?,
                );
                sink.part(
                    b"target",
                    &serde_json::to_vec(target).map_err(ModelError::codec)?,
                );
                let mut canonical = surrealdb::types::Object::new();
                canonical.insert("id", RecordId::new("compiler_alias", sink.finish().hex()));
                canonical.insert("source", source.clone());
                canonical.insert("target", target.clone());
                if expected.row() != &Value::Object(canonical) {
                    return Err(ModelError::Conflict("installed exact alias envelope"));
                }
                crate::completed_state::require_hold(reader, &id, source).await?;
                let backing = crate::control::inspect_expected_read(reader, target.clone(), budget)
                    .await?
                    .ok_or(ModelError::Conflict("installed alias target missing"))?;
                crate::completed_state::require_hold(reader, &owner, target).await?;
                let (endpoints, original, _decode) = audit_backing(backing.row(), budget)?;
                if original.is_some() {
                    return Err(ModelError::Conflict(
                        "installed entity alias has original metadata",
                    ));
                }
                for window in endpoints.chunks(crate::loader::NATIVE_WINDOW_ROWS) {
                    let expected = window
                        .iter()
                        .map(|row| {
                            let Some(Value::RecordId(id)) =
                                row.as_object().and_then(|row| row.get("id"))
                            else {
                                return Err(ModelError::Schema(
                                    "installed alias endpoint identity",
                                ));
                            };
                            Ok((id.clone(), row.clone()))
                        })
                        .collect::<Result<BTreeMap<_, _>, ModelError>>()?;
                    audit_endpoint_window(reader, &owner, &expected, budget).await?;
                }
                aliases.push(expected.row().clone())?;
            }
            Ok::<_, ModelError>(())
        }
        .await;
        let mut completion = Completion::default();
        completion.step(
            "installed alias hold drainage",
            holds.drain_transport().await,
        );
        complete(result, completion)?;
    }
    let aliases = aliases.finish()?.into_prepared();
    audit_alias_reachability(seeds.finish()?.into_prepared(), &aliases, budget)?;
    Ok(aliases)
}
fn audit_alias_reachability(
    mut reached: crate::ordered_rows::PreparedRows,
    aliases: &crate::ordered_rows::PreparedRows,
    budget: &ResourceBudget,
) -> Result<(), ModelError> {
    use crate::ordered_rows::SortedRows;
    fn pointer(id: RecordId) -> Value {
        let mut row = surrealdb::types::Object::new();
        row.insert("id", id);
        Value::Object(row)
    }
    loop {
        let index = reached.point_index(budget)?;
        let mut membership = index.cursor(budget)?;
        let mut next = SortedRows::with_budget(budget)?;
        let mut cursor = reached.cursor_with_budget(budget)?;
        while let Some(row) = cursor.next_row()? {
            next.push(row)?;
        }
        let mut aliases = aliases.cursor_with_budget(budget)?;
        let mut changed = false;
        let mut unreachable = false;
        while let Some(row) = aliases.next_row()? {
            let object = row
                .as_object()
                .ok_or(ModelError::Schema("installed alias reachability row"))?;
            let Some(Value::RecordId(source)) = object.get("source") else {
                return Err(ModelError::Schema("installed alias reachability source"));
            };
            let Some(Value::RecordId(target)) = object.get("target") else {
                return Err(ModelError::Schema("installed alias reachability target"));
            };
            if membership.contains(source)? {
                if !membership.contains(target)? {
                    next.push(pointer(target.clone()))?;
                    changed = true;
                }
            } else {
                unreachable = true;
            }
        }
        if !changed {
            if unreachable {
                return Err(ModelError::Conflict(
                    "installed held alias outside selected reachable closure",
                ));
            }
            return Ok(());
        }
        reached = next.finish()?.into_prepared();
    }
}

#[cfg(test)]
mod alias_tests {
    use super::*;
    use crate::ordered_rows::{PreparedRows, SortedRows};
    fn rows(
        budget: &ResourceBudget,
        ids: &[&str],
        edges: &[(&str, &str)],
    ) -> (PreparedRows, PreparedRows) {
        let mut nodes = SortedRows::with_budget(budget).unwrap();
        for id in ids {
            let mut row = surrealdb::types::Object::new();
            row.insert("id", RecordId::new("entity", *id));
            nodes.push(Value::Object(row)).unwrap();
        }
        let mut aliases = SortedRows::with_budget(budget).unwrap();
        for (ordinal, (source, target)) in edges.iter().enumerate() {
            let mut row = surrealdb::types::Object::new();
            row.insert("id", RecordId::new("compiler_alias", ordinal as i64));
            row.insert("source", RecordId::new("entity", *source));
            row.insert("target", RecordId::new("entity", *target));
            aliases.push(Value::Object(row)).unwrap();
        }
        (
            nodes.finish().unwrap().into_prepared(),
            aliases.finish().unwrap().into_prepared(),
        )
    }
    #[test]
    fn owned_alias_audit_reaches_full_chains_and_cycles_but_refuses_disconnected_owned_suffix() {
        let budget = ResourceBudget::fixed(16 << 20).unwrap();
        for edges in [
            vec![("seed", "b"), ("b", "c"), ("c", "b")],
            vec![("seed", "c"), ("c", "b"), ("b", "seed")],
        ] {
            let (seeds, aliases) = rows(&budget, &["seed"], &edges);
            audit_alias_reachability(seeds, &aliases, &budget).unwrap();
        }
        for edges in [
            vec![("seed", "b"), ("foreign", "suffix")],
            vec![("foreign", "suffix"), ("suffix", "foreign")],
        ] {
            let (seeds, aliases) = rows(&budget, &["seed"], &edges);
            assert!(
                audit_alias_reachability(seeds, &aliases, &budget).is_err(),
                "unreachable owned aliases remain a refusal, including a disjoint cycle"
            );
        }
        assert_eq!(
            budget.reserved(),
            0,
            "all external runs/index reservations finish with their owned audit"
        );
    }
}

/// Compute all six normalized logical family roots from the already independently
/// audited exact closure. This never normalizes a physical row and then treats it as
/// a physical validator input; native codecs always run before logical lowering.
pub(crate) async fn logical_roots(
    reader: &NativeReader<()>,
    closure: &InspectedClosure,
    bindings: &[d::completed::CompletedBinding],
    aliases: &crate::ordered_rows::PreparedRows,
    budget: &ResourceBudget,
) -> Result<d::completed_identity::CompletedFamilyRoots, ModelError> {
    logical_roots_many(
        reader,
        std::slice::from_ref(closure),
        bindings,
        aliases,
        budget,
    )
    .await
}

pub(crate) async fn logical_roots_many(
    reader: &NativeReader<()>,
    closures: &[InspectedClosure],
    bindings: &[d::completed::CompletedBinding],
    aliases: &crate::ordered_rows::PreparedRows,
    budget: &ResourceBudget,
) -> Result<d::completed_identity::CompletedFamilyRoots, ModelError> {
    use crate::ordered_rows::SortedRows;
    use d::{
        completed_identity::CompletedFamilyHasher,
        completion::{Completion, complete},
        recovery_closure::COMPLETED_FAMILIES,
    };
    d::completed::binding_inventory_identity(bindings)?;
    let mut owner_charge =
        StateCharge::new(budget, "installed logical normalization owner descriptors");
    let mut owners = BTreeMap::new();
    for closure in closures {
        for mapping in &closure.mappings {
            let descriptor = &closure.logical.contributions()[&mapping.logical];
            let expected = &closure.contributors[&mapping.physical];
            if let Some(previous) = owners.get(expected.record()) {
                if previous != descriptor {
                    return Err(ModelError::Conflict(
                        "installed normalization physical owner descriptor collision",
                    ));
                }
            } else {
                // Sizing serializes a temporary copy. Admit it before allocation;
                // ExpectedRecord independently owns the original native-row charge.
                let _sizing = budget.reserve(
                    "installed normalization owner descriptor sizing",
                    crate::loader::native_bytes(expected.row())
                        .saturating_mul(32)
                        .saturating_add(65536),
                )?;
                let encoded = serde_json::to_vec(descriptor).map_err(ModelError::codec)?;
                owner_charge.grow(encoded.len().saturating_mul(16).saturating_add(1024))?;
                owners.insert(expected.record().clone(), descriptor.clone());
            }
        }
    }
    let mut families = COMPLETED_FAMILIES
        .map(|_| SortedRows::with_budget_and_row_bytes(budget, d::resources::MAX_ROW_BYTES))
        .into_iter()
        .collect::<Result<Vec<_>, _>>()?;
    let mut backing_ids = SortedRows::with_budget(budget)?;
    for closure in closures {
        for mapping in &closure.mappings {
            let expected = &closure.contributors[&mapping.physical];
            families[0].push(crate::completed_logical_rows::normalize(
                "compiler_contribution",
                expected.row(),
                &owners,
            )?)?;
            let mut variables = Variables::new();
            variables.insert("owner", expected.record().clone());
            let mut rows=reader.stream_prepared(crate::prepared::PreparedQuery::new(variables,vec![],vec!["SELECT * FROM compiler_membership WITH INDEX contribution_rows WHERE contribution=$owner".into()])?)?;
            let result = async {
                while let Some(row) = rows.next().await? {
                    validate_state_row("compiler_membership", &row)?;
                    families[1].push(crate::completed_logical_rows::normalize(
                        "compiler_membership",
                        &row,
                        &owners,
                    )?)?;
                    let Some(Value::RecordId(node)) =
                        row.as_object().and_then(|row| row.get("node"))
                    else {
                        return Err(ModelError::Schema("logical backing member node"));
                    };
                    if node.table.as_str() == "compiler_record" {
                        let mut pointer = surrealdb::types::Object::new();
                        pointer.insert("id", node.clone());
                        backing_ids.push(Value::Object(pointer))?;
                    }
                }
                Ok::<_, ModelError>(())
            }
            .await;
            let mut completion = Completion::default();
            completion.step(
                "installed logical membership drainage",
                rows.drain_transport().await,
            );
            complete(result, completion)?;
        }
    }
    for closure in closures {
        for view in closure.logical.views().values() {
            let mut variables = Variables::new();
            variables.insert("view", RecordId::new("compiler_view", view.identity.hex()));
            let _read =
                budget.reserve("logical declared view read", std::mem::size_of::<Value>())?;
            let rows: Vec<Value> = reader
                .query_native("SELECT * FROM $view", variables)
                .await?;
            let [row] = rows.as_slice() else {
                return Err(ModelError::Conflict("logical declared view cardinality"));
            };
            let _decode = budget.reserve(
                "logical declared view normalization",
                crate::loader::native_bytes(row)
                    .saturating_mul(32)
                    .saturating_add(65536),
            )?;
            validate_state_row("compiler_view", row)?;
            if descriptor::<CompletedView>(row)? != *view {
                return Err(ModelError::Conflict("logical declared view changed"));
            }
            families[2].push(crate::completed_logical_rows::normalize(
                "compiler_view",
                row,
                &owners,
            )?)?;
        }
    }
    let mut backing_ids = backing_ids.finish()?;
    while let Some(pointer) = backing_ids.next_row()? {
        let Some(Value::RecordId(id)) = pointer.as_object().and_then(|row| row.get("id")) else {
            return Err(ModelError::Schema("logical backing identity"));
        };
        let actual = crate::control::inspect_expected_read(reader, id.clone(), budget)
            .await?
            .ok_or(ModelError::Conflict("logical backing missing"))?;
        validate_state_row("compiler_record", actual.row())?;
        families[3].push(crate::completed_logical_rows::normalize(
            "compiler_record",
            actual.row(),
            &owners,
        )?)?;
    }
    for binding in bindings {
        if !closures.iter().any(|closure| {
            closure.logical.views().get(&binding.view.identity) == Some(&binding.view)
        }) {
            return Err(ModelError::Conflict(
                "logical binding outside exact dependency closure",
            ));
        }
        let _decode = budget.reserve(
            "logical binding normalization",
            serde_json::to_vec(binding)
                .map_err(ModelError::codec)?
                .len()
                .saturating_mul(16)
                .saturating_add(65536),
        )?;
        let mut row = surrealdb::types::Object::new();
        row.insert(
            "descriptor",
            surrealdb::types::Bytes::from(serde_json::to_vec(binding).map_err(ModelError::codec)?),
        );
        families[4].push(crate::completed_logical_rows::normalize(
            "compiler_binding",
            &Value::Object(row),
            &owners,
        )?)?;
    }
    let mut aliases = aliases.cursor_with_budget(budget)?;
    while let Some(row) = aliases.next_row()? {
        families[5].push(crate::completed_logical_rows::normalize(
            "compiler_alias",
            &row,
            &owners,
        )?)?;
    }
    let mut roots = Vec::with_capacity(6);
    for (family, rows) in COMPLETED_FAMILIES.into_iter().zip(families) {
        let mut hasher = CompletedFamilyHasher::new(family)?;
        let mut rows = rows.finish()?;
        while let Some(row) = rows.next_row()? {
            hasher.row(&serde_json::to_vec(&row).map_err(ModelError::codec)?)?;
        }
        roots.push(hasher.finish());
    }
    d::completed_identity::CompletedFamilyRoots::new(
        roots
            .try_into()
            .map_err(|_| ModelError::Schema("logical completed family cardinality"))?,
    )
}
