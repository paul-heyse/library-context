//! Compact selection for assertion support; the model's support predicate owns admission.
//! Immutable nominal edges are prepared once, then rich rows live only for one bounded grain.
use crate::{
    consumed_rows::{ClosureTable, PreparedClosure, PreparedRoot, PreparedRootKind, identifier},
    workspace::Cancellation,
};
use arrow_array::{Array, FixedSizeBinaryArray};
use datafusion::prelude::SessionContext;
use futures::TryStreamExt;
use lctx_model::domain::{
    admission_scope_program::{OwnershipRows, OwnershipSet, SupportOwnership},
    assertion::{AssertionQualification, SupportScope},
    resources::ResourceBudget,
    *,
};
use std::any::TypeId;

const ROOT_ROWS: usize = 128;
type Grain = (Option<[u8; 16]>, Option<[u8; 16]>);

fn typed<R: Record>(tables: &[ClosureTable]) -> Option<usize> {
    let mut choices = tables
        .iter()
        .enumerate()
        .filter(|(_, table)| table.relation.type_id() == TypeId::of::<R>());
    let (index, _) = choices.next()?;
    if choices.next().is_some() {
        return None;
    }
    Some(index)
}
pub(crate) fn declared(
    tables: &[ClosureTable],
    inputs: &[ValidationInput],
    input: &ValidationInput,
) -> Result<usize, ModelError> {
    inputs
        .iter()
        .position(|candidate| {
            candidate.type_id() == input.type_id() && candidate.prefix() == input.prefix()
        })
        .filter(|index| tables[*index].relation.type_id() == input.type_id())
        .ok_or(ModelError::Conflict("support input immutable binding"))
}
pub(crate) fn field_target(
    inputs: &[ValidationInput],
    source: usize,
    target: TypeId,
) -> Result<Option<usize>, ModelError> {
    let candidates: Vec<_> = inputs
        .iter()
        .enumerate()
        .filter(|(_, input)| input.type_id() == target)
        .map(|(index, _)| index)
        .collect();
    if candidates.is_empty() {
        return Ok(None);
    }
    if let [only] = candidates.as_slice() {
        return Ok(Some(*only));
    }
    let matching: Vec<_> = candidates
        .into_iter()
        .filter(|index| inputs[*index].prefix() == inputs[source].prefix())
        .collect();
    match matching.as_slice() {
        [] => Err(ModelError::Conflict("support dependency epoch absent")),
        [only] => Ok(Some(*only)),
        _ => Err(ModelError::Conflict("ambiguous support dependency epoch")),
    }
}
pub(crate) fn column(
    batch: &arrow_array::RecordBatch,
    name: &str,
    row: usize,
) -> Result<Option<[u8; 16]>, ModelError> {
    let values = batch
        .column_by_name(name)
        .and_then(|column| column.as_any().downcast_ref::<FixedSizeBinaryArray>())
        .ok_or(ModelError::Schema("support grain nominal projection"))?;
    if values.is_null(row) {
        return Ok(None);
    }
    Ok(Some(values.value(row).try_into().map_err(|_| {
        ModelError::Schema("support grain nominal width")
    })?))
}
fn ownership_rows(
    rows: &OwnershipRows,
    closure: &PreparedClosure,
    tables: &[ClosureTable],
) -> Result<String, ModelError> {
    match rows {
        OwnershipRows::Selected(input) => closure.select(*input),
        OwnershipRows::Full { input, filters } => {
            let table = tables
                .get(*input)
                .ok_or(ModelError::Schema("ownership physical input absent"))?;
            let filters = filters
                .iter()
                .map(|filter| {
                    Ok(format!(
                        "{} IN ({})",
                        identifier(filter.field),
                        ownership_set(&filter.values, closure, tables)?
                    ))
                })
                .collect::<Result<Vec<_>, ModelError>>()?
                .join(" AND ");
            Ok(format!(
                "SELECT * FROM {}{}",
                identifier(&table.alias),
                if filters.is_empty() {
                    String::new()
                } else {
                    format!(" WHERE {filters}")
                }
            ))
        }
    }
}
fn ownership_set(
    set: &OwnershipSet,
    closure: &PreparedClosure,
    tables: &[ClosureTable],
) -> Result<String, ModelError> {
    match set {
        OwnershipSet::Projection {
            rows,
            field,
            non_null,
        } => {
            let column = identifier(field);
            let filter = if *non_null {
                format!(" WHERE {column} IS NOT NULL")
            } else {
                String::new()
            };
            Ok(format!(
                "SELECT {column} AS value FROM ({}) AS ownership_rows{filter}",
                ownership_rows(rows, closure, tables)?
            ))
        }
        OwnershipSet::Union(parts) => Ok(parts
            .iter()
            .map(|part| ownership_set(part, closure, tables))
            .collect::<Result<Vec<_>, _>>()?
            .join(" UNION ")),
    }
}

