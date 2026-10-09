//! Compact canonical topology admission and one derived snapshot body at a time.
use crate::{
    consumed_rows::{ClosureTable, identifier, stream_batches},
    workspace::Cancellation,
};
use futures::TryStreamExt;
use lctx_model::domain::{
    normalized::Rows,
    projection::{
        compact::compact_columns,
        normalization::{
            AdmissionScope, CompactProjectionData, ProjectionData, ProjectionKey, ProjectionOutput,
        },
        *,
    },
    *,
};
use std::any::TypeId;
fn table<R: Record>(invariant: &Invariant, tables: &[ClosureTable]) -> Result<String, ModelError> {
    let index = invariant
        .inputs
        .iter()
        .position(|input| input.type_id() == TypeId::of::<R>())
        .ok_or(ModelError::Schema(R::NAME))?;
    tables
        .get(index)
        .map(|table| identifier(&table.alias))
        .ok_or(ModelError::Schema(R::NAME))
}
fn binary<R>(id: Id<R>) -> String {
    format!(
        "X'{}'",
        id.bytes()
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>()
    )
}
fn predicate(key: ProjectionKey) -> String {
    format!(
        "a.input={} AND a.context={} AND a.projection={}",
        binary(key.input),
        binary(key.context),
        key.name.code()
    )
}
async fn read<R: Record>(
    invariant: &Invariant,
    session: &datafusion::prelude::SessionContext,
    sql: &str,
    rows: &mut Rows<R>,
    cancellation: &Cancellation,
) -> Result<(), ModelError> {
    let input = invariant.inputs.iter()
        .find(|input| input.type_id() == TypeId::of::<R>() && input.name() == R::NAME)
        .ok_or(ModelError::Schema(R::NAME))?;
    let mut visit = |batch: &arrow_array::RecordBatch| {
        cancellation.check()?;
        rows.decode(batch)
    };
    stream_batches(input, session, sql, None, &mut visit).await
}

