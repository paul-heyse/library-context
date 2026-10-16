//! One retained hydration per selected projection in an analysis collection.
//!
//! Each borrowing consumer still needs its own declared completed-input permits. Stored snapshots
//! supply topology; neither preparation nor repeated algorithms rebuild edges from semantic rows.
use crate::workspace::{CheckedInputs, CompletedInputs, Workspace};
use futures::{FutureExt, TryStreamExt, future::BoxFuture};
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
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::Arc,
};

struct PreparedProjection {
    assessment: ProjectionSourceAssessment,
    graph: MaterializedGraph,
    schedule: Option<execution::summary_schedule::SccSchedule>,
    _charge: StateCharge,
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
    graphs: Vec<Arc<PreparedProjection>>,
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
        &permit,
        &declaration,
        access,
        session,
        &selected,
        Some(predicate),
        |_, batch| rows.decode(batch),
    )
    .await
}

fn id_literal<R>(id: Id<R>) -> datafusion::logical_expr::Expr {
    datafusion::prelude::lit(datafusion::common::ScalarValue::FixedSizeBinary(
        16,
        Some(id.bytes().to_vec()),
    ))
}

impl PreparedGraphs {
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
        let mut graphs = Vec::new();
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
                && key.name == ProjectionName::CallableInvocation
            {
                // Checking persisted components repeats both graph traversals and the
                // condensation kernel. Retain one fresh schedule for this materialization.
                Some(execution::summary_schedule::invocation_sccs(
                    &graph, budget,
                )?)
            } else {
                None
            };
            let mut projection_charge = StateCharge::new(budget, "prepared-analysis-projection");
            projection_charge.grow(size_of::<PreparedProjection>() + assessment.heap_bytes())?;
            charge.grow(size_of::<Arc<PreparedProjection>>())?;
            graphs.push(Arc::new(PreparedProjection {
                assessment: assessment.clone(),
                graph,
                schedule,
                _charge: projection_charge,
            }));
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
    pub(crate) fn schedule<'a>(
        &'a self,
        access: &CompletedInputs,
        runtime: &Workspace,
        key: ProjectionKey,
    ) -> Result<&'a execution::summary_schedule::SccSchedule, ModelError> {
        self.admit(access, runtime)?;
        let projection = self
            .graphs
            .binary_search_by_key(&key, |projection| projection.graph.key())
            .ok()
            .map(|index| &self.graphs[index])
            .ok_or(ModelError::Conflict("selected SCC projection absent"))?;
        projection
            .schedule
            .as_ref()
            .ok_or(ModelError::Conflict("selected SCC schedule absent"))
    }
}

/// The coordinator retains one immutable preparation slot per selected projection.
/// Running consumers own their slots too; retiring pending demand cannot invalidate a borrow.
pub(crate) struct GraphPreparation {
    projections: BTreeMap<ProjectionName, Arc<PreparedSlot>>,
    charge: StateCharge,
}
struct PreparedSlot {
    graph: tokio::sync::OnceCell<PreparedGraphs>,
    _charge: StateCharge,
}
pub(crate) struct SelectedGraphs {
    projections: Vec<(ProjectionName, Arc<PreparedSlot>)>,
    _charge: StateCharge,
}
impl GraphPreparation {
    const ENTRY_BYTES: usize = size_of::<(ProjectionName, Arc<PreparedSlot>)>() + 64;
    pub(crate) fn new(budget: &resources::ResourceBudget) -> Result<Self, ModelError> {
        let mut charge = StateCharge::new(budget, "prepared-graph-slots");
        charge.grow(size_of::<Self>())?;
        Ok(Self {
            projections: BTreeMap::new(),
            charge,
        })
    }
    pub(crate) fn select(
        &mut self,
        names: &[ProjectionName],
    ) -> Result<Option<SelectedGraphs>, ModelError> {
        if names.is_empty() {
            return Ok(None);
        }
        let budget = self
            .charge
            .budget()
            .expect("graph slot owner budget")
            .clone();
        let mut charge = StateCharge::new(&budget, "selected-graph-slots");
        charge.grow(
            size_of::<SelectedGraphs>()
                + names
                    .len()
                    .saturating_mul(size_of::<(ProjectionName, Arc<PreparedSlot>)>()),
        )?;
        let mut projections = Vec::with_capacity(names.len());
        for &name in names {
            if !self.projections.contains_key(&name) {
                // The slot and its Arc allocation retain their own charge through the
                // final borrower, independently of the coordinator's map membership.
                let mut slot_charge = StateCharge::new(&budget, "prepared-graph-slot");
                slot_charge.grow(size_of::<PreparedSlot>() + 2 * size_of::<usize>())?;
                self.charge.grow(Self::ENTRY_BYTES)?;
                self.projections.insert(
                    name,
                    Arc::new(PreparedSlot {
                        graph: tokio::sync::OnceCell::new(),
                        _charge: slot_charge,
                    }),
                );
            }
            projections.push((name, self.projections[&name].clone()));
        }
        Ok(Some(SelectedGraphs {
            projections,
            _charge: charge,
        }))
    }
    pub(crate) fn retain(&mut self, names: &BTreeSet<ProjectionName>) {
        let before = self.projections.len();
        self.projections.retain(|name, _| names.contains(name));
        self.charge
            .release((before - self.projections.len()) * Self::ENTRY_BYTES);
    }
}
impl SelectedGraphs {
    pub(crate) fn load<'a>(
        &'a self,
        access: &'a CompletedInputs,
        runtime: &'a Workspace,
        checked: &'a CheckedInputs,
        model: &'a Arc<ValidatedModel>,
    ) -> BoxFuture<'a, Result<PreparedGraphs, ModelError>> {
        async move {
            let mut graphs = Vec::new();
            let mut charge =
                StateCharge::new(runtime.budget(), "selected-prepared-analysis-graphs");
            charge.grow(size_of::<PreparedGraphs>())?;
            let mut authority = None;
            for (name, slot) in &self.projections {
                let prepared = slot
                    .graph
                    .get_or_try_init(|| async {
                        PreparedGraphs::load(
                            access,
                            runtime,
                            checked,
                            model,
                            &BTreeSet::from([*name]),
                        )
                        .await
                    })
                    .await?;
                prepared.admit(access, runtime)?;
                authority = Some(prepared.checked.clone());
                charge.grow(
                    prepared
                        .graphs
                        .len()
                        .saturating_mul(size_of::<Arc<PreparedProjection>>()),
                )?;
                graphs.extend(prepared.graphs.iter().cloned());
            }
            graphs.sort_by_key(|projection| projection.graph.key());
            if graphs
                .windows(2)
                .any(|pair| pair[0].graph.key() == pair[1].graph.key())
            {
                return Err(invalid("duplicate selected prepared projection"));
            }
            Ok(PreparedGraphs {
                checked: authority.ok_or_else(|| invalid("selected graph names absent"))?,
                graphs,
                budget: runtime.budget().clone(),
                _charge: charge,
            })
        }
        .boxed()
    }
}

