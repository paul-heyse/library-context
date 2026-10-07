//! Necessary E0 admission over exact unit grains and one externally ordered request group.
use crate::{
    consumed_rows::{ClosureTable, NominalClosure, identifier},
    scoped_admission::field_target,
    workspace::Cancellation,
};
use datafusion::prelude::SessionContext;
use futures::TryStreamExt;
use lctx_model::domain::{
    normalized::Rows,
    resources::ResourceBudget,
    retrieval::{
        self,
        build::{self, Data, Output},
        consumption::{self, ConsumptionData, RetrievalEmbeddingUse},
    },
    *,
};
use std::any::TypeId;
fn index<R: Record>(inputs: &[ValidationInput]) -> Result<usize, ModelError> {
    let mut matches = inputs
        .iter()
        .enumerate()
        .filter(|(_, input)| input.type_id() == TypeId::of::<R>());
    let first = matches
        .next()
        .ok_or(ModelError::Schema("E0 admission declaration absent"))?;
    if matches.next().is_none() {
        return Ok(first.0);
    }
    inputs
        .iter()
        .position(|input| {
            input.type_id() == TypeId::of::<R>()
                && input.prefix() == Some(stages::PublicationBoundary::Facts)
        })
        .ok_or(ModelError::Conflict("E0 admission immutable epoch"))
}
fn alias<R: Record>(
    inputs: &[ValidationInput],
    tables: &[ClosureTable],
) -> Result<String, ModelError> {
    Ok(identifier(&tables[index::<R>(inputs)?].alias))
}
fn nominal<R: serde::de::DeserializeOwned>(bytes: &[u8]) -> Result<R, ModelError> {
    serde::Deserialize::deserialize(serde::de::value::SeqDeserializer::<
        _,
        serde::de::value::Error,
    >::new(bytes.iter().copied()))
    .map_err(ModelError::codec)
}
fn bytes<'a>(
    batch: &'a arrow_array::RecordBatch,
    name: &str,
    row: usize,
) -> Result<&'a [u8], ModelError> {
    use arrow_array::Array;
    let array = batch
        .column_by_name(name)
        .ok_or(ModelError::Schema("E0 admission identity column"))?;
    if array.is_null(row) {
        return Err(ModelError::Schema("E0 admission null identity"));
    }
    array
        .as_any()
        .downcast_ref::<arrow_array::FixedSizeBinaryArray>()
        .map(|array| array.value(row))
        .ok_or(ModelError::Schema("E0 admission identity shape"))
}
async fn stream(
    session: &SessionContext,
    sql: &str,
    cancellation: &Cancellation,
    mut visit: impl FnMut(&arrow_array::RecordBatch) -> Result<(), ModelError>,
) -> Result<(), ModelError> {
    let mut rows = crate::sql::query(session, sql)
        .await
        .map_err(ModelError::codec)?
        .execute_stream()
        .await
        .map_err(ModelError::codec)?;
    while let Some(batch) = rows.try_next().await.map_err(ModelError::codec)? {
        cancellation.check()?;
        visit(&batch)?;
    }
    Ok(())
}
async fn refuse_rows(
    session: &SessionContext,
    sql: String,
    cancellation: &Cancellation,
    message: &'static str,
) -> Result<(), ModelError> {
    stream(session, &sql, cancellation, |batch| {
        if batch.num_rows() != 0 {
            return Err(build::invalid(message));
        }
        Ok(())
    })
    .await
}
fn plan(inputs: &[ValidationInput], tables: &[ClosureTable]) -> Result<NominalClosure, ModelError> {
    let mut plan = NominalClosure::new(tables.to_vec())?;
    for (source, table) in tables.iter().enumerate() {
        for field in table.relation.fields() {
            let Some((target, _)) = field.target() else {
                continue;
            };
            let Some(target) = field_target(inputs, source, target)? else {
                continue;
            };
            if field.list() {
                plan.pairs(
                    source,
                    target,
                    format!(
                        "SELECT id AS source_id,UNNEST({}) AS target_id FROM {}",
                        identifier(field.name()),
                        identifier(&table.alias)
                    ),
                )?;
            } else {
                plan.follow(source, field.name(), target)?;
            }
        }
    }
    let unit = index::<retrieval::Unit>(inputs)?;
    let part=index::<retrieval::ContentPart>(inputs)?;
    let window=index::<retrieval::SearchWindow>(inputs)?;
    for (member, field, owner) in [
        (index::<retrieval::UnitRoot>(inputs)?, "unit", unit),
        (index::<retrieval::OriginalAnchor>(inputs)?, "unit", unit),
        (index::<retrieval::UnitSubject>(inputs)?, "unit", unit),
        (part,"unit",unit),(window,"unit",unit),
        (index::<retrieval::PartSourceMap>(inputs)?,"part",part),
        (index::<retrieval::WindowPart>(inputs)?,"window",window),
        (index::<retrieval::WindowSourceMap>(inputs)?,"window",window),
        (index::<retrieval::WindowBinding>(inputs)?,"window",window),
    ] {
        plan.own(member, field, owner)?;
    }
    let memberships=build::memberships();
    for (source,table) in tables.iter().enumerate(){for field in table.relation.fields(){if memberships.contains(&(table.relation.type_id(),field.name())){if let Some((target,_))=field.target(){if let Some(target)=field_target(inputs,source,target)?{plan.own(source,field.name(),target)?;}}}}}
    for (member,field,owner) in [
        (index::<documents::DocumentMentionObservation>(inputs)?,"passage",index::<documents::DocumentNode>(inputs)?),
        (index::<normalized::links::MentionEntityAssessment>(inputs)?,"observation",index::<documents::DocumentMentionObservation>(inputs)?),
        (index::<normalized::links::MentionEntityCandidate>(inputs)?,"assessment",index::<normalized::links::MentionEntityAssessment>(inputs)?),
        (index::<catalog::evidence::DocumentAssociation>(inputs)?,"candidate",index::<normalized::links::MentionEntityCandidate>(inputs)?),
    ]{plan.own(member,field,owner)?;}
    Ok(plan)
}
/// Root integrates this declared scope arm in Workspace::validate_scope. Frozen aliases and
/// all catalogs are supplied by that existing admission authority; no input is reacquired.
pub(crate) async fn validate_retrieval(
    invariant: &Invariant,
    _scope: consumption::Scope,
    tables: Vec<ClosureTable>,
    session: &SessionContext,
    budget: &ResourceBudget,
    cancellation: &Cancellation,
) -> Result<(), ModelError> {
    let inputs = &invariant.inputs;
    let mut metadata = ConsumptionData::new(budget);
    let frames = Data::frame_inputs();
    let mut invocations = Rows::<analysis::retrieval::Invocation>::new(budget);
    let mut outcomes = Rows::<analysis::retrieval::AnalysisOutcome>::new(budget);
    for (index, input) in inputs.iter().enumerate() {
        if frames.iter().any(|candidate| {
            candidate.type_id() == input.type_id() && candidate.prefix() == input.prefix()
        }) || [
            TypeId::of::<embedding::EmbeddingSpec>(),
            TypeId::of::<embedding::configuration::ServiceConfiguration>(),
            TypeId::of::<embedding::DocumentRecipe>(),TypeId::of::<embedding::projection::ProjectionDefinition>(),
            TypeId::of::<embedding::value::FullValue>(),TypeId::of::<embedding::projection::ProjectedValue>(),
            TypeId::of::<analysis::retrieval::InvocationSource>(),
            TypeId::of::<analysis::retrieval::AnalysisInput>(),
        ]
        .contains(&input.type_id())
        {
            stream(
                session,
                &format!("SELECT * FROM {}", identifier(&tables[index].alias)),
                cancellation,
                |batch| {
                    metadata.visit_input(input, batch)?;
                    Ok(())
                },
            )
            .await?;
        } else if input.type_id() == TypeId::of::<analysis::retrieval::Invocation>() {
            stream(
                session,
                &format!("SELECT * FROM {}", identifier(&tables[index].alias)),
                cancellation,
                |batch| {
                    invocations.decode(batch)?;
                    Ok(())
                },
            )
            .await?;
        } else if input.type_id() == TypeId::of::<analysis::retrieval::AnalysisOutcome>() {
            stream(
                session,
                &format!("SELECT * FROM {}", identifier(&tables[index].alias)),
                cancellation,
                |batch| {
                    outcomes.decode(batch)?;
                    Ok(())
                },
            )
            .await?;
        }
    }
    let selected = metadata.render.selected()?.embedding_requested;
    if selected {
        metadata.selected_spec()?;
    }
    let unit = alias::<retrieval::Unit>(inputs, &tables)?;
    let corpus = alias::<retrieval::CorpusText>(inputs, &tables)?;
    let window = alias::<retrieval::SearchWindow>(inputs, &tables)?;
    let uses = alias::<RetrievalEmbeddingUse>(inputs, &tables)?;
    let invocation = alias::<analysis::retrieval::Invocation>(inputs, &tables)?;
    refuse_rows(session,format!("SELECT c.id FROM {corpus} c WHERE NOT EXISTS (SELECT 1 FROM {unit} u WHERE u.corpus=c.id) AND NOT EXISTS(SELECT 1 FROM {window} w WHERE w.corpus=c.id) LIMIT 1"),cancellation,"retrieval corpus has no contextual unit").await?;
    refuse_rows(session,format!("SELECT v.id FROM {uses} v JOIN {invocation} i ON v.invocation=i.id JOIN {window} f ON v.window=f.id WHERE NOT EXISTS (SELECT 1 FROM {unit} u WHERE u.id=f.unit AND u.input=i.input AND u.context=i.context) LIMIT 1"),cancellation,"retrieval use has no exact owning native frame").await?;
    let actual_roots=alias::<catalog::evidence::EvidenceRoot>(inputs,&tables)?;
    let unit_roots=alias::<retrieval::UnitRoot>(inputs,&tables)?;
    refuse_rows(session,format!("SELECT r.id FROM {actual_roots} r WHERE NOT EXISTS (SELECT 1 FROM {unit_roots} x JOIN {unit} u ON x.unit=u.id WHERE x.root=r.id AND u.input=r.input AND u.context=r.context) LIMIT 1"),cancellation,"completed C1 root has no exact contextual retrieval unit").await?;
    if !selected {
        refuse_rows(
            session,
            format!("SELECT id FROM {uses} LIMIT 1"),
            cancellation,
            "retrieval has unrequested vector uses",
        )
        .await?;
    }
    let prepared = plan(inputs, &tables)?.prepare(session, budget).await?;
    let unit_index = index::<retrieval::Unit>(inputs)?;
    let completion = Data::completion_types();
    let outputs = Output::inputs();
    let mut roots = crate::sql::query(session, &format!("SELECT id FROM {unit} ORDER BY id"))
        .await
        .map_err(ModelError::codec)?
        .execute_stream()
        .await
        .map_err(ModelError::codec)?;
    while let Some(batch) = roots.try_next().await.map_err(ModelError::codec)? {
        cancellation.check()?;
        for row in 0..batch.num_rows() {
            let id: Id<retrieval::Unit> = nominal(bytes(&batch, "id", row)?)?;
            let closure = prepared
                .grain(unit_index, &format!("id=X'{}'", id.hex()), budget)
                .await?;
            let mut data = Data::new(budget);
            data.facts
                .definitions
                .insert(metadata.render.selected()?.clone())?;
            let mut output = Output::new(budget);
            for (index, input) in inputs.iter().enumerate() {
                if !completion.contains(&input.type_id())
                    && !outputs
                        .iter()
                        .any(|candidate| candidate.type_id() == input.type_id())
                {
                    continue;
                }
                if input.type_id() == TypeId::of::<retrieval::RetrievalDefinition>() {
                    continue;
                }
                let selected = closure.select(index)?;
                let sql = if input.type_id() == TypeId::of::<catalog::CatalogMember>() {
                    format!("SELECT id,input,access FROM ({selected}) ownership")
                } else if input.type_id() == TypeId::of::<source::Module>()
                    || input.type_id() == TypeId::of::<documents::DocumentObservation>()
                {
                    format!("SELECT id,source FROM ({selected}) ownership")
                } else if input.type_id() == TypeId::of::<synthesis::briefs::Brief>() {
                    format!("SELECT id,seed FROM ({selected}) ownership")
                } else if input.type_id() == TypeId::of::<source::SourceArtifact>() {
                    format!("SELECT id,input,byte_len FROM ({selected}) ownership")
                } else {
                    selected
                };
                stream(closure.session(), &sql, cancellation, |batch| {
                    data.completion_visit(input, batch)?;
                    output.visit(input.name(), batch)?;
                    Ok(())
                })
                .await?;
            }
            output.verify_completion(&data, budget)?;
            let owner = build::need(&output.units, id)?;
            let mut parents = invocations.iter().filter(|invocation| {
                invocation.input == owner.input && invocation.context == owner.context
            });
            let parent = parents
                .next()
                .ok_or_else(|| build::invalid("retrieval unit native frame absent"))?;
            if parents.next().is_some() {
                return Err(build::invalid("retrieval unit native frame ambiguous"));
            }
            let mut unit_uses = Rows::new(budget);
            let sql = format!(
                "SELECT v.* FROM {uses} v JOIN {window} f ON v.window=f.id WHERE f.unit=X'{}' AND v.invocation=X'{}'",
                owner.id().hex(),
                parent.id().hex()
            );
            stream(session, &sql, cancellation, |batch| {
                unit_uses.decode(batch)?;
                Ok(())
            })
            .await?;
            consumption::verify_uses(
                &output,
                parent,
                if selected {
                    Some(metadata.selected_consumption()?)
                } else {
                    None
                },
                &unit_uses,
                budget,
            )?;
            metadata.verify_canonical_uses(&unit_uses)?;
        }
    }
    drop(roots);
    drop(prepared);
    // Outcome reasons depend on all units in one native frame, but only availability is needed.
    let mut expected = Rows::new(budget);
    for parent in invocations.iter() {
        let mut disposition = consumption::Disposition::default();
        let sql = format!(
            "SELECT DISTINCT availability FROM {uses} WHERE invocation=X'{}'",
            parent.id().hex()
        );
        stream(session, &sql, cancellation, |batch| {
            let values = batch
                .column_by_name("availability")
                .and_then(|array| array.as_any().downcast_ref::<arrow_array::Int16Array>())
                .ok_or(ModelError::Schema("retrieval availability shape"))?;
            for value in values.iter() {
                disposition.observe_availability(match value {
                    Some(0) => embedding::analytic::VectorAvailability::Available,
                    Some(1) => embedding::analytic::VectorAvailability::ServiceUnavailable,
                    Some(2) => embedding::analytic::VectorAvailability::TokenLimit,
                    _ => return Err(ModelError::Schema("retrieval availability code")),
                });
            }
            Ok(())
        })
        .await?;
        expected.insert(disposition.outcome(parent))?;
    }
    metadata.verify_frames(&invocations, &outcomes, &expected, budget)?;
    Ok(())
}
