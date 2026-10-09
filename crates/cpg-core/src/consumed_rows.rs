//! Explicit completed-input dispatch for pure kernels. There are no persisted grants or epochs.
use crate::workspace::CompletedInputs;
use datafusion::prelude::SessionContext;
use futures::TryStreamExt;
use lctx_model::domain::{
    analysis::sources::CompletedInput, charged, resources::ResourceBudget, *,
};
use std::{
    any::TypeId,
    collections::{BTreeMap, BTreeSet},
};
/// Expected coverage needs artifact identity, input ownership and the model's bounded
/// classifier property. Ordinary selected algorithm reads still receive canonical rows.
pub(crate) async fn stream_artifact_admission(
    access: &CompletedInputs,
    input: &ValidationInput,
    session: &SessionContext,
    admission: &mut analysis::expected::CoverageAdmission<'_>,
) -> Result<bool, ModelError> {
    if input.type_id() != TypeId::of::<source::SourceArtifact>() {
        return Ok(false);
    }
    let permit = access.read_at::<source::SourceArtifact>(input.prefix())?;
    let table = access.table_for(input)?;
    let selected = format!(
        "SELECT {} FROM {}",
        analysis::expected::CoverageAdmission::artifact_property_columns(),
        identifier(&table)
    );
    stream_query_at(
        &permit,
        input,
        access,
        session,
        &selected,
        |permit, batch| admission.visit_artifact_properties(permit, batch),
    )
    .await?;
    Ok(true)
}
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
pub fn stream_at<'a, R: Record>(
    input: &'a CompletedInput<R>,
    declaration: &'a ValidationInput,
    inputs: &'a CompletedInputs,
    session: &'a SessionContext,
    consume: impl FnMut(&CompletedInput<R>, &arrow_array::RecordBatch) -> Result<(), ModelError>
    + Send
    + 'a,
) -> futures::future::BoxFuture<'a, Result<(), ModelError>> {
    stream_where_at(input, declaration, inputs, session, None, consume)
}
/// Read the exact immutable input through an owner-selected predicate before rich decoding.
/// The predicate is compiler SQL, never a provider string or a source of semantic authority.
pub fn stream_where_at<'a, R: Record>(
    input: &'a CompletedInput<R>,
    declaration: &'a ValidationInput,
    inputs: &'a CompletedInputs,
    session: &'a SessionContext,
    predicate: Option<&str>,
    consume: impl FnMut(&CompletedInput<R>, &arrow_array::RecordBatch) -> Result<(), ModelError>
    + Send
    + 'a,
) -> futures::future::BoxFuture<'a, Result<(), ModelError>> {
    let selected = inputs.table_at::<R>(declaration.prefix()).map(|table| {
        format!(
            "SELECT * FROM {}{}",
            identifier(&table),
            predicate
                .map(|predicate| format!(" WHERE ({predicate})"))
                .unwrap_or_default()
        )
    });
    // Table selection errors precede permit errors on this route. Checking immutable
    // captured identities does not plan or start a stream; errors are delivered on polling.
    let checked = if selected.is_ok() {
        check_stream_input(input, declaration, inputs)
    } else {
        Ok(())
    };
    let mut consume = consume;
    let visit: BatchConsumer<'a> = Box::new(move |batch| consume(input, batch));
    stream_checked_owned(checked, declaration, session, selected, None, visit)
}
/// Stream an owner-declared SELECT after checking the exact nominal record and captured view.
/// Joins and closure predicates are supplied by the owning semantic kernel.
pub fn stream_query_at<'a, R: Record>(
    input: &'a CompletedInput<R>,
    declaration: &'a ValidationInput,
    inputs: &'a CompletedInputs,
    session: &'a SessionContext,
    selected: &str,
    consume: impl FnMut(&CompletedInput<R>, &arrow_array::RecordBatch) -> Result<(), ModelError>
    + Send
    + 'a,
) -> futures::future::BoxFuture<'a, Result<(), ModelError>> {
    stream_query_filter_at(input, declaration, inputs, session, selected, None, consume)
}
/// Preserve an owner-authored DataFusion predicate at the checked completed-input boundary.
/// The predicate remains an Expr; it is never converted into SQL text.
pub(crate) fn stream_query_filter_at<'a, R: Record>(
    input: &'a CompletedInput<R>,
    declaration: &'a ValidationInput,
    inputs: &'a CompletedInputs,
    session: &'a SessionContext,
    selected: &str,
    predicate: Option<datafusion::logical_expr::Expr>,
    mut consume: impl FnMut(&CompletedInput<R>, &arrow_array::RecordBatch) -> Result<(), ModelError>
    + Send
    + 'a,
) -> futures::future::BoxFuture<'a, Result<(), ModelError>> {
    let checked = check_stream_input(input, declaration, inputs);
    let visit: BatchConsumer<'a> = Box::new(move |batch| consume(input, batch));
    stream_checked_owned(
        checked,
        declaration,
        session,
        Ok(selected.to_owned()),
        predicate,
        visit,
    )
}

fn check_stream_input<R: Record>(
    input: &CompletedInput<R>,
    declaration: &ValidationInput,
    inputs: &CompletedInputs,
) -> Result<(), ModelError> {
    if declaration.type_id() != TypeId::of::<R>() || declaration.name() != R::NAME {
        return Err(ModelError::Invalid(
            "scoped stream record declaration mismatch".into(),
        ));
    }
    if input.source() != inputs.read_at::<R>(declaration.prefix())?.source() {
        return Err(ModelError::Conflict(
            "scoped stream completed input mismatch",
        ));
    }
    Ok(())
}

type BatchConsumer<'a> =
    Box<dyn FnMut(&arrow_array::RecordBatch) -> Result<(), ModelError> + Send + 'a>;

/// Own callback erasure and selected SQL before creating the asynchronous state machine.
/// Record-specific validation/decoding remain at the typed caller boundary.
fn stream_checked_owned<'a>(
    checked: Result<(), ModelError>,
    declaration: &'a ValidationInput,
    session: &'a SessionContext,
    selected: Result<String, ModelError>,
    predicate: Option<datafusion::logical_expr::Expr>,
    mut consume: BatchConsumer<'a>,
) -> futures::future::BoxFuture<'a, Result<(), ModelError>> {
    Box::pin(async move {
        let selected = selected?;
        checked?;
        stream_batches(declaration, session, &selected, predicate, consume.as_mut()).await
    })
}

