//! Ordinary restore lowers untrusted dump values through typed native ingress.
//! Temporary files isolate parsing and large data, never a disposable database or raw SQL session.
use crate::{
    backup_import::{DataDump, Item, definition_metadata, dump_tables, table_codec},
    native_publication::{Admission, Publication},
};
use lctx_compiler_store::compiler::NativeCompilerStore;
use lctx_model::domain::{
    ContentHash, ModelError,
    completed::CompletedBinding,
    graph::{Assertion, Entity, Manifest, semantic_contract},
    recovery_closure::{RECOVERY_FAMILIES, RecoveryFamily},
    resources::{Reservation, ResourceBudget},
    serving::SnapshotHandle,
};
use lctx_native::{
    RuntimeConfig,
    surrealdb::types::{Bytes, RecordId, Value},
};
use std::{
    collections::{BTreeMap, BTreeSet},
    io::{BufRead, Read, Seek, Write},
    path::Path,
    sync::Arc,
};

const SPOOL_BUFFER_BYTES: usize = 64 * 1024;

/// Flush explicitly, then disarm BufWriter's best-effort Drop flush even after failure.
pub(crate) fn finish_spool<W: Write>(mut writer: std::io::BufWriter<W>) -> Result<(), ModelError> {
    let result = writer.flush().map_err(ModelError::codec);
    let (_sink, _unflushed) = writer.into_parts();
    result
}

