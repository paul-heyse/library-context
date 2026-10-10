//! Fixed read-only lowering for one external native snapshot. No request-controlled SQL,
//! operational authority, service-wide content scan or per-page transaction is admitted.
use crate::{backup_import, definitions, inspection};
use lctx_model::domain::{ContentHash, ModelError, completed::{CompletedBinding, CompletedContribution, CompletedView}, recovery_closure::{RecoveryTraversal, RecoveryFamily, RECOVERY_FAMILIES}, resources::{ResourceBudget, Reservation}, serving::SnapshotHandle};
use lctx_surrealdb::{prepared::PreparedQuery, reader::NativeRows, selection::SelectedPayloads, surrealdb::{method::Transaction, engine::remote::grpc::Client, types::{Object, RecordId, SurrealValue, Value, Variables}}};
use std::{collections::{BTreeMap, BTreeSet}, io::Write, path::Path, sync::Arc};

struct Snapshot<'a> { transaction: &'a Arc<Transaction<Client>>, cancel: &'a crate::owned_read::Cancellation }
async fn read(
    transaction: &Snapshot<'_>, sql: String, bindings: Variables,
    mut accept: impl FnMut(Value) -> Result<(), ModelError>,
) -> Result<(), ModelError> {
    transaction.cancel.check()?;
    let mut rows = PreparedQuery::new(bindings, vec![], vec![sql])?.stream_transaction(transaction.transaction)?;
    let result = async { while let Some(row) = rows.next().await? { transaction.cancel.check()?; accept(row)?; } Ok(()) }.await;
    let mut completion = lctx_model::domain::completion::Completion::default(); completion.step("selected backup query drainage", rows.drain_transport().await);
    lctx_model::domain::completion::complete(result, completion)
}
async fn one(transaction: &Snapshot<'_>, sql: String, variables: Variables) -> Result<Value, ModelError> {
    let mut value = None;
    read(transaction, sql, variables, |row| {
        if value.replace(row).is_some() { return Err(ModelError::Conflict("selected backup singleton inventory")); } Ok(())
    }).await?;
    value.ok_or(ModelError::Conflict("selected backup required singleton"))
}
fn object(value: &Value) -> Result<&Object, ModelError> {
    let Value::Object(object) = value else { return Err(ModelError::Schema("selected backup object")); }; Ok(object)
}
fn id(value: &Value) -> Result<RecordId, ModelError> {
    RecordId::from_value(object(value)?.get("id").cloned().ok_or(ModelError::Schema("selected backup record identity"))?).map_err(ModelError::codec)
}
fn descriptor<T: serde::de::DeserializeOwned>(value: &Value) -> Result<T, ModelError> {
    let Some(Value::Bytes(bytes)) = object(value)?.get("descriptor") else { return Err(ModelError::Schema("selected backup descriptor bytes")); };
    serde_json::from_slice(bytes).map_err(ModelError::codec)
}
fn exact(key: &str, value: RecordId) -> Variables { let mut vars = Variables::new(); vars.insert(key, value); vars }
fn definition_groups(object: &Object, group: &str, output: &mut BTreeSet<String>) -> Result<(), ModelError> {
    let Some(Value::Object(group)) = object.get(group) else { return Err(ModelError::Schema("snapshot definition group")); };
    for value in group.values() {
        let Value::String(sql) = value else { return Err(ModelError::Schema("snapshot definition text")); };
        output.insert(definitions::normalize(sql)?);
    }
    Ok(())
}
async fn definition_inventory(transaction: &Snapshot<'_>) -> Result<BTreeSet<String>, ModelError> {
    let database = one(transaction, "INFO FOR DB".into(), Variables::new()).await?;
    let database = object(&database)?;
    let mut inventory = BTreeSet::new();
    for group in ["functions", "analyzers", "tables"] { definition_groups(database, group, &mut inventory)?; }
    let Some(Value::Object(tables)) = database.get("tables") else { return Err(ModelError::Schema("snapshot table metadata")); };
    for name in tables.keys() {
        let name = name.replace('\\', "\\\\").replace('`', "\\`");
        let table = one(transaction, format!("INFO FOR TABLE `{name}`"), Variables::new()).await?;
        for group in ["fields", "indexes"] { definition_groups(object(&table)?, group, &mut inventory)?; }
    }
    Ok(inventory)
}
async fn view(transaction: &Snapshot<'_>, identity: ContentHash) -> Result<(Value, CompletedView), ModelError> {
    let row = one(transaction, "SELECT * FROM $view".into(), exact("view", RecordId::new("compiler_view", identity.hex()))).await?;
    let descriptor: CompletedView = descriptor(&row)?; descriptor.validate()?;
    if descriptor.identity != identity { return Err(ModelError::Conflict("snapshot view identity")); }
    Ok((row, descriptor))
}
async fn write_stream(writer: &mut impl Write, charge: &mut dyn Reservation, mut rows: NativeRows, cancel: &crate::owned_read::Cancellation) -> Result<(), ModelError> {
    let result = async { while let Some(row) = rows.next().await? { cancel.check()?; backup_import::write_row(writer, row, charge)?; } Ok(()) }.await;
    let mut completion = lctx_model::domain::completion::Completion::default(); completion.step("selected backup content drainage", rows.drain_transport().await);
    lctx_model::domain::completion::complete(result, completion)
}
/// Write one coherent provisional data dump. The caller drains/cancels this same transaction
/// before any file publication and retains the exact pin through cancellation acknowledgement.
pub(crate) async fn export(
    transaction: &Arc<Transaction<Client>>, version: &str, handle: &SnapshotHandle,
    native_definitions: &str, output: &Path, budget: &ResourceBudget, cancel: &crate::owned_read::Cancellation,
) -> Result<(), ModelError> {
    cancel.check()?;
    let snapshot = Snapshot { transaction, cancel }; let transaction = &snapshot;
    let publication = one(transaction, "SELECT * FROM $publication".into(), exact("publication", RecordId::new("publication", handle.publication.hex()))).await?;
    let marker = inspection::Marker::from_value(publication.clone()).map_err(ModelError::codec)?;
    let (actual_handle, manifest, bindings) = inspection::decode(marker)?;
    if actual_handle != *handle { return Err(ModelError::Conflict("snapshot exact publication handle")); }
    if manifest.semantic_contract != lctx_model::domain::graph::semantic_contract(&lctx_model::domain::model()?) {
        return Err(ModelError::Conflict("snapshot semantic contract"));
    }
    if definitions::epoch_identity(native_definitions) != handle.definition_epoch { return Err(ModelError::Conflict("snapshot pinned definition epoch")); }
    let inventory = definition_inventory(transaction).await?;
    if definitions::verify_inventory(native_definitions, version, &inventory)? != handle.realization {
        return Err(ModelError::Conflict("snapshot pinned realization"));
    }
    let mut writer = std::io::BufWriter::new(std::fs::OpenOptions::new().write(true).truncate(true).open(output).map_err(ModelError::codec)?);
    writeln!(writer, "OPTION IMPORT;").map_err(ModelError::codec)?;
    let (base, functions) = definitions::expected(native_definitions)?;
    let pinned = base.into_iter().chain(functions).collect::<BTreeSet<_>>();
    // Schema metadata are comparison-only. Exclude runtime controls and other executable epochs.
    for definition in &inventory {
        if pinned.contains(definition) || content_definition(definition)? { writeln!(writer, "{definition};").map_err(ModelError::codec)?; }
    }
    let mut encoded = budget.reserve("selected-backup-encoded-row", 0)?;
    backup_import::write_row(&mut writer, publication, encoded.as_mut())?;
    let mut traversal = RecoveryTraversal::new(bindings.clone())?;
    let mut chosen = BTreeMap::<ContentHash, Value>::new();
    let mut physical = BTreeSet::new();
    let mut view_rows = BTreeMap::new();
    let mut descriptor_charge = budget.reserve("selected-backup-descriptor-closure", 0)?;
    while let Some(requested) = traversal.next_view()? {
        let (row, actual) = view(transaction, requested.identity).await?;
        descriptor_charge.try_resize(descriptor_charge.size().saturating_add(serde_json::to_vec(&row).map_err(ModelError::codec)?.len().saturating_mul(4)))?;
        view_rows.insert(actual.identity, row);
        let mut contributors = Vec::new(); let mut inputs = BTreeMap::new();
        for logical in &actual.contributions {
            if !chosen.contains_key(logical) {
                let mut vars = Variables::new(); vars.insert("logical", logical.hex());
                let mut selected: Option<Value> = None; let mut known: Option<CompletedContribution> = None;
                read(transaction, "SELECT * FROM compiler_contribution WITH INDEX logical_contribution WHERE logical=$logical AND completed=true ORDER BY id".into(), vars, |row| {
                    let descriptor: CompletedContribution = descriptor(&row)?;
                    if descriptor.identity()? != *logical || known.as_ref().is_some_and(|old| old != &descriptor) { return Err(ModelError::Conflict("snapshot logical contributor collision")); }
                    known = Some(descriptor);
                    if selected.is_none() { selected = Some(row); } Ok(())
                }).await?;
                let row = selected.ok_or(ModelError::Conflict("snapshot dependency contributor absent"))?;
                descriptor_charge.try_resize(descriptor_charge.size().saturating_add(serde_json::to_vec(&row).map_err(ModelError::codec)?.len().saturating_mul(4)))?;
                physical.insert(id(&row)?); chosen.insert(*logical, row);
            }
            let descriptor: CompletedContribution = descriptor(&chosen[logical])?;
            for input in &descriptor.spec.inputs { if !inputs.contains_key(&input.view()) { inputs.insert(input.view(), view(transaction, input.view()).await?.1); } }
            contributors.push(descriptor);
        }
        traversal.include_view(&requested, actual, contributors, |id| inputs.get(&id).cloned().ok_or(ModelError::Conflict("snapshot dependency view absent")))?;
    }
    let closure = traversal.finish()?;
    let owners = physical.iter().map(|owner| {
        let lctx_surrealdb::surrealdb::types::RecordIdKey::String(key) = &owner.key else { return Err(ModelError::Schema("snapshot contributor key")); };
        Ok(ContentHash(hex::decode(key).map_err(ModelError::codec)?.try_into().map_err(|_| ModelError::Schema("snapshot contributor hash"))?))
    }).collect::<Result<Vec<_>, _>>()?;
    let selected = SelectedPayloads::for_cancellable_transaction(transaction.transaction.clone(), &owners, true, cancel.flag(), budget).await?;
    let mut originals = manifest.originals.iter().map(|original| RecordId::new("original", original.source.0.hex())).collect::<BTreeSet<_>>();
    let mut role_claims = None;
    let mut search_claims = None;
    // This match is the actual family dispatcher, not a coverage assertion over table names.
    // A new model family enters this loop and requires an explicit export treatment.
    for family in RECOVERY_FAMILIES.iter().copied() {
        cancel.check()?;
        match family {
            RecoveryFamily::Views => for row in view_rows.values() { backup_import::write_row(&mut writer, row.clone(), encoded.as_mut())?; },
            RecoveryFamily::Contributions => for row in chosen.values() { backup_import::write_row(&mut writer, row.clone(), encoded.as_mut())?; },
            RecoveryFamily::Memberships => for owner in &physical {
                cancel.check()?;
                let rows = PreparedQuery::new(exact("owner", owner.clone()), vec![], vec!["SELECT * FROM compiler_membership WITH INDEX contribution_rows WHERE contribution=$owner".into()])?.stream_transaction(transaction.transaction)?;
                write_stream(&mut writer, encoded.as_mut(), rows, cancel).await?;
            },
            RecoveryFamily::GraphPayloads | RecoveryFamily::TypedBacking => {
                for table in lctx_surrealdb::compiler::recovery_tables(family) {
                    let mut rows = selected.rows(table, "true", Variables::new(), vec![], "id", None)?;
                    let result = async { while let Some(row) = rows.next().await? {
                        cancel.check()?;
                        if family == RecoveryFamily::TypedBacking { if let Some(Value::Object(body)) = object(&row)?.get("body") { if let Some(Value::RecordId(source)) = body.get("original") {
                            if source.table.as_str() != "original" { return Err(ModelError::Schema("snapshot original backing source")); }
                            if !originals.contains(source) { descriptor_charge.try_resize(descriptor_charge.size().saturating_add(512))?; originals.insert(source.clone()); }
                        } } }
                        backup_import::write_row(&mut writer, row, encoded.as_mut())?;
                    } Ok(()) }.await;
                    let mut completion = lctx_model::domain::completion::Completion::default(); completion.step("snapshot canonical/backing drainage", rows.drain_transport().await); lctx_model::domain::completion::complete(result, completion)?;
                }
            },
            RecoveryFamily::GraphRoles | RecoveryFamily::ExternalEndpoints => {
                if role_claims.is_none() { role_claims = Some(capture_claims(selected.recovery_roles()?, budget, cancel).await?); }
                write_claim_family(role_claims.as_ref().unwrap(), family, &mut writer, encoded.as_mut(), cancel)?;
            },
            RecoveryFamily::SearchOccurrences | RecoveryFamily::SearchDocuments | RecoveryFamily::SearchVectors => {
                if search_claims.is_none() {
                    let roots = bindings.iter().filter(|binding| binding.boundary.is_none()).map(|binding| binding.view.identity).collect::<Vec<_>>();
                    let serving = SelectedPayloads::for_cancellable_transaction(transaction.transaction.clone(), &roots, false, cancel.flag(), budget).await?;
                    search_claims = Some(capture_claims(serving.recovery_search()?, budget, cancel).await?);
                }
                write_claim_family(search_claims.as_ref().unwrap(), family, &mut writer, encoded.as_mut(), cancel)?;
            },
            RecoveryFamily::Aliases => {
                let mut aliases = selected.aliases()?;
                while let Some(row) = aliases.next_row()? { cancel.check()?; backup_import::write_row(&mut writer, row, encoded.as_mut())?; }
            },
            RecoveryFamily::Bindings => {
                let mut written = BTreeSet::new();
                for binding in closure.bindings() {
                    let mut candidate = None;
                    read(transaction, "SELECT * FROM compiler_binding WITH INDEX binding_view WHERE view=$view ORDER BY id".into(), exact("view", RecordId::new("compiler_view", binding.view.identity.hex())), |row| {
                        if descriptor::<CompletedBinding>(&row)? == *binding && candidate.is_none() { candidate = Some(row); } Ok(())
                    }).await?;
                    let row = candidate.ok_or(ModelError::Conflict("snapshot required binding absent"))?;
                    if written.insert(id(&row)?) { backup_import::write_row(&mut writer, row, encoded.as_mut())?; }
                }
            },
            RecoveryFamily::OriginalHeaders => for source in &originals {
                let header = one(transaction, "SELECT * FROM $source".into(), exact("source", source.clone())).await?;
                backup_import::write_row(&mut writer, header, encoded.as_mut())?;
            },
            RecoveryFamily::OriginalChunks => for source in &originals {
                cancel.check()?;
                let rows = PreparedQuery::new(exact("source", source.clone()), vec![], vec!["SELECT * FROM original_chunk WITH INDEX position WHERE source=$source ORDER BY start".into()])?.stream_transaction(transaction.transaction)?;
                write_stream(&mut writer, encoded.as_mut(), rows, cancel).await?;
            },
        }
    }
    cancel.check()?;
    writer.flush().map_err(ModelError::codec)
}
fn content_definition(sql: &str) -> Result<bool, ModelError> {
    use surrealdb_sql::{Expr, TopLevelExpr, statements::DefineStatement};
    let parsed = surrealdb_syn::parse(sql).map_err(ModelError::codec)?;
    let Some(TopLevelExpr::Expr(Expr::Define(definition))) = parsed.expressions.first() else { return Err(ModelError::Schema("snapshot definition grammar")); };
    let table = match definition.as_ref() {
        DefineStatement::Table(table) => &table.name,
        DefineStatement::Field(field) => &field.what,
        DefineStatement::Index(index) => &index.what,
        _ => return Ok(false),
    };
    let Expr::Table(table) = table else { return Err(ModelError::Schema("snapshot definition table")); };
    let table = table.as_str();
    Ok(lctx_surrealdb::compiler::classify_recovery_table(table).is_some())
}
/// One actual native read feeds reusable disk-backed family cursors. Related semantic
/// families do not repeat native nomination or retain an in-memory payload collection.
async fn capture_claims(mut rows: NativeRows, budget: &ResourceBudget, cancel: &crate::owned_read::Cancellation) -> Result<lctx_surrealdb::ordered_rows::PreparedRows, ModelError> {
    let mut sorted = lctx_surrealdb::ordered_rows::SortedRows::with_budget(budget)?;
    let result = async { while let Some(row) = rows.next().await? {
        cancel.check()?;
        let codec = backup_import::table_codec(id(&row)?.table.as_str())?;
        let lctx_surrealdb::compiler::RecoveryTableKind::Content(family) = codec.kind else { return Err(ModelError::Schema("actual claim publication root")); };
        if !family.is_integrity_claim() { return Err(ModelError::Schema("actual claim content family")); }
        sorted.push(row)?;
    } Ok(()) }.await;
    let mut completion = lctx_model::domain::completion::Completion::default(); completion.step("snapshot actual claims drainage", rows.drain_transport().await);
    lctx_model::domain::completion::complete(result, completion)?;
    Ok(sorted.finish()?.into_prepared())
}
fn write_claim_family(rows: &lctx_surrealdb::ordered_rows::PreparedRows, family: RecoveryFamily, writer: &mut impl Write, charge: &mut dyn Reservation, cancel: &crate::owned_read::Cancellation) -> Result<(), ModelError> {
    let mut rows = rows.cursor()?;
    while let Some(row) = rows.next_row()? {
        cancel.check()?;
        if backup_import::table_codec(id(&row)?.table.as_str())?.kind == lctx_surrealdb::compiler::RecoveryTableKind::Content(family) {
            backup_import::write_row(writer, row, charge)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn family_dispatch_exports_every_actual_claim_without_expected_row_filtering() {
        let budget = ResourceBudget::fixed(256 << 20).unwrap();
        let mut sorted = lctx_surrealdb::ordered_rows::SortedRows::with_budget(&budget).unwrap();
        let mut expected = Vec::new();
        for family in RECOVERY_FAMILIES.iter().copied().filter(|family| family.is_integrity_claim()) {
            for table in lctx_surrealdb::compiler::recovery_tables(family) {
                let mut object = Object::new(); object.insert("id", RecordId::new(*table, "unexpected-actual-claim"));
                object.insert("field", "forged-role"); object.insert("deps", Vec::<RecordId>::new());
                let row = Value::Object(object); expected.push(row.clone()); sorted.push(row).unwrap();
            }
        }
        let rows = sorted.finish().unwrap().into_prepared();
        let cancel = crate::owned_read::Cancellation::default();
        let mut charge = budget.reserve("family-export-control", 0).unwrap();
        let mut encoded = b"OPTION IMPORT;\n".to_vec();
        for family in RECOVERY_FAMILIES.iter().copied().filter(|family| family.is_integrity_claim()) {
            write_claim_family(&rows, family, &mut encoded, charge.as_mut(), &cancel).unwrap();
        }
        let mut decoded = backup_import::DataDump::new(encoded.as_slice()); let mut actual = Vec::new();
        while let Some(item) = decoded.next().unwrap() { let backup_import::Item::Rows(rows) = item else { panic!("data only"); }; actual.extend(rows); }
        assert_eq!(actual, expected, "all source anomalies survive family export and local decoding");
    }
    #[tokio::test(flavor = "multi_thread")]
    async fn selected_snapshot_cancelled_delivery_joins_late_protocol_failures_before_same_session_cancel() {
        use crate::backup::grpc_export_fixture::{Fixture, QueryFault};
        for fault in [QueryFault::Success, QueryFault::LateStatementError, QueryFault::OuterError, QueryFault::MissingOuterEnd, QueryFault::LateTransportError, QueryFault::LatePayload] {
            let fixture = Fixture::start_query(fault).await;
            let session = lctx_surrealdb::reader::connect(&fixture.endpoint, &lctx_surrealdb::Credentials::Root { username: "fixture".into(), password: "fixture".into() }, "injected_export", "fixture").await.unwrap();
            let transaction = Arc::new(Arc::try_unwrap(session).unwrap().begin().retain_on_error().await.map_err(|(error, _)| error).unwrap());
            let (row_seen, row_ready) = tokio::sync::oneshot::channel();
            let (finished, terminal) = tokio::sync::oneshot::channel();
            let waiter = tokio::spawn(crate::owned_read::run("selected snapshot protocol control", move |cancel| async move {
                let snapshot = Snapshot { transaction: &transaction, cancel: &cancel };
                let mut row_seen = Some(row_seen);
                let result = read(&snapshot, "SELECT * FROM selected_control".into(), Variables::new(), |_row| {
                    if let Some(seen) = row_seen.take() { let _ = seen.send(()); } Ok(())
                }).await.and_then(|()| cancel.check());
                let mut completion = lctx_model::domain::completion::Completion::default();
                completion.step("composed selected cancel", transaction.cancel_ref().await.map_err(ModelError::codec));
                completion.step("composed selected invalidate", transaction.invalidate_session().await.map_err(ModelError::codec));
                let result = lctx_model::domain::completion::complete(result, completion);
                finished.send(format!("{result:?}")).unwrap(); result
            }));
            row_ready.await.unwrap(); fixture.provisional.notified().await;
            waiter.abort(); assert!(waiter.await.unwrap_err().is_cancelled());
            assert_eq!(*fixture.events.lock().unwrap(), ["begin", "query"], "no terminal release before physical tail for {fault:?}");
            fixture.release_terminal();
            let outcome = terminal.await.unwrap();
            assert!(outcome.starts_with("Err("), "cancelled delivery cannot publish success: {fault:?}: {outcome}");
            let fragment = match fault {
                QueryFault::Success => "native read delivery cancelled",
                QueryFault::LateStatementError => "injected late selected statement failure",
                QueryFault::OuterError => "injected selected outer failure",
                QueryFault::MissingOuterEnd => "end",
                QueryFault::LateTransportError => "injected selected late physical status",
                QueryFault::LatePayload => "after its end",
            };
            assert!(outcome.contains(fragment), "late source failure lost for {fault:?}: {outcome}");
            assert_eq!(*fixture.events.lock().unwrap(), ["begin", "query", "physical-tail-sent", "cancel", "invalidate"]);
            fixture.close().await;
        }
    }
    #[tokio::test(flavor = "multi_thread")]
    async fn selected_external_snapshot_pins_metadata_and_rows_then_acknowledges_cancel() {
        let config = lctx_surrealdb::RuntimeConfig::read(std::path::Path::new(&std::env::var_os("LCTX_COMPILER_RUNTIME_CONFIG").expect("stable validation runtime"))).unwrap();
        let writer = lctx_surrealdb::compiler::check_installation(&config).await.unwrap();
        let nonce = lctx_surrealdb::control::fresh_identity("selected-snapshot-control").unwrap();
        let source = RecordId::new("native_guard", format!("snapshot_{}", nonce.hex()));
        let vars = exact("source", source.clone());
        writer.query("CREATE $source SET revision=0,retired=false,phase='active',incarnation=1 RETURN NONE").bind(vars.clone()).await.unwrap().check().unwrap();
        let session = lctx_surrealdb::reader::connect(&config.endpoint, &config.writer_credentials(), config.namespace.as_str(), config.database.as_str()).await.unwrap();
        let session = Arc::try_unwrap(session).unwrap();
        let transaction = Arc::new(session.begin().retain_on_error().await.map_err(|(error, _)| error).unwrap());
        let cancel = crate::owned_read::Cancellation::default(); let snapshot = Snapshot { transaction: &transaction, cancel: &cancel };
        let result: Result<(), ModelError> = async {
            let metadata = one(&snapshot, "INFO FOR TABLE native_guard".into(), Variables::new()).await?;
            let before = one(&snapshot, "SELECT * FROM $source".into(), vars.clone()).await?;
            writer.query("UPDATE $source SET revision=1 RETURN NONE").bind(vars.clone()).await.map_err(ModelError::codec)?.check().map_err(ModelError::codec)?;
            assert_eq!(one(&snapshot, "SELECT * FROM $source".into(), vars.clone()).await?, before, "all fixed reads must share the external transaction snapshot");
            assert_eq!(one(&snapshot, "INFO FOR TABLE native_guard".into(), Variables::new()).await?, metadata);
            Ok(())
        }.await;
        let cancelled = transaction.cancel_ref().await;
        assert!(cancelled.is_ok(), "same-session external cancel must acknowledge: {cancelled:?}");
        let after_cancel = transaction.query("SELECT * FROM $source").bind(vars.clone()).await;
        assert!(after_cancel.and_then(|response| response.check()).is_err(), "cancelled snapshot cannot accept a later read");
        transaction.invalidate_session().await.unwrap();
        writer.query("DELETE $source RETURN NONE").bind(vars).await.unwrap().check().unwrap();
        writer.invalidate().await.unwrap(); result.unwrap();
    }
    #[tokio::test(flavor = "multi_thread")]
    async fn selected_external_snapshot_late_statement_failure_is_checked_before_cancel() {
        let config = lctx_surrealdb::RuntimeConfig::read(std::path::Path::new(&std::env::var_os("LCTX_COMPILER_RUNTIME_CONFIG").expect("stable validation runtime"))).unwrap();
        let client = lctx_surrealdb::reader::connect(&config.endpoint, &config.writer_credentials(), config.namespace.as_str(), config.database.as_str()).await.unwrap();
        let transaction = Arc::new(Arc::try_unwrap(client).unwrap().begin().retain_on_error().await.map_err(|(error, _)| error).unwrap());
        let mut rows = PreparedQuery::new(Variables::new(), vec![], vec!["RETURN 1".into(), "THROW 'selected snapshot late engine failure'".into()]).unwrap().stream_transaction(&transaction).unwrap();
        assert!(rows.next().await.unwrap().is_some());
        let failure = rows.next().await.expect_err("late engine statement failure cannot be accepted as EOF");
        assert!(failure.to_string().contains("selected snapshot late engine failure"));
        let _terminal = rows.drain_transport().await;
        transaction.cancel_ref().await.expect("failed fixed read retains same-session cancellation route");
        transaction.invalidate_session().await.unwrap();
    }
}
