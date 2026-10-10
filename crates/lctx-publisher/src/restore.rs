//! Ordinary restore lowers untrusted dump values through typed native ingress.
//! Temporary files isolate parsing and large data, never a disposable database or raw SQL session.
use crate::{
    backup_import::{DATA_TABLES, DataDump, Item, definition_metadata},
    native_publication::{Admission, Publication},
};
use lctx_model::domain::{
    ContentHash, ModelError,
    completed::{CompletedBinding, CompletedStateHeader},
    graph::{Assertion, Entity, Manifest, semantic_contract},
    resources::{Reservation, ResourceBudget},
    serving::SnapshotHandle,
};
use lctx_surrealdb::{
    RuntimeConfig,
    compiler::NativeCompilerStore,
    surrealdb::types::{Bytes, RecordId, Value},
};
use std::{
    collections::{BTreeMap, BTreeSet},
    io::{BufRead, Read, Seek, Write},
    path::Path,
    sync::Arc,
};

const STATE_TABLES: &[&str] = &[
    "compiler_contribution",
    "compiler_membership",
    "compiler_view",
    "compiler_record",
    "compiler_binding",
    "compiler_alias",
];
struct Dump {
    _directory: tempfile::TempDir,
    files: BTreeMap<String, std::fs::File>,
    definitions: BTreeSet<String>,
    original_chunks: BTreeMap<RecordId, std::path::PathBuf>,
    _charge: Box<dyn Reservation>,
}
impl Dump {
    fn decode(input: &Path, budget: &ResourceBudget) -> Result<Self, ModelError> {
        let directory = tempfile::tempdir().map_err(ModelError::codec)?;
        let mut files = BTreeMap::new();
        for table in DATA_TABLES {
            files.insert(
                table.to_string(),
                std::fs::OpenOptions::new()
                    .read(true)
                    .write(true)
                    .create_new(true)
                    .open(directory.path().join(table))
                    .map_err(ModelError::codec)?,
            );
        }
        let mut result = Self {
            _directory: directory,
            files,
            definitions: BTreeSet::new(),
            original_chunks: BTreeMap::new(),
            _charge: budget.reserve(
                "restore-parse-and-definition-metadata",
                crate::backup_import::MAX_DUMP_RECORD_BYTES * 4,
            )?,
        };
        let mut dump = DataDump::new(std::fs::File::open(input).map_err(ModelError::codec)?);
        while let Some(item) = dump.next()? {
            match item {
                Item::Definition(value) => {
                    if !result.definitions.contains(&value) {
                        result
                            ._charge
                            .try_resize(result._charge.size().saturating_add(value.len() + 128))?;
                        result.definitions.insert(value);
                    }
                }
                Item::Rows(rows) => {
                    for row in rows {
                        let table = record_id(&row)?.table.to_string();
                        if table == "original_chunk" {
                            let Some(Value::RecordId(source)) = object(&row)?.get("source") else {
                                return Err(ModelError::Schema("restore original chunk source"));
                            };
                            if source.table.as_str() != "original" {
                                return Err(ModelError::Schema(
                                    "restore original chunk control identity",
                                ));
                            }
                            let source = source.clone();
                            if !result.original_chunks.contains_key(&source) {
                                result
                                    ._charge
                                    .try_resize(result._charge.size().saturating_add(512))?;
                                let key = ContentHash::of(
                                    &serde_json::to_vec(&source).map_err(ModelError::codec)?,
                                )
                                .hex();
                                result.original_chunks.insert(
                                    source.clone(),
                                    result
                                        ._directory
                                        .path()
                                        .join(format!("original-chunks-{key}")),
                                );
                            }
                            let path = result
                                .original_chunks
                                .get(&source)
                                .expect("inserted source spool");
                            let mut file = std::fs::OpenOptions::new()
                                .create(true)
                                .append(true)
                                .open(path)
                                .map_err(ModelError::codec)?;
                            serde_json::to_writer(&mut file, &row).map_err(ModelError::codec)?;
                            file.write_all(b"\n").map_err(ModelError::codec)?;
                        } else {
                            let file = result
                                .files
                                .get_mut(&table)
                                .ok_or(ModelError::Schema("restore spool table"))?;
                            serde_json::to_writer(&mut *file, &row).map_err(ModelError::codec)?;
                            file.write_all(b"\n").map_err(ModelError::codec)?;
                        }
                    }
                }
            }
        }
        for file in result.files.values_mut() {
            file.flush().map_err(ModelError::codec)?;
            file.rewind().map_err(ModelError::codec)?;
        }
        Ok(result)
    }
    fn rows(&self, table: &str) -> Result<Rows, ModelError> {
        let file =
            std::fs::File::open(self._directory.path().join(table)).map_err(ModelError::codec)?;
        Ok(Rows {
            input: std::io::BufReader::new(file),
            bytes: Vec::new(),
        })
    }
    fn original_rows(&self, source: &RecordId) -> Result<Option<Rows>, ModelError> {
        self.original_chunks
            .get(source)
            .map(|path| {
                std::fs::File::open(path)
                    .map(|file| Rows {
                        input: std::io::BufReader::new(file),
                        bytes: Vec::new(),
                    })
                    .map_err(ModelError::codec)
            })
            .transpose()
    }
    fn publication(
        &self,
        selected: Option<ContentHash>,
    ) -> Result<(Manifest, Vec<CompletedBinding>), ModelError> {
        let mut result = None;
        for row in self.rows("publication")? {
            let row = row?;
            let object = object(&row)?;
            let encoded = match object.get("handle") {
                Some(Value::String(value)) => value,
                _ => return Err(ModelError::Schema("restore claimed publication handle")),
            };
            let handle: SnapshotHandle =
                serde_json::from_slice(&hex::decode(encoded).map_err(ModelError::codec)?)
                    .map_err(ModelError::codec)?;
            handle.validate_identity()?;
            if hex::encode(serde_json::to_vec(&handle).map_err(ModelError::codec)?) != *encoded {
                return Err(ModelError::Conflict("restore canonical publication handle"));
            }
            if selected.is_some_and(|identity| identity != handle.publication) {
                continue;
            }
            if result.is_some() {
                return Err(ModelError::Invalid("restore requires one explicit publication identity for a multi-publication dump".into()));
            }
            if record_id(&row)? != &RecordId::new("publication", handle.publication.hex()) {
                return Err(ModelError::Conflict("restore publication record identity"));
            }
            let manifest = Manifest::decode(bytes(object, "manifest")?)?;
            let bindings: Vec<CompletedBinding> =
                serde_json::from_slice(bytes(object, "views")?).map_err(ModelError::codec)?;
            if manifest.content() != handle.semantic
                || crate::native_publication::view_identity(&bindings)? != handle.view
            {
                return Err(ModelError::Conflict("restore claimed publication content"));
            }
            result = Some((manifest, bindings));
        }
        result.ok_or(ModelError::Schema("restore requested publication absent"))
    }
    async fn compare_definitions(&self, config: &RuntimeConfig) -> Result<(), ModelError> {
        let client = lctx_surrealdb::reader::connect(
            &config.endpoint,
            &config.writer_credentials(),
            config.namespace.as_str(),
            config.database.as_str(),
        )
        .await?;
        let result = async {
            let mut known = BTreeSet::new();
            let mut response = client
                .query("INFO FOR DB")
                .await
                .map_err(ModelError::codec)?
                .check()
                .map_err(ModelError::codec)?;
            let db: Value = response.take(0).map_err(ModelError::codec)?;
            let db = object(&db)?;
            for group in ["functions", "analyzers", "tables"] {
                if let Some(Value::Object(definitions)) = db.get(group) {
                    for definition in definitions.values() {
                        collect_definition(&mut known, definition)?;
                    }
                }
            }
            if let Some(Value::Object(tables)) = db.get("tables") {
                for table in tables.keys() {
                    let escaped = table.replace('`', "\\`");
                    let mut response = client
                        .query(format!("INFO FOR TABLE `{escaped}`"))
                        .await
                        .map_err(ModelError::codec)?
                        .check()
                        .map_err(ModelError::codec)?;
                    let value: Value = response.take(0).map_err(ModelError::codec)?;
                    for group in ["fields", "indexes"] {
                        if let Some(Value::Object(definitions)) = object(&value)?.get(group) {
                            for definition in definitions.values() {
                                collect_definition(&mut known, definition)?;
                            }
                        }
                    }
                }
            }
            if !self.definitions.is_subset(&known) {
                return Err(ModelError::Conflict(
                    "restore definitions differ from installed immutable epoch/schema",
                ));
            }
            Ok(())
        }
        .await;
        let mut completion = lctx_model::domain::completion::Completion::default();
        completion.step(
            "restore definition comparison session invalidation",
            client.invalidate().await.map_err(ModelError::codec),
        );
        lctx_model::domain::completion::complete(result, completion)
    }
}
fn collect_definition(known: &mut BTreeSet<String>, value: &Value) -> Result<(), ModelError> {
    let Value::String(sql) = value else {
        return Err(ModelError::Schema("installed definition text"));
    };
    let parsed = surrealdb_syn::parse(sql).map_err(ModelError::codec)?;
    if parsed.expressions.len() != 1 {
        return Err(ModelError::Schema("installed definition inventory"));
    }
    let surrealdb_sql::TopLevelExpr::Expr(surrealdb_sql::Expr::Define(definition)) =
        &parsed.expressions[0]
    else {
        return Err(ModelError::Schema("installed definition inventory"));
    };
    known.insert(definition_metadata(definition)?);
    Ok(())
}
struct Rows {
    input: std::io::BufReader<std::fs::File>,
    bytes: Vec<u8>,
}
impl Iterator for Rows {
    type Item = Result<Value, ModelError>;
    fn next(&mut self) -> Option<Self::Item> {
        self.bytes.clear();
        match self
            .input
            .by_ref()
            .take(crate::backup_import::MAX_DUMP_RECORD_BYTES as u64 + 1)
            .read_until(b'\n', &mut self.bytes)
        {
            Ok(0) => None,
            Ok(n) if n > crate::backup_import::MAX_DUMP_RECORD_BYTES => {
                Some(Err(ModelError::Schema("restore spool row bound")))
            }
            Ok(_) => Some(serde_json::from_slice(&self.bytes).map_err(ModelError::codec)),
            Err(error) => Some(Err(ModelError::codec(error))),
        }
    }
}
fn object(value: &Value) -> Result<&lctx_surrealdb::surrealdb::types::Object, ModelError> {
    match value {
        Value::Object(value) => Ok(value),
        _ => Err(ModelError::Schema("restore record object")),
    }
}
fn record_id(value: &Value) -> Result<&RecordId, ModelError> {
    match object(value)?.get("id") {
        Some(Value::RecordId(value)) => Ok(value),
        _ => Err(ModelError::Schema("restore record identity")),
    }
}
fn bytes<'a>(
    value: &'a lctx_surrealdb::surrealdb::types::Object,
    field: &str,
) -> Result<&'a Bytes, ModelError> {
    match value.get(field) {
        Some(Value::Bytes(value)) => Ok(value),
        _ => Err(ModelError::Schema("restore canonical bytes")),
    }
}