/// Record-independent physical planning and iteration. Dispatch occurs once per batch.
/// Detached validation callers establish their invariant/table/type binding before entering.
/// This physical loop grants no completed-input authority of its own.
pub(crate) fn stream_batches<'a>(
    declaration: &'a ValidationInput,
    session: &'a SessionContext,
    selected: &'a str,
    predicate: Option<datafusion::logical_expr::Expr>,
    consume: &'a mut (dyn FnMut(&arrow_array::RecordBatch) -> Result<(), ModelError> + Send),
) -> futures::future::BoxFuture<'a, Result<(), ModelError>> {
    Box::pin(async move {
        let order = declaration
            .order()
            .iter()
            .map(|column| identifier(column))
            .collect::<Vec<_>>()
            .join(",");
        let sql = format!(
            "SELECT * FROM ({selected}) AS scoped_input{}",
            if order.is_empty() {
                String::new()
            } else {
                format!(" ORDER BY {order}")
            }
        );
        let mut selected = crate::sql::query(session, &sql)
            .await
            .map_err(ModelError::codec)?;
        if let Some(predicate) = predicate {
            selected = selected.filter(predicate).map_err(ModelError::codec)?;
        }
        let mut batches = selected.execute_stream().await.map_err(ModelError::codec)?;
        while let Some(batch) = batches.try_next().await.map_err(ModelError::codec)? {
            consume(&batch)?;
            tokio::task::yield_now().await;
        }
        Ok(())
    })
}
pub(crate) fn identifier(name: &str) -> String {
    format!("\"{}\"", name.replace('"', "\"\""))
}
#[cfg(test)]
mod checked_stream_controls {
    use super::*;
    use std::sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    };

    #[tokio::test]
    async fn stream_adapter_refuses_before_planning_or_visiting_and_releases_callback() {
        let budget = ResourceBudget::fixed(1 << 20).unwrap();
        let charge = budget.reserve("stream-callback-control", 4096).unwrap();
        let visited = Arc::new(AtomicBool::new(false));
        let observed = visited.clone();
        let consume: BatchConsumer<'_> = Box::new(move |_| {
            let _retained = &charge;
            observed.store(true, Ordering::Relaxed);
            Ok(())
        });
        let declaration = ValidationInput::of::<input::Package>(&["id"]);
        let session = SessionContext::new();
        let future = stream_checked_owned(
            Err(ModelError::Conflict("stream admission refusal")),
            &declaration,
            &session,
            Ok("not valid SQL".to_owned()),
            None,
            consume,
        );
        assert_eq!(budget.reserved(), 4096);
        assert!(!visited.load(Ordering::Relaxed));
        assert!(matches!(
            future.await,
            Err(ModelError::Conflict("stream admission refusal"))
        ));
        assert!(!visited.load(Ordering::Relaxed));
        assert_eq!(budget.reserved(), 0);
    }

    #[tokio::test]
    async fn stream_table_selection_error_precedes_admission_error() {
        let declaration = ValidationInput::of::<input::Package>(&["id"]);
        let session = SessionContext::new();
        let mut visited = false;
        let consume: BatchConsumer<'_> = Box::new(|_| {
            visited = true;
            Ok(())
        });
        let result = stream_checked_owned(
            Err(ModelError::Conflict("later permit failure")),
            &declaration,
            &session,
            Err(ModelError::Conflict("earlier table selection failure")),
            None,
            consume,
        )
        .await;
        assert!(matches!(
            result,
            Err(ModelError::Conflict("earlier table selection failure"))
        ));
        assert!(!visited);
    }
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
    lowering_charges: Vec<Box<dyn lctx_model::domain::resources::Reservation>>,
    programs: Vec<std::sync::Arc<scope_program::CompiledScopeProgram>>,
    program_charge: Option<charged::StateCharge>,
    tables: Vec<ClosureTable>,
    pairs: BTreeSet<(usize, usize, String)>,
    native: BTreeMap<(usize, usize, String), NativeEdge>,
}
#[derive(Clone, Copy)]
struct NativeEdge {
    source: usize,
    target: usize,
    table: usize,
    field: &'static str,
    reverse: bool,
    require_owner_presence: bool,
}
impl NominalClosure {
    pub(crate) fn into_pair_queries(self) -> Vec<(usize, usize, String)> {
        self.pairs.into_iter().collect()
    }
    pub(crate) fn extend(&mut self, other: Self) -> Result<(), ModelError> {
        if self.tables.len() != other.tables.len()
            || self
                .tables
                .iter()
                .zip(&other.tables)
                .any(|(a, b)| a.alias != b.alias || a.relation.type_id() != b.relation.type_id())
        {
            return Err(ModelError::Conflict("scope program physical port bindings"));
        }
        let budget = other
            .program_charge
            .as_ref()
            .and_then(|charge| charge.budget())
            .cloned();
        if let Some(budget) = budget {
            for program in other.programs {
                self.retain_program(program, &budget)?;
            }
        }
        self.lowering_charges.extend(other.lowering_charges);
        self.pairs.extend(other.pairs);
        self.native.extend(other.native);
        Ok(())
    }
    pub(crate) fn retain_lowering(
        &mut self,
        charge: Box<dyn lctx_model::domain::resources::Reservation>,
    ) {
        self.lowering_charges.push(charge);
    }
    pub(crate) fn retain_program(
        &mut self,
        program: std::sync::Arc<scope_program::CompiledScopeProgram>,
        budget: &ResourceBudget,
    ) -> Result<(), ModelError> {
        if self
            .programs
            .iter()
            .any(|owner| std::sync::Arc::ptr_eq(owner, &program))
        {
            return Ok(());
        }
        let charge = self
            .program_charge
            .get_or_insert_with(|| charged::StateCharge::new(budget, "scope-program-owners"));
        if !charge
            .budget()
            .expect("scope owner budget")
            .shares_pool(budget)
        {
            return Err(ModelError::Conflict("scope program owner foreign budget"));
        }
        let additional = if self.programs.len() == self.programs.capacity() {
            self.programs.capacity().max(4)
        } else {
            0
        };
        charge
            .grow(additional.saturating_mul(size_of::<
                std::sync::Arc<scope_program::CompiledScopeProgram>,
            >()))?;
        self.programs.reserve_exact(additional);
        self.programs.push(program);
        Ok(())
    }
    pub fn new(tables: Vec<ClosureTable>) -> Result<Self, ModelError> {
        if tables.is_empty() || tables.iter().any(|table| table.alias.is_empty()) {
            return Err(ModelError::Invalid(
                "closure requires immutable table bindings".into(),
            ));
        }
        Ok(Self {
            lowering_charges: Vec::new(),
            programs: Vec::new(),
            program_charge: None,
            tables,
            pairs: BTreeSet::new(),
            native: BTreeMap::new(),
        })
    }
    /// Follow a declared nominal field. Target selection is explicit, so two vocabulary epochs
    /// of the same relation cannot silently collapse into a latest-alias dependency.
    pub fn follow(&mut self, source: usize, field: &str, target: usize) -> Result<(), ModelError> {
        let from = self.table(source)?;
        let to = self.table(target)?;
        let descriptor = from
            .relation
            .fields()
            .iter()
            .find(|candidate| candidate.name() == field)
            .ok_or_else(|| ModelError::Invalid("closure field absent".into()))?;
        if descriptor.target().map(|(kind, _)| kind) != Some(to.relation.type_id())
            || descriptor.list()
        {
            return Err(ModelError::Invalid(
                "closure field has another nominal target or list shape".into(),
            ));
        }
        let sql = format!(
            "SELECT id AS source_id, {} AS target_id FROM {} WHERE {} IS NOT NULL",
            identifier(field),
            identifier(&from.alias),
            identifier(field)
        );
        let edge = NativeEdge {
            source,
            target,
            table: source,
            field: descriptor.name(),
            reverse: false,
            require_owner_presence: false,
        };
        self.native.insert((source, target, sql.clone()), edge);
        self.pairs.insert((source, target, sql));
        Ok(())
    }
    /// Include rows owned through this nominal field, as well as their forward owner reference.
    /// Callers declare this only for actual membership, never for every incoming reference.
    pub fn own(
        &mut self,
        member: usize,
        owner_field: &str,
        owner: usize,
    ) -> Result<(), ModelError> {
        self.own_selected(member, owner_field, owner, false, true)
    }
    /// One directed owned arc. Unlike `own`, it cannot activate a virtual owner from a
    /// supporting member's outgoing reference; forward dependencies are declared separately.
    pub fn own_reverse(
        &mut self,
        member: usize,
        owner_field: &str,
        owner: usize,
    ) -> Result<(), ModelError> {
        self.own_selected(member, owner_field, owner, false, false)
    }
    /// Reverse membership requires an owner physically present in its exact captured table.
    /// Dangling nominal references never create an owner merely by reaching its key.
    pub fn own_existing(
        &mut self,
        member: usize,
        owner_field: &str,
        owner: usize,
    ) -> Result<(), ModelError> {
        self.own_selected(member, owner_field, owner, true, true)
    }
    fn own_selected(
        &mut self,
        member: usize,
        owner_field: &str,
        owner: usize,
        require_owner_presence: bool,
        forward: bool,
    ) -> Result<(), ModelError> {
        let descriptor = self
            .table(member)?
            .relation
            .fields()
            .iter()
            .find(|f| f.name() == owner_field)
            .ok_or(ModelError::Schema("closure owner field absent"))?;
        if descriptor.list()
            || descriptor.target().map(|(kind, _)| kind)
                != Some(self.table(owner)?.relation.type_id())
        {
            return Err(ModelError::Schema("closure owner nominal type"));
        }
        if forward {
            self.follow(member, owner_field, owner)?;
        }
        let from = self.table(member)?;
        let sql = if require_owner_presence {
            let owners = self.table(owner)?;
            format!(
                "SELECT o.id AS source_id,m.id AS target_id FROM {} m JOIN {} o ON m.{}=o.id",
                identifier(&from.alias),
                identifier(&owners.alias),
                identifier(owner_field)
            )
        } else {
            format!(
                "SELECT {} AS source_id, id AS target_id FROM {} WHERE {} IS NOT NULL",
                identifier(owner_field),
                identifier(&from.alias),
                identifier(owner_field)
            )
        };
        let field = from
            .relation
            .fields()
            .iter()
            .find(|descriptor| descriptor.name() == owner_field)
            .expect("validated owner field")
            .name();
        self.native.insert(
            (owner, member, sql.clone()),
            NativeEdge {
                source: owner,
                target: member,
                table: member,
                field,
                reverse: true,
                require_owner_presence,
            },
        );
        self.pairs.insert((owner, member, sql));
        Ok(())
    }
    /// Explicit semantic edges whose key includes a non-nominal domain, such as RunFamily.
    /// SELECT must return source_id and target_id (both nominal 16-byte IDs). The owner supplies
    /// the complete family/context predicates; this utility does not infer applicability.
    pub fn pairs(
        &mut self,
        source: usize,
        target: usize,
        select: String,
    ) -> Result<(), ModelError> {
        self.table(source)?;
        self.table(target)?;
        if select.trim().is_empty() {
            return Err(ModelError::Invalid("closure pair SELECT absent".into()));
        }
        self.pairs.insert((source, target, select));
        Ok(())
    }
    fn table(&self, index: usize) -> Result<&ClosureTable, ModelError> {
        self.tables
            .get(index)
            .ok_or_else(|| ModelError::Invalid("closure table index absent".into()))
    }
    fn pair_sql(source: usize, target: usize, sql: &str) -> String {
        format!(
            "SELECT CAST({source} AS BIGINT) AS source_kind, source_id, CAST({target} AS BIGINT) AS target_kind, target_id FROM ({sql}) AS pair_rows WHERE source_id IS NOT NULL AND target_id IS NOT NULL"
        )
    }
    /// Project nominal edges once for an immutable dependency set into an external ordered IPC
    /// stream. All subsequent grains reuse these compact edges without rescanning rich inputs.
    pub async fn prepare(
        &self,
        session: &SessionContext,
        budget: &ResourceBudget,
    ) -> Result<PreparedEdges, ModelError> {
        use datafusion::{
            arrow::ipc::writer::FileWriter,
            execution::{options::ArrowReadOptions, session_state::SessionStateBuilder},
            prelude::col,
        };
        use std::{
            fs::File,
            io::{Read, Seek, SeekFrom},
        };
        let state = session.state();
        let catalogs = state.catalog_list().clone();
        let config = state
            .config()
            .clone()
            .with_create_default_catalog_and_schema(false)
            .with_target_partitions(state.config().target_partitions().max(2))
            .with_repartition_joins(true)
            .set_bool("datafusion.optimizer.prefer_hash_join", false);
        let session = SessionContext::new_with_state(
            SessionStateBuilder::new_from_existing(state)
                .with_config(config)
                .with_catalog_list(catalogs)
                .build(),
        );
        let directory = tempfile::tempdir().map_err(ModelError::codec)?;
        let pending = directory.path().join("pending-edges.arrow");
        let path = directory.path().join("nominal-edges.arrow");
        let schema = std::sync::Arc::new(arrow_schema::Schema::new(vec![
            arrow_schema::Field::new("source_kind", arrow_schema::DataType::Int64, false),
            arrow_schema::Field::new(
                "source_id",
                arrow_schema::DataType::FixedSizeBinary(16),
                false,
            ),
            arrow_schema::Field::new("target_kind", arrow_schema::DataType::Int64, false),
            arrow_schema::Field::new(
                "target_id",
                arrow_schema::DataType::FixedSizeBinary(16),
                false,
            ),
        ]));
        // Materialize one pair at a time. ORDER BY over a UNION permits the optimizer to
        // introduce one live sort/merge reservation per branch; an IPC scan is one sort input.
        // The staging envelope covers its writer/footer block descriptors, not per-edge state.
        let mut staging_charge =
            charged::StateCharge::new(budget, "semantic-edge-staging-metadata");
        staging_charge.grow(4096)?;
        // Copy fixed nominal values into one charged storage batch. Upstream sorting/filtering
        // can emit tiny fragments; neither IPC footer nor sparse index may grow per fragment.
        let coalescing_charge = budget.reserve(
            "semantic-edge-coalescing",
            resources::TRANSFER_ROWS
                .saturating_mul(48 * 3)
                .saturating_add(4096),
        )?;
        let mut buffer = EdgeBuffer::new(schema.clone());
        let mut staged =
            FileWriter::try_new(File::create(&pending).map_err(ModelError::codec)?, &schema)
                .map_err(ModelError::codec)?;
        let mut provider_charge =
            charged::StateCharge::new(budget, "semantic-edge-provider-bindings");
        provider_charge.grow(
            self.tables
                .len()
                .saturating_mul(
                    size_of::<std::sync::Arc<dyn datafusion::catalog::TableProvider>>()
                        + size_of::<ClosureTable>(),
                )
                .saturating_add(
                    self.tables
                        .iter()
                        .map(|table| table.alias.len())
                        .sum::<usize>(),
                )
                .saturating_add(self.native.len().saturating_mul(size_of::<NativeEdge>()))
                .saturating_add(self.programs.len().saturating_mul(size_of::<
                    std::sync::Arc<scope_program::CompiledScopeProgram>,
                >())),
        )?;
        let mut providers = Vec::with_capacity(self.tables.len());
        for table in &self.tables {
            providers.push(
                session
                    .table_provider(table.alias.as_str())
                    .await
                    .map_err(ModelError::codec)?,
            );
        }
        let mut native_edges = Vec::with_capacity(self.native.len());
        for (source, target, sql) in &self.pairs {
            if let Some(edge) = self.native.get(&(*source, *target, sql.clone()))
                && lctx_surrealdb::compiler_provider::is_native_table(&providers[edge.table])
                && (!edge.require_owner_presence
                    || lctx_surrealdb::compiler_provider::is_native_table(&providers[edge.source]))
            {
                native_edges.push(*edge);
                continue;
            }
            // Only detached finite sources and owner-authored contextual pair queries need
            // a compact bulk adjacency universe. Native declared references use the frontier.
            let frame = crate::sql::query(&session, &Self::pair_sql(*source, *target, sql))
                .await
                .map_err(ModelError::codec)?;
            let mut stream = frame.execute_stream().await.map_err(ModelError::codec)?;
            while let Some(batch) = stream.try_next().await.map_err(ModelError::codec)? {
                let _transfer = budget.reserve(
                    "semantic-edge-transfer",
                    logical_batch_bytes(&batch)?
                        .saturating_mul(3)
                        .saturating_add(4096),
                )?;
                validate_edge_batch(&batch)?;
                buffer.push(&batch, |edge_batch| {
                    staging_charge.grow(128)?;
                    staged.write(&edge_batch).map_err(ModelError::codec)
                })?;
                tokio::task::yield_now().await;
            }
        }
        if let Some(batch) = buffer.finish()? {
            staging_charge.grow(128)?;
            staged.write(&batch).map_err(ModelError::codec)?;
        }
        staged.finish().map_err(ModelError::codec)?;
        drop(staged);
        let frame = session
            .read_arrow(
                pending.to_string_lossy().into_owned(),
                ArrowReadOptions::default().schema(schema.as_ref()),
            )
            .await
            .map_err(ModelError::codec)?
            .sort(vec![
                col("source_kind").sort(true, false),
                col("source_id").sort(true, false),
                col("target_kind").sort(true, false),
                col("target_id").sort(true, false),
            ])
            .map_err(ModelError::codec)?;
        let mut stream = frame.execute_stream().await.map_err(ModelError::codec)?;
        let mut batches = charged::ChargedVec::default();
        let mut index_charge = charged::StateCharge::new(budget, "semantic-edge-batch-index");
        let mut writer =
            FileWriter::try_new(File::create(&path).map_err(ModelError::codec)?, &schema)
                .map_err(ModelError::codec)?;
        while let Some(batch) = stream.try_next().await.map_err(ModelError::codec)? {
            let _transfer = budget.reserve(
                "semantic-edge-transfer",
                logical_batch_bytes(&batch)?
                    .saturating_mul(3)
                    .saturating_add(4096),
            )?;
            validate_edge_batch(&batch)?;
            buffer.push(&batch, |edge_batch| {
                write_edge_batch(&edge_batch, &mut writer, &mut batches, &mut index_charge)
            })?;
            tokio::task::yield_now().await;
        }
        if let Some(batch) = buffer.finish()? {
            write_edge_batch(&batch, &mut writer, &mut batches, &mut index_charge)?;
        }
        writer.finish().map_err(ModelError::codec)?;
        drop(buffer);
        drop(coalescing_charge);
        drop(writer);
        drop(stream);
        drop(staging_charge);
        std::fs::remove_file(&pending).map_err(ModelError::codec)?;
        // Reserve the reader's footer/block metadata before FileReader allocates it in a grain.
        let mut file = File::open(&path).map_err(ModelError::codec)?;
        file.seek(SeekFrom::End(-10)).map_err(ModelError::codec)?;
        let mut length = [0; 4];
        file.read_exact(&mut length).map_err(ModelError::codec)?;
        let footer = u32::from_le_bytes(length) as usize;
        let reader_allowance = footer
            .saturating_mul(2)
            .saturating_add(batches.len().saturating_mul(64))
            .saturating_add(4096);
        // Bulk edge preparation retains compact topology. Each native grain binds its keys
        // before payload hydration; detached finite tables use the same keys in a local join.
        let state = session.state();
        let catalogs = state.catalog_list().clone();
        let config = state
            .config()
            .clone()
            .with_create_default_catalog_and_schema(false)
            .with_repartition_joins(false)
            .set_bool("datafusion.optimizer.prefer_hash_join", true);
        let session = SessionContext::new_with_state(
            SessionStateBuilder::new_from_existing(state)
                .with_config(config)
                .with_catalog_list(catalogs)
                .build(),
        );
        Ok(PreparedEdges(std::sync::Arc::new(EdgeSource {
            session,
            path,
            batches,
            reader_allowance,
            tables: self.tables.clone(),
            _programs: self.programs.clone(),
            providers,
            native_edges,
            _provider_charge: provider_charge,
            _index_charge: index_charge,
            _directory: directory,
        })))
    }
}
/// A fixed Arrow builder window copies only nominal values, releasing each input fragment.
/// Arrow's generic BatchCoalescer retains FixedSizeBinary slices until concatenation; that can
/// retain one IPC buffer/header per tiny fragment. Fixed-size builders avoid that physical shape.
struct EdgeBuffer {
    schema: arrow_schema::SchemaRef,
    source_kind: arrow_array::builder::Int64Builder,
    source_id: arrow_array::builder::FixedSizeBinaryBuilder,
    target_kind: arrow_array::builder::Int64Builder,
    target_id: arrow_array::builder::FixedSizeBinaryBuilder,
    rows: usize,
}
impl EdgeBuffer {
    fn new(schema: arrow_schema::SchemaRef) -> Self {
        let rows = resources::TRANSFER_ROWS;
        Self {
            schema,
            source_kind: arrow_array::builder::Int64Builder::with_capacity(rows),
            source_id: arrow_array::builder::FixedSizeBinaryBuilder::with_capacity(rows, 16),
            target_kind: arrow_array::builder::Int64Builder::with_capacity(rows),
            target_id: arrow_array::builder::FixedSizeBinaryBuilder::with_capacity(rows, 16),
            rows: 0,
        }
    }
    fn push(
        &mut self,
        batch: &arrow_array::RecordBatch,
        mut emit: impl FnMut(arrow_array::RecordBatch) -> Result<(), ModelError>,
    ) -> Result<(), ModelError> {
        for row in 0..batch.num_rows() {
            let source = edge_key(batch, "source_kind", "source_id", row)?;
            let target = edge_key(batch, "target_kind", "target_id", row)?;
            self.source_kind.append_value(source.0);
            self.source_id
                .append_value(source.1)
                .map_err(ModelError::codec)?;
            self.target_kind.append_value(target.0);
            self.target_id
                .append_value(target.1)
                .map_err(ModelError::codec)?;
            self.rows += 1;
            if self.rows == resources::TRANSFER_ROWS {
                emit(self.finish()?.expect("full nominal storage batch"))?;
            }
        }
        Ok(())
    }
    fn finish(&mut self) -> Result<Option<arrow_array::RecordBatch>, ModelError> {
        if self.rows == 0 {
            return Ok(None);
        }
        let batch = arrow_array::RecordBatch::try_new(
            self.schema.clone(),
            vec![
                std::sync::Arc::new(self.source_kind.finish()),
                std::sync::Arc::new(self.source_id.finish()),
                std::sync::Arc::new(self.target_kind.finish()),
                std::sync::Arc::new(self.target_id.finish()),
            ],
        )
        .map_err(ModelError::codec)?;
        self.rows = 0;
        Ok(Some(batch))
    }
}
fn write_edge_batch(
    batch: &arrow_array::RecordBatch,
    writer: &mut datafusion::arrow::ipc::writer::FileWriter<std::fs::File>,
    batches: &mut charged::ChargedVec<EdgeBatch>,
    charge: &mut charged::StateCharge,
) -> Result<(), ModelError> {
    use std::io::Seek;
    let first = edge_key(batch, "source_kind", "source_id", 0)?;
    let last = edge_key(batch, "source_kind", "source_id", batch.num_rows() - 1)?;
    if first > last
        || batches
            .last()
            .is_some_and(|previous: &EdgeBatch| previous.last > first)
    {
        return Err(ModelError::Schema(
            "nominal edges are not ordered by complete source key",
        ));
    }
    charge.grow(128)?;
    let before = writer
        .get_mut()
        .stream_position()
        .map_err(ModelError::codec)?;
    writer.write(batch).map_err(ModelError::codec)?;
    let after = writer
        .get_mut()
        .stream_position()
        .map_err(ModelError::codec)?;
    let ipc_bytes = usize::try_from(after - before).map_err(ModelError::codec)?;
    batches.push(
        charge,
        EdgeBatch {
            first,
            last,
            rows: batch.num_rows(),
            ipc_bytes,
        },
    )
}
fn scope_alias(kind: &str) -> String {
    use std::sync::atomic::{AtomicU64, Ordering};
    static NEXT_SCOPE: AtomicU64 = AtomicU64::new(0);
    format!(
        "__semantic_{kind}_{}",
        NEXT_SCOPE.fetch_add(1, Ordering::Relaxed)
    )
}
type NominalKey = (i64, [u8; 16]);
/// Exactly one entry per IPC record batch. Duplicate source ranges may span arbitrarily many
/// adjacent batches; searching by BOTH first and last includes every such batch.
struct EdgeBatch {
    first: NominalKey,
    last: NominalKey,
    rows: usize,
    ipc_bytes: usize,
}
impl HeapSize for EdgeBatch {}
struct EdgeSource {
    _programs: Vec<std::sync::Arc<scope_program::CompiledScopeProgram>>,
    session: SessionContext,
    path: std::path::PathBuf,
    batches: charged::ChargedVec<EdgeBatch>,
    reader_allowance: usize,
    tables: Vec<ClosureTable>,
    providers: Vec<std::sync::Arc<dyn datafusion::catalog::TableProvider>>,
    native_edges: Vec<NativeEdge>,
    _provider_charge: charged::StateCharge,
    _index_charge: charged::StateCharge,
    _directory: tempfile::TempDir,
}
impl EdgeSource {
    fn matching_batches(&self, key: NominalKey) -> std::ops::Range<usize> {
        let start = self.batches.partition_point(|batch| batch.last < key);
        let end = self.batches.partition_point(|batch| batch.first <= key);
        start..end
    }
}
/// Four fixed, non-null nominal columns are the complete edge row shape. Buffer
/// capacities may include other arrays in the same IPC block; logical payload cannot.
fn validate_edge_batch(batch: &arrow_array::RecordBatch) -> Result<(), ModelError> {
    use arrow_schema::DataType;
    let fields = [
        ("source_kind", DataType::Int64),
        ("source_id", DataType::FixedSizeBinary(16)),
        ("target_kind", DataType::Int64),
        ("target_id", DataType::FixedSizeBinary(16)),
    ];
    if batch.num_columns() != fields.len()
        || fields.iter().enumerate().any(|(index, (name, kind))| {
            batch.schema().field(index).name().as_str() != *name
                || batch.column(index).data_type() != kind
                || batch.column(index).null_count() != 0
        })
        || logical_batch_bytes(batch)? != batch.num_rows().saturating_mul(48)
    {
        return Err(ModelError::Schema(
            "nominal edge batch requires four non-null fixed nominal columns",
        ));
    }
    Ok(())
}
fn edge_key(
    batch: &arrow_array::RecordBatch,
    kind: &str,
    id: &str,
    row: usize,
) -> Result<NominalKey, ModelError> {
    use arrow_array::{Array, FixedSizeBinaryArray, Int64Array};
    let kinds = batch
        .column_by_name(kind)
        .and_then(|column| column.as_any().downcast_ref::<Int64Array>())
        .ok_or(ModelError::Schema("nominal edge kind must be Int64"))?;
    let ids = batch
        .column_by_name(id)
        .and_then(|column| column.as_any().downcast_ref::<FixedSizeBinaryArray>())
        .filter(|ids| ids.value_length() == 16)
        .ok_or(ModelError::Schema("nominal edge ID must have 16 bytes"))?;
    if row >= batch.num_rows() || kinds.is_null(row) || ids.is_null(row) {
        return Err(ModelError::Schema("nominal edge key absent"));
    }
    Ok((
        kinds.value(row),
        ids.value(row).try_into().map_err(ModelError::codec)?,
    ))
}
/// One FileReader and at most one current batch. Reader/block metadata and the decoded edge
/// batch are charged before allocation; the cache is replaced before reading another batch.
struct EdgeCursor<'a> {
    source: &'a EdgeSource,
    reader: datafusion::arrow::ipc::reader::FileReader<std::fs::File>,
    current: Option<(
        usize,
        arrow_array::RecordBatch,
        Box<dyn resources::Reservation>,
    )>,
    _reader_charge: Box<dyn resources::Reservation>,
}
impl<'a> EdgeCursor<'a> {
    fn new(source: &'a EdgeSource, budget: &ResourceBudget) -> Result<Self, ModelError> {
        let charge = budget.reserve("semantic-edge-reader", source.reader_allowance)?;
        let reader = datafusion::arrow::ipc::reader::FileReader::try_new(
            std::fs::File::open(&source.path).map_err(ModelError::codec)?,
            None,
        )
        .map_err(ModelError::codec)?;
        if reader.num_batches() != source.batches.len() {
            return Err(ModelError::Schema(
                "nominal edge sparse index does not match IPC batches",
            ));
        }
        Ok(Self {
            source,
            reader,
            current: None,
            _reader_charge: charge,
        })
    }
    fn batch(
        &mut self,
        index: usize,
        budget: &ResourceBudget,
    ) -> Result<&arrow_array::RecordBatch, ModelError> {
        if self
            .current
            .as_ref()
            .is_none_or(|(current, _, _)| *current != index)
        {
            self.current = None;
            let descriptor = &self.source.batches[index];
            // FileReader owns one metadata+body block buffer, shared by all four arrays.
            // get_array_memory_size counts that entire capacity once per column and is not
            // the logical shape. The recorded serialized block includes IPC metadata/padding.
            let retained = descriptor.ipc_bytes.saturating_add(4096);
            let mut charge = budget.reserve(
                "semantic-edge-current-batch",
                descriptor.ipc_bytes.saturating_mul(2).saturating_add(4096),
            )?;
            self.reader.set_index(index).map_err(ModelError::codec)?;
            let batch = self
                .reader
                .next()
                .ok_or(ModelError::Schema("nominal edge IPC batch missing"))?
                .map_err(ModelError::codec)?;
            validate_edge_batch(&batch)?;
            if batch.num_rows() != descriptor.rows {
                return Err(ModelError::Schema(
                    "nominal edge batch does not match its sparse descriptor",
                ));
            }
            charge.try_resize(retained)?;
            self.current = Some((index, batch, charge));
        }
        Ok(&self.current.as_ref().expect("current edge batch").1)
    }
    fn targets(
        &mut self,
        key: NominalKey,
        budget: &ResourceBudget,
        mut accept: impl FnMut(NominalKey) -> Result<(), ModelError>,
    ) -> Result<(), ModelError> {
        for index in self.source.matching_batches(key) {
            let batch = self.batch(index, budget)?;
            // Find the first matching row within this selected batch without scanning its prefix.
            let mut left = 0usize;
            let mut right = batch.num_rows();
            while left < right {
                let middle = left + (right - left) / 2;
                if edge_key(batch, "source_kind", "source_id", middle)? < key {
                    left = middle + 1;
                } else {
                    right = middle;
                }
            }
            for row in left..batch.num_rows() {
                if edge_key(batch, "source_kind", "source_id", row)? != key {
                    break;
                }
                accept(edge_key(batch, "target_kind", "target_id", row)?)?;
            }
        }
        Ok(())
    }
}
/// Dependency-set preparation, reused by source/context/family grains. It selects rows only;
/// all ordinary owner predicates and the independent global reference pass still apply.
#[derive(Clone)]
pub struct PreparedEdges(std::sync::Arc<EdgeSource>);
/// One exact selected-key demand and the reservation retained by its native provider.
struct NativeFrontier {
    keys: std::sync::Arc<Vec<[u8; 16]>>,
    charge: std::sync::Arc<charged::StateCharge>,
}
impl PreparedEdges {
    /// Forward fields share one exact table/frontier demand. Decode only their nominal columns;
    /// each declaration retains its target namespace, including duplicate fields across epochs.
    /// Strict reverse links reuse the same id projection to establish physical owner presence.
    async fn native_forward_targets(
        &self,
        source: usize,
        edges: &[NativeEdge],
        frontier: NativeFrontier,
        budget: &ResourceBudget,
        cancellation: Option<&crate::workspace::Cancellation>,
        mut accept: impl FnMut([u8; 16], usize, [u8; 16]) -> Result<(), ModelError>,
    ) -> Result<Option<std::sync::Arc<Vec<[u8; 16]>>>, ModelError> {
        use arrow_array::{Array, FixedSizeBinaryArray};
        let NativeFrontier { keys, charge } = frontier;
        let presence = edges.iter().any(|edge| edge.require_owner_presence);
        if !presence && !edges.iter().any(|edge| !edge.reverse) {
            return Ok(None);
        }
        // Forward declarations bind their source table. A source-kind frontier therefore has
        // exactly one physical provider; reverse members remain separate indexed demands.
        if edges
            .iter()
            .filter(|edge| !edge.reverse)
            .any(|edge| edge.table != source)
        {
            return Err(ModelError::Schema("native forward table binding"));
        }
        let selected = lctx_surrealdb::compiler_provider::select_table(
            &self.0.providers[source],
            keys.clone(),
            charge,
        )?
        .ok_or(ModelError::Schema("native nominal edge binding"))?;
        let schema = selected.schema();
        let _projection_charge = budget.reserve(
            "native-forward-projection",
            edges
                .len()
                .saturating_add(1)
                .saturating_mul(size_of::<usize>())
                .saturating_add(4096),
        )?;
        let mut projection = Vec::with_capacity(edges.len() + 1);
        projection.push(schema.index_of("id").map_err(ModelError::codec)?);
        for edge in edges.iter().filter(|edge| !edge.reverse) {
            projection.push(schema.index_of(edge.field).map_err(ModelError::codec)?);
        }
        projection.sort_unstable();
        projection.dedup();
        let state = self.0.session.state();
        let plan = selected
            .scan(&state, Some(&projection), &[], None)
            .await
            .map_err(ModelError::codec)?;
        let mut stream = plan
            .execute(0, self.0.session.task_ctx())
            .map_err(ModelError::codec)?;
        // The existing frontier charge covers this additional compact key vector, and follows
        // any reverse selected provider that borrows it. No rich owner rows are retained.
        let mut present = if presence {
            Vec::with_capacity(keys.len())
        } else {
            Vec::new()
        };
        while let Some(batch) = stream.try_next().await.map_err(ModelError::codec)? {
            check_cancellation(cancellation)?;
            let ids = batch
                .column_by_name("id")
                .and_then(|column| column.as_any().downcast_ref::<FixedSizeBinaryArray>())
                .ok_or(ModelError::Schema("native nominal source key"))?;
            if presence {
                for row in 0..batch.num_rows() {
                    if ids.is_null(row) {
                        return Err(ModelError::Schema("native nominal source key null"));
                    }
                    let id: [u8; 16] = ids
                        .value(row)
                        .try_into()
                        .map_err(|_| ModelError::Schema("native nominal source width"))?;
                    if keys.binary_search(&id).is_err() {
                        return Err(ModelError::Conflict("native nominal frontier mismatch"));
                    }
                    if present.last().is_some_and(|previous| previous >= &id) {
                        return Err(ModelError::Conflict("native nominal owner order"));
                    }
                    present.push(id);
                }
            }
            for edge in edges.iter().filter(|edge| !edge.reverse) {
                let fields = batch
                    .column_by_name(edge.field)
                    .and_then(|column| column.as_any().downcast_ref::<FixedSizeBinaryArray>())
                    .ok_or(ModelError::Schema("native nominal target key"))?;
                for row in 0..batch.num_rows() {
                    if fields.is_null(row) {
                        continue;
                    }
                    if ids.is_null(row) {
                        return Err(ModelError::Schema("native nominal source key null"));
                    }
                    let id: [u8; 16] = ids
                        .value(row)
                        .try_into()
                        .map_err(|_| ModelError::Schema("native nominal source width"))?;
                    let target: [u8; 16] = fields
                        .value(row)
                        .try_into()
                        .map_err(|_| ModelError::Schema("native nominal target width"))?;
                    if keys.binary_search(&id).is_err() {
                        return Err(ModelError::Conflict("native nominal frontier mismatch"));
                    }
                    accept(id, edge.target, target)?;
                }
            }
            tokio::task::yield_now().await;
        }
        Ok(presence.then(|| std::sync::Arc::new(present)))
    }
    /// Project one reverse membership over its indexed field frontier. The captured provider
    /// is the exact physical table epoch; target namespace remains owner-declared.
    async fn native_reverse_targets(
        &self,
        edge: NativeEdge,
        keys: std::sync::Arc<Vec<[u8; 16]>>,
        charge: std::sync::Arc<charged::StateCharge>,
        cancellation: Option<&crate::workspace::Cancellation>,
        mut accept: impl FnMut([u8; 16], [u8; 16]) -> Result<(), ModelError>,
    ) -> Result<(), ModelError> {
        use arrow_array::{Array, FixedSizeBinaryArray};
        let provider = &self.0.providers[edge.table];
        let selected = lctx_surrealdb::compiler_provider::select_field_table(
            provider,
            edge.field,
            keys.clone(),
            charge,
        )?
        .ok_or(ModelError::Schema("native nominal edge binding"))?;
        let schema = selected.schema();
        let projection = vec![
            schema.index_of("id").map_err(ModelError::codec)?,
            schema.index_of(edge.field).map_err(ModelError::codec)?,
        ];
        let state = self.0.session.state();
        let plan = selected
            .scan(&state, Some(&projection), &[], None)
            .await
            .map_err(ModelError::codec)?;
        let mut stream = plan
            .execute(0, self.0.session.task_ctx())
            .map_err(ModelError::codec)?;
        while let Some(batch) = stream.try_next().await.map_err(ModelError::codec)? {
            check_cancellation(cancellation)?;
            let ids = batch
                .column_by_name("id")
                .and_then(|column| column.as_any().downcast_ref::<FixedSizeBinaryArray>())
                .ok_or(ModelError::Schema("native nominal source key"))?;
            let fields = batch
                .column_by_name(edge.field)
                .and_then(|column| column.as_any().downcast_ref::<FixedSizeBinaryArray>())
                .ok_or(ModelError::Schema("native nominal target key"))?;
            for row in 0..batch.num_rows() {
                if fields.is_null(row) {
                    continue;
                }
                if ids.is_null(row) {
                    return Err(ModelError::Schema("native nominal source key null"));
                }
                let id: [u8; 16] = ids
                    .value(row)
                    .try_into()
                    .map_err(|_| ModelError::Schema("native nominal source width"))?;
                let field: [u8; 16] = fields
                    .value(row)
                    .try_into()
                    .map_err(|_| ModelError::Schema("native nominal target width"))?;
                let (source, target) = (field, id);
                if keys.binary_search(&source).is_err() {
                    return Err(ModelError::Conflict("native nominal frontier mismatch"));
                }
                accept(source, target)?;
            }
            tokio::task::yield_now().await;
        }
        Ok(())
    }
    /// Execute compatible explicit owner roots together. Physical absence is an outcome,
    /// while a virtual root is activated only by this explicit request. Supporting references
    /// reached during discovery never acquire root ownership. Input order defines partitions;
    /// duplicate requests retain separate partitions even when their physical demand is shared.
    pub async fn batch(
        &self,
        roots: &[PreparedRoot],
        budget: &ResourceBudget,
    ) -> Result<PreparedRootBatch, ModelError> {
        self.batch_inner(roots, budget, None).await
    }
    pub async fn batch_with_cancellation(
        &self,
        roots: &[PreparedRoot],
        budget: &ResourceBudget,
        cancellation: &crate::workspace::Cancellation,
    ) -> Result<PreparedRootBatch, ModelError> {
        self.batch_inner(roots, budget, Some(cancellation)).await
    }
    async fn batch_inner(
        &self,
        roots: &[PreparedRoot],
        budget: &ResourceBudget,
        cancellation: Option<&crate::workspace::Cancellation>,
    ) -> Result<PreparedRootBatch, ModelError> {
        check_cancellation(cancellation)?;
        let mut charge = charged::StateCharge::new(budget, "semantic-root-batch-memberships");
        let mut discovery_charge =
            charged::StateCharge::new(budget, "semantic-root-batch-discovery");
        charge.grow(roots.len().saturating_mul(size_of::<PreparedRootOutcome>()))?;
        let mut presence_charge = charged::StateCharge::new(budget, "semantic-root-batch-presence");
        let mut requested = charged::ChargedSet::default();
        for root in roots {
            check_cancellation(cancellation)?;
            if root.table >= self.0.tables.len() {
                return Err(ModelError::Invalid("closure table index absent".into()));
            }
            if root.kind == PreparedRootKind::Physical {
                requested.insert(&mut presence_charge, (root.table as i64, root.key))?;
            }
        }
        let mut present = charged::ChargedSet::default();
        let mut selected = requested.iter().peekable();
        // Root availability uses bounded, per-namespace point demand, not one query per root.
        while let Some(&(kind, _)) = selected.peek().copied() {
            check_cancellation(cancellation)?;
            let mut window_charge =
                charged::StateCharge::new(budget, "semantic-root-presence-window");
            window_charge.grow(
                resources::TRANSFER_ROWS
                    .saturating_mul(128)
                    .saturating_add(4096),
            )?;
            let mut literals = Vec::new();
            while literals.len() < resources::TRANSFER_ROWS
                && selected.peek().is_some_and(|key| key.0 == kind)
            {
                let (_, key) = selected.next().expect("selected root window");
                let hex = key
                    .iter()
                    .map(|byte| format!("{byte:02x}"))
                    .collect::<String>();
                literals.push(format!("X'{hex}'"));
            }
            let sql = format!(
                "SELECT CAST({kind} AS BIGINT) AS kind,id FROM {} WHERE id IN ({}) ORDER BY id",
                identifier(&self.0.tables[kind as usize].alias),
                literals.join(",")
            );
            let mut stream = crate::sql::query(&self.0.session, &sql)
                .await
                .map_err(ModelError::codec)?
                .execute_stream()
                .await
                .map_err(ModelError::codec)?;
            while let Some(batch) = stream.try_next().await.map_err(ModelError::codec)? {
                check_cancellation(cancellation)?;
                let _transfer = budget.reserve(
                    "semantic-root-presence-transfer",
                    batch.get_array_memory_size(),
                )?;
                for row in 0..batch.num_rows() {
                    let key = edge_key(&batch, "kind", "id", row)?;
                    if !requested.contains(&key) {
                        return Err(ModelError::Conflict("selected root presence mismatch"));
                    }
                    present.insert(&mut presence_charge, key)?;
                }
                tokio::task::yield_now().await;
            }
        }
        let mut outcomes = Vec::with_capacity(roots.len());
        let mut visited = charged::ChargedSet::default();
        let mut pending = charged::ChargedVec::default();
        for root in roots {
            check_cancellation(cancellation)?;
            let key = (root.table as i64, root.key);
            let outcome = if root.kind == PreparedRootKind::Virtual {
                PreparedRootOutcome::Virtual
            } else if present.contains(&key) {
                PreparedRootOutcome::Present
            } else {
                PreparedRootOutcome::Absent
            };
            outcomes.push(outcome);
            if outcome != PreparedRootOutcome::Absent
                && visited.insert(&mut discovery_charge, key)?
            {
                pending.push(&mut discovery_charge, key)?;
            }
        }
        drop(present);
        drop(requested);
        drop(presence_charge);
        let mut adjacency = charged::ChargedSet::default();
        let mut adjacency_charge =
            charged::StateCharge::new(budget, "semantic-root-batch-adjacency");
        self.discover(
            &mut visited,
            &mut pending,
            &mut discovery_charge,
            Some((&mut adjacency, &mut adjacency_charge)),
            budget,
            cancellation,
        )
        .await?;
        drop(pending);
        // Nominal adjacency is discovered once for the union. Logical ownership propagates
        // only through this compact immutable cache, including cycles and late overlap.
        let mut memberships = charged::ChargedSet::default();
        let mut pending = charged::ChargedVec::default();
        for (partition, root) in roots.iter().enumerate() {
            check_cancellation(cancellation)?;
            if outcomes[partition] == PreparedRootOutcome::Absent {
                continue;
            }
            let key = (root.table as i64, root.key);
            memberships.insert(&mut charge, (partition, key))?;
            pending.push(&mut charge, (partition, key))?;
        }
        let mut steps = 0usize;
        while let Some((partition, key)) = pending.take_last(&mut charge) {
            check_cancellation(cancellation)?;
            for (_, target) in
                adjacency.range((key, (i64::MIN, [0; 16]))..=(key, (i64::MAX, [255; 16])))
            {
                check_cancellation(cancellation)?;
                if memberships.insert(&mut charge, (partition, *target))? {
                    pending.push(&mut charge, (partition, *target))?;
                }
                steps += 1;
                if steps.is_multiple_of(resources::TRANSFER_ROWS) {
                    tokio::task::yield_now().await;
                }
            }
            steps += 1;
            if steps.is_multiple_of(resources::TRANSFER_ROWS) {
                tokio::task::yield_now().await;
            }
        }
        drop(pending);
        drop(adjacency);
        drop(adjacency_charge);
        let union = self.finish_keys(visited, budget)?;
        drop(discovery_charge);
        Ok(PreparedRootBatch {
            union,
            outcomes,
            memberships,
            _charge: charge,
        })
    }

