//! Native physical completion earning. All public inputs are data, independently audited
//! against exact held physical copies; semantic admission is a separate opaque transition.
use super::*;
use crate::{
    installed_view::{self, ContributorRealization, InstalledViewCompletion},
    ordered_rows::PreparedRows,
};
use d::{
    charged::StateCharge,
    completion::{Completion, complete},
    resources::ResourceBudget,
};
use std::sync::Arc;
use surrealdb::{Surreal, engine::remote::grpc::Client, types::Variables};

pub struct ViewSelection {
    pub view: d::completed::CompletedView,
    pub mappings: Vec<ContributorRealization>,
}
pub(crate) async fn revision(
    reader: &crate::NativeReader<()>,
    record: &RecordId,
) -> Result<i64, ModelError> {
    let mut variables = Variables::new();
    variables.insert("record", record.clone());
    let value: Option<i64> = reader
        .query_native(
            "RETURN (SELECT VALUE revision FROM ONLY type::record('native_predicate',[$record]))",
            variables,
        )
        .await?;
    Ok(value.unwrap_or(0))
}
pub(crate) fn member(realization: ContentHash, row: &Value) -> Result<Value, ModelError> {
    let object = row
        .as_object()
        .ok_or(ModelError::Schema("completed root member source"))?;
    let Some(Value::String(relation)) = object.get("relation") else {
        return Err(ModelError::Schema("completed root member relation"));
    };
    let Some(Value::String(key)) = object.get("semantic_key") else {
        return Err(ModelError::Schema("completed root member key"));
    };
    let key = crate::completed_rows::decode_nominal_key(key)?;
    let mut result = Object::new();
    result.insert("id", installed_view::member_id(realization, relation, &key));
    result.insert("selector", realization.hex());
    for field in ["relation", "semantic_key", "node", "content"] {
        result.insert(
            field,
            object
                .get(field)
                .cloned()
                .ok_or(ModelError::Schema("completed root member field"))?,
        );
    }
    Ok(Value::Object(result))
}
/// Fold only the exact canonical selector union. The rows are temporary ordered
/// validation projections; no independently retained per-view member namespace exists.
pub(crate) fn selected_membership_content(
    members: &PreparedRows,
    rows: u64,
    budget: &ResourceBudget,
) -> Result<ContentHash, ModelError> {
    let mut sink = KeySink::new("native-selected-membership/v3");
    let count = PreparedRows::fold_union(
        std::slice::from_ref(members),
        budget,
        || Ok(()),
        |_, payload| {
            sink.part(b"member", payload);
            Ok(())
        },
    )?;
    if count as u64 != rows {
        return Err(ModelError::Conflict(
            "compound selector distinct membership count",
        ));
    }
    sink.part(b"rows", &(count as u64).to_le_bytes());
    Ok(sink.finish())
}
/// Cancellation between actual pin acquisition and returned owner construction
/// retains that pin; only a terminal logical rejection may explicitly release it.
struct RetainedPinConstruction {
    pin: Arc<crate::control::ReaderPin>,
    confirmed: bool,
}
impl Drop for RetainedPinConstruction {
    fn drop(&mut self) {
        if !self.confirmed {
            self.pin.retain_unknown();
        }
    }
}
enum BoundaryMode<'a> {
    Frozen(&'a FrozenContentFence),
    Live(crate::control::live_view::LiveViewOwner),
}
fn boundary_writer<'a>(
    client: &'a Surreal<Client>,
    mode: &BoundaryMode<'_>,
    budget: &'a ResourceBudget,
) -> Result<crate::control::completed::WitnessWriter<'a>, ModelError> {
    match mode {
        BoundaryMode::Frozen(fence) => {
            crate::control::completed::witness_writer(client, fence, budget)
        }
        BoundaryMode::Live(owner) => {
            crate::control::live_view::witness_writer(client, owner, budget)
        }
    }
}
async fn hold_boundary_copies(
    client: &Surreal<Client>,
    mode: &BoundaryMode<'_>,
    root: &RecordId,
    expected: &[Arc<crate::control::ExpectedRecord>],
    budget: &ResourceBudget,
) -> Result<(), ModelError> {
    match mode {
        BoundaryMode::Frozen(fence) => {
            crate::control::completed::hold_selected_contributors(
                client, fence, root, expected, budget,
            )
            .await
        }
        BoundaryMode::Live(owner) => {
            crate::control::live_view::hold_contributors(client, owner, root, expected, budget)
                .await
        }
    }
}
async fn hold_boundary(
    client: &Surreal<Client>,
    mode: &BoundaryMode<'_>,
    data: &FrozenCompletion,
    expected: crate::control::ExpectedRecord,
    budget: &ResourceBudget,
) -> Result<crate::control::HeldRecord, ModelError> {
    match mode {
        BoundaryMode::Frozen(_) => {
            crate::control::hold_frozen_physical_expected(client, data, expected, budget).await
        }
        BoundaryMode::Live(owner) => {
            crate::control::hold_expected(
                client,
                owner.attempt(),
                RecordId::new("native_attempt", owner.attempt().hex()),
                expected,
            )
            .await
        }
    }
}
pub async fn complete_frozen_content(
    client: Arc<Surreal<Client>>,
    fence: &FrozenContentFence,
    bindings: &[d::completed::CompletedBinding],
    selections: &[ViewSelection],
    budget: &ResourceBudget,
) -> Result<NativeCompletedState, ModelError> {
    let audit = installed_view::InstallationAudit::new(client.clone(), budget).await?;
    let result =
        complete_frozen_content_scoped(client, fence, bindings, selections, budget, &audit).await;
    let mut terminal = Completion::default();
    terminal.step("frozen boundary owned audit close", audit.close().await);
    complete(result, terminal)
}
pub async fn complete_frozen_content_scoped(
    client: Arc<Surreal<Client>>,
    fence: &FrozenContentFence,
    bindings: &[d::completed::CompletedBinding],
    selections: &[ViewSelection],
    budget: &ResourceBudget,
    audit: &installed_view::InstallationAudit,
) -> Result<NativeCompletedState, ModelError> {
    complete_boundary_content(
        client,
        fence,
        bindings,
        selections,
        budget,
        BoundaryMode::Frozen(fence),
        audit,
    )
    .await
}
/// Separate retained lifetime, with operation-local preparation reused when supplied.
pub async fn complete_retained_content(
    client: Arc<Surreal<Client>>,
    attempt: ContentHash,
    bindings: &[d::completed::CompletedBinding],
    selections: &[ViewSelection],
    budget: &ResourceBudget,
) -> Result<NativeRetainedState, ModelError> {
    let audit = installed_view::InstallationAudit::new(client.clone(), budget).await?;
    let result =
        complete_retained_content_scoped(client, attempt, bindings, selections, budget, &audit)
            .await;
    let mut terminal = Completion::default();
    terminal.step("retained boundary owned audit close", audit.close().await);
    complete(result, terminal)
}
pub async fn complete_retained_content_scoped(
    client: Arc<Surreal<Client>>,
    attempt: ContentHash,
    bindings: &[d::completed::CompletedBinding],
    selections: &[ViewSelection],
    budget: &ResourceBudget,
    audit: &installed_view::InstallationAudit,
) -> Result<NativeRetainedState, ModelError> {
    let reader = crate::NativeReader::private(client.clone()).with_budget(budget);
    let result = async {
        let owner = crate::control::live_view::capture(&reader, attempt, budget).await?;
        // This is captured source metadata, not an assertion that the entire
        // still-open attempt is frozen. The boundary has its own inventory CAS.
        let fence = FrozenContentFence {
            generation: owner.generation(),
            attempt,
            attempt_epoch: owner.epoch(),
            completed_revision: owner.revision(),
            validity_epoch: owner.validity_epoch(),
            bindings: d::completed::binding_inventory_identity(bindings)?,
        };
        let inner = complete_boundary_content(
            client.clone(),
            &fence,
            bindings,
            selections,
            budget,
            BoundaryMode::Live(owner),
            audit,
        )
        .await?;
        let pin = crate::control::ReaderPin::acquire(client.clone(), &[]).await?;
        let mut preservation = RetainedPinConstruction {
            pin: pin.clone(),
            confirmed: false,
        };
        let transferred = async {
            let expected =
                crate::control::inspect_expected_read(&reader, inner.completion.record()?, budget)
                    .await?
                    .ok_or(ModelError::Conflict(
                        "retained boundary disappeared before pin transfer",
                    ))?;
            if expected.incarnation() != inner.incarnation()
                || expected.row() != inner.completion.value(true)?
            {
                return Err(ModelError::Conflict(
                    "retained boundary changed before pin transfer",
                ));
            }
            crate::control::live_view::transfer_expected(&pin, &expected, budget).await
        }
        .await;
        if let Err(error) = transferred {
            if !error.permits_storage_cleanup() {
                return Err(error);
            }
            let released = pin.release().await;
            if released.is_ok() {
                preservation.confirmed = true;
            }
            let mut terminal = Completion::default();
            terminal.step(
                "rejected retained boundary construction pin release",
                released,
            );
            return complete(Err(error), terminal);
        }
        let retained = NativeRetainedState {
            inner,
            pin,
            closed: false,
        };
        preservation.confirmed = true;
        Ok(retained)
    }
    .await;
    let mut terminal = Completion::default();
    terminal.step(
        "retained boundary source capture close",
        reader.close().await,
    );
    complete(result, terminal)
}
async fn complete_boundary_content(
    client: Arc<Surreal<Client>>,
    fence: &FrozenContentFence,
    bindings: &[d::completed::CompletedBinding],
    selections: &[ViewSelection],
    budget: &ResourceBudget,
    mode: BoundaryMode<'_>,
    audit: &installed_view::InstallationAudit,
) -> Result<NativeCompletedState, ModelError> {
    let reader = crate::NativeReader::private(client.clone()).with_budget(budget);
    let result = async {
        audit.ensure_context(&reader, budget).await?;
        if d::completed::binding_inventory_identity(bindings)? != fence.bindings()
            || selections
                .windows(2)
                .any(|pair| pair[0].view.identity >= pair[1].view.identity)
        {
            return Err(ModelError::Conflict("compound binding/selector inventory"));
        }
        if matches!(mode, BoundaryMode::Frozen(_)) {
            audit_attempt_bindings(&reader, fence, bindings, budget).await?;
        }
        let mut charge = StateCharge::new(budget, "compound exact selected physical audits");
        charge.grow(selections.len().saturating_mul(8192))?;
        let mut closures = Vec::with_capacity(selections.len());
        let mut references = Vec::with_capacity(selections.len());
        let mut copies = std::collections::BTreeMap::<ContentHash, ContributorCopy>::new();
        let mut alias_runs = Vec::new();
        let assurance_phase = crate::phase::Phase::begin("native_compound_selection_assurance");
        let mut inspections = installed_view::audit::ClosureInspectionScope::new(audit, budget)?;
        inspections
            .prepare_views(
                &reader,
                selections
                    .iter()
                    .map(|selection| (&selection.view, selection.mappings.as_slice())),
                audit,
                budget,
            )
            .await?;
        for selection in selections {
            let closure = installed_view::audit::inspect_closure_scoped(
                &reader,
                &selection.view,
                &selection.mappings,
                budget,
                audit,
                &mut inspections,
            )
            .await?;
            let aliases = installed_view::audit::assure_boundary_closure(
                &reader,
                &closure,
                budget,
                audit,
                &mut inspections,
            )
            .await?;
            if bindings.is_empty()
                || bindings
                    .iter()
                    .any(|binding| binding.view.identity == selection.view.identity)
            {
                alias_runs.push(aliases);
            }
            for (physical, expected) in &closure.contributors {
                let descriptor = closure.descriptor_for_copy(physical)?;
                if let Some(previous) = copies.get(physical) {
                    if previous.incarnation != expected.incarnation()
                        || previous.descriptor != *descriptor
                    {
                        return Err(ModelError::Conflict(
                            "compound selected physical copy collision",
                        ));
                    }
                } else {
                    charge.grow(
                        std::mem::size_of::<(ContentHash, ContributorCopy)>()
                            .saturating_add(32)
                            .saturating_add(installed_view::audit::contribution_heap(descriptor)),
                    )?;
                    copies.insert(
                        *physical,
                        ContributorCopy {
                            physical: *physical,
                            incarnation: expected.incarnation(),
                            descriptor: descriptor.clone(),
                        },
                    );
                }
            }
            let (_members, membership_content) =
                installed_view::audit::selected_prepared_members_with_content(
                    &closure,
                    &selection.view,
                    budget,
                    audit,
                )
                .await?;
            references.push(InstalledViewRef {
                logical_view: selection.view.identity,
                selector: InstalledViewCompletion {
                    version: 2,
                    parent_completion: ContentHash([0; 32]),
                    generation: fence.generation(),
                    validity_epoch: fence.validity_epoch(),
                    view: selection.view.clone(),
                    mappings: selection.mappings.clone(),
                    member_codec: installed_view::MEMBER_CODEC,
                    membership_rows: selection.view.rows,
                    membership_content,
                },
            });
            closures.push(closure);
        }
        assurance_phase.finish(crate::phase::Terminal::Passed);
        let all_views = closures
            .iter()
            .flat_map(|closure| closure.logical.views().keys().copied())
            .collect::<std::collections::BTreeSet<_>>();
        if !all_views
            .iter()
            .copied()
            .eq(selections.iter().map(|selection| selection.view.identity))
        {
            return Err(ModelError::Conflict(
                "compound inventory must include exact dependency views",
            ));
        }
        let aliases = PreparedRows::union_prepared(&alias_runs, budget, || Ok(()))?;
        let semantic_closures = closures
            .iter()
            .zip(selections)
            .filter_map(|(closure, selection)| {
                (bindings.is_empty()
                    || bindings
                        .iter()
                        .any(|binding| binding.view.identity == selection.view.identity))
                .then_some(closure)
            })
            .collect::<Vec<_>>();
        let family_phase = crate::phase::Phase::begin("native_compound_family_union");
        let roots = installed_view::audit::logical_roots_many_scoped(
            &reader,
            &semantic_closures,
            bindings,
            &aliases,
            budget,
            audit,
        )
        .await;
        family_phase.finish_result(&roots);
        let roots = roots?;
        let mut selector_hasher = RealizationHasher::default();
        for reference in &references {
            selector_hasher.row(reference)?;
        }
        let mut contributor_hasher = ContributorHasher::default();
        for copy in copies.values() {
            contributor_hasher.row(copy)?;
        }
        let data = FrozenCompletion {
            version: VERSION,
            generation: fence.generation(),
            attempt: fence.attempt(),
            attempt_epoch: fence.attempt_epoch(),
            completed_revision: fence.completed_revision(),
            validity_epoch: fence.validity_epoch(),
            completed_state: roots.identity()?,
            roots,
            bindings: fence.bindings(),
            realization_inventory: selector_hasher.finish(),
            contributor_inventory: contributor_hasher.finish(),
        };
        let identity = data.identity()?;
        for reference in &mut references {
            reference.selector.parent_completion = identity;
        }
        let existing =
            crate::control::inspect_expected_read(&reader, data.record()?, budget).await?;
        if let Some(existing) = existing
            .as_ref()
            .filter(|expected| expected.row() == data.value(true).unwrap_or(Value::None))
        {
            let inventory = {
                let phase = crate::phase::Phase::begin("native_compound_inventory_receiving");
                let result = inspect_inventory(&reader, &data, budget).await;
                phase.finish_result(&result);
                result?
            };
            if inventory.references() != references.as_slice() {
                return Err(ModelError::Conflict(
                    "compound replay exact selector inventory",
                ));
            }
            let expected = crate::control::inspect_expected_read(&reader, data.record()?, budget)
                .await?
                .ok_or(ModelError::Conflict("compound replay disappeared"))?;
            if expected.incarnation() != existing.incarnation() {
                return Err(ModelError::Conflict("compound replay ABA"));
            }
            let held = hold_boundary(&client, &mode, &data, expected, budget).await?;
            return Ok(NativeCompletedState {
                completion: data,
                held,
                inventory,
                _charge: charge,
            });
        }
        let parent = data.record()?;
        let write_phase = crate::phase::Phase::begin("native_compound_child_installation");
        let mut writer = boundary_writer(&client, &mode, budget)?;
        writer
            .push(&data, || {
                Ok((
                    data.value(false)?,
                    vec![(
                        RecordId::new("native_attempt", fence.attempt().hex()),
                        parent.clone(),
                    )],
                ))
            })
            .await?;
        writer.flush().await?;
        {
            let _handles = budget.reserve(
                "compound exact contributor hold handles",
                copies
                    .len()
                    .saturating_mul(std::mem::size_of::<Arc<crate::control::ExpectedRecord>>())
                    .saturating_add(4096),
            )?;
            let expected = copies
                .keys()
                .map(|physical| inspections.expected_copy(physical).map(Arc::clone))
                .collect::<Result<Vec<_>, _>>()?;
            hold_boundary_copies(&client, &mode, &parent, &expected, budget).await?;
        }
        for copy in copies.values() {
            writer
                .push(copy, || {
                    Ok((
                        copy.value(identity)?,
                        vec![(parent.clone(), copy.record(identity))],
                    ))
                })
                .await?;
        }
        writer.flush().await?;
        for reference in &references {
            writer
                .push(reference, || {
                    Ok((
                        reference.value(identity)?,
                        vec![
                            (parent.clone(), reference.record(identity)),
                            (
                                parent.clone(),
                                RecordId::new("compiler_view", reference.logical_view.hex()),
                            ),
                        ],
                    ))
                })
                .await?;
        }
        for binding in bindings {
            if selections
                .binary_search_by_key(&binding.view.identity, |selection| selection.view.identity)
                .is_err()
            {
                return Err(ModelError::Conflict(
                    "compound binding selected view absent",
                ));
            }
            writer
                .push(binding, || {
                    let address = crate::binding_descriptor::record(binding)?;
                    Ok((
                        crate::binding_descriptor::value(binding)?,
                        vec![
                            (parent.clone(), address.clone()),
                            (
                                address,
                                RecordId::new("compiler_view", binding.view.identity.hex()),
                            ),
                        ],
                    ))
                })
                .await?;
        }
        writer.flush().await?;
        drop(writer);
        write_phase.finish(crate::phase::Terminal::Passed);
        let expected = crate::control::inspect_expected_read(&reader, data.record()?, budget)
            .await?
            .ok_or(ModelError::Conflict(
                "compound completion disappeared before finality",
            ))?;
        let before = revision(&reader, &data.record()?).await?;
        // Full child inventories and all exact selected-copy incidences precede the
        // single original-parent revision CAS; no prefix grants completion.
        let inventory = {
            let phase = crate::phase::Phase::begin("native_compound_inventory_receiving");
            let result = inspect_inventory(&reader, &data, budget).await;
            phase.finish_result(&result);
            result?
        };
        match &mode {
            BoundaryMode::Frozen(_) => {
                crate::control::completed::finish_physical_completion(
                    &client, &data, &expected, before, budget,
                )
                .await?
            }
            BoundaryMode::Live(owner) => {
                crate::control::live_view::finish_boundary(
                    &client, owner, &data, &expected, before, budget,
                )
                .await?
            }
        }
        let expected = crate::control::inspect_expected_read(&reader, data.record()?, budget)
            .await?
            .ok_or(ModelError::Conflict("compound completed parent absent"))?;
        let held = hold_boundary(&client, &mode, &data, expected, budget).await?;
        Ok(NativeCompletedState {
            completion: data,
            held,
            inventory,
            _charge: charge,
        })
    }
    .await;
    if result
        .as_ref()
        .err()
        .is_some_and(|error| !error.permits_storage_cleanup())
    {
        audit.retain_unknown();
    }
    let mut terminal = Completion::default();
    terminal.step("compound completion reader close", reader.close().await);
    complete(result, terminal)
}

