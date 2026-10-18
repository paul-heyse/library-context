//! Compact selection for assertion support; the model's support predicate owns admission.
//! Immutable nominal edges are prepared once, then rich rows live only for one bounded grain.
use crate::{
    admission::Cancellation,
    consumed_rows::{ClosureTable, PreparedClosure, PreparedRoot, PreparedRootKind, identifier},
    scoped_positions::{ColumnarInputs, PreparedPositions},
};
use arrow_array::{Array, FixedSizeBinaryArray};
use datafusion::prelude::SessionContext;
use futures::{TryStreamExt, future::BoxFuture};
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
pub fn declared(
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
pub fn field_target(
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
pub fn column(
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
/// Lower the model-owned selection directly. Repeated ownership expressions must
/// never become nested SQL text reparsed through IN/derived-table recursion.
fn ownership_rows<'a>(
    rows: &'a OwnershipRows,
    closure: &'a PreparedClosure,
    tables: &'a [ClosureTable],
) -> BoxFuture<'a, Result<datafusion::dataframe::DataFrame, ModelError>> {
    Box::pin(async move {
        match rows {
            OwnershipRows::Selected(input) => closure.selected_input(*input).await,
            OwnershipRows::Full { input, filters } => {
                let table = tables
                    .get(*input)
                    .ok_or(ModelError::Schema("ownership physical input absent"))?;
                let mut frame = closure
                    .session()
                    .table(datafusion::common::TableReference::bare(
                        table.alias.clone(),
                    ))
                    .await
                    .map_err(crate::sql::model_error)?;
                for filter in filters {
                    let values = ownership_set(&filter.values, closure, tables).await?;
                    // LeftSemi preserves every matching left row, with no duplicate
                    // multiplication; ordinary equality never matches a NULL key.
                    // Explicit side-specific keys also handle a left field named value.
                    frame = frame
                        .join(
                            values,
                            datafusion::logical_expr::JoinType::LeftSemi,
                            &[filter.field],
                            &["value"],
                            None,
                        )
                        .map_err(crate::sql::model_error)?;
                }
                Ok(frame)
            }
        }
    })
}
fn ownership_set<'a>(
    set: &'a OwnershipSet,
    closure: &'a PreparedClosure,
    tables: &'a [ClosureTable],
) -> BoxFuture<'a, Result<datafusion::dataframe::DataFrame, ModelError>> {
    Box::pin(async move {
        match set {
            OwnershipSet::Projection {
                rows,
                field,
                non_null,
            } => {
                let mut frame = ownership_rows(rows, closure, tables).await?;
                if *non_null {
                    frame = frame
                        .filter(datafusion::prelude::col(*field).is_not_null())
                        .map_err(crate::sql::model_error)?;
                }
                frame
                    .select(vec![datafusion::prelude::col(*field).alias("value")])
                    .map_err(crate::sql::model_error)
            }
            OwnershipSet::Union(parts) => {
                let mut parts = parts.iter();
                let first = parts
                    .next()
                    .ok_or(ModelError::Schema("empty ownership union"))?;
                let mut frame = ownership_set(first, closure, tables).await?;
                for part in parts {
                    frame = frame
                        .union_distinct(ownership_set(part, closure, tables).await?)
                        .map_err(crate::sql::model_error)?;
                }
                Ok(frame)
            }
        }
    })
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
        for alias in &self.aliases {
            let _ = self.session.deregister_table(alias);
        }
    }
}
fn full_rows(rows: &OwnershipRows, inputs: &mut std::collections::BTreeSet<usize>) {
    if let OwnershipRows::Full { input, filters } = rows {
        inputs.insert(*input);
        for filter in filters {
            full_set(&filter.values, inputs);
        }
    }
}
fn full_set(set: &OwnershipSet, inputs: &mut std::collections::BTreeSet<usize>) {
    match set {
        OwnershipSet::Projection { rows, .. } => full_rows(rows, inputs),
        OwnershipSet::Union(parts) => {
            for part in parts {
                full_set(part, inputs);
            }
        }
    }
}
fn full_input_alias(directory: &std::path::Path, input: usize) -> String {
    // String registration normalizes unquoted identifiers; SQL reads quote the exact name.
    // Canonical lowercase ASCII keeps both boundaries bound to the same private catalog port.
    let directory = directory
        .file_name()
        .expect("temporary run name")
        .to_string_lossy();
    let identifier = directory
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() {
                c.to_ascii_lowercase()
            } else {
                '_'
            }
        })
        .collect::<String>();
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
    let mut prepared = FullInputs {
        session: session.clone(),
        aliases: Vec::new(),
        _directory: tempfile::tempdir().map_err(ModelError::codec)?,
        _charge: charged::StateCharge::new(budget, "support-full-input-runs"),
    };
    let mut complete = std::collections::BTreeSet::new();
    full_rows(&ownership.corpus, &mut complete);
    full_rows(&ownership.distributions, &mut complete);
    for index in complete {
        let table = tables
            .get_mut(index)
            .ok_or(ModelError::Schema("support full input binding"))?;
        let path = prepared
            ._directory
            .path()
            .join(format!("input-{index}.arrow"));
        let mut writer = FileWriter::try_new(
            std::fs::File::create(&path).map_err(ModelError::codec)?,
            table.relation.schema(),
        )
        .map_err(ModelError::codec)?;
        let mut stream = crate::sql::query(
            session,
            &format!("SELECT * FROM {}", identifier(&table.alias)),
        )
        .await
        .map_err(crate::sql::model_error)?
        .execute_stream()
        .await
        .map_err(crate::sql::model_error)?;
        while let Some(batch) = stream.try_next().await.map_err(crate::sql::model_error)? {
            cancellation.check()?;
            let _transfer = budget.reserve(
                "support-full-input-write",
                batch
                    .get_array_memory_size()
                    .saturating_mul(2)
                    .saturating_add(4096),
            )?;
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
        prepared._charge.grow(
            (u32::from_le_bytes(length) as usize)
                .saturating_mul(2)
                .saturating_add(4096),
        )?;
        let frame = session
            .read_arrow(
                path.to_string_lossy().into_owned(),
                ArrowReadOptions::default().schema(table.relation.schema().as_ref()),
            )
            .await
            .map_err(crate::sql::model_error)?;
        // The unique directory name prevents simultaneous checks from replacing each other's
        // immutable ports in the supplied session catalog.
        let alias = full_input_alias(prepared._directory.path(), index);
        session
            .register_table(&alias, frame.into_view())
            .map_err(ModelError::codec)?;
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
    let corpus = ownership_rows(&ownership.corpus, closure, tables).await?;
    let distributions = ownership_rows(&ownership.distributions, closure, tables).await?;
    let mut check = (invariant.create)(budget);
    for (index, input) in invariant.inputs.iter().enumerate() {
        if index != ownership.corpus_input && index != ownership.distributions_input {
            let mut rows = closure
                .ordered_input(index, input)
                .await?
                .execute_stream()
                .await
                .map_err(crate::sql::model_error)?;
            while let Some(batch) = rows.try_next().await.map_err(crate::sql::model_error)? {
                cancellation.check()?;
                check.visit_input(input, &batch)?;
            }
            continue;
        }
        let frame = if index == ownership.corpus_input {
            corpus.clone()
        } else {
            distributions.clone()
        };
        let mut rows = closure
            .order_input(frame, input)?
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

struct SupportGroupValidation<'a> {
    invariant: &'a Invariant,
    ownership: &'a SupportOwnership,
    tables: &'a [ClosureTable],
    prepared: &'a crate::consumed_rows::PreparedEdges,
    root: usize,
    budget: &'a ResourceBudget,
    cancellation: &'a Cancellation,
}
impl SupportGroupValidation<'_> {
    async fn validate(&self, ids: &[[u8; 16]], grains: &[Grain]) -> Result<(), ModelError> {
        let Self {
            invariant,
            ownership,
            tables,
            prepared,
            root,
            budget,
            cancellation,
        } = *self;
        let roots = ids
            .iter()
            .map(|key| PreparedRoot {
                table: root,
                key: *key,
                kind: PreparedRootKind::Physical,
            })
            .collect::<Vec<_>>();
        let batch = prepared
            .batch_with_cancellation(&roots, budget, cancellation)
            .await?;
        let mut full = std::collections::BTreeSet::new();
        full_rows(&ownership.corpus, &mut full);
        full_rows(&ownership.distributions, &mut full);
        let mut union = ColumnarInputs::new(&batch, &invariant.inputs, budget)?;
        // Complete Full ownership is supplied by its spill run. Hydrate every other bound port
        // once for the entire root group, preserving each input's own declared total order.
        for (index, input) in invariant.inputs.iter().enumerate() {
            if full.contains(&index) {
                continue;
            }
            let mut stream = batch
                .union
                .ordered_input(index, input)
                .await?
                .execute_stream()
                .await
                .map_err(crate::sql::model_error)?;
            while let Some(rows) = stream.try_next().await.map_err(crate::sql::model_error)? {
                cancellation.check()?;
                union.push(index, &rows)?;
            }
        }
        let positions =
            PreparedPositions::new(&batch, &invariant.inputs, &union, budget, cancellation)?;
        let mut start = 0;
        while start < ids.len() {
            let end = start + grains[start..].partition_point(|grain| *grain == grains[start]);
            let partitions = (start..end).collect::<Vec<_>>();
            // Declare retained gathers before the scope so error unwinding removes providers before
            // releasing their backing charges. Every checker still gets its own fresh state.
            let _handles = budget.reserve(
                "support-gather-handles",
                invariant
                    .inputs
                    .len()
                    .saturating_mul(size_of::<crate::scoped_positions::GatheredRows<'_>>()),
            )?;
            let mut retained = Vec::with_capacity(invariant.inputs.len());
            let selected = batch.select_partitions(&partitions, budget)?;
            let closure = selected.union;
            for index in 0..invariant.inputs.len() {
                if full.contains(&index) {
                    continue;
                }
                let gathered = positions.gather(&partitions, index, index, cancellation)?;
                closure.bind_prepared_rows(index, gathered.rows().to_vec())?;
                retained.push(gathered);
            }
            validate_grain(invariant, ownership, tables, &closure, budget, cancellation).await?;
            // The borrowed batch providers are removed before their retained row charge is released.
            drop(closure);
            drop(retained);
            start = end;
        }
        Ok(())
    }
}

pub async fn validate_support(
    invariant: &Invariant,
    scope: &SupportScope,
    mut tables: Vec<ClosureTable>,
    model: &ValidatedModel,
    session: &SessionContext,
    budget: &ResourceBudget,
    cancellation: &Cancellation,
) -> Result<(), ModelError> {
    let declared = scope.program(invariant.inputs.clone(), model, budget)?;
    let _full_inputs = prepare_full_inputs(
        &declared.ownership,
        &mut tables,
        session,
        budget,
        cancellation,
    )
    .await?;
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
    let group = SupportGroupValidation {
        invariant,
        ownership: &declared.ownership,
        tables: &tables,
        prepared: &prepared,
        root,
        budget,
        cancellation,
    };
    let _root_charge = budget.reserve("support-admission-roots", ROOT_ROWS * 128)?;
    let mut ids = Vec::with_capacity(ROOT_ROWS);
    let mut grains = Vec::with_capacity(ROOT_ROWS);
    while let Some(batch) = roots.try_next().await.map_err(crate::sql::model_error)? {
        cancellation.check()?;
        for row in 0..batch.num_rows() {
            ids.push(column(&batch, "id", row)?.ok_or(ModelError::Schema("support assertion ID"))?);
            grains.push((
                column(&batch, "scope", row)?,
                column(&batch, "context", row)?,
            ));
            if ids.len() == ROOT_ROWS {
                group.validate(&ids, &grains).await?;
                ids.clear();
                grains.clear();
            }
        }
    }
    if !ids.is_empty() {
        group.validate(&ids, &grains).await?;
    }
    Ok(())
}
pub fn root_predicate(ids: &[[u8; 16]]) -> String {
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
        assert_eq!(
            full_input_alias(std::path::Path::new("/scratch/.tmpMiXeD-123"), 3),
            "support_full__tmpmixed_123_3"
        );
        let session = SessionContext::new();
        let budget = ResourceBudget::fixed(1 << 20).unwrap();
        let row = input::Package {
            name: "full-input-alias".into(),
        };
        let batch = input::Package::encode(std::slice::from_ref(&row)).unwrap();
        session
            .register_table(
                "source_port",
                Arc::new(MemTable::try_new(batch.schema(), vec![vec![batch]]).unwrap()),
            )
            .unwrap();
        let mut tables = vec![ClosureTable {
            relation: Relation::of::<input::Package>(),
            alias: "source_port".into(),
        }];
        let ownership = SupportOwnership {
            corpus_input: 0,
            corpus: Arc::new(OwnershipRows::Full {
                input: 0,
                filters: vec![],
            }),
            distributions_input: 0,
            distributions: Arc::new(OwnershipRows::Selected(0)),
        };
        let runs = prepare_full_inputs(
            &ownership,
            &mut tables,
            &session,
            &budget,
            &Cancellation::default(),
        )
        .await
        .unwrap();
        let alias = tables[0].alias.clone();
        assert!(
            alias
                .chars()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
        );
        session.table_provider(alias.as_str()).await.unwrap();
        let batches = crate::sql::query(&session, &format!("SELECT * FROM {}", identifier(&alias)))
            .await
            .unwrap()
            .collect()
            .await
            .unwrap();
        let rows = batches
            .iter()
            .flat_map(|batch| input::Package::decode(batch).unwrap())
            .collect::<Vec<_>>();
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
        let batch = input::Package::encode(&[input::Package {
            name: "first-full-port".into(),
        }])
        .unwrap();
        session
            .register_table(
                "source_port",
                Arc::new(MemTable::try_new(batch.schema(), vec![vec![batch]]).unwrap()),
            )
            .unwrap();
        let mut tables = vec![
            ClosureTable {
                relation: relation.clone(),
                alias: "source_port".into(),
            },
            ClosureTable {
                relation,
                alias: "missing_second_port".into(),
            },
        ];
        let ownership = SupportOwnership {
            corpus_input: 0,
            corpus: Arc::new(OwnershipRows::Full {
                input: 0,
                filters: vec![],
            }),
            distributions_input: 1,
            distributions: Arc::new(OwnershipRows::Full {
                input: 1,
                filters: vec![],
            }),
        };
        let schema = session
            .catalog("datafusion")
            .unwrap()
            .schema("public")
            .unwrap();
        let baseline = schema.table_names();
        assert!(
            prepare_full_inputs(
                &ownership,
                &mut tables,
                &session,
                &budget,
                &Cancellation::default()
            )
            .await
            .is_err()
        );
        assert!(
            tables[0].alias.starts_with("support_full_"),
            "first Full port was registered before injected second-port failure"
        );
        assert_eq!(schema.table_names(), baseline);
        assert_eq!(budget.reserved(), 0);
    }
    #[tokio::test]
    async fn support_union_keeps_two_grains_and_complete_ownership_independent() {
        use std::sync::atomic::{AtomicUsize, Ordering};
        let mut fixture = Fixture::new();
        let index = typed::<AssertionQualification>(&fixture.tables).unwrap();
        let batches = crate::sql::query(
            &fixture.session,
            &format!("SELECT * FROM {}", identifier(&fixture.tables[index].alias)),
        )
        .await
        .unwrap()
        .collect()
        .await
        .unwrap();
        let qualification = batches
            .iter()
            .flat_map(|batch| AssertionQualification::decode(batch).unwrap())
            .next()
            .unwrap();
        let second_qualification = AssertionQualification {
            context: nominal(27),
            ..qualification.clone()
        };
        let second_run = ProviderRun {
            context: second_qualification.context,
            input: nominal(28),
            ..fixture.run.clone()
        };
        let second_assertion = SyntaxObservation {
            qualification: second_qualification.id(),
            ..fixture.assertion.clone()
        };
        let second_support = SyntaxSupport {
            assertion: second_assertion.id(),
            run: second_run.id(),
            ..fixture.support.clone()
        };
        fixture.put(&[qualification, second_qualification]);
        fixture.put(&[fixture.run.clone(), second_run.clone()]);
        fixture.put(&[fixture.assertion.clone(), second_assertion]);
        fixture.put(&[fixture.support.clone(), second_support]);
        fixture.put(&[
            RunFamily {
                run: fixture.run.id(),
                family: FactFamily::Syntax,
            },
            RunFamily {
                run: second_run.id(),
                family: FactFamily::Syntax,
            },
        ]);
        let first_membership = CorpusLibrary {
            corpus: fixture.run.input,
            library: fixture.artifact.input,
        };
        fixture.put(&[
            first_membership.clone(),
            CorpusLibrary {
                corpus: second_run.input,
                library: fixture.artifact.input,
            },
        ]);
        let checks = Arc::new(AtomicUsize::new(0));
        let count = checks.clone();
        let original = fixture.invariant.create.clone();
        fixture.invariant.create = Arc::new(move |budget| {
            count.fetch_add(1, Ordering::Relaxed);
            original(budget)
        });
        fixture.validate().await.unwrap();
        assert_eq!(
            checks.load(Ordering::Relaxed),
            2,
            "shared occurrence/evidence remains in two independent grain checks"
        );
        assert_eq!(fixture.budget.reserved(), 0);
        fixture.put(&[first_membership]);
        assert!(
            fixture.validate().await.is_err(),
            "one grain cannot borrow another grain's complete CorpusLibrary membership"
        );
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
        let corpus = ownership_rows(&scope_program.ownership.corpus, &closure, &fixture.tables)
            .await
            .unwrap();
        let batches = corpus.collect().await.unwrap();
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
    async fn typed_ownership_executes_actual_model_distribution_dag_without_sql_expansion() {
        use lctx_model::domain::input::{DistributionRole, InputDistribution};
        let mut fixture = Fixture::new();
        let release = nominal(41);
        let scope = CoverageScope::Release { release };
        let qi = typed::<AssertionQualification>(&fixture.tables).unwrap();
        let batches = fixture
            .session
            .table(fixture.tables[qi].alias.as_str())
            .await
            .unwrap()
            .collect()
            .await
            .unwrap();
        let mut qualification = batches
            .iter()
            .flat_map(|batch| AssertionQualification::decode(batch).unwrap())
            .next()
            .unwrap();
        qualification.scope = scope.id();
        fixture.assertion.qualification = qualification.id();
        fixture.support.assertion = fixture.assertion.id();
        fixture.put(&[scope]);
        fixture.put(&[qualification]);
        fixture.put(std::slice::from_ref(&fixture.assertion));
        fixture.put(std::slice::from_ref(&fixture.support));
        let wanted = InputDistribution {
            input: fixture.artifact.input,
            release,
            role: DistributionRole::FirstParty,
        };
        fixture.put(&[
            wanted.clone(),
            InputDistribution {
                input: nominal(42),
                ..wanted.clone()
            },
            InputDistribution {
                release: nominal(43),
                ..wanted.clone()
            },
        ]);
        let member = CorpusLibrary {
            corpus: fixture.run.input,
            library: fixture.artifact.input,
        };
        fixture.put(&[
            member.clone(),
            CorpusLibrary {
                library: nominal(44),
                ..member.clone()
            },
            CorpusLibrary {
                corpus: nominal(45),
                ..member.clone()
            },
        ]);
        let model = lctx_model::domain::model().unwrap();
        let program = fixture
            .scope
            .program(fixture.invariant.inputs.clone(), &model, &fixture.budget)
            .unwrap();
        let root = declared(
            &fixture.tables,
            &fixture.invariant.inputs,
            &fixture.scope.assertion,
        )
        .unwrap();
        let edges = crate::scope_compilation::lower(&program.program, &fixture.tables)
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
        let corpus = ownership_rows(&program.ownership.corpus, &closure, &fixture.tables)
            .await
            .unwrap()
            .collect()
            .await
            .unwrap();
        let distributions =
            ownership_rows(&program.ownership.distributions, &closure, &fixture.tables)
                .await
                .unwrap()
                .collect()
                .await
                .unwrap();
        assert_eq!(
            corpus
                .iter()
                .flat_map(|batch| CorpusLibrary::decode(batch).unwrap())
                .collect::<Vec<_>>(),
            vec![member]
        );
        assert_eq!(
            distributions
                .iter()
                .flat_map(|batch| InputDistribution::decode(batch).unwrap())
                .collect::<Vec<_>>(),
            vec![wanted]
        );
        drop(closure);
        drop(edges);
        drop(program);
        assert_eq!(fixture.budget.reserved(), 0);
    }
    #[tokio::test]
    async fn typed_ownership_preserves_null_duplicates_scope_and_literal_table_names() {
        use lctx_model::domain::{flow::FlowDefinitionObservation, lexical::BindingEventKind};
        let session = SessionContext::new();
        let budget = ResourceBudget::fixed(64 << 20).unwrap();
        let hit = FlowDefinitionObservation {
            qualification: nominal(1),
            definition: nominal(2),
            scope: nominal(3),
            kind: BindingEventKind::Assignment,
            value: Some(nominal(4)),
        };
        let null = FlowDefinitionObservation {
            definition: nominal(5),
            value: None,
            ..hit.clone()
        };
        let foreign = FlowDefinitionObservation {
            definition: nominal(6),
            scope: nominal(7),
            ..hit.clone()
        };
        let left = [hit.clone(), hit.clone(), null.clone(), foreign];
        let right = [hit.clone(), hit.clone(), null];
        let tables = ["Full.Left", "Full.Right"]
            .into_iter()
            .zip([&left[..], &right[..]])
            .map(|(alias, rows)| {
                let batch = FlowDefinitionObservation::encode(rows).unwrap();
                session
                    .register_table(
                        datafusion::common::TableReference::bare(alias),
                        Arc::new(MemTable::try_new(batch.schema(), vec![vec![batch]]).unwrap()),
                    )
                    .unwrap();
                ClosureTable {
                    relation: Relation::of::<FlowDefinitionObservation>(),
                    alias: alias.into(),
                }
            })
            .collect::<Vec<_>>();
        let edges = crate::consumed_rows::NominalClosure::new(tables.clone())
            .unwrap()
            .prepare(&session, &budget)
            .await
            .unwrap();
        let closure = edges.grain(0, "FALSE", &budget).await.unwrap();
        let full = Arc::new(OwnershipRows::Full {
            input: 1,
            filters: vec![],
        });
        let projection = Arc::new(OwnershipSet::Projection {
            rows: full.clone(),
            field: "value",
            non_null: false,
        });
        let values = OwnershipSet::Union(vec![projection.clone(), projection.clone()]);
        let batches = ownership_set(&values, &closure, &tables)
            .await
            .unwrap()
            .collect()
            .await
            .unwrap();
        assert_eq!(
            batches.iter().map(|batch| batch.num_rows()).sum::<usize>(),
            2,
            "UNION is distinct including one NULL"
        );
        assert_eq!(
            batches
                .iter()
                .map(|batch| batch.column(0).null_count())
                .sum::<usize>(),
            1
        );
        let nonnull = OwnershipSet::Projection {
            rows: full.clone(),
            field: "value",
            non_null: true,
        };
        let batches = ownership_set(&nonnull, &closure, &tables)
            .await
            .unwrap()
            .collect()
            .await
            .unwrap();
        assert_eq!(
            batches.iter().map(|batch| batch.num_rows()).sum::<usize>(),
            2,
            "projection retains duplicate non-NULL values"
        );
        assert_eq!(
            batches
                .iter()
                .map(|batch| batch.column(0).null_count())
                .sum::<usize>(),
            0
        );
        let selected = OwnershipRows::Full {
            input: 0,
            filters: vec![
                lctx_model::domain::admission_scope_program::OwnershipFilter {
                    field: "value",
                    values: Arc::new(values),
                },
                lctx_model::domain::admission_scope_program::OwnershipFilter {
                    field: "scope",
                    values: Arc::new(OwnershipSet::Projection {
                        rows: full,
                        field: "scope",
                        non_null: false,
                    }),
                },
            ],
        };
        let batches = ownership_rows(&selected, &closure, &tables)
            .await
            .unwrap()
            .collect()
            .await
            .unwrap();
        assert_eq!(
            batches
                .iter()
                .flat_map(|batch| FlowDefinitionObservation::decode(batch).unwrap())
                .collect::<Vec<_>>(),
            vec![hit.clone(), hit],
            "NULL never matches, foreign scope is excluded, left duplicates survive and right duplicates do not multiply them"
        );
        assert!(matches!(
            ownership_set(&OwnershipSet::Union(vec![]), &closure, &tables).await,
            Err(ModelError::Schema("empty ownership union"))
        ));
        assert!(
            ownership_rows(
                &OwnershipRows::Full {
                    input: tables.len(),
                    filters: vec![]
                },
                &closure,
                &tables
            )
            .await
            .is_err()
        );
        drop(closure);
        drop(edges);
        assert_eq!(budget.reserved(), 0);
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
