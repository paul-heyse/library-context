//! Portable correspondence earned from exact protected selections. Labels never grant native authority.
use crate::{
    NativeReader,
    installed_view::{ContributorRealization, HeldInstalledView, InstallationAudit},
    ordered_rows::{PreparedRows, SortedRows},
};
use lctx_model::domain::{
    self as d, ContentHash, ModelError,
    charged::StateCharge,
    completed::{CompletedContribution, CompletedStateIdentity, CompletedView},
    completed_identity::CompletedFamilyRoots,
    completed_transport::*,
};
use std::{collections::BTreeMap, sync::Arc};
use surrealdb::{
    Surreal,
    engine::remote::grpc::Client,
    types::{RecordId, Value, Variables},
};

pub struct PreparedTransportCopy {
    pub label: CopyLabel,
    pub physical: ContentHash,
    pub memberships: PreparedRows,
    pub aliases: PreparedRows,
}
pub struct PreparedTransport {
    envelope: CompletedTransportEnvelope,
    copies: Vec<PreparedTransportCopy>,
    _charge: StateCharge,
}
impl PreparedTransport {
    pub fn envelope(&self) -> &CompletedTransportEnvelope {
        &self.envelope
    }
    pub fn copies(&self) -> &[PreparedTransportCopy] {
        &self.copies
    }
}
fn view_heap(view: &CompletedView) -> usize {
    std::mem::size_of::<CompletedView>()
        .saturating_add(view.relation.len())
        .saturating_add(view.contributions.len().saturating_mul(96))
        .saturating_add(512)
}
fn source_charge(
    roots: impl IntoIterator<Item = (usize, usize)>,
    budget: &d::resources::ResourceBudget,
) -> Result<StateCharge, ModelError> {
    let mut charge = StateCharge::new(budget, "transport selected descriptor handles");
    for (view, mappings) in roots {
        charge.grow(
            view.saturating_add(
                mappings.saturating_mul(std::mem::size_of::<ContributorRealization>()),
            )
            .saturating_add(1024),
        )?;
    }
    Ok(charge)
}
struct SelectionInventory {
    roots: Vec<TransportRoot>,
    copies: Vec<TransportCopy>,
    prepared: Vec<PreparedTransportCopy>,
    _charge: StateCharge,
}
pub fn inventory(
    run: &PreparedRows,
    kind: TransportInventoryKind,
    budget: &d::resources::ResourceBudget,
) -> Result<TransportInventory, ModelError> {
    let mut hash = TransportInventoryHasher::new(kind);
    let mut cursor = run.frame_cursor(budget)?;
    while let Some((_, bytes)) = cursor.next_frame()? {
        hash.row(bytes)?;
    }
    Ok(hash.finish())
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct SelectedCopyRequest {
    label: CopyLabel,
    physical: ContentHash,
    incarnation: i64,
    logical: ContentHash,
}
struct SelectedRequests {
    copies: Vec<SelectedCopyRequest>,
    labels: BTreeMap<ContentHash, CopyLabel>,
    dependency_views: Vec<ContentHash>,
    charge: StateCharge,
}
fn selected_requests(
    roots: &[(CompletedView, Vec<ContributorRealization>)],
    budget: &d::resources::ResourceBudget,
) -> Result<SelectedRequests, ModelError> {
    let mut copies: Vec<SelectedCopyRequest> = Vec::new();
    let mut labels: BTreeMap<ContentHash, CopyLabel> = BTreeMap::new();
    let mut views = std::collections::BTreeSet::new();
    let mut charge = StateCharge::new(budget, "transport exact observation requests");
    for (root, mappings) in roots {
        root.validate()?;
        for mapping in mappings {
            let request = SelectedCopyRequest {
                label: labels
                    .get(&mapping.physical)
                    .copied()
                    .unwrap_or(copies.len() as CopyLabel),
                physical: mapping.physical,
                incarnation: mapping.incarnation,
                logical: mapping.logical,
            };
            if let Some(label) = labels.get(&mapping.physical) {
                if copies[usize::try_from(*label).map_err(ModelError::codec)?] != request {
                    return Err(ModelError::Conflict(
                        "transport selected physical mapping contradiction",
                    ));
                }
            } else {
                charge.grow(std::mem::size_of::<SelectedCopyRequest>().saturating_add(256))?;
                labels.insert(mapping.physical, copies.len() as CopyLabel);
                copies.push(request);
            }
            if mapping.view != root.identity && !views.contains(&mapping.view) {
                charge.grow(std::mem::size_of::<ContentHash>().saturating_add(128))?;
                views.insert(mapping.view);
            }
        }
    }
    charge.grow(
        views
            .len()
            .saturating_mul(std::mem::size_of::<ContentHash>()),
    )?;
    Ok(SelectedRequests {
        copies,
        labels,
        dependency_views: views.into_iter().collect(),
        charge,
    })
}
async fn prepare_transport_copy(
    reader: &NativeReader<()>,
    request: SelectedCopyRequest,
    expected: crate::control::ExpectedRecord,
    include_memberships: bool,
    audit: &InstallationAudit,
    charge: &mut StateCharge,
    budget: &d::resources::ResourceBudget,
) -> Result<(TransportCopy, PreparedTransportCopy), ModelError> {
    crate::completed_rows::validate_state_row("compiler_contribution", expected.row())?;
    charge.grow(
        crate::loader::native_bytes(expected.row())
            .saturating_mul(32)
            .saturating_add(2048),
    )?;
    let descriptor: CompletedContribution =
        crate::installed_view::audit::descriptor(expected.row())?;
    if expected.incarnation() != request.incarnation || descriptor.identity()? != request.logical {
        return Err(ModelError::Conflict(
            "transport exact copy descriptor/incarnation",
        ));
    }
    let mut members = SortedRows::with_budget(budget)?;
    if include_memberships {
        let owner = expected.record().clone();
        let mut vars = Variables::new();
        vars.insert("owner", owner.clone());
        let mut stream = reader.stream_prepared(crate::prepared::PreparedQuery::new(vars, vec![], vec![
            "SELECT * FROM compiler_membership WITH INDEX contribution_rows WHERE contribution=$owner".into(),
        ])?)?.with_row_bytes(d::resources::MAX_ROW_BYTES);
        let owners = [(owner, descriptor.clone())].into();
        let observed = async {
            while let Some(row) = stream.next().await? {
                crate::completed_rows::validate_state_row("compiler_membership", &row)?;
                members.push(crate::completed_logical_rows::normalize(
                    "compiler_membership",
                    &row,
                    &owners,
                )?)?;
            }
            Ok(())
        }
        .await;
        let mut terminal = d::completion::Completion::default();
        terminal.step(
            "transport local membership EOF/drain",
            stream.drain_transport().await,
        );
        d::completion::complete(observed, terminal)?;
    }
    let memberships = members.finish()?.into_prepared();
    let aliases = audit.local_alias_run(reader, &expected, budget).await?;
    Ok((
        TransportCopy {
            label: request.label,
            descriptor,
            membership: inventory(&memberships, TransportInventoryKind::Memberships, budget)?,
            aliases: inventory(&aliases, TransportInventoryKind::Aliases, budget)?,
        },
        PreparedTransportCopy {
            label: request.label,
            physical: request.physical,
            memberships,
            aliases,
        },
    ))
}
fn selected_inventory<'a>(
    reader: &'a NativeReader<()>,
    roots: &'a [(CompletedView, Vec<ContributorRealization>)],
    include_memberships: bool,
    budget: &'a d::resources::ResourceBudget,
) -> Result<futures::future::BoxFuture<'a, Result<SelectionInventory, ModelError>>, ModelError> {
    let body = async move {
        let audit = InstallationAudit::new(reader.shared_client(), budget).await?;
        let result = async {
            let SelectedRequests {
                copies: requests,
                labels,
                dependency_views: views,
                charge: _requests,
            } = selected_requests(roots, budget)?;
            let mut result = SelectionInventory {
                roots: vec![],
                copies: vec![],
                prepared: vec![],
                _charge: StateCharge::new(budget, "portable root-qualified correspondence"),
            };
            let mut descriptors = BTreeMap::new();
            let mut offset = 0;
            let mut width = crate::loader::NATIVE_WINDOW_ROWS;
            while offset < requests.len() {
                let count = width.min(requests.len() - offset);
                let _page = match if count > 1 {
                    budget
                        .reserve(
                            "transport exact copy page identities",
                            count.saturating_mul(512),
                        )
                        .map(Some)
                } else {
                    Ok(None)
                } {
                    Err(ModelError::Resource { .. }) if count > 1 => {
                        width = (count / 2).max(1);
                        continue;
                    }
                    other => other?,
                };
                let ids = requests[offset..offset + count]
                    .iter()
                    .map(|request| RecordId::new("compiler_contribution", request.physical.hex()))
                    .collect::<Vec<_>>();
                let mut observed = match crate::control::inspect_expected_window_read(
                    reader, &ids, budget,
                )
                .await
                {
                    Err(ModelError::Resource { .. }) if count > 1 => {
                        width = (count / 2).max(1);
                        continue;
                    }
                    other => other?,
                };
                let start = offset;
                let mut retry = false;
                for (index, id) in ids.iter().enumerate() {
                    let expected = observed.take_required(id)?;
                    let retained = result._charge.reserved();
                    let prepared = prepare_transport_copy(
                        reader,
                        requests[start + index],
                        expected,
                        include_memberships,
                        &audit,
                        &mut result._charge,
                        budget,
                    )
                    .await;
                    match prepared {
                        Ok((copy, prepared)) => {
                            result.copies.push(copy);
                            result.prepared.push(prepared);
                            offset += 1;
                        }
                        Err(ModelError::Resource { .. }) if count > 1 => {
                            result
                                ._charge
                                .release(result._charge.reserved().saturating_sub(retained));
                            width = (count / 2).max(1);
                            retry = true;
                            break;
                        }
                        Err(error) => return Err(error),
                    }
                }
                if retry {
                    continue;
                }
                observed.finish()?;
                width = crate::loader::NATIVE_WINDOW_ROWS;
            }
            offset = 0;
            width = crate::loader::NATIVE_WINDOW_ROWS;
            while offset < views.len() {
                let count = width.min(views.len() - offset);
                let _page = match if count > 1 {
                    budget
                        .reserve(
                            "transport exact view page identities",
                            count.saturating_mul(512),
                        )
                        .map(Some)
                } else {
                    Ok(None)
                } {
                    Err(ModelError::Resource { .. }) if count > 1 => {
                        width = (count / 2).max(1);
                        continue;
                    }
                    other => other?,
                };
                let ids = views[offset..offset + count]
                    .iter()
                    .map(|view| RecordId::new("compiler_view", view.hex()))
                    .collect::<Vec<_>>();
                let mut observed = match crate::control::inspect_expected_window_read(
                    reader, &ids, budget,
                )
                .await
                {
                    Err(ModelError::Resource { .. }) if count > 1 => {
                        width = (count / 2).max(1);
                        continue;
                    }
                    other => other?,
                };
                let mut retry = false;
                for id in &ids {
                    let expected = observed.take_required(id)?;
                    let bytes = crate::loader::native_bytes(expected.row())
                        .saturating_mul(32)
                        .saturating_add(2048);
                    match result._charge.grow(bytes) {
                        Err(ModelError::Resource { .. }) if count > 1 => {
                            width = (count / 2).max(1);
                            retry = true;
                            break;
                        }
                        other => other?,
                    }
                    let view: CompletedView =
                        crate::installed_view::audit::descriptor(expected.row())?;
                    view.validate()?;
                    if view.identity != views[offset] {
                        return Err(ModelError::Conflict("transport dependency view descriptor"));
                    }
                    descriptors.insert(view.identity, view);
                    offset += 1;
                }
                if retry {
                    continue;
                }
                observed.finish()?;
                width = crate::loader::NATIVE_WINDOW_ROWS;
            }
            for (root, mappings) in roots {
                let mut slots = vec![];
                let mut dependencies = BTreeMap::new();
                result._charge.grow(view_heap(root))?;
                for mapping in mappings {
                    result._charge.grow(2048)?;
                    let label = labels[&mapping.physical];
                    if mapping.view != root.identity && !dependencies.contains_key(&mapping.view) {
                        let view = &descriptors[&mapping.view];
                        result._charge.grow(view_heap(view))?;
                        dependencies.insert(mapping.view, view.clone());
                    }
                    slots.push(TransportSlot {
                        view: mapping.view,
                        logical: mapping.logical,
                        copy: label,
                    });
                }
                result.roots.push(TransportRoot {
                    view: root.clone(),
                    dependencies: dependencies.into_values().collect(),
                    slots,
                });
            }
            Ok(result)
        }
        .await;
        let mut terminal = d::completion::Completion::default();
        terminal.step("transport local inventory audit close", audit.close().await);
        d::completion::complete(result, terminal)
    };
    // One complete inventory operation owns its continuation through audit cleanup.
    // Admit the concrete execution body before allocation or the first native read.
    crate::continuation::admit(budget, "native selected inventory continuation", body)
}

