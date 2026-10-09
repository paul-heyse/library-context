//! Single S0 writer; rendering never reconstructs graphs or invents completed parents.
use crate::producer_operations::{self, Declaration};
use crate::{
    synthesis_preparation,
    workspace::{CompletedInputs, ProducerOutput, Workspace},
};
use futures::future::BoxFuture;
use lctx_model::domain::{
    analysis::{self, synthesis::*},
    normalized::Rows,
    stages::*,
    synthesis::{self, production::Data},
    *,
};
use std::sync::Arc;
// Decoder reachability is separate from the model-owned consumed source inventory.
macro_rules! decoder_inputs {
    ($apply:ident) => {
        lctx_model::synthesis_frame_inputs!($apply);
        lctx_model::synthesis_documentary_inputs!($apply);
        lctx_model::synthesis_automatic_inputs!($apply);
        lctx_model::synthesis_observation_inputs!($apply);
        lctx_model::synthesis_summary_inputs!($apply);
        lctx_model::synthesis_terminal_inputs!($apply);
        lctx_model::synthesis_pattern_inputs!($apply);
        lctx_model::synthesis_setup_inputs!($apply);
        lctx_model::synthesis_pattern_named_inputs!($apply);
        lctx_model::synthesis_control_text_inputs!($apply);
        lctx_model::ownership_scope_inputs!($apply);
        lctx_model::expected_domain_inputs!($apply);
        $apply! {public:structural::PublicCandidate,handoff_values:structural::handoffs::ValueSource,}
    };
}
use lctx_model::domain::catalog_scope_program::SynthesisTextPhase as LoadPhase;
struct SynthesisScopes {
    inputs: Vec<ValidationInput>,
    tables: Vec<crate::consumed_rows::ClosureTable>,
    edges: crate::consumed_rows::PreparedEdges,
    member: usize,
    inventories: Vec<(
        synthesis_inventory_program::Inventory,
        Arc<scope_program::CompiledScopeProgram>,
    )>,
    _charge: charged::StateCharge,
}
fn typed<R: Record>(inputs: &[ValidationInput]) -> Result<usize, ModelError> {
    inputs
        .iter()
        .position(|input| input.type_id() == std::any::TypeId::of::<R>())
        .ok_or(ModelError::Schema("S0 scope relation absent"))
}
type MetadataLoader = for<'a, 'sources> fn(
    &'a CompletedInputs,
    &'a datafusion::prelude::SessionContext,
    &'a [ValidationInput],
    &'a mut crate::consumed_rows::ConsumedInputs,
    &'a mut analysis::expected::CoverageAdmission<'sources>,
    &'a mut Data,
) -> BoxFuture<'a, Result<(), ModelError>>;
fn load_metadata<'a, 'sources, R: Record>(
    access: &'a CompletedInputs,
    session: &'a datafusion::prelude::SessionContext,
    topology: &'a [ValidationInput],
    consumed: &'a mut crate::consumed_rows::ConsumedInputs,
    admission: &'a mut analysis::expected::CoverageAdmission<'sources>,
    data: &'a mut Data,
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
                if topology.iter().any(|item| {
                    item.type_id() == input.type_id() && item.prefix() == input.prefix()
                }) {
                    data.visit_input(&input, batch)?;
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
    session: &'a datafusion::prelude::SessionContext,
    topology: &'a [ValidationInput],
    consumed: &'a mut crate::consumed_rows::ConsumedInputs,
    admission: &'a mut analysis::expected::CoverageAdmission<'sources>,
    data: &'a mut Data,
) -> BoxFuture<'a, Result<(), ModelError>> {
    Box::pin(async move {
        let mut loaders: Vec<MetadataLoader> = Vec::new();
        macro_rules! read {($($field:ident:$ty:ty,)*) => {$(loaders.push(load_metadata::<$ty>);)*};}
        decoder_inputs!(read);
        for loader in loaders {
            loader(access, session, topology, consumed, admission, data).await?;
        }
        Ok(())
    })
}
type ScopedLoader = for<'a> fn(
    &'a SynthesisScopes,
    &'a CompletedInputs,
    &'a crate::consumed_rows::PreparedClosure,
    LoadPhase,
    &'a mut crate::consumed_rows::ConsumedInputs,
    &'a mut Data,
) -> BoxFuture<'a, Result<(), ModelError>>;
fn load_scoped<'a, R: Record>(
    scopes: &'a SynthesisScopes,
    access: &'a CompletedInputs,
    scope: &'a crate::consumed_rows::PreparedClosure,
    phase: LoadPhase,
    consumed: &'a mut crate::consumed_rows::ConsumedInputs,
    data: &'a mut Data,
) -> BoxFuture<'a, Result<(), ModelError>> {
    Box::pin(async move {
        while let Some((input, permit)) = consumed.next::<R>(access)? {
            let table = scopes
                .inputs
                .iter()
                .position(|candidate| {
                    candidate.type_id() == input.type_id() && candidate.prefix() == input.prefix()
                })
                .ok_or(ModelError::Conflict("S0 scoped declaration"))?;
            let selected = if input.type_id() == std::any::TypeId::of::<artifact::ArtifactChunk>() {
                scopes.chunks(scope, phase)?
            } else {
                scope.select(table)?
            };
            crate::consumed_rows::stream_query_at(
                &permit,
                &input,
                access,
                scope.session(),
                &selected,
                |_, batch| {
                    data.visit_input(&input, batch)?;
                    Ok(())
                },
            )
            .await?;
        }
        Ok(())
    })
}
impl SynthesisScopes {
    async fn prepare(
        access: &CompletedInputs,
        model: &ValidatedModel,
        session: &datafusion::prelude::SessionContext,
        parents: &[synthesis::frames::Parents],
        budget: &resources::ResourceBudget,
    ) -> Result<Self, ModelError> {
        let inputs = Data::inputs(access.profile());
        let tables = inputs
            .iter()
            .map(|input| {
                Ok(crate::consumed_rows::ClosureTable {
                    relation: model
                        .relation(input.name())
                        .ok_or(ModelError::Schema("S0 scope model relation"))?
                        .clone(),
                    alias: access.table_for(input)?,
                })
            })
            .collect::<Result<Vec<_>, ModelError>>()?;
        Self::prepare_bound(inputs, tables, session, Some(parents), budget, model).await
    }
    async fn prepare_bound(
        inputs: Vec<ValidationInput>,
        tables: Vec<crate::consumed_rows::ClosureTable>,
        session: &datafusion::prelude::SessionContext,
        parents: Option<&[synthesis::frames::Parents]>,
        budget: &resources::ResourceBudget,
        model: &ValidatedModel,
    ) -> Result<Self, ModelError> {
        let mut owner_charge = charged::StateCharge::new(budget, "synthesis-preparation-owners");
        owner_charge.grow(inputs.len() * 2048 + 65536)?;
        let inventories = synthesis_inventory_program::Inventory::ALL
            .into_iter()
            .map(|kind| {
                let _construction = budget.reserve(
                    "synthesis-inventory-construction",
                    inputs.len() * 2048 + 4096,
                )?;
                let program = synthesis_inventory_program::program(inputs.clone(), kind)?;
                Ok((
                    kind,
                    crate::scope_compilation::compile(&program, model, budget, None)?,
                ))
            })
            .collect::<Result<Vec<_>, ModelError>>()?;
        let link = typed::<catalog::CatalogMemberInvocation>(&inputs)?;
        let occurrence = typed::<source::Occurrence>(&inputs)?;
        let placement = typed::<syntax::SyntaxPlacement>(&inputs)?;
        let member = tables.len();
        let mut bindings = tables.clone();
        bindings.extend([
            tables[link].clone(),
            tables[occurrence].clone(),
            tables[placement].clone(),
        ]);
        let relations = tables
            .iter()
            .map(|table| table.relation.clone())
            .collect::<Vec<_>>();
        let program =
            catalog_scope_program::synthesis(inputs.clone(), &relations, parents, budget)?;
        let plan = crate::scope_compilation::lower_compiled(
            crate::scope_compilation::compile(program.program(), model, budget, None)?,
            &bindings,
            program.parameters(),
            budget,
        )?;
        let edges = plan.prepare(session, budget).await?;
        Ok(Self {
            inputs,
            tables,
            edges,
            member,
            inventories,
            _charge: owner_charge,
        })
    }
    fn inventory_sql(
        &self,
        kind: std::any::TypeId,
        parameters: &scope_program::ScopeParameters,
        budget: &resources::ResourceBudget,
    ) -> Result<(String, Box<dyn resources::Reservation>), ModelError> {
        let (_, program) = self
            .inventories
            .iter()
            .find(|(inventory, _)| inventory.type_id() == kind)
            .ok_or(ModelError::Schema("synthesis inventory kind"))?;
        let charge = budget.reserve(
            "synthesis-inventory-query",
            crate::scope_compilation::lowering_allowance(
                program.program(),
                &self.tables,
                parameters,
            ),
        )?;
        let queries = crate::scope_compilation::select_pair_queries(
            program.program(),
            &self.tables,
            parameters,
        )?;
        let query = queries
            .first()
            .ok_or(ModelError::Schema("synthesis inventory query"))?;
        Ok((
            format!(
                "SELECT DISTINCT target_id AS id FROM ({}) inventory ORDER BY id",
                query.2
            ),
            charge,
        ))
    }
    async fn load(
        &self,
        access: &CompletedInputs,
        scope: &crate::consumed_rows::PreparedClosure,
        phase: LoadPhase,
        budget: &resources::ResourceBudget,
    ) -> Result<Data, ModelError> {
        let declarations = match phase {
            LoadPhase::Documentary => synthesis::documentary::Data::facts_inputs(),
            LoadPhase::Member => self.inputs.clone(),
            LoadPhase::Conclusion => synthesis::production::conclusion_inputs(),
        };
        let mut consumed = crate::consumed_rows::ConsumedInputs::new(declarations, budget)?;
        let mut data = Data::new(budget);
        let mut loaders: Vec<ScopedLoader> = Vec::new();
        macro_rules! scoped {($($field:ident:$ty:ty,)*) => {$(loaders.push(load_scoped::<$ty>);)*};}
        decoder_inputs!(scoped);
        for loader in loaders {
            loader(self, access, scope, phase, &mut consumed, &mut data).await?;
        }
        consumed.finish(access.name())?;
        Ok(data)
    }
    fn partition_data(
        &self,
        batch: &crate::consumed_rows::PreparedRootBatch,
        partition: usize,
        union: &Data,
        phase: LoadPhase,
        budget: &resources::ResourceBudget,
    ) -> Result<Data, ModelError> {
        let mut selected = union.selected_copy(
            &mut |kind, epoch, key| {
                if kind == std::any::TypeId::of::<artifact::ArtifactChunk>() {
                    return Ok(false);
                }
                for (table, input) in self.inputs.iter().enumerate().filter(|(_, input)| {
                    input.type_id() == kind && (epoch.is_none() || input.prefix() == epoch)
                }) {
                    let _ = input;
                    if batch.contains(partition, table, key)? {
                        return Ok(true);
                    }
                }
                Ok(false)
            },
            budget,
        )?;
        selected.copy_text_chunks(union, phase, budget)?;
        Ok(selected)
    }
    fn chunks(
        &self,
        scope: &crate::consumed_rows::PreparedClosure,
        phase: LoadPhase,
    ) -> Result<String, ModelError> {
        use crate::consumed_rows::identifier;
        let chunks = typed::<artifact::ArtifactChunk>(&self.inputs)?;
        let occurrence = typed::<source::Occurrence>(&self.inputs)?;
        let evidence = typed::<assertion::Evidence>(&self.inputs)?;
        // The scope already selects actual documentary spans and authored statement syntax.
        // Module/function declarations are not text ranges requested by these renderers.
        let nodes = typed::<documents::DocumentNode>(&self.inputs)?;
        let demand = catalog_scope_program::synthesis_text(phase);
        let occurrences = format!(
            "SELECT source,start,\"end\" FROM ({}) WHERE syntax_kind IN ({})",
            scope.select(occurrence)?,
            demand
                .syntax_kinds
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join(",")
        );
        let spans = if demand.document_nodes_only {
            format!(
                "SELECT e.sourcespan_source AS source,e.sourcespan_start AS start,e.sourcespan_end AS \"end\" FROM ({}) e LEFT SEMI JOIN ({}) n ON e.id=n.passage_span OR e.id=n.component_span WHERE e.sourcespan_source IS NOT NULL",
                scope.select(evidence)?,
                scope.select(nodes)?
            )
        } else {
            format!(
                "SELECT sourcespan_source AS source,sourcespan_start AS start,sourcespan_end AS \"end\" FROM ({}) WHERE sourcespan_source IS NOT NULL",
                scope.select(evidence)?
            )
        };
        Ok(format!(
            "SELECT c.* FROM {} c LEFT SEMI JOIN ({occurrences} UNION ALL {spans}) r ON c.artifact=r.source AND c.ordinal*{}<r.\"end\" AND (c.ordinal+1)*{}>r.start",
            identifier(&self.tables[chunks].alias),
            artifact::ARTIFACT_CHUNK_BYTES,
            artifact::ARTIFACT_CHUNK_BYTES
        ))
    }
    async fn emission_presence(
        &self,
        batch: &crate::consumed_rows::PreparedRootBatch,
        budget: &resources::ResourceBudget,
        cancellation: &crate::workspace::Cancellation,
    ) -> Result<EmissionPresence, ModelError> {
        use futures::TryStreamExt;
        let mut found = EmissionPresence {
            keys: Default::default(),
            _charge: charged::StateCharge::new(budget, "synthesis-emission-presence"),
        };
        for kind in synthesis::production::emission_roots() {
            let table = self
                .inputs
                .iter()
                .position(|input| input.type_id() == kind)
                .ok_or(ModelError::Schema("S0 emission owner"))?;
            let mut stream = crate::sql::query(
                batch.union.session(),
                &format!(
                    "SELECT id FROM ({}) selected ORDER BY id",
                    batch.union.select(table)?
                ),
            )
            .await
            .map_err(ModelError::codec)?
            .execute_stream()
            .await
            .map_err(ModelError::codec)?;
            while let Some(rows) = stream.try_next().await.map_err(ModelError::codec)? {
                cancellation.check()?;
                let _transfer = budget.reserve(
                    "synthesis-emission-presence-transfer",
                    rows.get_array_memory_size(),
                )?;
                for row in 0..rows.num_rows() {
                    let key = crate::scoped_admission::column(&rows, "id", row)?
                        .ok_or(ModelError::Schema("S0 emission ID"))?;
                    found.keys.insert(&mut found._charge, (table, key))?;
                }
            }
        }
        Ok(found)
    }
}
struct EmissionPresence {
    keys: charged::ChargedSet<(usize, [u8; 16])>,
    _charge: charged::StateCharge,
}
impl EmissionPresence {
    fn selected(
        &self,
        batch: &crate::consumed_rows::PreparedRootBatch,
        partition: usize,
    ) -> Result<bool, ModelError> {
        for (table, key) in self.keys.iter() {
            if batch.contains(partition, *table, *key)? {
                return Ok(true);
            }
        }
        Ok(false)
    }
}
fn nominal<T>(bytes: &[u8]) -> Result<Id<T>, ModelError> {
    serde::Deserialize::deserialize(serde::de::value::SeqDeserializer::<
        _,
        serde::de::value::Error,
    >::new(bytes.iter().copied()))
    .map_err(ModelError::codec)
}