    /// Compute compact keys for one explicitly selected grain. An unsupported root remains
    /// selected, and visited complete nominal keys terminate cycles before another edge read.
    pub async fn grain(
        &self,
        root: usize,
        predicate: &str,
        budget: &ResourceBudget,
    ) -> Result<PreparedClosure, ModelError> {
        self.grain_roots(&[(root, predicate.to_owned())], budget)
            .await
    }
    /// Select an explicit owner-declared union of typed roots. The prepared nominal edges and
    /// traversal are shared with single-root grains; no source/metadata key becomes a new root.
    pub async fn grain_roots(
        &self,
        roots: &[(usize, String)],
        budget: &ResourceBudget,
    ) -> Result<PreparedClosure, ModelError> {
        if roots.is_empty() {
            return Err(ModelError::Invalid("closure root domain absent".into()));
        }
        let mut seeds = Vec::new();
        for (root, predicate) in roots {
            let table = self
                .0
                .tables
                .get(*root)
                .ok_or_else(|| ModelError::Invalid("closure table index absent".into()))?;
            if predicate.trim().is_empty() {
                return Err(ModelError::Invalid("closure root predicate absent".into()));
            }
            seeds.push(format!(
                "SELECT CAST({root} AS BIGINT) AS kind,id FROM {} WHERE ({predicate})",
                identifier(&table.alias)
            ));
        }
        let sql = format!(
            "SELECT kind,id FROM ({}) actual_roots ORDER BY kind,id",
            seeds.join(" UNION ALL ")
        );
        let mut roots = crate::sql::query(&self.0.session, &sql)
            .await
            .map_err(ModelError::codec)?
            .execute_stream()
            .await
            .map_err(ModelError::codec)?;
        let mut visited = charged::ChargedSet::default();
        let mut pending = charged::ChargedVec::default();
        let mut traversal_charge =
            charged::StateCharge::new(budget, "semantic-grain-traversal-keys");
        while let Some(batch) = roots.try_next().await.map_err(ModelError::codec)? {
            let _transfer = budget.reserve(
                "semantic-grain-root-transfer",
                batch.get_array_memory_size(),
            )?;
            for row in 0..batch.num_rows() {
                let key = edge_key(&batch, "kind", "id", row)?;
                if visited.insert(&mut traversal_charge, key)? {
                    pending.push(&mut traversal_charge, key)?;
                }
            }
            tokio::task::yield_now().await;
        }
        drop(roots);
        self.discover(
            &mut visited,
            &mut pending,
            &mut traversal_charge,
            None,
            budget,
            None,
        )
        .await?;
        drop(pending);
        self.finish_keys(visited, budget)
    }