struct Dump {
    _directory: tempfile::TempDir,
    definitions: BTreeSet<String>,
    original_chunks: BTreeMap<RecordId, std::path::PathBuf>,
    _charge: Box<dyn Reservation>,
}
impl Dump {
    fn decode(input: &Path, budget: &ResourceBudget) -> Result<Self, ModelError> {
        let charge = budget.reserve(
            "restore-parse-and-definition-metadata",
            crate::backup_import::MAX_DUMP_RECORD_BYTES * 4,
        )?;
        // One fixed buffer per table and at most one original-chunk append buffer.
        let _spool_buffers = budget.reserve(
            "restore-spool-buffers",
            SPOOL_BUFFER_BYTES * (dump_tables().count() + 1),
        )?;
        let directory = tempfile::tempdir().map_err(ModelError::codec)?;
        let files = (|| {
            let mut files = BTreeMap::new();
            for table in dump_tables().map(|table| table.name) {
                files.insert(
                    table.to_string(),
                    std::io::BufWriter::with_capacity(
                        SPOOL_BUFFER_BYTES,
                        std::fs::OpenOptions::new()
                            .write(true)
                            .create_new(true)
                            .open(directory.path().join(table))
                            .map_err(ModelError::codec)?,
                    ),
                );
            }
            Ok(files)
        })();
        let mut files = match files {
            Ok(files) => files,
            Err(error) => {
                let mut completion = lctx_model::domain::completion::Completion::default();
                completion.cleanup(
                    directory.path().display().to_string(),
                    directory.close().map_err(ModelError::codec),
                );
                return lctx_model::domain::completion::complete(Err(error), completion);
            }
        };
        let mut result = Self {
            _directory: directory,
            definitions: BTreeSet::new(),
            original_chunks: BTreeMap::new(),
            _charge: charge,
        };
        let decoded = (|| {
            let mut dump = DataDump::new(std::fs::File::open(input).map_err(ModelError::codec)?);
            while let Some(item) = dump.next()? {
                match item {
                    Item::Definition(value) => {
                        if !result.definitions.contains(&value) {
                            result._charge.try_resize(
                                result._charge.size().saturating_add(value.len() + 128),
                            )?;
                            result.definitions.insert(value);
                        }
                    }
                    Item::Rows(rows) => {
                        for row in rows {
                            let table = record_id(&row)?.table.to_string();
                            if table == "original_chunk" {
                                let Some(Value::RecordId(source)) = object(&row)?.get("blob")
                                else {
                                    return Err(ModelError::Schema(
                                        "restore original chunk source",
                                    ));
                                };
                                if source.table.as_str() != "original_blob" {
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
                                let mut file = std::io::BufWriter::with_capacity(
                                    SPOOL_BUFFER_BYTES,
                                    std::fs::OpenOptions::new()
                                        .create(true)
                                        .append(true)
                                        .open(path)
                                        .map_err(ModelError::codec)?,
                                );
                                let written = (|| {
                                    serde_json::to_writer(&mut file, &row)
                                        .map_err(ModelError::codec)?;
                                    file.write_all(b"\n").map_err(ModelError::codec)
                                })();
                                let mut completion =
                                    lctx_model::domain::completion::Completion::default();
                                completion
                                    .step("restore original chunk spool flush", finish_spool(file));
                                lctx_model::domain::completion::complete(written, completion)?;
                            } else {
                                let file = files
                                    .get_mut(&table)
                                    .ok_or(ModelError::Schema("restore spool table"))?;
                                serde_json::to_writer(&mut *file, &row)
                                    .map_err(ModelError::codec)?;
                                file.write_all(b"\n").map_err(ModelError::codec)?;
                            }
                        }
                    }
                }
            }
            Ok(())
        })();
        // Attempt every buffer's flush even after a parse/write failure. Readers reopen the
        // completed spool at offset zero, only after all buffered writes succeeded.
        let mut completion = lctx_model::domain::completion::Completion::default();
        for file in files.into_values() {
            completion.step("restore table spool flush", finish_spool(file));
        }
        let decoded = lctx_model::domain::completion::complete(decoded, completion);
        if let Err(error) = decoded {
            let mut completion = lctx_model::domain::completion::Completion::default();
            completion.cleanup(
                result._directory.path().display().to_string(),
                result._directory.close().map_err(ModelError::codec),
            );
            return lctx_model::domain::completion::complete(Err(error), completion);
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
    ) -> Result<(SnapshotHandle, Manifest, Vec<CompletedBinding>), ModelError> {
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
            if object.get("definition_epoch") != Some(&Value::String(handle.definition_epoch.hex()))
            {
                return Err(ModelError::Conflict("restore claimed definition epoch"));
            }
            result = Some((handle, manifest, bindings));
        }
        result.ok_or(ModelError::Schema("restore requested publication absent"))
    }
    async fn compare_definitions(&self, config: &RuntimeConfig) -> Result<(), ModelError> {
        let client = lctx_native::reader::connect(
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
fn object(value: &Value) -> Result<&lctx_native::surrealdb::types::Object, ModelError> {
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
    value: &'a lctx_native::surrealdb::types::Object,
    field: &str,
) -> Result<&'a Bytes, ModelError> {
    match value.get(field) {
        Some(Value::Bytes(value)) => Ok(value),
        _ => Err(ModelError::Schema("restore canonical bytes")),
    }
}

fn identity_charge(id: &RecordId) -> Result<usize, ModelError> {
    // Native addresses use string keys. Preserve grammar-owned non-string claimed keys
    // for subsequent admission, accounting their nested representation conservatively.
    let key = match &id.key {
        lctx_native::surrealdb::types::RecordIdKey::String(key) => key.len(),
        _ => serde_json::to_vec(id)
            .map_err(ModelError::codec)?
            .len()
            .saturating_mul(64),
    };
    Ok(key
        .saturating_add(id.table.as_str().len())
        .saturating_add(256))
}

/// Compact graph demand derived separately for each root-qualified copy selection.
struct DumpGraph {
    nodes: BTreeSet<RecordId>,
    aliases: BTreeSet<RecordId>,
    _charge: Box<dyn Reservation>,
}
impl DumpGraph {
    fn from_transport(
        transport: &lctx_native::completed_transport::DecodedTransport,
        budget: &ResourceBudget,
    ) -> Result<Self, ModelError> {
        let mut charge = budget.reserve("restore-root-qualified-dump-graph", 0)?;
        let mut copies = BTreeMap::new();
        let mut aliases = BTreeSet::new();
        for copy in transport.copies() {
            let mut members = BTreeSet::new();
            let mut edges = Vec::new();
            let mut rows = copy.memberships.cursor_with_budget(budget)?;
            while let Some(row) = rows.next_row()? {
                let Some(Value::RecordId(node)) = object(&row)?.get("node") else {
                    return Err(ModelError::Schema("restore transport member node"));
                };
                charge.try_resize(charge.size().saturating_add(identity_charge(node)?))?;
                members.insert(node.clone());
            }
            let mut rows = copy.aliases.cursor_with_budget(budget)?;
            while let Some(row) = rows.next_row()? {
                let row_object = object(&row)?;
                let (Some(Value::RecordId(source)), Some(Value::RecordId(target))) =
                    (row_object.get("source"), row_object.get("target"))
                else {
                    return Err(ModelError::Schema("restore transport alias endpoints"));
                };
                charge.try_resize(
                    charge
                        .size()
                        .saturating_add(identity_charge(source)?)
                        .saturating_add(identity_charge(target)?)
                        .saturating_add(identity_charge(record_id(&row)?)?),
                )?;
                edges.push((source.clone(), target.clone()));
                aliases.insert(record_id(&row)?.clone());
            }
            copies.insert(copy.label, (members, edges));
        }
        let mut nodes = BTreeSet::new();
        for root in &transport.envelope().roots {
            let mut root_charge = budget.reserve(
                "restore-one-root-alias-closure",
                root.slots.len().saturating_mul(64),
            )?;
            let labels = root
                .slots
                .iter()
                .map(|slot| slot.copy)
                .collect::<BTreeSet<_>>();
            let mut selected = BTreeSet::new();
            let mut adjacency = BTreeMap::<RecordId, Vec<RecordId>>::new();
            for label in labels {
                let (members, edges) = copies.get(&label).ok_or(ModelError::Conflict(
                    "restore selected copy inventory absent",
                ))?;
                for node in members {
                    root_charge.try_resize(
                        root_charge
                            .size()
                            .saturating_add(identity_charge(node)?.saturating_mul(3)),
                    )?;
                    selected.insert(node.clone());
                }
                for (source, target) in edges {
                    root_charge.try_resize(
                        root_charge
                            .size()
                            .saturating_add(identity_charge(source)?)
                            .saturating_add(identity_charge(target)?),
                    )?;
                    adjacency
                        .entry(source.clone())
                        .or_default()
                        .push(target.clone());
                }
            }
            let mut pending = selected.iter().cloned().collect::<Vec<_>>();
            while let Some(source) = pending.pop() {
                for target in adjacency.get(&source).into_iter().flatten() {
                    if !selected.contains(target) {
                        root_charge.try_resize(
                            root_charge
                                .size()
                                .saturating_add(identity_charge(target)?.saturating_mul(3)),
                        )?;
                        selected.insert(target.clone());
                        pending.push(target.clone());
                    }
                }
            }
            for node in selected {
                if !nodes.contains(&node) {
                    charge.try_resize(charge.size().saturating_add(identity_charge(&node)?))?;
                    nodes.insert(node);
                }
            }
        }
        drop(copies);
        let retained = nodes.iter().chain(&aliases).try_fold(0usize, |total, id| {
            Ok::<_, ModelError>(total.saturating_add(identity_charge(id)?))
        })?;
        charge.try_resize(retained)?;
        Ok(Self {
            nodes,
            aliases,
            _charge: charge,
        })
    }
}

/// Charged claimed graph selection within one fully decoded dump. This is preparation,
/// never an admission capability: restore still validates typed/cold state independently.
struct PreparedClosure {
    handle: SnapshotHandle,
    manifest: Manifest,
    bindings: Vec<CompletedBinding>,
    originals: BTreeMap<RecordId, lctx_model::domain::graph::Original>,
    layouts: BTreeMap<RecordId, lctx_native::originals::Layout>,
    blobs: BTreeMap<RecordId, lctx_native::originals::Blob>,
    binding_rows: BTreeSet<RecordId>,
    owners: BTreeSet<RecordId>,
    aliases: BTreeSet<RecordId>,
    views: BTreeSet<String>,
    nodes: BTreeSet<RecordId>,
    derived_rows: BTreeSet<RecordId>,
    _charge: Box<dyn Reservation>,
    _graph_charge: Box<dyn Reservation>,
}
impl PreparedClosure {
    fn prepare(
        dump: &Dump,
        selected: Option<ContentHash>,
        budget: &ResourceBudget,
        graph: DumpGraph,
    ) -> Result<Self, ModelError> {
        let (handle, manifest, bindings) = dump.publication(selected)?;
        if manifest.semantic_contract != semantic_contract(&lctx_model::domain::model()?) {
            return Err(ModelError::Conflict("restore semantic contract"));
        }
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
                selection_charge.try_resize(
                    selection_charge
                        .size()
                        .saturating_add(identity_charge(&owner)?),
                )?;
                owners.insert(owner);
            } else {
                selection_charge.try_resize(
                    selection_charge.size().saturating_add(
                        serde_json::to_vec(&descriptor)
                            .map_err(ModelError::codec)?
                            .len()
                            .saturating_mul(32)
                            + identity_charge(&owner)?,
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
                selection_charge.size().saturating_add(
                    serde_json::to_vec(&view)
                        .map_err(ModelError::codec)?
                        .len()
                        .saturating_mul(32)
                        + 256,
                ),
            )?;
            if view_inventory
                .insert(view.identity, view.clone())
                .is_some_and(|actual| actual != view)
            {
                return Err(ModelError::Conflict("restore completed view collision"));
            }
        }
        let mut owners = BTreeSet::new();
        let mut views = BTreeSet::new();
        let mut traversal =
            lctx_model::domain::recovery_closure::RecoveryTraversal::new(bindings.clone())?;
        while let Some(view) = traversal.next_view()? {
            let actual = view_inventory
                .get(&view.identity)
                .cloned()
                .ok_or(ModelError::Conflict("restore exact dependency view"))?;
            let mut descriptors = Vec::new();
            for logical in &view.contributions {
                let (descriptor, physical) = contributions
                    .get(logical)
                    .ok_or(ModelError::Schema("restore dependency contributor absent"))?;
                for owner in physical {
                    if !owners.contains(owner) {
                        selection_charge.try_resize(
                            selection_charge
                                .size()
                                .saturating_add(identity_charge(owner)?),
                        )?;
                        owners.insert(owner.clone());
                    }
                }
                descriptors.push(descriptor.clone());
            }
            traversal.include_view(&view, actual, descriptors, |id| {
                view_inventory.get(&id).cloned().ok_or(ModelError::Schema(
                    "restore transitive dependency view absent",
                ))
            })?;
        }
        let closure = traversal.finish()?;
        views.extend(closure.views().keys().map(ContentHash::hex));
        // Correspondence and root-local alias closure come exclusively from the independently
        // checked completed transport; raw SQL cannot choose a logical owner's physical copy.
        let DumpGraph {
            nodes,
            aliases,
            _charge: graph_charge,
        } = graph;
        // The compiler-record pass establishes demands without reopening source tables.
        let mut demanded = manifest
            .originals
            .iter()
            .map(|original| RecordId::new("original", original.source.0.hex()))
            .collect::<BTreeSet<_>>();
        selection_charge.try_resize(
            selection_charge
                .size()
                .saturating_add(demanded.len().saturating_mul(512)),
        )?;
        // A shared store must never supply rows omitted from an untrusted backup. Establish
        // the complete selected payload inventory from this dump before creating a native attempt.
        let mut present = BTreeSet::new();
        for table in [RecoveryFamily::GraphPayloads, RecoveryFamily::TypedBacking]
            .into_iter()
            .flat_map(lctx_compiler_store::compiler::recovery_tables)
        {
            if *table == "entity" {
                continue;
            }
            for row in dump.rows(table)? {
                let row = row?;
                let id = record_id(&row)?;
                if nodes.contains(id) {
                    selection_charge.try_resize(selection_charge.size().saturating_add(256))?;
                    if !present.insert(id.clone()) {
                        return Err(ModelError::Conflict("restore duplicate selected payload"));
                    }
                    if *table == "compiler_record" {
                        if let Some(Value::Object(body)) = object(&row)?.get("body") {
                            if let Some(Value::RecordId(source)) = body.get("original") {
                                hash_record(source, "original")?;
                                if !demanded.contains(source) {
                                    selection_charge.try_resize(
                                        selection_charge
                                            .size()
                                            .saturating_add(identity_charge(source)?),
                                    )?;
                                    demanded.insert(source.clone());
                                }
                            }
                        }
                    }
                }
            }
        }
        if present
            != nodes
                .iter()
                .filter(|id| id.table.as_str() != "entity")
                .cloned()
                .collect()
        {
            return Err(ModelError::Conflict(
                "restore selected payload absent from dump",
            ));
        }
        drop(present);

        let mut binding_copies = BTreeMap::<ContentHash, RecordId>::new();
        let mut physical_binding_ids = BTreeSet::new();
        for row in dump.rows("compiler_binding")? {
            let row = row?;
            let binding: CompletedBinding =
                serde_json::from_slice(bytes(object(&row)?, "descriptor")?)
                    .map_err(ModelError::codec)?;
            if !bindings.contains(&binding) {
                continue;
            }
            let id = record_id(&row)?;
            selection_charge.try_resize(
                selection_charge
                    .size()
                    .saturating_add(identity_charge(id)?.saturating_mul(2)),
            )?;
            if !physical_binding_ids.insert(id.clone()) {
                return Err(ModelError::Conflict(
                    "restore duplicate selected physical binding",
                ));
            }
            if binding_copies.insert(binding.key(), id.clone()).is_some() {
                return Err(ModelError::Conflict("restore duplicate logical binding"));
            }
        }
        if binding_copies.keys().copied().collect::<BTreeSet<_>>()
            != bindings.iter().map(CompletedBinding::key).collect()
        {
            return Err(ModelError::Conflict(
                "restore required binding absent from dump",
            ));
        }
        drop(physical_binding_ids);
        let binding_rows = binding_copies.into_values().collect();
        let (originals, layouts, blobs) = prepare_originals(
            dump,
            &manifest.originals,
            &demanded,
            &nodes,
            selection_charge.as_mut(),
            budget,
        )?;
        drop(demanded);
        let mut derived_rows = BTreeSet::new();
        // Family dispatch selects actual source claims, never producer-expected roles or
        // occurrences. Endpoint rows follow the actual claimed edge for cold comparison.
        for family in RECOVERY_FAMILIES.iter().copied() {
            let fields = match family {
                RecoveryFamily::GraphRoles => Some(("in", "out", true)),
                RecoveryFamily::SearchOccurrences => Some(("unit_payload", "in", false)),
                RecoveryFamily::ExternalEndpoints
                | RecoveryFamily::SearchDocuments
                | RecoveryFamily::SearchVectors => None,
                RecoveryFamily::Contributions
                | RecoveryFamily::Memberships
                | RecoveryFamily::Views
                | RecoveryFamily::TypedBacking
                | RecoveryFamily::Bindings
                | RecoveryFamily::Aliases
                | RecoveryFamily::GraphPayloads
                | RecoveryFamily::OriginalHeaders
                | RecoveryFamily::OriginalChunks => None,
            };
            let Some((source_field, endpoint_field, external_only)) = fields else {
                continue;
            };
            for table in lctx_compiler_store::compiler::recovery_tables(family) {
                for row in dump.rows(table)? {
                    let row = row?;
                    let obj = object(&row)?;
                    if matches!(obj.get(source_field), Some(Value::RecordId(source)) if nodes.contains(source))
                    {
                        let mut ids = vec![record_id(&row)?.clone()];
                        if let Some(Value::RecordId(endpoint)) = obj.get(endpoint_field) {
                            if !external_only || endpoint.table.as_str() == "external" {
                                ids.push(endpoint.clone());
                            }
                        }
                        for id in ids {
                            if !derived_rows.contains(&id) {
                                selection_charge.try_resize(
                                    selection_charge
                                        .size()
                                        .saturating_add(identity_charge(&id)?),
                                )?;
                                derived_rows.insert(id);
                            }
                        }
                    }
                }
            }
        }
        let prepared = Self {
            handle,
            manifest,
            bindings,
            originals,
            layouts,
            blobs,
            binding_rows,
            owners,
            aliases,
            views,
            nodes,
            derived_rows,
            _charge: selection_charge,
            _graph_charge: graph_charge,
        };
        prepared.validate_inventory(dump, budget)?;
        Ok(prepared)
    }
    fn keep_row(&self, table: &str, row: &Value) -> Result<bool, ModelError> {
        use lctx_compiler_store::compiler::RecoveryTableKind;
        let obj = object(row)?;
        let id = record_id(row)?;
        if id.table.as_str() != table {
            return Err(ModelError::Schema("restore row table/key"));
        }
        Ok(match table_codec(table)?.kind {
            RecoveryTableKind::PublicationRoot => {
                id == &RecordId::new("publication", self.handle.publication.hex())
            }
            RecoveryTableKind::InstallationWitness => false,
            RecoveryTableKind::Content(family) => match family {
                RecoveryFamily::Contributions => self.owners.contains(id),
                RecoveryFamily::Memberships => obj.get("contribution").is_some_and(
                    |v| matches!(v, Value::RecordId(owner) if self.owners.contains(owner)),
                ),
                RecoveryFamily::Views => match &id.key {
                    lctx_native::surrealdb::types::RecordIdKey::String(id) => {
                        self.views.contains(id)
                    }
                    _ => false,
                },
                RecoveryFamily::GraphPayloads | RecoveryFamily::TypedBacking => {
                    self.nodes.contains(id)
                }
                RecoveryFamily::Bindings => self.binding_rows.contains(id),
                RecoveryFamily::Aliases => self.aliases.contains(id),
                RecoveryFamily::OriginalHeaders => {
                    if table == "original" {
                        self.layouts.contains_key(id)
                    } else {
                        self.blobs.contains_key(id)
                    }
                }
                RecoveryFamily::OriginalChunks => obj.get("blob").is_some_and(
                    |v| matches!(v,Value::RecordId(blob) if self.blobs.contains_key(blob)),
                ),
                RecoveryFamily::GraphRoles
                | RecoveryFamily::ExternalEndpoints
                | RecoveryFamily::SearchOccurrences
                | RecoveryFamily::SearchDocuments
                | RecoveryFamily::SearchVectors => self.derived_rows.contains(id),
            },
        })
    }
    fn validate_inventory(&self, dump: &Dump, budget: &ResourceBudget) -> Result<(), ModelError> {
        let mut charge = budget.reserve("restore-selected-physical-inventory", 0)?;
        let mut ids = BTreeSet::new();
        let mut binding_keys = BTreeSet::new();
        for table in dump_tables()
            .filter(|table| match table.kind {
                lctx_compiler_store::compiler::RecoveryTableKind::PublicationRoot => true,
                lctx_compiler_store::compiler::RecoveryTableKind::InstallationWitness => false,
                lctx_compiler_store::compiler::RecoveryTableKind::Content(family) => {
                    !family.is_integrity_claim() && family != RecoveryFamily::OriginalChunks
                }
            })
            .map(|table| table.name)
            .filter(|table| !matches!(*table, "original" | "original_blob" | "entity"))
        {
            // Original headers, blob descriptors and selected entity presence/duplicates
            // were checked to EOF by the typed preparation pass above.
            for row in dump.rows(table)? {
                let row = row?;
                if self.keep_row(table, &row)? {
                    charge.try_resize(
                        charge
                            .size()
                            .saturating_add(identity_charge(record_id(&row)?)?),
                    )?;
                    if !ids.insert(record_id(&row)?.clone()) {
                        return Err(ModelError::Conflict(
                            "restore duplicate selected physical row",
                        ));
                    }
                    if table == "compiler_binding" {
                        let binding: CompletedBinding =
                            serde_json::from_slice(bytes(object(&row)?, "descriptor")?)
                                .map_err(ModelError::codec)?;
                        binding_keys.insert(binding.key());
                    }
                }
            }
        }
        if binding_keys != self.bindings.iter().map(CompletedBinding::key).collect() {
            return Err(ModelError::Conflict(
                "restore required binding absent from dump",
            ));
        }
        for (blob_id, blob) in &self.blobs {
            let mut file = tempfile::tempfile().map_err(ModelError::codec)?;
            let mut seen = BTreeSet::new();
            let mut total = 0u64;
            for row in dump.original_rows(blob_id)?.into_iter().flatten() {
                let piece = lctx_native::originals::decode_piece(row?, blob)?;
                charge.try_resize(charge.size().saturating_add(256))?;
                if !seen.insert(piece.start) {
                    return Err(ModelError::Conflict("restore duplicate original piece"));
                }
                file.seek(std::io::SeekFrom::Start(piece.start))
                    .map_err(ModelError::codec)?;
                file.write_all(&piece.bytes).map_err(ModelError::codec)?;
                total = total
                    .checked_add(piece.bytes.len() as u64)
                    .ok_or(ModelError::Schema("restore original piece total"))?;
            }
            if total != blob.byte_len || seen.len() as u64 != blob.piece_count() {
                return Err(ModelError::Conflict("restore original length"));
            }
            file.rewind().map_err(ModelError::codec)?;
            let mut hasher = lctx_model::domain::ContentHasher::default();
            let mut window = [0u8; 65536];
            loop {
                let count = file.read(&mut window).map_err(ModelError::codec)?;
                if count == 0 {
                    break;
                }
                hasher.update(&window[..count]);
            }
            if hasher.finish() != blob.content {
                return Err(ModelError::Conflict("restore original full content"));
            }
        }
        Ok(())
    }
}

/// Transient identity checking is independent of selected retention. Each table is fully
/// consumed, including unselected headers and the tail after the last demanded record.
struct DumpIdentityCheck {
    sorted: lctx_native::ordered_rows::SortedRows,
    ordinal: u64,
}
impl DumpIdentityCheck {
    fn new(budget: &ResourceBudget) -> Result<Self, ModelError> {
        Ok(Self {
            sorted: lctx_native::ordered_rows::SortedRows::with_budget_and_row_bytes(budget, 4096)?,
            ordinal: 0,
        })
    }
    fn include(&mut self, table: &str, row: &Value) -> Result<(), ModelError> {
        let id = record_id(row)?;
        if id.table.as_str() != table {
            return Err(ModelError::Schema("restore source table identity"));
        }
        // A distinct ordinal deliberately makes even byte-identical duplicate IDs a
        // collision. The existing sorter owns bounded runs/spill and duplicate checks.
        let mut identity = lctx_native::surrealdb::types::Object::new();
        identity.insert("id", id.clone());
        identity.insert("ordinal", self.ordinal.to_string());
        self.ordinal = self
            .ordinal
            .checked_add(1)
            .ok_or(ModelError::Schema("restore source identity count"))?;
        self.sorted.push(Value::Object(identity))
    }
    fn finish(self) -> Result<(), ModelError> {
        self.sorted.finish().map(|_| ())
    }
}

type OriginalPreparation = (
    BTreeMap<RecordId, lctx_model::domain::graph::Original>,
    BTreeMap<RecordId, lctx_native::originals::Layout>,
    BTreeMap<RecordId, lctx_native::originals::Blob>,
);
fn prepare_originals(
    dump: &Dump,
    declared: &[lctx_model::domain::graph::Original],
    demanded: &BTreeSet<RecordId>,
    nodes: &BTreeSet<RecordId>,
    charge: &mut dyn Reservation,
    budget: &ResourceBudget,
) -> Result<OriginalPreparation, ModelError> {
    charge.try_resize(
        charge
            .size()
            .saturating_add(declared.len().saturating_mul(128)),
    )?;
    let declared = declared
        .iter()
        .map(|original| (original.source, original))
        .collect::<BTreeMap<_, _>>();
    let mut originals = BTreeMap::new();
    let mut layouts = BTreeMap::new();
    let mut required_blobs = BTreeMap::new();
    let mut artifacts = BTreeMap::<RecordId, Vec<RecordId>>::new();
    {
        let mut identities = DumpIdentityCheck::new(budget)?;
        for row in dump.rows("original")? {
            let row = row?;
            identities.include("original", &row)?;
            // Decode even unretained headers: selecting a subset cannot conceal corruption.
            let layout = lctx_native::originals::decode_layout(row)?;
            let source = hash_record(&layout.id, "original")?;
            if !demanded.contains(&layout.id) {
                continue;
            }
            if !nodes.contains(&layout.artifact) {
                return Err(ModelError::Conflict(
                    "restore original exact source authority",
                ));
            }
            let original = lctx_model::domain::graph::Original {
                source: lctx_model::domain::graph::EntityId(source),
                content: layout.blob.content,
                byte_len: layout.blob.byte_len,
            };
            if declared
                .get(&original.source)
                .is_some_and(|expected| *expected != &original)
            {
                return Err(ModelError::Conflict("restore original header"));
            }
            charge.try_resize(
                charge
                    .size()
                    .saturating_add(2048)
                    .saturating_add(identity_charge(&layout.id)?)
                    .saturating_add(identity_charge(&layout.artifact)?),
            )?;
            artifacts
                .entry(layout.artifact.clone())
                .or_default()
                .push(layout.id.clone());
            if let Some(actual) = required_blobs.insert(layout.blob.id(), layout.blob.clone()) {
                if actual != layout.blob {
                    return Err(ModelError::Conflict("restore original blob collision"));
                }
            }
            originals.insert(layout.id.clone(), original);
            layouts.insert(layout.id.clone(), layout);
        }
        identities.finish()?;
    }
    if layouts.keys().cloned().collect::<BTreeSet<_>>() != *demanded {
        return Err(ModelError::Conflict("restore original layout absent"));
    }
    {
        let mut identities = DumpIdentityCheck::new(budget)?;
        let mut present = BTreeSet::new();
        for row in dump.rows("entity")? {
            let row = row?;
            identities.include("entity", &row)?;
            let id = record_id(&row)?;
            if nodes.contains(id) {
                present.insert(id.clone());
            }
            let Some(sources) = artifacts.get(id) else {
                continue;
            };
            let entity: Entity = serde_json::from_slice(bytes(object(&row)?, "canonical")?)
                .map_err(ModelError::codec)?;
            entity.validate()?;
            if id != &lctx_native::loader::entity_payload_id(&entity)? {
                return Err(ModelError::Conflict(
                    "restore original canonical source address",
                ));
            }
            let Entity::Source(source) = entity else {
                return Err(ModelError::Schema("restore original source kind"));
            };
            for original in sources {
                layouts[original].validate_source(&source)?;
            }
        }
        identities.finish()?;
        if present
            != nodes
                .iter()
                .filter(|id| id.table.as_str() == "entity")
                .cloned()
                .collect()
        {
            return Err(ModelError::Conflict(
                "restore selected payload absent from dump",
            ));
        }
    }
    drop(artifacts);
    let mut blobs = BTreeMap::new();
    {
        let mut identities = DumpIdentityCheck::new(budget)?;
        for row in dump.rows("original_blob")? {
            let row = row?;
            identities.include("original_blob", &row)?;
            let (blob, completed) = lctx_native::originals::decode_blob(row)?;
            let Some(expected) = required_blobs.get(&blob.id()) else {
                continue;
            };
            if expected != &blob || !completed {
                return Err(ModelError::Conflict("restore original blob incomplete"));
            }
            charge.try_resize(charge.size().saturating_add(1024))?;
            blobs.insert(blob.id(), blob);
        }
        identities.finish()?;
    }
    if blobs.len() != required_blobs.len() {
        return Err(ModelError::Conflict("restore original blob absent"));
    }
    Ok((originals, layouts, blobs))
}

/// Compare the actual SQL claims against the independently checked transport union.
/// Physical addresses are source-local claims; they never choose destination copies.
fn compare_dump_state(
    dump: &Dump,
    prepared: &PreparedClosure,
    transport: &lctx_native::completed_transport::DecodedTransport,
    budget: &ResourceBudget,
) -> Result<(), ModelError> {
    let mut charge = budget.reserve("restore-dump-logical-owner-comparison", 0)?;
    let mut owners = BTreeMap::new();
    let mut descriptors = BTreeMap::<ContentHash, usize>::new();
    for row in dump.rows("compiler_contribution")? {
        let row = row?;
        if !prepared.keep_row("compiler_contribution", &row)? {
            return Err(ModelError::Conflict("restore foreign dump contribution"));
        }
        let descriptor: lctx_model::domain::completed::CompletedContribution =
            serde_json::from_slice(bytes(object(&row)?, "descriptor")?)
                .map_err(ModelError::codec)?;
        charge.try_resize(
            charge
                .size()
                .saturating_add(lctx_native::loader::native_bytes(&row).saturating_mul(32))
                .saturating_add(1024),
        )?;
        *descriptors.entry(descriptor.identity()?).or_default() += 1;
        if owners
            .insert(record_id(&row)?.clone(), descriptor)
            .is_some()
        {
            return Err(ModelError::Conflict("restore duplicate dump contributor"));
        }
    }
    let mut expected = BTreeMap::<ContentHash, usize>::new();
    for copy in &transport.envelope().copies {
        *expected.entry(copy.descriptor.identity()?).or_default() += 1;
    }
    if descriptors != expected {
        return Err(ModelError::Conflict(
            "restore exact physical copy descriptor inventory",
        ));
    }
    for (index, table) in lctx_compiler_store::compiler::STATE_TABLES
        .iter()
        .enumerate()
    {
        let mut sorted = lctx_native::ordered_rows::SortedRows::with_budget_and_row_bytes(
            budget,
            lctx_model::domain::resources::MAX_ROW_BYTES,
        )?;
        for row in dump.rows(table)? {
            let row = row?;
            if !prepared.keep_row(table, &row)? {
                return Err(ModelError::Conflict("restore foreign completed dump row"));
            }
            sorted.push(lctx_native::completed_logical_rows::normalize(
                table, &row, &owners,
            )?)?;
        }
        let rows = sorted.finish()?.into_prepared();
        let mut hash = lctx_model::domain::completed_identity::CompletedFamilyHasher::new(
            lctx_model::domain::recovery_closure::COMPLETED_FAMILIES[index],
        )?;
        let mut cursor = rows.frame_cursor(budget)?;
        while let Some((_, bytes)) = cursor.next_frame()? {
            hash.row(bytes)?;
        }
        if hash.finish() != transport.envelope().families.roots()[index] {
            return Err(ModelError::Conflict("restore actual dump semantic family"));
        }
    }
    for table in dump_tables()
        .filter(|table| {
            !matches!(
                table.kind,
                lctx_compiler_store::compiler::RecoveryTableKind::Content(
                    RecoveryFamily::OriginalChunks
                )
            )
        })
        .map(|table| table.name)
    {
        for row in dump.rows(table)? {
            let row = row?;
            if !prepared.keep_row(table, &row)? {
                return Err(ModelError::Conflict(
                    "restore foreign row outside selected dump closure",
                ));
            }
        }
    }
    if dump
        .original_chunks
        .keys()
        .any(|blob| !prepared.blobs.contains_key(blob))
    {
        return Err(ModelError::Conflict("restore foreign original chunk owner"));
    }
    Ok(())
}

pub(crate) async fn restore(
    config: &RuntimeConfig,
    input: &Path,
    selected: Option<ContentHash>,
    definitions: &str,
    expected: &lctx_model::domain::completed_transport::BackupReceipt,
) -> Result<SnapshotHandle, ModelError> {
    expected.validate()?;
    if expected.definitions != ContentHash::of(definitions.as_bytes()) {
        return Err(ModelError::Conflict("restore expected definition content"));
    }
    if selected.is_some_and(|publication| publication != expected.source.publication) {
        return Err(ModelError::Conflict(
            "restore expected publication selection",
        ));
    }
    let options = lctx_compiler_store::admission::AdmissionOptions::default();
    let budget = ResourceBudget::fixed(options.memory_bytes)?;
    let carrier = tempfile::tempdir().map_err(ModelError::codec)?;
    let result = async {
        let sql = carrier.path().join("claims.sql");
        let state = carrier.path().join("completed-state");
        crate::backup_import::read_carrier(input, &sql, &state, expected, &budget)?;
        let transport =
            lctx_native::completed_transport::read_transport(&state, &expected.state, &budget)?;
        let dump = Dump::decode(&sql, &budget)?;
        let result = async {
            let graph = DumpGraph::from_transport(&transport, &budget)?;
            let prepared =
                PreparedClosure::prepare(&dump, Some(expected.source.publication), &budget, graph)?;
            if prepared.handle != expected.source
                || prepared.manifest.content() != expected.manifest
                || prepared.manifest.completed_state != expected.state.semantic
                || prepared
                    .bindings
                    .iter()
                    .map(|binding| binding.view.identity)
                    .collect::<BTreeSet<_>>()
                    != transport
                        .envelope()
                        .roots
                        .iter()
                        .map(|root| root.view.identity)
                        .collect()
            {
                return Err(ModelError::Conflict(
                    "restore expected publication/manifest/transport roots",
                ));
            }
            compare_dump_state(&dump, &prepared, &transport, &budget)?;
            dump.compare_definitions(config).await?;
            let native = NativeCompilerStore::begin(config, prepared.manifest.frontier).await?;
            // Keep standalone admission ownership through publication success or error cleanup.
            let mut runtime = None;
            let result = async {
                let owner = lctx_compiler_store::admission::AdmissionRuntime::with_budget(
                    Arc::new(lctx_model::domain::model()?),
                    options,
                    budget.clone(),
                    native.clone(),
                )?;
                runtime = Some(owner.clone());
                let admitted =
                    reconstruct(&dump, &owner, &prepared, &budget, &state, &expected.state).await?;
                let publication =
                    Publication::new(Admission::Restored(&admitted), config, definitions).await?;
                let result = async {
                    publication.materialize_search().await?;
                    compare_derived_claims(&dump, &prepared, publication.loader(), &budget).await?;
                    publication.seal(config, definitions).await
                }
                .await;
                let mut completion = lctx_model::domain::completion::Completion::default();
                if let Err(error) = &result {
                    completion.step(
                        "restore publication session invalidation",
                        publication.invalidate(error).await,
                    );
                }
                lctx_model::domain::completion::complete(result, completion)
            }
            .await;
            let mut completion = crate::publication_completion(&native, &result).await;
            if let Some(runtime) = runtime {
                completion.step(
                    "restored admission owned drain",
                    lctx_model::domain::completion::complete(
                        Ok(()),
                        runtime.drain_owned_report().await,
                    ),
                );
                let settled = runtime.complete_signal(&completion).await;
                completion.step("restored admission signal finality", settled);
            }
            lctx_model::domain::completion::complete(result, completion)
        }
        .await;
        let mut completion = lctx_model::domain::completion::Completion::default();
        completion.cleanup(
            dump._directory.path().display().to_string(),
            dump._directory.close().map_err(ModelError::codec),
        );
        lctx_model::domain::completion::complete(result, completion)
    }
    .await;
    let mut completion = lctx_model::domain::completion::Completion::default();
    completion.cleanup(
        carrier.path().display().to_string(),
        carrier.close().map_err(ModelError::codec),
    );
    lctx_model::domain::completion::complete(result, completion)
}

fn hash_hex(value: &str) -> Result<ContentHash, ModelError> {
    Ok(ContentHash(
        hex::decode(value)
            .map_err(ModelError::codec)?
            .try_into()
            .map_err(|_| ModelError::Schema("restore hash width"))?,
    ))
}
fn hash_record(value: &RecordId, table: &str) -> Result<ContentHash, ModelError> {
    if value.table.as_str() != table {
        return Err(ModelError::Schema("restore hashed table identity"));
    }
    let lctx_native::surrealdb::types::RecordIdKey::String(key) = &value.key else {
        return Err(ModelError::Schema("restore hashed record key"));
    };
    hash_hex(key)
}
async fn compare_derived_claims(
    dump: &Dump,
    prepared: &PreparedClosure,
    loader: &lctx_native::Loader,
    budget: &ResourceBudget,
) -> Result<(), ModelError> {
    let views = prepared
        .views
        .iter()
        .map(|view| hash_hex(view))
        .collect::<Result<Vec<_>, _>>()?;
    let staging = loader.staging().ok_or(ModelError::Conflict(
        "restore derived comparison staging owner",
    ))?;
    let all = lctx_native::NativeReader::for_completed_views(
        loader.reader().shared_client(),
        staging.completed_owner(),
        &views,
        budget,
    )
    .await?
    .with_derivation(loader.derivation()?);
    let mut graph_tables = Vec::new();
    let mut search_tables = Vec::new();
    for family in RECOVERY_FAMILIES.iter().copied() {
        match family {
            RecoveryFamily::GraphRoles | RecoveryFamily::ExternalEndpoints => graph_tables
                .extend_from_slice(lctx_compiler_store::compiler::recovery_tables(family)),
            RecoveryFamily::SearchOccurrences
            | RecoveryFamily::SearchDocuments
            | RecoveryFamily::SearchVectors => search_tables
                .extend_from_slice(lctx_compiler_store::compiler::recovery_tables(family)),
            RecoveryFamily::Contributions
            | RecoveryFamily::Memberships
            | RecoveryFamily::Views
            | RecoveryFamily::TypedBacking
            | RecoveryFamily::Bindings
            | RecoveryFamily::Aliases
            | RecoveryFamily::GraphPayloads
            | RecoveryFamily::OriginalHeaders
            | RecoveryFamily::OriginalChunks => {}
        }
    }
    let result = async {
        for (tables, actual) in [
            (
                graph_tables,
                lctx_native::reconciliation::actual_roles(&all, budget)?,
            ),
            (
                search_tables,
                lctx_native::derived_search::actual_search_rows(
                    &loader.reader().with_budget(budget),
                    budget,
                )?,
            ),
        ] {
            let mut claimed = lctx_native::ordered_rows::SortedRows::with_budget(budget)?;
            for table in tables {
                for row in dump.rows(table)? {
                    let row = row?;
                    if prepared.keep_row(table, &row)? {
                        claimed.push(row)?;
                    }
                }
            }
            let mut actual = actual;
            claimed.finish()?.reconcile(&mut actual).await?;
        }
        Ok(())
    }
    .await;
    let mut completion = lctx_model::domain::completion::Completion::default();
    let released = all.release_preparation().await;
    if released.is_err() {
        all.retain_unknown();
    }
    completion.step("restore derived comparison preparation release", released);
    completion.step("restore derived comparison reader close", all.close().await);
    lctx_model::domain::completion::complete(result, completion)
}

async fn reconstruct(
    dump: &Dump,
    runtime: &Arc<lctx_compiler_store::admission::AdmissionRuntime>,
    prepared: &PreparedClosure,
    budget: &ResourceBudget,
    state: &Path,
    expected: &lctx_model::domain::completed_transport::CompletedTransportReceipt,
) -> Result<lctx_compiler_store::artifact::RestoredAdmission, ModelError> {
    let native = runtime.native();
    let manifest = &prepared.manifest;
    let nodes = &prepared.nodes;
    let mut batch_charge = budget.reserve("restore-canonical-ingress", 0)?;
    for &table in lctx_compiler_store::compiler::recovery_tables(RecoveryFamily::GraphPayloads) {
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
                if record_id(&row)? != &lctx_native::loader::entity_payload_id(&entity)? {
                    return Err(ModelError::Conflict("restore entity canonical address"));
                }
                entities.push(entity);
            } else {
                let assertion =
                    serde_json::from_slice::<Assertion>(payload).map_err(ModelError::codec)?;
                if record_id(&row)? != &lctx_native::loader::assertion_payload_id(&assertion)? {
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
    for (blob_id, blob) in &prepared.blobs {
        let mut file = tempfile::tempfile().map_err(ModelError::codec)?;
        let mut seen = BTreeSet::new();
        let mut total = 0u64;
        let mut chunks_charge = budget.reserve("restore-original-chunk-positions", 0)?;
        for row in dump.original_rows(blob_id)?.into_iter().flatten() {
            let piece = lctx_native::originals::decode_piece(row?, blob)?;
            if !seen.insert(piece.start) {
                return Err(ModelError::Conflict("restore duplicate original piece"));
            }
            chunks_charge.try_resize(seen.len().saturating_mul(64))?;
            file.seek(std::io::SeekFrom::Start(piece.start))
                .map_err(ModelError::codec)?;
            file.write_all(&piece.bytes).map_err(ModelError::codec)?;
            total += piece.bytes.len() as u64;
        }
        if total != blob.byte_len {
            return Err(ModelError::Conflict("restore original length"));
        }
        for (source, layout) in &prepared.layouts {
            if &layout.blob.id() != blob_id {
                continue;
            }
            let original = &prepared.originals[source];
            file.rewind().map_err(ModelError::codec)?;
            // Destination admission earns completion and full-byte warm replay afresh.
            native
                .import_original_stream(
                    original.source.0,
                    original.content,
                    original.byte_len,
                    &mut file,
                    budget,
                )
                .await?;
        }
    }
    native.import_state(state, expected, budget).await?;
    runtime.restore(manifest.profile).await?;
    lctx_compiler_store::artifact::verify_restored(runtime, manifest).await
}

#[cfg(test)]
#[path = "../../lctx-serving/tests/fixtures/scoped.rs"]
mod cold_claim_fixture;

#[cfg(test)]
mod definition_tests {
    use super::*;

    use lctx_model::domain::{
        Record, Relation,
        analysis::sources::SourceSnapshot,
        completed::{CompletedContribution, CompletedView, ContributionSpec, OutputContent},
        graph::{EntityId, Original},
        input::Package,
        serving::{DatabaseIdentity, Name},
        stages::Profile,
    };
    use lctx_native::surrealdb::types::Object;

    #[tokio::test(flavor = "multi_thread")]
    async fn cold_restore_compares_inert_role_and_occurrence_claims_against_regenerated_native_rows()
     {
        let config = super::cold_claim_fixture::config();
        let package = Package {
            name: format!(
                "cold-claim-{}",
                lctx_native::control::fresh_identity("cold-claim")
                    .unwrap()
                    .hex()
            ),
        };
        let release = lctx_model::domain::input::Release {
            package: package.id(),
            version: "1".into(),
        };
        let entities = vec![Entity::from(package), Entity::from(release)];
        let fixture = super::cold_claim_fixture::reader(&config, &entities, &[])
            .await
            .unwrap();
        let control_budget = budget();
        // This partial fixture completes views without creating publication bindings.
        // Preserve its exact selected views rather than reconstructing an empty selection.
        let selected = fixture.view_bindings();
        let Some(Value::Array(selected)) = selected.get("lctx_views") else {
            panic!("scoped fixture selected native views");
        };
        let views = selected
            .iter()
            .map(|value| {
                let Value::RecordId(id) = value else {
                    panic!("scoped fixture native view ID");
                };
                hash_record(id, "compiler_view").unwrap()
            })
            .collect::<Vec<_>>();
        assert_eq!(
            views.len(),
            2,
            "Package and Release completed relation views"
        );
        let loader = lctx_native::Loader::for_views(fixture.shared_client(), views.clone())
            .with_budget(&control_budget);
        let mut actual =
            lctx_native::reconciliation::actual_roles(&loader.reader(), &control_budget).unwrap();
        let mut baseline = Vec::new();
        while let Some(row) = actual.next().await.unwrap() {
            baseline.push(row);
        }
        actual.drain_transport().await.unwrap();
        assert_eq!(
            baseline.len(),
            1,
            "one actual Package reference is independently generated from Release canonical data"
        );
        for fault in ["baseline", "missing", "changed", "extra", "occurrence"] {
            let mut claims = baseline.clone();
            if fault == "missing" {
                claims.clear();
            }
            if fault == "changed" {
                let Value::Object(row) = &mut claims[0] else {
                    unreachable!()
                };
                row.insert("field", "forged-role");
            }
            if fault == "extra" {
                let mut extra = claims[0].clone();
                let Value::Object(row) = &mut extra else {
                    unreachable!()
                };
                row.insert("id", RecordId::new("reference", "unexpected-late-role"));
                claims.push(extra);
            }
            if fault == "occurrence" {
                let mut extra = row("lex_occurs", "unexpected-late-occurrence");
                extra.insert(
                    "unit_payload",
                    lctx_native::loader::entity_payload_id(&entities[0]).unwrap(),
                );
                extra.insert("in", RecordId::new("search_source", "unexpected-value"));
                claims.push(Value::Object(extra));
            }
            let scratch = tempfile::tempdir().unwrap();
            let path = scratch.path().join("claims.surql");
            let mut file = std::fs::File::create(&path).unwrap();
            writeln!(file, "OPTION IMPORT;").unwrap();
            let budget = budget();
            let mut encoded = budget.reserve("cold-claim-encoded", 0).unwrap();
            for claim in &claims {
                crate::backup_import::write_row(&mut file, claim.clone(), encoded.as_mut())
                    .unwrap();
            }
            drop(file);
            let dump = Dump::decode(&path, &budget).unwrap();
            let fake = claimed_fixture();
            let prepared = PreparedClosure {
                handle: fake.handle,
                manifest: Manifest::decode(
                    bytes(
                        object(
                            fake.rows
                                .iter()
                                .find(|row| record_id(row).unwrap().table.as_str() == "publication")
                                .unwrap(),
                        )
                        .unwrap(),
                        "manifest",
                    )
                    .unwrap(),
                )
                .unwrap(),
                bindings: vec![],
                originals: BTreeMap::new(),
                layouts: BTreeMap::new(),
                blobs: BTreeMap::new(),
                binding_rows: BTreeSet::new(),
                owners: BTreeSet::new(),
                aliases: BTreeSet::new(),
                views: views.iter().map(|view| view.hex()).collect(),
                nodes: entities
                    .iter()
                    .map(|entity| lctx_native::loader::entity_payload_id(entity).unwrap())
                    .collect(),
                derived_rows: claims
                    .iter()
                    .map(|claim| record_id(claim).unwrap().clone())
                    .collect(),
                _charge: budget.reserve("cold-claim-test-selection", 1024).unwrap(),
                _graph_charge: budget.reserve("cold-claim-test-graph", 1024).unwrap(),
            };
            let compared = compare_derived_claims(&dump, &prepared, &loader, &budget).await;
            assert_eq!(
                compared.is_ok(),
                fault == "baseline",
                "source claim {fault}: {compared:?}"
            );
        }
        drop(loader);
        fixture.close().await.unwrap();
    }
    #[test]
    fn buffered_spool_reports_final_flush_failure_without_drop_retry() {
        use std::sync::atomic::{AtomicUsize, Ordering};
        struct FaultSink(Arc<AtomicUsize>);
        impl Write for FaultSink {
            fn write(&mut self, _: &[u8]) -> std::io::Result<usize> {
                self.0.fetch_add(1, Ordering::SeqCst);
                Err(std::io::Error::new(
                    std::io::ErrorKind::BrokenPipe,
                    "spool sink failure",
                ))
            }
            fn flush(&mut self) -> std::io::Result<()> {
                Ok(())
            }
        }
        let attempts = Arc::new(AtomicUsize::new(0));
        let mut writer =
            std::io::BufWriter::with_capacity(SPOOL_BUFFER_BYTES, FaultSink(attempts.clone()));
        writer.write_all(b"provisional claimed row").unwrap();
        assert_eq!(attempts.load(Ordering::SeqCst), 0);
        let error = finish_spool(writer).unwrap_err();
        assert!(error.to_string().contains("spool sink failure"), "{error}");
        assert_eq!(
            attempts.load(Ordering::SeqCst),
            1,
            "failed final flush must not be silently retried by Drop"
        );
    }

    // Claimed transport metadata exercises pure preparation only, never semantic admission.
    struct ClaimedFixture {
        rows: Vec<Value>,
        handle: SnapshotHandle,
        nodes: BTreeSet<RecordId>,
    }
    fn row(table: &str, id: &str) -> Object {
        let mut value = Object::new();
        value.insert("id", RecordId::new(table, id));
        value
    }
    fn descriptor_row(table: &str, id: &str, descriptor: &impl serde::Serialize) -> Object {
        let mut value = row(table, id);
        value.insert(
            "descriptor",
            Bytes::from(serde_json::to_vec(descriptor).unwrap()),
        );
        value
    }
    fn claimed_fixture() -> ClaimedFixture {
        let model = lctx_model::domain::model().unwrap();
        let relation = Relation::of::<Package>();
        let descriptor = |producer: &str, inputs: Vec<SourceSnapshot>, extra: bool| {
            let mut outputs = BTreeMap::from([(
                Package::NAME.into(),
                OutputContent {
                    rows: 1,
                    content: ContentHash::of(producer.as_bytes()),
                },
            )]);
            if extra {
                outputs.insert(
                    "other_output".into(),
                    OutputContent {
                        rows: 1,
                        content: ContentHash::of(b"extra"),
                    },
                );
            }
            CompletedContribution { assurance: lctx_model::domain::producer_assurance::ProducerAssurance::CurrentProviderInterface,
                spec: ContributionSpec {
                    captured_binding: None,
                    producer: producer.into(),
                    profile: lctx_model::domain::completed::OutputProfile::Specific(Profile::Catalog),
                    model: model.digest(),
                    implementation: ContentHash::of(b"pure-transport-test"),
                    configuration: None,
                    inputs,
                    outputs: outputs.keys().cloned().collect(),
                },
                outcome: 0,
                outputs,
            }
        };
        let seed = descriptor("seed", vec![], false);
        let seed_view = CompletedView::new(
            Package::NAME.into(),
            BTreeSet::from([seed.identity().unwrap()]),
            1,
        )
        .unwrap();
        let seed_source =
            SourceSnapshot::of_completed_view(&relation, model.digest(), &seed_view).unwrap();
        let root = descriptor("root", vec![seed_source], true);
        let root_view = CompletedView::new(
            Package::NAME.into(),
            BTreeSet::from([root.identity().unwrap()]),
            1,
        )
        .unwrap();
        let binding = CompletedBinding {
            boundary: None,
            source: SourceSnapshot::of_completed_view(&relation, model.digest(), &root_view)
                .unwrap(),
            view: root_view.clone(),
            configuration: None,
        };
        let mut rows = Vec::new();
        for (id, descriptor) in [
            ("a-seed", &seed),
            ("z-equal-seed", &seed),
            ("a-root", &root),
            ("z-equal-root", &root),
        ] {
            let mut value = descriptor_row("compiler_contribution", id, descriptor);
            value.insert("completed", true);
            rows.push(Value::Object(value));
        }
        for view in [&seed_view, &root_view] {
            rows.push(Value::Object(descriptor_row(
                "compiler_view",
                &view.identity.hex(),
                view,
            )));
        }
        let mut binding_row = descriptor_row("compiler_binding", "selected-binding", &binding);
        binding_row.insert("attempt", RecordId::new("native_attempt", "claimed"));
        rows.push(Value::Object(binding_row));
        let mut nodes = BTreeSet::from([
            RecordId::new("entity", "seed"),
            RecordId::new("entity", "root"),
            RecordId::new("compiler_record", "extra"),
            RecordId::new("entity", "alias-a"),
            RecordId::new("compiler_record", "alias-b"),
        ]);
        for (id, owner, table, node) in [
            ("seed-member", "a-seed", "entity", "seed"),
            ("root-member", "a-root", "entity", "root"),
            ("other-output-member", "a-root", "compiler_record", "extra"),
            (
                "unrelated-owner-member",
                "z-equal-root",
                "entity",
                "unrelated",
            ),
        ] {
            let mut value = row("compiler_membership", id);
            value.insert(
                "contribution",
                RecordId::new("compiler_contribution", owner),
            );
            value.insert("node", RecordId::new(table, node));
            rows.push(Value::Object(value));
        }
        // Reverse dependency order prevents a file-order-only alias walk from passing.
        for (id, source, table, target) in [
            ("alias-second", "alias-a", "compiler_record", "alias-b"),
            ("alias-first", "root", "entity", "alias-a"),
        ] {
            let mut value = row("compiler_alias", id);
            value.insert("source", RecordId::new("entity", source));
            value.insert("target", RecordId::new(table, target));
            rows.push(Value::Object(value));
        }
        for id in nodes
            .iter()
            .cloned()
            .chain(std::iter::once(RecordId::new("entity", "unrelated")))
        {
            let mut value = Object::new();
            value.insert("id", id);
            value.insert(
                "canonical",
                Bytes::from(b"unchanged claimed canonical bytes".to_vec()),
            );
            rows.push(Value::Object(value));
        }
        let original = b"original; bytes remain exact";
        let input: lctx_model::domain::Id<lctx_model::domain::input::InputRevision> =
            serde_json::from_value(serde_json::to_value([3u8; 16]).unwrap()).unwrap();
        let source_row = lctx_model::domain::source::SourceArtifact::from_bytes(
            input,
            "claimed.py".into(),
            original,
        )
        .unwrap();
        let layout = lctx_native::originals::Layout::source(&source_row).unwrap();
        let source = lctx_model::domain::graph::EntityId::of(source_row.id()).0;
        nodes.insert(layout.artifact.clone());
        let mut artifact = Object::new();
        artifact.insert("id", layout.artifact.clone());
        artifact.insert(
            "canonical",
            Bytes::from(serde_json::to_vec(&Entity::Source(source_row)).unwrap()),
        );
        rows.push(Value::Object(artifact));
        let mut member = row("compiler_membership", "source-artifact-member");
        member.insert(
            "contribution",
            RecordId::new("compiler_contribution", "a-root"),
        );
        member.insert("node", layout.artifact.clone());
        rows.push(Value::Object(member));
        rows.push(layout.value());
        rows.push(layout.blob.value(true));
        rows.push(layout.blob.piece(0, original).unwrap());
        let manifest = Manifest {
            admission_contract: ContentHash::of(b"claimed-only"),
            completed_state: lctx_model::domain::completed::CompletedStateIdentity {
                format_version: lctx_model::domain::completed::STATE_FORMAT_VERSION,
                contributions: 2,
                memberships: 3,
                backing_rows: 1,
                content: ContentHash::of(b"claimed-state-not-admitted"),
            },
            format_version: lctx_model::domain::graph::ARTIFACT_FORMAT_VERSION,
            frontier: lctx_model::domain::admission::Frontier::Facts,
            profile: Profile::Catalog,
            captures: vec![],
            semantic_contract: semantic_contract(&model),
            producers: vec![],
            settings: ContentHash::of(b"selected-settings"),
            families: vec![],
            required_outcomes: vec![],
            outcomes: vec![],
            originals: vec![Original {
                source: EntityId(source),
                content: ContentHash::of(original),
                byte_len: original.len() as u64,
            }],
            projections: vec![],
            embeddings: vec![],
        };
        let mut handle = SnapshotHandle {
            publication: ContentHash([0; 32]),
            eligibility: lctx_model::domain::completed_transport::EligibilityIdentity(
                ContentHash::of(b"claimed-eligibility"),
            ),
            semantic: manifest.content(),
            realization: ContentHash::of(b"claimed-realization"),
            view: crate::native_publication::view_identity(std::slice::from_ref(&binding)).unwrap(),
            service_generation: ContentHash::of(b"claimed-generation"),
            definition_epoch: ContentHash::of(b"claimed-epoch"),
            database: DatabaseIdentity {
                namespace: Name::new("library_context").unwrap(),
                database: Name::new("validation").unwrap(),
            },
        };
        handle.publication = handle.expected_publication();
        for unrelated in [false, true] {
            let mut handle = handle.clone();
            let mut manifest = manifest.clone();
            if unrelated {
                manifest.settings = ContentHash::of(b"unrelated-settings");
                handle.semantic = manifest.content();
                handle.publication = handle.expected_publication();
            }
            let mut value = row("publication", &handle.publication.hex());
            value.insert("handle", hex::encode(serde_json::to_vec(&handle).unwrap()));
            value.insert(
                "manifest",
                Bytes::from(serde_json::to_vec(&manifest).unwrap()),
            );
            value.insert(
                "views",
                Bytes::from(serde_json::to_vec(&vec![binding.clone()]).unwrap()),
            );
            value.insert("definition_epoch", handle.definition_epoch.hex());
            rows.push(Value::Object(value));
        }
        ClaimedFixture {
            rows,
            handle,
            nodes,
        }
    }
    // These deliberately provisional graph selections test dump lookup/claim validation,
    // not transport correspondence or admission. The current wire has its own codec controls.
    fn claimed_graph(fixture: &ClaimedFixture, budget: &ResourceBudget) -> DumpGraph {
        DumpGraph {
            nodes: fixture.nodes.clone(),
            aliases: fixture
                .rows
                .iter()
                .filter(|row| record_id(row).unwrap().table.as_str() == "compiler_alias")
                .map(|row| record_id(row).unwrap().clone())
                .collect(),
            _charge: budget.reserve("claimed-graph-control", 8192).unwrap(),
        }
    }
    fn write_claimed(path: &Path, rows: &[Value]) {
        use lctx_native::surrealdb::types::ToSql;
        let mut file = std::fs::File::create(path).unwrap();
        writeln!(
            file,
            "OPTION IMPORT; DEFINE FIELD OVERWRITE canonical ON entity TYPE bytes PERMISSIONS FULL;"
        )
        .unwrap();
        for row in rows {
            writeln!(
                file,
                "INSERT {};",
                Value::Array(vec![row.clone()].into()).to_sql()
            )
            .unwrap();
        }
    }
    fn budget() -> ResourceBudget {
        ResourceBudget::fixed(lctx_model::domain::resources::DEFAULT_MEMORY_BYTES).unwrap()
    }
    #[test]
    fn transport_graph_keeps_alias_chains_within_each_roots_copy_selection() {
        use lctx_model::domain::{
            completed_identity::{CompletedFamilyHasher, CompletedFamilyRoots},
            completed_transport::*,
        };
        let budget = budget();
        let mut descriptor = CompletedContribution {
            assurance:
                lctx_model::domain::producer_assurance::ProducerAssurance::CurrentProviderInterface,
            spec: ContributionSpec {
                captured_binding: None,
                producer: "transport-graph-control".into(),
                profile: lctx_model::domain::completed::OutputProfile::Specific(Profile::Catalog),
                model: lctx_model::domain::model().unwrap().digest(),
                implementation: ContentHash::of(b"graph control"),
                configuration: None,
                inputs: vec![],
                outputs: BTreeSet::from(["packages".into(), "releases".into()]),
            },
            outcome: 0,
            outputs: BTreeMap::new(),
        };
        for relation in ["packages", "releases"] {
            descriptor.outputs.insert(
                relation.into(),
                OutputContent {
                    rows: 1,
                    content: ContentHash::of(relation.as_bytes()),
                },
            );
        }
        let logical = descriptor.identity().unwrap();
        let mut views = [
            CompletedView::new("packages".into(), BTreeSet::from([logical]), 1).unwrap(),
            CompletedView::new("releases".into(), BTreeSet::from([logical]), 1).unwrap(),
        ];
        views.sort_by_key(|view| view.identity);
        let shared = RecordId::new("entity", "shared");
        let middle = RecordId::new("entity", "middle");
        let foreign = RecordId::new("entity", "foreign");
        let mut member = Object::new();
        member.insert(
            "id",
            lctx_native::completed_rows::membership_id(logical, "packages", &[1; 16]),
        );
        member.insert(
            "contribution",
            RecordId::new("compiler_contribution", logical.hex()),
        );
        member.insert("relation", "packages");
        member.insert("semantic_key", hex::encode([1; 16]));
        member.insert("node", shared.clone());
        member.insert("content", ContentHash::of(b"shared").hex());
        let member = Value::Object(member);
        let alias = |source: RecordId, target: RecordId| {
            let mut sink = lctx_model::domain::KeySink::new("native-place-alias/v1");
            sink.part(b"source", &serde_json::to_vec(&source).unwrap());
            sink.part(b"target", &serde_json::to_vec(&target).unwrap());
            let mut row = Object::new();
            row.insert("id", RecordId::new("compiler_alias", sink.finish().hex()));
            row.insert("source", source);
            row.insert("target", target);
            Value::Object(row)
        };
        let aliases = [
            alias(shared.clone(), middle.clone()),
            alias(middle.clone(), foreign.clone()),
        ];
        let inventory = |kind, rows: &[Value]| {
            let mut hash = TransportInventoryHasher::new(kind);
            for row in rows {
                hash.row(&serde_json::to_vec(row).unwrap()).unwrap();
            }
            hash.finish()
        };
        let copies = (0..2)
            .map(|label| TransportCopy {
                label,
                descriptor: descriptor.clone(),
                membership: inventory(
                    TransportInventoryKind::Memberships,
                    std::slice::from_ref(&member),
                ),
                aliases: inventory(
                    TransportInventoryKind::Aliases,
                    std::slice::from_ref(&aliases[label as usize]),
                ),
            })
            .collect::<Vec<_>>();
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
        let mut family_rows: [Vec<Value>; 6] = std::array::from_fn(|_| vec![]);
        family_rows[0].push(Value::Object(descriptor_row(
            "compiler_contribution",
            &logical.hex(),
            &descriptor,
        )));
        family_rows[1].push(member.clone());
        for view in &views {
            family_rows[2].push(Value::Object(descriptor_row(
                "compiler_view",
                &view.identity.hex(),
                view,
            )));
        }
        family_rows[5].extend(aliases.clone());
        let mut family_roots = Vec::new();
        for (index, rows) in family_rows.iter_mut().enumerate() {
            rows.sort_by_key(|row| record_id(row).unwrap().clone());
            let mut hash = CompletedFamilyHasher::new(
                lctx_model::domain::recovery_closure::COMPLETED_FAMILIES[index],
            )
            .unwrap();
            for row in rows {
                hash.row(&serde_json::to_vec(row).unwrap()).unwrap();
            }
            family_roots.push(hash.finish());
        }
        let families = CompletedFamilyRoots::new(family_roots.try_into().unwrap()).unwrap();
        let envelope = CompletedTransportEnvelope {
            version: 1,
            semantic: lctx_model::domain::completed::CompletedStateIdentity::from_roots(&families)
                .unwrap(),
            roots,
            copies,
            families,
        };
        let receipt = envelope.receipt().unwrap();
        let scratch = tempfile::tempdir().unwrap();
        let path = scratch.path().join("wire");
        let mut wire = std::fs::File::create(&path).unwrap();
        let mut frame = |value: serde_json::Value| {
            serde_json::to_writer(&mut wire, &value).unwrap();
            wire.write_all(b"\n").unwrap();
        };
        frame(serde_json::json!({"header":{"envelope":envelope}}));
        for (index, rows) in family_rows.iter().enumerate() {
            for row in rows {
                frame(
                    serde_json::json!({"row":{"table":lctx_compiler_store::compiler::STATE_TABLES[index],"row":row}}),
                );
            }
        }
        for label in 0..2 {
            frame(serde_json::json!({"copy_membership":{"copy":label,"row":member}}));
            frame(serde_json::json!({"copy_alias":{"copy":label,"row":aliases[label]}}));
        }
        frame(serde_json::json!({"end":{"receipt":receipt}}));
        drop(wire);
        let decoded =
            lctx_native::completed_transport::read_transport(&path, &receipt, &budget).unwrap();
        let graph = DumpGraph::from_transport(&decoded, &budget).unwrap();
        assert_eq!(graph.nodes, BTreeSet::from([shared, middle]));
        assert!(
            !graph.nodes.contains(&foreign),
            "a second root's unreachable source must not continue the first root's alias chain"
        );
    }

    fn original_lookup_fixture() -> (
        Vec<Value>,
        Vec<Original>,
        BTreeSet<RecordId>,
        BTreeSet<RecordId>,
    ) {
        let input = serde_json::from_value(serde_json::to_value([11u8; 16]).unwrap()).unwrap();
        let mut rows = Vec::new();
        let mut declared = Vec::new();
        let mut demands = BTreeSet::new();
        let mut nodes = BTreeSet::new();
        let mut blobs = BTreeSet::new();
        for (index, path, bytes) in [
            (0, "late.py", b"shared bytes".as_slice()),
            (1, "first.py", b"other bytes".as_slice()),
            (2, "same.py", b"shared bytes".as_slice()),
        ] {
            let source =
                lctx_model::domain::source::SourceArtifact::from_bytes(input, path.into(), bytes)
                    .unwrap();
            let layout = lctx_native::originals::Layout::source(&source).unwrap();
            let mut entity = Object::new();
            entity.insert("id", layout.artifact.clone());
            entity.insert(
                "canonical",
                Bytes::from(serde_json::to_vec(&Entity::Source(source)).unwrap()),
            );
            rows.push(Value::Object(entity));
            rows.push(layout.value());
            if blobs.insert(layout.blob.id()) {
                rows.push(layout.blob.value(true));
            }
            nodes.insert(layout.artifact.clone());
            demands.insert(layout.id.clone());
            if index == 0 {
                declared.push(Original {
                    source: EntityId(hash_record(&layout.id, "original").unwrap()),
                    content: layout.blob.content,
                    byte_len: layout.blob.byte_len,
                });
            }
        }
        rows.reverse();
        (rows, declared, demands, nodes)
    }
    #[test]
    fn original_lookup_identity_runs_spill_and_refuse_duplicate_after_complete_prefix() {
        for duplicate in [false, true] {
            let budget = ResourceBudget::fixed(1024 * 1024).unwrap();
            let mut identities = DumpIdentityCheck::new(&budget).unwrap();
            for index in 0..10_000 {
                identities
                    .include(
                        "entity",
                        &Value::Object(row("entity", &format!("{index:064x}"))),
                    )
                    .unwrap();
            }
            let result = if duplicate {
                identities
                    .include(
                        "entity",
                        &Value::Object(row("entity", &format!("{:064x}", 0))),
                    )
                    .and_then(|()| identities.finish())
            } else {
                identities.finish()
            };
            assert_eq!(
                result.is_ok(),
                !duplicate,
                "complete identity runs, duplicate={duplicate}: {result:?}"
            );
            assert_eq!(budget.reserved(), 0);
        }
    }

    #[test]
    fn original_lookup_prepares_shuffled_selected_headers_sources_and_reused_blobs_once() {
        let (rows, declared, demands, nodes) = original_lookup_fixture();
        let scratch = tempfile::tempdir().unwrap();
        let path = scratch.path().join("lookup.surql");
        write_claimed(&path, &rows);
        let budget = budget();
        let dump = Dump::decode(&path, &budget).unwrap();
        let mut charge = budget.reserve("lookup-control", 0).unwrap();
        let (originals, layouts, blobs) =
            prepare_originals(&dump, &declared, &demands, &nodes, charge.as_mut(), &budget)
                .unwrap();
        assert_eq!(originals.len(), 3);
        assert_eq!(layouts.len(), 3);
        assert_eq!(blobs.len(), 2);
        assert_eq!(
            originals[&RecordId::new("original", declared[0].source.0.hex())],
            declared[0]
        );
        assert!(
            charge.size() >= 3 * 2048,
            "retained typed metadata is charged"
        );
    }
    #[test]
    fn original_lookup_rejects_missing_duplicate_foreign_and_corrupt_unretained_records() {
        for fault in [
            "missing-header",
            "missing-source",
            "missing-blob",
            "duplicate-header",
            "duplicate-source",
            "duplicate-blob",
            "corrupt-unretained",
            "foreign-artifact",
        ] {
            let (mut rows, declared, demands, nodes) = original_lookup_fixture();
            let table = if fault.contains("source") {
                "entity"
            } else if fault.contains("blob") {
                "original_blob"
            } else {
                "original"
            };
            let index = rows
                .iter()
                .position(|value| record_id(value).unwrap().table.as_str() == table)
                .unwrap();
            if fault.starts_with("missing") {
                rows.remove(index);
            } else if fault.starts_with("duplicate") {
                rows.push(rows[index].clone());
            } else if fault == "corrupt-unretained" {
                let mut row = rows[index].clone();
                let Value::Object(value) = &mut row else {
                    unreachable!()
                };
                value.insert(
                    "id",
                    RecordId::new("original", ContentHash::of(b"unretained").hex()),
                );
                value.insert("unexpected", true);
                rows.push(row);
            } else {
                let Value::Object(value) = &mut rows[index] else {
                    unreachable!()
                };
                value.insert("artifact", RecordId::new("entity", "foreign"));
            }
            let scratch = tempfile::tempdir().unwrap();
            let path = scratch.path().join("fault.surql");
            write_claimed(&path, &rows);
            let budget = budget();
            let dump = Dump::decode(&path, &budget).unwrap();
            let mut charge = budget.reserve("lookup-control", 0).unwrap();
            assert!(
                prepare_originals(&dump, &declared, &demands, &nodes, charge.as_mut(), &budget)
                    .is_err(),
                "accepted {fault}"
            );
        }
    }
    #[test]
    fn original_lookup_consumes_late_spool_failure_after_all_demands() {
        let (rows, declared, demands, nodes) = original_lookup_fixture();
        let scratch = tempfile::tempdir().unwrap();
        let path = scratch.path().join("late.surql");
        write_claimed(&path, &rows);
        let budget = budget();
        let dump = Dump::decode(&path, &budget).unwrap();
        let mut spool = std::fs::OpenOptions::new()
            .append(true)
            .open(dump._directory.path().join("original"))
            .unwrap();
        spool.write_all(b"{broken late spool\n").unwrap();
        drop(spool);
        let mut charge = budget.reserve("lookup-control", 0).unwrap();
        assert!(
            prepare_originals(&dump, &declared, &demands, &nodes, charge.as_mut(), &budget)
                .is_err()
        );
    }

    #[test]
    fn every_recovery_family_dispatches_exact_restore_selection_without_table_fallback() {
        let fixture = claimed_fixture();
        let scratch = tempfile::tempdir().unwrap();
        let input = scratch.path().join("families.surql");
        write_claimed(&input, &fixture.rows);
        let budget = budget();
        let dump = Dump::decode(&input, &budget).unwrap();
        let mut prepared = PreparedClosure::prepare(
            &dump,
            Some(fixture.handle.publication),
            &budget,
            claimed_graph(&fixture, &budget),
        )
        .unwrap();
        for family in RECOVERY_FAMILIES.iter().copied() {
            for table in lctx_compiler_store::compiler::recovery_tables(family) {
                let mut selected = row(table, "family-control");
                match family {
                    RecoveryFamily::Contributions => {
                        selected.insert("id", prepared.owners.first().unwrap().clone());
                    }
                    RecoveryFamily::Memberships => {
                        selected.insert("contribution", prepared.owners.first().unwrap().clone());
                    }
                    RecoveryFamily::Views => {
                        selected.insert(
                            "id",
                            RecordId::new(*table, prepared.views.first().unwrap().clone()),
                        );
                    }
                    RecoveryFamily::TypedBacking | RecoveryFamily::GraphPayloads => {
                        prepared
                            .nodes
                            .insert(RecordId::new(*table, "family-control"));
                    }
                    RecoveryFamily::Bindings => {
                        prepared
                            .binding_rows
                            .insert(RecordId::new(*table, "family-control"));
                    }
                    RecoveryFamily::Aliases => {
                        prepared
                            .aliases
                            .insert(RecordId::new(*table, "family-control"));
                    }
                    RecoveryFamily::OriginalHeaders => {
                        selected.insert(
                            "id",
                            if *table == "original" {
                                prepared.layouts.keys().next().unwrap().clone()
                            } else {
                                prepared.blobs.keys().next().unwrap().clone()
                            },
                        );
                    }
                    RecoveryFamily::OriginalChunks => {
                        selected.insert("blob", prepared.blobs.keys().next().unwrap().clone());
                    }
                    RecoveryFamily::GraphRoles
                    | RecoveryFamily::ExternalEndpoints
                    | RecoveryFamily::SearchOccurrences
                    | RecoveryFamily::SearchDocuments
                    | RecoveryFamily::SearchVectors => {
                        prepared
                            .derived_rows
                            .insert(RecordId::new(*table, "family-control"));
                    }
                }
                assert!(
                    prepared.keep_row(table, &Value::Object(selected)).unwrap(),
                    "explicit family selector: {family:?}/{table}"
                );
                let mut unrelated = row(table, "unselected-control");
                unrelated.insert(
                    "contribution",
                    RecordId::new("compiler_contribution", "unselected"),
                );
                unrelated.insert("source", RecordId::new("original", "unselected"));
                unrelated.insert("blob", RecordId::new("original_blob", "unselected"));
                assert!(
                    !prepared.keep_row(table, &Value::Object(unrelated)).unwrap(),
                    "exact exclusion: {family:?}/{table}"
                );
            }
        }
        assert!(
            prepared
                .keep_row("native_pin", &Value::Object(row("native_pin", "control")))
                .is_err()
        );
        let publication = fixture
            .rows
            .iter()
            .find(|row| {
                record_id(row).unwrap().table.as_str() == "publication"
                    && record_id(row).unwrap().key
                        == lctx_native::surrealdb::types::RecordIdKey::String(
                            fixture.handle.publication.hex(),
                        )
            })
            .unwrap();
        assert!(
            prepared.keep_row("publication", publication).unwrap(),
            "publication envelope remains separate from completed content"
        );
    }
    #[test]
    fn claimed_closure_refuses_missing_payload_binding_original_and_selected_duplicates() {
        let fixture = claimed_fixture();
        let scratch = tempfile::tempdir().unwrap();
        let input = scratch.path().join("raw.surql");
        for missing in [
            RecordId::new("entity", "root"),
            RecordId::new("entity", "seed"),
            RecordId::new("compiler_record", "extra"),
            RecordId::new("compiler_record", "alias-b"),
            RecordId::new("compiler_binding", "selected-binding"),
            lctx_native::originals::chunk_id(
                &lctx_native::originals::Blob::new(
                    ContentHash::of(b"original; bytes remain exact"),
                    b"original; bytes remain exact".len() as u64,
                )
                .unwrap()
                .id(),
                0,
            )
            .unwrap(),
        ] {
            let rows = fixture
                .rows
                .iter()
                .filter(|row| {
                    record_id(row).unwrap() != &missing
                        && !(missing.table.as_str() == "compiler_binding"
                            && record_id(row).unwrap().table.as_str() == "compiler_binding")
                })
                .cloned()
                .collect::<Vec<_>>();
            write_claimed(&input, &rows);
            let dump = Dump::decode(&input, &budget()).unwrap();
            assert!(
                PreparedClosure::prepare(
                    &dump,
                    Some(fixture.handle.publication),
                    &budget(),
                    claimed_graph(&fixture, &budget())
                )
                .is_err(),
                "accepted omitted {missing:?}"
            );
        }
        for duplicate_id in [
            RecordId::new("entity", "root"),
            RecordId::new("compiler_binding", "selected-binding"),
            RecordId::new("compiler_membership", "root-member"),
            RecordId::new("compiler_alias", "alias-first"),
        ] {
            let mut duplicate = fixture.rows.clone();
            duplicate.push(
                fixture
                    .rows
                    .iter()
                    .find(|row| record_id(row).unwrap() == &duplicate_id)
                    .unwrap()
                    .clone(),
            );
            write_claimed(&input, &duplicate);
            let dump = Dump::decode(&input, &budget()).unwrap();
            assert!(
                PreparedClosure::prepare(
                    &dump,
                    Some(fixture.handle.publication),
                    &budget(),
                    claimed_graph(&fixture, &budget())
                )
                .is_err(),
                "accepted duplicate {duplicate_id:?}"
            );
        }
    }
    #[test]
    fn claimed_closure_refuses_missing_dependencies_and_corrupt_original_metadata() {
        let fixture = claimed_fixture();
        let scratch = tempfile::tempdir().unwrap();
        let input = scratch.path().join("raw.surql");
        for missing_table in ["compiler_view", "compiler_contribution"] {
            let rows = fixture
                .rows
                .iter()
                .filter(|row| record_id(row).unwrap().table.as_str() != missing_table)
                .cloned()
                .collect::<Vec<_>>();
            write_claimed(&input, &rows);
            let dump = Dump::decode(&input, &budget()).unwrap();
            assert!(
                PreparedClosure::prepare(
                    &dump,
                    Some(fixture.handle.publication),
                    &budget(),
                    claimed_graph(&fixture, &budget())
                )
                .is_err()
            );
        }
        for corruption in 0..4 {
            let mut rows = fixture.rows.clone();
            let table = if corruption == 0 {
                "original"
            } else {
                "original_chunk"
            };
            let Value::Object(row) = rows
                .iter_mut()
                .find(|row| record_id(row).unwrap().table.as_str() == table)
                .unwrap()
            else {
                unreachable!()
            };
            match corruption {
                0 => {
                    row.insert("content", ContentHash::of(b"different header").hex());
                }
                1 => {
                    row.insert("content", ContentHash::of(b"different checksum").hex());
                }
                2 => {
                    row.insert("start", 1i64);
                }
                _ => {
                    let mut changed = bytes(row, "bytes").unwrap().to_vec();
                    changed[0] ^= 1;
                    row.insert("bytes", Bytes::from(changed.clone()));
                    row.insert("content", ContentHash::of(&changed).hex());
                }
            }
            write_claimed(&input, &rows);
            let dump = Dump::decode(&input, &budget()).unwrap();
            assert!(
                PreparedClosure::prepare(
                    &dump,
                    Some(fixture.handle.publication),
                    &budget(),
                    claimed_graph(&fixture, &budget())
                )
                .is_err(),
                "accepted original corruption {corruption}"
            );
        }
    }
    #[test]
    fn claimed_dump_checks_unselected_executable_tail_and_preserves_input_on_failure() {
        let fixture = claimed_fixture();
        let scratch = tempfile::tempdir().unwrap();
        let input = scratch.path().join("raw.surql");
        write_claimed(&input, &fixture.rows);
        let mut file = std::fs::OpenOptions::new()
            .append(true)
            .open(&input)
            .unwrap();
        writeln!(
            file,
            "DEFINE USER stranger ON ROOT PASSWORD 'x' ROLES OWNER;"
        )
        .unwrap();
        drop(file);
        let before = std::fs::read(&input).unwrap();
        assert!(Dump::decode(&input, &budget()).is_err());
        assert_eq!(std::fs::read(&input).unwrap(), before);
    }
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
