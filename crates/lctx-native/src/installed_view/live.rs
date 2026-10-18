//! Exact selected access under operation-local earned contributors or a compound boundary.
use super::*;
use crate::control::live_view;
use d::{
    charged::StateCharge,
    completion::{Completion, complete},
    resources::ResourceBudget,
};
use std::sync::Arc;
use surrealdb::{
    Surreal,
    engine::remote::grpc::Client,
    types::{Object, Variables},
};
#[derive(Debug, Clone)]
pub struct InstalledMember {
    pub key: [u8; 16],
    pub node: RecordId,
    pub content: ContentHash,
    pub locator: Value,
}
impl PartialEq for InstalledMember {
    fn eq(&self, other: &Self) -> bool {
        self.key == other.key && self.node == other.node && self.content == other.content
    }
}
impl Eq for InstalledMember {}
pub struct SelectedMembers {
    pub(super) rows: Vec<InstalledMember>,
    pub(super) _charge: StateCharge,
}
impl SelectedMembers {
    pub fn rows(&self) -> &[InstalledMember] {
        &self.rows
    }
}
struct InstalledMemberLifetime {
    owner: Option<Arc<HeldInstalledView>>,
    reader: crate::NativeReader<()>,
    confirmed: bool,
}
impl Drop for InstalledMemberLifetime {
    fn drop(&mut self) {
        if !self.confirmed {
            self.reader.retain_unknown();
            if let Some(owner) = &self.owner {
                owner.retain_unknown();
            }
        }
    }
}
impl HeldInstalledView {
    /// A live consumer may borrow the exact immutable premises already earned
    /// beneath an internal source pin or this operation's complete receiving pin.
    pub async fn ensure_borrowed_current(
        &self,
        reader: &crate::NativeReader<()>,
        budget: &ResourceBudget,
    ) -> Result<(), ModelError> {
        self.require_pool(budget)?;
        if reader.holds_installed_premise(self, budget)? {
            return Ok(());
        }
        if let InstalledViewOwner::Earned { set } = &self.owner {
            return set.ensure_borrowed_current(reader, budget).await;
        }
        self.ensure_current(reader, budget).await
    }
    pub async fn ensure_current(
        &self,
        reader: &crate::NativeReader<()>,
        budget: &ResourceBudget,
    ) -> Result<(), ModelError> {
        self.require_pool(budget)?;
        if let InstalledViewOwner::Earned { set } = &self.owner {
            return set.ensure_current(reader, budget).await;
        }
        let selector = self.retained_completion()?;
        let expected = crate::control::inspect_expected_read(reader, selector.record()?, budget)
            .await?
            .ok_or(ModelError::Conflict("retained exact selector missing"))?;
        if expected.incarnation() != self.completion_incarnation()
            || expected.row() != self.expected_bytes()
        {
            return Err(ModelError::Conflict(
                "retained selector incarnation/full bytes changed",
            ));
        }
        let boundary = RecordId::new("native_completed_state", selector.parent_completion.hex());
        let actual = crate::control::inspect_expected_read(reader, boundary.clone(), budget)
            .await?
            .ok_or(ModelError::Conflict("retained compound boundary missing"))?;
        let data = crate::completed_state::FrozenCompletion::from_row(actual.row(), true)?;
        if data.generation != selector.generation
            || data.validity_epoch != selector.validity_epoch
            || data.identity()? != selector.parent_completion
        {
            return Err(ModelError::Conflict("retained compound boundary context"));
        }
        crate::completed_state::require_hold(reader, &boundary, &selector.record()?).await?;
        let reference = crate::completed_state::InstalledViewRef {
            logical_view: selector.view.identity,
            selector: selector.clone(),
        };
        // The opaque retained inventory was independently earned through complete
        // copy/selector/hold EOF. Exact immutable parent and selector checks above
        // preserve it; recapturing every contributor per selected view adds no authority.
        let inventory = match &self.owner {
            InstalledViewOwner::Earned { .. } => unreachable!(),
            InstalledViewOwner::Recovery { inventory, .. } => inventory.as_ref(),
            InstalledViewOwner::Physical { completed, .. } => completed.inventory(),
            InstalledViewOwner::Frozen { completed, .. } => completed.inventory(),
            InstalledViewOwner::Published { completed, .. } => completed.inventory(),
        };
        if inventory.view(selector.view.identity)? != &reference {
            return Err(ModelError::Conflict(
                "retained selector absent from protected complete inventory",
            ));
        }
        crate::completed_state::validate_selector_copies(&data, selector, inventory.copies())?;
        match &self.owner {
            InstalledViewOwner::Earned { .. } => unreachable!(),
            InstalledViewOwner::Recovery { pin, owner, .. } => {
                pin.ensure_live()?;
                crate::completed_state::require_hold(reader, owner, &boundary).await?;
            }
            InstalledViewOwner::Published { completed, .. } => {
                let proof = completed
                    .retained_reader()
                    .protected_publication_marker(budget)
                    .await?;
                if proof.row() != completed.marker() {
                    return Err(ModelError::Conflict("retained published marker changed"));
                }
            }
            InstalledViewOwner::Physical { completed, .. } => {
                crate::completed_state::require_hold(reader, completed.held.owner(), &boundary)
                    .await?;
                if data != *completed.completion()
                    || actual.incarnation() != completed.incarnation()
                {
                    return Err(ModelError::Conflict("retained physical boundary changed"));
                }
            }
            InstalledViewOwner::Frozen { completed, .. } => {
                crate::completed_state::require_hold(reader, completed.owner(), &boundary).await?;
                crate::completed_state::require_hold(
                    reader,
                    completed.admission_record(),
                    &boundary,
                )
                .await?;
                if data != *completed.completion()
                    || actual.incarnation() != completed.incarnation()
                {
                    return Err(ModelError::Conflict("retained admitted boundary changed"));
                }
            }
        }
        let installations: Vec<Object> = reader
            .query_native(
                "SELECT generation,validity_epoch FROM native_installation:current",
                Variables::new(),
            )
            .await?;
        let [installation] = installations.as_slice() else {
            return Err(ModelError::Conflict("retained installation cardinality"));
        };
        if installation.get("generation") != Some(&Value::String(selector.generation.hex()))
            || installation.get("validity_epoch")
                != Some(&Value::Number(surrealdb::types::Number::Int(
                    selector.validity_epoch,
                )))
        {
            return Err(ModelError::Conflict("retained installation changed"));
        }
        Ok(())
    }
    pub async fn members(
        self: &Arc<Self>,
        reader: &crate::NativeReader<()>,
        keys: &[[u8; 16]],
        budget: &ResourceBudget,
    ) -> Result<SelectedMembers, ModelError> {
        if keys.len() > crate::loader::NATIVE_WINDOW_ROWS
            || keys.windows(2).any(|pair| pair[0] >= pair[1])
        {
            return Err(ModelError::Conflict(
                "installed member demand order/duplicate/bound",
            ));
        }
        let setup = budget.reserve(
            "owned compact member demand",
            keys.len().saturating_mul(64).saturating_add(4096),
        )?;
        let owner = self.clone();
        let protected = reader.holds_installed_premise(self, budget)?;
        let observed_reader = reader.clone();
        let reader = reader.clone();
        let driver_budget = budget.clone();
        let keys = keys.to_vec();
        // The native driver owns the cap, reader, requested keys and reservations until
        // every accepted response is terminal, even when its caller drops delivery.
        let mut stream =
            crate::reader::NativeRows::owned(budget.clone(), move |sender| async move {
                let _setup = setup;
                let mut lifetime = InstalledMemberLifetime {
                    owner: (!protected).then(|| owner.clone()),
                    reader: reader.clone(),
                    confirmed: false,
                };
                let result = owner.members_inner(&reader, &keys, &driver_budget).await;
                lifetime.confirmed = result
                    .as_ref()
                    .err()
                    .is_none_or(ModelError::permits_storage_cleanup);
                let selected = result?;
                for member in selected.rows() {
                    let mut value = Object::new();
                    value.insert("semantic_key", hex::encode(member.key));
                    value.insert("node", member.node.clone());
                    value.insert("content", member.content.hex());
                    value.insert("locator", member.locator.clone());
                    if !sender.send(Value::Object(value)).await? {
                        break;
                    }
                }
                Ok(())
            })?;
        let result = async {
            let mut charge = StateCharge::new(budget, "delivered compact installed members");
            let mut rows = Vec::new();
            while let Some(value) = stream.next().await? {
                let object = value
                    .as_object()
                    .ok_or(ModelError::Schema("owned compact member delivery"))?;
                let Some(Value::String(key)) = object.get("semantic_key") else {
                    return Err(ModelError::Schema("owned compact member key"));
                };
                let Some(Value::RecordId(node)) = object.get("node") else {
                    return Err(ModelError::Schema("owned compact member node"));
                };
                let Some(Value::String(content)) = object.get("content") else {
                    return Err(ModelError::Schema("owned compact member content"));
                };
                let locator = object
                    .get("locator")
                    .ok_or(ModelError::Schema("owned member physical locator"))?;
                crate::compiler_blocks::validate_locator(locator)?;
                charge.grow(crate::loader::native_bytes(locator).saturating_mul(8))?;
                charge.grow(
                    crate::selection::node_bytes(node)
                        .saturating_mul(2)
                        .saturating_add(std::mem::size_of::<InstalledMember>())
                        .saturating_add(128),
                )?;
                rows.push(InstalledMember {
                    key: crate::completed_rows::decode_nominal_key(key)?,
                    node: node.clone(),
                    locator: locator.clone(),
                    content: ContentHash(
                        hex::decode(content)
                            .map_err(ModelError::codec)?
                            .try_into()
                            .map_err(|_| ModelError::Schema("owned compact content width"))?,
                    ),
                });
            }
            Ok(SelectedMembers {
                rows,
                _charge: charge,
            })
        }
        .await;
        let mut terminal = Completion::default();
        terminal.step(
            "compact installed member driver drain",
            stream.drain_transport().await,
        );
        let result = complete(result, terminal);
        if result
            .as_ref()
            .err()
            .is_some_and(|error| !error.permits_storage_cleanup())
        {
            observed_reader.retain_unknown();
            if !protected {
                self.retain_unknown();
            }
        }
        result
    }