/// Complete ownership inputs can contain rows absent from every producer-positive closure.
/// Capture each Full domain once into bounded IPC, retaining independent query cursors for each
/// grain. These runs are attempt-local derived inputs, never prior semantic acceptance.
struct FullInputs {
    session: SessionContext,
    aliases: Vec<String>,
    _directory: tempfile::TempDir,
    _charge: charged::StateCharge,
}
impl Drop for FullInputs {
    fn drop(&mut self) {
        for alias in &self.aliases { let _ = self.session.deregister_table(alias); }
    }
}
fn full_rows(rows: &OwnershipRows, inputs: &mut std::collections::BTreeSet<usize>) {
    if let OwnershipRows::Full { input, filters } = rows {
        inputs.insert(*input);
        for filter in filters { full_set(&filter.values, inputs); }
    }
}
fn full_set(set: &OwnershipSet, inputs: &mut std::collections::BTreeSet<usize>) {
    match set {
        OwnershipSet::Projection { rows, .. } => full_rows(rows, inputs),
        OwnershipSet::Union(parts) => for part in parts { full_set(part, inputs); },
    }
}
fn full_input_alias(directory: &std::path::Path, input: usize) -> String {
    // String registration normalizes unquoted identifiers; SQL reads quote the exact name.
    // Canonical lowercase ASCII keeps both boundaries bound to the same private catalog port.
    let directory = directory.file_name().expect("temporary run name").to_string_lossy();
    let identifier = directory.chars().map(|c| {
        if c.is_ascii_alphanumeric() { c.to_ascii_lowercase() } else { '_' }
    }).collect::<String>();
    format!("support_full_{identifier}_{input}")
}
async fn prepare_full_inputs(
    ownership: &SupportOwnership,
    tables: &mut [ClosureTable],
    session: &SessionContext,
    budget: &ResourceBudget,
    cancellation: &Cancellation,
) -> Result<FullInputs, ModelError> {
    use datafusion::arrow::ipc::writer::FileWriter;
    use datafusion::execution::options::ArrowReadOptions;
    let mut prepared = FullInputs { session: session.clone(), aliases: Vec::new(), _directory: tempfile::tempdir().map_err(ModelError::codec)?, _charge: charged::StateCharge::new(budget, "support-full-input-runs") };
    let mut complete = std::collections::BTreeSet::new();
    full_rows(&ownership.corpus, &mut complete);
    full_rows(&ownership.distributions, &mut complete);
    for index in complete {
        let table = tables.get_mut(index).ok_or(ModelError::Schema("support full input binding"))?;
        let path = prepared._directory.path().join(format!("input-{index}.arrow"));
        let mut writer = FileWriter::try_new(std::fs::File::create(&path).map_err(ModelError::codec)?, table.relation.schema())
            .map_err(ModelError::codec)?;
        let mut stream = crate::sql::query(session, &format!("SELECT * FROM {}", identifier(&table.alias)))
            .await.map_err(crate::sql::model_error)?.execute_stream().await.map_err(crate::sql::model_error)?;
        while let Some(batch) = stream.try_next().await.map_err(crate::sql::model_error)? {
            cancellation.check()?;
            let _transfer = budget.reserve("support-full-input-write", batch.get_array_memory_size().saturating_mul(2).saturating_add(4096))?;
            prepared._charge.grow(256)?;
            writer.write(&batch).map_err(ModelError::codec)?;
        }
        writer.finish().map_err(ModelError::codec)?;
        drop(writer);
        use std::io::{Read, Seek, SeekFrom};
        let mut file = std::fs::File::open(&path).map_err(ModelError::codec)?;
        file.seek(SeekFrom::End(-10)).map_err(ModelError::codec)?;
        let mut length = [0; 4];
        file.read_exact(&mut length).map_err(ModelError::codec)?;
        prepared._charge.grow((u32::from_le_bytes(length) as usize).saturating_mul(2).saturating_add(4096))?;
        let frame = session.read_arrow(path.to_string_lossy().into_owned(), ArrowReadOptions::default().schema(table.relation.schema().as_ref()))
            .await.map_err(crate::sql::model_error)?;
        // The unique directory name prevents simultaneous checks from replacing each other's
        // immutable ports in the supplied session catalog.
        let alias = full_input_alias(prepared._directory.path(), index);
        session.register_table(&alias, frame.into_view()).map_err(ModelError::codec)?;
        prepared.aliases.push(alias.clone());
        table.alias = alias;
    }
    Ok(prepared)
}

