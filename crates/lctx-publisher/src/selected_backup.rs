//! Fixed read-only lowering for one external native snapshot. No request-controlled SQL,
//! operational authority, service-wide content scan or per-page transaction is admitted.
use crate::{backup_import, definitions, inspection};
use lctx_model::domain::{
    ContentHash, ModelError,
    completed::{CompletedContribution, CompletedView},
    graph::Entity,
    recovery_closure::{RECOVERY_FAMILIES, RecoveryFamily, RecoveryTraversal},
    resources::{Reservation, ResourceBudget},
    serving::SnapshotHandle,
};
use lctx_native::{
    prepared::PreparedQuery,
    reader::NativeRows,
    selection::SelectedPayloads,
    surrealdb::{
        engine::remote::grpc::Client,
        method::Transaction,
        types::{Object, RecordId, SurrealValue, Value, Variables},
    },
};
use std::{
    collections::{BTreeMap, BTreeSet},
    io::Write,
    path::Path,
    sync::Arc,
};

pub(crate) struct Snapshot<'a> {
    pub(crate) transaction: &'a Arc<Transaction<Client>>,
    pub(crate) cancel: &'a crate::owned_read::Cancellation,
    pub(crate) budget: &'a ResourceBudget,
}
pub(crate) struct BackupSpools<'a> {
    pub(crate) claims: &'a Path,
    pub(crate) completed_state: &'a Path,
}
async fn read(
    transaction: &Snapshot<'_>,
    sql: String,
    bindings: Variables,
    mut accept: impl FnMut(Value) -> Result<(), ModelError>,
) -> Result<(), ModelError> {
    transaction.cancel.check()?;
    let mut rows = PreparedQuery::new(bindings, vec![], vec![sql])?
        .stream_transaction(transaction.transaction, transaction.budget)?;
    let result = async {
        while let Some(row) = rows.next().await? {
            transaction.cancel.check()?;
            accept(row)?;
        }
        Ok(())
    }
    .await;
    let mut completion = lctx_model::domain::completion::Completion::default();
    completion.step(
        "selected backup query drainage",
        rows.drain_transport().await,
    );
    lctx_model::domain::completion::complete(result, completion)
}
async fn one(
    transaction: &Snapshot<'_>,
    sql: String,
    variables: Variables,
) -> Result<Value, ModelError> {
    let mut value = None;
    read(transaction, sql, variables, |row| {
        if value.replace(row).is_some() {
            return Err(ModelError::Conflict("selected backup singleton inventory"));
        }
        Ok(())
    })
    .await?;
    value.ok_or(ModelError::Conflict("selected backup required singleton"))
}
fn object(value: &Value) -> Result<&Object, ModelError> {
    let Value::Object(object) = value else {
        return Err(ModelError::Schema("selected backup object"));
    };
    Ok(object)
}
fn id(value: &Value) -> Result<RecordId, ModelError> {
    RecordId::from_value(
        object(value)?
            .get("id")
            .cloned()
            .ok_or(ModelError::Schema("selected backup record identity"))?,
    )
    .map_err(ModelError::codec)
}
fn descriptor<T: serde::de::DeserializeOwned>(value: &Value) -> Result<T, ModelError> {
    let Some(Value::Bytes(bytes)) = object(value)?.get("descriptor") else {
        return Err(ModelError::Schema("selected backup descriptor bytes"));
    };
    serde_json::from_slice(bytes).map_err(ModelError::codec)
}
fn exact(key: &str, value: RecordId) -> Variables {
    let mut vars = Variables::new();
    vars.insert(key, value);
    vars
}
fn definition_groups(
    object: &Object,
    group: &str,
    output: &mut BTreeSet<String>,
) -> Result<(), ModelError> {
    let Some(Value::Object(group)) = object.get(group) else {
        return Err(ModelError::Schema("snapshot definition group"));
    };
    for value in group.values() {
        let Value::String(sql) = value else {
            return Err(ModelError::Schema("snapshot definition text"));
        };
        output.insert(definitions::normalize(sql)?);
    }
    Ok(())
}
async fn definition_inventory(transaction: &Snapshot<'_>) -> Result<BTreeSet<String>, ModelError> {
    let database = one(transaction, "INFO FOR DB".into(), Variables::new()).await?;
    let database = object(&database)?;
    let mut inventory = BTreeSet::new();
    for group in ["functions", "analyzers", "tables"] {
        definition_groups(database, group, &mut inventory)?;
    }
    let Some(Value::Object(tables)) = database.get("tables") else {
        return Err(ModelError::Schema("snapshot table metadata"));
    };
    for name in tables.keys() {
        let name = name.replace('\\', "\\\\").replace('`', "\\`");
        let table = one(
            transaction,
            format!("INFO FOR TABLE `{name}`"),
            Variables::new(),
        )
        .await?;
        for group in ["fields", "indexes"] {
            definition_groups(object(&table)?, group, &mut inventory)?;
        }
    }
    Ok(inventory)
}
async fn view(transaction: &Snapshot<'_>, identity: ContentHash) -> Result<Value, ModelError> {
    one(
        transaction,
        "SELECT * FROM $view".into(),
        exact("view", RecordId::new("compiler_view", identity.hex())),
    )
    .await
}
/// Exact rows and their validated descriptors belong to this snapshot alone. Dependency
/// discovery and eventual view export share the same acquisition, not two authorities.
struct SnapshotViews {
    rows: BTreeMap<ContentHash, (Value, CompletedView)>,
    charge: Box<dyn Reservation>,
}
impl SnapshotViews {
    fn new(budget: &ResourceBudget) -> Result<Self, ModelError> {
        Ok(Self {
            rows: BTreeMap::new(),
            charge: budget.reserve("selected-backup-view-descriptors", 0)?,
        })
    }
    async fn acquire<F>(
        &mut self,
        identity: ContentHash,
        read: impl FnOnce(ContentHash) -> F,
    ) -> Result<&(Value, CompletedView), ModelError>
    where
        F: std::future::Future<Output = Result<Value, ModelError>>,
    {
        if let std::collections::btree_map::Entry::Vacant(entry) = self.rows.entry(identity) {
            let row = read(identity).await?;
            // Admit both the retained native row and decoded descriptor before decoding.
            // This also covers tree entries and the traversal's temporary descriptor copy.
            let previous = self.charge.size();
            self.charge.try_resize(
                previous.saturating_add(
                    lctx_native::loader::native_bytes(&row)
                        .saturating_mul(4)
                        .saturating_add(512),
                ),
            )?;
            let decoded = (|| {
                let descriptor: CompletedView = descriptor(&row)?;
                descriptor.validate()?;
                if descriptor.identity != identity
                    || id(&row)? != RecordId::new("compiler_view", identity.hex())
                {
                    return Err(ModelError::Conflict("snapshot view identity"));
                }
                Ok(descriptor)
            })();
            let descriptor = match decoded {
                Ok(descriptor) => descriptor,
                Err(error) => {
                    self.charge.try_resize(previous)?;
                    return Err(error);
                }
            };
            entry.insert((row, descriptor));
        }
        self.rows
            .get(&identity)
            .ok_or(ModelError::Conflict("snapshot view acquisition absent"))
    }
}
async fn snapshot_closure<F>(
    views: &mut SnapshotViews,
    mut traversal: RecoveryTraversal,
    logical_descriptors: &BTreeMap<ContentHash, &CompletedContribution>,
    cancel: &crate::owned_read::Cancellation,
    mut read: impl FnMut(ContentHash) -> F,
) -> Result<lctx_model::domain::recovery_closure::RecoveryClosure, ModelError>
where
    F: std::future::Future<Output = Result<Value, ModelError>>,
{
    while let Some(requested) = traversal.next_view()? {
        cancel.check()?;
        let actual = views
            .acquire(requested.identity, &mut read)
            .await?
            .1
            .clone();
        let mut contributors = Vec::new();
        for logical in &actual.contributions {
            let descriptor = (*logical_descriptors
                .get(logical)
                .ok_or(ModelError::Conflict(
                    "snapshot exact selected logical descriptor absent",
                ))?)
            .clone();
            for input in &descriptor.spec.inputs {
                cancel.check()?;
                views.acquire(input.view(), &mut read).await?;
            }
            contributors.push(descriptor);
        }
        traversal.include_view(&requested, actual, contributors, |id| {
            views
                .rows
                .get(&id)
                .map(|(_, descriptor)| descriptor.clone())
                .ok_or(ModelError::Conflict("snapshot dependency view absent"))
        })?;
    }
    traversal.finish()
}
async fn write_stream(
    writer: &mut impl Write,
    charge: &mut dyn Reservation,
    mut rows: NativeRows,
    cancel: &crate::owned_read::Cancellation,
) -> Result<(), ModelError> {
    let result = async {
        while let Some(row) = rows.next().await? {
            cancel.check()?;
            backup_import::write_row(writer, row, charge)?;
        }
        Ok(())
    }
    .await;
    let mut completion = lctx_model::domain::completion::Completion::default();
    completion.step(
        "selected backup content drainage",
        rows.drain_transport().await,
    );
    lctx_model::domain::completion::complete(result, completion)
}
/// Write one coherent provisional data dump. The caller drains/cancels this same transaction
/// before any file publication and retains the exact pin through cancellation acknowledgement.
pub(crate) async fn export(
    transaction: &Snapshot<'_>,
    completed: &lctx_native::completed_state::PublishedCompletedState,
    version: &str,
    handle: &SnapshotHandle,
    native_definitions: &str,
    spools: BackupSpools<'_>,
    budget: &ResourceBudget,
) -> Result<
    (
        ContentHash,
        lctx_model::domain::completed_transport::CompletedTransportReceipt,
    ),
    ModelError,
