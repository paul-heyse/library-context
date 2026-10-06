//! Explicit completed-input dispatch for pure kernels. There are no persisted grants or epochs.
use crate::workspace::CompletedInputs;
use datafusion::prelude::SessionContext;
use futures::TryStreamExt;
use lctx_model::domain::{
    analysis::sources::CompletedInput, charged, resources::ResourceBudget, *,
};
use std::{any::TypeId, collections::BTreeSet};
pub struct ConsumedInputs {
    declarations: Vec<ValidationInput>,
    dispatched: BTreeSet<usize>,
    _charge: charged::StateCharge,
}
impl ConsumedInputs {
    pub fn new(
        mut declarations: Vec<ValidationInput>,
        budget: &ResourceBudget,
    ) -> Result<Self, ModelError> {
        declarations.sort_by_key(|input| (input.name(), input.prefix(), input.order().to_vec()));
        declarations.dedup_by(|a, b| {
            a.name() == b.name() && a.prefix() == b.prefix() && a.order() == b.order()
        });
        let mut charge = charged::StateCharge::new(budget, "completed-consumer-inputs");
        charge.grow(
            declarations
                .capacity()
                .saturating_mul(size_of::<ValidationInput>()),
        )?;
        Ok(Self {
            declarations,
            dispatched: Default::default(),
            _charge: charge,
        })
    }
    pub fn next<R: Record>(
        &mut self,
        inputs: &CompletedInputs,
    ) -> Result<Option<(ValidationInput, CompletedInput<R>)>, ModelError> {
        let Some((index, declaration)) =
            self.declarations
                .iter()
                .enumerate()
                .find(|(index, declaration)| {
                    declaration.type_id() == TypeId::of::<R>() && !self.dispatched.contains(index)
                })
        else {
            return Ok(None);
        };
        let source = inputs.read_at::<R>(declaration.prefix())?;
        self.dispatched.insert(index);
        Ok(Some((declaration.clone(), source)))
    }
    pub fn finish(self, producer: &str) -> Result<(), ModelError> {
        if let Some((_, input)) = self
            .declarations
            .iter()
            .enumerate()
            .find(|(index, _)| !self.dispatched.contains(index))
        {
            return Err(ModelError::Invalid(format!(
                "producer {producer} has no typed loader for {}",
                input.name()
            )));
        }
        Ok(())
    }
}
pub async fn stream_at<R: Record>(
    input: &CompletedInput<R>,
    declaration: &ValidationInput,
    inputs: &CompletedInputs,
    session: &SessionContext,
    consume: impl FnMut(&CompletedInput<R>, &arrow_array::RecordBatch) -> Result<(), ModelError>,
) -> Result<(), ModelError> {
    stream_where_at(input, declaration, inputs, session, None, consume).await
}
/// Read the exact immutable input through an owner-selected predicate before rich decoding.
/// The predicate is compiler SQL, never a provider string or a source of semantic authority.
pub async fn stream_where_at<R: Record>(
    input: &CompletedInput<R>,
    declaration: &ValidationInput,
    inputs: &CompletedInputs,
    session: &SessionContext,
    predicate: Option<&str>,
    consume: impl FnMut(&CompletedInput<R>, &arrow_array::RecordBatch) -> Result<(), ModelError>,
) -> Result<(), ModelError> {
    let table = inputs.table_at::<R>(declaration.prefix())?;
    let selected = format!("SELECT * FROM {}{}", identifier(&table), predicate
        .map(|predicate| format!(" WHERE ({predicate})")).unwrap_or_default());
    stream_query_at(input, declaration, session, &selected, consume).await
}
/// Stream an owner-declared SELECT with the record's exact schema and ordering contract.
/// Joins and closure predicates are supplied by the owning semantic kernel.
pub async fn stream_query_at<R: Record>(
    input: &CompletedInput<R>,
    declaration: &ValidationInput,
    session: &SessionContext,
    selected: &str,
    mut consume: impl FnMut(&CompletedInput<R>, &arrow_array::RecordBatch) -> Result<(), ModelError>,
) -> Result<(), ModelError> {
    if declaration.type_id() != TypeId::of::<R>() {
        return Err(ModelError::Invalid("scoped stream record declaration mismatch".into()));
    }
    let order = declaration.order().iter().map(|column| identifier(column))
        .collect::<Vec<_>>().join(",");
    let sql = format!("SELECT * FROM ({selected}) AS scoped_input{}",
        if order.is_empty() { String::new() } else { format!(" ORDER BY {order}") });
    let mut batches = crate::sql::query(session, &sql)
        .await
        .map_err(ModelError::codec)?
        .execute_stream()
        .await
        .map_err(ModelError::codec)?;
    while let Some(batch) = batches.try_next().await.map_err(ModelError::codec)? {
        consume(input, &batch)?;
        tokio::task::yield_now().await;
    }
    Ok(())
}
pub(crate) fn identifier(name: &str) -> String {
    format!("\"{}\"", name.replace('"', "\"\""))
}
#[cfg(test)]
pub(crate) fn assert_decoder_reachability(
    declarations: Vec<ValidationInput>,
    decoders: &BTreeSet<TypeId>,
) {
    let missing: Vec<_> = declarations
        .iter()
        .filter(|input| !decoders.contains(&input.type_id()))
        .map(ValidationInput::name)
        .collect();
    assert!(
        missing.is_empty(),
        "completed kernel input lacks typed decoder: {missing:?}"
    );
}
/// An immutable table binding chosen by the consumer, including its exact vocabulary epoch.
/// The alias is supplied by the admitted completed-input or detached validation session.
#[derive(Clone)]
pub struct ClosureTable {
    pub relation: Relation,
    pub alias: String,
}
/// A semantic owner explicitly selects forward nominal dependencies and reverse memberships.
/// This plan describes row selection, not admission; the selected owner predicates still run.
pub struct NominalClosure {
    tables: Vec<ClosureTable>,
    pairs: Vec<(usize, usize, String)>,
}
impl NominalClosure {
    pub fn new(tables: Vec<ClosureTable>) -> Result<Self, ModelError> {
        if tables.is_empty() || tables.iter().any(|table| table.alias.is_empty()) {
            return Err(ModelError::Invalid("closure requires immutable table bindings".into()));
        }
        Ok(Self { tables, pairs: Vec::new() })
    }
    /// Follow a declared nominal field. Target selection is explicit, so two vocabulary epochs
    /// of the same relation cannot silently collapse into a latest-alias dependency.
    pub fn follow(&mut self, source: usize, field: &str, target: usize) -> Result<(), ModelError> {
        let from = self.table(source)?;
        let to = self.table(target)?;
        let descriptor = from.relation.fields().iter().find(|candidate| candidate.name() == field)
            .ok_or_else(|| ModelError::Invalid("closure field absent".into()))?;
        if descriptor.target().map(|(kind, _)| kind) != Some(to.relation.type_id()) || descriptor.list() {
            return Err(ModelError::Invalid("closure field has another nominal target or list shape".into()));
        }
        let sql = format!("SELECT id AS source_id, {} AS target_id FROM {} WHERE {} IS NOT NULL",
            identifier(field), identifier(&from.alias), identifier(field));
        self.pairs.push((source, target, sql));
        Ok(())
    }
    /// Include rows owned through this nominal field, as well as their forward owner reference.
    /// Callers declare this only for actual membership, never for every incoming reference.
    pub fn own(&mut self, member: usize, owner_field: &str, owner: usize) -> Result<(), ModelError> {
        self.follow(member, owner_field, owner)?;
        let from = self.table(member)?;
        let sql = format!("SELECT {} AS source_id, id AS target_id FROM {} WHERE {} IS NOT NULL",
            identifier(owner_field), identifier(&from.alias), identifier(owner_field));
        self.pairs.push((owner, member, sql));
        Ok(())
    }
    /// Explicit semantic edges whose key includes a non-nominal domain, such as RunFamily.
    /// SELECT must return source_id and target_id (both nominal 16-byte IDs). The owner supplies
    /// the complete family/context predicates; this utility does not infer applicability.
    pub fn pairs(&mut self, source: usize, target: usize, select: String) -> Result<(), ModelError> {
        self.table(source)?;
        self.table(target)?;
        if select.trim().is_empty() {
            return Err(ModelError::Invalid("closure pair SELECT absent".into()));
        }
        self.pairs.push((source, target, select));
        Ok(())
    }
    fn table(&self, index: usize) -> Result<&ClosureTable, ModelError> {
        self.tables.get(index).ok_or_else(|| ModelError::Invalid("closure table index absent".into()))
    }
    fn edges_sql(&self) -> String {
        if self.pairs.is_empty() {
            return "SELECT CAST(0 AS BIGINT) AS source_kind, arrow_cast(NULL,'FixedSizeBinary(16)') AS source_id, CAST(0 AS BIGINT) AS target_kind, arrow_cast(NULL,'FixedSizeBinary(16)') AS target_id WHERE false".into();
        }
        self.pairs.iter().map(|(source, target, sql)| format!(
            "SELECT CAST({source} AS BIGINT) AS source_kind, source_id, CAST({target} AS BIGINT) AS target_kind, target_id FROM ({sql}) AS pair_rows WHERE source_id IS NOT NULL AND target_id IS NOT NULL"
        )).collect::<Vec<_>>().join(" UNION ALL ")
    }
    /// Project nominal edges once for an immutable dependency set into an external ordered IPC
    /// stream. All subsequent grains reuse these compact edges without rescanning rich inputs.
    pub async fn prepare(
        &self,
        session: &SessionContext,
        budget: &ResourceBudget,
    ) -> Result<PreparedEdges, ModelError> {
        use datafusion::{arrow::ipc::writer::FileWriter, execution::{options::ArrowReadOptions, session_state::SessionStateBuilder}};
        use std::fs::File;
        let state = session.state();
        let config = state.config().clone()
            .with_target_partitions(state.config().target_partitions().max(2))
            .with_repartition_joins(true)
            .set_bool("datafusion.optimizer.prefer_hash_join", false);
        let session = SessionContext::new_with_state(SessionStateBuilder::new_from_existing(state)
            .with_config(config).build());
        let directory = tempfile::tempdir().map_err(ModelError::codec)?;
        let path = directory.path().join("nominal-edges.arrow");
        let sql = format!("SELECT * FROM ({}) AS projected_edges ORDER BY source_kind,source_id,target_kind,target_id", self.edges_sql());
        let frame = crate::sql::query(&session, &sql).await.map_err(ModelError::codec)?;
        let schema = frame.schema().as_arrow().clone();
        let mut stream = frame.execute_stream().await.map_err(ModelError::codec)?;
        let mut writer = FileWriter::try_new(File::create(&path).map_err(ModelError::codec)?, &schema).map_err(ModelError::codec)?;
        while let Some(batch) = stream.try_next().await.map_err(ModelError::codec)? {
            let _transfer = budget.reserve("semantic-edge-transfer", batch.get_array_memory_size())?;
            writer.write(&batch).map_err(ModelError::codec)?;
            tokio::task::yield_now().await;
        }
        writer.finish().map_err(ModelError::codec)?;
        drop(writer);
        let alias = scope_alias("edges");
        session.register_arrow(alias.clone(), path.to_string_lossy(), ArrowReadOptions::default().schema(&schema))
            .await.map_err(ModelError::codec)?;
        Ok(PreparedEdges(std::sync::Arc::new(EdgeSource { session, alias, tables: self.tables.clone(), _directory: directory })))
    }
}
fn scope_alias(kind: &str) -> String {
    use std::sync::atomic::{AtomicU64, Ordering};
    static NEXT_SCOPE: AtomicU64 = AtomicU64::new(0);
    format!("__semantic_{kind}_{}", NEXT_SCOPE.fetch_add(1, Ordering::Relaxed))
}
struct EdgeSource {
    session: SessionContext,
    alias: String,
    tables: Vec<ClosureTable>,
    _directory: tempfile::TempDir,
}
impl Drop for EdgeSource {
    fn drop(&mut self) { let _ = self.session.deregister_table(self.alias.as_str()); }
}
/// Dependency-set preparation, reused by source/context/family grains. It selects rows only;
/// all ordinary owner predicates and the independent global reference pass still apply.
#[derive(Clone)]
pub struct PreparedEdges(std::sync::Arc<EdgeSource>);
impl PreparedEdges {
    /// Compute compact keys for one explicitly selected grain. The root always remains in the
    /// closure, including an assertion with no supporting rows. UNION DISTINCT terminates cycles.
    pub async fn grain(&self, root: usize, predicate: &str, budget: &ResourceBudget) -> Result<PreparedClosure, ModelError> {
        use datafusion::datasource::MemTable;
        use std::sync::Arc;
        let table = self.0.tables.get(root).ok_or_else(|| ModelError::Invalid("closure table index absent".into()))?;
        if predicate.trim().is_empty() { return Err(ModelError::Invalid("closure root predicate absent".into())); }
        let sql = format!("WITH RECURSIVE closure_keys AS (SELECT CAST({root} AS BIGINT) AS kind,id FROM {} WHERE ({predicate}) UNION SELECT e.target_kind AS kind,e.target_id AS id FROM closure_keys k JOIN {} e ON k.kind=e.source_kind AND k.id=e.source_id) SELECT kind,id FROM closure_keys ORDER BY kind,id", identifier(&table.alias), identifier(&self.0.alias));
        let frame = crate::sql::query(&self.0.session, &sql).await.map_err(ModelError::codec)?;
        let schema = frame.schema().as_arrow().clone();
        let mut stream = frame.execute_stream().await.map_err(ModelError::codec)?;
        let mut keys = Vec::new();
        let mut charge = charged::StateCharge::new(budget, "semantic-grain-keys");
        while let Some(batch) = stream.try_next().await.map_err(ModelError::codec)? {
            charge.grow(batch.get_array_memory_size().saturating_add(size_of::<arrow_array::RecordBatch>()))?;
            keys.push(batch);
            tokio::task::yield_now().await;
        }
        let alias = scope_alias("keys");
        let provider = MemTable::try_new(Arc::new(schema), vec![keys]).map_err(ModelError::codec)?;
        self.0.session.register_table(alias.clone(), Arc::new(provider)).map_err(ModelError::codec)?;
        Ok(PreparedClosure { edges: self.0.clone(), alias, _charge: charge })
    }
}
/// A short-lived compact nominal key set for one owner grain. It keeps the external edge source
/// alive; dropping the scope removes its temporary key table without rewriting semantic inputs.
pub struct PreparedClosure {
    edges: std::sync::Arc<EdgeSource>,
    alias: String,
    _charge: charged::StateCharge,
}
impl PreparedClosure {
    pub fn session(&self) -> &SessionContext { &self.edges.session }
    pub fn select(&self, table: usize) -> Result<String, ModelError> {
        let table_binding = self.edges.tables.get(table).ok_or_else(|| ModelError::Invalid("closure table index absent".into()))?;
        Ok(format!("SELECT source.* FROM {} AS source JOIN {} AS keys ON keys.kind={table} AND source.id=keys.id",
            identifier(&table_binding.alias), identifier(&self.alias)))
    }
}
impl Drop for PreparedClosure {
    fn drop(&mut self) { let _ = self.edges.session.deregister_table(self.alias.as_str()); }
}

