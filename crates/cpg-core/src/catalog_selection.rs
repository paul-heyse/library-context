//! C2 declaration domains consume completed C0/C1 receipts independently of optional behavior.
use crate::consumed_rows::{ClosureTable, PreparedEdges, identifier};
use crate::producer_operations::{self, Declaration};
use crate::workspace::{CompletedInputs, ProducerOutput, Workspace};
use datafusion::execution::context::SessionContext;
use futures::{TryStreamExt, future::BoxFuture};
use lctx_model::domain::stages::ProviderOutcome;
use lctx_model::domain::{
    analysis::{self, selection::*},
    normalized::Rows,
    selection::{
        self,
        build::{self, Data},
    },
    *,
};
use std::{any::TypeId, sync::Arc};
// The semantic owner declares exact views; these macros provide typed decoders.
macro_rules! decoder_inputs {
    ($apply:ident) => {
        lctx_model::catalog_inputs!($apply);
        lctx_model::catalog_outputs!($apply);
        lctx_model::catalog_evidence_inputs!($apply);
        lctx_model::catalog_evidence_outputs!($apply);
        lctx_model::catalog_selection_inputs!($apply);
        lctx_model::catalog_runtime_inputs!($apply);
        $apply! {definitions:analysis::AnalysisDefinition,parameters:analysis::MethodParameters,}
        lctx_model::expected_domain_inputs!($apply);
    };
}
type InventoryLoader = for<'a, 'sources> fn(
    &'a CompletedInputs,
    &'a SessionContext,
    &'a mut crate::consumed_rows::ConsumedInputs,
    &'a mut analysis::expected::CoverageAdmission<'sources>,
    &'a mut selection::frames::Frames,
    &'a mut Rows<analysis::AnalysisDefinition>,
    &'a mut Rows<analysis::MethodParameters>,
) -> BoxFuture<'a, Result<(), ModelError>>;
fn load<'a, 'sources, R: Record>(
    access: &'a CompletedInputs,
    session: &'a SessionContext,
    consumed: &'a mut crate::consumed_rows::ConsumedInputs,
    admission: &'a mut analysis::expected::CoverageAdmission<'sources>,
    frames: &'a mut selection::frames::Frames,
    definitions: &'a mut Rows<analysis::AnalysisDefinition>,
    parameters: &'a mut Rows<analysis::MethodParameters>,
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
                frames.visit(input.name(), batch)?;
                if input.name() == analysis::AnalysisDefinition::NAME {
                    definitions.decode(batch)?;
                }
                if input.name() == analysis::MethodParameters::NAME {
                    parameters.decode(batch)?;
                }
                Ok(())
            })
            .await?;
        }
        Ok(())
    })
}
fn load_inventory<'a, 'sources>(
    access: &'a CompletedInputs,
    session: &'a SessionContext,
    consumed: &'a mut crate::consumed_rows::ConsumedInputs,
    admission: &'a mut analysis::expected::CoverageAdmission<'sources>,
    frames: &'a mut selection::frames::Frames,
    definitions: &'a mut Rows<analysis::AnalysisDefinition>,
    parameters: &'a mut Rows<analysis::MethodParameters>,
) -> BoxFuture<'a, Result<(), ModelError>> {
    Box::pin(async move {
        let mut loaders: Vec<InventoryLoader> = Vec::new();
        macro_rules! inventory {($($field:ident:$ty:ty,)*) => {$(loaders.push(load::<$ty>);)*};}
        decoder_inputs!(inventory);
        for loader in loaders {
            loader(
                access,
                session,
                consumed,
                admission,
                frames,
                definitions,
                parameters,
            )
            .await?;
        }
        Ok(())
    })
}
fn declare_outputs(output: &ProducerOutput) -> BoxFuture<'_, Result<(), ModelError>> {
    Box::pin(async move {
        let mut declarations: Vec<Declaration> = Vec::new();
        macro_rules! declare {($($field:ident:$ty:ty,)*) => {$(declarations.push(producer_operations::declare::<$ty>);)*};}
        lctx_model::catalog_selection_outputs!(declare);
        macro_rules! common {($($record:ident,)*) => {$(declarations.push(producer_operations::declare::<analysis::selection::$record>);)*};}
        lctx_model::analysis_publication!(common);
        declarations.push(producer_operations::declare::<selection::SelectionInvocation>);
        producer_operations::declare_ordered(output, &declarations).await
    })
}
fn publish_selection<'a>(
    rows: &'a build::Output,
    output: &'a ProducerOutput,
) -> BoxFuture<'a, Result<(), ModelError>> {
    Box::pin(async move {
        type Emit = for<'a> fn(
            &'a build::Output,
            &'a ProducerOutput,
        ) -> BoxFuture<'a, Result<(), ModelError>>;
        macro_rules! entries {($($field:ident:$ty:ty,)*) => {const EMITTERS: &[Emit] = &[$(|rows, output| producer_operations::emit(&rows.$field, output),)*];};}
        lctx_model::catalog_selection_outputs!(entries);
        for emit in EMITTERS {
            emit(rows, output).await?;
        }
        Ok(())
    })
}