pub async fn prepare_selected(
    client: Arc<Surreal<Client>>,
    roots: &[Arc<HeldInstalledView>],
    semantic: &CompletedStateIdentity,
    families: &CompletedFamilyRoots,
    budget: &d::resources::ResourceBudget,
) -> Result<PreparedTransport, ModelError> {
    let reader = NativeReader::private(client).with_budget(budget);
    let result = async {
        let _selection_charge = source_charge(
            roots.iter().map(|root| {
                (
                    view_heap(&root.completion().view),
                    root.completion().mappings.len(),
                )
            }),
            budget,
        )?;
        let mut selected = vec![];
        for root in roots {
            root.ensure_current(&reader, budget).await?;
            selected.push((
                root.completion().view.clone(),
                root.completion().mappings.clone(),
            ));
        }
        selected.sort_by_key(|root| root.0.identity);
        let prepared = selected_inventory(&reader, &selected, true, budget)?.await?;
        for root in roots {
            root.ensure_current(&reader, budget).await?;
        }
        let envelope = CompletedTransportEnvelope {
            version: 1,
            semantic: semantic.clone(),
            families: families.clone(),
            roots: prepared.roots,
            copies: prepared.copies,
        };
        envelope.validate()?;
        Ok(PreparedTransport {
            envelope,
            copies: prepared.prepared,
            _charge: prepared._charge,
        })
    }
    .await;
    let mut terminal = d::completion::Completion::default();
    terminal.step(
        "transport protected selection reader close",
        reader.close().await,
    );
    d::completion::complete(result, terminal)
}
pub async fn eligibility_selected(
    client: Arc<Surreal<Client>>,
    roots: &[Arc<HeldInstalledView>],
    budget: &d::resources::ResourceBudget,
) -> Result<EligibilityIdentity, ModelError> {
    let reader = NativeReader::private(client).with_budget(budget);
    let result = async {
        let _selection_charge = source_charge(
            roots.iter().map(|root| {
                (
                    view_heap(&root.completion().view),
                    root.completion().mappings.len(),
                )
            }),
            budget,
        )?;
        let mut selected = vec![];
        for root in roots {
            root.ensure_current(&reader, budget).await?;
            selected.push((
                root.completion().view.clone(),
                root.completion().mappings.clone(),
            ));
        }
        selected.sort_by_key(|root| root.0.identity);
        let prepared = selected_inventory(&reader, &selected, false, budget)?.await?;
        for root in roots {
            root.ensure_current(&reader, budget).await?;
        }
        eligibility_projection(&prepared.roots, &prepared.copies)
    }
    .await;
    let mut terminal = d::completion::Completion::default();
    terminal.step(
        "eligibility protected selection reader close",
        reader.close().await,
    );
    d::completion::complete(result, terminal)
}
async fn inventory_projection(
    reader: &NativeReader<()>,
    inventory: &crate::completed_state::InstalledViewInventory,
    include_memberships: bool,
    budget: &d::resources::ResourceBudget,
) -> Result<SelectionInventory, ModelError> {
    let _selection_charge = source_charge(
        inventory.references().iter().map(|reference| {
            (
                view_heap(&reference.selector.view),
                reference.selector.mappings.len(),
            )
        }),
        budget,
    )?;
    let mut selected = vec![];
    for reference in inventory.references() {
        let root = inventory.installed(reference.logical_view)?;
        selected.push((root.view.clone(), root.mappings.clone()));
    }
    selected_inventory(reader, &selected, include_memberships, budget)?.await
}
pub async fn eligibility_frozen(
    client: Arc<Surreal<Client>>,
    completed: &crate::completed_state::HeldFrozenCompletedState,
    budget: &d::resources::ResourceBudget,
) -> Result<EligibilityIdentity, ModelError> {
    let reader = NativeReader::private(client).with_budget(budget);
    let result = async {
        crate::completed_state::verify_installed_refs(
            &reader,
            completed.completion(),
            completed.inventory().references(),
            budget,
        )
        .await?;
        let prepared = inventory_projection(&reader, completed.inventory(), false, budget).await?;
        eligibility_projection(&prepared.roots, &prepared.copies)
    }
    .await;
    let mut terminal = d::completion::Completion::default();
    terminal.step("frozen eligibility reader close", reader.close().await);
    d::completion::complete(result, terminal)
}
pub async fn prepare_physical(
    client: Arc<Surreal<Client>>,
    completed: &crate::completed_state::NativeCompletedState,
    budget: &d::resources::ResourceBudget,
) -> Result<PreparedTransport, ModelError> {
    let reader = NativeReader::private(client).with_budget(budget);
    let result = async {
        crate::completed_state::verify_installed_refs(
            &reader,
            completed.completion(),
            completed.inventory().references(),
            budget,
        )
        .await?;
        let prepared = inventory_projection(&reader, completed.inventory(), true, budget).await?;
        let envelope = CompletedTransportEnvelope {
            version: 1,
            semantic: CompletedStateIdentity::from_roots(&completed.completion().roots)?,
            families: completed.completion().roots.clone(),
            roots: prepared.roots,
            copies: prepared.copies,
        };
        envelope.validate()?;
        Ok(PreparedTransport {
            envelope,
            copies: prepared.prepared,
            _charge: prepared._charge,
        })
    }
    .await;
    let mut terminal = d::completion::Completion::default();
    terminal.step("physical transport reader close", reader.close().await);
    d::completion::complete(result, terminal)
}
pub async fn prepare_published(
    completed: &crate::completed_state::PublishedCompletedState,
    budget: &d::resources::ResourceBudget,
) -> Result<PreparedTransport, ModelError> {
    let reader = completed
        .retained_reader()
        .transport_reader()
        .with_budget(budget);
    let recovery=reader.attached_recovery_scope(completed.completion().generation)?;
    let result: Result<PreparedTransport, ModelError> = crate::control::with_recovery_scope(recovery,async {
        crate::completed_state::verify_installed_refs(
            &reader,
            completed.completion(),
            completed.inventory().references(),
            budget,
        )
        .await?;
        let prepared = inventory_projection(&reader, completed.inventory(), true, budget).await?;
        let envelope = CompletedTransportEnvelope {
            version: 1,
            semantic: CompletedStateIdentity::from_roots(&completed.completion().roots)?,
            families: completed.completion().roots.clone(),
            roots: prepared.roots,
            copies: prepared.copies,
        };
        envelope.validate()?;
        Ok(PreparedTransport {
            envelope,
            copies: prepared.prepared,
            _charge: prepared._charge,
        })
    }).await;
    // This is a borrowed transport view of the retained publication's reader/pin,
    // not a separately owned reader. Each operation above drains its own transport;
    // the enclosing publication owner closes the shared reader after all borrowers.
    if result
        .as_ref()
        .err()
        .is_some_and(|error| !error.permits_storage_cleanup())
    {
        reader.retain_unknown();
    }
    result
}
pub async fn eligibility_published(
    completed: &crate::completed_state::PublishedCompletedState,
    budget: &d::resources::ResourceBudget,
) -> Result<EligibilityIdentity, ModelError> {
    let reader = completed
        .retained_reader()
        .transport_reader()
        .with_budget(budget);
    let recovery=reader.attached_recovery_scope(completed.completion().generation)?;
    let result: Result<EligibilityIdentity, ModelError> = crate::control::with_recovery_scope(recovery,async {
        crate::completed_state::verify_installed_refs(
            &reader,
            completed.completion(),
            completed.inventory().references(),
            budget,
        )
        .await?;
        let prepared = inventory_projection(&reader, completed.inventory(), false, budget).await?;
        eligibility_projection(&prepared.roots, &prepared.copies)
    }).await;
    // Closing this shared transport view would attempt to close its live parent.
    if result
        .as_ref()
        .err()
        .is_some_and(|error| !error.permits_storage_cleanup())
    {
        reader.retain_unknown();
    }
    result
}

