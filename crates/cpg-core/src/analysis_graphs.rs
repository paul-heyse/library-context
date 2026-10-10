//! One retained hydration per selected projection in an analysis collection.
//!
//! Each borrowing consumer still needs its own declared completed-input permits. Stored snapshots
//! supply topology; neither preparation nor repeated algorithms rebuild edges from semantic rows.
use crate::workspace::{CheckedInputs, CompletedInputs, Workspace};
use futures::TryStreamExt;
use lctx_model::domain::{
    charged::StateCharge,
    normalized::Rows,
    projection::{
        normalization::ProjectionKey,
        snapshot::{self, MaterializedGraph},
        *,
    },
    *,
};
use std::{collections::BTreeSet, sync::Arc};

struct PreparedProjection {
    assessment: ProjectionSourceAssessment,
    graph: MaterializedGraph,
    schedule: Option<execution::summary_schedule::SccSchedule>,
}

/// Collection-owned hydrated graphs. Dropping the owner releases every graph reservation.
/// A borrowed graph cannot outlive either this owner or its consumer's stage access.
///
/// ```compile_fail
/// use cpg_core::analysis_graphs::PreparedGraphs;
/// use lctx_model::domain::{projection::{normalization::ProjectionKey, snapshot::MaterializedGraph},
///     resources::ResourceBudget};
/// use cpg_core::workspace::{CompletedInputs, Workspace};
/// fn escape<'a>(graphs: &'a PreparedGraphs, access: CompletedInputs,
///     runtime: &Workspace, key: ProjectionKey) -> &'a MaterializedGraph {
///     graphs.graph(&access, runtime, key).unwrap()
/// }
/// ```
pub struct PreparedGraphs {
    checked: CheckedInputs,
    graphs: Vec<PreparedProjection>,
    budget: resources::ResourceBudget,
    _charge: StateCharge,
}
fn invalid(message: &str) -> ModelError {
    ModelError::Invalid(message.into())
}

async fn load<R: Record>(
    access: &CompletedInputs,
    session: &datafusion::prelude::SessionContext,
    rows: &mut Rows<R>,
    predicate: datafusion::logical_expr::Expr,
) -> Result<(), ModelError> {
    let declaration = ValidationInput::of::<R>(&[]);
    let permit = access.read_at::<R>(declaration.prefix())?;
    let table = access.table_for(&declaration)?;
    let selected = format!("SELECT * FROM {}", crate::consumed_rows::identifier(&table));
    crate::consumed_rows::stream_query_filter_at(
        &permit, &declaration, access, session, &selected, Some(predicate),
        |_, batch| rows.decode(batch),
    ).await
}

fn id_literal<R>(id: Id<R>) -> datafusion::logical_expr::Expr {
    datafusion::prelude::lit(datafusion::common::ScalarValue::FixedSizeBinary(
        16,
        Some(id.bytes().to_vec()),
    ))
}