    /// One physical discovery loop, independent of record type and logical root count.
    async fn discover(
        &self,
        visited: &mut charged::ChargedSet<NominalKey>,
        pending: &mut charged::ChargedVec<NominalKey>,
        traversal_charge: &mut charged::StateCharge,
        mut adjacency: Option<(
            &mut charged::ChargedSet<(NominalKey, NominalKey)>,
            &mut charged::StateCharge,
        )>,
        budget: &ResourceBudget,
        cancellation: Option<&crate::workspace::Cancellation>,
    ) -> Result<(), ModelError> {
        let mut cursor = EdgeCursor::new(&self.0, budget)?;
        while !pending.is_empty() {
            check_cancellation(cancellation)?;
            let count = pending
                .len()
                .min(lctx_surrealdb::compiler_provider::REFERENCE_KEYS);
            let mut frontier_charge =
                charged::StateCharge::new(budget, "semantic-grain-native-frontier");
            frontier_charge.grow(count.saturating_mul(256).saturating_add(4096))?;
            let mut frontier = Vec::with_capacity(count);
            for _ in 0..count {
                frontier.push(
                    pending
                        .take_last(traversal_charge)
                        .expect("finite frontier count"),
                );
            }
            frontier.sort_unstable();
            for key in &frontier {
                cursor.targets(*key, budget, |target| {
                    check_cancellation(cancellation)?;
                    if let Some((arcs, charge)) = adjacency.as_mut() {
                        arcs.insert(charge, (*key, target))?;
                    }
                    if target.0 < 0 || target.0 as usize >= self.0.tables.len() {
                        return Err(ModelError::Schema("nominal edge target namespace absent"));
                    }
                    if visited.insert(traversal_charge, target)? {
                        pending.push(traversal_charge, target)?;
                    }
                    Ok(())
                })?;
            }
            let frontier_charge = std::sync::Arc::new(frontier_charge);
            let mut offset = 0;
            while offset < frontier.len() {
                let kind = frontier[offset].0;
                let end = offset + frontier[offset..].partition_point(|key| key.0 == kind);
                let keys = std::sync::Arc::new(
                    frontier[offset..end]
                        .iter()
                        .map(|(_, key)| *key)
                        .collect::<Vec<_>>(),
                );
                let first = self
                    .0
                    .native_edges
                    .partition_point(|edge| (edge.source as i64) < kind);
                let last = self
                    .0
                    .native_edges
                    .partition_point(|edge| (edge.source as i64) <= kind);
                let edges = &self.0.native_edges[first..last];
                let kind_source = kind;
                let present = self
                    .native_forward_targets(
                        usize::try_from(kind).map_err(ModelError::codec)?,
                        edges,
                        NativeFrontier {
                            keys: keys.clone(),
                            charge: frontier_charge.clone(),
                        },
                        budget,
                        cancellation,
                        |source, kind, key| {
                            check_cancellation(cancellation)?;
                            let target = (kind as i64, key);
                            if let Some((arcs, charge)) = adjacency.as_mut() {
                                arcs.insert(charge, ((kind_source, source), target))?;
                            }
                            if visited.insert(traversal_charge, target)? {
                                pending.push(traversal_charge, target)?;
                            }
                            Ok(())
                        },
                    )
                    .await?;
                for edge in edges.iter().filter(|edge| edge.reverse) {
                    let selected = if edge.require_owner_presence {
                        present
                            .clone()
                            .ok_or(ModelError::Schema("native owner presence absent"))?
                    } else {
                        keys.clone()
                    };
                    self.native_reverse_targets(
                        *edge,
                        selected,
                        frontier_charge.clone(),
                        cancellation,
                        |source, key| {
                            check_cancellation(cancellation)?;
                            let target = (edge.target as i64, key);
                            if let Some((arcs, charge)) = adjacency.as_mut() {
                                arcs.insert(charge, ((kind, source), target))?;
                            }
                            if visited.insert(traversal_charge, target)? {
                                pending.push(traversal_charge, target)?;
                            }
                            Ok(())
                        },
                    )
                    .await?;
                }
                offset = end;
            }
            tokio::task::yield_now().await;
        }
        drop(cursor);
        Ok(())
    }