#[derive(serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
enum TransportFrame {
    Header {
        envelope: Box<CompletedTransportEnvelope>,
    },
    Row {
        table: String,
        row: Value,
    },
    CopyMembership {
        copy: CopyLabel,
        row: Value,
    },
    CopyAlias {
        copy: CopyLabel,
        row: Value,
    },
    End {
        receipt: CompletedTransportReceipt,
    },
}
/// Allocation preflight for this pinned JSON/NativeValue carrier, without a JSON tree.
/// Native Bytes is a byte vector; other sequences contain typed values/map nodes.
#[derive(Clone, Copy)]
struct DecoderShape {
    compact_bytes: bool,
}
impl<'de> serde::de::DeserializeSeed<'de> for DecoderShape {
    type Value = usize;
    fn deserialize<D: serde::Deserializer<'de>>(self, de: D) -> Result<usize, D::Error> {
        de.deserialize_any(self)
    }
}
impl<'de> serde::de::Visitor<'de> for DecoderShape {
    type Value = usize;
    fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        f.write_str("completed transport JSON value")
    }
    fn visit_unit<E: serde::de::Error>(self) -> Result<usize, E> {
        Ok(64)
    }
    fn visit_bool<E: serde::de::Error>(self, _: bool) -> Result<usize, E> {
        Ok(64)
    }
    fn visit_i64<E: serde::de::Error>(self, _: i64) -> Result<usize, E> {
        Ok(64)
    }
    fn visit_u64<E: serde::de::Error>(self, _: u64) -> Result<usize, E> {
        Ok(64)
    }
    fn visit_f64<E: serde::de::Error>(self, _: f64) -> Result<usize, E> {
        Ok(64)
    }
    fn visit_str<E: serde::de::Error>(self, value: &str) -> Result<usize, E> {
        Ok(value.len().saturating_mul(2).saturating_add(128))
    }
    fn visit_map<A: serde::de::MapAccess<'de>>(self, mut map: A) -> Result<usize, A::Error> {
        let mut bytes = 128usize;
        while let Some(key) = map.next_key::<String>()? {
            bytes = bytes
                .saturating_add(key.len().saturating_mul(2))
                .saturating_add(256);
            bytes = bytes.saturating_add(map.next_value_seed(DecoderShape {
                compact_bytes: key == "Bytes",
            })?);
        }
        Ok(bytes)
    }
    fn visit_seq<A: serde::de::SeqAccess<'de>>(self, mut sequence: A) -> Result<usize, A::Error> {
        let mut bytes = 128usize;
        if self.compact_bytes {
            while sequence.next_element::<u8>()?.is_some() {
                bytes = bytes.saturating_add(1);
            }
        } else {
            while let Some(value) = sequence.next_element_seed(DecoderShape {
                compact_bytes: false,
            })? {
                bytes = bytes.saturating_add(value).saturating_add(64);
            }
        }
        Ok(bytes)
    }
}
#[derive(serde::Serialize)]
#[serde(rename_all = "snake_case")]
enum BorrowedTransportFrame<'a> {
    Header {
        envelope: &'a CompletedTransportEnvelope,
    },
    Row {
        table: &'a str,
        row: &'a Value,
    },
    CopyMembership {
        copy: CopyLabel,
        row: &'a Value,
    },
    CopyAlias {
        copy: CopyLabel,
        row: &'a Value,
    },
    End {
        receipt: &'a CompletedTransportReceipt,
    },
}
const MAX_FRAME_BYTES: usize = d::resources::MAX_ROW_BYTES + 4096;
const TABLES: [&str; 6] = [
    "compiler_contribution",
    "compiler_membership",
    "compiler_view",
    "compiler_record",
    "compiler_binding",
    "compiler_alias",
];
struct FrameCounter(usize);
impl std::io::Write for FrameCounter {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        self.0 = self
            .0
            .checked_add(bytes.len())
            .ok_or_else(|| std::io::Error::other("transport frame count overflow"))?;
        if self.0 > MAX_FRAME_BYTES - 1 {
            return Err(std::io::Error::other("transport complete frame bound"));
        }
        Ok(bytes.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}
fn write_frame(
    writer: &mut impl std::io::Write,
    frame: &impl serde::Serialize,
) -> Result<(), ModelError> {
    // Count the exact complete codec before emitting it, with no extra serialized buffer.
    serde_json::to_writer(&mut FrameCounter(0), frame).map_err(ModelError::codec)?;
    serde_json::to_writer(&mut *writer, frame).map_err(ModelError::codec)?;
    writer.write_all(b"\n").map_err(ModelError::codec)
}
/// Returns a receipt only after complete framed EOF and file sync. Caller owns outer publication/finalizers.
pub fn write_transport(
    path: &std::path::Path,
    prepared: &PreparedTransport,
    families: &[PreparedRows; 6],
    budget: &d::resources::ResourceBudget,
) -> Result<CompletedTransportReceipt, ModelError> {
    use std::io::Write;
    let receipt = prepared.envelope.receipt()?;
    let _charge = budget.reserve("completed transport output buffer", 8192)?;
    let mut writer =
        std::io::BufWriter::new(std::fs::File::create(path).map_err(ModelError::codec)?);
    write_frame(
        &mut writer,
        &BorrowedTransportFrame::Header {
            envelope: &prepared.envelope,
        },
    )?;
    for (table, run) in TABLES.iter().zip(families) {
        let mut cursor = run.cursor_with_budget(budget)?;
        while let Some(row) = cursor.next_row()? {
            write_frame(
                &mut writer,
                &BorrowedTransportFrame::Row { table, row: &row },
            )?;
        }
    }
    for copy in &prepared.copies {
        let mut cursor = copy.memberships.cursor_with_budget(budget)?;
        while let Some(row) = cursor.next_row()? {
            write_frame(
                &mut writer,
                &BorrowedTransportFrame::CopyMembership {
                    copy: copy.label,
                    row: &row,
                },
            )?;
        }
        let mut cursor = copy.aliases.cursor_with_budget(budget)?;
        while let Some(row) = cursor.next_row()? {
            write_frame(
                &mut writer,
                &BorrowedTransportFrame::CopyAlias {
                    copy: copy.label,
                    row: &row,
                },
            )?;
        }
    }
    write_frame(
        &mut writer,
        &BorrowedTransportFrame::End { receipt: &receipt },
    )?;
    let flushed = writer.flush().map_err(ModelError::codec);
    let (file, pending) = writer.into_parts();
    flushed?;
    let pending = pending.map_err(|_| ModelError::Conflict("completed transport writer panic"))?;
    if !pending.is_empty() {
        return Err(ModelError::Conflict(
            "completed transport unflushed final bytes",
        ));
    }
    file.sync_all().map_err(ModelError::codec)?;
    Ok(receipt)
}
pub struct DecodedTransport {
    envelope: CompletedTransportEnvelope,
    families: [PreparedRows; 6],
    copies: Vec<PreparedTransportCopy>,
    _charge: StateCharge,
}
impl DecodedTransport {
    pub fn envelope(&self) -> &CompletedTransportEnvelope {
        &self.envelope
    }
    pub fn families(&self) -> &[PreparedRows; 6] {
        &self.families
    }
    pub fn copies(&self) -> &[PreparedTransportCopy] {
        &self.copies
    }
}
fn read_frame(
    reader: &mut impl std::io::BufRead,
    budget: &d::resources::ResourceBudget,
) -> Result<Option<(TransportFrame, StateCharge)>, ModelError> {
    let mut bytes = vec![];
    let mut charge = StateCharge::new(
        budget,
        "completed transport bounded frame and decoder workspace",
    );
    charge.grow(65536)?;
    loop {
        let available = reader.fill_buf().map_err(ModelError::codec)?;
        if available.is_empty() {
            break;
        }
        let size = available
            .iter()
            .position(|byte| *byte == b'\n')
            .map_or(available.len(), |i| i + 1);
        if bytes.len().saturating_add(size) > MAX_FRAME_BYTES {
            return Err(ModelError::Schema("completed transport row bound"));
        }
        charge.grow(size.saturating_mul(2))?;
        bytes.extend_from_slice(&available[..size]);
        reader.consume(size);
        if bytes.last() == Some(&b'\n') {
            break;
        }
    }
    let size = bytes.len();
    if size == 0 {
        return Ok(None);
    }
    if size > MAX_FRAME_BYTES || bytes.last() != Some(&b'\n') {
        return Err(ModelError::Schema(
            "completed transport bounded complete frame",
        ));
    }
    use serde::de::DeserializeSeed;
    let mut probe = serde_json::Deserializer::from_slice(&bytes);
    let decoded = DecoderShape {
        compact_bytes: false,
    }
    .deserialize(&mut probe)
    .map_err(ModelError::codec)?;
    probe.end().map_err(ModelError::codec)?;
    // Two times live decoded structure covers container growth and transient decode
    // copies. The independently charged raw Vec capacity remains live alongside it.
    charge.grow(decoded.saturating_mul(2).saturating_add(65536))?;
    Ok(Some((
        serde_json::from_slice(&bytes).map_err(ModelError::codec)?,
        charge,
    )))
}
/// Independently supplied expectation is checked before any native effects. Header and EOF cannot supply trust.
pub fn read_transport(
    path: &std::path::Path,
    expected: &CompletedTransportReceipt,
    budget: &d::resources::ResourceBudget,
) -> Result<DecodedTransport, ModelError> {
    expected.validate()?;
    let _input_charge = budget.reserve("completed transport input buffer", 8192)?;
    let mut reader = std::io::BufReader::new(std::fs::File::open(path).map_err(ModelError::codec)?);
    let Some((TransportFrame::Header { envelope }, mut charge)) = read_frame(&mut reader, budget)?
    else {
        return Err(ModelError::Schema("completed transport envelope header"));
    };
    envelope.validate()?;
    if &envelope.receipt()? != expected {
        return Err(ModelError::Conflict(
            "completed transport independent expectation",
        ));
    }
    charge.grow(
        envelope
            .copies
            .len()
            .saturating_mul(2048)
            .saturating_add(8192),
    )?;
    // Completed family rows retain the existing physical-row codec contract.
    // The default sorter profile is for narrow keys/members, not typed backing.
    let mut families = (0..6)
        .map(|_| SortedRows::with_budget_and_row_bytes(budget, d::resources::MAX_ROW_BYTES))
        .collect::<Result<Vec<_>, _>>()?;
    let mut members = envelope
        .copies
        .iter()
        .map(|_| SortedRows::with_budget(budget))
        .collect::<Result<Vec<_>, _>>()?;
    let mut aliases = envelope
        .copies
        .iter()
        .map(|_| SortedRows::with_budget(budget))
        .collect::<Result<Vec<_>, _>>()?;
    let mut previous: Option<(usize, u64, RecordId)> = None;
    loop {
        let (frame, _frame_charge) = read_frame(&mut reader, budget)?
            .ok_or(ModelError::Conflict("completed transport missing EOF"))?;
        let (phase, label, table, row) = match frame {
            TransportFrame::Row { table, row } => {
                let index = TABLES
                    .iter()
                    .position(|candidate| *candidate == table)
                    .ok_or(ModelError::Schema("completed transport family"))?;
                (index, 0, table, row)
            }
            TransportFrame::CopyMembership { copy, row } => {
                (6, copy, "compiler_membership".into(), row)
            }
            TransportFrame::CopyAlias { copy, row } => (6, copy, "compiler_alias".into(), row),
            TransportFrame::End { receipt } => {
                if &receipt != expected || read_frame(&mut reader, budget)?.is_some() {
                    return Err(ModelError::Conflict(
                        "completed transport EOF receipt/trailing frame",
                    ));
                }
                break;
            }
            TransportFrame::Header { .. } => {
                return Err(ModelError::Schema("completed transport duplicate header"));
            }
        };
        validate_logical_row(&table, &row)?;
        let Some(Value::RecordId(id)) = row.as_object().and_then(|row| row.get("id")) else {
            return Err(ModelError::Schema("completed transport row id"));
        };
        // Copy sections are ordered by label, membership then aliases. Table is in the key.
        let section = if phase == 6 {
            if table == "compiler_membership" {
                label.saturating_mul(2)
            } else {
                label.saturating_mul(2).saturating_add(1)
            }
        } else {
            0
        };
        let key = (phase, section, id.clone());
        if previous.as_ref().is_some_and(|previous| previous >= &key) {
            return Err(ModelError::Conflict("completed transport exact row order"));
        }
        previous = Some(key);
        if phase < 6 {
            families[phase].push(row)?;
        } else {
            let label = usize::try_from(label).map_err(ModelError::codec)?;
            let expected = envelope
                .copies
                .get(label)
                .ok_or(ModelError::Conflict("completed transport foreign copy"))?;
            if table == "compiler_membership" {
                let Some(Value::RecordId(owner)) =
                    row.as_object().and_then(|row| row.get("contribution"))
                else {
                    return Err(ModelError::Schema("completed transport member owner"));
                };
                if *owner
                    != RecordId::new(
                        "compiler_contribution",
                        expected.descriptor.identity()?.hex(),
                    )
                {
                    return Err(ModelError::Conflict(
                        "completed transport member exact logical owner",
                    ));
                }
                members[label].push(row)?;
            } else {
                aliases[label].push(row)?;
            }
        }
    }
    let families: [PreparedRows; 6] = families
        .into_iter()
        .map(|sort| sort.finish().map(|rows| rows.into_prepared()))
        .collect::<Result<Vec<_>, _>>()?
        .try_into()
        .map_err(|_| ModelError::Schema("completed transport six families"))?;
    let mut copies = vec![];
    for ((expected, members), aliases) in envelope.copies.iter().zip(members).zip(aliases) {
        let memberships = members.finish()?.into_prepared();
        let aliases = aliases.finish()?.into_prepared();
        if inventory(&memberships, TransportInventoryKind::Memberships, budget)?
            != expected.membership
            || inventory(&aliases, TransportInventoryKind::Aliases, budget)? != expected.aliases
        {
            return Err(ModelError::Conflict(
                "completed transport exact local inventory",
            ));
        }
        copies.push(PreparedTransportCopy {
            label: expected.label,
            physical: ContentHash([0; 32]),
            memberships,
            aliases,
        });
    }
    let mut roots = vec![];
    for (index, run) in families.iter().enumerate() {
        let mut hash = d::completed_identity::CompletedFamilyHasher::new(
            d::recovery_closure::COMPLETED_FAMILIES[index],
        )?;
        let mut cursor = run.frame_cursor(budget)?;
        while let Some((_, bytes)) = cursor.next_frame()? {
            hash.row(bytes)?;
        }
        roots.push(hash.finish());
    }
    let actual = CompletedFamilyRoots::new(
        roots
            .try_into()
            .map_err(|_| ModelError::Schema("transport family roots"))?,
    )?;
    if actual != envelope.families {
        return Err(ModelError::Conflict(
            "completed transport actual six families",
        ));
    }
    // Every semantic local membership and alias must match the complete union carried in v4.
    for (index, runs) in [
        (
            1,
            copies
                .iter()
                .map(|copy| copy.memberships.clone())
                .collect::<Vec<_>>(),
        ),
        (
            5,
            copies
                .iter()
                .map(|copy| copy.aliases.clone())
                .collect::<Vec<_>>(),
        ),
    ] {
        let run = PreparedRows::union_prepared(&runs, budget, || Ok(()))?;
        let mut hash = d::completed_identity::CompletedFamilyHasher::new(
            d::recovery_closure::COMPLETED_FAMILIES[index],
        )?;
        let mut cursor = run.frame_cursor(budget)?;
        while let Some((_, bytes)) = cursor.next_frame()? {
            hash.row(bytes)?;
        }
        if hash.finish() != envelope.families.roots()[index] {
            return Err(ModelError::Conflict(
                "completed transport local inventory semantic union",
            ));
        }
    }
    Ok(DecodedTransport {
        envelope: *envelope,
        families,
        copies,
        _charge: charge,
    })
}

/// Exact data-only canonical row codec. Native attempts, projections and grants are absent.
pub fn validate_logical_row(table: &str, row: &Value) -> Result<(), ModelError> {
    use surrealdb::types::{Bytes, Object};
    let object = row
        .as_object()
        .ok_or(ModelError::Schema("transport logical object"))?;
    let mut canonical = Object::new();
    match table {
        "compiler_contribution" => {
            let descriptor: CompletedContribution = crate::installed_view::audit::descriptor(row)?;
            canonical.insert("id", RecordId::new(table, descriptor.identity()?.hex()));
            canonical.insert(
                "descriptor",
                Bytes::from(serde_json::to_vec(&descriptor).map_err(ModelError::codec)?),
            );
        }
        "compiler_view" => {
            let descriptor: CompletedView = crate::installed_view::audit::descriptor(row)?;
            descriptor.validate()?;
            canonical.insert("id", RecordId::new(table, descriptor.identity.hex()));
            canonical.insert(
                "descriptor",
                Bytes::from(serde_json::to_vec(&descriptor).map_err(ModelError::codec)?),
            );
        }
        "compiler_binding" => {
            let descriptor: d::completed::CompletedBinding =
                crate::installed_view::audit::descriptor(row)?;
            descriptor.validate()?;
            canonical.insert("id", RecordId::new(table, descriptor.key().hex()));
            canonical.insert(
                "descriptor",
                Bytes::from(serde_json::to_vec(&descriptor).map_err(ModelError::codec)?),
            );
        }
        "compiler_membership" | "compiler_record" => {
            crate::completed_rows::validate_state_row(table, row)?;
            return Ok(());
        }
        "compiler_alias" => {
            let (Some(Value::RecordId(source)), Some(Value::RecordId(target))) =
                (object.get("source"), object.get("target"))
            else {
                return Err(ModelError::Schema("transport alias endpoints"));
            };
            if source.table.as_str() != "entity" || target.table.as_str() != "entity" {
                return Err(ModelError::Schema("transport alias entity family"));
            }
            let mut sink = d::KeySink::new("native-place-alias/v1");
            sink.part(
                b"source",
                &serde_json::to_vec(source).map_err(ModelError::codec)?,
            );
            sink.part(
                b"target",
                &serde_json::to_vec(target).map_err(ModelError::codec)?,
            );
            canonical.insert("id", RecordId::new(table, sink.finish().hex()));
            canonical.insert("source", source.clone());
            canonical.insert("target", target.clone());
        }
        _ => return Err(ModelError::Schema("transport logical table")),
    }
    if *row != Value::Object(canonical) {
        return Err(ModelError::Conflict(
            "transport exact canonical logical row",
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use d::{
        completed::{ContributionSpec, OutputContent, OutputProfile},
        producer_assurance::ProducerAssurance,
        stages::Profile,
    };
    use surrealdb::types::{Bytes, Object};
    fn run(rows: Vec<Value>, budget: &d::resources::ResourceBudget) -> PreparedRows {
        let mut sorted = SortedRows::with_budget(budget).unwrap();
        for row in rows {
            sorted.push(row).unwrap();
        }
        sorted.finish().unwrap().into_prepared()
    }
    #[test]
    fn selected_observation_requests_keep_first_copy_labels_and_exact_shared_incidence() {
        let budget = d::resources::ResourceBudget::fixed(8 << 20).unwrap();
        let (transport, _) = fixture(false, &budget);
        let mut roots = transport
            .envelope()
            .roots
            .iter()
            .enumerate()
            .map(|(index, root)| {
                (
                    root.view.clone(),
                    vec![ContributorRealization {
                        view: root.view.identity,
                        logical: transport.envelope().copies[0]
                            .descriptor
                            .identity()
                            .unwrap(),
                        physical: ContentHash([if index == 0 { 9 } else { 1 }; 32]),
                        incarnation: 3,
                    }],
                )
            })
            .collect::<Vec<_>>();
        let SelectedRequests {
            copies,
            labels,
            dependency_views: views,
            charge,
        } = selected_requests(&roots, &budget).unwrap();
        assert!(charge.reserved() > 0);
        assert_eq!(
            copies.iter().map(|copy| copy.physical).collect::<Vec<_>>(),
            vec![ContentHash([9; 32]), ContentHash([1; 32])]
        );
        assert_eq!(labels[&ContentHash([9; 32])], 0);
        assert_eq!(labels[&ContentHash([1; 32])], 1);
        assert!(views.is_empty());
        roots[1].1[0].physical = roots[0].1[0].physical;
        let SelectedRequests { copies, labels, .. } = selected_requests(&roots, &budget).unwrap();
        assert_eq!(copies.len(), 1);
        assert_eq!(labels[&ContentHash([9; 32])], 0);
        roots[1].1[0].incarnation += 1;
        assert!(selected_requests(&roots, &budget).is_err());
        roots[1].1[0].incarnation -= 1;
        roots[1].1[0].logical = ContentHash([8; 32]);
        assert!(selected_requests(&roots, &budget).is_err());
    }
    #[test]
    fn selected_observation_dependency_targets_are_exact_and_deduplicated() {
        let budget = d::resources::ResourceBudget::fixed(8 << 20).unwrap();
        let (transport, _) = fixture(false, &budget);
        let root = transport.envelope().roots[0].view.clone();
        let dependency = transport.envelope().roots[1].view.identity;
        let logical = transport.envelope().copies[0]
            .descriptor
            .identity()
            .unwrap();
        let mapping = ContributorRealization {
            view: dependency,
            logical,
            physical: ContentHash([7; 32]),
            incarnation: 2,
        };
        let roots = vec![(root, vec![mapping.clone(), mapping])];
        let SelectedRequests {
            copies,
            dependency_views: views,
            ..
        } = selected_requests(&roots, &budget).unwrap();
        assert_eq!(copies.len(), 1);
        assert_eq!(views, vec![dependency]);
    }
    fn alias(source: &str, target: &str) -> Value {
        let source = RecordId::new("entity", source);
        let target = RecordId::new("entity", target);
        let mut sink = d::KeySink::new("native-place-alias/v1");
        sink.part(b"source", &serde_json::to_vec(&source).unwrap());
        sink.part(b"target", &serde_json::to_vec(&target).unwrap());
        let mut row = Object::new();
        row.insert("id", RecordId::new("compiler_alias", sink.finish().hex()));
        row.insert("source", source);
        row.insert("target", target);
        Value::Object(row)
    }
    fn fixture(
        swapped: bool,
        budget: &d::resources::ResourceBudget,
    ) -> (PreparedTransport, [PreparedRows; 6]) {
        let descriptor = CompletedContribution {
            spec: ContributionSpec {
                captured_binding: None,
                producer: "copy-provider".into(),
                profile: OutputProfile::Specific(Profile::Catalog),
                model: ContentHash::of(b"model"),
                implementation: ContentHash::of(b"implementation"),
                configuration: None,
                inputs: vec![],
                outputs: ["packages".into(), "releases".into()].into(),
            },
            assurance: ProducerAssurance::CurrentProviderInterface,
            outcome: 0,
            outputs: ["packages", "releases"]
                .into_iter()
                .map(|relation| {
                    (
                        relation.into(),
                        OutputContent {
                            rows: 0,
                            content: ContentHash::of(b"empty"),
                        },
                    )
                })
                .collect(),
        };
        let logical = descriptor.identity().unwrap();
        let mut views = ["packages", "releases"]
            .into_iter()
            .map(|relation| CompletedView::new(relation.into(), [logical].into(), 0).unwrap())
            .collect::<Vec<_>>();
        views.sort_by_key(|view| view.identity);
        let roots = views
            .iter()
            .enumerate()
            .map(|(label, view)| TransportRoot {
                view: view.clone(),
                dependencies: vec![],
                slots: vec![TransportSlot {
                    view: view.identity,
                    logical,
                    copy: label as u64,
                }],
            })
            .collect();
        let first = alias("source-a", "target-a");
        let second = alias("source-b", "target-b");
        let mut copies = vec![];
        let mut prepared = vec![];
        for label in 0..2u64 {
            let memberships = run(vec![], budget);
            let selected = if (label == 0) ^ swapped {
                first.clone()
            } else {
                second.clone()
            };
            let aliases = run(vec![selected], budget);
            copies.push(TransportCopy {
                label,
                descriptor: descriptor.clone(),
                membership: inventory(&memberships, TransportInventoryKind::Memberships, budget)
                    .unwrap(),
                aliases: inventory(&aliases, TransportInventoryKind::Aliases, budget).unwrap(),
            });
            prepared.push(PreparedTransportCopy {
                label,
                physical: ContentHash::of(&label.to_le_bytes()),
                memberships,
                aliases,
            });
        }
        let mut contribution = Object::new();
        contribution.insert("id", RecordId::new("compiler_contribution", logical.hex()));
        contribution.insert(
            "descriptor",
            Bytes::from(serde_json::to_vec(&descriptor).unwrap()),
        );
        let view_rows = views
            .iter()
            .map(|view| {
                let mut row = Object::new();
                row.insert("id", RecordId::new("compiler_view", view.identity.hex()));
                row.insert("descriptor", Bytes::from(serde_json::to_vec(view).unwrap()));
                Value::Object(row)
            })
            .collect();
        let families = [
            run(vec![Value::Object(contribution)], budget),
            run(vec![], budget),
            run(view_rows, budget),
            run(vec![], budget),
            run(vec![], budget),
            run(vec![first, second], budget),
        ];
        let roots_hash = CompletedFamilyRoots::new(std::array::from_fn(|index| {
            let mut hash = d::completed_identity::CompletedFamilyHasher::new(
                d::recovery_closure::COMPLETED_FAMILIES[index],
            )
            .unwrap();
            let mut cursor = families[index].frame_cursor(budget).unwrap();
            while let Some((_, bytes)) = cursor.next_frame().unwrap() {
                hash.row(bytes).unwrap();
            }
            hash.finish()
        }))
        .unwrap();
        let envelope = CompletedTransportEnvelope {
            version: 1,
            semantic: CompletedStateIdentity::from_roots(&roots_hash).unwrap(),
            families: roots_hash,
            roots,
            copies,
        };
        envelope.validate().unwrap();
        (
            PreparedTransport {
                envelope,
                copies: prepared,
                _charge: StateCharge::new(budget, "transport test owner"),
            },
            families,
        )
    }
    #[test]
    fn equal_v4_swapped_copy_incidence_requires_the_independent_receipt() {
        let budget = d::resources::ResourceBudget::fixed(256 << 20).unwrap();
        let (one, families) = fixture(false, &budget);
        let (two, other) = fixture(true, &budget);
        assert_eq!(one.envelope.semantic, two.envelope.semantic);
        assert_ne!(
            one.envelope.receipt().unwrap(),
            two.envelope.receipt().unwrap()
        );
        let file = tempfile::NamedTempFile::new().unwrap();
        let expected = write_transport(file.path(), &one, &families, &budget).unwrap();
        let decoded = read_transport(file.path(), &expected, &budget).unwrap();
        assert_eq!(decoded.copies.len(), 2);
        assert_eq!(
            decoded.envelope.copies[0].descriptor,
            decoded.envelope.copies[1].descriptor
        );
        assert!(
            decoded
                .envelope
                .copies
                .iter()
                .all(|copy| copy.membership.rows == 0)
        );
        write_transport(file.path(), &two, &other, &budget).unwrap();
        assert!(matches!(
            read_transport(file.path(), &expected, &budget),
            Err(ModelError::Conflict(
                "completed transport independent expectation"
            ))
        ));
    }
    #[test]
    fn missing_local_alias_inventory_and_failed_eof_never_decode() {
        let budget = d::resources::ResourceBudget::fixed(256 << 20).unwrap();
        let (one, families) = fixture(false, &budget);
        let file = tempfile::NamedTempFile::new().unwrap();
        let expected = write_transport(file.path(), &one, &families, &budget).unwrap();
        let original = std::fs::read_to_string(file.path()).unwrap();
        let missing = original
            .lines()
            .filter(|line| !line.contains("\"copy_alias\""))
            .map(|line| format!("{line}\n"))
            .collect::<String>();
        std::fs::write(file.path(), missing).unwrap();
        assert!(matches!(
            read_transport(file.path(), &expected, &budget),
            Err(ModelError::Conflict(
                "completed transport exact local inventory"
            ))
        ));
        let truncated = original
            .lines()
            .filter(|line| !line.contains("\"end\""))
            .map(|line| format!("{line}\n"))
            .collect::<String>();
        std::fs::write(file.path(), truncated).unwrap();
        assert!(matches!(
            read_transport(file.path(), &expected, &budget),
            Err(ModelError::Conflict("completed transport missing EOF"))
        ));
        std::fs::write(file.path(), format!("{original}{{\"end\":{{}}}}\n")).unwrap();
        assert!(read_transport(file.path(), &expected, &budget).is_err());
    }
    #[test]
    fn maximum_projection_binary_codec_uses_compact_bytes_under_original_budget() {
        let budget = d::resources::ResourceBudget::fixed(256 << 20).unwrap();
        let _source_charge = budget
            .reserve("maximum codec fixture source", 32 << 20)
            .unwrap();
        let payload = vec![255u8; d::projection::snapshot::CHUNK_BYTES];
        let canonical = serde_json::to_vec(&d::EvidenceBytes(payload.clone())).unwrap();
        let mut body = Object::new();
        body.insert("payload", Bytes::from(payload));
        let mut row = Object::new();
        row.insert("canonical", Bytes::from(canonical));
        row.insert("body", Value::Object(body));
        let row = Value::Object(row);
        let file = tempfile::NamedTempFile::new().unwrap();
        let mut output = std::fs::File::create(file.path()).unwrap();
        write_frame(
            &mut output,
            &BorrowedTransportFrame::Row {
                table: "compiler_record",
                row: &row,
            },
        )
        .unwrap();
        assert!(file.as_file().metadata().unwrap().len() > d::resources::TRANSFER_BYTES as u64);
        let mut input = std::io::BufReader::new(std::fs::File::open(file.path()).unwrap());
        let (actual, _charge) = read_frame(&mut input, &budget).unwrap().unwrap();
        assert!(matches!(actual,TransportFrame::Row{row:actual,..} if actual==row));
        assert!(budget.reserved() < 256 << 20);
    }
    #[test]
    fn oversized_complete_outgoing_frame_refuses_before_output() {
        let budget = d::resources::ResourceBudget::fixed(128 << 20).unwrap();
        let _payload_charge = budget
            .reserve("oversized codec fixture", MAX_FRAME_BYTES)
            .unwrap();
        let payload = "x".repeat(MAX_FRAME_BYTES);
        let mut output = Vec::new();
        assert!(write_frame(&mut output, &payload).is_err());
        assert!(output.is_empty());
    }

    #[test]
    fn maximum_projection_backing_family_reaches_full_transport_decode_under_original_budget() {
        use d::{
            Record,
            projection::{ProjectionSnapshotChunk, snapshot},
        };
        let budget = d::resources::ResourceBudget::fixed(256 << 20).unwrap();
        let _source_charge = budget
            .reserve("maximum backing codec fixture source", 32 << 20)
            .unwrap();
        let chunk = ProjectionSnapshotChunk {
            snapshot: serde_json::from_value(serde_json::to_value([7u8; 16]).unwrap()).unwrap(),
            ordinal: 0,
            payload: d::EvidenceBytes(vec![255; snapshot::CHUNK_BYTES]),
        };
        let view = crate::codec::views(std::slice::from_ref(&chunk))
            .unwrap()
            .pop()
            .unwrap();
        let canonical = serde_json::to_vec(&view.body).unwrap();
        let content = ContentHash::of(&canonical);
        let key = crate::completed_rows::decode_nominal_key(&view.semantic_key).unwrap();
        let id = crate::loader::payload_id(
            "compiler_record",
            ProjectionSnapshotChunk::NAME,
            &key,
            content,
        )
        .unwrap();
        let backing = crate::adapter::physical_row(id, content, canonical, None, view).unwrap();
        validate_logical_row("compiler_record", &backing).unwrap();
        let (mut prepared, mut families) = fixture(false, &budget);
        let mut sorted =
            SortedRows::with_budget_and_row_bytes(&budget, d::resources::MAX_ROW_BYTES).unwrap();
        sorted.push(backing).unwrap();
        families[3] = sorted.finish().unwrap().into_prepared();
        let mut hash = d::completed_identity::CompletedFamilyHasher::new(
            d::recovery_closure::COMPLETED_FAMILIES[3],
        )
        .unwrap();
        let mut rows = families[3].frame_cursor(&budget).unwrap();
        let (_, bytes) = rows.next_frame().unwrap().unwrap();
        assert!(bytes.len() > d::resources::TRANSFER_BYTES);
        hash.row(bytes).unwrap();
        assert!(rows.next_frame().unwrap().is_none());
        drop(rows);
        let mut roots = prepared.envelope.families.roots().clone();
        roots[3] = hash.finish();
        prepared.envelope.families = CompletedFamilyRoots::new(roots).unwrap();
        prepared.envelope.semantic =
            CompletedStateIdentity::from_roots(&prepared.envelope.families).unwrap();
        let file = tempfile::NamedTempFile::new().unwrap();
        let expected = write_transport(file.path(), &prepared, &families, &budget).unwrap();
        let decoded = read_transport(file.path(), &expected, &budget).unwrap();
        assert_eq!(decoded.envelope.semantic.backing_rows, 1);
        assert_eq!(decoded.envelope.receipt().unwrap(), expected);
    }
}