impl PreparedGraphs {
    /// Retire projections whose final possible consumer has completed in this schedule.
    /// Topology and SCC reservations belong to each projection and drop here. The small
    /// collection/index allowance remains charged until the collection itself is released.
    pub(crate) fn retain(&mut self, names: &BTreeSet<ProjectionName>) {
        self.graphs.retain(|projection| names.contains(&projection.graph.key().name));
    }
    /// Borrow normalization-owned admission, retaining only the exact graph source descriptors.
    /// Each consumer must still declare those immutable streams. Selection is explicit; missing
    /// selected snapshots refuse without rerunning predecessor admission or graph construction.
    pub async fn load(
        access: &CompletedInputs,
        runtime: &Workspace,
        checked: &CheckedInputs,
        _model: &Arc<ValidatedModel>,
        names: &BTreeSet<ProjectionName>,
    ) -> Result<Self, ModelError> {
        if names.is_empty() {
            return Err(invalid("graph preparation needs a named projection"));
        }
        let checked = checked.select(&[
            ValidationInput::of::<ProjectionSourceAssessment>(&["id"]),
            ValidationInput::of::<ProjectionSnapshot>(&["id"]),
            ValidationInput::of::<ProjectionSnapshotChunk>(&["id"]),
        ])?;
        checked.require_subset(runtime, access)?;
        let session = access.session(runtime).await?;
        let budget = runtime.budget();
        // Decode selected metadata only. The predicate is applied to the attempt scan, before
        // rich chunk decoding; disabled named projections never enter retained state.
        use datafusion::prelude::{col, lit};
        let predicate =
            col("projection").in_list(names.iter().map(|name| lit(*name as i16)).collect(), false);
        let mut assessments = Rows::<ProjectionSourceAssessment>::new(budget);
        load(access, &session, &mut assessments, predicate).await?;
        let mut charge = StateCharge::new(budget, "prepared-analysis-graphs");
        charge.grow(size_of::<Self>())?;
        let mut graphs: Vec<PreparedProjection> = Vec::new();
        let mut keys = BTreeSet::new();
        for assessment in assessments.iter() {
            runtime.cancellation().check()?;
            let key = ProjectionKey {
                input: assessment.input,
                context: assessment.context,
                name: assessment.projection,
            };
            if !keys.insert(key) {
                return Err(invalid("ambiguous prepared graph identity"));
            }
            charge.grow(size_of::<ProjectionKey>() + 96)?;
            // Each encoded representation has the lifetime of this hydration only. Retaining
            // every selected graph's chunks alongside the compact topology duplicates the
            // entire collection and can crowd out the next graph's legitimate allocation.
            let mut headers = Rows::<ProjectionSnapshot>::new(budget);
            load(
                access,
                &session,
                &mut headers,
                col("assessment").eq(id_literal(assessment.id())),
            )
            .await?;
            let mut matching = headers.iter();
            let header = matching
                .next()
                .ok_or_else(|| invalid("selected graph snapshot absent"))?;
            if matching.next().is_some() {
                return Err(invalid("ambiguous selected graph snapshot"));
            }
            let mut assembly = snapshot::GraphAssembly::new(header, assessment, budget)?;
            let mut stream = crate::sql::query(
                &session,
                &format!("SELECT * FROM \"{}\"", ProjectionSnapshotChunk::NAME),
            )
            .await
            .map_err(ModelError::codec)?
            .filter(col("snapshot").eq(id_literal(header.id())))
            .map_err(ModelError::codec)?
            .sort(vec![col("ordinal").sort(true, false)])
            .map_err(ModelError::codec)?
            .execute_stream()
            .await
            .map_err(ModelError::codec)?;
            while let Some(batch) = stream.try_next().await.map_err(ModelError::codec)? {
                runtime.cancellation().check()?;
                // Typed payload ownership is restricted to this transfer batch.
                let _decode = budget.reserve(
                    "graph-chunk-decode",
                    lctx_model::domain::decode_allowance::<ProjectionSnapshotChunk>(&batch)?,
                )?;
                for row in ProjectionSnapshotChunk::decode(&batch)? {
                    assembly.push(&row)?;
                }
                tokio::task::yield_now().await;
            }
            let graph =
                crate::stage_runtime::borrowed_cpu("projection-hydration", || assembly.finish())?;
            let schedule = if access.profile() == stages::Profile::Behavioral
                && key.name == ProjectionName::CallableInvocation {
                // Checking persisted components repeats both graph traversals and the
                // condensation kernel. Retain one fresh schedule for this materialization.
                Some(execution::summary_schedule::invocation_sccs(&graph, budget)?)
            } else { None };
            charge.grow(size_of::<PreparedProjection>())?;
            graphs.push(PreparedProjection {
                assessment: assessment.clone(),
                graph,
                schedule,
            });
        }
        graphs.sort_by_key(|p| p.graph.key());
        Ok(Self {
            checked,
            graphs,
            budget: budget.clone(),
            _charge: charge,
        })
    }
    fn admit(&self, access: &CompletedInputs, runtime: &Workspace) -> Result<(), ModelError> {
        self.checked.require_subset(runtime, access)?;
        if !self.budget.shares_pool(runtime.budget()) {
            return Err(invalid(
                "prepared graph belongs to another budget or completed source set",
            ));
        }
        Ok(())
    }
    pub fn keys<'a>(
        &'a self,
        access: &'a CompletedInputs,
        runtime: &Workspace,
    ) -> Result<impl Iterator<Item = ProjectionKey> + 'a, ModelError> {
        self.admit(access, runtime)?;
        Ok(self.graphs.iter().map(|p| p.graph.key()))
    }
    pub fn graph<'a>(
        &'a self,
        access: &'a CompletedInputs,
        runtime: &Workspace,
        key: ProjectionKey,
    ) -> Result<&'a MaterializedGraph, ModelError> {
        self.admit(access, runtime)?;
        self.graphs
            .binary_search_by_key(&key, |projection| projection.graph.key())
            .ok()
            .map(|index| &self.graphs[index].graph)
            .ok_or_else(|| invalid("projection was not prepared for this collection"))
    }
    /// Availability remains independent of successful hydration and algorithm execution.
    pub fn assessment<'a>(
        &'a self,
        access: &'a CompletedInputs,
        runtime: &Workspace,
        key: ProjectionKey,
    ) -> Result<&'a ProjectionSourceAssessment, ModelError> {
        self.admit(access, runtime)?;
        self.graphs
            .binary_search_by_key(&key, |projection| projection.graph.key())
            .ok()
            .map(|index| &self.graphs[index].assessment)
            .ok_or_else(|| invalid("projection was not prepared for this collection"))
    }
    pub(crate) fn schedule<'a>(&'a self, access: &CompletedInputs, runtime: &Workspace,
        key: ProjectionKey) -> Result<&'a execution::summary_schedule::SccSchedule, ModelError> {
        self.admit(access, runtime)?;
        let projection = self.graphs.binary_search_by_key(&key, |projection| projection.graph.key())
            .ok().map(|index| &self.graphs[index]).ok_or(ModelError::Conflict("selected SCC projection absent"))?;
        projection.schedule.as_ref().ok_or(ModelError::Conflict("selected SCC schedule absent"))
    }
}