    fn finish_keys(
        &self,
        visited: charged::ChargedSet<NominalKey>,
        budget: &ResourceBudget,
    ) -> Result<PreparedClosure, ModelError> {
        use arrow_array::{FixedSizeBinaryArray, Int64Array, RecordBatch};
        use arrow_schema::{DataType, Field, Schema};
        use datafusion::datasource::MemTable;
        use std::sync::Arc;
        let schema = Arc::new(Schema::new(vec![
            Field::new("kind", DataType::Int64, false),
            Field::new("id", DataType::FixedSizeBinary(16), false),
        ]));
        let mut keys = Vec::new();
        let mut charge = charged::StateCharge::new(budget, "semantic-grain-keys");
        charge.grow(self.0.tables.len().saturating_mul(
            size_of::<Vec<[u8; 16]>>() + size_of::<std::sync::Arc<Vec<[u8; 16]>>>() + 160,
        ))?;
        let mut native_keys = vec![Vec::new(); self.0.tables.len()];
        let mut selected = visited.iter();
        loop {
            let count = selected.len().min(resources::TRANSFER_ROWS);
            if count == 0 {
                break;
            }
            let mut transfer = budget.reserve(
                "semantic-grain-key-transfer",
                count.saturating_mul(256).saturating_add(4096),
            )?;
            let values = selected.by_ref().take(count).copied().collect::<Vec<_>>();
            for (kind, id) in &values {
                let selected = native_keys
                    .get_mut(usize::try_from(*kind).map_err(ModelError::codec)?)
                    .ok_or(ModelError::Schema("nominal selected key namespace"))?;
                if selected.len() == selected.capacity() {
                    let additional = selected.capacity().max(16);
                    charge.grow(additional.saturating_mul(size_of::<[u8; 16]>()))?;
                    selected.reserve_exact(additional);
                }
                selected.push(*id);
            }
            let kinds = Int64Array::from_iter_values(values.iter().map(|key| key.0));
            let ids = FixedSizeBinaryArray::try_from_iter(values.iter().map(|key| key.1))
                .map_err(ModelError::codec)?;
            let batch = RecordBatch::try_new(schema.clone(), vec![Arc::new(kinds), Arc::new(ids)])
                .map_err(ModelError::codec)?;
            let additional = if keys.len() == keys.capacity() {
                keys.capacity().max(4)
            } else {
                0
            };
            charge.grow(
                batch
                    .get_array_memory_size()
                    .saturating_add(additional.saturating_mul(size_of::<RecordBatch>())),
            )?;
            keys.reserve_exact(additional);
            keys.push(batch);
            transfer.try_resize(0)?;
        }
        drop(visited);
        let alias = scope_alias("keys");
        let provider = MemTable::try_new(schema, vec![keys]).map_err(ModelError::codec)?;
        self.0
            .session
            .register_table(alias.clone(), Arc::new(provider))
            .map_err(ModelError::codec)?;
        Ok(PreparedClosure {
            edges: self.0.clone(),
            alias,
            native_keys: native_keys.into_iter().map(std::sync::Arc::new).collect(),
            selected_aliases: std::sync::Mutex::new(BTreeMap::new()),
            _charge: std::sync::Arc::new(charge),
        })
    }
}
fn check_cancellation(
    cancellation: Option<&crate::workspace::Cancellation>,
) -> Result<(), ModelError> {
    if let Some(cancellation) = cancellation {
        cancellation.check()?;
    }
    Ok(())
}