async fn selected(
    invariant: &Invariant,
    tables: &[ClosureTable],
    session: &datafusion::prelude::SessionContext,
    key: ProjectionKey,
    budget: &resources::ResourceBudget,
    cancellation: &Cancellation,
) -> Result<ProjectionOutput, ModelError> {
    let mut stored = ProjectionOutput::new(budget);
    let assessments = table::<ProjectionSourceAssessment>(invariant, tables)?;
    let gaps = table::<ProjectionGap>(invariant, tables)?;
    let where_key = predicate(key);
    read(
        invariant,
        session,
        &format!("SELECT a.* FROM {assessments} a WHERE {where_key} ORDER BY a.id"),
        &mut stored.assessments,
        cancellation,
    )
    .await?;
    read(invariant,session,&format!("SELECT g.* FROM {gaps} g JOIN {assessments} a ON a.id=g.assessment WHERE {where_key} ORDER BY g.id"),&mut stored.gaps,cancellation).await?;
    let subjects = table::<ProjectionGapSubject>(invariant, tables)?;
    read(invariant,session,&format!("SELECT s.* FROM {subjects} s JOIN {gaps} g ON g.subject=s.id JOIN {assessments} a ON a.id=g.assessment WHERE {where_key} ORDER BY s.id"),&mut stored.subjects,cancellation).await?;
    let coverage = table::<ProjectionSourceCoverage>(invariant, tables)?;
    read(invariant,session,&format!("SELECT c.* FROM {coverage} c JOIN {assessments} a ON a.id=c.assessment WHERE {where_key} ORDER BY c.id"),&mut stored.coverage,cancellation).await?;
    Ok(stored)
}
pub async fn validate_projections(
    scope: AdmissionScope,
    invariant: &Invariant,
    tables: Vec<ClosureTable>,
    session: &datafusion::prelude::SessionContext,
    budget: &resources::ResourceBudget,
    cancellation: &Cancellation,
) -> Result<(), ModelError> {
    let mut data = CompactProjectionData::new(budget);
    let input_kinds: Vec<_> = ProjectionData::validation_inputs()
        .iter()
        .map(ValidationInput::type_id)
        .collect();
    for (index, input) in invariant
        .inputs
        .iter()
        .enumerate()
        .filter(|(_, input)| input_kinds.contains(&input.type_id()))
    {
        let columns = compact_columns(input.name())
            .map(|columns| {
                columns
                    .iter()
                    .map(|column| identifier(column))
                    .collect::<Vec<_>>()
                    .join(",")
            })
            .unwrap_or_else(|| "*".into());
        let sql = format!(
            "SELECT {columns} FROM {} ORDER BY id",
            identifier(&tables[index].alias)
        );
        let mut visit = |batch: &arrow_array::RecordBatch| {
            cancellation.check()?;
            data.visit(input.name(), batch).map(|_| ())
        };
        stream_batches(input, session, &sql, None, &mut visit).await?;
    }
    let prepared = data.prepare(budget)?;
    let assessments = table::<ProjectionSourceAssessment>(invariant, &tables)?;
    let owned = if scope == AdmissionScope::Canonical {
        vec![
            table::<ProjectionGap>(invariant, &tables)?,
            table::<ProjectionSourceCoverage>(invariant, &tables)?,
        ]
    } else {
        vec![table::<ProjectionSnapshot>(invariant, &tables)?]
    };
    for table in owned {
        let sql = format!(
            "SELECT r.id FROM {table} r LEFT JOIN {assessments} a ON a.id=r.assessment WHERE a.id IS NULL LIMIT 1"
        );
        let mut orphans = crate::sql::query(session, &sql)
            .await
            .map_err(ModelError::codec)?
            .execute_stream()
            .await
            .map_err(ModelError::codec)?;
        while let Some(batch) = orphans.try_next().await.map_err(ModelError::codec)? {
            cancellation.check()?;
            if batch.num_rows() != 0 {
                return Err(ModelError::Invalid(
                    "projection member has no source assessment".into(),
                ));
            }
        }
    }
    if scope == AdmissionScope::Canonical {
        // Every candidate key must have an outcome, independently of which rows were advertised.
        for key in prepared.keys() {
            let stored = selected(invariant, &tables, session, key, budget, cancellation).await?;
            prepared.validate_key(&stored, key, false, budget)?;
            tokio::task::yield_now().await;
        }
        let mut stream =
            crate::sql::query(session, &format!("SELECT * FROM {assessments} ORDER BY id"))
                .await
                .map_err(ModelError::codec)?
                .execute_stream()
                .await
                .map_err(ModelError::codec)?;
        while let Some(batch) = stream.try_next().await.map_err(ModelError::codec)? {
            cancellation.check()?;
            let mut roots = Rows::<ProjectionSourceAssessment>::new(budget);
            roots.decode(&batch)?;
            for root in roots.iter() {
                let key = ProjectionKey {
                    input: root.input,
                    context: root.context,
                    name: root.projection,
                };
                if !prepared.contains(key) {
                    return Err(ModelError::Invalid(
                        "advertised projection outside actual input/context collection".into(),
                    ));
                }
            }
        }
        let subjects = table::<ProjectionGapSubject>(invariant, &tables)?;
        let gaps = table::<ProjectionGap>(invariant, &tables)?;
        let mut orphans=crate::sql::query(session,&format!("SELECT s.id FROM {subjects} s LEFT JOIN {gaps} g ON g.subject=s.id WHERE g.id IS NULL LIMIT 1")).await.map_err(ModelError::codec)?.execute_stream().await.map_err(ModelError::codec)?;
        while let Some(batch) = orphans.try_next().await.map_err(ModelError::codec)? {
            if batch.num_rows() != 0 {
                return Err(ModelError::Invalid("unowned projection gap subject".into()));
            }
        }
    } else {
        // Derived bodies are optional in portable graphs. Every actual advertised body is checked.
        let headers = table::<ProjectionSnapshot>(invariant, &tables)?;
        let mut stream=crate::sql::query(session,&format!("SELECT a.* FROM {assessments} a JOIN {headers} h ON h.assessment=a.id ORDER BY a.id")).await.map_err(ModelError::codec)?.execute_stream().await.map_err(ModelError::codec)?;
        while let Some(batch) = stream.try_next().await.map_err(ModelError::codec)? {
            cancellation.check()?;
            let mut roots = Rows::<ProjectionSourceAssessment>::new(budget);
            roots.decode(&batch)?;
            for root in roots.iter() {
                let key = ProjectionKey {
                    input: root.input,
                    context: root.context,
                    name: root.projection,
                };
                let stored =
                    selected(invariant, &tables, session, key, budget, cancellation).await?;
                prepared.validate_key(&stored, key, false, budget)?;
                let mut selected_headers = Rows::<ProjectionSnapshot>::new(budget);
                let sql = format!(
                    "SELECT h.* FROM {headers} h JOIN {assessments} a ON a.id=h.assessment WHERE {} ORDER BY h.id",
                    predicate(key)
                );
                read(invariant,session, &sql, &mut selected_headers, cancellation).await?;
                if selected_headers.len() != 1 {
                    return Err(ModelError::Invalid(
                        "projection snapshot domain differs".into(),
                    ));
                }
                let header = selected_headers.iter().next().expect("one snapshot");
                let assessment = stored
                    .assessments
                    .iter()
                    .next()
                    .expect("validated assessment");
                let mut assembly = snapshot::GraphAssembly::new(header, assessment, budget)?;
                let chunks = table::<ProjectionSnapshotChunk>(invariant, &tables)?;
                let sql = format!(
                    "SELECT * FROM {chunks} WHERE snapshot={} ORDER BY ordinal",
                    binary(header.id())
                );
                let mut chunks = crate::sql::query(session, &sql)
                    .await
                    .map_err(ModelError::codec)?
                    .execute_stream()
                    .await
                    .map_err(ModelError::codec)?;
                while let Some(batch) = chunks.try_next().await.map_err(ModelError::codec)? {
                    cancellation.check()?;
                    let _decode = budget.reserve(
                        "projection-admission-chunk",
                        decode_allowance::<ProjectionSnapshotChunk>(&batch)?,
                    )?;
                    for chunk in ProjectionSnapshotChunk::decode(&batch)? {
                        assembly.push(&chunk)?;
                    }
                    tokio::task::yield_now().await;
                }
                let graph = assembly.finish()?;
                prepared.validate_graph(key, &graph, budget)?;
                tokio::task::yield_now().await;
            }
        }
        let chunks = table::<ProjectionSnapshotChunk>(invariant, &tables)?;
        let mut orphans=crate::sql::query(session,&format!("SELECT c.id FROM {chunks} c LEFT JOIN {headers} h ON h.id=c.snapshot WHERE h.id IS NULL LIMIT 1")).await.map_err(ModelError::codec)?.execute_stream().await.map_err(ModelError::codec)?;
        while let Some(batch) = orphans.try_next().await.map_err(ModelError::codec)? {
            if batch.num_rows() != 0 {
                return Err(ModelError::Invalid("orphan graph chunk".into()));
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod projection_scope_controls {
    use super::*;
    use datafusion::{datasource::MemTable, prelude::SessionContext};
    use lctx_model::domain::{attribution::*, source::*};
    use std::{collections::BTreeMap, sync::Arc};
    fn nominal<R>(value: u8) -> Id<R> {
        serde::Deserialize::deserialize(serde::de::value::SeqDeserializer::<
            _,
            serde::de::value::Error,
        >::new([value; 16].into_iter()))
        .unwrap()
    }
    fn register(session: &SessionContext, name: &str, batch: arrow_array::RecordBatch) {
        session
            .register_table(
                name,
                Arc::new(MemTable::try_new(batch.schema(), vec![vec![batch]]).unwrap()),
            )
            .unwrap();
    }
    #[tokio::test]
    async fn compact_projection_metadata_excludes_payload_and_admits_optional_one_key_body() {
        let session = SessionContext::new();
        let budget = resources::ResourceBudget::fixed(128 << 10).unwrap();
        let context = nominal(3);
        let input = nominal(1);
        let provider = nominal(2);
        let artifact =
            SourceArtifact::from_bytes(input, format!("{}.py", "x".repeat(256 << 10)), b"pass")
                .unwrap();
        let module = Module {
            source: artifact.id(),
            qualified_name: "m".repeat(256 << 10),
        };
        let occurrence = Occurrence {
            source: artifact.id(),
            start: 0,
            end: 1,
            syntax_kind: SyntaxKind::ExprName,
            role: OccurrenceRole::Read,
            structural_path: vec![1; 64 << 10],
        };
        let scope = CoverageScope::Input { input };
        let (run, _) = ProviderRun::new(
            provider,
            context,
            input,
            ContentHash::of(b"projection"),
            [FactFamily::Calls],
        )
        .unwrap();
        let coverage = ProviderCoverage {
            scope: scope.id(),
            provider: Some(provider),
            context,
            family: FactFamily::Calls,
            run: Some(run.id()),
            status: CoverageStatus::CompleteUnderStatedModel,
            reason: None,
            diagnostic: Some("d".repeat(256 << 10)),
        };
        let mut batches = BTreeMap::new();
        macro_rules! batch {
            ($ty:ty; $value:expr) => {
                batches.insert(
                    <$ty>::NAME,
                    <$ty as Record>::encode(std::slice::from_ref(&$value)).unwrap(),
                );
            };
            ($ty:ty,$values:expr) => {
                batches.insert(<$ty>::NAME, <$ty as Record>::encode(&$values).unwrap());
            };
        }
        batch!(SourceArtifact, [artifact]);
        batch!(Module; module);
        batch!(Occurrence; occurrence);
        batch!(CoverageScope, [scope]);
        batch!(ProviderRun, [run]);
        batch!(ProviderCoverage, [coverage]);
        batch!(
            normalized::entities::EntityRef,
            [
                normalized::entities::EntityRef::Module {
                    module: module.id()
                },
                normalized::entities::EntityRef::Occurrence {
                    occurrence: occurrence.id()
                }
            ]
        );
        let mut compact = CompactProjectionData::new(&budget);
        macro_rules! inputs {($($field:ident:$ty:ty,)*)=>{$({
            let batch=batches.remove(<$ty>::NAME).unwrap_or_else(||<$ty as Record>::encode(&[]).unwrap());
            let indices=compact_columns(<$ty>::NAME).map(|fields|fields.iter().map(|field|batch.schema().index_of(field).unwrap()).collect::<Vec<_>>());
            compact.visit(<$ty>::NAME,&match indices{Some(indices)=>batch.project(&indices).unwrap(),None=>batch.clone()}).unwrap();
            register(&session,<$ty>::NAME,batch);
        })*};}
        lctx_model::projection_inputs!(inputs);
        let prepared = compact.prepare(&budget).unwrap();
        let mut stored = ProjectionOutput::new(&budget);
        for key in prepared.keys() {
            let produced = prepared.produce(key, &budget).unwrap();
            if key.name == ProjectionName::DefinitionContainment {
                let row = produced.assessments.iter().next().unwrap();
                assert_eq!((row.vertices, row.arcs), (2, 0));
                for header in produced.snapshots.iter() {
                    stored.snapshots.insert(header.clone()).unwrap();
                }
                for chunk in produced.chunks.iter() {
                    stored.chunks.insert(chunk.clone()).unwrap();
                }
            }
            for row in produced.assessments.iter() {
                stored.assessments.insert(row.clone()).unwrap();
            }
            for row in produced.subjects.iter() {
                stored.subjects.insert(row.clone()).unwrap();
            }
            for row in produced.gaps.iter() {
                stored.gaps.insert(row.clone()).unwrap();
            }
            for row in produced.coverage.iter() {
                stored.coverage.insert(row.clone()).unwrap();
            }
        }
        macro_rules! outputs {($($field:ident:$ty:ty,)*)=>{$(register(&session,<$ty>::NAME,<$ty as Record>::encode(&stored.$field.iter().cloned().collect::<Vec<_>>()).unwrap());)*};}
        lctx_model::projection_outputs!(outputs);
        let model = model().unwrap();
        for invariant in projection::normalization::invariants() {
            let scope = if invariant.name == "normalized_projection_canonical" {
                AdmissionScope::Canonical
            } else {
                AdmissionScope::Snapshot
            };
            let tables = invariant
                .inputs
                .iter()
                .map(|input| ClosureTable {
                    relation: model.relation(input.name()).unwrap().clone(),
                    alias: input.name().into(),
                })
                .collect();
            validate_projections(
                scope,
                &invariant,
                tables,
                &session,
                &budget,
                &Cancellation::default(),
            )
            .await
            .unwrap();
        }
        let key = prepared
            .keys()
            .find(|key| key.name == ProjectionName::DefinitionContainment)
            .unwrap();
        let mut missing = prepared.produce(key, &budget).unwrap();
        let bad = ProjectionSourceAssessment {
            vertices: 3,
            ..missing.assessments.iter().next().unwrap().clone()
        };
        missing.assessments = Rows::new(&budget);
        missing.assessments.insert(bad).unwrap();
        assert!(prepared.validate_key(&missing, key, true, &budget).is_err());
        drop((missing, stored, prepared));
        drop(compact);
        assert_eq!(budget.reserved(), 0);
    }
}