> {
    let cancel = transaction.cancel;
    cancel.check()?;
    let publication = one(
        transaction,
        "SELECT * FROM $publication".into(),
        exact(
            "publication",
            RecordId::new("publication", handle.publication.hex()),
        ),
    )
    .await?;
    let marker = inspection::Marker::from_value(publication.clone()).map_err(ModelError::codec)?;
    let (actual_handle, manifest, bindings) = inspection::decode(marker)?;
    if actual_handle != *handle {
        return Err(ModelError::Conflict("snapshot exact publication handle"));
    }
    if manifest.semantic_contract
        != lctx_model::domain::graph::semantic_contract(&lctx_model::domain::model()?)
    {
        return Err(ModelError::Conflict("snapshot semantic contract"));
    }
    if definitions::epoch_identity(native_definitions) != handle.definition_epoch {
        return Err(ModelError::Conflict("snapshot pinned definition epoch"));
    }
    let inventory = definition_inventory(transaction).await?;
    if definitions::verify_inventory(native_definitions, version, &inventory)? != handle.realization
    {
        return Err(ModelError::Conflict("snapshot pinned realization"));
    }
    let transport = lctx_native::completed_transport::prepare_published(completed, budget).await?;
    if transport.envelope().semantic != manifest.completed_state
        || transport.envelope().eligibility_identity()? != handle.eligibility
    {
        return Err(ModelError::Conflict(
            "snapshot protected completed transport identity",
        ));
    }
    let _output_buffer = budget.reserve("selected-backup-output-buffer", 8192)?;
    let mut writer = std::io::BufWriter::new(
        std::fs::OpenOptions::new()
            .write(true)
            .truncate(true)
            .open(spools.claims)
            .map_err(ModelError::codec)?,
    );
    writeln!(writer, "OPTION IMPORT;").map_err(ModelError::codec)?;
    let (base, functions) = definitions::expected(native_definitions)?;
    let pinned = base.into_iter().chain(functions).collect::<BTreeSet<_>>();
    // Schema metadata are comparison-only. Exclude runtime controls and other executable epochs.
    for definition in &inventory {
        if pinned.contains(definition) || content_definition(definition)? {
            writeln!(writer, "{definition};").map_err(ModelError::codec)?;
        }
    }
    let mut encoded = budget.reserve("selected-backup-encoded-row", 0)?;
    backup_import::write_row(&mut writer, publication, encoded.as_mut())?;
    let traversal = RecoveryTraversal::new(bindings.clone())?;
    // A logical contributor may have several equivalent physical copies selected by
    // different installed views. Retain those exact selected copies rather than replace
    // the view-local choice with one process-global first logical match.
    let mut chosen = BTreeMap::<RecordId, Value>::new();
    let mut physical = BTreeSet::new();
    let mut views = SnapshotViews::new(budget)?;
    let mut descriptor_charge = budget.reserve("selected-backup-descriptor-closure", 0)?;
    // Every root-qualified copy in the envelope is retained, including equal logical
    // descriptors with different local alias inventories. Never select one first copy.
    for copy in transport.copies() {
        let owner = RecordId::new("compiler_contribution", copy.physical.hex());
        let row = one(
            transaction,
            "SELECT * FROM $owner".into(),
            exact("owner", owner.clone()),
        )
        .await?;
        let descriptor: CompletedContribution = descriptor(&row)?;
        let expected = &transport.envelope().copies[copy.label as usize].descriptor;
        if id(&row)? != owner
            || object(&row)?.get("completed") != Some(&Value::Bool(true))
            || &descriptor != expected
        {
            return Err(ModelError::Conflict(
                "snapshot exact selected copy descriptor",
            ));
        }
        descriptor_charge.try_resize(
            descriptor_charge.size().saturating_add(
                serde_json::to_vec(&row)
                    .map_err(ModelError::codec)?
                    .len()
                    .saturating_mul(4),
            ),
        )?;
        physical.insert(owner.clone());
        chosen.insert(owner, row);
    }
    descriptor_charge.try_resize(
        descriptor_charge
            .size()
            .saturating_add(transport.envelope().copies.len().saturating_mul(128)),
    )?;
    let logical_descriptors = transport
        .envelope()
        .copies
        .iter()
        .map(|copy| Ok((copy.descriptor.identity()?, &copy.descriptor)))
        .collect::<Result<BTreeMap<_, _>, ModelError>>()?;
    let closure = snapshot_closure(
        &mut views,
        traversal,
        &logical_descriptors,
        cancel,
        |identity| view(transaction, identity),
    )
    .await?;
    let all_views = transport
        .envelope()
        .roots
        .iter()
        .map(|root| root.view.identity)
        .collect::<Vec<_>>();
    let mut owners = BTreeMap::new();
    for (owner, row) in &chosen {
        let descriptor: CompletedContribution = descriptor(row)?;
        descriptor_charge.try_resize(
            descriptor_charge
                .size()
                .saturating_add(
                    serde_json::to_vec(&descriptor)
                        .map_err(ModelError::codec)?
                        .len()
                        .saturating_mul(32),
                )
                .saturating_add(512),
        )?;
        owners.insert(owner.clone(), descriptor);
    }
    let mut state = (0..6)
        .map(|_| {
            lctx_native::ordered_rows::SortedRows::with_budget_and_row_bytes(
                budget,
                lctx_model::domain::resources::MAX_ROW_BYTES,
            )
        })
        .collect::<Result<Vec<_>, _>>()?;
    // The native factory repeats the marker, inventory, exact incarnations, membership
    // and hold checks in this same external snapshot before any selected rows are emitted.
    let selected = SelectedPayloads::for_published_transaction(
        transaction.transaction.clone(),
        completed,
        &all_views,
        cancel.flag(),
        budget,
    )
    .await?
    .with_derivation(handle.realization);
    let mut originals = manifest
        .originals
        .iter()
        .map(|original| lctx_native::originals::layout_id(original.source))
        .collect::<Result<BTreeSet<_>, _>>()?;
    let mut blobs = BTreeMap::new();
    let mut role_claims = None;
    let mut search_claims = None;
    // This match is the actual family dispatcher, not a coverage assertion over table names.
    // A new model family enters this loop and requires an explicit export treatment.
    for family in RECOVERY_FAMILIES.iter().copied() {
        cancel.check()?;
        match family {
            RecoveryFamily::Views => {
                for (row, _) in views.rows.values() {
                    write_state_row(
                        &mut writer,
                        row.clone(),
                        encoded.as_mut(),
                        &mut state,
                        &owners,
                    )?;
                }
            }
            RecoveryFamily::Contributions => {
                for row in chosen.values() {
                    write_state_row(
                        &mut writer,
                        row.clone(),
                        encoded.as_mut(),
                        &mut state,
                        &owners,
                    )?;
                }
            }
            RecoveryFamily::Memberships => {
                for owner in &physical {
                    cancel.check()?;
                    let rows = PreparedQuery::new(exact("owner", owner.clone()), vec![], vec!["SELECT * FROM compiler_membership WITH INDEX contribution_rows WHERE contribution=$owner".into()])?.stream_transaction(transaction.transaction, transaction.budget)?;
                    let mut rows = rows;
                    let result = async {
                        while let Some(row) = rows.next().await? {
                            cancel.check()?;
                            write_state_row(
                                &mut writer,
                                lctx_native::compiler_blocks::logical_membership(&row)?,
                                encoded.as_mut(),
                                &mut state,
                                &owners,
                            )?;
                        }
                        Ok(())
                    }
                    .await;
                    let mut completion = lctx_model::domain::completion::Completion::default();
                    completion.step(
                        "snapshot transport membership drainage",
                        rows.drain_transport().await,
                    );
                    lctx_model::domain::completion::complete(result, completion)?;
                }
            }
            RecoveryFamily::GraphPayloads | RecoveryFamily::TypedBacking => {
                for table in lctx_compiler_store::compiler::logical_recovery_tables(family) {
                    let mut rows =
                        selected.rows(table, "true", Variables::new(), vec![], "id", None)?;
                    let result = async {
                        while let Some(row) = rows.next().await? {
                            cancel.check()?;
                            if family == RecoveryFamily::TypedBacking
                                && let Some(Value::Object(body)) = object(&row)?.get("body")
                                && let Some(Value::RecordId(source)) = body.get("original")
                            {
                                if source.table.as_str() != "original" {
                                    return Err(ModelError::Schema(
                                        "snapshot original backing source",
                                    ));
                                }
                                if !originals.contains(source) {
                                    descriptor_charge
                                        .try_resize(descriptor_charge.size().saturating_add(512))?;
                                    originals.insert(source.clone());
                                }
                            }
                            write_state_row(
                                &mut writer,
                                row,
                                encoded.as_mut(),
                                &mut state,
                                &owners,
                            )?;
                        }
                        Ok(())
                    }
                    .await;
                    let mut completion = lctx_model::domain::completion::Completion::default();
                    completion.step(
                        "snapshot canonical/backing drainage",
                        rows.drain_transport().await,
                    );
                    lctx_model::domain::completion::complete(result, completion)?;
                }
            }
            RecoveryFamily::GraphRoles | RecoveryFamily::ExternalEndpoints => {
                if role_claims.is_none() {
                    role_claims =
                        Some(capture_claims(selected.recovery_roles()?, budget, cancel).await?);
                }
                write_claim_family(
                    role_claims.as_ref().unwrap(),
                    family,
                    &mut writer,
                    encoded.as_mut(),
                    cancel,
                )?;
            }
            RecoveryFamily::SearchOccurrences
            | RecoveryFamily::SearchDocuments
            | RecoveryFamily::SearchVectors => {
                if search_claims.is_none() {
                    // Binding-key order and multiplicity are part of the binding
                    // inventory, not the selector set consumed by the reader.
                    let mut roots = bindings
                        .iter()
                        .filter(|binding| binding.boundary.is_none())
                        .map(|binding| binding.view.identity)
                        .collect::<Vec<_>>();
                    roots.sort_unstable();
                    roots.dedup();
                    let serving = SelectedPayloads::for_published_transaction(
                        transaction.transaction.clone(),
                        completed,
                        &roots,
                        cancel.flag(),
                        budget,
                    )
                    .await?
                    .with_derivation(handle.realization);
                    search_claims =
                        Some(capture_claims(serving.recovery_search()?, budget, cancel).await?);
                }
                write_claim_family(
                    search_claims.as_ref().unwrap(),
                    family,
                    &mut writer,
                    encoded.as_mut(),
                    cancel,
                )?;
            }
            RecoveryFamily::Aliases => {
                let mut aliases = selected.aliases()?;
                while let Some(row) = aliases.next_row()? {
                    cancel.check()?;
                    write_state_row(&mut writer, row, encoded.as_mut(), &mut state, &owners)?;
                }
            }
            RecoveryFamily::Bindings => {
                let mut written = BTreeSet::new();
                for binding in closure.bindings() {
                    // The published completion retains this exact full descriptor,
                    // independent of every source attempt's replaceable slot incidence.
                    let stored = one(
                        transaction,
                        "SELECT * FROM $binding".into(),
                        exact("binding", lctx_native::binding_descriptor::record(binding)?),
                    )
                    .await?;
                    if lctx_native::binding_descriptor::decode(&stored)? != *binding {
                        return Err(ModelError::Conflict("snapshot canonical binding changed"));
                    }
                    // Portable binding rows are data, not installation witnesses. The
                    // destination independently rebinds and earns physical completion.
                    let row = lctx_native::binding_descriptor::portable_row(binding)?;
                    if written.insert(id(&row)?) {
                        write_state_row(&mut writer, row, encoded.as_mut(), &mut state, &owners)?;
                    }
                }
            }
            RecoveryFamily::OriginalHeaders => {
                for source in &originals {
                    cancel.check()?;
                    let header = one(
                        transaction,
                        "SELECT * FROM $source".into(),
                        exact("source", source.clone()),
                    )
                    .await?;
                    let layout = lctx_native::originals::decode_layout(header.clone())?;
                    let artifact = one(
                        transaction,
                        "SELECT * FROM $source".into(),
                        exact("source", layout.artifact.clone()),
                    )
                    .await?;
                    let entity: Entity =
                        serde_json::from_slice(match object(&artifact)?.get("canonical") {
                            Some(Value::Bytes(bytes)) => bytes,
                            _ => {
                                return Err(ModelError::Schema(
                                    "snapshot original canonical payload",
                                ));
                            }
                        })
                        .map_err(ModelError::codec)?;
                    entity.validate()?;
                    let Entity::Source(source_row) = entity else {
                        return Err(ModelError::Schema("snapshot original source kind"));
                    };
                    layout.validate_source(&source_row)?;
                    if !blobs.contains_key(&layout.blob.id()) {
                        descriptor_charge
                            .try_resize(descriptor_charge.size().saturating_add(1024))?;
                        let blob_header = one(
                            transaction,
                            "SELECT * FROM $source".into(),
                            exact("source", layout.blob.id()),
                        )
                        .await?;
                        let (blob, completed) =
                            lctx_native::originals::decode_blob(blob_header.clone())?;
                        if blob != layout.blob || !completed {
                            return Err(ModelError::Conflict("snapshot original blob incomplete"));
                        }
                        backup_import::write_row(&mut writer, blob_header, encoded.as_mut())?;
                        blobs.insert(blob.id(), blob);
                    }
                    backup_import::write_row(&mut writer, header, encoded.as_mut())?;
                }
            }
            RecoveryFamily::OriginalChunks => {
                for blob in blobs.keys() {
                    cancel.check()?;
                    let rows = PreparedQuery::new(exact("blob",blob.clone()),vec![],vec!["SELECT * FROM original_chunk WITH INDEX position WHERE blob=$blob ORDER BY start".into()])?.stream_transaction(transaction.transaction, transaction.budget)?;
                    write_stream(&mut writer, encoded.as_mut(), rows, cancel).await?;
                }
            }
        }
    }
    cancel.check()?;
    super::restore::finish_spool(writer)?;
    let state = state
        .into_iter()
        .map(|rows| rows.finish().map(|rows| rows.into_prepared()))
        .collect::<Result<Vec<_>, _>>()?
        .try_into()
        .map_err(|_| ModelError::Schema("snapshot six completed families"))?;
    let receipt = lctx_native::completed_transport::write_transport(
        spools.completed_state,
        &transport,
        &state,
        budget,
    )?;
    Ok((manifest.content(), receipt))
}
fn write_state_row(
    writer: &mut impl Write,
    row: Value,
    charge: &mut dyn Reservation,
    state: &mut [lctx_native::ordered_rows::SortedRows],
    owners: &BTreeMap<RecordId, CompletedContribution>,
) -> Result<(), ModelError> {
    let row_id = id(&row)?;
    if let Some(index) = lctx_compiler_store::compiler::STATE_TABLES
        .iter()
        .position(|table| *table == row_id.table.as_str())
    {
        let table = lctx_compiler_store::compiler::STATE_TABLES[index];
        state[index].push(lctx_native::completed_logical_rows::normalize(
            table, &row, owners,
        )?)?;
    }
    backup_import::write_row(writer, row, charge)
}