async fn audit_attempt_bindings(
    reader: &crate::NativeReader<()>,
    fence: &FrozenContentFence,
    bindings: &[d::completed::CompletedBinding],
    budget: &ResourceBudget,
) -> Result<(), ModelError> {
    let mut variables = Variables::new();
    variables.insert(
        "attempt",
        RecordId::new("native_attempt", fence.attempt().hex()),
    );
    let mut stream = reader.stream_prepared(crate::prepared::PreparedQuery::new(
        variables,
        vec![],
        vec![
            "SELECT * FROM compiler_binding WITH INDEX binding_attempt WHERE attempt=$attempt"
                .into(),
        ],
    )?)?;
    let outcome = async {
        let mut charge = StateCharge::new(budget, "frozen actual binding inventory audit");
        let mut actual = std::collections::BTreeMap::new();
        while let Some(row) = stream.next().await? {
            charge.grow(
                crate::loader::native_bytes(&row)
                    .saturating_mul(32)
                    .saturating_add(2048),
            )?;
            crate::completed_rows::validate_state_row("compiler_binding", &row)?;
            let Some(Value::Bytes(bytes)) = row.as_object().and_then(|row| row.get("descriptor"))
            else {
                return Err(ModelError::Schema("frozen actual binding descriptor"));
            };
            let binding: d::completed::CompletedBinding =
                serde_json::from_slice(bytes).map_err(ModelError::codec)?;
            if actual.insert(binding.key(), binding).is_some() {
                return Err(ModelError::Conflict("frozen duplicate actual binding slot"));
            }
        }
        if actual.len() != bindings.len()
            || bindings
                .iter()
                .any(|binding| actual.get(&binding.key()) != Some(binding))
            || d::completed::binding_inventory_identity(&actual.into_values().collect::<Vec<_>>())?
                != fence.bindings()
        {
            return Err(ModelError::Conflict(
                "frozen complete exact actual binding EOF",
            ));
        }
        Ok(())
    }
    .await;
    let mut completion = Completion::default();
    completion.step(
        "frozen binding inventory stream drain",
        stream.drain_transport().await,
    );
    complete(outcome, completion)
}

#[cfg(test)]
mod retained_pin_controls {
    use super::*;
    #[tokio::test]
    async fn abandoned_construction_preserves_actual_pin_while_confirmed_transfer_does_not() {
        let pin = crate::control::ReaderPin::test_detached(false);
        {
            let _construction = RetainedPinConstruction {
                pin: pin.clone(),
                confirmed: false,
            };
        }
        assert!(pin.test_uncertain());
        let transferred = crate::control::ReaderPin::test_detached(false);
        {
            let _construction = RetainedPinConstruction {
                pin: transferred.clone(),
                confirmed: true,
            };
        }
        assert!(!transferred.test_uncertain());
        transferred.retain_unknown();
    }
}
