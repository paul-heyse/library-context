//! E0 selects actual C1 roots and canonical brief documents before decoding their rich premises.
use crate::{
    consumed_rows::{ClosureTable, NominalClosure, PreparedClosure, PreparedEdges, identifier},
    workspace::{CompletedInputs, ProducerOutput, Workspace},
};
use datafusion::prelude::SessionContext;
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
    let selected: Vec<_> = candidates
        .into_iter()
        .filter(|index| inputs[*index].prefix() == inputs[source].prefix())
        .collect();
    if selected.len() != 1 {
        return Err(ModelError::Conflict("E0 immutable dependency epoch"));
    }
    Ok(selected.first().copied())
}
/// Only this metadata is shared between rendering grains. Originals and output text are released
/// by the caller after publishing each root or brief.
pub struct Preparation {
    pub metadata: Data,
    session: SessionContext,
    inputs: Vec<ValidationInput>,
    tables: Vec<ClosureTable>,
    edges: PreparedEdges,
    root: usize,
    brief: usize,
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
        macro_rules! read {($($field:ident:$ty:ty,)*)=>{$(while let Some((input,permit))=consumed.next::<$ty>(access)?{
            if input.type_id()==std::any::TypeId::of::<source::SourceArtifact>(){
                let actual=access.read_at::<source::SourceArtifact>(input.prefix())?;
                let alias=access.table_for(&input)?;
                let selected=format!("SELECT {} FROM {}",analysis::expected::CoverageAdmission::artifact_property_columns(),crate::consumed_rows::identifier(&alias));
                crate::consumed_rows::stream_query_at(&actual,&input,&session,&selected,|permit,batch|admission.visit_artifact_properties(permit,batch)).await?;
                continue;
            }
            crate::consumed_rows::stream_at(&permit,&input,access,&session,|permit,batch|{
                admission.visit_if_expected(permit,batch)?;
                if frames.iter().any(|frame|frame.type_id()==input.type_id() && frame.prefix()==input.prefix()){metadata.visit_input(&input,batch)?;}Ok(())
            }).await?;
        })*};}
        decoder_inputs!(read);
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
        let (edges, root, brief) =
            Self::prepare_bound(&inputs, &tables, &session, runtime.budget()).await?;
        Ok(Self {
            metadata,
            session,
            inputs,
            tables,
            edges,
            root,
            brief,
        })
    }
    async fn prepare_bound(
        inputs: &[ValidationInput],
        tables: &[ClosureTable],
        session: &SessionContext,
        budget: &resources::ResourceBudget,
    ) -> Result<(PreparedEdges, usize, usize), ModelError> {
        let root_table = typed::<c1::EvidenceRoot>(inputs)?;
        let brief_table = typed::<synthesis::briefs::Brief>(inputs)?;
        let mut bindings = tables.to_vec();
        let root = bindings.len();
        bindings.push(tables[root_table].clone());
        let brief = bindings.len();
        bindings.push(tables[brief_table].clone());
        let mut plan = NominalClosure::new(bindings)?;
        for (from, to) in [(root, root_table), (brief, brief_table)] {
            plan.pairs(
                from,
                to,
                format!(
                    "SELECT id AS source_id,id AS target_id FROM {}",
                    identifier(&tables[to].alias)
                ),
            )?;
        }
        let memberships = build::memberships();
        let subjects = typed::<c1::RootSubject>(inputs)?;
        for (source, table) in tables.iter().enumerate() {
            for field in table.relation.fields() {
                let Some((kind, _)) = field.target() else {
                    continue;
                };
                let Some(to) = target(inputs, source, kind)? else {
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
                if memberships.contains(&(table.relation.type_id(), field.name())) {
                    if kind == TypeId::of::<catalog::CatalogMember>() {
                        let (joins, predicate) = if table.relation.type_id()
                            == TypeId::of::<catalog::CatalogExposure>()
                        {
                            (
                                format!(
                                    " JOIN {} e ON x.exposure=e.id",
                                    identifier(
                                        &tables[typed::<normalized::entities::PublicExposure>(
                                            inputs
                                        )?]
                                        .alias
                                    )
                                ),
                                "e.context=r.context",
                            )
                        } else if table.relation.type_id()
                            == TypeId::of::<catalog::CatalogCallable>()
                        {
                            (
                                format!(
                                    " JOIN {} a ON x.assessment=a.id",
                                    identifier(
                                        &tables[typed::<
                                            normalized::callables::EffectiveCallableAssessment,
                                        >(inputs)?]
                                        .alias
                                    )
                                ),
                                "a.context=r.context",
                            )
                        } else {
                            (String::new(), "TRUE")
                        };
                        plan.pairs(root,source,format!("SELECT r.id AS source_id,x.id AS target_id FROM {} r JOIN {} s ON r.subject=s.id JOIN {} x ON x.{}=s.member_member{joins} WHERE {predicate}",identifier(&tables[root_table].alias),identifier(&tables[subjects].alias),identifier(&table.alias),identifier(field.name())))?;
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
        // The document grain consists of its actual captured passages in the selected context.
        let document = typed::<documents::DocumentObservation>(inputs)?;
        let passage = typed::<documents::PassageObservation>(inputs)?;
        let nodes = typed::<documents::DocumentNode>(inputs)?;
        let evidence = typed::<assertion::Evidence>(inputs)?;
        let qualifications = typed::<assertion::AssertionQualification>(inputs)?;
        plan.pairs(root,passage,format!("SELECT r.id AS source_id,p.id AS target_id FROM {} r JOIN {} s ON r.subject=s.id JOIN {} d ON s.document_observation=d.id JOIN {} e ON e.sourcespan_source=d.source JOIN {} n ON n.passage_span=e.id JOIN {} p ON p.passage=n.id JOIN {} q ON p.qualification=q.id WHERE q.context=r.context",identifier(&tables[root_table].alias),identifier(&tables[subjects].alias),identifier(&tables[document].alias),identifier(&tables[evidence].alias),identifier(&tables[nodes].alias),identifier(&tables[passage].alias),identifier(&tables[qualifications].alias)))?;
        let mention = typed::<documents::DocumentMentionObservation>(inputs)?;
        let assessments = typed::<normalized::links::MentionEntityAssessment>(inputs)?;
        let candidates = typed::<normalized::links::MentionEntityCandidate>(inputs)?;
        let associations = typed::<c1::DocumentAssociation>(inputs)?;
        plan.own(mention, "passage", nodes)?;
        plan.own(assessments, "observation", mention)?;
        plan.own(candidates, "assessment", assessments)?;
        plan.own(associations, "candidate", candidates)?;
        // A canonical brief belongs to one actual selected C0 member and the same C1 root.
        let seeds = typed::<synthesis::seeds::SelectedSeed>(inputs)?;
        let plans = typed::<synthesis::seeds::SeedPlan>(inputs)?;
        let invocation = typed::<analysis::synthesis::Invocation>(inputs)?;
        let member = typed::<catalog::CatalogMemberInvocation>(inputs)?;
        plan.pairs(brief,root_table,format!("SELECT b.id AS source_id,r.id AS target_id FROM {} b JOIN {} z ON b.seed=z.id JOIN {} p ON z.plan=p.id JOIN {} i ON p.invocation=i.id JOIN {} m ON z.member=m.id JOIN {} s ON s.member_member=m.member JOIN {} r ON r.subject=s.id AND r.input=i.input AND r.context=i.context",identifier(&tables[brief_table].alias),identifier(&tables[seeds].alias),identifier(&tables[plans].alias),identifier(&tables[invocation].alias),identifier(&tables[member].alias),identifier(&tables[subjects].alias),identifier(&tables[root_table].alias)))?;
        Ok((plan.prepare(session, budget).await?, root, brief))
    }
    pub fn session(&self) -> &SessionContext {
        &self.session
    }
    pub async fn roots(
        &self,
        input: Id<input::InputRevision>,
        context: Id<attribution::AnalysisContext>,
    ) -> Result<datafusion::physical_plan::SendableRecordBatchStream, ModelError> {
        let table = &self.tables[typed::<c1::EvidenceRoot>(&self.inputs)?].alias;
        crate::sql::query(
            &self.session,
            &format!(
                "SELECT id FROM {} WHERE input=X'{}' AND context=X'{}' ORDER BY id",
                identifier(table),
                input.hex(),
                context.hex()
            ),
        )
        .await
        .map_err(ModelError::codec)?
        .execute_stream()
        .await
        .map_err(ModelError::codec)
    }
    pub async fn briefs(
        &self,
        input: Id<input::InputRevision>,
        context: Id<attribution::AnalysisContext>,
    ) -> Result<datafusion::physical_plan::SendableRecordBatchStream, ModelError> {
        let table = |kind| {
            self.inputs
                .iter()
                .position(|input| input.type_id() == kind)
                .map(|index| identifier(&self.tables[index].alias))
                .ok_or(ModelError::Schema("E0 brief fixed relation"))
        };
        let sql = format!(
            "SELECT b.id FROM {} b JOIN {} z ON b.seed=z.id JOIN {} p ON z.plan=p.id JOIN {} i ON p.invocation=i.id WHERE i.input=X'{}' AND i.context=X'{}' ORDER BY b.id",
            table(TypeId::of::<synthesis::briefs::Brief>())?,
            table(TypeId::of::<synthesis::seeds::SelectedSeed>())?,
            table(TypeId::of::<synthesis::seeds::SeedPlan>())?,
            table(TypeId::of::<analysis::synthesis::Invocation>())?,
            input.hex(),
            context.hex()
        );
        crate::sql::query(&self.session, &sql)
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
        crate::consumed_rows::stream_query_at(&permit, input, &self.session, &sql, |_, batch| {
            rows.decode(batch)?;
            Ok(())
        })
        .await?;
        Ok(build::need(&rows, id)?.clone())
    }
    fn artifacts(
        &self,
        root: Option<(Id<c1::EvidenceRoot>, &c1::RootSubject)>,
        brief: Option<Id<synthesis::briefs::Brief>>,
    ) -> Result<String, ModelError> {
        let table = |kind| {
            self.inputs
                .iter()
                .position(|input| input.type_id() == kind)
                .map(|index| identifier(&self.tables[index].alias))
                .ok_or(ModelError::Schema("E0 original relation absent"))
        };
        let evidence = table(TypeId::of::<assertion::Evidence>())?;
        let occurrence = table(TypeId::of::<source::Occurrence>())?;
        if let Some((id, subject)) = root {
            let roots = table(TypeId::of::<c1::EvidenceRoot>())?;
            let subjects = table(TypeId::of::<c1::RootSubject>())?;
            let prefix = format!("FROM {roots} r JOIN {subjects} s ON r.subject=s.id");
            let predicate = format!("r.id=X'{}'", id.hex());
            return Ok(match subject {
                c1::RootSubject::Member { .. } => format!(
                    "SELECT m.source {prefix} JOIN {} c ON s.member_member=c.id JOIN {} m ON c.access=m.id WHERE {predicate}",
                    table(TypeId::of::<catalog::CatalogMember>())?,
                    table(TypeId::of::<source::Module>())?
                ),
                c1::RootSubject::Document { .. } => format!(
                    "SELECT d.source {prefix} JOIN {} d ON s.document_observation=d.id WHERE {predicate}",
                    table(TypeId::of::<documents::DocumentObservation>())?
                ),
                c1::RootSubject::Scenario { .. } => format!(
                    "SELECT COALESCE(o.artifact_artifact,c.source,e.sourcespan_source) {prefix} JOIN {} p ON p.scenario=s.scenario_scenario JOIN {} o ON p.source=o.id LEFT JOIN {occurrence} c ON o.occurrence_occurrence=c.id LEFT JOIN {evidence} e ON o.span_span=e.id WHERE {predicate}",
                    table(TypeId::of::<c1::ScenarioSpan>())?,
                    table(TypeId::of::<c1::OriginalSource>())?
                ),
                c1::RootSubject::Deployment { .. } => format!(
                    "SELECT e.sourcespan_source {prefix} JOIN {} c ON s.deployment_deployment=c.id JOIN {} d ON c.observation=d.id JOIN {evidence} e ON d.span=e.id WHERE {predicate}",
                    table(TypeId::of::<c1::CatalogDeployment>())?,
                    table(TypeId::of::<deployment::DeploymentObservation>())?
                ),
                c1::RootSubject::Option { .. } | c1::RootSubject::Release { .. } => {
                    "SELECT CAST(NULL AS BINARY) WHERE FALSE".into()
                }
            });
        }
        let id = brief.ok_or(ModelError::Schema("E0 grain owner absent"))?;
        Ok(format!(
            "SELECT COALESCE(o.source,e.sourcespan_source) FROM {} b JOIN {} d ON b.documentary=d.id JOIN {} p ON d.prose=p.id JOIN {} s ON p.source=s.id LEFT JOIN {occurrence} o ON COALESCE(s.literal_occurrence,s.occurrence_occurrence)=o.id LEFT JOIN {evidence} e ON s.span_span=e.id WHERE b.brief=X'{}'",
            table(TypeId::of::<synthesis::briefs::BriefSource>())?,
            table(TypeId::of::<synthesis::documentary::DocumentaryConclusion>())?,
            table(TypeId::of::<synthesis::documentary::ProseSlice>())?,
            table(TypeId::of::<synthesis::documentary::ProseSource>())?,
            id.hex()
        ))
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
        Ok(Some(
            if input.type_id() == TypeId::of::<catalog::CatalogMember>()
                && allowed.contains(&TypeId::of::<synthesis::briefs::Brief>())
            {
                format!(
                    "SELECT id,input,access FROM ({}) ownership",
                    scope.select(index)?
                )
            } else if input.type_id() == TypeId::of::<diagnostics::RuffDiagnosticObservation>() {
                format!(
                    "SELECT id,channel,settings FROM ({}) diagnostic_properties",
                    scope.select(index)?
                )
            } else if input.type_id() == TypeId::of::<diagnostics::PyreflyDiagnosticObservation>() {
                format!(
                    "SELECT id,channel FROM ({}) diagnostic_properties",
                    scope.select(index)?
                )
            } else if input.type_id() == TypeId::of::<source::SourceArtifact>() {
                format!(
                    "SELECT * FROM {} WHERE id IN ({artifacts})",
                    identifier(&self.tables[index].alias)
                )
            } else {
                scope.select(index)?
            },
        ))
    }
    async fn load(
        &self,
        access: &CompletedInputs,
        scope: &PreparedClosure,
        root: Option<(Id<c1::EvidenceRoot>, &c1::RootSubject)>,
        brief: Option<Id<synthesis::briefs::Brief>>,
        budget: &resources::ResourceBudget,
    ) -> Result<Data, ModelError> {
        let allowed = if let Some((_, subject)) = root {
            Data::root_types(subject)
        } else {
            Data::brief_types()
        };
        let artifacts = self.artifacts(root, brief)?;
        let mut data = Data::new(budget);
        let mut consumed = crate::consumed_rows::ConsumedInputs::new(self.inputs.clone(), budget)?;
        macro_rules! read {($($field:ident:$ty:ty,)*)=>{$(while let Some((input,permit))=consumed.next::<$ty>(access)?{
            let index=self.inputs.iter().position(|candidate|candidate.type_id()==input.type_id() && candidate.prefix()==input.prefix()).ok_or(ModelError::Conflict("E0 scoped input"))?;
            let Some(sql)=self.select(scope,index,&allowed,&artifacts)? else{continue;};
            crate::consumed_rows::stream_query_at(&permit,&input,scope.session(),&sql,|_,batch|{if brief.is_some() && input.type_id()==TypeId::of::<catalog::CatalogMember>(){data.completion_visit(&input,batch)?;}else{data.visit_input(&input,batch)?;}Ok(())}).await?;
        })*};}
        decoder_inputs!(read);
        consumed.finish("retrieval-document-grain")?;
        data.facts
            .definitions
            .insert(self.metadata.selected()?.clone())?;
        Ok(data)
    }
    async fn chunks(
        &self,
        access: &CompletedInputs,
        ranges: &retrieval::source::Ranges,
        data: &mut Data,
        budget: &resources::ResourceBudget,
    ) -> Result<(), ModelError> {
        let input = ValidationInput::of::<artifact::ArtifactChunk>(&["id"]);
        let table = access.table_for(&input)?;
        let permit = access.read::<artifact::ArtifactChunk>()?;
        for (artifact, start, end) in ranges.iter() {
            if start == end {
                continue;
            }
            let first = *start as usize / artifact::ARTIFACT_CHUNK_BYTES;
            let last = (*end as usize - 1) / artifact::ARTIFACT_CHUNK_BYTES;
            let sql = format!(
                "SELECT * FROM {} WHERE artifact=X'{}' AND ordinal BETWEEN {first} AND {last}",
                identifier(&table),
                artifact.hex()
            );
            crate::consumed_rows::stream_query_at(
                &permit,
                &input,
                &self.session,
                &sql,
                |_, batch| {
                    data.facts.chunks.decode(batch)?;
                    Ok(())
                },
            )
            .await?;
        }
        let _ = budget;
        Ok(())
    }
    pub async fn render_root(
        &self,
        access: &CompletedInputs,
        id: Id<c1::EvidenceRoot>,
        budget: &resources::ResourceBudget,
    ) -> Result<(Data, Output), ModelError> {
        let header = self.header::<c1::EvidenceRoot>(access, id, budget).await?;
        let subject = self
            .header::<c1::RootSubject>(access, header.subject, budget)
            .await?;
        let scope = self
            .edges
            .grain(self.root, &format!("id=X'{}'", id.hex()), budget)
            .await?;
        let mut data = self
            .load(access, &scope, Some((id, &subject)), None, budget)
            .await?;
        drop(scope);
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
    pub async fn render_brief(
        &self,
        access: &CompletedInputs,
        id: Id<synthesis::briefs::Brief>,
        budget: &resources::ResourceBudget,
    ) -> Result<(Data, Output), ModelError> {
        let scope = self
            .edges
            .grain(self.brief, &format!("id=X'{}'", id.hex()), budget)
            .await?;
        let data = self.load(access, &scope, None, Some(id), budget).await?;
        drop(scope);
        if data.synthesis.briefs.len() != 1 || data.synthesis.briefs.get(id).is_none() {
            return Err(build::invalid("E0 brief grain membership differs"));
        }
        let mut rows = Output::new(budget);
        build::extend_synthesis(&data, &mut rows, budget)?;
        rows.verify_completion(&data, budget)?;
        Ok((data, rows))
    }
}
pub async fn publish_mandatory(
    output: &mut ProducerOutput,
    rows: &Output,
) -> Result<(), ModelError> {
    macro_rules! write {($($field:ident:$ty:ty,)*)=>{$(output.declare::<$ty>()?;for row in rows.$field.iter(){output.push(row.clone()).await?;})*};}
    lctx_model::retrieval_outputs!(write);
    Ok(())
}

#[cfg(test)]
mod controls {
    use super::*;
    use datafusion::datasource::MemTable;
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
            .artifacts(Some((root.id(), subject)), None)
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
        install(&session, &inputs, &tables, &[qualification.clone()]);
        install(&session, &inputs, &tables, &[original.clone()]);
        install(&session, &inputs, &tables, &[scenario.clone()]);
        install(&session, &inputs, &tables, &[span.clone()]);
        install(&session, &inputs, &tables, &[association.clone()]);
        install(&session, &inputs, &tables, &[subject.clone()]);
        install(&session, &inputs, &tables, &[root.clone()]);
        let mut metadata = Data::new(&budget);
        metadata
            .facts
            .definitions
            .insert(retrieval::Definition::builtin(false))
            .unwrap();
        let (edges, root_index, brief) =
            Preparation::prepare_bound(&inputs, &tables, &session, &budget)
                .await
                .unwrap();
        let preparation = Preparation {
            metadata,
            session,
            inputs,
            tables,
            edges,
            root: root_index,
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
