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
async fn verify_root_domain(
    session: &SessionContext,
    roots: &str,
    unit_roots: &str,
    units: &str,
    cancellation: &Cancellation,
) -> Result<(), ModelError> {
    refuse_rows(session,format!("SELECT r.id FROM {roots} r WHERE NOT EXISTS (SELECT 1 FROM {unit_roots} x JOIN {units} u ON x.unit=u.id WHERE x.root=r.id AND u.input=r.input AND u.context=r.context) LIMIT 1"),cancellation,"completed C1 root has no exact contextual retrieval unit").await
}
async fn verify_corpus_domain(
    session: &SessionContext,
    corpus: &str,
    units: &str,
    windows: &str,
    cancellation: &Cancellation,
) -> Result<(), ModelError> {
    refuse_rows(session,format!("SELECT c.id FROM {corpus} c WHERE NOT EXISTS (SELECT 1 FROM {units} u WHERE u.corpus=c.id) AND NOT EXISTS(SELECT 1 FROM {windows} w WHERE w.corpus=c.id) LIMIT 1"),cancellation,"retrieval corpus has no contextual unit/window").await
}
fn plan(inputs: &[ValidationInput], tables: &[ClosureTable]) -> Result<NominalClosure, ModelError> {
    let mut plan = NominalClosure::new(tables.to_vec())?;
    for (source, table) in tables.iter().enumerate() {
        for field in table.relation.fields() {
            let Some((target, _)) = field.target() else {
                continue;
            };
            let Some(target) = retrieval_field_target(inputs, source, target)? else {
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
    let part = index::<retrieval::ContentPart>(inputs)?;
    let window = index::<retrieval::SearchWindow>(inputs)?;
    for (member, field, owner) in [
        (index::<retrieval::UnitRoot>(inputs)?, "unit", unit),
        (index::<retrieval::OriginalAnchor>(inputs)?, "unit", unit),
        (index::<retrieval::UnitSubject>(inputs)?, "unit", unit),
        (part, "unit", unit),
        (window, "unit", unit),
        (index::<retrieval::PartContext>(inputs)?, "primary", part),
        (index::<retrieval::PartSourceMap>(inputs)?, "part", part),
        (index::<retrieval::WindowPart>(inputs)?, "window", window),
        (
            index::<retrieval::WindowSourceMap>(inputs)?,
            "window",
            window,
        ),
        (index::<retrieval::WindowBinding>(inputs)?, "window", window),
    ] {
        plan.own(member, field, owner)?;
    }
    let memberships = build::memberships();
    for (source, table) in tables.iter().enumerate() {
        for field in table.relation.fields() {
            if memberships.contains(&(table.relation.type_id(), field.name()))
                && let Some((target, _)) = field.target()
                && let Some(target) = retrieval_field_target(inputs, source, target)?
            {
                plan.own(source, field.name(), target)?;
            }
        }
    }
    for (member, field, owner) in [
        (
            index::<documents::DocumentMentionObservation>(inputs)?,
            "passage",
            index::<documents::DocumentNode>(inputs)?,
        ),
        (
            index::<normalized::links::MentionEntityAssessment>(inputs)?,
            "observation",
            index::<documents::DocumentMentionObservation>(inputs)?,
        ),
        (
            index::<normalized::links::MentionEntityCandidate>(inputs)?,
            "assessment",
            index::<normalized::links::MentionEntityAssessment>(inputs)?,
        ),
        (
            index::<catalog::evidence::DocumentAssociation>(inputs)?,
            "candidate",
            index::<normalized::links::MentionEntityCandidate>(inputs)?,
        ),
    ] {
        plan.own(member, field, owner)?;
    }
    Ok(plan)
}
fn retrieval_field_target(
    inputs: &[ValidationInput],
    source: usize,
    target: TypeId,
) -> Result<Option<usize>, ModelError> {
    // Native source/part/binding qualifications retain Facts identity. The documentary
    // conclusion alone carries the association qualification authored at Local.
    if target == TypeId::of::<assertion::AssertionQualification>()
        && inputs[source].prefix().is_none()
    {
        let epoch = if inputs[source].type_id()
            == TypeId::of::<synthesis::documentary::DocumentaryConclusion>()
        {
            stages::PublicationBoundary::Local
        } else {
            stages::PublicationBoundary::Facts
        };
        return inputs
            .iter()
            .position(|input| input.type_id() == target && input.prefix() == Some(epoch))
            .map(Some)
            .ok_or(ModelError::Conflict("retrieval qualification epoch absent"));
    }
    field_target(inputs, source, target)
}
fn original_chunk_keys(
    data: &Data,
    output: &Output,
    charge: &mut charged::StateCharge,
) -> Result<charged::ChargedSet<Id<artifact::ArtifactChunk>>, ModelError> {
    let mut selected = charged::ChargedSet::default();
    for map in output.part_maps.iter() {
        if let Some(original) = map.original {
            let (artifact, anchor_start, anchor_end) = retrieval::source::coordinates(
                data,
                build::need(&output.anchor_sources, original)?,
            )?;
            let start = map.original_start.unwrap();
            let end = map.original_end.unwrap();
            if start < anchor_start || end < start || end > anchor_end {
                return Err(build::invalid(
                    "original chunk selection exceeds its anchor",
                ));
            }
            if start == end {
                continue;
            }
            let first = start / artifact::ARTIFACT_CHUNK_BYTES as i64;
            let last = (end - 1) / artifact::ARTIFACT_CHUNK_BYTES as i64;
            for ordinal in first..=last {
                selected.insert(
                    charge,
                    Id::of(&artifact::ArtifactChunkKey { artifact, ordinal }),
                )?;
            }
        }
    }
    Ok(selected)
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
            TypeId::of::<embedding::DocumentRecipe>(),
            TypeId::of::<embedding::projection::ProjectionDefinition>(),
            TypeId::of::<embedding::value::FullValue>(),
            TypeId::of::<embedding::projection::ProjectedValue>(),
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
    verify_corpus_domain(session, &corpus, &unit, &window, cancellation).await?;
    refuse_rows(session,format!("SELECT v.id FROM {uses} v JOIN {invocation} i ON v.invocation=i.id JOIN {window} f ON v.window=f.id WHERE NOT EXISTS (SELECT 1 FROM {unit} u WHERE u.id=f.unit AND u.input=i.input AND u.context=i.context) LIMIT 1"),cancellation,"retrieval use has no exact owning native frame").await?;
    let actual_roots = alias::<catalog::evidence::EvidenceRoot>(inputs, &tables)?;
    let unit_roots = alias::<retrieval::UnitRoot>(inputs, &tables)?;
    verify_root_domain(session, &actual_roots, &unit_roots, &unit, cancellation).await?;
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
            // Hydrate only chunks intersecting original maps in this actual unit; API access
            // modules and unrelated source grains are never hydrated or rerendered here.
            let chunks = alias::<artifact::ArtifactChunk>(inputs, &tables)?;
            let mut chunk_charge =
                charged::StateCharge::new(budget, "retrieval-completion-original-chunks");
            let selected_chunks = original_chunk_keys(&data, &output, &mut chunk_charge)?;
            if !selected_chunks.is_empty() {
                chunk_charge.grow(selected_chunks.len().checked_mul(128).ok_or_else(|| {
                    build::invalid("retrieval original chunk query size overflow")
                })?)?;
                let selected = selected_chunks
                    .iter()
                    .map(|key| format!("X'{}'", key.hex()))
                    .collect::<Vec<_>>()
                    .join(",");
                stream(
                    session,
                    &format!("SELECT * FROM {chunks} WHERE id IN ({selected})"),
                    cancellation,
                    |batch| {
                        data.facts.chunks.decode(batch)?;
                        Ok(())
                    },
                )
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

#[cfg(test)]
mod controls {
    use super::*;
    use datafusion::datasource::MemTable;
    use std::sync::Arc;
    fn id<R>(n: u8) -> Id<R> {
        nominal(&[n; 16]).unwrap()
    }
    fn install<R: Record>(session: &SessionContext, name: &str, rows: &[R]) {
        let batch = R::encode(rows).unwrap();
        session.deregister_table(name).unwrap();
        session
            .register_table(
                name,
                Arc::new(MemTable::try_new(batch.schema(), vec![vec![batch]]).unwrap()),
            )
            .unwrap();
    }
    fn corpus(text: &str) -> retrieval::CorpusText {
        retrieval::CorpusText {
            family: retrieval::Family::Source,
            rendering_version: retrieval::RENDER_VERSION,
            digest: ContentHash::of(text.as_bytes()),
            text: text.into(),
        }
    }
    fn unit(corpus: Id<retrieval::CorpusText>) -> retrieval::Unit {
        retrieval::Unit {
            input: id(1),
            context: id(2),
            family: retrieval::Family::Source,
            origin: id(3),
            corpus,
            title: "source".into(),
        }
    }
    fn window(
        unit: Id<retrieval::Unit>,
        corpus: &retrieval::CorpusText,
    ) -> retrieval::SearchWindow {
        retrieval::SearchWindow {
            definition: retrieval::Definition::builtin(false).id(),
            unit,
            ordinal: 0,
            corpus: corpus.id(),
            digest: corpus.digest,
            text: corpus.text.clone(),
            input_text: corpus.text.clone(),
            encoded_digest: embedding::value::input_hash(corpus.text.as_str()),
            tokenizer: None,
            tokens: None,
            availability: retrieval::WindowAvailability::TokenizerUnavailable,
        }
    }
    #[test]
    fn original_chunk_batch_excludes_unrelated_bytes_deduplicates_overlap_and_keeps_empty_source_synthetic()
     {
        let budget = ResourceBudget::fixed(16 << 20).unwrap();
        let mut data = Data::new(&budget);
        let mut output = Output::new(&budget);
        let width = artifact::ARTIFACT_CHUNK_BYTES as i64;
        let source = source::SourceArtifact::from_bytes(
            id(1),
            "guide.md".into(),
            &vec![b'x'; artifact::ARTIFACT_CHUNK_BYTES * 4],
        )
        .unwrap();
        data.source.core.artifacts.insert(source.clone()).unwrap();
        let original = output
            .anchor_sources
            .insert(retrieval::AnchorSource::Artifact {
                artifact: source.id(),
            })
            .unwrap();
        let mut charge = charged::StateCharge::new(&budget, "original-chunk-control");
        assert!(
            original_chunk_keys(&data, &output, &mut charge)
                .unwrap()
                .is_empty()
        );
        for (ordinal, (start, end)) in [
            (width - 1, width + 1),
            (width, width + 2),
            (3 * width, 3 * width + 1),
        ]
        .into_iter()
        .enumerate()
        {
            output
                .part_maps
                .insert(retrieval::PartSourceMap {
                    part: id(4),
                    ordinal: ordinal as i64,
                    start: 0,
                    end: end - start,
                    original: Some(original),
                    original_start: Some(start),
                    original_end: Some(end),
                })
                .unwrap();
        }
        let selected = original_chunk_keys(&data, &output, &mut charge).unwrap();
        let expected = [0, 1, 3]
            .into_iter()
            .map(|ordinal| {
                Id::<artifact::ArtifactChunk>::of(&artifact::ArtifactChunkKey {
                    artifact: source.id(),
                    ordinal,
                })
            })
            .collect::<std::collections::BTreeSet<_>>();
        assert_eq!(&*selected, &expected);
        data.facts
            .definitions
            .insert(retrieval::Definition::builtin(false))
            .unwrap();
        let empty = source::SourceArtifact::from_bytes(id(1), "empty.md".into(), b"").unwrap();
        data.source.core.artifacts.insert(empty.clone()).unwrap();
        let subject = data
            .evidence
            .subjects
            .insert(catalog::evidence::RootSubject::Source {
                artifact: empty.id(),
            })
            .unwrap();
        data.evidence
            .roots
            .insert(catalog::evidence::EvidenceRoot {
                input: id(1),
                context: id(2),
                subject,
            })
            .unwrap();
        let empty_output = build::build(&data, &budget).unwrap();
        empty_output.verify_completion(&data, &budget).unwrap();
        assert!(
            empty_output
                .parts
                .iter()
                .all(|p| p.purpose == retrieval::PartPurpose::Context)
        );
        assert!(empty_output.part_maps.iter().all(|m| m.original.is_none()));
        assert!(
            original_chunk_keys(&data, &empty_output, &mut charge)
                .unwrap()
                .is_empty()
        );
    }
    #[tokio::test]
    async fn omitted_roots_and_foreign_context_cannot_hide_behind_unit_scopes() {
        let session = SessionContext::new();
        let cancellation = Cancellation::default();
        let root = catalog::evidence::EvidenceRoot {
            input: id(1),
            context: id(2),
            subject: id(4),
        };
        let corpus = corpus("complete grain");
        let valid = unit(corpus.id());
        install(&session, "roots", std::slice::from_ref(&root));
        install(&session, "units", std::slice::from_ref(&valid));
        install::<retrieval::UnitRoot>(&session, "unit_roots", &[]);
        assert!(
            verify_root_domain(&session, "roots", "unit_roots", "units", &cancellation)
                .await
                .is_err()
        );
        let mapping = retrieval::UnitRoot {
            unit: valid.id(),
            root: root.id(),
        };
        install(&session, "unit_roots", &[mapping]);
        verify_root_domain(&session, "roots", "unit_roots", "units", &cancellation)
            .await
            .unwrap();
        let mut foreign = valid;
        foreign.context = id(9);
        install(&session, "units", &[foreign]);
        assert!(
            verify_root_domain(&session, "roots", "unit_roots", "units", &cancellation)
                .await
                .is_err()
        );
    }
    #[tokio::test]
    async fn split_window_corpus_is_owned_without_requiring_another_unit() {
        let session = SessionContext::new();
        let cancellation = Cancellation::default();
        let complete = corpus("complete grain");
        let split = corpus("selected primary");
        let unit = unit(complete.id());
        let window = window(unit.id(), &split);
        install(&session, "corpus", &[complete, split]);
        install(&session, "units", &[unit]);
        install(&session, "windows", &[window]);
        verify_corpus_domain(&session, "corpus", "units", "windows", &cancellation)
            .await
            .unwrap();
        install::<retrieval::SearchWindow>(&session, "windows", &[]);
        assert!(
            verify_corpus_domain(&session, "corpus", "units", "windows", &cancellation)
                .await
                .is_err()
        );
    }
    #[tokio::test]
    async fn actual_unit_inverse_closure_includes_parts_contexts_maps_windows_and_bindings() {
        let model = lctx_model::domain::model().unwrap();
        let inputs = consumption::invariants().remove(0).inputs;
        let session = SessionContext::new();
        let tables: Vec<_> = inputs
            .iter()
            .enumerate()
            .map(|(n, input)| {
                let relation = model.relation(input.name()).unwrap().clone();
                let alias = format!("closure_{n}");
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
        let corpus = corpus("primary");
        let unit = unit(corpus.id());
        let window = window(unit.id(), &corpus);
        let part = retrieval::ContentPart {
            unit: unit.id(),
            ordinal: 0,
            purpose: retrieval::PartPurpose::Primary,
            scope: None,
            qualification: None,
            digest: corpus.digest,
            text: corpus.text.clone(),
        };
        let context = retrieval::ContentPart {
            unit: unit.id(),
            ordinal: 1,
            purpose: retrieval::PartPurpose::Context,
            scope: None,
            qualification: None,
            digest: ContentHash::of(b"context"),
            text: "context".into(),
        };
        macro_rules! put {
            ($ty:ty,$rows:expr) => {
                install::<$ty>(
                    &session,
                    &tables[index::<$ty>(&inputs).unwrap()].alias,
                    $rows,
                );
            };
        }
        put!(retrieval::Unit, std::slice::from_ref(&unit));
        put!(retrieval::CorpusText, std::slice::from_ref(&corpus));
        put!(retrieval::ContentPart, &[part.clone(), context.clone()]);
        put!(
            retrieval::PartContext,
            &[retrieval::PartContext {
                primary: part.id(),
                context: context.id()
            }]
        );
        put!(retrieval::SearchWindow, std::slice::from_ref(&window));
        put!(
            retrieval::PartSourceMap,
            &[
                retrieval::PartSourceMap {
                    part: part.id(),
                    ordinal: 0,
                    start: 0,
                    end: 7,
                    original: None,
                    original_start: None,
                    original_end: None
                },
                retrieval::PartSourceMap {
                    part: context.id(),
                    ordinal: 0,
                    start: 0,
                    end: 7,
                    original: None,
                    original_start: None,
                    original_end: None
                }
            ]
        );
        put!(
            retrieval::WindowPart,
            &[retrieval::WindowPart {
                window: window.id(),
                ordinal: 0,
                part: part.id(),
                start: 0,
                end: 7
            }]
        );
        put!(
            retrieval::WindowSourceMap,
            &[retrieval::WindowSourceMap {
                window: window.id(),
                ordinal: 0,
                start: 0,
                end: 7,
                part: Some(part.id()),
                original: None,
                original_start: None,
                original_end: None
            }]
        );
        put!(
            retrieval::WindowBinding,
            &[retrieval::WindowBinding {
                window: window.id(),
                part: part.id(),
                subject: id(9),
                basis: retrieval::BindingBasis::Source,
                qualification: None
            }]
        );
        let budget = ResourceBudget::fixed(16 << 20).unwrap();
        let prepared = plan(&inputs, &tables)
            .unwrap()
            .prepare(&session, &budget)
            .await
            .unwrap();
        let closure = prepared
            .grain(
                index::<retrieval::Unit>(&inputs).unwrap(),
                &format!("id=X'{}'", unit.id().hex()),
                &budget,
            )
            .await
            .unwrap();
        for ty in [
            TypeId::of::<retrieval::PartContext>(),
            TypeId::of::<retrieval::ContentPart>(),
            TypeId::of::<retrieval::PartSourceMap>(),
            TypeId::of::<retrieval::SearchWindow>(),
            TypeId::of::<retrieval::WindowPart>(),
            TypeId::of::<retrieval::WindowSourceMap>(),
            TypeId::of::<retrieval::WindowBinding>(),
        ] {
            let index = inputs
                .iter()
                .position(|input| input.type_id() == ty)
                .unwrap();
            let selected = closure.select(index).unwrap();
            let batches = crate::sql::query(closure.session(), &selected)
                .await
                .unwrap()
                .collect()
                .await
                .unwrap();
            assert_eq!(
                batches.iter().map(|batch| batch.num_rows()).sum::<usize>(),
                if ty == TypeId::of::<retrieval::ContentPart>()
                    || ty == TypeId::of::<retrieval::PartSourceMap>()
                {
                    2
                } else {
                    1
                }
            );
        }
    }
}
