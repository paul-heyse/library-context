//! C2 declaration domains consume completed C0/C1 receipts independently of optional behavior.
use crate::consumed_rows::{ClosureTable, NominalClosure, PreparedEdges, identifier};
use crate::workspace::{CompletedInputs, ProducerOutput, Workspace};
use datafusion::execution::context::SessionContext;
use futures::TryStreamExt;
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
async fn load<R: Record>(
    access: &CompletedInputs,
    session: &SessionContext,
    consumed: &mut crate::consumed_rows::ConsumedInputs,
    admission: &mut analysis::expected::CoverageAdmission<'_>,
    mut visit: impl FnMut(&ValidationInput, &arrow_array::RecordBatch) -> Result<(), ModelError>,
) -> Result<(), ModelError> {
    while let Some((input, permit)) = consumed.next::<R>(access)? {
        if crate::consumed_rows::stream_artifact_admission(access, &input, session, admission)
            .await?
        {
            continue;
        }
        crate::consumed_rows::stream_at(&permit, &input, access, session, |permit, batch| {
            admission.visit_if_expected(permit, batch)?;
            visit(&input, batch)
        })
        .await?;
    }
    Ok(())
}
struct SelectionScopes {
    inputs: Vec<ValidationInput>,
    edges: PreparedEdges,
    member: usize,
}
fn typed<R: Record>(inputs: &[ValidationInput]) -> Result<usize, ModelError> {
    inputs
        .iter()
        .position(|input| input.type_id() == TypeId::of::<R>())
        .ok_or(ModelError::Schema("C2 declared scope relation absent"))
}
fn target(
    inputs: &[ValidationInput],
    source: usize,
    kind: TypeId,
) -> Result<Option<usize>, ModelError> {
    let candidates: Vec<_> = inputs
        .iter()
        .enumerate()
        .filter(|(_, input)| input.type_id() == kind)
        .map(|(index, _)| index)
        .collect();
    if candidates.len() <= 1 {
        return Ok(candidates.first().copied());
    }
    let prefix = if kind == TypeId::of::<assertion::AssertionQualification>() {
        Some(
            if inputs[source].type_id() == TypeId::of::<local_fields::FieldLocation>() {
                stages::PublicationBoundary::Local
            } else {
                stages::PublicationBoundary::Facts
            },
        )
    } else {
        inputs[source].prefix()
    };
    let selected: Vec<_> = candidates
        .into_iter()
        .filter(|index| inputs[*index].prefix() == prefix)
        .collect();
    if selected.len() != 1 {
        return Err(ModelError::Conflict("C2 dependency immutable epoch"));
    }
    Ok(selected.first().copied())
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
        Self::prepare_bound(inputs, tables, session, budget).await
    }
    async fn prepare_bound(
        inputs: Vec<ValidationInput>,
        tables: Vec<ClosureTable>,
        session: &SessionContext,
        budget: &resources::ResourceBudget,
    ) -> Result<Self, ModelError> {
        let member_table = typed::<catalog::CatalogMember>(&inputs)?;
        let member_link = typed::<catalog::CatalogMemberInvocation>(&inputs)?;
        let core_invocation = typed::<analysis::catalog_core::Invocation>(&inputs)?;
        let occurrence = typed::<source::Occurrence>(&inputs)?;
        let mut bindings = tables.clone();
        let member = bindings.len();
        bindings.push(tables[member_link].clone());
        let head = bindings.len();
        bindings.push(tables[occurrence].clone());
        let mut plan = NominalClosure::new(bindings)?;
        plan.pairs(
            member,
            member_link,
            format!(
                "SELECT id AS source_id,id AS target_id FROM {}",
                identifier(&tables[member_link].alias)
            ),
        )?;
        let memberships = build::memberships();
        for (source, table) in tables.iter().enumerate() {
            for field in table.relation.fields() {
                let Some((kind, _)) = field.target() else {
                    continue;
                };
                let Some(to) = target(&inputs, source, kind)? else {
                    continue;
                };
                let values = if field.list() {
                    format!("UNNEST({})", identifier(field.name()))
                } else {
                    identifier(field.name())
                };
                plan.pairs(
                    source,
                    to,
                    format!(
                        "SELECT id AS source_id,{values} AS target_id FROM {}",
                        identifier(&table.alias)
                    ),
                )?;
                if memberships.contains(&(table.relation.type_id(), field.name()))
                    && table.relation.type_id()
                        != TypeId::of::<catalog::evidence::FieldAccessAssessment>()
                {
                    if kind == TypeId::of::<catalog::CatalogMember>() {
                        if table.relation.type_id()
                            == TypeId::of::<catalog::CatalogMemberInvocation>()
                        {
                            continue;
                        }
                        let (joins, predicate) = if table.relation.type_id()
                            == TypeId::of::<catalog::CatalogExposure>()
                        {
                            (
                                format!(
                                    " JOIN {} e ON r.exposure=e.id",
                                    identifier(
                                        &tables[typed::<normalized::entities::PublicExposure>(
                                            &inputs
                                        )?]
                                        .alias
                                    )
                                ),
                                "e.context=i.context",
                            )
                        } else if table.relation.type_id()
                            == TypeId::of::<catalog::CatalogCallable>()
                        {
                            (
                                format!(
                                    " JOIN {} a ON r.assessment=a.id",
                                    identifier(
                                        &tables[typed::<
                                            normalized::callables::EffectiveCallableAssessment,
                                        >(&inputs)?]
                                        .alias
                                    )
                                ),
                                "a.context=i.context",
                            )
                        } else if table.relation.type_id()
                            == TypeId::of::<catalog::evidence::ScenarioAssociation>()
                        {
                            let q = target(
                                &inputs,
                                source,
                                TypeId::of::<assertion::AssertionQualification>(),
                            )?
                            .ok_or(ModelError::Schema("C2 scenario context"))?;
                            (
                                format!(
                                    " JOIN {} q ON r.qualification=q.id",
                                    identifier(&tables[q].alias)
                                ),
                                "q.context=i.context",
                            )
                        } else if table.relation.type_id()
                            == TypeId::of::<catalog::evidence::DocumentAssociation>()
                        {
                            let candidate =
                                typed::<normalized::links::MentionEntityCandidate>(&inputs)?;
                            let assessment =
                                typed::<normalized::links::MentionEntityAssessment>(&inputs)?;
                            let mention = typed::<documents::DocumentMentionObservation>(&inputs)?;
                            let q = target(
                                &inputs,
                                mention,
                                TypeId::of::<assertion::AssertionQualification>(),
                            )?
                            .ok_or(ModelError::Schema("C2 mention context"))?;
                            (
                                format!(
                                    " JOIN {} c ON r.candidate=c.id JOIN {} a ON c.assessment=a.id JOIN {} o ON a.observation=o.id JOIN {} q ON o.qualification=q.id",
                                    identifier(&tables[candidate].alias),
                                    identifier(&tables[assessment].alias),
                                    identifier(&tables[mention].alias),
                                    identifier(&tables[q].alias)
                                ),
                                "q.context=i.context",
                            )
                        } else {
                            (String::new(), "TRUE")
                        };
                        plan.pairs(member,source,format!("SELECT l.id AS source_id,r.id AS target_id FROM {} l JOIN {} i ON l.invocation=i.id JOIN {} r ON r.{}=l.member{joins} WHERE {predicate}",identifier(&tables[member_link].alias),identifier(&tables[core_invocation].alias),identifier(&table.alias),identifier(field.name())))?;
                    } else {
                        plan.pairs(
                            to,
                            source,
                            format!(
                                "SELECT {} AS source_id,id AS target_id FROM {}",
                                identifier(field.name()),
                                identifier(&table.alias)
                            ),
                        )?;
                    }
                }
            }
        }
        // Coverage closure includes every fixed C0/C1 receipt in this member's input/context,
        // including scopes with no declaration roots. The model filters the exact context.
        for kind in [
            TypeId::of::<analysis::catalog_core::Invocation>(),
            TypeId::of::<analysis::catalog_evidence::Invocation>(),
        ] {
            let to = inputs
                .iter()
                .position(|input| input.type_id() == kind)
                .ok_or(ModelError::Schema("C2 fixed frame scope"))?;
            plan.pairs(member,to,format!("SELECT l.id AS source_id,i.id AS target_id FROM {} l JOIN {} m ON l.member=m.id JOIN {} c ON l.invocation=c.id JOIN {} i ON m.input=i.input AND c.context=i.context",identifier(&tables[member_link].alias),identifier(&tables[member_table].alias),identifier(&tables[core_invocation].alias),identifier(&tables[to].alias)))?;
        }
        for kind in [
            TypeId::of::<analysis::catalog_core::AnalysisCoverage>(),
            TypeId::of::<analysis::catalog_evidence::AnalysisCoverage>(),
        ] {
            let from = inputs
                .iter()
                .position(|input| input.type_id() == kind)
                .ok_or(ModelError::Schema("C2 coverage scope"))?;
            let field = tables[from]
                .relation
                .fields()
                .iter()
                .find(|field| field.name() == "invocation")
                .ok_or(ModelError::Schema("C2 coverage invocation"))?;
            let to = target(
                &inputs,
                from,
                field
                    .target()
                    .ok_or(ModelError::Schema("C2 nominal coverage invocation"))?
                    .0,
            )?
            .ok_or(ModelError::Schema("C2 coverage invocation owner"))?;
            plan.own(from, "invocation", to)?;
        }
        let access = typed::<catalog::evidence::FieldAccessAssessment>(&inputs)?;
        let option = typed::<catalog::CatalogOption>(&inputs)?;
        let q = target(
            &inputs,
            access,
            TypeId::of::<assertion::AssertionQualification>(),
        )?
        .ok_or(ModelError::Schema("C2 access context"))?;
        plan.pairs(member,access,format!("SELECT l.id AS source_id,a.id AS target_id FROM {} l JOIN {} i ON l.invocation=i.id JOIN {} o ON o.member=l.member JOIN {} a ON a.option=o.id JOIN {} q ON a.qualification=q.id WHERE q.context=i.context",identifier(&tables[member_link].alias),identifier(&tables[core_invocation].alias),identifier(&tables[option].alias),identifier(&tables[access].alias),identifier(&tables[q].alias)))?;
        // Decorator head syntax is its own namespace. Following a parent into ordinary syntax
        // must not open the enclosing declaration's body or sibling statements.
        let decorator = typed::<syntax::DeclarationDecorator>(&inputs)?;
        plan.pairs(
            decorator,
            head,
            format!(
                "SELECT id AS source_id,decorator AS target_id FROM {}",
                identifier(&tables[decorator].alias)
            ),
        )?;
        plan.pairs(
            head,
            occurrence,
            format!(
                "SELECT id AS source_id,id AS target_id FROM {}",
                identifier(&tables[occurrence].alias)
            ),
        )?;
        let placement = typed::<syntax::SyntaxPlacement>(&inputs)?;
        let syntax = format!(
            "SELECT parent AS source_id,id AS target_id FROM {} WHERE field IN ({},{})",
            identifier(&tables[placement].alias),
            lexical::SyntaxField::Child as i16,
            lexical::SyntaxField::Callee as i16
        );
        plan.pairs(head, placement, syntax)?;
        plan.pairs(
            placement,
            head,
            format!(
                "SELECT id AS source_id,occurrence AS target_id FROM {} WHERE field IN ({},{})",
                identifier(&tables[placement].alias),
                lexical::SyntaxField::Child as i16,
                lexical::SyntaxField::Callee as i16
            ),
        )?;
        let edges = plan.prepare(session, budget).await?;
        Ok(Self {
            inputs,
            edges,
            member,
        })
    }
    async fn load(
        &self,
        access: &CompletedInputs,
        scope: &crate::consumed_rows::PreparedClosure,
        budget: &resources::ResourceBudget,
    ) -> Result<Data, ModelError> {
        let mut data = Data::new(budget);
        let mut consumed = crate::consumed_rows::ConsumedInputs::new(self.inputs.clone(), budget)?;
        macro_rules! scoped {($($field:ident:$ty:ty,)*)=>{$(while let Some((input,permit))=consumed.next::<$ty>(access)?{
            let table=self.inputs.iter().position(|candidate|candidate.type_id()==input.type_id() && candidate.prefix()==input.prefix()).ok_or(ModelError::Conflict("C2 scoped declaration"))?;
            crate::consumed_rows::stream_query_at(&permit,&input,scope.session(),&scope.select(table)?,|_,batch|{data.visit_input(&input,batch)?;Ok(())}).await?;
        })*};}
        decoder_inputs!(scoped);
        consumed.finish(access.name())?;
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
    macro_rules! inventory {($($field:ident:$ty:ty,)*)=>{$(load::<$ty>(&access,&session,&mut consumed,&mut admission,|input,batch|{
        frames.visit(input.name(),batch)?;
        if input.name()==analysis::AnalysisDefinition::NAME {definitions.decode(batch)?;}
        if input.name()==analysis::MethodParameters::NAME {parameters.decode(batch)?;}
        Ok(())
    }).await?;)*};}
    decoder_inputs!(inventory);
    consumed.finish(access.name())?;
    let (expected_parameters, definition) = build::definition();
    if definitions.get(definition.id()) != Some(&definition)
        || parameters.get(expected_parameters.id()) != Some(&expected_parameters)
    {
        return Err(ModelError::Invalid(
            "C2 requires its completed authored definition".into(),
        ));
    }
    macro_rules! declare_outputs {($($f:ident:$ty:ty,)*)=>{$(output.declare::<$ty>()?;)*};}
    lctx_model::catalog_selection_outputs!(declare_outputs);
    macro_rules! declare {($($ty:ty),*)=>{$(output.declare::<$ty>()?;)*};}
    macro_rules! common_publication {($($record:ident,)*)=>{$(output.declare::<analysis::selection::$record>()?;)*};}
    lctx_model::analysis_publication!(common_publication);
    declare!(selection::SelectionInvocation);
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
        let admitted = coverage::admit(
            &invocation,
            &definition,
            analysis::AnalysisCapability::CatalogSelection,
            &admission,
            runtime.budget(),
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
                runtime.budget(),
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
        for index in 0..batch.num_rows() {
            runtime.cancellation().check()?;
            let member = nominal(members.value(index))?;
            let context = nominal(contexts.value(index))?;
            let scope = scopes
                .edges
                .grain(
                    scopes.member,
                    &format!(
                        "id=X'{}'",
                        roots
                            .value(index)
                            .iter()
                            .map(|byte| format!("{byte:02x}"))
                            .collect::<String>()
                    ),
                    runtime.budget(),
                )
                .await?;
            let data = scopes.load(&access, &scope, runtime.budget()).await?;
            drop(scope);
            let rows = build::member(&data, member, context, runtime.budget())?;
            let links = selection::frames::links(
                &rows,
                &invocations,
                &data.source.catalog.members,
                runtime.budget(),
            )?;
            macro_rules! write {($($f:ident:$ty:ty,)*)=>{$(for row in rows.$f.iter(){output.push(row.clone()).await?;})*};}
            lctx_model::catalog_selection_outputs!(write);
            for row in links.iter() {
                output.push(row.clone()).await?;
            }
            drop(links);
            drop(rows);
            drop(data);
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
            for index in 0..batch.num_rows() {
                runtime.cancellation().check()?;
                let predicate = format!(
                    "id=X'{}'",
                    ids.value(index)
                        .iter()
                        .map(|byte| format!("{byte:02x}"))
                        .collect::<String>()
                );
                let scope = scopes
                    .edges
                    .grain(root, &predicate, runtime.budget())
                    .await?;
                let data = scopes.load(&access, &scope, runtime.budget()).await?;
                drop(scope);
                let rows = build::witnesses(&data, runtime.budget())?;
                for row in rows.witnesses.iter() {
                    output.push(row.clone()).await?;
                }
                drop(rows);
                drop(data);
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
    async fn member_grain_union_matches_finite_domains_and_excludes_unrelated_rich_members() {
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
        let scopes = SelectionScopes::prepare_bound(inputs, tables, &session, &budget)
            .await
            .unwrap();
        let mut actual = build::Output::new(&budget);
        for (link, invocation) in links.iter().zip(&core) {
            let scope = scopes
                .edges
                .grain(
                    scopes.member,
                    &format!("id=X'{}'", link.id().hex()),
                    &budget,
                )
                .await
                .unwrap();
            let data = hydrate(&scopes, &scope, &budget).await;
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
        let scopes = SelectionScopes::prepare_bound(inputs, tables, &session, &budget)
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