/// Root presence is an explicit owner request, never inferred from a supporting reference.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PreparedRootKind {
    Physical,
    Virtual,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PreparedRoot {
    pub table: usize,
    pub key: [u8; 16],
    pub kind: PreparedRootKind,
}
pub use lctx_model::domain::scope_program::ScopeRootOutcome as PreparedRootOutcome;
/// The physical union and ordered nominal memberships retain distinct logical partitions.
/// No rich records are cached. Membership reservations live with the returned batch.
pub struct PreparedRootBatch {
    pub union: PreparedClosure,
    outcomes: Vec<PreparedRootOutcome>,
    memberships: charged::ChargedSet<(usize, NominalKey)>,
    _charge: charged::StateCharge,
}
impl PreparedRootBatch {
    pub fn outcomes(&self) -> &[PreparedRootOutcome] {
        &self.outcomes
    }
    pub fn relation(&self, table: usize) -> Result<&Relation, ModelError> {
        self.union
            .edges
            .tables
            .get(table)
            .map(|binding| &binding.relation)
            .ok_or_else(|| ModelError::Invalid("closure table index absent".into()))
    }
    /// Materialize one exact existing partition; discovery and logical root ownership are
    /// already complete. Nominal dangling keys remain present and physical hydration joins them.
    pub(crate) fn partition_scope(
        &self,
        partition: usize,
        budget: &ResourceBudget,
    ) -> Result<PreparedClosure, ModelError> {
        let mut charge = charged::StateCharge::new(budget, "scope-partition-key-copy");
        let mut keys = charged::ChargedSet::default();
        for (table, key) in self.memberships(partition)? {
            keys.insert(&mut charge, (table as i64, key))?;
        }
        PreparedEdges(self.union.edges.clone()).finish_keys(keys, budget)
    }
    /// Exact membership lookup over the charged ordered partition index.
    pub fn contains(
        &self,
        partition: usize,
        table: usize,
        key: [u8; 16],
    ) -> Result<bool, ModelError> {
        self.relation(table)?;
        if partition >= self.outcomes.len() {
            return Err(ModelError::Invalid("closure root partition absent".into()));
        }
        Ok(self.memberships.contains(&(partition, (table as i64, key))))
    }
    /// Ordered namespace/key memberships for a selected partition, including dangling forward
    /// references. A physical absent root has an explicit outcome and an empty membership set.
    pub fn memberships(
        &self,
        partition: usize,
    ) -> Result<impl Iterator<Item = (usize, [u8; 16])> + '_, ModelError> {
        if partition >= self.outcomes.len() {
            return Err(ModelError::Invalid("closure root partition absent".into()));
        }
        Ok(self
            .memberships
            .range((partition, (i64::MIN, [0; 16]))..=(partition, (i64::MAX, [255; 16])))
            .map(|(_, (kind, key))| (*kind as usize, *key)))
    }
    /// Sorted, deduplicated nominal keys for a model-owned RowsView; roots themselves remain
    /// separate even when their reachable keys coincide.
    pub fn keys(
        &self,
        partition: usize,
        table: usize,
    ) -> Result<impl Iterator<Item = [u8; 16]> + '_, ModelError> {
        if table >= self.union.edges.tables.len() {
            return Err(ModelError::Invalid("closure table index absent".into()));
        }
        if partition >= self.outcomes.len() {
            return Err(ModelError::Invalid("closure root partition absent".into()));
        }
        Ok(self
            .memberships
            .range((partition, (table as i64, [0; 16]))..=(partition, (table as i64, [255; 16])))
            .map(|(_, (_, key))| *key))
    }
}

/// A short-lived compact nominal key set for one owner grain. It keeps the external edge source
/// alive; dropping the scope removes its temporary key table without rewriting semantic inputs.
pub struct PreparedClosure {
    edges: std::sync::Arc<EdgeSource>,
    alias: String,
    native_keys: Vec<std::sync::Arc<Vec<[u8; 16]>>>,
    selected_aliases: std::sync::Mutex<BTreeMap<usize, String>>,
    _charge: std::sync::Arc<charged::StateCharge>,
}
impl PreparedClosure {
    pub fn session(&self) -> &SessionContext {
        &self.edges.session
    }
    pub fn select(&self, table: usize) -> Result<String, ModelError> {
        let table_binding = self
            .edges
            .tables
            .get(table)
            .ok_or_else(|| ModelError::Invalid("closure table index absent".into()))?;
        let mut aliases = self
            .selected_aliases
            .lock()
            .map_err(|_| ModelError::Conflict("native scope alias ownership"))?;
        if let Some(alias) = aliases.get(&table) {
            return Ok(format!("SELECT * FROM {}", identifier(alias)));
        }
        if let Some(provider) = lctx_surrealdb::compiler_provider::select_table(
            &self.edges.providers[table],
            self.native_keys[table].clone(),
            self._charge.clone(),
        )? {
            let alias = scope_alias("selected");
            self.edges
                .session
                .register_table(alias.clone(), provider)
                .map_err(ModelError::codec)?;
            aliases.insert(table, alias.clone());
            return Ok(format!("SELECT * FROM {}", identifier(&alias)));
        }
        Ok(format!(
            "SELECT source.* FROM {} AS keys JOIN {} AS source ON keys.kind={table} AND source.id=keys.id",
            identifier(&self.alias),
            identifier(&table_binding.alias)
        ))
    }
}
impl Drop for PreparedClosure {
    fn drop(&mut self) {
        let _ = self.edges.session.deregister_table(self.alias.as_str());
        if let Ok(aliases) = self.selected_aliases.lock() {
            for alias in aliases.values() {
                let _ = self.edges.session.deregister_table(alias.as_str());
            }
        }
    }
}

#[cfg(test)]
mod nominal_closure_controls {
    use super::*;
    use datafusion::datasource::MemTable;
    use input::{Package, Release};
    use std::sync::Arc;