#[cfg(test)]
mod scoped_loading_controls {
    use super::*;
    use datafusion::{
        datasource::MemTable,
        prelude::{SessionContext, col},
    };

    #[test]
    fn projection_slots_coalesce_and_running_selection_survives_pool_retirement() {
        use ProjectionName::*;
        let budget = resources::ResourceBudget::fixed(1 << 20).unwrap();
        let mut owner = GraphPreparation::new(&budget).unwrap();
        let summary = owner.select(&[CallableInvocation]).unwrap().unwrap();
        let structural = owner
            .select(&[CallableInvocation, DefinitionContainment])
            .unwrap()
            .unwrap();
        assert!(Arc::ptr_eq(
            &summary.projections[0].1,
            &structural.projections[0].1
        ));
        let invocation = Arc::downgrade(&summary.projections[0].1);
        let containment = Arc::downgrade(&structural.projections[1].1);
        let before_retirement = budget.reserved();
        owner.retain(&BTreeSet::from([CallableInvocation]));
        assert_eq!(
            budget.reserved(),
            before_retirement - GraphPreparation::ENTRY_BYTES,
            "map membership is released while the running slot reservation remains"
        );
        assert!(
            containment.upgrade().is_some(),
            "running Structural still owns containment"
        );
        drop(structural);
        assert!(containment.upgrade().is_none());
        owner.retain(&BTreeSet::new());
        assert!(
            invocation.upgrade().is_some(),
            "running Summary still owns invocation"
        );
        drop(summary);
        assert!(invocation.upgrade().is_none());
        assert_eq!(owner.charge.reserved(), size_of::<GraphPreparation>());
        assert_eq!(budget.reserved(), owner.charge.accounted_bytes());
        drop(owner);
        assert_eq!(budget.reserved(), 0);
    }

    #[test]
    fn graph_selection_refuses_descriptor_capacity_before_retaining_slots() {
        let budget = resources::ResourceBudget::fixed(1 << 20).unwrap();
        let mut owner = GraphPreparation::new(&budget).unwrap();
        let retained = budget.reserved();
        let pressure = budget
            .reserve(
                "graph descriptor refusal pressure",
                budget.limit() - retained,
            )
            .unwrap();
        assert!(owner.select(&[ProjectionName::CallableInvocation]).is_err());
        assert!(owner.projections.is_empty());
        drop(pressure);
        assert_eq!(budget.reserved(), retained);
        assert_eq!(owner.charge.reserved(), size_of::<GraphPreparation>());
        drop(owner);
        assert_eq!(budget.reserved(), 0);
    }

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
        let selected_sql = format!(
            "SELECT * FROM {}",
            crate::consumed_rows::identifier(ProjectionSnapshotChunk::NAME)
        );
        let mut visit = |batch: &arrow_array::RecordBatch| decoded.decode(batch);
        crate::consumed_rows::stream_batches(
            &declaration,
            &session,
            &selected_sql,
            Some(col("snapshot").eq(id_literal(selected.id()))),
            &mut visit,
        )
        .await
        .unwrap();
        assert_eq!(decoded.len(), 1);
        assert_eq!(decoded.get(row.id()), Some(&row));
    }
}