struct RankingTables<'a> {
    scopes: &'a SynthesisScopes,
    budget: &'a resources::ResourceBudget,
}
type RankingLoader = for<'a> fn(
    &'a CompletedInputs,
    &'a datafusion::prelude::SessionContext,
    &'a synthesis::frames::Parents,
    &'a RankingTables<'a>,
    &'a mut crate::consumed_rows::ConsumedInputs,
    &'a mut synthesis::automatic::Data,
) -> BoxFuture<'a, Result<(), ModelError>>;
fn load_ranking<'a, R: Record>(
    access: &'a CompletedInputs,
    session: &'a datafusion::prelude::SessionContext,
    parent: &'a synthesis::frames::Parents,
    tables: &'a RankingTables<'a>,
    consumed: &'a mut crate::consumed_rows::ConsumedInputs,
    automatic: &'a mut synthesis::automatic::Data,
) -> BoxFuture<'a, Result<(), ModelError>> {
    Box::pin(async move {
        use crate::consumed_rows::identifier;
        while let Some((input, permit)) = consumed.next::<R>(access)? {
            let alias = identifier(&access.table_for(&input)?);
            let parameters = scope_program::ScopeParameters(vec![
                scope_program::ScopeValue::Nominal(*parent.structural.bytes()),
                scope_program::ScopeValue::Nominal(*parent.analytic.bytes()),
                scope_program::ScopeValue::Nominal(*parent.summary.bytes()),
            ]);
            let (roots, _query) =
                tables
                    .scopes
                    .inventory_sql(input.type_id(), &parameters, tables.budget)?;
            let selected =
                format!("SELECT r.* FROM {alias} r WHERE r.id IN ({roots}) ORDER BY r.id");
            crate::consumed_rows::stream_query_at(
                &permit,
                &input,
                access,
                session,
                &selected,
                |_, batch| {
                    automatic.visit(input.name(), batch)?;
                    Ok(())
                },
            )
            .await?;
        }
        Ok(())
    })
}
async fn ranking(
    access: &CompletedInputs,
    session: &datafusion::prelude::SessionContext,
    parent: &synthesis::frames::Parents,
    scopes: &SynthesisScopes,
    budget: &resources::ResourceBudget,
) -> Result<(synthesis::seeds::PublicSlots, synthesis::automatic::Data), ModelError> {
    use crate::consumed_rows::identifier;
    use arrow_array::Array;
    use futures::TryStreamExt;
    let public = access.table_for(&ValidationInput::of::<structural::PublicCandidate>(&["id"]))?;
    let mut slots = synthesis::seeds::PublicSlots::new(budget);
    let parameters = scope_program::ScopeParameters(vec![
        scope_program::ScopeValue::Nominal(*parent.structural.bytes()),
        scope_program::ScopeValue::Nominal(*parent.analytic.bytes()),
        scope_program::ScopeValue::Nominal(*parent.summary.bytes()),
    ]);
    let (roots, _query) = scopes.inventory_sql(
        std::any::TypeId::of::<structural::PublicCandidate>(),
        &parameters,
        budget,
    )?;
    let mut stream = crate::sql::query(
        session,
        &format!(
            "SELECT id,frame,member,entity,in_subsystem FROM {} WHERE id IN ({roots}) ORDER BY id",
            identifier(&public)
        ),
    )
    .await
    .map_err(ModelError::codec)?
    .execute_stream()
    .await
    .map_err(ModelError::codec)?;
    while let Some(batch) = stream.try_next().await.map_err(ModelError::codec)? {
        let ids = |name| {
            batch
                .column_by_name(name)
                .and_then(|column| {
                    column
                        .as_any()
                        .downcast_ref::<arrow_array::FixedSizeBinaryArray>()
                })
                .filter(|column| column.null_count() == 0)
                .ok_or(ModelError::Schema("S0 public slot projection"))
        };
        let (id, frame, member, entity) =
            (ids("id")?, ids("frame")?, ids("member")?, ids("entity")?);
        let within = batch
            .column_by_name("in_subsystem")
            .and_then(|column| column.as_any().downcast_ref::<arrow_array::BooleanArray>())
            .filter(|column| column.null_count() == 0)
            .ok_or(ModelError::Schema("S0 public slot scope property"))?;
        for index in 0..batch.num_rows() {
            slots.observe(synthesis::seeds::PublicSlot {
                id: nominal(id.value(index))?,
                frame: nominal(frame.value(index))?,
                member: nominal(member.value(index))?,
                entity: nominal(entity.value(index))?,
                in_subsystem: within.value(index),
            })?;
        }
    }
    drop(stream);
    let mut automatic = synthesis::automatic::Data::new(budget);
    let mut consumed =
        crate::consumed_rows::ConsumedInputs::new(synthesis::automatic::Data::inputs(), budget)?;
    macro_rules! read {($($field:ident:$ty:ty,)*)=>{const LOADERS:&[RankingLoader]=&[$(load_ranking::<$ty>,)*];};}
    lctx_model::synthesis_automatic_unique_inputs!(read);
    let tables = RankingTables { scopes, budget };
    for loader in LOADERS {
        loader(
            access,
            session,
            parent,
            &tables,
            &mut consumed,
            &mut automatic,
        )
        .await?;
    }
    consumed.finish(access.name())?;
    Ok((slots, automatic))
}