async fn validate_grain(
    invariant: &Invariant,
    ownership: &SupportOwnership,
    tables: &[ClosureTable],
    closure: &PreparedClosure,
    budget: &ResourceBudget,
    cancellation: &Cancellation,
) -> Result<(), ModelError> {
    let corpus = ownership_rows(&ownership.corpus, closure, tables)?;
    let distributions = ownership_rows(&ownership.distributions, closure, tables)?;
    let mut check = (invariant.create)(budget);
    for (index, input) in invariant.inputs.iter().enumerate() {
        if index != ownership.corpus_input && index != ownership.distributions_input {
            let mut rows = closure.ordered_input(index, input).await?.execute_stream().await.map_err(crate::sql::model_error)?;
            while let Some(batch) = rows.try_next().await.map_err(crate::sql::model_error)? {
                cancellation.check()?;
                check.visit_input(input, &batch)?;
            }
            continue;
        }
        let select = if index == ownership.corpus_input {
            corpus.clone()
        } else if index == ownership.distributions_input {
            distributions.clone()
        } else {
            closure.select(index)?
        };
        let order = input
            .order()
            .iter()
            .map(|field| identifier(field))
            .collect::<Vec<_>>()
            .join(",");
        let sql = format!(
            "SELECT * FROM ({select}) AS selected_rows{}",
            if order.is_empty() {
                String::new()
            } else {
                format!(" ORDER BY {order}")
            }
        );
        let mut rows = crate::sql::query(closure.session(), &sql)
            .await
            .map_err(crate::sql::model_error)?
            .execute_stream()
            .await
            .map_err(crate::sql::model_error)?;
        while let Some(batch) = rows.try_next().await.map_err(crate::sql::model_error)? {
            cancellation.check()?;
            check.visit_input(input, &batch)?;
        }
    }
    check.finish()
}