    async fn members_inner(
        &self,
        reader: &crate::NativeReader<()>,
        keys: &[[u8; 16]],
        budget: &ResourceBudget,
    ) -> Result<SelectedMembers, ModelError> {
        if keys.len() > crate::loader::NATIVE_WINDOW_ROWS
            || keys.windows(2).any(|pair| pair[0] >= pair[1])
        {
            return Err(ModelError::Conflict("selected member demand order/bound"));
        }
        self.ensure_borrowed_current(reader, budget).await?;
        super::earned::selected_members(self, reader, keys, budget).await
    }
}
fn retained(
    client: Arc<Surreal<Client>>,
    completion: InstalledViewCompletion,
    owner: InstalledViewOwner,
    budget: &ResourceBudget,
) -> Result<HeldInstalledView, ModelError> {
    let inventory = match &owner {
        InstalledViewOwner::Earned { .. } => {
            return Err(ModelError::Conflict(
                "retained selector requires inventory owner",
            ));
        }
        InstalledViewOwner::Physical { completed, .. } => completed.inventory(),
        InstalledViewOwner::Frozen { completed, .. } => completed.inventory(),
        InstalledViewOwner::Published { completed, .. } => completed.inventory(),
        InstalledViewOwner::Recovery { inventory, .. } => inventory.as_ref(),
    };
    inventory.require_pool(budget)?;
    let canonical = inventory.installed(completion.view.identity)?;
    if canonical != &completion {
        return Err(ModelError::Conflict(
            "retained selector differs from exact inventory",
        ));
    }
    let completion = canonical.clone();
    let mut charge = StateCharge::new(budget, "retained compound selector handle");
    charge.grow(
        std::mem::size_of::<HeldInstalledView>().saturating_add(2 * std::mem::size_of::<usize>()),
    )?;
    // The inventory's typed charge owns the shared selection. This cap retains
    // that exact inventory and its native protection through InstalledViewOwner.
    Ok(HeldInstalledView {
        client,
        selector: HeldSelector::Retained(completion),
        owner,
        _charge: charge,
    })
}
pub(crate) fn recovery_view(
    client: Arc<Surreal<Client>>,
    pin: Arc<crate::control::ReaderPin>,
    completion: InstalledViewCompletion,
    expected: crate::control::ExpectedRecord,
    inventory: Arc<crate::completed_state::InstalledViewInventory>,
    budget: &ResourceBudget,
) -> Result<HeldInstalledView, ModelError> {
    pin.ensure_live()?;
    pin.require_client(&client)?;
    inventory.require_pool(budget)?;
    {
        let _validation = budget.reserve(
            "recovery selector exact envelope workspace",
            crate::loader::native_bytes(expected.row())
                .saturating_mul(32)
                .saturating_add(4096),
        )?;
        if expected.row() != completion.value()? {
            return Err(ModelError::Conflict("recovery compound selector envelope"));
        }
    }
    let owner = RecordId::new("native_pin", pin.identity.hex());
    retained(
        client,
        completion,
        InstalledViewOwner::Recovery {
            expected: Arc::new(expected),
            pin,
            owner,
            inventory,
        },
        budget,
    )
}
pub async fn hold_published_view(
    completed: &crate::completed_state::PublishedCompletedState,
    view: ContentHash,
    budget: &ResourceBudget,
) -> Result<HeldInstalledView, ModelError> {
    completed.inventory().require_pool(budget)?;
    let installed = completed.inventory().installed(view)?.clone();
    let reader = completed
        .retained_reader()
        .transport_reader()
        .with_budget(budget);
    let expected = crate::control::inspect_expected_read(&reader, installed.record()?, budget)
        .await?
        .ok_or(ModelError::Conflict("published compound selector missing"))?;
    let owner = completed.completion().record()?;
    let held = retained(
        reader.shared_client(),
        installed,
        InstalledViewOwner::Published {
            expected: Arc::new(expected),
            completed: completed.clone(),
            owner,
        },
        budget,
    )?;
    held.ensure_current(&reader, budget).await?;
    Ok(held)
}
pub async fn hold_frozen_view(
    client: Arc<Surreal<Client>>,
    completed: Arc<crate::completed_state::HeldFrozenCompletedState>,
    view: ContentHash,
    budget: &ResourceBudget,
) -> Result<HeldInstalledView, ModelError> {
    if !Arc::ptr_eq(&client, &completed.client) {
        return Err(ModelError::Conflict("frozen selector foreign client"));
    }
    completed.inventory().require_pool(budget)?;
    let reader = crate::NativeReader::private(client.clone()).with_budget(budget);
    let result = async {
        let installed = completed.inventory().installed(view)?.clone();
        let expected = crate::control::inspect_expected_read(&reader, installed.record()?, budget)
            .await?
            .ok_or(ModelError::Conflict("frozen compound selector missing"))?;
        let owner = completed.completion().record()?;
        let held = retained(
            client.clone(),
            installed,
            InstalledViewOwner::Frozen {
                expected: Arc::new(expected),
                completed,
                owner,
            },
            budget,
        )?;
        held.ensure_current(&reader, budget).await?;
        Ok(held)
    }
    .await;
    let mut terminal = Completion::default();
    terminal.step("frozen selector reader close", reader.close().await);
    complete(result, terminal)
}
pub async fn hold_physical_view(
    client: Arc<Surreal<Client>>,
    completed: Arc<crate::completed_state::NativeCompletedState>,
    view: ContentHash,
    budget: &ResourceBudget,
) -> Result<HeldInstalledView, ModelError> {
    if !Arc::ptr_eq(&client, &completed.client) {
        return Err(ModelError::Conflict("physical selector foreign client"));
    }
    completed.inventory().require_pool(budget)?;
    let reader = crate::NativeReader::private(client.clone()).with_budget(budget);
    let result = async {
        let installed = completed.inventory().installed(view)?.clone();
        let expected = crate::control::inspect_expected_read(&reader, installed.record()?, budget)
            .await?
            .ok_or(ModelError::Conflict("physical compound selector missing"))?;
        let owner = completed.completion().record()?;
        let held = retained(
            client.clone(),
            installed,
            InstalledViewOwner::Physical {
                expected: Arc::new(expected),
                completed,
                owner,
            },
            budget,
        )?;
        held.ensure_current(&reader, budget).await?;
        Ok(held)
    }
    .await;
    let mut terminal = Completion::default();
    terminal.step("physical selector reader close", reader.close().await);
    complete(result, terminal)
}
pub async fn attach_live_views(
    client: Arc<Surreal<Client>>,
    attempt: ContentHash,
    sources: &[Arc<HeldInstalledView>],
    contributors: Vec<crate::control::ExpectedRecord>,
    budget: &ResourceBudget,
) -> Result<Vec<HeldInstalledView>, ModelError> {
    attach_live_views_inner(client, attempt, sources, contributors, vec![], budget).await
}
/// Retain an original candidate copy for bookkeeping while equivalent receiver roots
/// keep their own selectors. The original protected cap, never descriptor equality,
/// authorizes the extra exact contributor and its existing dependency hold chain.
pub async fn attach_live_views_retaining(
    client: Arc<Surreal<Client>>,
    attempt: ContentHash,
    sources: &[Arc<HeldInstalledView>],
    extras: Vec<(crate::control::ExpectedRecord, Arc<HeldInstalledView>)>,
    budget: &ResourceBudget,
) -> Result<Vec<HeldInstalledView>, ModelError> {
    attach_live_views_inner(client, attempt, sources, vec![], extras, budget).await
}
async fn attach_live_views_inner(
    client: Arc<Surreal<Client>>,
    attempt: ContentHash,
    sources: &[Arc<HeldInstalledView>],
    contributors: Vec<crate::control::ExpectedRecord>,
    extras: Vec<(crate::control::ExpectedRecord, Arc<HeldInstalledView>)>,
    budget: &ResourceBudget,
) -> Result<Vec<HeldInstalledView>, ModelError> {
    let reader = crate::NativeReader::private(client.clone()).with_budget(budget);
    let audit = InstallationAudit::new(client.clone(), budget).await?;
    let prepared = async {
        let owner = live_view::capture(&reader, attempt, budget).await?;
        let mut inventory_charge =
            StateCharge::new(budget, "atomic attached exact source inventory");
        inventory_charge.grow(
            sources
                .len()
                .saturating_mul(2048)
                .saturating_add(contributors.len().saturating_mul(1024))
                .saturating_add(4096),
        )?;
        let mut supplied = std::collections::BTreeMap::new();
        for record in contributors {
            if supplied.insert(record.record().clone(), record).is_some() {
                return Err(ModelError::Conflict(
                    "attached duplicate supplied contributor",
                ));
            }
        }
        let mut selected =
            std::collections::BTreeMap::<RecordId, crate::control::ExpectedRecord>::new();
        let mut logical = std::collections::BTreeSet::new();
        let mut result = Vec::with_capacity(sources.len());
        for source in sources {
            if !logical.insert(source.selector.view.identity) {
                return Err(ModelError::Conflict("attached duplicate source selector"));
            }
            source.ensure_current(&reader, budget).await?;
            for mapping in &source.selector.mappings {
                let id = RecordId::new("compiler_contribution", mapping.physical.hex());
                if let Some(previous) = selected.get(&id) {
                    if previous.incarnation() != mapping.incarnation
                        || super::audit::descriptor::<d::completed::CompletedContribution>(
                            previous.row(),
                        )?
                        .identity()?
                            != mapping.logical
                    {
                        return Err(ModelError::Conflict(
                            "attached conflicting selected contributor",
                        ));
                    }
                    continue;
                }
                inventory_charge.grow(1024)?;
                let actual = crate::control::inspect_expected_read(&reader, id.clone(), budget)
                    .await?
                    .ok_or(ModelError::Conflict("attached exact contributor missing"))?;
                if actual.incarnation() != mapping.incarnation
                    || super::audit::descriptor::<d::completed::CompletedContribution>(
                        actual.row(),
                    )?
                    .identity()?
                        != mapping.logical
                {
                    return Err(ModelError::Conflict(
                        "attached exact contributor descriptor/incarnation",
                    ));
                }
                if let Some(provided) = supplied.remove(&id)
                    && (provided.incarnation() != actual.incarnation()
                        || provided.row() != actual.row())
                {
                    return Err(ModelError::Conflict(
                        "attached supplied contributor bytes/incarnation changed",
                    ));
                }
                selected.insert(id, actual);
            }
            // Earn every prospective returned cap before the single receiving effect;
            // these independent pin-owned observations make no receiver attachment.
            result.push(
                super::earned::select_live_view_scoped(
                    client.clone(),
                    attempt,
                    &source.selector.view,
                    &source.selector.mappings,
                    budget,
                    &audit,
                )
                .await?,
            );
        }
        if !supplied.is_empty() {
            return Err(ModelError::Conflict(
                "attached contributor outside selected inventory",
            ));
        }
        for (expected, source) in &extras {
            source.ensure_current(&reader, budget).await?;
            let descriptor: d::completed::CompletedContribution =
                super::audit::descriptor(expected.row())?;
            let logical = descriptor.identity()?;
            if !source.completion().mappings.iter().any(|mapping| {
                RecordId::new("compiler_contribution", mapping.physical.hex()) == *expected.record()
                    && mapping.incarnation == expected.incarnation()
                    && mapping.logical == logical
            }) {
                return Err(ModelError::Conflict(
                    "extra candidate is outside original protected root",
                ));
            }
            if let Some(actual) = selected.get(expected.record()) {
                if actual.incarnation() != expected.incarnation() || actual.row() != expected.row()
                {
                    return Err(ModelError::Conflict(
                        "extra candidate exact selected collision",
                    ));
                }
            } else {
                inventory_charge.grow(
                    crate::loader::native_bytes(expected.row())
                        .saturating_mul(32)
                        .saturating_add(2048),
                )?;
                let actual = crate::control::inspect_expected_read(
                    &reader,
                    expected.record().clone(),
                    budget,
                )
                .await?
                .ok_or(ModelError::Conflict("extra candidate exact record missing"))?;
                if actual.incarnation() != expected.incarnation() || actual.row() != expected.row()
                {
                    return Err(ModelError::Conflict("extra candidate exact record changed"));
                }
                selected.insert(expected.record().clone(), actual);
            }
        }
        for (_, source) in &extras {
            source.ensure_current(&reader, budget).await?;
        }
        for source in sources {
            source.ensure_current(&reader, budget).await?;
        }
        inventory_charge.grow(
            selected
                .len()
                .saturating_mul(std::mem::size_of::<crate::control::ExpectedRecord>()),
        )?;
        Ok((
            owner,
            selected.into_values().collect::<Vec<_>>(),
            result,
            inventory_charge,
        ))
    }
    .await;
    let mut terminal = Completion::default();
    terminal.step(
        "atomic attachment prevalidation reader close",
        reader.close().await,
    );
    let prepared = complete(prepared, terminal);
    let (owner, expected, result, _inventory_charge) = match prepared {
        Ok(prepared) => prepared,
        Err(error) => {
            if !error.permits_storage_cleanup() {
                audit.retain_unknown();
            }
            let mut terminal = Completion::default();
            terminal.step(
                "failed attachment prospective audit close",
                audit.close().await,
            );
            return complete(Err(error), terminal);
        }
    };
    let attached = live_view::attach_contributors(&client, &owner, &expected, budget).await;
    if let Err(error) = attached {
        if !error.permits_storage_cleanup() {
            audit.retain_unknown();
        }
        drop(result);
        let mut terminal = Completion::default();
        terminal.step(
            "rejected atomic attachment audit close",
            audit.close().await,
        );
        return complete(Err(error), terminal);
    }
    audit.detach_borrowed();
    Ok(result)
}
/// Single-view convenience uses the same atomic reference-only factory.
pub async fn attach_live_view(
    client: Arc<Surreal<Client>>,
    attempt: ContentHash,
    source: &Arc<HeldInstalledView>,
    budget: &ResourceBudget,
) -> Result<HeldInstalledView, ModelError> {
    let mut held = attach_live_views(
        client,
        attempt,
        std::slice::from_ref(source),
        vec![],
        budget,
    )
    .await?;
    held.pop().ok_or(ModelError::Conflict(
        "attached singleton witness cardinality",
    ))
}
