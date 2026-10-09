//! E0 selects actual C1 roots and canonical brief documents before decoding their rich premises.
use crate::producer_operations;
use crate::{
    consumed_rows::{ClosureTable, PreparedClosure, PreparedEdges, identifier},
    workspace::{CompletedInputs, ProducerOutput, Workspace},
};
use datafusion::prelude::SessionContext;
use futures::future::BoxFuture;
use lctx_model::domain::{
    catalog::evidence as c1,
    retrieval::build::{self, Data, Output},
    *,
};
use std::{any::TypeId, sync::Arc};
macro_rules! decoder_inputs {
    ($apply:ident) => {
        lctx_model::catalog_inputs!($apply);
        lctx_model::catalog_outputs!($apply);
        lctx_model::catalog_evidence_inputs!($apply);
        lctx_model::catalog_evidence_outputs!($apply);
        lctx_model::retrieval_inputs!($apply);
        lctx_model::retrieval_synthesis_inputs!($apply);
        lctx_model::catalog_runtime_inputs!($apply);
        lctx_model::expected_domain_inputs!($apply);
    };
}
fn typed<R: Record>(inputs: &[ValidationInput]) -> Result<usize, ModelError> {
    inputs
        .iter()
        .position(|input| input.type_id() == TypeId::of::<R>())
        .ok_or(ModelError::Schema("E0 scope relation absent"))
}

struct ArtifactPrograms {
    owners: [Arc<scope_program::CompiledScopeProgram>; 7],
    _charge: Box<dyn resources::Reservation>,
}
impl ArtifactPrograms {
    fn index(kind: catalog_scope_program::RetrievalArtifactRoot) -> usize {
        use catalog_scope_program::RetrievalArtifactRoot as R;
        match kind {
            R::Member => 0,
            R::Document => 1,
            R::Scenario => 2,
            R::Deployment => 3,
            R::Source => 4,
            R::Empty => 5,
            R::Brief => 6,
        }
    }
    fn new(
        inputs: &[ValidationInput],
        tables: &[ClosureTable],
        model: &ValidatedModel,
        budget: &resources::ResourceBudget,
        interner: Option<&std::sync::Mutex<scope_program::ScopeInterner>>,
    ) -> Result<Self, ModelError> {
        let charge = budget.reserve(
            "retrieval-artifact-program-owners",
            7 * size_of::<Arc<scope_program::CompiledScopeProgram>>(),
        )?;
        let _construction = budget.reserve(
            "retrieval-artifact-program-construction",
            tables
                .len()
                .saturating_mul(1024)
                .saturating_add(7 * size_of::<Arc<scope_program::CompiledScopeProgram>>())
                .saturating_add(4096),
        )?;
        let relations = tables
            .iter()
            .map(|table| table.relation.clone())
            .collect::<Vec<_>>();
        use catalog_scope_program::RetrievalArtifactRoot as R;
        let mut owners = Vec::with_capacity(7);
        for kind in [
            R::Member,
            R::Document,
            R::Scenario,
            R::Deployment,
            R::Source,
            R::Empty,
            R::Brief,
        ] {
            let semantic = catalog_scope_program::retrieval_artifacts(
                inputs.to_vec(),
                &relations,
                kind,
                budget,
            )?;
            owners.push(crate::scope_compilation::compile(
                semantic.program(),
                model,
                budget,
                interner,
            )?);
        }
        Ok(Self {
            owners: owners
                .try_into()
                .map_err(|_| ModelError::Schema("E0 artifact program inventory"))?,
            _charge: charge,
        })
    }
    fn get(
        &self,
        kind: catalog_scope_program::RetrievalArtifactRoot,
    ) -> &Arc<scope_program::CompiledScopeProgram> {
        &self.owners[Self::index(kind)]
    }
}
struct InventoryPrograms {
    owners: [Arc<scope_program::CompiledScopeProgram>; 2],
    _charge: Box<dyn resources::Reservation>,
}
impl InventoryPrograms {
    fn new(
        inputs: &[ValidationInput],
        tables: &[ClosureTable],
        model: &ValidatedModel,
        budget: &resources::ResourceBudget,
        interner: Option<&std::sync::Mutex<scope_program::ScopeInterner>>,
    ) -> Result<Self, ModelError> {
        let charge = budget.reserve(
            "retrieval-inventory-program-owners",
            2 * size_of::<Arc<scope_program::CompiledScopeProgram>>(),
        )?;
        let _construction = budget.reserve(
            "retrieval-inventory-construction",
            tables.len() * 2048 + 4096,
        )?;
        let relations = tables
            .iter()
            .map(|t| t.relation.clone())
            .collect::<Vec<_>>();
        let compile = |kind| {
            let program = catalog_scope_program::retrieval_inventory(
                inputs.to_vec(),
                &relations,
                kind,
                budget,
            )?;
            crate::scope_compilation::compile(program.program(), model, budget, interner)
        };
        Ok(Self {
            owners: [
                compile(catalog_scope_program::RetrievalInventory::Roots)?,
                compile(catalog_scope_program::RetrievalInventory::Briefs)?,
            ],
            _charge: charge,
        })
    }
}
struct ArtifactSelection {
    sql: String,
    _charge: Box<dyn resources::Reservation>,
}
impl std::ops::Deref for ArtifactSelection {
    type Target = str;
    fn deref(&self) -> &str {
        &self.sql
    }
}