async fn validate_group(
    invariant: &Invariant,
    ownership: &SupportOwnership,
    tables: &[ClosureTable],
    prepared: &crate::consumed_rows::PreparedEdges,
    root: usize,
    ids: &[[u8; 16]],
    grains: &[Grain],
    budget: &ResourceBudget,
    cancellation: &Cancellation,
) -> Result<(), ModelError> {
    let roots = ids.iter().map(|key| PreparedRoot { table: root, key: *key, kind: PreparedRootKind::Physical }).collect::<Vec<_>>();
    let batch = prepared.batch_with_cancellation(&roots, budget, cancellation).await?;
    let mut full = std::collections::BTreeSet::new();
    full_rows(&ownership.corpus, &mut full);
    full_rows(&ownership.distributions, &mut full);
    let mut charge = charged::StateCharge::new(budget, "support-admission-columnar-union");
    charge.grow(invariant.inputs.len().saturating_mul(size_of::<Vec<arrow_array::RecordBatch>>()).saturating_add(4096))?;
    let mut union = (0..invariant.inputs.len()).map(|_| Vec::new()).collect::<Vec<_>>();
    // Complete Full ownership is supplied by its spill run. Hydrate every other bound port
    // once for the entire root group, preserving each input's own declared total order.
    for (index, input) in invariant.inputs.iter().enumerate() {
        if full.contains(&index) { continue; }
        let mut stream = batch.union.ordered_input(index, input).await?.execute_stream().await.map_err(crate::sql::model_error)?;
        while let Some(rows) = stream.try_next().await.map_err(crate::sql::model_error)? {
            cancellation.check()?;
            charge.grow(rows.get_array_memory_size().saturating_add(128))?;
            union[index].push(rows);
        }
    }
    let mut start = 0;
    while start < ids.len() {
        let end = start + grains[start..].partition_point(|grain| *grain == grains[start]);
        let partitions = (start..end).collect::<Vec<_>>();
        let selected = batch.select_partitions(&partitions, budget)?;
        let closure = selected.union;
        let mut partition_charge = charged::StateCharge::new(budget, "support-admission-columnar-partition");
        for (index, rows) in union.iter().enumerate() {
            if full.contains(&index) { continue; }
            let mut keys = charged::ChargedSet::default();
            for partition in start..end {
                for key in batch.keys(partition, index)? { keys.insert(&mut partition_charge, key)?; }
            }
            let mut selected_rows = Vec::new();
            for rows in rows {
                partition_charge.grow(rows.get_array_memory_size().saturating_add(rows.num_rows().saturating_mul(32)).saturating_add(4096))?;
                let mut mask = Vec::with_capacity(rows.num_rows());
                for row in 0..rows.num_rows() { mask.push(column(rows, "id", row)?.is_some_and(|key| keys.contains(&key))); }
                let filtered = datafusion::arrow::compute::filter_record_batch(rows, &arrow_array::BooleanArray::from(mask)).map_err(ModelError::codec)?;
                selected_rows.push(filtered);
            }
            closure.bind_prepared_rows(index, selected_rows)?;
        }
        validate_grain(invariant, ownership, tables, &closure, budget, cancellation).await?;
        // The borrowed batch providers are removed before their retained row charge is released.
        drop(closure);
        drop(partition_charge);
        start = end;
    }
    Ok(())
}

pub(crate) async fn validate_support(
    invariant: &Invariant,
    scope: &SupportScope,
    mut tables: Vec<ClosureTable>,
    model: &ValidatedModel,
    session: &SessionContext,
    budget: &ResourceBudget,
    cancellation: &Cancellation,
) -> Result<(), ModelError> {
    let declared = scope.program(invariant.inputs.clone(), model, budget)?;
    let _full_inputs = prepare_full_inputs(&declared.ownership, &mut tables, session, budget, cancellation).await?;
    let root = declared.root;
    let qualification = typed::<AssertionQualification>(&tables)
        .ok_or(ModelError::Schema("support qualification"))?;
    let prepared = crate::scope_compilation::lower_compiled(
        crate::scope_compilation::compile(&declared.program, model, budget, None)?,
        &tables,
        &scope_program::ScopeParameters(vec![]),
        budget,
    )?
    .prepare(session, budget)
    .await?;
    // LEFT JOIN keeps unsupported assertions and assertions with an absent qualification in the
    // root domain. The independent global reference pass still refuses every missing reference.
    let sql = format!(
        "SELECT a.id,q.scope,q.context FROM {} AS a LEFT JOIN {} AS q ON a.qualification=q.id ORDER BY q.scope,q.context,a.id",
        identifier(&tables[root].alias),
        identifier(&tables[qualification].alias)
    );
    let mut roots = crate::sql::query(session, &sql)
        .await
        .map_err(crate::sql::model_error)?
        .execute_stream()
        .await
        .map_err(crate::sql::model_error)?;
    let _root_charge = budget.reserve("support-admission-roots", ROOT_ROWS * 128)?;
    let mut ids = Vec::with_capacity(ROOT_ROWS);
    let mut grains = Vec::with_capacity(ROOT_ROWS);
    while let Some(batch) = roots.try_next().await.map_err(crate::sql::model_error)? {
        cancellation.check()?;
        for row in 0..batch.num_rows() {
            ids.push(column(&batch, "id", row)?.ok_or(ModelError::Schema("support assertion ID"))?);
            grains.push((column(&batch, "scope", row)?, column(&batch, "context", row)?));
            if ids.len() == ROOT_ROWS {
                validate_group(invariant, &declared.ownership, &tables, &prepared, root, &ids, &grains, budget, cancellation).await?;
                ids.clear();
                grains.clear();
            }
        }
    }
    if !ids.is_empty() {
        validate_group(invariant, &declared.ownership, &tables, &prepared, root, &ids, &grains, budget, cancellation).await?;
    }
    Ok(())
}
pub(crate) fn root_predicate(ids: &[[u8; 16]]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut predicate = String::with_capacity(9 + ids.len() * 37);
    predicate.push_str("id IN (");
    for (index, id) in ids.iter().enumerate() {
        if index != 0 {
            predicate.push(',');
        }
        predicate.push_str("X'");
        for byte in id {
            predicate.push(HEX[usize::from(byte >> 4)] as char);
            predicate.push(HEX[usize::from(byte & 15)] as char);
        }
        predicate.push('\'');
    }
    predicate.push(')');
    predicate
}