fn declare_outputs(output: &ProducerOutput) -> BoxFuture<'_, Result<(), ModelError>> {
    Box::pin(async move {
        let mut declarations: Vec<Declaration> = Vec::new();
        macro_rules! common_publication {($($record:ident,)*)=>{$(declarations.push(producer_operations::declare::<analysis::synthesis::$record>);)*};}
        lctx_model::analysis_publication!(common_publication);
        macro_rules! declare {($($ty:ty),*)=>{$(declarations.push(producer_operations::declare::<$ty>);)*};}
        declare!(
            synthesis::seeds::SeedPlan,
            synthesis::seeds::ConfiguredSeedDecision,
            synthesis::seeds::ConfiguredSeedCandidate,
            synthesis::seeds::SelectedSeedSource,
            synthesis::seeds::SelectedSeed,
            synthesis::automatic::Decision,
            synthesis::summary::SummaryFacet,
            synthesis::frames::Frame,
            synthesis::assertions::ProgrammaticAssertion,
            synthesis::assertions::AssertionTemplate,
            synthesis::assertions::AssertionSource,
            synthesis::assertions::ProgrammaticAssertionSupport,
            synthesis::briefs::Brief,
            synthesis::briefs::BriefAssertion,
            synthesis::briefs::BriefSource,
            synthesis::briefs::BriefSummary,
            synthesis::briefs::BriefCodeBoundary,
            synthesis::briefs::BriefDocument,
            synthesis::briefs::BriefOmission
        );
        macro_rules! declare_rows {($($field:ident:$ty:ty,)*)=>{$(if <$ty>::NAME!=assertion::AssertionQualification::NAME{declarations.push(producer_operations::declare::<$ty>);})*};}
        lctx_model::synthesis_pattern_outputs!(declare_rows);
        lctx_model::synthesis_observation_outputs!(declare_rows);
        producer_operations::declare_ordered(output, &declarations).await
    })
}
fn publish_seed<'a>(
    rows: &'a synthesis::seeds::Output,
    output: &'a ProducerOutput,
) -> BoxFuture<'a, Result<(), ModelError>> {
    Box::pin(async move {
        type Emit = for<'a> fn(
            &'a synthesis::seeds::Output,
            &'a ProducerOutput,
        ) -> BoxFuture<'a, Result<(), ModelError>>;
        macro_rules! entries {($($field:ident),*) => {const EMITTERS: &[Emit] = &[$(|rows, output| producer_operations::emit(&rows.$field, output),)*];};}
        entries!(plans, automatic, decisions, candidates, sources, selected);
        for emit in EMITTERS {
            emit(rows, output).await?;
        }
        Ok(())
    })
}
fn publish_assertions<'a>(
    rows: &'a synthesis::assertions::Output,
    output: &'a ProducerOutput,
) -> BoxFuture<'a, Result<(), ModelError>> {
    Box::pin(async move {
        type Emit = for<'a> fn(
            &'a synthesis::assertions::Output,
            &'a ProducerOutput,
        ) -> BoxFuture<'a, Result<(), ModelError>>;
        macro_rules! entries {($($field:ident),*) => {const EMITTERS: &[Emit] = &[$(|rows, output| producer_operations::emit(&rows.$field, output),)*];};}
        entries!(assertions, templates, sources, supports);
        for emit in EMITTERS {
            emit(rows, output).await?;
        }
        Ok(())
    })
}
fn publish_briefs<'a>(
    rows: &'a synthesis::briefs::Output,
    output: &'a ProducerOutput,
) -> BoxFuture<'a, Result<(), ModelError>> {
    Box::pin(async move {
        type Emit = for<'a> fn(
            &'a synthesis::briefs::Output,
            &'a ProducerOutput,
        ) -> BoxFuture<'a, Result<(), ModelError>>;
        macro_rules! entries {($($field:ident),*) => {const EMITTERS: &[Emit] = &[$(|rows, output| producer_operations::emit(&rows.$field, output),)*];};}
        entries!(
            briefs,
            assertions,
            sources,
            summary,
            code_boundaries,
            documents,
            omissions
        );
        for emit in EMITTERS {
            emit(rows, output).await?;
        }
        Ok(())
    })
}
fn publish_patterns<'a>(
    rows: &'a synthesis::patterns::Output,
    output: &'a ProducerOutput,
) -> BoxFuture<'a, Result<(), ModelError>> {
    Box::pin(async move {
        type Emit = for<'a> fn(
            &'a synthesis::patterns::Output,
            &'a ProducerOutput,
        ) -> BoxFuture<'a, Result<(), ModelError>>;
        macro_rules! entries {($($field:ident:$ty:ty,)*) => {const EMITTERS: &[Emit] = &[$(|rows, output| producer_operations::emit(&rows.$field, output),)*];};}
        lctx_model::synthesis_pattern_outputs!(entries);
        for emit in EMITTERS {
            emit(rows, output).await?;
        }
        Ok(())
    })
}
fn publish_observations<'a>(
    rows: &'a synthesis::observations::Output,
    output: &'a ProducerOutput,
) -> BoxFuture<'a, Result<(), ModelError>> {
    Box::pin(async move {
        type Emit = for<'a> fn(
            &'a synthesis::observations::Output,
            &'a ProducerOutput,
        ) -> BoxFuture<'a, Result<(), ModelError>>;
        macro_rules! entries {($($field:ident:$ty:ty,)*) => {const EMITTERS: &[Emit] = &[$(|rows, output| producer_operations::emit(&rows.$field, output),)*];};}
        lctx_model::synthesis_observation_outputs!(entries);
        for emit in EMITTERS {
            emit(rows, output).await?;
        }
        Ok(())
    })
}
fn publish_frames<'a>(
    frames: &'a Rows<synthesis::frames::Frame>,
    sources: &'a Rows<InvocationSource>,
    inputs: &'a Rows<AnalysisInput>,
    receipts: &'a Rows<SourceReceipt>,
    output: &'a ProducerOutput,
) -> BoxFuture<'a, Result<(), ModelError>> {
    Box::pin(async move {
        producer_operations::emit(frames, output).await?;
        producer_operations::emit(sources, output).await?;
        producer_operations::emit(inputs, output).await?;
        producer_operations::emit(receipts, output).await
    })
}
fn publish_coverage<'a, 'sources>(
    invocations: &'a Rows<Invocation>,
    definition: &'a analysis::AnalysisDefinition,
    admission: &'a analysis::expected::CoverageAdmission<'sources>,
    budget: &'a resources::ResourceBudget,
    output: &'a ProducerOutput,
) -> BoxFuture<'a, Result<(), ModelError>> {
    Box::pin(async move {
        for invocation in invocations.iter() {
            let admitted = coverage::admit(
                invocation,
                definition,
                analysis::AnalysisCapability::Synthesis,
                admission,
                budget,
            )?;
            for scope in admitted.scopes() {
                let (requirement, members) = scope.expectation().records()?;
                output.push(requirement).await?;
                for row in members {
                    output.push(row).await?;
                }
                for row in scope.observations() {
                    output.push(row.source().clone()).await?;
                }
                let (coverage, members) = coverage::assess(
                    scope.expectation(),
                    scope.observations(),
                    analysis::AnalysisStatus::Completed,
                    None,
                    budget,
                )?;
                output.push(coverage).await?;
                for row in members {
                    output.push(row).await?;
                }
            }
            output
                .push(AnalysisOutcome {
                    invocation: invocation.id(),
                    status: analysis::AnalysisStatus::Completed,
                    reason: None,
                })
                .await?;
            output.push(invocation.clone()).await?;
        }
        Ok(())
    })
}
pub async fn produce(
    access: CompletedInputs,
    mut output: ProducerOutput,
    runtime: &Workspace,
    model: &Arc<ValidatedModel>,
) -> Result<(), ModelError> {
    let captured = analysis::sources::CapturedSources::capture(
        access.profile(),
        access.snapshots(),
        runtime.budget(),
    )?;
    let mut admission = analysis::expected::CoverageAdmission::new(&captured, runtime.budget())?;
    let mut data = Data::new(runtime.budget());
    let session = access.session(runtime).await?;
    let topology = synthesis::production::topology_inputs();
    let mut consumed = crate::consumed_rows::ConsumedInputs::new(
        {
            let mut inputs = topology.clone();
            inputs.extend(analysis::expected::inputs(
                analysis::AnalysisMethod::Synthesis,
            ));
            inputs
        },
        runtime.budget(),
    )?;
    read_metadata(
        &access,
        &session,
        &topology,
        &mut consumed,
        &mut admission,
        &mut data,
    )
    .await?;
    consumed.finish(access.name())?;
    let (_, definition) = synthesis::build::definition();
    let settings = data.frames.configuration()?;
    let parents = synthesis::frames::parents(&data.frames, runtime.budget())?;
    declare_outputs(&output).await?;
    let scopes =
        SynthesisScopes::prepare(&access, model, &session, &parents, runtime.budget()).await?;
    let mut documentary_spool = crate::documentary_spool::DocumentarySpool::new(runtime.budget())?;
    let mut requests = synthesis::seeds::ConfiguredRequests::new(runtime.budget());
    let mut summaries = synthesis::documentary::LiteralSummaries::new(runtime.budget());
    let mut emission = charged::ChargedSet::default();
    let mut emission_charge =
        charged::StateCharge::new(runtime.budget(), "synthesis-emission-member-identities");
    // First documentary pass publishes every boundary/conclusion and retains only exact matches,
    // literal identities and members needing later text. No upstream producer is replayed.
    synthesis_preparation::publish_documentary(
        &mut output,
        &synthesis::documentary::Output::new(runtime.budget()),
    )
    .await?;
    let links = access.table_for(&ValidationInput::of::<catalog::CatalogMemberInvocation>(&[
        "id",
    ]))?;
    let mut roots = crate::sql::query(
        &session,
        &format!(
            "SELECT id FROM {} ORDER BY id",
            crate::consumed_rows::identifier(&links)
        ),
    )
    .await
    .map_err(ModelError::codec)?
    .execute_stream()
    .await
    .map_err(ModelError::codec)?;
    use futures::TryStreamExt;
    while let Some(batch) = roots.try_next().await.map_err(ModelError::codec)? {
        let ids = batch
            .column_by_name("id")
            .and_then(|column| {
                column
                    .as_any()
                    .downcast_ref::<arrow_array::FixedSizeBinaryArray>()
            })
            .ok_or(ModelError::Schema("S0 member root identity"))?;
        for start in (0..batch.num_rows()).step_by(32) {
            let _window = runtime
                .budget()
                .reserve("synthesis-documentary-root-window", 32 * 256)?;
            let mut requests_roots = Vec::new();
            let mut root_ids = Vec::new();
            for index in start..(start + 32).min(batch.num_rows()) {
                let id: Id<catalog::CatalogMemberInvocation> = nominal(ids.value(index))?;
                root_ids.push(id);
                requests_roots.push(crate::consumed_rows::PreparedRoot {
                    table: scopes.member,
                    key: *id.bytes(),
                    kind: crate::consumed_rows::PreparedRootKind::Virtual,
                });
            }
            let selected = scopes
                .edges
                .batch_with_cancellation(&requests_roots, runtime.budget(), &runtime.cancellation())
                .await?;
            let presence = scopes
                .emission_presence(&selected, runtime.budget(), &runtime.cancellation())
                .await?;
            let union = scopes
                .load(
                    &access,
                    &selected.union,
                    LoadPhase::Documentary,
                    runtime.budget(),
                )
                .await?;
            for (partition, id) in root_ids.into_iter().enumerate() {
                runtime.cancellation().check()?;
                let grain = scopes.partition_data(
                    &selected,
                    partition,
                    &union,
                    LoadPhase::Documentary,
                    runtime.budget(),
                )?;
                let docs = synthesis::documentary::build(&grain.documentary, runtime.budget())?;
                requests.observe(&grain.documentary, settings, runtime.budget())?;
                summaries.observe(&docs)?;
                documentary_spool.append(id, &docs)?;
                if docs
                    .conclusions
                    .iter()
                    .any(|row| row.status() == analysis::policy::EvidenceStatus::Documented)
                    || presence.selected(&selected, partition)?
                {
                    emission.insert(&mut emission_charge, id)?;
                }
                synthesis_preparation::publish_documentary_grain(&output, &docs).await?;
            }
        }
    }
    drop(roots);
    let mut invocations = Rows::new(runtime.budget());
    let mut frames = Rows::new(runtime.budget());
    let mut sources = Rows::new(runtime.budget());
    let mut inputs = Rows::new(runtime.budget());
    let mut receipts = Rows::new(runtime.budget());
    let mut seeds = synthesis::seeds::Output::new(runtime.budget());
    for parent in &parents {
        let mut source_ids = charged::ChargedSet::default();
        let mut charge = charged::StateCharge::new(runtime.budget(), "synthesis-parent-identities");
        for source in &parent.sources {
            source_ids.insert(&mut charge, sources.insert(source.clone())?)?;
        }
        let (invocation, members, source_receipts, projections) = Invocation::admitted(
            parent.input,
            parent.context,
            definition.id(),
            None,
            source_ids.iter().copied(),
            &captured,
            [],
            runtime.budget(),
        )?;
        if !projections.is_empty() {
            return Err(ModelError::Invalid("S0 has no projection parameter".into()));
        }
        for row in members {
            inputs.insert(row)?;
        }
        for row in source_receipts {
            receipts.insert(row)?;
        }
        frames.insert(synthesis::frames::frame(parent, invocation.id()))?;
        let (public, automatic) =
            ranking(&access, &session, parent, &scopes, runtime.budget()).await?;
        let mut selected = synthesis::seeds::configured_indexed(
            &requests,
            &public,
            &data.frames.structural,
            &data.frames.structural_invocations,
            settings,
            &invocation,
            runtime.budget(),
        )?;
        synthesis::automatic::complete_indexed(
            &data.documentary,
            &summaries,
            &public,
            &data.frames.structural,
            &data.frames.structural_invocations,
            &automatic,
            &data.frames.analytic_parents,
            settings,
            &invocation,
            &mut selected,
            runtime.budget(),
        )?;
        // Every candidate and ranking decision is emitted; only selected navigation identities
        // and their plan survive until the member render pass.
        publish_seed(&selected, &output).await?;
        for row in selected.plans.iter() {
            seeds.plans.insert(row.clone())?;
        }
        for row in selected.selected.iter() {
            emission.insert(&mut emission_charge, row.member)?;
            seeds.selected.insert(row.clone())?;
        }
        drop(selected);
        drop(automatic);
        drop(public);
        invocations.insert(invocation)?;
    }
    synthesis::frames::verify(
        &data.frames,
        &frames,
        &invocations,
        &sources,
        &inputs,
        runtime.budget(),
    )?;
    let mut coverages = Rows::new(runtime.budget());
    for invocation in invocations.iter() {
        let admitted = coverage::admit(
            invocation,
            &definition,
            analysis::AnalysisCapability::Synthesis,
            &admission,
            runtime.budget(),
        )?;
        for scope in admitted.scopes() {
            coverages.insert(
                coverage::assess(
                    scope.expectation(),
                    scope.observations(),
                    analysis::AnalysisStatus::Completed,
                    None,
                    runtime.budget(),
                )?
                .0,
            )?;
        }
    }
    // Second pass loads only members needed by mandatory assertions/code or selected briefs.
    // Compact correspondence remains shared for independent Summary roots below.
    let mut remaining = emission.iter().copied();
    loop {
        let _window = runtime
            .budget()
            .reserve("synthesis-member-root-window", 32 * 256)?;
        let ids = remaining.by_ref().take(32).collect::<Vec<_>>();
        if ids.is_empty() {
            break;
        }
        let roots = ids
            .iter()
            .map(|id| crate::consumed_rows::PreparedRoot {
                table: scopes.member,
                key: *id.bytes(),
                kind: crate::consumed_rows::PreparedRootKind::Virtual,
            })
            .collect::<Vec<_>>();
        let selected = scopes
            .edges
            .batch_with_cancellation(&roots, runtime.budget(), &runtime.cancellation())
            .await?;
        let union = scopes
            .load(
                &access,
                &selected.union,
                LoadPhase::Member,
                runtime.budget(),
            )
            .await?;
        for (partition, id) in ids.into_iter().enumerate() {
            runtime.cancellation().check()?;
            let grain = scopes.partition_data(
                &selected,
                partition,
                &union,
                LoadPhase::Member,
                runtime.budget(),
            )?;
            let docs = documentary_spool.read(id)?;
            let assertions = synthesis::assertions::build_all(
                &grain.documentary,
                &docs,
                &grain.observations,
                &grain.controls,
                &grain.summary,
                &grain.terminal,
                &grain.patterns,
                &grain.public,
                &frames,
                &invocations,
                runtime.budget(),
            )?;
            let (facets, _) = synthesis::summary::build(
                &grain.summary,
                &grain.documentary,
                &frames,
                &invocations,
                runtime.budget(),
            )?;
            let patterns = synthesis::patterns::build(
                &grain.patterns,
                &grain.documentary,
                &frames,
                &invocations,
                runtime.budget(),
            )?;
            let mut selected = synthesis::seeds::Output::new(runtime.budget());
            for row in seeds.selected.iter().filter(|row| row.member == id) {
                selected.selected.insert(row.clone())?;
                selected.plans.insert(
                    seeds
                        .plans
                        .get(row.plan)
                        .ok_or(ModelError::Schema("S0 selected plan absent"))?
                        .clone(),
                )?;
            }
            let briefs = synthesis::briefs::build_with_summary(
                &grain.documentary,
                &docs,
                &assertions,
                &selected,
                &grain.summary,
                &grain.observations.qualifications,
                &facets,
                &frames,
                &patterns,
                runtime.budget(),
            )?;
            publish_assertions(&assertions, &output).await?;
            publish_briefs(&briefs, &output).await?;
            publish_patterns(&patterns, &output).await?;
            drop(briefs);
            drop(selected);
            drop(patterns);
            drop(facets);
            drop(assertions);
            drop(docs);
            drop(grain);
        }
    }
    drop(documentary_spool);
    drop(emission);
    drop(emission_charge);
    drop(requests);
    drop(summaries);
    drop(seeds);
    // Independent conclusion roots keep unlinked consequences. Summary linkage uses complete
    // compact member/entity topology, so a local missing member never invents an unlinked facet.
    for frame in frames.iter() {
        for kind in synthesis::production::conclusion_roots() {
            let root = scopes
                .inputs
                .iter()
                .position(|input| input.type_id() == kind)
                .ok_or(ModelError::Schema("S0 conclusion root"))?;
            let parameters = scope_program::ScopeParameters(vec![
                scope_program::ScopeValue::Nominal(*frame.structural.bytes()),
                scope_program::ScopeValue::Nominal(*frame.analytic.bytes()),
                scope_program::ScopeValue::Nominal(*frame.summary.bytes()),
            ]);
            let (roots_sql, _query) = scopes.inventory_sql(kind, &parameters, runtime.budget())?;
            let mut roots = crate::sql::query(&session, &roots_sql)
                .await
                .map_err(ModelError::codec)?
                .execute_stream()
                .await
                .map_err(ModelError::codec)?;
            while let Some(batch) = roots.try_next().await.map_err(ModelError::codec)? {
                let ids = batch
                    .column_by_name("id")
                    .and_then(|column| {
                        column
                            .as_any()
                            .downcast_ref::<arrow_array::FixedSizeBinaryArray>()
                    })
                    .ok_or(ModelError::Schema("S0 conclusion identity"))?;
                for start in (0..batch.num_rows()).step_by(32) {
                    let _window = runtime
                        .budget()
                        .reserve("synthesis-conclusion-root-window", 32 * 256)?;
                    let requested = (start..(start + 32).min(batch.num_rows()))
                        .map(|index| {
                            let key: [u8; 16] =
                                ids.value(index).try_into().map_err(ModelError::codec)?;
                            Ok(crate::consumed_rows::PreparedRoot {
                                table: root,
                                key,
                                kind: crate::consumed_rows::PreparedRootKind::Physical,
                            })
                        })
                        .collect::<Result<Vec<_>, ModelError>>()?;
                    let selected = scopes
                        .edges
                        .batch_with_cancellation(
                            &requested,
                            runtime.budget(),
                            &runtime.cancellation(),
                        )
                        .await?;
                    let union = scopes
                        .load(
                            &access,
                            &selected.union,
                            LoadPhase::Conclusion,
                            runtime.budget(),
                        )
                        .await?;
                    for partition in 0..requested.len() {
                        runtime.cancellation().check()?;
                        let grain = scopes.partition_data(
                            &selected,
                            partition,
                            &union,
                            LoadPhase::Conclusion,
                            runtime.budget(),
                        )?;
                        let observations = synthesis::observations::build_all(
                            &grain.observations,
                            &grain.summary,
                            &data.documentary,
                            &frames,
                            &invocations,
                            &coverages,
                            runtime.budget(),
                        )?;
                        let (facets, _) = synthesis::summary::build(
                            &grain.summary,
                            &data.documentary,
                            &frames,
                            &invocations,
                            runtime.budget(),
                        )?;
                        producer_operations::emit(&facets, &output).await?;
                        publish_observations(&observations, &output).await?;
                        drop(facets);
                        drop(observations);
                        drop(grain);
                    }
                }
            }
        }
    }
    publish_frames(&frames, &sources, &inputs, &receipts, &output).await?;
    publish_coverage(
        &invocations,
        &definition,
        &admission,
        runtime.budget(),
        &output,
    )
    .await?;
    drop(coverages);
    drop(scopes);
    drop(receipts);
    drop(inputs);
    drop(sources);
    drop(frames);
    drop(invocations);
    drop(parents);
    drop(data);
    drop(admission);
    drop(captured);
    output.finish(ProviderOutcome::Complete).await
}