struct SelectionScopes {
    inputs: Vec<ValidationInput>,
    edges: PreparedEdges,
    member: usize,
    _charge: charged::StateCharge,
}
fn typed<R: Record>(inputs: &[ValidationInput]) -> Result<usize, ModelError> {
    inputs
        .iter()
        .position(|input| input.type_id() == TypeId::of::<R>())
        .ok_or(ModelError::Schema("C2 declared scope relation absent"))
}
impl SelectionScopes {
    async fn prepare(
        access: &CompletedInputs,
        model: &ValidatedModel,
        session: &SessionContext,
        budget: &resources::ResourceBudget,
    ) -> Result<Self, ModelError> {
        let inputs = Data::inputs();
        let tables: Vec<_> = inputs
            .iter()
            .map(|input| {
                Ok(ClosureTable {
                    relation: model
                        .relation(input.name())
                        .ok_or(ModelError::Schema("C2 input model relation"))?
                        .clone(),
                    alias: access.table_for(input)?,
                })
            })
            .collect::<Result<_, ModelError>>()?;
        Self::prepare_bound(inputs, tables, session, budget, model).await
    }
    async fn prepare_bound(
        inputs: Vec<ValidationInput>,
        tables: Vec<ClosureTable>,
        session: &SessionContext,
        budget: &resources::ResourceBudget,
        model: &ValidatedModel,
    ) -> Result<Self, ModelError> {
        let mut charge = charged::StateCharge::new(budget, "C2-scope-descriptors");
        charge.grow(inputs.capacity() * 512)?;
        let member_link = typed::<catalog::CatalogMemberInvocation>(&inputs)?;
        let occurrence = typed::<source::Occurrence>(&inputs)?;
        let member = tables.len();
        let mut bindings = tables.clone();
        bindings.extend([tables[member_link].clone(), tables[occurrence].clone()]);
        let relations = tables
            .iter()
            .map(|table| table.relation.clone())
            .collect::<Vec<_>>();
        let program = catalog_scope_program::selection(inputs.clone(), &relations, budget)?;
        let plan = crate::scope_compilation::lower_compiled(
            crate::scope_compilation::compile(program.program(), model, budget, None)?,
            &bindings,
            &scope_program::ScopeParameters(vec![]),
            budget,
        )?;
        let edges = plan.prepare(session, budget).await?;
        Ok(Self {
            inputs,
            edges,
            member,
            _charge: charge,
        })
    }
    async fn union(
        &self,
        selected: &crate::consumed_rows::PreparedRootBatch,
        runtime: &Workspace,
    ) -> Result<Data, ModelError> {
        let mut data = Data::new(runtime.budget());
        crate::scoped_batch::hydrate_union(
            selected,
            &self.inputs,
            runtime.budget(),
            &runtime.cancellation(),
            &mut |_, input, batch| data.visit_input(input, batch).map(|_| ()),
        )
        .await?;
        Ok(data)
    }
}
fn nominal<T>(bytes: &[u8]) -> Result<Id<T>, ModelError> {
    serde::Deserialize::deserialize(serde::de::value::SeqDeserializer::<
        _,
        serde::de::value::Error,
    >::new(bytes.iter().copied()))
    .map_err(ModelError::codec)
}
fn publish_coverage<'a, 'sources>(
    invocation: &'a Invocation,
    definition: &'a analysis::AnalysisDefinition,
    admission: &'a analysis::expected::CoverageAdmission<'sources>,
    budget: &'a resources::ResourceBudget,
    output: &'a ProducerOutput,
) -> BoxFuture<'a, Result<(), ModelError>> {
    Box::pin(async move {
        let admitted = coverage::admit(
            invocation,
            definition,
            analysis::AnalysisCapability::CatalogSelection,
            admission,
            budget,
        )?;
        for scope in admitted.scopes() {
            let (requirement, members) = scope.expectation().records()?;
            output.push(requirement).await?;
            for member in members {
                output.push(member).await?;
            }
            for observed in scope.observations() {
                output.push(observed.source().clone()).await?;
            }
            let (coverage, members) = coverage::assess(
                scope.expectation(),
                scope.observations(),
                analysis::AnalysisStatus::Completed,
                None,
                budget,
            )?;
            output.push(coverage).await?;
            for member in members {
                output.push(member).await?;
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
        Ok(())
    })
}
type MemberDemand = (
    crate::consumed_rows::PreparedRoot,
    Id<catalog::CatalogMember>,
    Id<attribution::AnalysisContext>,
);
async fn member_batch(
    scopes: &SelectionScopes,
    window: &[MemberDemand],
    runtime: &Workspace,
    output: &ProducerOutput,
    invocations: &Rows<Invocation>,
) -> Result<(), ModelError> {
    let _roots = runtime.budget().reserve(
        "C2-member-root-transfer",
        window.len() * size_of::<crate::consumed_rows::PreparedRoot>(),
    )?;
    let roots = window.iter().map(|(root, _, _)| *root).collect::<Vec<_>>();
    let selected = scopes
        .edges
        .batch_with_cancellation(&roots, runtime.budget(), &runtime.cancellation())
        .await?;
    let union = scopes.union(&selected, runtime).await?;
    for (partition, (_, member, context)) in window.iter().enumerate() {
        runtime.cancellation().check()?;
        let data = union.selected_copy(
            &scopes.inputs,
            &mut |table, key| selected.contains(partition, table, key),
            runtime.budget(),
        )?;
        let rows = build::member(&data, *member, *context, runtime.budget())?;
        let links = selection::frames::links(
            &rows,
            invocations,
            &data.source.catalog.members,
            runtime.budget(),
        )?;
        publish_selection(&rows, output).await?;
        producer_operations::emit(&links, output).await?;
        tokio::task::yield_now().await;
    }
    Ok(())
}
async fn witness_batch(
    scopes: &SelectionScopes,
    roots: &[crate::consumed_rows::PreparedRoot],
    runtime: &Workspace,
    output: &ProducerOutput,
) -> Result<(), ModelError> {
    let selected = scopes
        .edges
        .batch_with_cancellation(roots, runtime.budget(), &runtime.cancellation())
        .await?;
    let union = scopes.union(&selected, runtime).await?;
    for partition in 0..roots.len() {
        runtime.cancellation().check()?;
        let data = union.selected_copy(
            &scopes.inputs,
            &mut |table, key| selected.contains(partition, table, key),
            runtime.budget(),
        )?;
        let rows = build::witnesses(&data, runtime.budget())?;
        producer_operations::emit(&rows.witnesses, output).await?;
        tokio::task::yield_now().await;
    }
    Ok(())
}
pub async fn produce(
    access: CompletedInputs,
    output: ProducerOutput,
    runtime: &Workspace,
    model: &Arc<ValidatedModel>,
) -> Result<(), ModelError> {
    let sources = analysis::sources::CapturedSources::capture(
        access.profile(),
        access.snapshots(),
        runtime.budget(),
    )?;
    let mut admission = analysis::expected::CoverageAdmission::new(&sources, runtime.budget())?;
    let session = access.session(runtime).await?;
    let mut frames = selection::frames::Frames::new(runtime.budget());
    let mut definitions = Rows::<analysis::AnalysisDefinition>::new(runtime.budget());
    let mut parameters = Rows::<analysis::MethodParameters>::new(runtime.budget());
    let mut consumed = crate::consumed_rows::ConsumedInputs::new(
        {
            let mut inputs = selection::frames::Frames::inputs();
            inputs.extend([
                ValidationInput::of::<analysis::AnalysisDefinition>(&["id"]),
                ValidationInput::of::<analysis::MethodParameters>(&["id"]),
            ]);
            inputs.extend(analysis::expected::inputs(build::definition().1.method));
            inputs
        },
        runtime.budget(),
    )?;
    load_inventory(
        &access,
        &session,
        &mut consumed,
        &mut admission,
        &mut frames,
        &mut definitions,
        &mut parameters,
    )
    .await?;
    consumed.finish(access.name())?;
    let (expected_parameters, definition) = build::definition();
    if definitions.get(definition.id()) != Some(&definition)
        || parameters.get(expected_parameters.id()) != Some(&expected_parameters)
    {
        return Err(ModelError::Invalid(
            "C2 requires its completed authored definition".into(),
        ));
    }
    declare_outputs(&output).await?;
    let mut invocations = Rows::new(runtime.budget());
    let expected_parents = frames.parents(runtime.budget())?;
    for parent in expected_parents.iter() {
        let source = InvocationSource::CatalogEvidence {
            invocation: parent.id(),
        };
        let (invocation, parents, receipts, projections) = Invocation::admitted(
            parent.input,
            parent.context,
            definition.id(),
            None,
            [source.id()],
            &sources,
            [],
            runtime.budget(),
        )?;
        if !projections.is_empty() {
            return Err(ModelError::Invalid(
                "C2 has no projection requirement".into(),
            ));
        }
        output.push(source).await?;
        for row in parents {
            output.push(row).await?;
        }
        for receipt in receipts {
            output.push(receipt).await?;
        }
        publish_coverage(
            &invocation,
            &definition,
            &admission,
            runtime.budget(),
            &output,
        )
        .await?;
        invocations.insert(invocation)?;
    }
    drop(expected_parents);
    drop(frames);
    drop(definitions);
    drop(parameters);
    let scopes = SelectionScopes::prepare(&access, model, &session, runtime.budget()).await?;
    let links = access.table_for(&ValidationInput::of::<catalog::CatalogMemberInvocation>(&[
        "id",
    ]))?;
    let core = access.table_for(&ValidationInput::of::<analysis::catalog_core::Invocation>(
        &["id"],
    ))?;
    let mut roots=crate::sql::query(&session,&format!("SELECT l.id,l.member,i.context FROM {} l LEFT JOIN {} i ON l.invocation=i.id ORDER BY l.id",identifier(&links),identifier(&core))).await.map_err(ModelError::codec)?.execute_stream().await.map_err(ModelError::codec)?;
    while let Some(batch) = roots.try_next().await.map_err(ModelError::codec)? {
        use arrow_array::Array;
        let roots = batch
            .column_by_name("id")
            .and_then(|a| {
                a.as_any()
                    .downcast_ref::<arrow_array::FixedSizeBinaryArray>()
            })
            .ok_or(ModelError::Schema("C2 member invocation roots"))?;
        let members = batch
            .column_by_name("member")
            .and_then(|a| {
                a.as_any()
                    .downcast_ref::<arrow_array::FixedSizeBinaryArray>()
            })
            .filter(|a| a.null_count() == 0)
            .ok_or(ModelError::Schema("C2 member roots"))?;
        let contexts = batch
            .column_by_name("context")
            .and_then(|a| {
                a.as_any()
                    .downcast_ref::<arrow_array::FixedSizeBinaryArray>()
            })
            .filter(|a| a.null_count() == 0)
            .ok_or(ModelError::Invalid("C2 member has no C0 invocation".into()))?;
        let _window = runtime
            .budget()
            .reserve("C2-member-window", 32 * size_of::<MemberDemand>())?;
        let mut window = Vec::with_capacity(32);
        for index in 0..batch.num_rows() {
            runtime.cancellation().check()?;
            let root = crate::consumed_rows::PreparedRoot {
                table: scopes.member,
                key: roots.value(index).try_into().map_err(ModelError::codec)?,
                kind: crate::consumed_rows::PreparedRootKind::Virtual,
            };
            window.push((
                root,
                nominal(members.value(index))?,
                nominal(contexts.value(index))?,
            ));
            if window.len() == 32 {
                member_batch(&scopes, &window, runtime, &output, &invocations).await?;
                window.clear();
            }
        }
        if !window.is_empty() {
            member_batch(&scopes, &window, runtime, &output, &invocations).await?;
        }
    }
    drop(roots);
    for kind in build::witness_roots() {
        let root = scopes
            .inputs
            .iter()
            .position(|input| input.type_id() == kind)
            .ok_or(ModelError::Schema("C2 witness root relation absent"))?;
        let alias = access.table_for(&scopes.inputs[root])?;
        let mut roots = crate::sql::query(
            &session,
            &format!("SELECT id FROM {} ORDER BY id", identifier(&alias)),
        )
        .await
        .map_err(ModelError::codec)?
        .execute_stream()
        .await
        .map_err(ModelError::codec)?;
        while let Some(batch) = roots.try_next().await.map_err(ModelError::codec)? {
            let ids = batch
                .column_by_name("id")
                .and_then(|a| {
                    a.as_any()
                        .downcast_ref::<arrow_array::FixedSizeBinaryArray>()
                })
                .ok_or(ModelError::Schema("C2 witness identity"))?;
            let _window = runtime.budget().reserve(
                "C2-witness-window",
                32 * size_of::<crate::consumed_rows::PreparedRoot>(),
            )?;
            let mut window = Vec::with_capacity(32);
            for index in 0..batch.num_rows() {
                runtime.cancellation().check()?;
                window.push(crate::consumed_rows::PreparedRoot {
                    table: root,
                    key: ids.value(index).try_into().map_err(ModelError::codec)?,
                    kind: crate::consumed_rows::PreparedRootKind::Physical,
                });
                if window.len() == 32 {
                    witness_batch(&scopes, &window, runtime, &output).await?;
                    window.clear();
                }
            }
            if !window.is_empty() {
                witness_batch(&scopes, &window, runtime, &output).await?;
            }
        }
    }
    drop(scopes);
    drop(session);
    drop(invocations);
    drop(admission);
    drop(sources);
    output.finish(ProviderOutcome::Complete).await
}

#[cfg(test)]
mod tests {
    use super::*;
    fn consumed_inputs(profile: stages::Profile) -> Vec<ValidationInput> {
        let mut declarations = Data::consumed_inputs(profile);
        declarations.extend([
            ValidationInput::of::<analysis::AnalysisDefinition>(&["id"]),
            ValidationInput::of::<analysis::MethodParameters>(&["id"]),
        ]);
        declarations.extend(analysis::expected::inputs(build::definition().1.method));
        declarations
    }

    #[test]
    fn declared_views_have_profile_decoder_reachability() {
        for profile in [stages::Profile::Catalog, stages::Profile::Behavioral] {
            let mut decoders = std::collections::BTreeSet::new();
            macro_rules! inventory {($($field:ident:$ty:ty,)*)=>{$(decoders.insert(std::any::TypeId::of::<$ty>());)*};}
            decoder_inputs!(inventory);
            crate::consumed_rows::assert_decoder_reachability(consumed_inputs(profile), &decoders);
        }
    }
    fn id<R>(byte: u8) -> Id<R> {
        nominal(&[byte; 16]).unwrap()
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
                let alias = format!("selection_fixture_{index}");
                let batch = arrow_array::RecordBatch::new_empty(relation.schema().clone());
                session
                    .register_table(
                        alias.as_str(),
                        Arc::new(
                            datafusion::datasource::MemTable::try_new(
                                batch.schema(),
                                vec![vec![batch]],
                            )
                            .unwrap(),
                        ),
                    )
                    .unwrap();
                ClosureTable { relation, alias }
            })
            .collect();
        (session, inputs, tables)
    }
    fn install<R: Record>(
        session: &SessionContext,
        tables: &[ClosureTable],
        inputs: &[ValidationInput],
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
                Arc::new(
                    datafusion::datasource::MemTable::try_new(batch.schema(), vec![vec![batch]])
                        .unwrap(),
                ),
            )
            .unwrap();
    }
    async fn hydrate(
        scopes: &SelectionScopes,
        scope: &crate::consumed_rows::PreparedClosure,
        budget: &resources::ResourceBudget,
    ) -> Data {
        let mut data = Data::new(budget);
        for (index, input) in scopes.inputs.iter().enumerate() {
            let mut stream = crate::sql::query(scope.session(), &scope.select(index).unwrap())
                .await
                .unwrap()
                .execute_stream()
                .await
                .unwrap();
            while let Some(batch) = stream.try_next().await.unwrap() {
                data.visit_input(input, &batch).unwrap();
            }
        }
        data
    }
    #[tokio::test]
    async fn member_batch_decodes_overlap_once_matches_finite_domains_and_excludes_unrelated_members()
     {
        let budget = resources::ResourceBudget::fixed(8 << 20).unwrap();
        let (session, inputs, tables) = fixture();
        let artifact = source::SourceArtifact::from_bytes(id(1), "api.py".into(), b"x").unwrap();
        let module = source::Module {
            source: artifact.id(),
            qualified_name: "api".into(),
        };
        let members: Vec<_> = ["a", "b"]
            .into_iter()
            .map(|name| catalog::CatalogMember {
                input: artifact.input,
                access: module.id(),
                path: vec![name.into()],
                name: name.into(),
            })
            .collect();
        // This shares the actual module and input. Nominal references must not open it as a root.
        let sibling = catalog::CatalogMember {
            input: artifact.input,
            access: module.id(),
            path: vec!["z".repeat(16 << 20)],
            name: "z".repeat(16 << 20),
        };
        let core: Vec<_> = [id(2), id(3), id(2)]
            .into_iter()
            .map(|context| {
                analysis::catalog_core::Invocation::new(
                    artifact.input,
                    context,
                    catalog::build::definition().1.id(),
                    None,
                    [],
                )
                .0
            })
            .collect();
        let links: Vec<_> = [0, 0, 1]
            .into_iter()
            .zip(&core)
            .map(|(member, invocation)| catalog::CatalogMemberInvocation {
                member: members[member].id(),
                invocation: invocation.id(),
            })
            .collect();
        let exposures: Vec<_> = [id(2), id(3)]
            .into_iter()
            .map(|context| normalized::entities::PublicExposure {
                access: module.id(),
                context,
                observation: id(8),
                origin: id(9),
                enumeration: None,
                publicity: normalized::entities::PublicPathKnowledge::Unknown,
                status: normalized::entities::ResolutionStatus::Unresolved,
                reason: normalized::entities::EntityReason::MissingDeclaration,
            })
            .collect();
        let exposed: Vec<_> = exposures
            .iter()
            .map(|exposure| catalog::CatalogExposure {
                member: members[0].id(),
                exposure: exposure.id(),
            })
            .collect();
        let candidates: Vec<_> = exposed
            .iter()
            .map(|exposure| catalog::CatalogCandidate {
                exposure: exposure.id(),
                candidate: None,
                entity: None,
                path: None,
                alias: None,
            })
            .collect();
        let original = catalog::evidence::OriginalSource::Artifact {
            artifact: artifact.id(),
        };
        install(&session, &tables, &inputs, std::slice::from_ref(&artifact));
        install(&session, &tables, &inputs, std::slice::from_ref(&module));
        install(
            &session,
            &tables,
            &inputs,
            &[members[0].clone(), members[1].clone(), sibling.clone()],
        );
        install(&session, &tables, &inputs, &exposures);
        install(&session, &tables, &inputs, &exposed);
        install(&session, &tables, &inputs, &candidates);
        install(&session, &tables, &inputs, &core[..2]);
        install(&session, &tables, &inputs, &links);
        install(&session, &tables, &inputs, std::slice::from_ref(&original));
        let mut all = Data::new(&budget);
        all.source.core.artifacts.insert(artifact).unwrap();
        all.source.core.modules.insert(module).unwrap();
        all.evidence.original_sources.insert(original).unwrap();
        for row in &members {
            all.source.catalog.members.insert(row.clone()).unwrap();
        }
        for row in &core {
            all.source
                .facts
                .core_invocations
                .insert(row.clone())
                .unwrap();
        }
        for row in &links {
            all.source.facts.core_links.insert(row.clone()).unwrap();
        }
        for row in &exposures {
            all.source.core.exposures.insert(row.clone()).unwrap();
        }
        for row in &exposed {
            all.source.catalog.exposures.insert(row.clone()).unwrap();
        }
        for row in &candidates {
            all.source.catalog.candidates.insert(row.clone()).unwrap();
        }
        let expected = build::build(&all, &budget).unwrap();
        drop(all);
        let scopes =
            SelectionScopes::prepare_bound(inputs, tables, &session, &budget, &model().unwrap())
                .await
                .unwrap();
        let roots = links
            .iter()
            .map(|link| crate::consumed_rows::PreparedRoot {
                table: scopes.member,
                key: *link.id().bytes(),
                kind: crate::consumed_rows::PreparedRootKind::Virtual,
            })
            .collect::<Vec<_>>();
        let selected = scopes.edges.batch(&roots, &budget).await.unwrap();
        let mut union = Data::new(&budget);
        let mut artifact_decodes = 0;
        crate::scoped_batch::hydrate_union(
            &selected,
            &scopes.inputs,
            &budget,
            &crate::workspace::Cancellation::default(),
            &mut |_, input, batch| {
                if input.name() == source::SourceArtifact::NAME {
                    artifact_decodes += batch.num_rows();
                }
                union.visit_input(input, batch).map(|_| ())
            },
        )
        .await
        .unwrap();
        assert_eq!(
            artifact_decodes, 1,
            "shared module artifact hydrated once across contexts and members"
        );
        let mut actual = build::Output::new(&budget);
        for (partition, (link, invocation)) in links.iter().zip(&core).enumerate() {
            let data = union
                .selected_copy(
                    &scopes.inputs,
                    &mut |table, key| selected.contains(partition, table, key),
                    &budget,
                )
                .unwrap();
            assert_eq!(data.source.catalog.members.len(), 1);
            assert!(data.source.catalog.members.get(sibling.id()).is_none());
            assert_eq!(data.source.facts.core_links.len(), 1);
            assert!(
                data.source
                    .core
                    .exposures
                    .iter()
                    .all(|exposure| exposure.context == invocation.context)
            );
            let rows = build::member(&data, link.member, invocation.context, &budget).unwrap();
            macro_rules! merge {($($field:ident:$ty:ty,)*)=>{$(for row in rows.$field.iter(){actual.$field.insert(row.clone()).unwrap();})*};}
            lctx_model::catalog_selection_outputs!(merge);
        }
        actual.matches(&expected).unwrap();
        assert_eq!(actual.domains.len(), 21);
        drop(actual);
        drop(expected);
        drop(union);
        drop(selected);
        drop(scopes);
        assert_eq!(budget.reserved(), 0);
    }
    #[tokio::test]
    async fn native_typing_witness_without_public_member_is_preserved() {
        let budget = resources::ResourceBudget::fixed(8 << 20).unwrap();
        let (session, inputs, tables) = fixture();
        let artifact =
            source::SourceArtifact::from_bytes(id(1), "private.py".into(), b"x").unwrap();
        let coverage_scope = source::CoverageScope::Artifact {
            artifact: artifact.id(),
        };
        let coverage = attribution::ProviderCoverage {
            scope: coverage_scope.id(),
            provider: Some(id(2)),
            context: id(3),
            family: attribution::FactFamily::Types,
            run: Some(id(4)),
            status: attribution::CoverageStatus::Failed,
            reason: Some(obligation::ObligationKind::SyntaxError),
            diagnostic: None,
        };
        install(&session, &tables, &inputs, std::slice::from_ref(&artifact));
        install(
            &session,
            &tables,
            &inputs,
            std::slice::from_ref(&coverage_scope),
        );
        install(&session, &tables, &inputs, std::slice::from_ref(&coverage));
        let mut all = Data::new(&budget);
        all.source.core.artifacts.insert(artifact).unwrap();
        all.source
            .core
            .native_coverage
            .insert(coverage.clone())
            .unwrap();
        let expected = build::build(&all, &budget).unwrap();
        drop(all);
        let root = typed::<attribution::ProviderCoverage>(&inputs).unwrap();
        let scopes =
            SelectionScopes::prepare_bound(inputs, tables, &session, &budget, &model().unwrap())
                .await
                .unwrap();
        let scope = scopes
            .edges
            .grain(root, &format!("id=X'{}'", coverage.id().hex()), &budget)
            .await
            .unwrap();
        let data = hydrate(&scopes, &scope, &budget).await;
        let actual = build::witnesses(&data, &budget).unwrap();
        actual.matches(&expected).unwrap();
        assert_eq!(actual.witnesses.len(), 1);
        assert!(actual.domains.is_empty());
        drop(actual);
        drop(expected);
        drop(data);
        drop(scope);
        drop(scopes);
        assert_eq!(budget.reserved(), 0);
    }
    #[test]
    fn member_kernel_refuses_redirected_c0_input() {
        let budget = resources::ResourceBudget::fixed(1 << 20).unwrap();
        let mut data = Data::new(&budget);
        let member = catalog::CatalogMember {
            input: id(1),
            access: id(2),
            path: vec!["x".into()],
            name: "x".into(),
        };
        let invocation = analysis::catalog_core::Invocation::new(
            id(3),
            id(4),
            catalog::build::definition().1.id(),
            None,
            [],
        )
        .0;
        data.source
            .facts
            .core_links
            .insert(catalog::CatalogMemberInvocation {
                member: member.id(),
                invocation: invocation.id(),
            })
            .unwrap();
        data.source
            .facts
            .core_invocations
            .insert(invocation.clone())
            .unwrap();
        data.source.catalog.members.insert(member.clone()).unwrap();
        assert!(build::member(&data, member.id(), invocation.context, &budget).is_err());
        drop(data);
        assert_eq!(budget.reserved(), 0);
    }
}