    fn register<R: Record>(session: &SessionContext, name: &str, rows: &[R]) {
        let batch = R::encode(rows).unwrap();
        session
            .register_table(
                name,
                Arc::new(MemTable::try_new(batch.schema(), vec![vec![batch]]).unwrap()),
            )
            .unwrap();
    }
    fn id_predicate<R: Record>(id: Id<R>) -> String {
        let bytes = id
            .bytes()
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>();
        format!("id=X'{bytes}'")
    }
    async fn decode<R: Record>(scope: &PreparedClosure, table: usize) -> Vec<R> {
        let mut stream = crate::sql::query(scope.session(), &scope.select(table).unwrap())
            .await
            .unwrap()
            .execute_stream()
            .await
            .unwrap();
        let mut rows = Vec::new();
        while let Some(batch) = stream.try_next().await.unwrap() {
            rows.extend(R::decode(&batch).unwrap());
        }
        rows
    }
    fn fixture() -> (SessionContext, Vec<ClosureTable>, Vec<Release>, Package) {
        let session = SessionContext::new();
        let selected = Package {
            name: "selected".into(),
        };
        let unrelated = Package {
            name: "unrelated".into(),
        };
        let releases = vec![
            Release {
                package: selected.id(),
                version: "1".into(),
            },
            Release {
                package: selected.id(),
                version: "2".into(),
            },
            Release {
                package: unrelated.id(),
                version: "3".into(),
            },
        ];
        register(
            &session,
            "packages_at_attempt",
            &[selected.clone(), unrelated],
        );
        register(&session, "releases_at_attempt", &releases);
        let tables = vec![
            ClosureTable {
                relation: Relation::of::<Release>(),
                alias: "releases_at_attempt".into(),
            },
            ClosureTable {
                relation: Relation::of::<Package>(),
                alias: "packages_at_attempt".into(),
            },
        ];
        (session, tables, releases, selected)
    }
    #[tokio::test]
    async fn batched_roots_keep_overlap_duplicate_partitions_and_absence_explicit() {
        let (session, tables, releases, package) = fixture();
        let mut plan = NominalClosure::new(tables).unwrap();
        plan.follow(0, "package", 1).unwrap();
        let budget = ResourceBudget::fixed(64 << 20).unwrap();
        let edges = plan.prepare(&session, &budget).await.unwrap();
        let prepared_charge = budget.reserved();
        let physical = |table, key| PreparedRoot {
            table,
            key,
            kind: PreparedRootKind::Physical,
        };
        let missing = [254; 16];
        let batch = edges
            .batch(
                &[
                    physical(0, *releases[0].id().bytes()),
                    physical(0, *releases[1].id().bytes()),
                    physical(0, *releases[0].id().bytes()),
                    physical(0, missing),
                    PreparedRoot {
                        table: 0,
                        key: missing,
                        kind: PreparedRootKind::Virtual,
                    },
                    PreparedRoot {
                        table: 1,
                        key: *releases[0].id().bytes(),
                        kind: PreparedRootKind::Virtual,
                    },
                ],
                &budget,
            )
            .await
            .unwrap();
        assert_eq!(
            batch.outcomes(),
            &[
                PreparedRootOutcome::Present,
                PreparedRootOutcome::Present,
                PreparedRootOutcome::Present,
                PreparedRootOutcome::Absent,
                PreparedRootOutcome::Virtual,
                PreparedRootOutcome::Virtual
            ]
        );
        for partition in [0, 1, 2] {
            assert_eq!(
                batch.keys(partition, 1).unwrap().collect::<Vec<_>>(),
                vec![*package.id().bytes()]
            );
            assert_eq!(
                batch.keys(partition, 0).unwrap().collect::<Vec<_>>(),
                vec![*releases[usize::from(partition == 1)].id().bytes()]
            );
        }
        assert!(batch.memberships(3).unwrap().next().is_none());
        assert_eq!(
            batch.memberships(4).unwrap().collect::<Vec<_>>(),
            vec![(0, missing)]
        );
        assert!(batch.keys(0, 2).is_err());
        assert_eq!(
            batch.memberships(5).unwrap().collect::<Vec<_>>(),
            vec![(1, *releases[0].id().bytes())]
        );
        assert!(batch.memberships(6).is_err());
        assert_eq!(decode::<Release>(&batch.union, 0).await.len(), 2);
        assert_eq!(decode::<Package>(&batch.union, 1).await.len(), 1);
        assert!(budget.reserved() > prepared_charge);
        drop(batch);
        assert_eq!(
            budget.reserved(),
            prepared_charge,
            "batch releases compact memberships and union"
        );
        let empty = edges.batch(&[], &budget).await.unwrap();
        assert!(empty.outcomes().is_empty());
        assert!(decode::<Release>(&empty.union, 0).await.is_empty());
        assert!(edges.batch(&[physical(2, missing)], &budget).await.is_err());
        let cancellation = crate::workspace::Cancellation::default();
        cancellation.cancel();
        assert!(
            edges
                .batch_with_cancellation(
                    &[physical(0, *releases[0].id().bytes())],
                    &budget,
                    &cancellation
                )
                .await
                .is_err()
        );
    }

    #[tokio::test]
    async fn batched_root_partitions_propagate_late_overlap_through_cycles() {
        let (session, tables, releases, package) = fixture();
        let mut plan = NominalClosure::new(tables).unwrap();
        plan.own_existing(0, "package", 1).unwrap();
        let budget = ResourceBudget::fixed(64 << 20).unwrap();
        let edges = plan.prepare(&session, &budget).await.unwrap();
        let batch = edges
            .batch(
                &[
                    PreparedRoot {
                        table: 0,
                        key: *releases[0].id().bytes(),
                        kind: PreparedRootKind::Physical,
                    },
                    PreparedRoot {
                        table: 1,
                        key: *package.id().bytes(),
                        kind: PreparedRootKind::Physical,
                    },
                    PreparedRoot {
                        table: 0,
                        key: *releases[2].id().bytes(),
                        kind: PreparedRootKind::Physical,
                    },
                ],
                &budget,
            )
            .await
            .unwrap();
        let mut expected = vec![
            (0, *releases[0].id().bytes()),
            (0, *releases[1].id().bytes()),
            (1, *package.id().bytes()),
        ];
        expected.sort_unstable();
        assert_eq!(batch.memberships(0).unwrap().collect::<Vec<_>>(), expected);
        assert_eq!(batch.memberships(1).unwrap().collect::<Vec<_>>(), expected);
        assert_eq!(
            batch.keys(2, 0).unwrap().collect::<Vec<_>>(),
            vec![*releases[2].id().bytes()]
        );
        let scalar = edges
            .grain(0, &id_predicate(releases[0].id()), &budget)
            .await
            .unwrap();
        assert_eq!(
            decode::<Release>(&scalar, 0).await.len(),
            batch.keys(0, 0).unwrap().count()
        );
    }

    #[tokio::test]
    async fn batched_virtual_owner_preserves_strict_and_advertised_reverse_distinction() {
        let (session, tables, releases, _) = fixture();
        // The package relation is empty; dangling member references do not establish presence.
        session.deregister_table("packages_at_attempt").unwrap();
        register::<Package>(&session, "packages_at_attempt", &[]);
        let budget = ResourceBudget::fixed(64 << 20).unwrap();
        for strict in [false, true] {
            let mut plan = NominalClosure::new(tables.clone()).unwrap();
            if strict {
                plan.own_existing(0, "package", 1).unwrap();
            } else {
                plan.own(0, "package", 1).unwrap();
            }
            let edges = plan.prepare(&session, &budget).await.unwrap();
            let key = *releases[0].package.bytes();
            let batch = edges
                .batch(
                    &[
                        PreparedRoot {
                            table: 1,
                            key,
                            kind: PreparedRootKind::Physical,
                        },
                        PreparedRoot {
                            table: 1,
                            key,
                            kind: PreparedRootKind::Virtual,
                        },
                        PreparedRoot {
                            table: 0,
                            key: *releases[0].id().bytes(),
                            kind: PreparedRootKind::Physical,
                        },
                    ],
                    &budget,
                )
                .await
                .unwrap();
            assert_eq!(
                batch.outcomes(),
                &[
                    PreparedRootOutcome::Absent,
                    PreparedRootOutcome::Virtual,
                    PreparedRootOutcome::Present
                ]
            );
            assert_eq!(
                batch.keys(1, 0).unwrap().count(),
                if strict { 0 } else { 2 }
            );
            assert_eq!(
                batch.keys(2, 0).unwrap().count(),
                if strict { 1 } else { 2 }
            );
            assert_eq!(
                batch.keys(2, 1).unwrap().collect::<Vec<_>>(),
                vec![key],
                "forward references retain absent keys"
            );
        }
    }