#[cfg(test)]
mod controls {
    use super::*;
    use datafusion::{datasource::MemTable, prelude::SessionConfig};
    use lctx_model::domain::{
        assertion::{Approximation, Evidence, ProviderSurface},
        attribution::{
            ExtractionMode, FactFamily, Fidelity, Modality, Origin, Provider, ProviderRun,
            RunFamily,
        },
        conditions::{Condition, ConditionNode},
        input::CorpusLibrary,
        source::{
            CoverageScope, Occurrence, OccurrenceRole, SourceArtifact, SyntaxKind,
            SyntaxObservation, SyntaxSupport,
        },
    };
    use std::sync::Arc;

    fn nominal<R>(byte: u8) -> Id<R> {
        serde_json::from_value(serde_json::to_value([byte; 16]).unwrap()).unwrap()
    }
    struct Fixture {
        invariant: Invariant,
        scope: SupportScope,
        tables: Vec<ClosureTable>,
        session: SessionContext,
        budget: ResourceBudget,
        assertion: SyntaxObservation,
        support: SyntaxSupport,
        artifact: SourceArtifact,
        run: ProviderRun,
    }
    impl Fixture {
        fn put<R: Record>(&self, rows: &[R]) {
            let index = typed::<R>(&self.tables).unwrap();
            let alias = &self.tables[index].alias;
            self.session.deregister_table(alias.as_str()).unwrap();
            let batch = R::encode(rows).unwrap();
            self.session
                .register_table(
                    alias.as_str(),
                    Arc::new(MemTable::try_new(batch.schema(), vec![vec![batch]]).unwrap()),
                )
                .unwrap();
        }
        fn new() -> Self {
            let model = lctx_model::domain::model().unwrap();
            let invariant = assertion::support_invariants::<SyntaxObservation, SyntaxSupport>()
                .pop()
                .unwrap();
            let budget = ResourceBudget::fixed(64 << 20).unwrap();
            let scope = (invariant.create)(&budget).support_scope().unwrap();
            let session =
                SessionContext::new_with_config(SessionConfig::new().with_target_partitions(2));
            let tables: Vec<_> = invariant
                .inputs
                .iter()
                .enumerate()
                .map(|(index, input)| ClosureTable {
                    relation: model.relation(input.name()).unwrap().clone(),
                    alias: format!("scope_input_{index}"),
                })
                .collect();
            for table in &tables {
                let batch = arrow_array::RecordBatch::new_empty(table.relation.schema().clone());
                session
                    .register_table(
                        table.alias.as_str(),
                        Arc::new(MemTable::try_new(batch.schema(), vec![vec![batch]]).unwrap()),
                    )
                    .unwrap();
            }
            let artifact =
                SourceArtifact::from_bytes(nominal(1), "selected.py".into(), b"x").unwrap();
            let occurrence = Occurrence {
                source: artifact.id(),
                start: 0,
                end: 1,
                syntax_kind: SyntaxKind::ExprName,
                role: OccurrenceRole::Syntax,
                structural_path: vec![],
            };
            let coverage = CoverageScope::Artifact {
                artifact: artifact.id(),
            };
            let condition = Condition {
                root: ConditionNode::True.id(),
            };
            let qualification = AssertionQualification {
                context: nominal(2),
                scope: coverage.id(),
                condition: condition.id(),
                modality: Modality::Definite,
                approximation: Approximation::Exact,
                assumptions: nominal(3),
            };
            let run = ProviderRun {
                provider: nominal::<Provider>(4),
                context: qualification.context,
                input: nominal(9),
                configuration: ContentHash::of(b"config"),
                requested_families: ContentHash::of(b"families"),
            };
            let surface = ProviderSurface {
                provider: run.provider,
                family: FactFamily::Syntax,
                name: "syntax".into(),
            };
            let evidence = Evidence::Occurrence {
                occurrence: occurrence.id(),
            };
            let assertion = SyntaxObservation {
                qualification: qualification.id(),
                occurrence: occurrence.id(),
                spelling: "x".into(),
            };
            let support = SyntaxSupport {
                assertion: assertion.id(),
                run: run.id(),
                surface: surface.id(),
                evidence: evidence.id(),
                origin: Origin::SourceObservation,
                mode: ExtractionMode::NativeTraversal,
                fidelity: Fidelity::NativeStructural,
            };
            let fixture = Self {
                invariant,
                scope,
                tables,
                session,
                budget,
                assertion,
                support,
                artifact,
                run,
            };
            fixture.put(std::slice::from_ref(&fixture.artifact));
            fixture.put(&[occurrence]);
            fixture.put(&[coverage]);
            fixture.put(&[ConditionNode::True]);
            fixture.put(&[condition]);
            fixture.put(&[qualification]);
            fixture.put(std::slice::from_ref(&fixture.run));
            fixture.put(&[surface]);
            fixture.put(&[evidence]);
            fixture.put(std::slice::from_ref(&fixture.assertion));
            fixture.put(std::slice::from_ref(&fixture.support));
            fixture.put(&[RunFamily {
                run: fixture.run.id(),
                family: FactFamily::Syntax,
            }]);
            fixture.put(&[CorpusLibrary {
                corpus: fixture.run.input,
                library: fixture.artifact.input,
            }]);
            fixture
        }
        async fn validate(&self) -> Result<(), ModelError> {
            validate_support(
                &self.invariant,
                &self.scope,
                self.tables.clone(),
                &lctx_model::domain::model()?,
                &self.session,
                &self.budget,
                &Cancellation::default(),
            )
            .await
        }
    }
    #[tokio::test]
    async fn full_input_aliases_resolve_identically_at_quoted_and_native_table_boundaries() {
        assert_eq!(full_input_alias(std::path::Path::new("/scratch/.tmpMiXeD-123"), 3), "support_full__tmpmixed_123_3");
        let session = SessionContext::new();
        let budget = ResourceBudget::fixed(1 << 20).unwrap();
        let row = input::Package { name: "full-input-alias".into() };
        let batch = input::Package::encode(std::slice::from_ref(&row)).unwrap();
        session.register_table("source_port", Arc::new(MemTable::try_new(batch.schema(), vec![vec![batch]]).unwrap())).unwrap();
        let mut tables = vec![ClosureTable { relation: Relation::of::<input::Package>(), alias: "source_port".into() }];
        let ownership = SupportOwnership { corpus_input: 0, corpus: Arc::new(OwnershipRows::Full { input: 0, filters: vec![] }), distributions_input: 0, distributions: Arc::new(OwnershipRows::Selected(0)) };
        let runs = prepare_full_inputs(&ownership, &mut tables, &session, &budget, &Cancellation::default()).await.unwrap();
        let alias = tables[0].alias.clone();
        assert!(alias.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_'));
        session.table_provider(alias.as_str()).await.unwrap();
        let batches = crate::sql::query(&session, &format!("SELECT * FROM {}", identifier(&alias))).await.unwrap().collect().await.unwrap();
        let rows = batches.iter().flat_map(|batch| input::Package::decode(batch).unwrap()).collect::<Vec<_>>();
        assert_eq!(rows, vec![row]);
        drop(runs);
        assert!(!session.table_exist(alias.as_str()).unwrap());
        assert_eq!(budget.reserved(), 0);
    }
    #[tokio::test]
    async fn full_input_partial_failure_removes_registered_runs_and_releases_charges() {
        let session = SessionContext::new();
        let budget = ResourceBudget::fixed(1 << 20).unwrap();
        let relation = Relation::of::<input::Package>();
        let batch = input::Package::encode(&[input::Package { name: "first-full-port".into() }]).unwrap();
        session.register_table("source_port", Arc::new(MemTable::try_new(batch.schema(), vec![vec![batch]]).unwrap())).unwrap();
        let mut tables = vec![ClosureTable { relation: relation.clone(), alias: "source_port".into() }, ClosureTable { relation, alias: "missing_second_port".into() }];
        let ownership = SupportOwnership { corpus_input: 0, corpus: Arc::new(OwnershipRows::Full { input: 0, filters: vec![] }), distributions_input: 1, distributions: Arc::new(OwnershipRows::Full { input: 1, filters: vec![] }) };
        let schema = session.catalog("datafusion").unwrap().schema("public").unwrap();
        let baseline = schema.table_names();
        assert!(prepare_full_inputs(&ownership, &mut tables, &session, &budget, &Cancellation::default()).await.is_err());
        assert!(tables[0].alias.starts_with("support_full_"), "first Full port was registered before injected second-port failure");
        assert_eq!(schema.table_names(), baseline);
        assert_eq!(budget.reserved(), 0);
    }
    #[tokio::test]
    async fn support_union_keeps_two_grains_and_complete_ownership_independent() {
        use std::sync::atomic::{AtomicUsize, Ordering};
        let mut fixture = Fixture::new();
        let index = typed::<AssertionQualification>(&fixture.tables).unwrap();
        let batches = crate::sql::query(&fixture.session, &format!("SELECT * FROM {}", identifier(&fixture.tables[index].alias))).await.unwrap().collect().await.unwrap();
        let qualification = batches.iter().flat_map(|batch| AssertionQualification::decode(batch).unwrap()).next().unwrap();
        let second_qualification = AssertionQualification { context: nominal(27), ..qualification.clone() };
        let second_run = ProviderRun { context: second_qualification.context, input: nominal(28), ..fixture.run.clone() };
        let second_assertion = SyntaxObservation { qualification: second_qualification.id(), ..fixture.assertion.clone() };
        let second_support = SyntaxSupport { assertion: second_assertion.id(), run: second_run.id(), ..fixture.support.clone() };
        fixture.put(&[qualification, second_qualification]);
        fixture.put(&[fixture.run.clone(), second_run.clone()]);
        fixture.put(&[fixture.assertion.clone(), second_assertion]);
        fixture.put(&[fixture.support.clone(), second_support]);
        fixture.put(&[RunFamily { run: fixture.run.id(), family: FactFamily::Syntax }, RunFamily { run: second_run.id(), family: FactFamily::Syntax }]);
        let first_membership = CorpusLibrary { corpus: fixture.run.input, library: fixture.artifact.input };
        fixture.put(&[first_membership.clone(), CorpusLibrary { corpus: second_run.input, library: fixture.artifact.input }]);
        let checks = Arc::new(AtomicUsize::new(0));
        let count = checks.clone();
        let original = fixture.invariant.create.clone();
        fixture.invariant.create = Arc::new(move |budget| { count.fetch_add(1, Ordering::Relaxed); original(budget) });
        fixture.validate().await.unwrap();
        assert_eq!(checks.load(Ordering::Relaxed), 2, "shared occurrence/evidence remains in two independent grain checks");
        assert_eq!(fixture.budget.reserved(), 0);
        fixture.put(&[first_membership]);
        assert!(fixture.validate().await.is_err(), "one grain cannot borrow another grain's complete CorpusLibrary membership");
        assert_eq!(fixture.budget.reserved(), 0);
    }
    #[tokio::test]
    async fn unsupported_roots_and_wrong_run_family_still_refuse() {
        let fixture = Fixture::new();
        fixture.validate().await.unwrap();
        fixture.put::<SyntaxSupport>(&[]);
        let error = fixture.validate().await.unwrap_err();
        assert!(
            error
                .to_string()
                .contains("assertion has no attributed support"),
            "{error}"
        );
        fixture.put(std::slice::from_ref(&fixture.support));
        fixture.put(&[RunFamily {
            run: fixture.run.id(),
            family: FactFamily::Calls,
        }]);
        assert!(fixture.validate().await.is_err());
        assert_eq!(fixture.budget.reserved(), 0);
    }
    #[tokio::test]
    async fn ownership_selects_actual_artifacts_not_all_input_neighbors() {
        let fixture = Fixture::new();
        let mut memberships = vec![CorpusLibrary {
            corpus: fixture.run.input,
            library: fixture.artifact.input,
        }];
        memberships.extend((10..=200).map(|byte| CorpusLibrary {
            corpus: fixture.run.input,
            library: nominal(byte),
        }));
        fixture.put(&memberships);
        let root = declared(
            &fixture.tables,
            &fixture.invariant.inputs,
            &fixture.scope.assertion,
        )
        .unwrap();
        let model = lctx_model::domain::model().unwrap();
        let scope_program = fixture
            .scope
            .program(fixture.invariant.inputs.clone(), &model, &fixture.budget)
            .unwrap();
        let edges = crate::scope_compilation::lower(&scope_program.program, &fixture.tables)
            .unwrap()
            .prepare(&fixture.session, &fixture.budget)
            .await
            .unwrap();
        let closure = edges
            .grain(
                root,
                &root_predicate(&[*fixture.assertion.id().bytes()]),
                &fixture.budget,
            )
            .await
            .unwrap();
        let corpus =
            ownership_rows(&scope_program.ownership.corpus, &closure, &fixture.tables).unwrap();
        let batches = crate::sql::query(closure.session(), &corpus)
            .await
            .unwrap()
            .collect()
            .await
            .unwrap();
        let rows = batches
            .iter()
            .flat_map(|batch| CorpusLibrary::decode(batch).unwrap())
            .collect::<Vec<_>>();
        assert_eq!(rows, vec![memberships[0].clone()]);
        drop(closure);
        drop(edges);
        drop(scope_program);
        fixture.validate().await.unwrap();
        fixture.put::<CorpusLibrary>(&[]);
        assert!(fixture.validate().await.is_err());
        assert_eq!(fixture.budget.reserved(), 0);
    }
    #[tokio::test]
    async fn absent_qualification_is_not_removed_by_grain_selection() {
        let fixture = Fixture::new();
        fixture.put::<AssertionQualification>(&[]);
        let error = fixture.validate().await.unwrap_err();
        assert!(
            error
                .to_string()
                .contains("assertion qualification missing"),
            "{error}"
        );
        assert_eq!(fixture.budget.reserved(), 0);
    }
    #[tokio::test]
    async fn rich_support_state_is_released_between_bounded_grains() {
        use std::sync::atomic::{AtomicUsize, Ordering};
        let mut fixture = Fixture::new();
        let assertions: Vec<_> = (0..ROOT_ROWS * 2 + 1)
            .map(|index| SyntaxObservation {
                spelling: format!("spelling-{index}"),
                ..fixture.assertion.clone()
            })
            .collect();
        let supports: Vec<_> = assertions
            .iter()
            .map(|row| SyntaxSupport {
                assertion: row.id(),
                ..fixture.support.clone()
            })
            .collect();
        fixture.put(&assertions);
        fixture.put(&supports);
        let checks = Arc::new(AtomicUsize::new(0));
        let count = checks.clone();
        let original = fixture.invariant.create.clone();
        fixture.invariant.create = Arc::new(move |budget| {
            count.fetch_add(1, Ordering::Relaxed);
            original(budget)
        });
        fixture.validate().await.unwrap();
        assert_eq!(checks.load(Ordering::Relaxed), 3);
        assert_eq!(fixture.budget.reserved(), 0);
        fixture.put(&supports[..supports.len() - 1]);
        let error = fixture.validate().await.unwrap_err();
        assert!(
            error
                .to_string()
                .contains("assertion has no attributed support"),
            "{error}"
        );
        assert_eq!(fixture.budget.reserved(), 0);
    }
    #[test]
    fn dependency_epochs_are_selected_explicitly() {
        use stages::PublicationBoundary;
        let inputs = vec![
            ValidationInput::of::<types::TypeSequenceMember>(&["id"])
                .at_epoch(PublicationBoundary::Facts),
            ValidationInput::of::<types::TypeTerm>(&["id"]),
            ValidationInput::of::<types::TypeTerm>(&["id"]).at_epoch(PublicationBoundary::Facts),
        ];
        assert_eq!(
            field_target(&inputs, 0, TypeId::of::<types::TypeTerm>()).unwrap(),
            Some(2)
        );
        let mut absent = inputs;
        absent[0] = ValidationInput::of::<types::TypeSequenceMember>(&["id"])
            .at_epoch(PublicationBoundary::Dispatch);
        assert!(field_target(&absent, 0, TypeId::of::<types::TypeTerm>()).is_err());
    }
}