/// Only this metadata is shared between rendering grains. Originals and output text are released
/// by the caller after publishing each root or brief.
pub struct Preparation {
    pub metadata: Data,
    budget: resources::ResourceBudget,
    session: SessionContext,
    inputs: Vec<ValidationInput>,
    tables: Vec<ClosureTable>,
    edges: PreparedEdges,
    artifact_programs: ArtifactPrograms,
    inventory_programs: InventoryPrograms,
    root: usize,
    brief: usize,
}
type MetadataLoader = for<'a, 'sources> fn(
    &'a CompletedInputs,
    &'a SessionContext,
    &'a [ValidationInput],
    &'a mut crate::consumed_rows::ConsumedInputs,
    &'a mut analysis::expected::CoverageAdmission<'sources>,
    &'a mut Data,
) -> BoxFuture<'a, Result<(), ModelError>>;
fn load_metadata<'a, 'sources, R: Record>(
    access: &'a CompletedInputs,
    session: &'a SessionContext,
    frames: &'a [ValidationInput],
    consumed: &'a mut crate::consumed_rows::ConsumedInputs,
    admission: &'a mut analysis::expected::CoverageAdmission<'sources>,
    metadata: &'a mut Data,
) -> BoxFuture<'a, Result<(), ModelError>> {
    Box::pin(async move {
        while let Some((input, permit)) = consumed.next::<R>(access)? {
            if crate::consumed_rows::stream_artifact_admission(access, &input, session, admission)
                .await?
            {
                continue;
            }
            crate::consumed_rows::stream_at(&permit, &input, access, session, |permit, batch| {
                admission.visit_if_expected(permit, batch)?;
                if frames.iter().any(|frame| {
                    frame.type_id() == input.type_id() && frame.prefix() == input.prefix()
                }) {
                    metadata.visit_input(&input, batch)?;
                }
                Ok(())
            })
            .await?;
        }
        Ok(())
    })
}
fn read_metadata<'a, 'sources>(
    access: &'a CompletedInputs,
    session: &'a SessionContext,
    frames: &'a [ValidationInput],
    consumed: &'a mut crate::consumed_rows::ConsumedInputs,
    admission: &'a mut analysis::expected::CoverageAdmission<'sources>,
    metadata: &'a mut Data,
) -> BoxFuture<'a, Result<(), ModelError>> {
    Box::pin(async move {
        let mut loaders: Vec<MetadataLoader> = Vec::new();
        macro_rules! read {($($field:ident:$ty:ty,)*) => {$(loaders.push(load_metadata::<$ty>);)*};}
        decoder_inputs!(read);
        for loader in loaders {
            loader(access, session, frames, consumed, admission, metadata).await?;
        }
        Ok(())
    })
}
struct WindowGrain {
    scope: PreparedClosure,
    allowed: Vec<TypeId>,
    artifacts: ArtifactSelection,
}
struct WindowColumns {
    rows: Vec<Vec<arrow_array::RecordBatch>>,
    memberships: charged::ChargedSet<(usize, usize, [u8; 16])>,
    selected: charged::ChargedSet<(usize, [u8; 16])>,
    consumed: Vec<crate::consumed_rows::ConsumedInputs>,
    union_consumed: crate::consumed_rows::ConsumedInputs,
    charge: charged::StateCharge,
}
pub struct RenderWindow<'a> {
    preparation: &'a Preparation,
    keys: Vec<[u8; 16]>,
    brief: bool,
    columns: WindowColumns,
}
impl RenderWindow<'_> {
    fn data(
        &self,
        partition: usize,
        budget: &resources::ResourceBudget,
        cancellation: &crate::workspace::Cancellation,
    ) -> Result<Data, ModelError> {
        self.keys
            .get(partition)
            .ok_or(ModelError::Schema("E0 window partition"))?;
        let mut data = Data::new(budget);
        for (table, input) in self.preparation.inputs.iter().enumerate() {
            for rows in &self.columns.rows[table] {
                cancellation.check()?;
                let _filter = budget.reserve(
                    "retrieval-columnar-partition",
                    rows.get_array_memory_size() + rows.num_rows() * 32 + 4096,
                )?;
                let mut mask = Vec::with_capacity(rows.num_rows());
                for row in 0..rows.num_rows() {
                    let key = crate::scoped_admission::column(rows, "id", row)?
                        .ok_or(ModelError::Schema("E0 union identity"))?;
                    mask.push(self.columns.memberships.contains(&(partition, table, key)));
                }
                let selected = datafusion::arrow::compute::filter_record_batch(
                    rows,
                    &arrow_array::BooleanArray::from(mask),
                )
                .map_err(ModelError::codec)?;
                if selected.num_rows() != 0 {
                    if self.brief && input.type_id() == TypeId::of::<catalog::CatalogMember>() {
                        data.completion_visit(input, &selected)?;
                    } else {
                        data.visit_input(input, &selected)?;
                    }
                }
            }
        }
        data.facts
            .definitions
            .insert_borrowed(self.preparation.metadata.selected()?)?;
        if let Some(tokenizer) = self.preparation.metadata.tokenizer() {
            data.set_tokenizer(tokenizer.clone());
        }
        Ok(data)
    }
    pub async fn render(
        &self,
        access: &CompletedInputs,
        partition: usize,
        budget: &resources::ResourceBudget,
        cancellation: &crate::workspace::Cancellation,
    ) -> Result<(Data, Output), ModelError> {
        let data = self.data(partition, budget, cancellation)?;
        let key = self.keys[partition];
        if self.brief {
            self.preparation
                .finish_brief(crate::retrieval::nominal(&key)?, data, budget)
        } else {
            self.preparation
                .finish_root(access, crate::retrieval::nominal(&key)?, data, budget)
                .await
        }
    }
}
type WindowLoader = for<'a> fn(
    &'a Preparation,
    &'a CompletedInputs,
    &'a [WindowGrain],
    bool,
    &'a resources::ResourceBudget,
    &'a crate::workspace::Cancellation,
    &'a mut WindowColumns,
) -> BoxFuture<'a, Result<(), ModelError>>;
fn load_window<'a, R: Record>(
    preparation: &'a Preparation,
    access: &'a CompletedInputs,
    grains: &'a [WindowGrain],
    brief: bool,
    budget: &'a resources::ResourceBudget,
    cancellation: &'a crate::workspace::Cancellation,
    columns: &'a mut WindowColumns,
) -> BoxFuture<'a, Result<(), ModelError>> {
    Box::pin(async move {
        for (partition, grain) in grains.iter().enumerate() {
            while let Some((input, permit)) = columns.consumed[partition].next::<R>(access)? {
                cancellation.check()?;
                let table = preparation
                    .inputs
                    .iter()
                    .position(|i| i.type_id() == input.type_id() && i.prefix() == input.prefix())
                    .ok_or(ModelError::Schema("E0 window immutable input"))?;
                let Some(selected) =
                    preparation.select(&grain.scope, table, &grain.allowed, &grain.artifacts)?
                else {
                    continue;
                };
                let query = format!("SELECT id FROM ({selected}) membership");
                crate::consumed_rows::stream_query_at(
                    &permit,
                    &input,
                    access,
                    grain.scope.session(),
                    &query,
                    |_, batch| {
                        cancellation.check()?;
                        let _transfer = budget.reserve(
                            "retrieval-membership-transfer",
                            batch.get_array_memory_size(),
                        )?;
                        for row in 0..batch.num_rows() {
                            let key = crate::scoped_admission::column(batch, "id", row)?
                                .ok_or(ModelError::Schema("E0 membership identity"))?;
                            columns
                                .memberships
                                .insert(&mut columns.charge, (partition, table, key))?;
                            columns.selected.insert(&mut columns.charge, (table, key))?;
                        }
                        Ok(())
                    },
                )
                .await?;
            }
        }
        while let Some((input, permit)) = columns.union_consumed.next::<R>(access)? {
            cancellation.check()?;
            let table = preparation
                .inputs
                .iter()
                .position(|i| i.type_id() == input.type_id() && i.prefix() == input.prefix())
                .ok_or(ModelError::Schema("E0 window union input"))?;
            let keys = columns
                .selected
                .range((table, [0; 16])..=(table, [255; 16]));
            if keys.clone().next().is_none() {
                continue;
            }
            let _query =
                budget.reserve("retrieval-union-query", keys.clone().count() * 128 + 4096)?;
            let keys = keys
                .map(|(_, key)| format!("X'{}'", hex::encode(key)))
                .collect::<Vec<_>>()
                .join(",");
            let selected = format!(
                "SELECT * FROM {} WHERE id IN ({keys})",
                identifier(&preparation.tables[table].alias)
            );
            let selected = preparation.projected(table, brief, &selected);
            crate::consumed_rows::stream_query_at(
                &permit,
                &input,
                access,
                &preparation.session,
                &selected,
                |_, batch| {
                    cancellation.check()?;
                    let target = &mut columns.rows[table];
                    let additional = if target.len() == target.capacity() {
                        target.capacity().max(4)
                    } else {
                        0
                    };
                    columns.charge.grow(
                        batch.get_array_memory_size()
                            + additional * size_of::<arrow_array::RecordBatch>(),
                    )?;
                    target.reserve_exact(additional);
                    target.push(batch.clone());
                    Ok(())
                },
            )
            .await?;
        }
        Ok(())
    })
}
impl Preparation {
    pub async fn prepare(
        access: &CompletedInputs,
        runtime: &Workspace,
        model: &Arc<ValidatedModel>,
        admission: &mut analysis::expected::CoverageAdmission<'_>,
    ) -> Result<Self, ModelError> {
        let session = access.session(runtime).await?;
        let mut metadata = Data::new(runtime.budget());
        let frames = Data::frame_inputs();
        let mut declarations = frames.clone();
        declarations.extend(analysis::expected::inputs(
            analysis::AnalysisMethod::Retrieval,
        ));
        let mut consumed =
            crate::consumed_rows::ConsumedInputs::new(declarations, runtime.budget())?;
        read_metadata(
            access,
            &session,
            &frames,
            &mut consumed,
            admission,
            &mut metadata,
        )
        .await?;
        consumed.finish("retrieval-metadata")?;
        metadata.selected()?;
        c1::frames::verify(
            &metadata.source.facts.runs,
            &metadata.source.facts.core_invocations,
            &metadata.facts.evidence_invocations,
            &metadata.facts.evidence_sources,
            &metadata.facts.evidence_inputs,
            &metadata.source.runtime.lower(),
            runtime.budget(),
        )?;
        let inputs = Data::inputs();
        let tables: Vec<_> = inputs
            .iter()
            .map(|input| {
                Ok(ClosureTable {
                    relation: model
                        .relation(input.name())
                        .ok_or(ModelError::Schema("E0 input model relation"))?
                        .clone(),
                    alias: access.table_for(input)?,
                })
            })
            .collect::<Result<_, ModelError>>()?;
        let artifact_programs = ArtifactPrograms::new(
            &inputs,
            &tables,
            model,
            runtime.budget(),
            Some(runtime.scope_programs()),
        )?;
        let inventory_programs = InventoryPrograms::new(
            &inputs,
            &tables,
            model,
            runtime.budget(),
            Some(runtime.scope_programs()),
        )?;
        let (edges, root, brief) = Self::prepare_bound_in(
            &inputs,
            &tables,
            model,
            &session,
            runtime.budget(),
            Some(runtime.scope_programs()),
        )
        .await?;
        Ok(Self {
            metadata,
            budget: runtime.budget().clone(),
            session,
            inputs,
            tables,
            edges,
            artifact_programs,
            inventory_programs,
            root,
            brief,
        })
    }
    #[cfg(test)]
    async fn prepare_bound(
        inputs: &[ValidationInput],
        tables: &[ClosureTable],
        model: &ValidatedModel,
        session: &SessionContext,
        budget: &resources::ResourceBudget,
    ) -> Result<(PreparedEdges, usize, usize), ModelError> {
        Self::prepare_bound_in(inputs, tables, model, session, budget, None).await
    }
    async fn prepare_bound_in(
        inputs: &[ValidationInput],
        tables: &[ClosureTable],
        model: &ValidatedModel,
        session: &SessionContext,
        budget: &resources::ResourceBudget,
        interner: Option<&std::sync::Mutex<scope_program::ScopeInterner>>,
    ) -> Result<(PreparedEdges, usize, usize), ModelError> {
        let root_table = typed::<c1::EvidenceRoot>(inputs)?;
        let brief_table = typed::<synthesis::briefs::Brief>(inputs)?;
        let root = tables.len();
        let brief = root + 1;
        let mut bindings = tables.to_vec();
        bindings.extend([tables[root_table].clone(), tables[brief_table].clone()]);
        let relations = tables
            .iter()
            .map(|table| table.relation.clone())
            .collect::<Vec<_>>();
        let program = catalog_scope_program::retrieval(inputs.to_vec(), &relations, model, budget)?;
        let compiled =
            crate::scope_compilation::compile(program.program(), model, budget, interner)?;
        let plan = crate::scope_compilation::lower_compiled(
            compiled,
            &bindings,
            &scope_program::ScopeParameters(vec![]),
            budget,
        )?;
        Ok((plan.prepare(session, budget).await?, root, brief))
    }
    /// Discover one bounded window together and retain each exact projected membership.
    /// Rich native rows are fetched once; each owning renderer decodes its own partition.
    pub async fn render_window<'a>(
        &'a self,
        access: &CompletedInputs,
        keys: &[[u8; 16]],
        brief: bool,
        budget: &resources::ResourceBudget,
        cancellation: &crate::workspace::Cancellation,
    ) -> Result<RenderWindow<'a>, ModelError> {
        if !self.budget.shares_pool(budget) {
            return Err(ModelError::Conflict("E0 window budget changed"));
        }
        let mut charge = charged::StateCharge::new(budget, "retrieval-columnar-window");
        charge.grow(
            keys.len()
                * (size_of::<[u8; 16]>()
                    + size_of::<crate::consumed_rows::PreparedRoot>()
                    + size_of::<WindowGrain>()
                    + size_of::<crate::consumed_rows::ConsumedInputs>()
                    + self.inputs.len() * size_of::<TypeId>())
                + self.inputs.len() * size_of::<Vec<arrow_array::RecordBatch>>()
                + 4096,
        )?;
        let root = if brief { self.brief } else { self.root };
        let roots = keys
            .iter()
            .map(|key| crate::consumed_rows::PreparedRoot {
                table: root,
                key: *key,
                kind: crate::consumed_rows::PreparedRootKind::Virtual,
            })
            .collect::<Vec<_>>();
        let batch = self
            .edges
            .batch_with_cancellation(&roots, budget, cancellation)
            .await?;
        drop(roots);
        let mut grains = Vec::with_capacity(keys.len());
        for (partition, key) in keys.iter().enumerate() {
            cancellation.check()?;
            let scope = batch.partition_scope(partition, budget)?;
            let (allowed, artifacts) = if brief {
                (
                    Data::brief_types(),
                    self.artifacts(None, Some(crate::retrieval::nominal(key)?), budget)?,
                )
            } else {
                let id = crate::retrieval::nominal(key)?;
                let header = self.header::<c1::EvidenceRoot>(access, id, budget).await?;
                let subject = self
                    .header::<c1::RootSubject>(access, header.subject, budget)
                    .await?;
                (
                    Data::root_types(&subject),
                    self.artifacts(Some((id, &subject)), None, budget)?,
                )
            };
            grains.push(WindowGrain {
                scope,
                allowed,
                artifacts,
            });
        }
        let mut columns = WindowColumns {
            rows: (0..self.inputs.len()).map(|_| Vec::new()).collect(),
            memberships: Default::default(),
            selected: Default::default(),
            consumed: (0..keys.len())
                .map(|_| crate::consumed_rows::ConsumedInputs::new(self.inputs.clone(), budget))
                .collect::<Result<Vec<_>, _>>()?,
            union_consumed: crate::consumed_rows::ConsumedInputs::new(self.inputs.clone(), budget)?,
            charge,
        };
        let mut loaders: Vec<WindowLoader> = Vec::new();
        macro_rules! read{($($field:ident:$ty:ty,)*)=>{$(loaders.push(load_window::<$ty>);)*};}
        decoder_inputs!(read);
        for loader in loaders {
            loader(
                self,
                access,
                &grains,
                brief,
                budget,
                cancellation,
                &mut columns,
            )
            .await?;
        }
        for consumed in std::mem::take(&mut columns.consumed) {
            consumed.finish("retrieval-window-partition")?;
        }
        let terminal = std::mem::replace(
            &mut columns.union_consumed,
            crate::consumed_rows::ConsumedInputs::new(vec![], budget)?,
        );
        terminal.finish("retrieval-window-union")?;
        drop(grains);
        drop(batch);
        Ok(RenderWindow {
            preparation: self,
            keys: keys.to_vec(),
            brief,
            columns,
        })
    }
    pub fn session(&self) -> &SessionContext {
        &self.session
    }
    pub async fn roots(
        &self,
        input: Id<input::InputRevision>,
        context: Id<attribution::AnalysisContext>,
    ) -> Result<datafusion::physical_plan::SendableRecordBatchStream, ModelError> {
        self.inventory(0, input, context).await
    }
    pub async fn briefs(
        &self,
        input: Id<input::InputRevision>,
        context: Id<attribution::AnalysisContext>,
    ) -> Result<datafusion::physical_plan::SendableRecordBatchStream, ModelError> {
        self.inventory(1, input, context).await
    }
    async fn inventory(
        &self,
        kind: usize,
        input: Id<input::InputRevision>,
        context: Id<attribution::AnalysisContext>,
    ) -> Result<datafusion::physical_plan::SendableRecordBatchStream, ModelError> {
        let compiled = &self.inventory_programs.owners[kind];
        let parameters = scope_program::ScopeParameters(vec![
            scope_program::ScopeValue::Nominal(*input.bytes()),
            scope_program::ScopeValue::Nominal(*context.bytes()),
        ]);
        let _query = self.budget.reserve(
            "retrieval-inventory-query",
            crate::scope_compilation::lowering_allowance(
                compiled.program(),
                &self.tables,
                &parameters,
            )
            .saturating_mul(2)
                + 4096,
        )?;
        let queries = crate::scope_compilation::select_pair_queries(
            compiled.program(),
            &self.tables,
            &parameters,
        )?;
        let query = &queries
            .first()
            .ok_or(ModelError::Schema("E0 root inventory absent"))?
            .2;
        crate::sql::query(
            &self.session,
            &format!("SELECT target_id AS id FROM ({query}) inventory ORDER BY id"),
        )
        .await
        .map_err(ModelError::codec)?
        .execute_stream()
        .await
        .map_err(ModelError::codec)
    }
    async fn header<R: Record>(
        &self,
        access: &CompletedInputs,
        id: Id<R>,
        budget: &resources::ResourceBudget,
    ) -> Result<R, ModelError> {
        let index = typed::<R>(&self.inputs)?;
        let input = &self.inputs[index];
        let permit = access.read_at::<R>(input.prefix())?;
        let mut rows = normalized::Rows::new(budget);
        let sql = format!(
            "SELECT * FROM {} WHERE id=X'{}'",
            identifier(&self.tables[index].alias),
            id.hex()
        );
        crate::consumed_rows::stream_query_at(
            &permit,
            input,
            access,
            &self.session,
            &sql,
            |_, batch| {
                rows.decode(batch)?;
                Ok(())
            },
        )
        .await?;
        Ok(build::need(&rows, id)?.clone())
    }
    fn artifacts(
        &self,
        root: Option<(Id<c1::EvidenceRoot>, &c1::RootSubject)>,
        brief: Option<Id<synthesis::briefs::Brief>>,
        budget: &resources::ResourceBudget,
    ) -> Result<ArtifactSelection, ModelError> {
        let (kind, key) = if let Some((id, subject)) = root {
            (
                catalog_scope_program::RetrievalArtifactRoot::subject(subject),
                *id.bytes(),
            )
        } else {
            (
                catalog_scope_program::RetrievalArtifactRoot::Brief,
                *brief
                    .ok_or(ModelError::Schema("E0 grain owner absent"))?
                    .bytes(),
            )
        };
        let program = self.artifact_programs.get(kind);
        let parameters =
            scope_program::ScopeParameters(vec![scope_program::ScopeValue::Nominal(key)]);
        let charge = budget.reserve(
            "retrieval-artifact-query",
            crate::scope_compilation::lowering_allowance(
                program.program(),
                &self.tables,
                &parameters,
            )
            .saturating_mul(2)
            .saturating_add(4096),
        )?;
        let queries = crate::scope_compilation::select_pair_queries(
            program.program(),
            &self.tables,
            &parameters,
        )?;
        let domains = queries
            .into_iter()
            .map(|(_, _, sql)| format!("SELECT target_id AS source FROM ({sql}) sources"))
            .collect::<Vec<_>>();
        let sql = if domains.is_empty() {
            let artifact = typed::<source::SourceArtifact>(&self.inputs)?;
            format!(
                "SELECT id AS source FROM {} WHERE FALSE",
                identifier(&self.tables[artifact].alias)
            )
        } else {
            domains.join(" UNION ALL ")
        };
        Ok(ArtifactSelection {
            sql,
            _charge: charge,
        })
    }
    fn select(
        &self,
        scope: &PreparedClosure,
        index: usize,
        allowed: &[TypeId],
        artifacts: &str,
    ) -> Result<Option<String>, ModelError> {
        let input = &self.inputs[index];
        if !allowed.contains(&input.type_id())
            || input.type_id() == TypeId::of::<artifact::ArtifactChunk>()
        {
            return Ok(None);
        }
        let selected = if input.type_id() == TypeId::of::<source::SourceArtifact>()
            && !allowed.contains(&TypeId::of::<catalog::CatalogCandidate>())
        {
            format!(
                "SELECT * FROM {} WHERE id IN ({artifacts})",
                identifier(&self.tables[index].alias)
            )
        } else {
            scope.select(index)?
        };
        Ok(Some(self.projected(
            index,
            allowed.contains(&TypeId::of::<synthesis::briefs::Brief>()),
            &selected,
        )))
    }
    fn projected(&self, index: usize, brief: bool, selected: &str) -> String {
        let kind = self.inputs[index].type_id();
        if kind == TypeId::of::<catalog::CatalogMember>() && brief {
            format!("SELECT id,input,access FROM ({selected}) ownership")
        } else if kind == TypeId::of::<diagnostics::RuffDiagnosticObservation>() {
            format!("SELECT id,channel,settings FROM ({selected}) diagnostic_properties")
        } else if kind == TypeId::of::<diagnostics::PyreflyDiagnosticObservation>() {
            format!("SELECT id,channel FROM ({selected}) diagnostic_properties")
        } else {
            selected.into()
        }
    }
    async fn chunks(
        &self,
        access: &CompletedInputs,
        ranges: &retrieval::source::Ranges,
        data: &mut Data,
        budget: &resources::ResourceBudget,
    ) -> Result<(), ModelError> {
        let input = ValidationInput::of::<artifact::ArtifactChunk>(&["id"]);
        let table = identifier(&access.table_for(&input)?);
        let mut charge = charged::StateCharge::new(budget, "retrieval-original-demand-union");
        let mut keys = charged::ChargedSet::default();
        for (artifact, start, end) in ranges.iter() {
            crate::original_demands::union_range(&mut keys, &mut charge, *artifact, *start, *end)?;
        }
        crate::original_demands::stream_selected(&self.session, &table, &keys, budget, |batch| {
            data.facts.chunks.decode(batch)
        })
        .await?;
        Ok(())
    }
    async fn finish_root(
        &self,
        access: &CompletedInputs,
        id: Id<c1::EvidenceRoot>,
        mut data: Data,
        budget: &resources::ResourceBudget,
    ) -> Result<(Data, Output), ModelError> {
        let root = build::need(&data.evidence.roots, id)?;
        let mut parents = self
            .metadata
            .facts
            .evidence_invocations
            .iter()
            .filter(|invocation| {
                invocation.input == root.input
                    && invocation.context == root.context
                    && invocation.definition == c1::build::definition().1.id()
                    && invocation.subject.is_none()
            });
        let parent = parents
            .next()
            .ok_or_else(|| build::invalid("E0 root fixed C1 parent absent"))?;
        if parents.next().is_some() {
            return Err(build::invalid("E0 root C1 parent ambiguous"));
        }
        if data.facts.evidence_links.len() != 1
            || data.facts.evidence_links.iter().next()
                != Some(&c1::EvidenceInvocation {
                    root: id,
                    invocation: parent.id(),
                })
        {
            return Err(build::invalid(
                "retrieval C1 root invocation closure differs",
            ));
        }
        let ranges = retrieval::source::root_ranges(&data, id, budget)?;
        self.chunks(access, &ranges, &mut data, budget).await?;
        drop(ranges);
        let rows = build::root(&data, id, budget)?;
        rows.verify_completion(&data, budget)?;
        Ok((data, rows))
    }
    fn finish_brief(
        &self,
        id: Id<synthesis::briefs::Brief>,
        data: Data,
        budget: &resources::ResourceBudget,
    ) -> Result<(Data, Output), ModelError> {
        if data.synthesis.briefs.len() != 1 || data.synthesis.briefs.get(id).is_none() {
            return Err(build::invalid("E0 brief grain membership differs"));
        }
        let mut rows = Output::new(budget);
        build::extend_synthesis(&data, &mut rows, budget)?;
        rows.verify_completion(&data, budget)?;
        Ok((data, rows))
    }
}
/// Append one rendered grain to the stage's already declared publications.
pub fn publish_mandatory<'a>(
    output: &'a mut ProducerOutput,
    rows: &'a Output,
) -> BoxFuture<'a, Result<(), ModelError>> {
    Box::pin(async move {
        type Emit =
            for<'a> fn(&'a Output, &'a ProducerOutput) -> BoxFuture<'a, Result<(), ModelError>>;
        macro_rules! entries {($($field:ident:$ty:ty,)*) => {const EMITTERS: &[Emit] = &[$(|rows, output| producer_operations::emit(&rows.$field, output),)*];};}
        lctx_model::retrieval_outputs!(entries);
        for emit in EMITTERS {
            emit(rows, output).await?;
        }
        Ok(())
    })
}