    #[tokio::test]
    async fn forward_closure_keeps_root_and_excludes_shared_target_neighbors() {
        let (session, tables, releases, package) = fixture();
        let mut plan = NominalClosure::new(tables).unwrap();
        plan.follow(0, "package", 1).unwrap();
        let budget = ResourceBudget::fixed(1 << 20).unwrap();
        let edges = plan.prepare(&session, &budget).await.unwrap();
        let index_usage = budget.reserved();
        assert!(index_usage > 0);
        let scope = edges
            .grain(0, &id_predicate(releases[0].id()), &budget)
            .await
            .unwrap();
        assert_eq!(
            decode::<Release>(&scope, 0).await,
            vec![releases[0].clone()]
        );
        assert_eq!(decode::<Package>(&scope, 1).await, vec![package]);
        drop(scope);
        assert_eq!(budget.reserved(), index_usage);
        drop(edges);
        assert_eq!(budget.reserved(), 0);
    }
    #[tokio::test]
    async fn explicit_reverse_membership_closes_cycle_without_unrelated_owners() {
        let (session, tables, releases, package) = fixture();
        let mut plan = NominalClosure::new(tables).unwrap();
        plan.follow(0, "package", 1).unwrap();
        plan.own(0, "package", 1).unwrap();
        plan.own(0, "package", 1).unwrap();
        let declared = plan.pairs.iter().next().unwrap().clone();
        plan.pairs(declared.0, declared.1, declared.2).unwrap();
        assert_eq!(
            plan.pairs.len(),
            2,
            "one exact forward query and its inverse ownership query"
        );
        let budget = ResourceBudget::fixed(1 << 20).unwrap();
        let edges = plan.prepare(&session, &budget).await.unwrap();
        let scope = edges
            .grain(1, &id_predicate(package.id()), &budget)
            .await
            .unwrap();
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
        let scope = edges
            .grain(0, &id_predicate(releases[0].id()), &budget)
            .await
            .unwrap();
        assert_eq!(
            decode::<Release>(&scope, 0).await,
            vec![releases[0].clone()]
        );
        assert!(decode::<Package>(&scope, 1).await.is_empty());
    }
    #[tokio::test]
    async fn sparse_batch_ranges_include_all_split_sources_and_exclude_unrelated_edges() {
        let session = SessionContext::new_with_config(
            datafusion::prelude::SessionConfig::new().with_batch_size(16),
        );
        let mut packages = (0..1000)
            .map(|index| Package {
                name: format!("owner-{index}"),
            })
            .collect::<Vec<_>>();
        packages.sort_by_key(Record::id);
        // A high-degree selected owner crosses storage batches, with many unrelated
        // full batches on both sides. Other roots remain small in the same window.
        let selected = packages[228].clone();
        let releases = packages
            .iter()
            .flat_map(|package| {
                (0..if package.id() == selected.id() {
                    4096
                } else {
                    80
                })
                    .map(move |index| Release {
                        package: package.id(),
                        version: index.to_string(),
                    })
            })
            .collect::<Vec<_>>();
        let expected = releases
            .iter()
            .filter(|row| row.package == selected.id())
            .cloned()
            .collect::<Vec<_>>();
        register(&session, "many_packages", &packages);
        register(&session, "many_releases", &releases);
        let mut plan = NominalClosure::new(vec![
            ClosureTable {
                relation: Relation::of::<Release>(),
                alias: "many_releases".into(),
            },
            ClosureTable {
                relation: Relation::of::<Package>(),
                alias: "many_packages".into(),
            },
        ])
        .unwrap();
        plan.own(0, "package", 1).unwrap();
        let budget = ResourceBudget::fixed(16 << 20).unwrap();
        let edges = plan.prepare(&session, &budget).await.unwrap();
        let index_usage = budget.reserved();
        assert!(
            edges.0.batches.len() > 32,
            "fixture requires many unrelated IPC batches"
        );
        let source = (1, *selected.id().bytes());
        let range = edges.0.matching_batches(source);
        assert!(
            range.len() > 1,
            "one source must cross record-batch boundaries"
        );
        assert!(range.len() < edges.0.batches.len());
        assert!(range.start > 0 && range.end < edges.0.batches.len());
        // The direct cursor exercise proves every duplicate source range contributes its keys.
        let mut cursor = EdgeCursor::new(&edges.0, &budget).unwrap();
        let mut targets = std::collections::BTreeSet::new();
        cursor
            .targets(source, &budget, |key| {
                targets.insert(key);
                Ok(())
            })
            .unwrap();
        assert_eq!(
            targets,
            expected
                .iter()
                .map(|row| (0, *row.id().bytes()))
                .collect::<std::collections::BTreeSet<_>>()
        );
        assert_eq!(cursor.current.as_ref().unwrap().0, range.end - 1);
        drop(cursor);
        assert_eq!(budget.reserved(), index_usage);
        let scope = edges
            .grain(1, &id_predicate(selected.id()), &budget)
            .await
            .unwrap();
        let found = decode::<Release>(&scope, 0).await;
        assert_eq!(found.len(), expected.len());
        assert!(expected.iter().all(|row| found.contains(row)));
        assert_eq!(decode::<Package>(&scope, 1).await, vec![selected.clone()]);
        drop(scope);
        assert_eq!(budget.reserved(), index_usage);
        let roots = [selected.id(), packages[229].id(), selected.id()];
        let requested = roots.map(|id| PreparedRoot {
            table: 1,
            key: *id.bytes(),
            kind: PreparedRootKind::Physical,
        });
        let batch = edges.batch(&requested, &budget).await.unwrap();
        let forward = batch.keys(0, 0).unwrap().collect::<Vec<_>>();
        assert_eq!(forward, batch.keys(2, 0).unwrap().collect::<Vec<_>>());
        assert_eq!(forward.len(), 4096);
        assert_eq!(batch.keys(1, 0).unwrap().count(), 80);
        let reordered = edges
            .batch(&[requested[1], requested[0]], &budget)
            .await
            .unwrap();
        assert_eq!(reordered.keys(1, 0).unwrap().collect::<Vec<_>>(), forward);
        assert_eq!(
            reordered.keys(0, 0).unwrap().collect::<Vec<_>>(),
            batch.keys(1, 0).unwrap().collect::<Vec<_>>()
        );
        drop(reordered);
        drop(batch);
        assert_eq!(budget.reserved(), index_usage);
        // Empty root selection stays empty and does not fabricate a source key.
        let empty = edges.grain(1, "false", &budget).await.unwrap();
        assert!(decode::<Release>(&empty, 0).await.is_empty());
        assert!(decode::<Package>(&empty, 1).await.is_empty());
        drop(empty);
        drop(edges);
        assert_eq!(budget.reserved(), 0);
    }
    #[tokio::test]
    async fn identical_ids_in_distinct_virtual_namespaces_remain_distinct_keys() {
        let (session, _, releases, package) = fixture();
        register(&session, "releases_in_other_epoch", &releases);
        let mut plan = NominalClosure::new(vec![
            ClosureTable {
                relation: Relation::of::<Release>(),
                alias: "releases_at_attempt".into(),
            },
            ClosureTable {
                relation: Relation::of::<Release>(),
                alias: "releases_in_other_epoch".into(),
            },
            ClosureTable {
                relation: Relation::of::<Package>(),
                alias: "packages_at_attempt".into(),
            },
        ])
        .unwrap();
        plan.pairs(
            0,
            1,
            "SELECT id AS source_id,id AS target_id FROM releases_at_attempt".into(),
        )
        .unwrap();
        plan.follow(1, "package", 2).unwrap();
        let budget = ResourceBudget::fixed(1 << 20).unwrap();
        let edges = plan.prepare(&session, &budget).await.unwrap();
        let scope = edges
            .grain(0, &id_predicate(releases[0].id()), &budget)
            .await
            .unwrap();
        assert_eq!(
            decode::<Release>(&scope, 0).await,
            vec![releases[0].clone()]
        );
        assert_eq!(
            decode::<Release>(&scope, 1).await,
            vec![releases[0].clone()]
        );
        assert_eq!(decode::<Package>(&scope, 2).await, vec![package]);
        drop(edges);
        assert!(
            budget.reserved() > 0,
            "closure retains the charged prepared index"
        );
        drop(scope);
        assert_eq!(budget.reserved(), 0);
    }

    #[tokio::test]
    async fn many_declared_pairs_prepare_without_a_left_deep_union_and_preserve_keys() {
        let (session, tables, releases, package) = fixture();
        let mut plan = NominalClosure::new(tables).unwrap();
        // Repeating one follow demand deduplicates its preparation and preserves selected rows.
        for _ in 0..257 {
            plan.follow(0, "package", 1).unwrap();
        }
        assert_eq!(plan.pairs.len(), 1, "identical follow demands are a set");
        let budget = ResourceBudget::fixed(16 << 20).unwrap();
        let edges = plan.prepare(&session, &budget).await.unwrap();
        assert_eq!(
            edges
                .0
                .batches
                .iter()
                .map(|batch| batch.rows)
                .sum::<usize>(),
            releases.len()
        );
        let scope = edges
            .grain(0, &id_predicate(releases[0].id()), &budget)
            .await
            .unwrap();
        assert_eq!(
            decode::<Release>(&scope, 0).await,
            vec![releases[0].clone()]
        );
        assert_eq!(decode::<Package>(&scope, 1).await, vec![package]);
        drop(scope);
        drop(edges);
        assert_eq!(budget.reserved(), 0);
    }

    #[tokio::test]
    async fn ipc_batch_allowance_accounts_for_shared_block_metadata_and_alignment() {
        let (session, tables, _, _) = fixture();
        let mut plan = NominalClosure::new(tables).unwrap();
        plan.follow(0, "package", 1).unwrap();
        let budget = ResourceBudget::fixed(1 << 20).unwrap();
        let edges = plan.prepare(&session, &budget).await.unwrap();
        let index_usage = budget.reserved();
        let descriptor = &edges.0.batches[0];
        let mut cursor = EdgeCursor::new(&edges.0, &budget).unwrap();
        let batch = cursor.batch(0, &budget).unwrap();
        validate_edge_batch(batch).unwrap();
        assert_eq!(logical_batch_bytes(batch).unwrap(), descriptor.rows * 48);
        assert!(
            descriptor.ipc_bytes > logical_batch_bytes(batch).unwrap(),
            "the IPC block includes metadata and alignment"
        );
        assert!(
            batch.get_array_memory_size() > descriptor.ipc_bytes,
            "columns report the same retained block capacity separately"
        );
        assert_eq!(
            budget.reserved(),
            index_usage + edges.0.reader_allowance + descriptor.ipc_bytes + 4096
        );
        let usage = budget.reserved();
        cursor.batch(0, &budget).unwrap();
        assert_eq!(budget.reserved(), usage);
        drop(cursor);
        assert_eq!(budget.reserved(), index_usage);
        drop(edges);
        assert_eq!(budget.reserved(), 0);
    }

    #[tokio::test]
    async fn many_pair_streams_share_one_external_sort_with_the_workspace_eight_mib_pool() {
        use crate::workspace::{Workspace, WorkspaceOptions};
        use lctx_model::domain::stages::Profile;
        let workspace = Workspace::new(
            Arc::new(model().unwrap()),
            WorkspaceOptions {
                memory_bytes: 8 << 20,
                partitions: 1,
                batch_rows: 16,
            },
            crate::test_native::store(),
        )
        .unwrap();
        let session = workspace
            .inputs("edge-pool-control", Profile::Catalog, std::iter::empty())
            .unwrap()
            .session(&workspace)
            .await
            .unwrap();
        let package = Package {
            name: "bounded-owner".into(),
        };
        const BINDINGS: usize = 257;
        const ROWS: usize = 400;
        register(&session, "bounded_packages", std::slice::from_ref(&package));
        let mut tables = Vec::new();
        let mut expected_sources = Vec::new();
        let mut selected = None;
        for binding in 0..BINDINGS {
            let releases = (0..ROWS)
                .map(|index| Release {
                    package: package.id(),
                    version: format!("{binding}-{index}"),
                })
                .collect::<Vec<_>>();
            if binding == 0 {
                selected = Some(releases[0].clone());
            }
            let alias = format!("bounded_releases_{binding}");
            register(&session, &alias, &releases);
            let mut ids = releases
                .iter()
                .map(|row| *row.id().bytes())
                .collect::<Vec<_>>();
            ids.sort_unstable();
            expected_sources.push(ids);
            tables.push(ClosureTable {
                relation: Relation::of::<Release>(),
                alias,
            });
        }
        tables.push(ClosureTable {
            relation: Relation::of::<Package>(),
            alias: "bounded_packages".into(),
        });
        let selected = selected.unwrap();
        let mut plan = NominalClosure::new(tables).unwrap();
        for binding in 0..BINDINGS {
            plan.follow(binding, "package", BINDINGS).unwrap();
        }
        let edges = plan.prepare(&session, workspace.budget()).await.unwrap();
        let edge_rows = ROWS * BINDINGS;
        assert_eq!(
            edges
                .0
                .batches
                .iter()
                .map(|batch| batch.rows)
                .sum::<usize>(),
            edge_rows,
            "all distinct declared edges survive staging and ordering"
        );
        assert_eq!(
            edges.0.batches.len(),
            edge_rows.div_ceil(resources::TRANSFER_ROWS),
            "index has one entry per full storage batch, independent of upstream tiny fragments"
        );
        assert!(
            edges.0.batches[..edges.0.batches.len() - 1]
                .iter()
                .all(|batch| batch.rows == resources::TRANSFER_ROWS)
        );
        let index_usage = workspace.budget().reserved();
        let mut cursor = EdgeCursor::new(&edges.0, workspace.budget()).unwrap();
        let mut seen = 0;
        for index in 0..edges.0.batches.len() {
            let batch = cursor.batch(index, workspace.budget()).unwrap();
            for row in 0..batch.num_rows() {
                let binding = seen / ROWS;
                assert_eq!(
                    edge_key(batch, "source_kind", "source_id", row).unwrap(),
                    (binding as i64, expected_sources[binding][seen % ROWS])
                );
                assert_eq!(
                    edge_key(batch, "target_kind", "target_id", row).unwrap(),
                    (BINDINGS as i64, *package.id().bytes())
                );
                seen += 1;
            }
        }
        assert_eq!(
            seen, edge_rows,
            "complete edge content is checked in exact sort order"
        );
        drop(cursor);
        assert_eq!(workspace.budget().reserved(), index_usage);
        let scope = edges
            .grain(0, &id_predicate(selected.id()), workspace.budget())
            .await
            .unwrap();
        assert_eq!(decode::<Release>(&scope, 0).await, vec![selected]);
        assert_eq!(decode::<Package>(&scope, BINDINGS).await, vec![package]);
        drop(scope);
        assert_eq!(workspace.budget().reserved(), index_usage);
        drop(edges);
        assert_eq!(workspace.budget().reserved(), 0);
    }
}