pub(crate) async fn restore(
    config: &RuntimeConfig,
    input: &Path,
    selected: Option<ContentHash>,
    definitions: &str,
) -> Result<SnapshotHandle, ModelError> {
    let options = cpg_core::workspace::WorkspaceOptions::default();
    let budget = ResourceBudget::fixed(options.memory_bytes)?;
    let dump = Dump::decode(input, &budget)?;
    dump.compare_definitions(config).await?;
    let (manifest, bindings) = dump.publication(selected)?;
    if manifest.semantic_contract != semantic_contract(&lctx_model::domain::model()?) {
        return Err(ModelError::Conflict("restore semantic contract"));
    }
    let native = NativeCompilerStore::begin(config, manifest.frontier).await?;
    let result = async {
        let admitted = reconstruct(&dump, &native, &manifest, &bindings, &budget, options).await?;
        let publication = Publication::new(Admission::Restored(&admitted), config).await?;
        let result = async {
            publication.install_definitions(definitions).await?;
            publication.materialize_search().await?;
            publication.seal(config, definitions).await
        }
        .await;
        let mut completion = lctx_model::domain::completion::Completion::default();
        if result.is_err() {
            completion.step(
                "restore publication session invalidation",
                publication.invalidate().await,
            );
        }
        lctx_model::domain::completion::complete(result, completion)
    }
    .await;
    let completion = crate::publication_completion(&native, &result).await;
    let mut completion = completion;
    let identity = dump._directory.path().display().to_string();
    completion.cleanup(identity, dump._directory.close().map_err(ModelError::codec));
    lctx_model::domain::completion::complete(result, completion)
}