fn content_definition(sql: &str) -> Result<bool, ModelError> {
    use surrealdb_sql::{Expr, TopLevelExpr, statements::DefineStatement};
    let parsed = surrealdb_syn::parse(sql).map_err(ModelError::codec)?;
    let Some(TopLevelExpr::Expr(Expr::Define(definition))) = parsed.expressions.first() else {
        return Err(ModelError::Schema("snapshot definition grammar"));
    };
    let table = match definition.as_ref() {
        DefineStatement::Table(table) => &table.name,
        DefineStatement::Field(field) => &field.what,
        DefineStatement::Index(index) => &index.what,
        _ => return Ok(false),
    };
    let Expr::Table(table) = table else {
        return Err(ModelError::Schema("snapshot definition table"));
    };
    let table = table.as_str();
    Ok(lctx_compiler_store::compiler::classify_recovery_table(table).is_some())
}
/// One actual native read feeds reusable disk-backed family cursors. Related semantic
/// families do not repeat native nomination or retain an in-memory payload collection.
async fn capture_claims(
    mut rows: NativeRows,
    budget: &ResourceBudget,
    cancel: &crate::owned_read::Cancellation,
) -> Result<lctx_native::ordered_rows::PreparedRows, ModelError> {
    let mut sorted = lctx_native::ordered_rows::SortedRows::with_budget(budget)?;
    let result = async {
        while let Some(row) = rows.next().await? {
            cancel.check()?;
            let codec = backup_import::table_codec(id(&row)?.table.as_str())?;
            let lctx_compiler_store::compiler::RecoveryTableKind::Content(family) = codec.kind
            else {
                return Err(ModelError::Schema("actual claim publication root"));
            };
            if !family.is_integrity_claim() {
                return Err(ModelError::Schema("actual claim content family"));
            }
            sorted.push(row)?;
        }
        Ok(())
    }
    .await;
    let mut completion = lctx_model::domain::completion::Completion::default();
    completion.step(
        "snapshot actual claims drainage",
        rows.drain_transport().await,
    );
    lctx_model::domain::completion::complete(result, completion)?;
    Ok(sorted.finish()?.into_prepared())
}
fn write_claim_family(
    rows: &lctx_native::ordered_rows::PreparedRows,
    family: RecoveryFamily,
    writer: &mut impl Write,
    charge: &mut dyn Reservation,
    cancel: &crate::owned_read::Cancellation,
) -> Result<(), ModelError> {
    let mut rows = rows.cursor()?;
    while let Some(row) = rows.next_row()? {
        cancel.check()?;
        if backup_import::table_codec(id(&row)?.table.as_str())?.kind
            == lctx_compiler_store::compiler::RecoveryTableKind::Content(family)
        {
            backup_import::write_row(writer, row, charge)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn view_record(view: &CompletedView) -> Value {
        let mut row = Object::new();
        row.insert("id", RecordId::new("compiler_view", view.identity.hex()));
        row.insert(
            "descriptor",
            Value::Bytes(serde_json::to_vec(view).unwrap().into()),
        );
        Value::Object(row)
    }
    #[tokio::test]
    async fn snapshot_closure_diamond_reuses_dependency_reads_during_actual_traversal() {
        use lctx_model::domain::{
            Relation,
            analysis::sources::SourceSnapshot,
            completed::{ContributionSpec, OutputContent, OutputProfile},
            input::Package,
            producer_assurance::ProducerAssurance,
            stages::Profile,
        };
        let model = ContentHash::of(b"diamond-model");
        let contribution = |name: &str, inputs| CompletedContribution {
            assurance: ProducerAssurance::CurrentProviderInterface,
            spec: ContributionSpec {
                captured_binding: None,
                producer: name.into(),
                profile: OutputProfile::Specific(Profile::Catalog),
                model,
                implementation: ContentHash::of(b"diamond-implementation"),
                configuration: None,
                inputs,
                outputs: ["packages".into()].into(),
            },
            outcome: 0,
            outputs: [(
                "packages".into(),
                OutputContent {
                    rows: 0,
                    content: ContentHash::of(b"empty"),
                },
            )]
            .into(),
        };
        let make_view = |contribution: &CompletedContribution| {
            CompletedView::new(
                "packages".into(),
                [contribution.identity().unwrap()].into(),
                0,
            )
            .unwrap()
        };
        let source = |view: &CompletedView| {
            SourceSnapshot::of_completed_view(&Relation::of::<Package>(), model, view).unwrap()
        };
        let seed = contribution("shared", vec![]);
        let shared = make_view(&seed);
        let left = contribution("left", vec![source(&shared)]);
        let left_view = make_view(&left);
        let right = contribution("right", vec![source(&shared)]);
        let right_view = make_view(&right);
        let root = contribution("root", vec![source(&left_view), source(&right_view)]);
        let root_view = make_view(&root);
        let descriptors = [&seed, &left, &right, &root]
            .into_iter()
            .map(|descriptor| (descriptor.identity().unwrap(), descriptor))
            .collect();
        let raw = [&shared, &left_view, &right_view, &root_view]
            .into_iter()
            .map(|view| (view.identity, view_record(view)))
            .collect::<BTreeMap<_, _>>();
        let budget = ResourceBudget::fixed(1 << 20).unwrap();
        let mut views = SnapshotViews::new(&budget).unwrap();
        let mut acquired = BTreeMap::<ContentHash, usize>::new();
        let closure = snapshot_closure(
            &mut views,
            RecoveryTraversal::for_views(vec![root_view]).unwrap(),
            &descriptors,
            &crate::owned_read::Cancellation::default(),
            |identity| {
                *acquired.entry(identity).or_default() += 1;
                std::future::ready(
                    raw.get(&identity)
                        .cloned()
                        .ok_or(ModelError::Conflict("missing diamond dependency")),
                )
            },
        )
        .await
        .unwrap();
        assert_eq!(closure.views().len(), 4);
        assert_eq!(closure.contributions().len(), 4);
        assert_eq!(
            acquired,
            raw.keys().map(|identity| (*identity, 1)).collect()
        );
        assert_eq!(
            views
                .rows
                .iter()
                .map(|(identity, (row, _))| (*identity, row.clone()))
                .collect::<BTreeMap<_, _>>(),
            raw
        );
    }
    #[tokio::test]
    async fn snapshot_descriptor_reuse_retains_exact_rows_and_is_snapshot_local() {
        let budget = ResourceBudget::fixed(1 << 20).unwrap();
        let mut views = SnapshotViews::new(&budget).unwrap();
        let descriptors = ["root", "left", "right", "shared"].map(|name| {
            CompletedView::new(name.into(), [ContentHash::of(name.as_bytes())].into(), 0).unwrap()
        });
        let raw = descriptors
            .iter()
            .map(|view| (view.identity, view_record(view)))
            .collect::<BTreeMap<_, _>>();
        let mut acquired = BTreeMap::<ContentHash, usize>::new();
        // Root traversal discovers left/right; both branches discover shared. Each
        // eventual traversal/export request must reuse the dependency acquisition.
        for index in [0, 1, 2, 1, 3, 2, 3, 3] {
            let expected = &descriptors[index];
            let actual = views
                .acquire(expected.identity, |identity| {
                    *acquired.entry(identity).or_default() += 1;
                    std::future::ready(Ok(raw[&identity].clone()))
                })
                .await
                .unwrap();
            assert_eq!(&actual.0, &raw[&expected.identity]);
            assert_eq!(&actual.1, expected);
        }
        assert_eq!(
            acquired,
            descriptors.iter().map(|view| (view.identity, 1)).collect()
        );
        assert_eq!(views.rows.len(), 4);
        assert!(budget.reserved() > 0);
        drop(views);
        assert_eq!(
            budget.reserved(),
            0,
            "snapshot consumer owns the entire retained map"
        );
        let mut independent = SnapshotViews::new(&budget).unwrap();
        independent
            .acquire(descriptors[3].identity, |identity| {
                *acquired.entry(identity).or_default() += 1;
                std::future::ready(Ok(raw[&identity].clone()))
            })
            .await
            .unwrap();
        assert_eq!(
            acquired[&descriptors[3].identity], 2,
            "another snapshot earns its own row"
        );
    }
    #[tokio::test]
    async fn snapshot_descriptor_failures_never_install_a_reusable_entry() {
        let view =
            CompletedView::new("fixture".into(), [ContentHash::of(b"copy")].into(), 0).unwrap();
        let budget = ResourceBudget::fixed(1 << 20).unwrap();
        let mut views = SnapshotViews::new(&budget).unwrap();
        assert!(
            views
                .acquire(view.identity, |_| std::future::ready(Err(
                    ModelError::Conflict("missing dependency")
                )))
                .await
                .is_err()
        );
        let mut unequal = view.clone();
        unequal.rows = 1; // Same alleged identity, unequal descriptor.
        assert!(
            views
                .acquire(view.identity, |_| std::future::ready(Ok(view_record(
                    &unequal
                ))))
                .await
                .is_err()
        );
        let wrong = Value::Object({
            let mut row = object(&view_record(&view)).unwrap().clone();
            row.insert("id", RecordId::new("compiler_view", "foreign"));
            row
        });
        assert!(
            views
                .acquire(view.identity, |_| std::future::ready(Ok(wrong)))
                .await
                .is_err()
        );
        assert!(views.rows.is_empty());
        assert_eq!(budget.reserved(), 0);
        views
            .acquire(view.identity, |_| {
                std::future::ready(Ok(view_record(&view)))
            })
            .await
            .unwrap();
        let tiny = ResourceBudget::fixed(1).unwrap();
        let mut refused = SnapshotViews::new(&tiny).unwrap();
        assert!(
            refused
                .acquire(view.identity, |_| std::future::ready(Ok(view_record(
                    &view
                ))))
                .await
                .is_err()
        );
        assert!(refused.rows.is_empty());
        assert_eq!(tiny.reserved(), 0);
    }
    #[test]
    fn family_dispatch_exports_every_actual_claim_without_expected_row_filtering() {
        let budget = ResourceBudget::fixed(256 << 20).unwrap();
        let mut sorted = lctx_native::ordered_rows::SortedRows::with_budget(&budget).unwrap();
        let mut expected = Vec::new();
        for family in RECOVERY_FAMILIES
            .iter()
            .copied()
            .filter(|family| family.is_integrity_claim())
        {
            for table in lctx_compiler_store::compiler::logical_recovery_tables(family) {
                let mut object = Object::new();
                object.insert("id", RecordId::new(*table, "unexpected-actual-claim"));
                object.insert("field", "forged-role");
                object.insert("deps", Vec::<RecordId>::new());
                let row = Value::Object(object);
                expected.push(row.clone());
                sorted.push(row).unwrap();
            }
        }
        let rows = sorted.finish().unwrap().into_prepared();
        let cancel = crate::owned_read::Cancellation::default();
        let mut charge = budget.reserve("family-export-control", 0).unwrap();
        let mut encoded = b"OPTION IMPORT;\n".to_vec();
        for family in RECOVERY_FAMILIES
            .iter()
            .copied()
            .filter(|family| family.is_integrity_claim())
        {
            write_claim_family(&rows, family, &mut encoded, charge.as_mut(), &cancel).unwrap();
        }
        let mut decoded = backup_import::DataDump::new(encoded.as_slice());
        let mut actual = Vec::new();
        while let Some(item) = decoded.next().unwrap() {
            let backup_import::Item::Rows(rows) = item else {
                panic!("data only");
            };
            actual.extend(rows);
        }
        assert_eq!(
            actual, expected,
            "all source anomalies survive family export and local decoding"
        );
    }
    #[tokio::test(flavor = "multi_thread")]
    async fn selected_snapshot_cancelled_delivery_joins_late_protocol_failures_before_same_session_cancel()
     {
        use crate::backup::grpc_export_fixture::{Fixture, QueryFault};
        for fault in [
            QueryFault::Success,
            QueryFault::LateStatementError,
            QueryFault::OuterError,
            QueryFault::MissingOuterEnd,
            QueryFault::LateTransportError,
            QueryFault::LatePayload,
        ] {
            let fixture = Fixture::start_query(fault).await;
            let session = lctx_native::reader::connect(
                &fixture.endpoint,
                &lctx_native::Credentials::Root {
                    username: "fixture".into(),
                    password: "fixture".into(),
                },
                "injected_export",
                "fixture",
            )
            .await
            .unwrap();
            let transaction = Arc::new(
                Arc::try_unwrap(session)
                    .unwrap()
                    .begin()
                    .retain_on_error()
                    .await
                    .map_err(|(error, _)| error)
                    .unwrap(),
            );
            let (row_seen, row_ready) = tokio::sync::oneshot::channel();
            let (finished, terminal) = tokio::sync::oneshot::channel();
            let waiter = tokio::spawn(crate::owned_read::run(
                "selected snapshot protocol control",
                move |cancel| async move {
                    let budget =
                        ResourceBudget::fixed(lctx_model::domain::resources::DEFAULT_MEMORY_BYTES)
                            .unwrap();
                    let snapshot = Snapshot {
                        transaction: &transaction,
                        cancel: &cancel,
                        budget: &budget,
                    };
                    let mut row_seen = Some(row_seen);
                    let result = read(
                        &snapshot,
                        "SELECT * FROM selected_control".into(),
                        Variables::new(),
                        |_row| {
                            if let Some(seen) = row_seen.take() {
                                let _ = seen.send(());
                            }
                            Ok(())
                        },
                    )
                    .await
                    .and_then(|()| cancel.check());
                    let mut completion = lctx_model::domain::completion::Completion::default();
                    completion.step(
                        "composed selected cancel",
                        transaction.cancel_ref().await.map_err(ModelError::codec),
                    );
                    completion.step(
                        "composed selected invalidate",
                        transaction
                            .invalidate_session()
                            .await
                            .map_err(ModelError::codec),
                    );
                    let result = lctx_model::domain::completion::complete(result, completion);
                    finished.send(format!("{result:?}")).unwrap();
                    result
                },
            ));
            row_ready.await.unwrap();
            fixture.provisional.notified().await;
            waiter.abort();
            assert!(waiter.await.unwrap_err().is_cancelled());
            assert_eq!(
                *fixture.events.lock().unwrap(),
                ["begin", "query"],
                "no terminal release before physical tail for {fault:?}"
            );
            fixture.release_terminal();
            let outcome = terminal.await.unwrap();
            assert!(
                outcome.starts_with("Err("),
                "cancelled delivery cannot publish success: {fault:?}: {outcome}"
            );
            let fragment = match fault {
                QueryFault::Success => "native read delivery cancelled",
                QueryFault::LateStatementError => "injected late selected statement failure",
                QueryFault::OuterError => "injected selected outer failure",
                QueryFault::MissingOuterEnd => "end",
                QueryFault::LateTransportError => "injected selected late physical status",
                QueryFault::LatePayload => "after its end",
            };
            assert!(
                outcome.contains(fragment),
                "late source failure lost for {fault:?}: {outcome}"
            );
            assert_eq!(
                *fixture.events.lock().unwrap(),
                [
                    "begin",
                    "query",
                    "physical-tail-sent",
                    "cancel",
                    "invalidate"
                ]
            );
            fixture.close().await;
        }
    }
    #[tokio::test(flavor = "multi_thread")]
    async fn selected_external_snapshot_pins_metadata_and_rows_then_acknowledges_cancel() {
        let config = lctx_native::RuntimeConfig::read(std::path::Path::new(
            &std::env::var_os("LCTX_COMPILER_RUNTIME_CONFIG").expect("stable validation runtime"),
        ))
        .unwrap();
        let writer = lctx_compiler_store::compiler::check_installation(&config)
            .await
            .unwrap();
        let nonce = lctx_native::control::fresh_identity("selected-snapshot-control").unwrap();
        let source = RecordId::new("native_guard", format!("snapshot_{}", nonce.hex()));
        let vars = exact("source", source.clone());
        writer.query("CREATE $source SET revision=0,retired=false,phase='active',incarnation=1 RETURN NONE").bind(vars.clone()).await.unwrap().check().unwrap();
        let session = lctx_native::reader::connect(
            &config.endpoint,
            &config.writer_credentials(),
            config.namespace.as_str(),
            config.database.as_str(),
        )
        .await
        .unwrap();
        let session = Arc::try_unwrap(session).unwrap();
        let transaction = Arc::new(
            session
                .begin()
                .retain_on_error()
                .await
                .map_err(|(error, _)| error)
                .unwrap(),
        );
        let cancel = crate::owned_read::Cancellation::default();
        let budget =
            ResourceBudget::fixed(lctx_model::domain::resources::DEFAULT_MEMORY_BYTES).unwrap();
        let snapshot = Snapshot {
            transaction: &transaction,
            cancel: &cancel,
            budget: &budget,
        };
        let result: Result<(), ModelError> = async {
            let metadata = one(
                &snapshot,
                "INFO FOR TABLE native_guard".into(),
                Variables::new(),
            )
            .await?;
            let before = one(&snapshot, "SELECT * FROM $source".into(), vars.clone()).await?;
            writer
                .query("UPDATE $source SET revision=1 RETURN NONE")
                .bind(vars.clone())
                .await
                .map_err(ModelError::codec)?
                .check()
                .map_err(ModelError::codec)?;
            assert_eq!(
                one(&snapshot, "SELECT * FROM $source".into(), vars.clone()).await?,
                before,
                "all fixed reads must share the external transaction snapshot"
            );
            assert_eq!(
                one(
                    &snapshot,
                    "INFO FOR TABLE native_guard".into(),
                    Variables::new()
                )
                .await?,
                metadata
            );
            Ok(())
        }
        .await;
        let cancelled = transaction.cancel_ref().await;
        assert!(
            cancelled.is_ok(),
            "same-session external cancel must acknowledge: {cancelled:?}"
        );
        let after_cancel = transaction
            .query("SELECT * FROM $source")
            .bind(vars.clone())
            .await;
        assert!(
            after_cancel.map_or(true, |response| response.check().is_err()),
            "cancelled snapshot cannot accept a later read"
        );
        transaction.invalidate_session().await.unwrap();
        writer
            .query("DELETE $source RETURN NONE")
            .bind(vars)
            .await
            .unwrap()
            .check()
            .unwrap();
        writer.invalidate().await.unwrap();
        result.unwrap();
    }
    #[tokio::test(flavor = "multi_thread")]
    async fn selected_external_snapshot_late_statement_failure_is_checked_before_cancel() {
        let config = lctx_native::RuntimeConfig::read(std::path::Path::new(
            &std::env::var_os("LCTX_COMPILER_RUNTIME_CONFIG").expect("stable validation runtime"),
        ))
        .unwrap();
        let client = lctx_native::reader::connect(
            &config.endpoint,
            &config.writer_credentials(),
            config.namespace.as_str(),
            config.database.as_str(),
        )
        .await
        .unwrap();
        let transaction = Arc::new(
            Arc::try_unwrap(client)
                .unwrap()
                .begin()
                .retain_on_error()
                .await
                .map_err(|(error, _)| error)
                .unwrap(),
        );
        let mut rows = PreparedQuery::new(
            Variables::new(),
            vec![],
            vec![
                "RETURN 1".into(),
                "THROW 'selected snapshot late engine failure'".into(),
            ],
        )
        .unwrap()
        .stream_transaction(
            &transaction,
            &ResourceBudget::fixed(lctx_model::domain::resources::DEFAULT_MEMORY_BYTES).unwrap(),
        )
        .unwrap();
        assert!(rows.next().await.unwrap().is_some());
        let failure = rows
            .next()
            .await
            .expect_err("late engine statement failure cannot be accepted as EOF");
        assert!(
            failure
                .to_string()
                .contains("selected snapshot late engine failure")
        );
        let _terminal = rows.drain_transport().await;
        transaction
            .cancel_ref()
            .await
            .expect("failed fixed read retains same-session cancellation route");
        transaction.invalidate_session().await.unwrap();
    }
}