#[cfg(test)]
mod nominal_closure_controls {
    use super::*;
    use datafusion::datasource::MemTable;
    use input::{Package, Release};
    use std::sync::Arc;

    fn register<R: Record>(session: &SessionContext, name: &str, rows: &[R]) {
        let batch = R::encode(rows).unwrap();
        session.register_table(name, Arc::new(MemTable::try_new(batch.schema(), vec![vec![batch]]).unwrap())).unwrap();
    }
    fn id_predicate<R: Record>(id: Id<R>) -> String {
        let bytes = id.bytes().iter().map(|byte| format!("{byte:02x}")).collect::<String>();
        format!("id=X'{bytes}'")
    }
    async fn decode<R: Record>(scope: &PreparedClosure, table: usize) -> Vec<R> {
        let mut stream = crate::sql::query(scope.session(), &scope.select(table).unwrap()).await.unwrap()
            .execute_stream().await.unwrap();
        let mut rows = Vec::new();
        while let Some(batch) = stream.try_next().await.unwrap() { rows.extend(R::decode(&batch).unwrap()); }
        rows
    }
    fn fixture() -> (SessionContext, Vec<ClosureTable>, Vec<Release>, Package) {
        let session = SessionContext::new();
        let selected = Package { name: "selected".into() };
        let unrelated = Package { name: "unrelated".into() };
        let releases = vec![
            Release { package: selected.id(), version: "1".into() },
            Release { package: selected.id(), version: "2".into() },
            Release { package: unrelated.id(), version: "3".into() },
        ];
        register(&session, "packages_at_attempt", &[selected.clone(), unrelated]);
        register(&session, "releases_at_attempt", &releases);
        let tables = vec![
            ClosureTable { relation: Relation::of::<Release>(), alias: "releases_at_attempt".into() },
            ClosureTable { relation: Relation::of::<Package>(), alias: "packages_at_attempt".into() },
        ];
        (session, tables, releases, selected)
    }
    #[tokio::test]
    async fn forward_closure_keeps_root_and_excludes_shared_target_neighbors() {
        let (session, tables, releases, package) = fixture();
        let mut plan = NominalClosure::new(tables).unwrap();
        plan.follow(0, "package", 1).unwrap();
        let budget = ResourceBudget::fixed(1 << 20).unwrap();
        let edges = plan.prepare(&session, &budget).await.unwrap();
        let scope = edges.grain(0, &id_predicate(releases[0].id()), &budget).await.unwrap();
        assert_eq!(decode::<Release>(&scope, 0).await, vec![releases[0].clone()]);
        assert_eq!(decode::<Package>(&scope, 1).await, vec![package]);
        drop(scope);
        assert_eq!(budget.reserved(), 0);
    }
    #[tokio::test]
    async fn explicit_reverse_membership_closes_cycle_without_unrelated_owners() {
        let (session, tables, releases, package) = fixture();
        let mut plan = NominalClosure::new(tables).unwrap();
        plan.own(0, "package", 1).unwrap();
        let budget = ResourceBudget::fixed(1 << 20).unwrap();
        let edges = plan.prepare(&session, &budget).await.unwrap();
        let scope = edges.grain(1, &id_predicate(package.id()), &budget).await.unwrap();
        let found = decode::<Release>(&scope, 0).await;
        assert_eq!(found.len(), 2);
        assert!(found.contains(&releases[0]) && found.contains(&releases[1]));
        assert_eq!(decode::<Package>(&scope, 1).await, vec![package]);
    }
    #[tokio::test]
    async fn root_without_support_is_selected_and_wrong_nominal_edge_refuses() {
        let (session, tables, releases, _) = fixture();
        let mut plan = NominalClosure::new(tables).unwrap();
        assert!(plan.follow(0, "package", 0).is_err());
        let budget = ResourceBudget::fixed(1 << 20).unwrap();
        let edges = plan.prepare(&session, &budget).await.unwrap();
        let scope = edges.grain(0, &id_predicate(releases[0].id()), &budget).await.unwrap();
        assert_eq!(decode::<Release>(&scope, 0).await, vec![releases[0].clone()]);
        assert!(decode::<Package>(&scope, 1).await.is_empty());
    }
}