#[cfg(test)]
mod controls {
    use super::*;
    use datafusion::datasource::MemTable;
    use futures::TryStreamExt;
    fn id<R>(n: u8) -> Id<R> {
        serde::Deserialize::deserialize(serde::de::value::SeqDeserializer::<
            _,
            serde::de::value::Error,
        >::new([n; 16].into_iter()))
        .unwrap()
    }
    fn fixture() -> (SessionContext, Vec<ValidationInput>, Vec<ClosureTable>) {
        let model = lctx_model::domain::model().unwrap();
        let inputs = Data::inputs();
        let session = SessionContext::new();
        let tables = inputs
            .iter()
            .enumerate()
            .map(|(index, input)| {
                let relation = model.relation(input.name()).unwrap().clone();
                let alias = format!("retrieval_fixture_{index}");
                let batch = arrow_array::RecordBatch::new_empty(relation.schema().clone());
                session
                    .register_table(
                        alias.as_str(),
                        Arc::new(MemTable::try_new(batch.schema(), vec![vec![batch]]).unwrap()),
                    )
                    .unwrap();
                ClosureTable { relation, alias }
            })
            .collect();
        (session, inputs, tables)
    }
    fn install<R: Record>(
        session: &SessionContext,
        inputs: &[ValidationInput],
        tables: &[ClosureTable],
        rows: &[R],
    ) {
        let index = typed::<R>(inputs).unwrap();
        let batch = R::encode(rows).unwrap();
        session
            .deregister_table(tables[index].alias.as_str())
            .unwrap();
        session
            .register_table(
                tables[index].alias.as_str(),
                Arc::new(MemTable::try_new(batch.schema(), vec![vec![batch]]).unwrap()),
            )
            .unwrap();
    }
    async fn scoped(
        preparation: &Preparation,
        root: &c1::EvidenceRoot,
        subject: &c1::RootSubject,
        budget: &resources::ResourceBudget,
    ) -> Data {
        let scope = preparation
            .edges
            .grain(
                preparation.root,
                &format!("id=X'{}'", root.id().hex()),
                budget,
            )
            .await
            .unwrap();
        let allowed = Data::root_types(subject);
        let artifacts = preparation
            .artifacts(Some((root.id(), subject)), None, budget)
            .unwrap();
        let mut data = Data::new(budget);
        for (index, input) in preparation.inputs.iter().enumerate() {
            if let Some(sql) = preparation
                .select(&scope, index, &allowed, &artifacts)
                .unwrap()
            {
                let mut rows = crate::sql::query(scope.session(), &sql)
                    .await
                    .unwrap()
                    .execute_stream()
                    .await
                    .unwrap();
                while let Some(batch) = rows.try_next().await.unwrap() {
                    data.visit_input(input, &batch).unwrap();
                }
            }
        }
        data.facts
            .definitions
            .insert(preparation.metadata.selected().unwrap().clone())
            .unwrap();
        data
    }
    #[tokio::test]
    async fn actual_root_grains_match_finite_renderer_and_exclude_referenced_rich_members() {
        let budget = resources::ResourceBudget::fixed(8 << 20).unwrap();
        let (session, inputs, tables) = fixture();
        let source =
            source::SourceArtifact::from_bytes(id(1), "example.py".into(), b"run()\n").unwrap();
        let other =
            source::SourceArtifact::from_bytes(id(1), "x".repeat(16 << 20), b"unread").unwrap();
        let module = source::Module {
            source: other.id(),
            qualified_name: "other".into(),
        };
        let member = catalog::CatalogMember {
            input: id(1),
            access: module.id(),
            path: vec!["x".repeat(16 << 20)],
            name: "x".repeat(16 << 20),
        };
        let qualification = assertion::AssertionQualification {
            assumptions: assumptions::AssumptionSet::empty_id(),
            context: id(2),
            scope: source::CoverageScope::Artifact {
                artifact: source.id(),
            }
            .id(),
            condition: conditions::Diagram::always().id(),
            modality: attribution::Modality::Definite,
            approximation: assertion::Approximation::Exact,
        };
        let original = c1::OriginalSource::Artifact {
            artifact: source.id(),
        };
        let scenario = c1::CatalogScenario {
            source: id(3),
            extraction: deployment::CheckStatus::Passed,
            parse: deployment::CheckStatus::Passed,
            binding: deployment::CheckStatus::Blocked,
            environment: deployment::CheckStatus::Blocked,
            execution: deployment::CheckStatus::NotRun,
            intent: c1::Intent::Demonstration,
        };
        let span = c1::ScenarioSpan {
            scenario: scenario.id(),
            ordinal: 0,
            role: c1::SpanRole::EnclosingModule,
            source: original.id(),
        };
        let association = c1::ScenarioAssociation {
            scenario: scenario.id(),
            member: member.id(),
            alternative: id(4),
            basis: c1::AssociationBasis::ResolvedTarget,
            phase: calls::CallPhase::Call,
            intent: c1::Intent::Demonstration,
            qualification: qualification.id(),
        };
        let subject = c1::RootSubject::Scenario {
            scenario: scenario.id(),
        };
        let root = c1::EvidenceRoot {
            input: source.input,
            context: id(2),
            subject: subject.id(),
        };
        install(&session, &inputs, &tables, &[source.clone(), other]);
        install(&session, &inputs, &tables, &[module]);
        install(&session, &inputs, &tables, &[member]);
        install(
            &session,
            &inputs,
            &tables,
            std::slice::from_ref(&qualification),
        );
        install(&session, &inputs, &tables, std::slice::from_ref(&original));
        install(&session, &inputs, &tables, std::slice::from_ref(&scenario));
        install(&session, &inputs, &tables, std::slice::from_ref(&span));
        install(
            &session,
            &inputs,
            &tables,
            std::slice::from_ref(&association),
        );
        install(&session, &inputs, &tables, std::slice::from_ref(&subject));
        install(&session, &inputs, &tables, std::slice::from_ref(&root));
        let mut metadata = Data::new(&budget);
        metadata
            .facts
            .definitions
            .insert(retrieval::Definition::builtin(false))
            .unwrap();
        let (edges, root_index, brief) = Preparation::prepare_bound(
            &inputs,
            &tables,
            &lctx_model::domain::model().unwrap(),
            &session,
            &budget,
        )
        .await
        .unwrap();
        let artifact_programs = ArtifactPrograms::new(
            &inputs,
            &tables,
            &lctx_model::domain::model().unwrap(),
            &budget,
            None,
        )
        .unwrap();
        let inventory_programs = InventoryPrograms::new(
            &inputs,
            &tables,
            &lctx_model::domain::model().unwrap(),
            &budget,
            None,
        )
        .unwrap();
        let preparation = Preparation {
            metadata,
            budget: budget.clone(),
            session,
            inputs,
            tables,
            edges,
            root: root_index,
            artifact_programs,
            inventory_programs,
            brief,
        };
        let mut actual = scoped(&preparation, &root, &subject, &budget).await;
        assert!(actual.source.catalog.members.is_empty());
        assert!(actual.source.core.modules.is_empty());
        assert_eq!(actual.source.core.artifacts.len(), 1);
        assert_eq!(
            actual.source.core.artifacts.iter().next().unwrap().id(),
            source.id()
        );
        let mut expected = Data::new(&budget);
        expected
            .facts
            .definitions
            .insert(retrieval::Definition::builtin(false))
            .unwrap();
        expected
            .source
            .core
            .artifacts
            .insert(source.clone())
            .unwrap();
        expected
            .source
            .core
            .qualifications
            .insert(qualification)
            .unwrap();
        expected.evidence.original_sources.insert(original).unwrap();
        expected.evidence.scenarios.insert(scenario).unwrap();
        expected.evidence.spans.insert(span).unwrap();
        expected.evidence.associations.insert(association).unwrap();
        expected.evidence.subjects.insert(subject).unwrap();
        expected.evidence.roots.insert(root.clone()).unwrap();
        for chunk in artifact::ArtifactChunk::split(&source, b"run()\n").unwrap() {
            actual.facts.chunks.insert(chunk.clone()).unwrap();
            expected.facts.chunks.insert(chunk).unwrap();
        }
        let actual_rows = build::root(&actual, root.id(), &budget).unwrap();
        let oracle = build::build(&expected, &budget).unwrap();
        actual_rows.matches(&oracle).unwrap();
        {
            let scope = preparation
                .edges
                .grain(
                    preparation.root,
                    &format!("id=X'{}'", root.id().hex()),
                    &budget,
                )
                .await
                .unwrap();
            let selected_subject = expected.evidence.subjects.get(root.subject).unwrap();
            let allowed = Data::root_types(selected_subject);
            let artifacts = preparation
                .artifacts(Some((root.id(), selected_subject)), None, &budget)
                .unwrap();
            let mut columns = WindowColumns {
                rows: (0..preparation.inputs.len()).map(|_| Vec::new()).collect(),
                memberships: Default::default(),
                selected: Default::default(),
                consumed: vec![],
                union_consumed: crate::consumed_rows::ConsumedInputs::new(vec![], &budget).unwrap(),
                charge: charged::StateCharge::new(&budget, "retrieval-window-control"),
            };
            columns
                .charge
                .grow(preparation.inputs.len() * size_of::<Vec<arrow_array::RecordBatch>>() + 4096)
                .unwrap();
            for (table, _input) in preparation.inputs.iter().enumerate() {
                if let Some(query) = preparation
                    .select(&scope, table, &allowed, &artifacts)
                    .unwrap()
                {
                    let mut stream = crate::sql::query(preparation.session(), &query)
                        .await
                        .unwrap()
                        .execute_stream()
                        .await
                        .unwrap();
                    while let Some(rows) = stream.try_next().await.unwrap() {
                        columns
                            .charge
                            .grow(
                                rows.get_array_memory_size()
                                    + size_of::<arrow_array::RecordBatch>(),
                            )
                            .unwrap();
                        for row in 0..rows.num_rows() {
                            let key = crate::scoped_admission::column(&rows, "id", row)
                                .unwrap()
                                .unwrap();
                            for partition in 0..2 {
                                columns
                                    .memberships
                                    .insert(&mut columns.charge, (partition, table, key))
                                    .unwrap();
                            }
                        }
                        columns.rows[table].push(rows);
                    }
                }
            }
            let window = RenderWindow {
                preparation: &preparation,
                keys: vec![*root.id().bytes(); 2],
                brief: false,
                columns,
            };
            for partition in 0..2 {
                let mut selected = window
                    .data(
                        partition,
                        &budget,
                        &crate::workspace::Cancellation::default(),
                    )
                    .unwrap();
                for chunk in expected.facts.chunks.iter() {
                    selected.facts.chunks.insert_borrowed(chunk).unwrap();
                }
                build::root(&selected, root.id(), &budget)
                    .unwrap()
                    .matches(&oracle)
                    .unwrap();
            }
            assert!(
                window
                    .data(2, &budget, &crate::workspace::Cancellation::default())
                    .is_err()
            );
        }
        assert_eq!(actual_rows.units.len(), 1);
        drop(actual_rows);
        drop(oracle);
        drop(actual);
        drop(expected);
        drop(preparation);
        assert_eq!(budget.reserved(), 0);
    }
    #[test]
    fn declared_metadata_and_root_inputs_have_typed_decoder_reachability() {
        let mut types = std::collections::BTreeSet::new();
        macro_rules! collect{($($field:ident:$ty:ty,)*)=>{$(types.insert(TypeId::of::<$ty>());)*};}
        decoder_inputs!(collect);
        crate::consumed_rows::assert_decoder_reachability(Data::inputs(), &types);
    }
}