#[cfg(test)]
mod decoder_tests {
    use super::*;
    #[test]
    fn declared_sources_have_decoder_reachability_in_both_profiles() {
        let mut decoders = std::collections::BTreeSet::new();
        macro_rules! collect {($($field:ident:$ty:ty,)*)=>{$(decoders.insert(std::any::TypeId::of::<$ty>());)*};}
        decoder_inputs!(collect);
        for profile in Profile::ALL {
            crate::consumed_rows::assert_decoder_reachability(
                Data::consumed_inputs(profile),
                &decoders,
            );
        }
    }
    fn id<T>(byte: u8) -> Id<T> {
        nominal(&[byte; 16]).unwrap()
    }
    fn fixture_data(
        b: &resources::ResourceBudget,
    ) -> (
        synthesis::documentary::Data,
        Id<catalog::CatalogMemberInvocation>,
    ) {
        let mut data = synthesis::documentary::Data::new(b);
        let text = "\"Run.\"";
        let artifact =
            source::SourceArtifact::from_bytes(id(1), "api.py".into(), text.as_bytes()).unwrap();
        data.artifacts.insert(artifact.clone()).unwrap();
        for row in artifact::ArtifactChunk::split(&artifact, text.as_bytes()).unwrap() {
            data.chunks.insert(row).unwrap();
        }
        let module = data
            .modules
            .insert(source::Module {
                source: artifact.id(),
                qualified_name: "pkg.api".into(),
            })
            .unwrap();
        let q = assertion::AssertionQualification {
            assumptions: assumptions::AssumptionSet::empty_id(),
            context: id(2),
            scope: source::CoverageScope::Artifact {
                artifact: artifact.id(),
            }
            .id(),
            condition: conditions::Diagram::always().id(),
            modality: attribution::Modality::Definite,
            approximation: assertion::Approximation::Exact,
        };
        data.qualifications.insert(q.clone()).unwrap();
        let mut occurrence = |kind, path| {
            data.occurrences
                .insert(source::Occurrence {
                    source: artifact.id(),
                    start: 0,
                    end: text.len() as i64,
                    syntax_kind: kind,
                    role: source::OccurrenceRole::Syntax,
                    structural_path: path,
                })
                .unwrap()
        };
        let declaration = occurrence(source::SyntaxKind::StmtFunctionDef, vec![]);
        let docstring = occurrence(source::SyntaxKind::StmtExpr, vec![0]);
        let literal_occurrence = occurrence(source::SyntaxKind::ExprStringLiteral, vec![0, 0]);
        let member = data
            .members
            .insert(catalog::CatalogMember {
                input: artifact.input,
                access: module,
                path: vec!["run".into()],
                name: "run".into(),
            })
            .unwrap();
        let core =
            analysis::catalog_core::Invocation::new(artifact.input, q.context, id(3), None, []).0;
        data.core_invocations.insert(core.clone()).unwrap();
        let frame = data
            .member_frames
            .insert(catalog::CatalogMemberInvocation {
                member,
                invocation: core.id(),
            })
            .unwrap();
        let public = data
            .public_exposures
            .insert(normalized::entities::PublicExposure {
                access: module,
                context: q.context,
                observation: id(4),
                origin: id(5),
                enumeration: None,
                publicity: normalized::entities::PublicPathKnowledge::Known,
                status: normalized::entities::ResolutionStatus::Unresolved,
                reason: normalized::entities::EntityReason::UntracedExposure,
            })
            .unwrap();
        let exposure = data
            .exposures
            .insert(catalog::CatalogExposure {
                member,
                exposure: public,
            })
            .unwrap();
        let callable = data
            .callables
            .insert(normalized::entities::CallableEntity::Source {
                declaration,
                kind: normalized::entities::CallableKind::Function,
            })
            .unwrap();
        let entity = data
            .refs
            .insert(normalized::entities::EntityRef::Callable { callable })
            .unwrap();
        let candidate = data
            .entity_candidates
            .insert(normalized::entities::SymbolEntityCandidate {
                resolution: id(6),
                entity,
            })
            .unwrap();
        data.candidates
            .insert(catalog::CatalogCandidate {
                exposure,
                candidate: Some(id(7)),
                entity: Some(candidate),
                path: None,
                alias: None,
            })
            .unwrap();
        let row = syntax::DeclarationObservation {
            qualification: q.id(),
            declaration,
            name: id(8),
            kind: syntax::DeclarationKind::Function,
            parent: None,
            overload: false,
            docstring: Some(docstring),
        };
        data.declarations.insert(row.clone()).unwrap();
        let placement = syntax::SyntaxPlacement {
            qualification: q.id(),
            occurrence: literal_occurrence,
            parent: Some(docstring),
            field: lexical::SyntaxField::Value,
            ordinal: 0,
        };
        data.placements.insert(placement.clone()).unwrap();
        let literal = data
            .literals
            .insert(value::Literal::String {
                value: "Run.".into(),
            })
            .unwrap();
        let detail = data
            .detail_values
            .insert(syntax::SyntaxDetail::Literal { literal })
            .unwrap();
        let detail = syntax::SyntaxDetailObservation {
            qualification: q.id(),
            occurrence: literal_occurrence,
            ordinal: 0,
            detail,
        };
        data.details.insert(detail.clone()).unwrap();
        for premise in [
            analysis::native::NativeAssertionPremise::DeclarationObservation {
                assertion: row.id(),
                support: id(9),
            },
            analysis::native::NativeAssertionPremise::SyntaxPlacement {
                assertion: placement.id(),
                support: id(10),
            },
            analysis::native::NativeAssertionPremise::SyntaxDetailObservation {
                assertion: detail.id(),
                support: id(11),
            },
        ] {
            data.native_qualifications
                .insert(analysis::native::NativeQualification {
                    premise: premise.id(),
                    qualification: q.id(),
                    family: premise.family(),
                    fidelity: attribution::Fidelity::NativeStructural,
                    status: analysis::policy::native_status(
                        premise.family(),
                        attribution::Fidelity::NativeStructural,
                    ),
                })
                .unwrap();
            data.native.insert(premise).unwrap();
        }
        (data, frame)
    }
    #[tokio::test]
    async fn member_documentary_batch_preserves_actual_oracle_and_excludes_unrelated_labels_and_chunks()
     {
        use crate::consumed_rows::ClosureTable;
        use datafusion::{datasource::MemTable, prelude::SessionContext};
        use futures::TryStreamExt;
        let budget = resources::ResourceBudget::fixed(8 << 20).unwrap();
        let model = lctx_model::domain::model().unwrap();
        let inputs = Data::inputs(Profile::Catalog);
        let session = SessionContext::new();
        let tables = inputs
            .iter()
            .enumerate()
            .map(|(index, input)| {
                let relation = model.relation(input.name()).unwrap().clone();
                let alias = format!("synthesis_fixture_{index}");
                let batch = arrow_array::RecordBatch::new_empty(relation.schema().clone());
                session
                    .register_table(
                        alias.as_str(),
                        Arc::new(MemTable::try_new(batch.schema(), vec![vec![batch]]).unwrap()),
                    )
                    .unwrap();
                ClosureTable { relation, alias }
            })
            .collect::<Vec<_>>();
        let finite_budget = resources::ResourceBudget::fixed(8 << 20).unwrap();
        let (data, member) = fixture_data(&finite_budget);
        let expected = synthesis::documentary::build(&data, &finite_budget).unwrap();
        assert_eq!(expected.conclusions.len(), 1);
        macro_rules! install {($($field:ident:$ty:ty,)*)=>{$(
            let input=synthesis::documentary::Data::facts_inputs().into_iter().find(|input|input.type_id()==std::any::TypeId::of::<$ty>()).unwrap();let index=inputs.iter().position(|candidate|candidate.type_id()==input.type_id()&&candidate.prefix()==input.prefix()).unwrap();
            let mut rows=data.$field.iter().cloned().collect::<Vec<_>>();
            let batch=<$ty as Record>::encode(&rows).unwrap();session.deregister_table(tables[index].alias.as_str()).unwrap();session.register_table(tables[index].alias.as_str(),Arc::new(MemTable::try_new(batch.schema(),vec![vec![batch]]).unwrap())).unwrap();rows.clear();
        )*};}
        lctx_model::synthesis_documentary_inputs!(install);
        let actual = data.members.iter().next().unwrap();
        let foreign = catalog::CatalogMember {
            path: vec!["x".repeat(16 << 20)],
            name: "y".repeat(16 << 20),
            ..actual.clone()
        };
        let index = typed::<catalog::CatalogMember>(&inputs).unwrap();
        let batch =
            <catalog::CatalogMember as Record>::encode(&[actual.clone(), foreign.clone()]).unwrap();
        session
            .deregister_table(tables[index].alias.as_str())
            .unwrap();
        session
            .register_table(
                tables[index].alias.as_str(),
                Arc::new(MemTable::try_new(batch.schema(), vec![vec![batch]]).unwrap()),
            )
            .unwrap();
        let source = data.artifacts.iter().next().unwrap();
        let foreign_source =
            source::SourceArtifact::from_bytes(source.input, "z".repeat(16 << 20), b"unrelated")
                .unwrap();
        let index = typed::<source::SourceArtifact>(&inputs).unwrap();
        let batch =
            <source::SourceArtifact as Record>::encode(&[source.clone(), foreign_source.clone()])
                .unwrap();
        session
            .deregister_table(tables[index].alias.as_str())
            .unwrap();
        session
            .register_table(
                tables[index].alias.as_str(),
                Arc::new(MemTable::try_new(batch.schema(), vec![vec![batch]]).unwrap()),
            )
            .unwrap();
        let index = typed::<artifact::ArtifactChunk>(&inputs).unwrap();
        let mut chunks = data.chunks.iter().cloned().collect::<Vec<_>>();
        chunks.push(artifact::ArtifactChunk {
            artifact: foreign_source.id(),
            ordinal: 0,
            body: EvidenceBytes(b"not a requested original".to_vec()),
        });
        let batch = <artifact::ArtifactChunk as Record>::encode(&chunks).unwrap();
        session
            .deregister_table(tables[index].alias.as_str())
            .unwrap();
        session
            .register_table(
                tables[index].alias.as_str(),
                Arc::new(MemTable::try_new(batch.schema(), vec![vec![batch]]).unwrap()),
            )
            .unwrap();
        let scopes =
            SynthesisScopes::prepare_bound(inputs, tables, &session, None, &budget, &model)
                .await
                .unwrap();
        let root = crate::consumed_rows::PreparedRoot {
            table: scopes.member,
            key: *member.bytes(),
            kind: crate::consumed_rows::PreparedRootKind::Virtual,
        };
        let selected_batch = scopes.edges.batch(&[root, root], &budget).await.unwrap();
        let mut union = Data::new(&budget);
        let mut decoded_chunks = 0;
        for input in synthesis::documentary::Data::facts_inputs() {
            let index = scopes
                .inputs
                .iter()
                .position(|candidate| {
                    candidate.type_id() == input.type_id() && candidate.prefix() == input.prefix()
                })
                .unwrap();
            let query = if input.type_id() == std::any::TypeId::of::<artifact::ArtifactChunk>() {
                scopes
                    .chunks(&selected_batch.union, LoadPhase::Documentary)
                    .unwrap()
            } else {
                selected_batch.union.select(index).unwrap()
            };
            let mut stream = crate::sql::query(selected_batch.union.session(), &query)
                .await
                .unwrap()
                .execute_stream()
                .await
                .unwrap();
            while let Some(batch) = stream.try_next().await.unwrap() {
                if input.type_id() == std::any::TypeId::of::<artifact::ArtifactChunk>() {
                    decoded_chunks += batch.num_rows();
                }
                union.visit_input(&input, &batch).unwrap();
            }
        }
        assert_eq!(
            decoded_chunks, 1,
            "overlapping roots share original chunk hydration"
        );
        for partition in 0..2 {
            let selected = scopes
                .partition_data(
                    &selected_batch,
                    partition,
                    &union,
                    LoadPhase::Documentary,
                    &budget,
                )
                .unwrap();
            assert_eq!(selected.documentary.members.len(), 1);
            assert!(selected.documentary.members.get(foreign.id()).is_none());
            assert_eq!(selected.documentary.artifacts.len(), 1);
            assert!(
                selected
                    .documentary
                    .artifacts
                    .get(foreign_source.id())
                    .is_none()
            );
            assert_eq!(selected.documentary.chunks.len(), 1);
            let actual = synthesis::documentary::build(&selected.documentary, &budget).unwrap();
            actual.matches(&expected).unwrap();
        }
        drop(union);
        drop(selected_batch);
        drop(scopes);
        assert_eq!(budget.reserved(), 0);
    }
    #[tokio::test]
    async fn control_formal_grain_selects_named_parameter_links_without_unrelated_parameters() {
        use crate::consumed_rows::ClosureTable;
        use datafusion::{datasource::MemTable, prelude::SessionContext};
        use futures::TryStreamExt;
        use normalized::entities::{ParameterEntity, ParameterEntityLink};
        fn install<R: Record>(
            session: &SessionContext,
            tables: &[ClosureTable],
            inputs: &[ValidationInput],
            rows: &[R],
        ) {
            let index = inputs
                .iter()
                .position(|input| input.type_id() == std::any::TypeId::of::<R>())
                .unwrap();
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
        let budget = resources::ResourceBudget::fixed(8 << 20).unwrap();
        let model = lctx_model::domain::model().unwrap();
        let inputs = Data::inputs(Profile::Catalog);
        let session = SessionContext::new();
        let tables = inputs
            .iter()
            .enumerate()
            .map(|(index, input)| {
                let relation = model.relation(input.name()).unwrap().clone();
                let alias = format!("control_parameter_fixture_{index}");
                let batch = arrow_array::RecordBatch::new_empty(relation.schema().clone());
                session
                    .register_table(
                        alias.as_str(),
                        Arc::new(MemTable::try_new(batch.schema(), vec![vec![batch]]).unwrap()),
                    )
                    .unwrap();
                ClosureTable { relation, alias }
            })
            .collect::<Vec<_>>();
        let formal = ParameterEntity::Source {
            declaration: nominal(&[1; 16]).unwrap(),
        };
        let unrelated = ParameterEntity::Source {
            declaration: nominal(&[2; 16]).unwrap(),
        };
        let shapes = [
            calls::ParameterShape {
                name: Some("flag".into()),
                kind: calls::ParameterKind::KeywordOnly,
                required: false,
            },
            calls::ParameterShape {
                name: Some("unrelated".into()),
                kind: calls::ParameterKind::KeywordOnly,
                required: false,
            },
        ];
        let parameters = [
            calls::SignatureParameter {
                signature: nominal(&[3; 16]).unwrap(),
                ordinal: 0,
                shape: shapes[0].id(),
            },
            calls::SignatureParameter {
                signature: nominal(&[4; 16]).unwrap(),
                ordinal: 0,
                shape: shapes[1].id(),
            },
        ];
        let links = [
            ParameterEntityLink {
                parameter: parameters[0].id(),
                entity: formal.id(),
                declaration: None,
            },
            ParameterEntityLink {
                parameter: parameters[1].id(),
                entity: unrelated.id(),
                declaration: None,
            },
        ];
        let path = structural::controls::ControlPath {
            traversal: nominal(&[5; 16]).unwrap(),
            target: nominal(&[6; 16]).unwrap(),
            formal: formal.id(),
            may_suppress: false,
            length: 0,
        };
        install(&session, &tables, &inputs, &[formal, unrelated]);
        install(&session, &tables, &inputs, &shapes);
        install(&session, &tables, &inputs, &parameters);
        install(&session, &tables, &inputs, &links);
        install(&session, &tables, &inputs, std::slice::from_ref(&path));
        let scopes =
            SynthesisScopes::prepare_bound(inputs, tables, &session, None, &budget, &model)
                .await
                .unwrap();
        let root = typed::<structural::controls::ControlPath>(&scopes.inputs).unwrap();
        let scope = scopes
            .edges
            .grain(root, &format!("id=X'{}'", path.id().hex()), &budget)
            .await
            .unwrap();
        let mut selected_links = Rows::<ParameterEntityLink>::new(&budget);
        let mut selected_shapes = Rows::<calls::ParameterShape>::new(&budget);
        for (index, is_link) in [
            (typed::<ParameterEntityLink>(&scopes.inputs).unwrap(), true),
            (
                typed::<calls::ParameterShape>(&scopes.inputs).unwrap(),
                false,
            ),
        ] {
            let mut stream = crate::sql::query(scope.session(), &scope.select(index).unwrap())
                .await
                .unwrap()
                .execute_stream()
                .await
                .unwrap();
            while let Some(batch) = stream.try_next().await.unwrap() {
                if is_link {
                    selected_links.decode(&batch).unwrap();
                } else {
                    selected_shapes.decode(&batch).unwrap();
                }
            }
        }
        assert_eq!(selected_links.len(), 1);
        assert_eq!(selected_links.get(links[0].id()), Some(&links[0]));
        assert!(selected_links.get(links[1].id()).is_none());
        assert_eq!(selected_shapes.len(), 1);
        assert_eq!(selected_shapes.get(shapes[0].id()), Some(&shapes[0]));
        drop(selected_links);
        drop(selected_shapes);
        drop(scope);
        drop(scopes);
        assert_eq!(budget.reserved(), 0);
    }

    #[tokio::test]
    async fn expected_artifact_property_query_preserves_exact_extension_policy() {
        use datafusion::{datasource::MemTable, prelude::SessionContext};
        use futures::TryStreamExt;
        let session = SessionContext::new();
        let paths = [
            "x.py",
            "x.pyi",
            "x.md",
            "x.mdx",
            "x.rst",
            "x.PY",
            "no_extension",
            ".py",
            "..md",
            "Unicode_π.md",
            "wrong.py.long",
            "dot.py/folder",
        ];
        let mut rows = paths
            .into_iter()
            .map(|path| source::SourceArtifact::from_bytes(id(1), path.into(), b"x").unwrap())
            .collect::<Vec<_>>();
        rows.push(
            source::SourceArtifact::from_bytes(id(1), format!("{}.pyi", "λ".repeat(1 << 20)), b"x")
                .unwrap(),
        );
        let batch = <source::SourceArtifact as Record>::encode(&rows).unwrap();
        session
            .register_table(
                "captured_artifacts",
                Arc::new(MemTable::try_new(batch.schema(), vec![vec![batch]]).unwrap()),
            )
            .unwrap();
        let mut stream = crate::sql::query(
            &session,
            &format!(
                "SELECT {} FROM captured_artifacts ORDER BY id",
                analysis::expected::CoverageAdmission::artifact_property_columns()
            ),
        )
        .await
        .unwrap()
        .execute_stream()
        .await
        .unwrap();
        let mut checked = 0;
        while let Some(batch) = stream.try_next().await.unwrap() {
            let ids = batch
                .column_by_name("id")
                .unwrap()
                .as_any()
                .downcast_ref::<arrow_array::FixedSizeBinaryArray>()
                .unwrap();
            let suffix = batch
                .column_by_name("path_suffix")
                .unwrap()
                .as_any()
                .downcast_ref::<arrow_array::StringArray>()
                .unwrap();
            for index in 0..batch.num_rows() {
                let id: Id<source::SourceArtifact> = nominal(ids.value(index)).unwrap();
                let original = rows.iter().find(|row| row.id() == id).unwrap();
                assert!(suffix.value(index).chars().count() <= 4);
                assert_eq!(
                    admission::ArtifactClass::of(suffix.value(index)),
                    admission::ArtifactClass::of(&original.path)
                );
                checked += 1;
            }
        }
        assert_eq!(checked, rows.len());
    }
}