async fn reconstruct(
    dump: &Dump,
    native: &Arc<NativeCompilerStore>,
    manifest: &Manifest,
    bindings: &[CompletedBinding],
    budget: &ResourceBudget,
    options: cpg_core::workspace::WorkspaceOptions,
) -> Result<cpg_core::artifact::RestoredAdmission, ModelError> {
    let runtime = cpg_core::workspace::Workspace::with_budget(
        Arc::new(lctx_model::domain::model()?),
        options,
        budget.clone(),
        native.clone(),
    )?;
    let mut selection_charge = budget.reserve(
        "restore-exact-selected-identities",
        bindings.len().saturating_mul(512),
    )?;
    // Inventory descriptors once, then close dependencies in memory over charged compact
    // graph metadata. Reused physical attempts must agree on one logical descriptor.
    let mut contributions: BTreeMap<
        ContentHash,
        (
            lctx_model::domain::completed::CompletedContribution,
            BTreeSet<RecordId>,
        ),
    > = BTreeMap::new();
    for row in dump.rows("compiler_contribution")? {
        let row = row?;
        if object(&row)?.get("completed") != Some(&Value::Bool(true)) {
            continue;
        }
        let descriptor: lctx_model::domain::completed::CompletedContribution =
            serde_json::from_slice(bytes(object(&row)?, "descriptor")?)
                .map_err(ModelError::codec)?;
        let logical = descriptor.identity()?;
        let owner = record_id(&row)?.clone();
        if let Some((actual, owners)) = contributions.get_mut(&logical) {
            if actual != &descriptor {
                return Err(ModelError::Conflict(
                    "restore logical contribution collision",
                ));
            }
            selection_charge.try_resize(selection_charge.size().saturating_add(256))?;
            owners.insert(owner);
        } else {
            selection_charge.try_resize(
                selection_charge.size().saturating_add(
                    serde_json::to_vec(&descriptor)
                        .map_err(ModelError::codec)?
                        .len()
                        + 512,
                ),
            )?;
            contributions.insert(logical, (descriptor, BTreeSet::from([owner])));
        }
    }
    let mut view_inventory = BTreeMap::new();
    for row in dump.rows("compiler_view")? {
        let row = row?;
        let view: lctx_model::domain::completed::CompletedView =
            serde_json::from_slice(bytes(object(&row)?, "descriptor")?)
                .map_err(ModelError::codec)?;
        view.validate()?;
        if record_id(&row)? != &RecordId::new("compiler_view", view.identity.hex()) {
            return Err(ModelError::Conflict(
                "restore completed view record identity",
            ));
        }
        selection_charge.try_resize(
            selection_charge
                .size()
                .saturating_add(serde_json::to_vec(&view).map_err(ModelError::codec)?.len() + 256),
        )?;
        if view_inventory
            .insert(view.identity, view.clone())
            .is_some_and(|actual| actual != view)
        {
            return Err(ModelError::Conflict("restore completed view collision"));
        }
    }
    let mut owners = BTreeSet::new();
    let mut owner_specs = BTreeMap::new();
    let mut views = BTreeSet::new();
    let mut queue = bindings
        .iter()
        .map(|binding| binding.view.clone())
        .collect::<Vec<_>>();
    while let Some(view) = queue.pop() {
        if !views.insert(view.identity.hex()) {
            continue;
        }
        if view_inventory.get(&view.identity) != Some(&view) {
            return Err(ModelError::Conflict("restore exact dependency view"));
        }
        for logical in &view.contributions {
            let (descriptor, physical) = contributions
                .get(logical)
                .ok_or(ModelError::Schema("restore dependency contributor absent"))?;
            let owner = physical
                .first()
                .ok_or(ModelError::Schema("restore physical contributor absent"))?
                .clone();
            if owners.insert(owner.clone()) {
                selection_charge.try_resize(selection_charge.size().saturating_add(512))?;
                owner_specs.insert(owner, descriptor.spec.clone());
            }
            for input in &descriptor.spec.inputs {
                queue.push(
                    view_inventory
                        .get(&input.view())
                        .ok_or(ModelError::Schema(
                            "restore transitive dependency view absent",
                        ))?
                        .clone(),
                );
            }
        }
    }
    let mut nodes = BTreeSet::new();
    for row in dump.rows("compiler_membership")? {
        let row = row?;
        let row = object(&row)?;
        if row
            .get("contribution")
            .is_some_and(|owner| matches!(owner,Value::RecordId(owner) if owners.contains(owner)))
        {
            if let Some(Value::RecordId(node)) = row.get("node") {
                if !nodes.contains(node) {
                    selection_charge.try_resize(selection_charge.size().saturating_add(256))?;
                    nodes.insert(node.clone());
                }
            }
        }
    }
    for row in dump.rows("compiler_alias")? {
        let row = row?;
        let row = object(&row)?;
        if row
            .get("source")
            .is_some_and(|value| matches!(value,Value::RecordId(source) if nodes.contains(source)))
        {
            let Some(Value::RecordId(target)) = row.get("target") else {
                return Err(ModelError::Schema("restore alias target"));
            };
            if nodes.insert(target.clone()) {
                selection_charge.try_resize(selection_charge.size().saturating_add(256))?;
            }
        }
    }
    // A shared store must never supply rows omitted from an untrusted backup. Establish
    // the complete selected payload inventory from this dump before any native effects.
    let mut present = BTreeSet::new();
    for table in ["entity", "assertion", "compiler_record"] {
        for row in dump.rows(table)? {
            let row = row?;
            let id = record_id(&row)?;
            if nodes.contains(id) {
                selection_charge.try_resize(selection_charge.size().saturating_add(256))?;
                if !present.insert(id.clone()) {
                    return Err(ModelError::Conflict("restore duplicate selected payload"));
                }
            }
        }
    }
    if present != nodes {
        return Err(ModelError::Conflict(
            "restore selected payload absent from dump",
        ));
    }
    drop(present);
    let mut batch_charge = budget.reserve("restore-canonical-ingress", 0)?;
    for table in ["entity", "assertion"] {
        let mut entities = Vec::new();
        let mut assertions = Vec::new();
        let mut batch_bytes = 0usize;
        for row in dump.rows(table)? {
            let row = row?;
            if !nodes.contains(record_id(&row)?) {
                continue;
            }
            let payload = bytes(object(&row)?, "canonical")?;
            if batch_bytes > 0
                && (entities.len() + assertions.len() >= 128
                    || batch_bytes.saturating_add(payload.len()) > 8 * 1024 * 1024)
            {
                native.import_entities(&entities).await?;
                native.import_assertions(&assertions).await?;
                entities.clear();
                assertions.clear();
                batch_bytes = 0;
                batch_charge.try_resize(0)?;
            }
            batch_bytes = batch_bytes.saturating_add(payload.len());
            batch_charge.try_resize(batch_bytes.saturating_mul(4))?;
            if table == "entity" {
                let entity =
                    serde_json::from_slice::<Entity>(payload).map_err(ModelError::codec)?;
                if record_id(&row)? != &lctx_surrealdb::loader::entity_payload_id(&entity)? {
                    return Err(ModelError::Conflict("restore entity canonical address"));
                }
                entities.push(entity);
            } else {
                let assertion =
                    serde_json::from_slice::<Assertion>(payload).map_err(ModelError::codec)?;
                if record_id(&row)? != &lctx_surrealdb::loader::assertion_payload_id(&assertion)? {
                    return Err(ModelError::Conflict("restore assertion canonical address"));
                }
                assertions.push(assertion);
            }
        }
        native.import_entities(&entities).await?;
        native.import_assertions(&assertions).await?;
        drop(entities);
        drop(assertions);
        batch_charge.try_resize(0)?;
    }
    for original in &manifest.originals {
        let mut file = tempfile::tempfile().map_err(ModelError::codec)?;
        let source = RecordId::new("original", original.source.0.hex());
        let mut seen = BTreeSet::new();
        let mut total = 0u64;
        let mut chunks_charge = budget.reserve("restore-original-chunk-positions", 0)?;
        for row in dump.original_rows(&source)?.into_iter().flatten() {
            let row = row?;
            let row = object(&row)?;
            if row.get("source") == Some(&Value::RecordId(source.clone())) {
                let Some(Value::Number(lctx_surrealdb::surrealdb::types::Number::Int(start))) =
                    row.get("start")
                else {
                    return Err(ModelError::Schema("restore original offset"));
                };
                let start = u64::try_from(*start).map_err(ModelError::codec)?;
                let chunk = bytes(row, "bytes")?;
                if start >= original.byte_len
                    || start % 65536 != 0
                    || chunk.len() as u64 != (original.byte_len - start).min(65536)
                    || !seen.insert(start)
                {
                    return Err(ModelError::Conflict("restore original chunk continuity"));
                }
                chunks_charge.try_resize(seen.len().saturating_mul(64))?;
                file.seek(std::io::SeekFrom::Start(start))
                    .map_err(ModelError::codec)?;
                file.write_all(chunk).map_err(ModelError::codec)?;
                total += chunk.len() as u64;
            }
        }
        if total != original.byte_len {
            return Err(ModelError::Conflict("restore original length"));
        }
        file.rewind().map_err(ModelError::codec)?;
        native
            .import_original_stream(
                original.source.0,
                original.content,
                original.byte_len,
                &mut file,
            )
            .await?;
    }
    let mut state = tempfile::NamedTempFile::new().map_err(ModelError::codec)?;
    serde_json::to_writer(&mut state, &CompletedStateHeader::current())
        .map_err(ModelError::codec)?;
    state.write_all(b"\n").map_err(ModelError::codec)?;
    let mut binding_keys = BTreeSet::new();
    for table in STATE_TABLES {
        let mut sorted = lctx_surrealdb::ordered_rows::SortedRows::with_budget(budget)?;
        for row in dump.rows(table)? {
            let row = row?;
            let obj = object(&row)?;
            let keep = match *table {
                "compiler_contribution" => owners.contains(record_id(&row)?),
                "compiler_membership" => obj
                    .get("contribution")
                    .is_some_and(|v| matches!(v,Value::RecordId(owner) if owners.contains(owner))),
                "compiler_view" => match &record_id(&row)?.key {
                    lctx_surrealdb::surrealdb::types::RecordIdKey::String(id) => views.contains(id),
                    _ => false,
                },
                "compiler_record" => nodes.contains(record_id(&row)?),
                "compiler_binding" => {
                    let descriptor: CompletedBinding =
                        serde_json::from_slice(bytes(obj, "descriptor")?)
                            .map_err(ModelError::codec)?;
                    bindings.contains(&descriptor) && binding_keys.insert(descriptor.key())
                }
                "compiler_alias" => {
                    obj.get("source")
                        .is_some_and(|v| matches!(v,Value::RecordId(node) if nodes.contains(node)))
                        && obj.get("target").is_some_and(
                            |v| matches!(v,Value::RecordId(node) if nodes.contains(node)),
                        )
                }
                _ => false,
            };
            if keep {
                sorted.push(lctx_surrealdb::compiler::normalize_transport_row(
                    table,
                    &row,
                    &owner_specs,
                )?)?;
            }
        }
        let mut sorted = sorted.finish()?;
        while let Some(row) = sorted.next_row()? {
            serde_json::to_writer(&mut state, &serde_json::json!({"table":table,"row":row}))
                .map_err(ModelError::codec)?;
            state.write_all(b"\n").map_err(ModelError::codec)?;
        }
    }
    state.flush().map_err(ModelError::codec)?;
    native
        .import_state(state.path(), &manifest.completed_state)
        .await?;
    runtime.restore(manifest.profile).await?;
    cpg_core::artifact::verify_restored(&runtime, manifest).await
}

#[cfg(test)]
mod definition_tests {
    use super::*;

    #[test]
    fn exported_field_metadata_matches_installed_info() {
        let info = "DEFINE FIELD canonical ON entity TYPE bytes PERMISSIONS FULL";
        let export = "OPTION IMPORT; DEFINE FIELD OVERWRITE canonical ON entity TYPE bytes PERMISSIONS FULL;";
        let mut known = BTreeSet::new();
        collect_definition(&mut known, &Value::String(info.into())).unwrap();
        let mut dump = DataDump::new(export.as_bytes());
        let Some(Item::Definition(metadata)) = dump.next().unwrap() else {
            panic!("exported declaration");
        };
        assert!(
            known.contains(&metadata),
            "ordinary INFO and native export describe the same schema"
        );
        assert!(dump.next().unwrap().is_none());
        let mut changed = BTreeSet::new();
        collect_definition(
            &mut changed,
            &Value::String("DEFINE FIELD canonical ON entity TYPE string PERMISSIONS FULL".into()),
        )
        .unwrap();
        assert!(
            !changed.contains(&metadata),
            "normalization cannot hide schema drift"
        );
    }
}