#[cfg(test)]
mod scoped_loading_controls {
    use super::*;
    use datafusion::{
        datasource::MemTable,
        prelude::{SessionContext, col},
    };

    #[tokio::test]
    async fn selected_snapshot_does_not_decode_unrelated_rich_payload() {
        let selected = ProjectionSnapshot {
            assessment: serde::Deserialize::deserialize(serde::de::value::SeqDeserializer::<
                _,
                serde::de::value::Error,
            >::new([1u8; 16].into_iter()))
            .unwrap(),
            format_version: snapshot::FORMAT_VERSION,
            petgraph_version: snapshot::PETGRAPH_VERSION.into(),
            codec: snapshot::CODEC.into(),
            bytes: 4,
            chunks: 1,
        };
        let foreign = ProjectionSnapshot {
            assessment: serde::Deserialize::deserialize(serde::de::value::SeqDeserializer::<
                _,
                serde::de::value::Error,
            >::new([2u8; 16].into_iter()))
            .unwrap(),
            ..selected.clone()
        };
        let row = ProjectionSnapshotChunk {
            snapshot: selected.id(),
            ordinal: 0,
            payload: EvidenceBytes(vec![1, 2, 3, 4]),
        };
        let unrelated = ProjectionSnapshotChunk {
            snapshot: foreign.id(),
            ordinal: 0,
            payload: EvidenceBytes(vec![0; snapshot::CHUNK_BYTES]),
        };
        let batch = ProjectionSnapshotChunk::encode(&[unrelated, row.clone()]).unwrap();
        let session = SessionContext::new();
        session
            .register_table(
                ProjectionSnapshotChunk::NAME,
                Arc::new(MemTable::try_new(batch.schema(), vec![vec![batch]]).unwrap()),
            )
            .unwrap();
        let budget = resources::ResourceBudget::fixed(16 << 10).unwrap();
        let mut decoded = Rows::new(&budget);
        let declaration = ValidationInput::of::<ProjectionSnapshotChunk>(&[]);
        let selected_sql = format!("SELECT * FROM {}", crate::consumed_rows::identifier(ProjectionSnapshotChunk::NAME));
        let mut visit = |batch: &arrow_array::RecordBatch| decoded.decode(batch);
        crate::consumed_rows::stream_batches(
            &declaration, &session, &selected_sql,
            Some(col("snapshot").eq(id_literal(selected.id()))), &mut visit,
        ).await.unwrap();
        assert_eq!(decoded.len(), 1);
        assert_eq!(decoded.get(row.id()), Some(&row));
    }
}
