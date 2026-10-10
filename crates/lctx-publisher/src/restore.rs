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
    surrealdb::types::{Bytes, RecordId, ToSql, Value},
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
const SPOOL_BUFFER_BYTES: usize = 64 * 1024;

/// Flush explicitly, then disarm BufWriter's best-effort Drop flush even after failure.
fn finish_spool<W: Write>(mut writer: std::io::BufWriter<W>) -> Result<(), ModelError> {
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
            SPOOL_BUFFER_BYTES * (DATA_TABLES.len() + 1),
        )?;
        let directory = tempfile::tempdir().map_err(ModelError::codec)?;
        let files = (|| {
            let mut files = BTreeMap::new();
            for table in DATA_TABLES {
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
                                let Some(Value::RecordId(source)) = object(&row)?.get("source")
                                else {
                                    return Err(ModelError::Schema(
                                        "restore original chunk source",
                                    ));
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

fn identity_charge(id: &RecordId) -> Result<usize, ModelError> {
    // Native addresses use string keys. Preserve grammar-owned non-string claimed keys
    // for subsequent admission, accounting their nested representation conservatively.
    let key = match &id.key {
        lctx_surrealdb::surrealdb::types::RecordIdKey::String(key) => key.len(),
        _ => serde_json::to_vec(id)
            .map_err(ModelError::codec)?
            .len()
            .saturating_mul(64),
    };
    Ok(key
        .saturating_add(id.table.as_str().len())
        .saturating_add(256))
}

/// Charged claimed graph selection within one fully decoded dump. This is preparation,
/// never an admission capability: restore still validates typed/cold state independently.
struct PreparedClosure {
    handle: SnapshotHandle,
    manifest: Manifest,
    bindings: Vec<CompletedBinding>,
    originals: BTreeMap<RecordId, lctx_model::domain::graph::Original>,
    binding_rows: BTreeSet<RecordId>,
    owners: BTreeSet<RecordId>,
    owner_specs: BTreeMap<RecordId, lctx_model::domain::completed::ContributionSpec>,
    views: BTreeSet<String>,
    nodes: BTreeSet<RecordId>,
    _charge: Box<dyn Reservation>,
}
impl PreparedClosure {
    fn prepare(
        dump: &Dump,
        selected: Option<ContentHash>,
        budget: &ResourceBudget,
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
            selection_charge.try_resize(selection_charge.size().saturating_add(
                serde_json::to_vec(&view).map_err(ModelError::codec)?.len() + 256,
            ))?;
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
                if !owners.contains(&owner) {
                    selection_charge.try_resize(
                        selection_charge
                            .size()
                            .saturating_add(identity_charge(&owner)?.saturating_mul(2))
                            .saturating_add(
                                serde_json::to_vec(&descriptor.spec)
                                    .map_err(ModelError::codec)?
                                    .len(),
                            ),
                    )?;
                    owners.insert(owner.clone());
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
            if row.get("contribution").is_some_and(
                |owner| matches!(owner,Value::RecordId(owner) if owners.contains(owner)),
            ) {
                if let Some(Value::RecordId(node)) = row.get("node") {
                    if !nodes.contains(node) {
                        selection_charge.try_resize(
                            selection_charge
                                .size()
                                .saturating_add(identity_charge(node)?.saturating_mul(3)),
                        )?;
                        nodes.insert(node.clone());
                    }
                }
            }
        }
        let mut aliases: BTreeMap<RecordId, BTreeSet<RecordId>> = BTreeMap::new();
        for row in dump.rows("compiler_alias")? {
            let row = row?;
            let row = object(&row)?;
            let (Some(Value::RecordId(source)), Some(Value::RecordId(target))) =
                (row.get("source"), row.get("target"))
            else {
                return Err(ModelError::Schema("restore alias endpoints"));
            };
            selection_charge.try_resize(selection_charge.size().saturating_add(
                identity_charge(source)?.saturating_add(identity_charge(target)?),
            ))?;
            aliases
                .entry(source.clone())
                .or_default()
                .insert(target.clone());
        }
        let mut pending = nodes.iter().cloned().collect::<Vec<_>>();
        while let Some(source) = pending.pop() {
            if let Some(targets) = aliases.get(&source) {
                for target in targets {
                    if !nodes.contains(target) {
                        selection_charge.try_resize(
                            selection_charge
                                .size()
                                .saturating_add(identity_charge(target)?.saturating_mul(3)),
                        )?;
                        nodes.insert(target.clone());
                        pending.push(target.clone());
                    }
                }
            }
        }
        drop(aliases);
        // A shared store must never supply rows omitted from an untrusted backup. Establish
        // the complete selected payload inventory from this dump before creating a native attempt.
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
            binding_copies
                .entry(binding.key())
                .and_modify(|selected| {
                    if id < selected {
                        *selected = id.clone();
                    }
                })
                .or_insert_with(|| id.clone());
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
        selection_charge.try_resize(
            selection_charge
                .size()
                .saturating_add(manifest.originals.len().saturating_mul(512)),
        )?;
        let originals = manifest
            .originals
            .iter()
            .map(|original| {
                (
                    RecordId::new("original", original.source.0.hex()),
                    original.clone(),
                )
            })
            .collect();
        let prepared = Self {
            handle,
            manifest,
            bindings,
            originals,
            binding_rows,
            owners,
            owner_specs,
            views,
            nodes,
            _charge: selection_charge,
        };
        prepared.validate_inventory(dump, budget)?;
        Ok(prepared)
    }
    fn keep_row(&self, table: &str, row: &Value) -> Result<bool, ModelError> {
        let obj = object(row)?;
        let id = record_id(row)?;
        Ok(match table {
            "publication" => id == &RecordId::new("publication", self.handle.publication.hex()),
            "compiler_contribution" => self.owners.contains(id),
            "compiler_membership" => obj.get("contribution").is_some_and(
                |v| matches!(v, Value::RecordId(owner) if self.owners.contains(owner)),
            ),
            "compiler_view" => match &id.key {
                lctx_surrealdb::surrealdb::types::RecordIdKey::String(id) => {
                    self.views.contains(id)
                }
                _ => false,
            },
            "entity" | "assertion" | "compiler_record" => self.nodes.contains(id),
            "compiler_binding" => self.binding_rows.contains(id),
            "compiler_alias" => obj.get("source").is_some_and(
                |v| matches!(v,Value::RecordId(source) if self.nodes.contains(source)),
            ),
            "original" => self.originals.contains_key(id),
            "original_chunk" => obj.get("source").is_some_and(
                |v| matches!(v,Value::RecordId(source) if self.originals.contains_key(source)),
            ),
            _ => false,
        })
    }
    fn validate_inventory(&self, dump: &Dump, budget: &ResourceBudget) -> Result<(), ModelError> {
        let mut charge = budget.reserve("restore-selected-physical-inventory", 0)?;
        let mut ids = BTreeSet::new();
        let mut binding_keys = BTreeSet::new();
        for table in DATA_TABLES.iter().filter(|t| **t != "original_chunk") {
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
                    if *table == "original" {
                        let obj = object(&row)?;
                        let original = &self.originals[record_id(&row)?];
                        if obj.get("content") != Some(&Value::String(original.content.hex()))
                            || obj.get("byte_len")
                                != Some(&Value::Number(
                                    lctx_surrealdb::surrealdb::types::Number::Int(
                                        i64::try_from(original.byte_len)
                                            .map_err(ModelError::codec)?,
                                    ),
                                ))
                        {
                            return Err(ModelError::Conflict("restore original header"));
                        }
                    }
                    if *table == "compiler_binding" {
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
        for (source, original) in &self.originals {
            if !ids.contains(source) {
                return Err(ModelError::Conflict(
                    "restore original header absent from dump",
                ));
            }
            // Positional writes validate arbitrarily ordered dump chunks without retaining bytes.
            let mut file = tempfile::tempfile().map_err(ModelError::codec)?;
            let mut seen = BTreeSet::new();
            let mut total = 0u64;
            for row in dump.original_rows(source)?.into_iter().flatten() {
                let row = row?;
                let obj = object(&row)?;
                let Some(Value::Number(lctx_surrealdb::surrealdb::types::Number::Int(start))) =
                    obj.get("start")
                else {
                    return Err(ModelError::Schema("restore original offset"));
                };
                let start = u64::try_from(*start).map_err(ModelError::codec)?;
                let chunk = bytes(obj, "bytes")?;
                if start >= original.byte_len
                    || start % 65536 != 0
                    || chunk.len() as u64 != (original.byte_len - start).min(65536)
                    || !seen.insert(start)
                    || record_id(&row)?
                        != &RecordId::new(
                            "original_chunk",
                            format!("{}_{}", original.source.0.hex(), start),
                        )
                    || obj.get("source") != Some(&Value::RecordId(source.clone()))
                    || obj.get("content") != Some(&Value::String(ContentHash::of(chunk).hex()))
                {
                    return Err(ModelError::Conflict("restore original chunk continuity"));
                }
                charge.try_resize(charge.size().saturating_add(256))?;
                file.seek(std::io::SeekFrom::Start(start))
                    .map_err(ModelError::codec)?;
                file.write_all(chunk).map_err(ModelError::codec)?;
                total += chunk.len() as u64;
            }
            if total != original.byte_len {
                return Err(ModelError::Conflict("restore original length"));
            }
            file.rewind().map_err(ModelError::codec)?;
            let mut hasher = lctx_model::domain::ContentHasher::default();
            let mut window = [0u8; 65536];
            loop {
                let n = file.read(&mut window).map_err(ModelError::codec)?;
                if n == 0 {
                    break;
                }
                hasher.update(&window[..n]);
            }
            if hasher.finish() != original.content {
                return Err(ModelError::Conflict("restore original content"));
            }
        }
        Ok(())
    }
}

/// Narrow one terminal, checked engine export to its claimed physical dependency closure.
/// Original physical IDs and canonical bytes remain unchanged; no runtime grant is imported.
pub(crate) fn compact_export(
    input: &Path,
    output: &Path,
    handle: &SnapshotHandle,
    budget: &ResourceBudget,
) -> Result<(), ModelError> {
    let dump = Dump::decode(input, budget)?;
    let result = (|| {
        let prepared = PreparedClosure::prepare(&dump, Some(handle.publication), budget)?;
        if &prepared.handle != handle {
            return Err(ModelError::Conflict("backup exact publication handle"));
        }
        let mut writer = std::io::BufWriter::new(
            std::fs::OpenOptions::new()
                .write(true)
                .truncate(true)
                .open(output)
                .map_err(ModelError::codec)?,
        );
        writeln!(writer, "OPTION IMPORT;").map_err(ModelError::codec)?;
        for definition in &dump.definitions {
            writeln!(writer, "{definition};").map_err(ModelError::codec)?;
        }
        let mut encoded_charge = budget.reserve("backup-compact-encoded-row", 0)?;
        let mut emit = |row: Value| -> Result<(), ModelError> {
            encoded_charge.try_resize(crate::backup_import::MAX_DUMP_RECORD_BYTES)?;
            let encoded = Value::Array(vec![row].into()).to_sql();
            encoded_charge.try_resize(encoded.len())?;
            if encoded.len().saturating_add(8) > crate::backup_import::MAX_DUMP_RECORD_BYTES {
                return Err(ModelError::Schema("compact dump statement bound"));
            }
            writeln!(writer, "INSERT {encoded};").map_err(ModelError::codec)?;
            encoded_charge.try_resize(0)?;
            Ok(())
        };
        for table in DATA_TABLES.iter().filter(|t| **t != "original_chunk") {
            for row in dump.rows(table)? {
                let row = row?;
                if prepared.keep_row(table, &row)? {
                    emit(row)?;
                }
            }
        }
        for original in &prepared.manifest.originals {
            for row in dump
                .original_rows(&RecordId::new("original", original.source.0.hex()))?
                .into_iter()
                .flatten()
            {
                emit(row?)?;
            }
        }
        writer.flush().map_err(ModelError::codec)
    })();
    let mut completion = lctx_model::domain::completion::Completion::default();
    completion.cleanup(
        dump._directory.path().display().to_string(),
        dump._directory.close().map_err(ModelError::codec),
    );
    lctx_model::domain::completion::complete(result, completion)
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
    let result = async {
        let prepared = PreparedClosure::prepare(&dump, selected, &budget)?;
        dump.compare_definitions(config).await?;
        let native = NativeCompilerStore::begin(config, prepared.manifest.frontier).await?;
        let result = async {
            let admitted = reconstruct(&dump, &native, &prepared, &budget, options).await?;
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

/// Assemble selected portable state synchronously; independent native admission follows.
fn write_completed_state<W: Write>(
    dump: &Dump,
    prepared: &PreparedClosure,
    budget: &ResourceBudget,
    mut writer: W,
) -> Result<(), ModelError> {
    serde_json::to_writer(&mut writer, &CompletedStateHeader::current())
        .map_err(ModelError::codec)?;
    writer.write_all(b"\n").map_err(ModelError::codec)?;
    let mut binding_keys = BTreeSet::new();
    for table in STATE_TABLES {
        let mut sorted = lctx_surrealdb::ordered_rows::SortedRows::with_budget_and_row_bytes(
            budget,
            lctx_model::domain::resources::MAX_ROW_BYTES,
        )?;
        for row in dump.rows(table)? {
            let row = row?;
            let obj = object(&row)?;
            let keep = prepared.keep_row(table, &row)?
                && (*table != "compiler_binding" || {
                    let descriptor: CompletedBinding =
                        serde_json::from_slice(bytes(obj, "descriptor")?)
                            .map_err(ModelError::codec)?;
                    binding_keys.insert(descriptor.key())
                });
            if keep {
                sorted.push(lctx_surrealdb::compiler::normalize_transport_row(
                    table,
                    &row,
                    &prepared.owner_specs,
                )?)?;
            }
        }
        let mut sorted = sorted.finish()?;
        while let Some(row) = sorted.next_row()? {
            serde_json::to_writer(&mut writer, &serde_json::json!({"table":table,"row":row}))
                .map_err(ModelError::codec)?;
            writer.write_all(b"\n").map_err(ModelError::codec)?;
        }
    }
    Ok(())
}

async fn reconstruct(
    dump: &Dump,
    native: &Arc<NativeCompilerStore>,
    prepared: &PreparedClosure,
    budget: &ResourceBudget,
    options: cpg_core::workspace::WorkspaceOptions,
) -> Result<cpg_core::artifact::RestoredAdmission, ModelError> {
    let runtime = cpg_core::workspace::Workspace::with_budget(
        Arc::new(lctx_model::domain::model()?),
        options,
        budget.clone(),
        native.clone(),
    )?;
    let manifest = &prepared.manifest;
    let nodes = &prepared.nodes;
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
    let _state_buffer = budget.reserve("restore-completed-state-buffer", SPOOL_BUFFER_BYTES)?;
    let mut state = tempfile::NamedTempFile::new().map_err(ModelError::codec)?;
    let mut writer = std::io::BufWriter::with_capacity(SPOOL_BUFFER_BYTES, &mut state);
    let assembled = write_completed_state(dump, prepared, budget, &mut writer);
    let mut completion = lctx_model::domain::completion::Completion::default();
    completion.step("restore completed-state spool flush", finish_spool(writer));
    let assembled = lctx_model::domain::completion::complete(assembled, completion);
    let result = async {
        assembled?;
        native
            .import_state(state.path(), &manifest.completed_state)
            .await?;
        runtime.restore(manifest.profile).await?;
        cpg_core::artifact::verify_restored(&runtime, manifest).await
    }
    .await;
    let mut completion = lctx_model::domain::completion::Completion::default();
    completion.cleanup(
        state.path().display().to_string(),
        state.close().map_err(ModelError::codec),
    );
    lctx_model::domain::completion::complete(result, completion)
}

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
    use lctx_surrealdb::surrealdb::types::Object;

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
            CompletedContribution {
                spec: ContributionSpec {
                    captured_binding: None,
                    producer: producer.into(),
                    profile: Profile::Catalog,
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
        rows.push(Value::Object(descriptor_row(
            "compiler_binding",
            "z-equivalent-binding",
            &binding,
        )));
        let nodes = BTreeSet::from([
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
        let source = ContentHash::of(b"claimed-original");
        let original = b"original; bytes remain exact";
        let mut header = row("original", &source.hex());
        header.insert("content", ContentHash::of(original).hex());
        header.insert("byte_len", original.len() as i64);
        rows.push(Value::Object(header));
        let mut chunk = row("original_chunk", &format!("{}_0", source.hex()));
        chunk.insert("source", RecordId::new("original", source.hex()));
        chunk.insert("start", 0i64);
        chunk.insert("bytes", Bytes::from(original.to_vec()));
        chunk.insert("content", ContentHash::of(original).hex());
        rows.push(Value::Object(chunk));
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
    fn write_claimed(path: &Path, rows: &[Value]) {
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
        ResourceBudget::fixed(cpg_core::workspace::WorkspaceOptions::default().memory_bytes)
            .unwrap()
    }
    #[test]
    fn selected_state_assembly_preserves_maximum_declared_projection_chunk() {
        use lctx_model::domain::{
            EvidenceBytes, Id,
            projection::{ProjectionSnapshotChunk, ProjectionSourceAssessment, snapshot},
            resources::{MAX_ROW_BYTES, TRANSFER_BYTES},
        };
        let assessment: Id<ProjectionSourceAssessment> =
            serde_json::from_value(serde_json::to_value([7u8; 16]).unwrap()).unwrap();
        let header = snapshot::header(assessment, snapshot::CHUNK_BYTES).unwrap();
        let chunk = ProjectionSnapshotChunk {
            snapshot: header.id(),
            ordinal: 0,
            payload: EvidenceBytes(vec![255; snapshot::CHUNK_BYTES]),
        };
        let relation = Relation::of::<ProjectionSnapshotChunk>();
        let batch = ProjectionSnapshotChunk::encode(&[chunk]).unwrap();
        let body = lctx_surrealdb::codec::batch_bodies(&relation, &batch)
            .unwrap()
            .pop()
            .unwrap();
        let mut fixture = claimed_fixture();
        // Compaction-only claimed rows omit fields consumed by portable normalization.
        for (index, row) in fixture.rows.iter_mut().enumerate() {
            if record_id(row).unwrap().table.as_str() == "compiler_membership" {
                let Value::Object(object) = row else { unreachable!() };
                object.insert("relation", Package::NAME);
                object.insert("semantic_key", format!("{index:032x}"));
            }
        }
        let backing_id = RecordId::new("compiler_record", "extra");
        let backing = fixture
            .rows
            .iter_mut()
            .find(|row| record_id(row).unwrap() == &backing_id)
            .unwrap();
        let Value::Object(object) = backing else {
            panic!("claimed physical backing");
        };
        object.insert("canonical", Bytes::from(serde_json::to_vec(&body).unwrap()));
        let expected = backing.clone();
        assert!(serde_json::to_vec(&expected).unwrap().len() > TRANSFER_BYTES);
        let scratch = tempfile::tempdir().unwrap();
        let input = scratch.path().join("selected.surql");
        write_claimed(&input, &fixture.rows);
        let budget = budget();
        {
            let dump = Dump::decode(&input, &budget).unwrap();
            let prepared =
                PreparedClosure::prepare(&dump, Some(fixture.handle.publication), &budget).unwrap();
            let reserved = budget.reserved();
            let mut encoded = Vec::new();
            write_completed_state(&dump, &prepared, &budget, &mut encoded).unwrap();
            assert_eq!(
                budget.reserved(),
                reserved,
                "ordering reservations released"
            );
            let mut observed = None;
            for line in encoded
                .split(|byte| *byte == b'\n')
                .skip(1)
                .filter(|line| !line.is_empty())
            {
                assert!(
                    line.len() + 1 <= MAX_ROW_BYTES,
                    "whole import envelope remains bounded"
                );
                let row: serde_json::Value = serde_json::from_slice(line).unwrap();
                if row["table"] == "compiler_record" {
                    let value: Value = serde_json::from_value(row["row"].clone()).unwrap();
                    if record_id(&value).unwrap() == &backing_id {
                        assert!(line.len() > TRANSFER_BYTES);
                        assert!(
                            observed.replace(value).is_none(),
                            "exactly one selected backing"
                        );
                    }
                }
            }
            assert_eq!(observed, Some(expected));
        }
        assert_eq!(budget.reserved(), 0);
    }
    #[test]
    fn compact_claimed_closure_preserves_transitive_outputs_aliases_originals_and_physical_ids() {
        let fixture = claimed_fixture();
        let scratch = tempfile::tempdir().unwrap();
        let input = scratch.path().join("raw.surql");
        let output = scratch.path().join("compact.surql");
        write_claimed(&input, &fixture.rows);
        std::fs::File::create(&output).unwrap();
        compact_export(&input, &output, &fixture.handle, &budget()).unwrap();
        let mut dump = DataDump::new(std::fs::File::open(&output).unwrap());
        let mut selected = Vec::new();
        let mut definitions = Vec::new();
        while let Some(item) = dump.next().unwrap() {
            match item {
                Item::Rows(rows) => {
                    assert_eq!(rows.len(), 1);
                    selected.extend(rows);
                }
                Item::Definition(value) => definitions.push(value),
            }
        }
        assert_eq!(
            definitions,
            vec!["DEFINE FIELD canonical ON entity TYPE bytes PERMISSIONS FULL"]
        );
        for row in &selected {
            assert!(
                fixture.rows.contains(row),
                "compaction changed a physical row"
            );
        }
        let ids = selected
            .iter()
            .map(|row| record_id(row).unwrap().clone())
            .collect::<BTreeSet<_>>();
        assert!(fixture.nodes.is_subset(&ids));
        assert!(ids.contains(&RecordId::new("compiler_membership", "other-output-member")));
        assert!(!ids.contains(&RecordId::new("compiler_contribution", "z-equal-seed")));
        assert!(!ids.contains(&RecordId::new("compiler_contribution", "z-equal-root")));
        assert!(!ids.contains(&RecordId::new(
            "compiler_membership",
            "unrelated-owner-member"
        )));
        assert!(!ids.contains(&RecordId::new("entity", "unrelated")));
        assert_eq!(
            ids.iter()
                .filter(|id| id.table.as_str() == "publication")
                .count(),
            1
        );
        assert_eq!(
            ids.iter()
                .filter(|id| id.table.as_str() == "compiler_binding")
                .count(),
            1
        );
        assert!(ids.contains(&RecordId::new("compiler_binding", "selected-binding")));
        assert_eq!(
            ids.iter()
                .filter(|id| id.table.as_str() == "original_chunk")
                .count(),
            1
        );
        let compact = Dump::decode(&output, &budget()).unwrap();
        let prepared =
            PreparedClosure::prepare(&compact, Some(fixture.handle.publication), &budget())
                .unwrap();
        assert_eq!(prepared.handle, fixture.handle);
        assert_eq!(prepared.nodes, fixture.nodes);
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
            RecordId::new(
                "original_chunk",
                format!("{}_0", ContentHash::of(b"claimed-original").hex()),
            ),
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
                PreparedClosure::prepare(&dump, Some(fixture.handle.publication), &budget())
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
                PreparedClosure::prepare(&dump, Some(fixture.handle.publication), &budget())
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
                PreparedClosure::prepare(&dump, Some(fixture.handle.publication), &budget())
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
                PreparedClosure::prepare(&dump, Some(fixture.handle.publication), &budget())
                    .is_err(),
                "accepted original corruption {corruption}"
            );
        }
    }
    #[test]
    fn compact_keeps_unexpected_selected_members_aliases_and_all_definition_metadata() {
        let mut fixture = claimed_fixture();
        let mut member = row("compiler_membership", "unexpected-selected-member");
        member.insert(
            "contribution",
            RecordId::new("compiler_contribution", "a-root"),
        );
        member.insert("node", RecordId::new("entity", "unrelated"));
        fixture.rows.push(Value::Object(member));
        let mut alias = row("compiler_alias", "unexpected-selected-alias");
        alias.insert("source", RecordId::new("entity", "root"));
        alias.insert("target", RecordId::new("entity", "unrelated"));
        fixture.rows.push(Value::Object(alias));
        let scratch = tempfile::tempdir().unwrap();
        let input = scratch.path().join("raw.surql");
        let output = scratch.path().join("compact.surql");
        write_claimed(&input, &fixture.rows);
        std::fs::File::create(&output).unwrap();
        let mut file = std::fs::OpenOptions::new()
            .append(true)
            .open(&input)
            .unwrap();
        writeln!(
            file,
            "DEFINE FIELD unrelated ON unselected TYPE string ASSERT $value != '' PERMISSIONS NONE;"
        )
        .unwrap();
        drop(file);
        compact_export(&input, &output, &fixture.handle, &budget()).unwrap();
        let compact = Dump::decode(&output, &budget()).unwrap();
        assert_eq!(compact.definitions.len(), 2);
        assert!(
            compact
                .definitions
                .iter()
                .any(|definition| definition.contains("unrelated ON unselected"))
        );
        let ids = compact
            .rows("compiler_membership")
            .unwrap()
            .chain(compact.rows("compiler_alias").unwrap())
            .map(|row| record_id(&row.unwrap()).unwrap().clone())
            .collect::<BTreeSet<_>>();
        assert!(ids.contains(&RecordId::new(
            "compiler_membership",
            "unexpected-selected-member"
        )));
        assert!(ids.contains(&RecordId::new(
            "compiler_alias",
            "unexpected-selected-alias"
        )));
        // Preparation preserves questionable claimed state; it does not normalize it into
        // admission. The existing completed-state hash and cold admission still decide validity.
    }
    #[test]
    fn claimed_alias_metadata_reserves_large_physical_keys_before_retaining_them() {
        let mut fixture = claimed_fixture();
        let mut alias = row("compiler_alias", "large-unselected-alias");
        alias.insert("source", RecordId::new("entity", "s".repeat(512 * 1024)));
        alias.insert("target", RecordId::new("entity", "t".repeat(512 * 1024)));
        fixture.rows.push(Value::Object(alias));
        let scratch = tempfile::tempdir().unwrap();
        let input = scratch.path().join("raw.surql");
        write_claimed(&input, &fixture.rows);
        let budget = ResourceBudget::fixed(
            crate::backup_import::MAX_DUMP_RECORD_BYTES * 4
                + SPOOL_BUFFER_BYTES * (DATA_TABLES.len() + 1)
                + 96 * 1024,
        )
        .unwrap();
        let dump = Dump::decode(&input, &budget).unwrap();
        let error = PreparedClosure::prepare(&dump, Some(fixture.handle.publication), &budget)
            .err()
            .expect("large unselected adjacency keys still require storage admission");
        assert!(matches!(error, ModelError::Resource { .. }), "{error}");
    }
    #[tokio::test]
    async fn missing_claimed_payload_refuses_before_native_connection_or_attempt() {
        let fixture = claimed_fixture();
        let scratch = tempfile::tempdir().unwrap();
        let input = scratch.path().join("missing.surql");
        let rows = fixture
            .rows
            .iter()
            .filter(|row| record_id(row).unwrap() != &RecordId::new("entity", "root"))
            .cloned()
            .collect::<Vec<_>>();
        write_claimed(&input, &rows);
        let cfg = RuntimeConfig {
            reuse: None,
            serving_limits: None,
            endpoint: "not-a-connectable-native-endpoint".into(),
            username: "unused".into(),
            password: "unused".into(),
            viewer_username: "viewer".into(),
            viewer_password: "unused".into(),
            namespace: Name::new("library_context").unwrap(),
            database: Name::new("validation").unwrap(),
            service_generation: fixture.handle.service_generation,
            authentication: lctx_surrealdb::AuthenticationScope::Database,
            cache_database: Name::new("validation").unwrap(),
            selection: scratch.path().join("unselected.json"),
        };
        let error = restore(&cfg, &input, Some(fixture.handle.publication), "")
            .await
            .unwrap_err();
        assert!(
            error
                .to_string()
                .contains("restore selected payload absent from dump"),
            "{error}"
        );
        assert!(!cfg.selection.exists());
    }
    #[test]
    fn compact_claimed_closure_checks_unselected_executable_tail_and_preserves_input_on_failure() {
        let fixture = claimed_fixture();
        let scratch = tempfile::tempdir().unwrap();
        let input = scratch.path().join("raw.surql");
        let output = scratch.path().join("compact.surql");
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
        std::fs::File::create(&output).unwrap();
        assert!(compact_export(&input, &output, &fixture.handle, &budget()).is_err());
        assert_eq!(std::fs::read(&input).unwrap(), before);
        assert_eq!(std::fs::metadata(&output).unwrap().len(), 0);
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
