//! One retained hydration per selected projection in an analysis collection.
//!
//! Each borrowing consumer still needs its own declared completed-input permits. Stored snapshots
//! supply topology; neither preparation nor repeated algorithms rebuild edges from semantic rows.
use crate::workspace::{CompletedInputs, Workspace};
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
}

/// Collection-owned hydrated graphs. Dropping the owner releases every graph reservation.
/// A borrowed graph cannot outlive either this owner or its consumer's stage access.
///
/// ```compile_fail
/// use cpg_core::analysis_graphs::PreparedGraphs;
/// use lctx_model::domain::{projection::{normalization::ProjectionKey, snapshot::MaterializedGraph},
///     resources::ResourceBudget};
/// use cpg_core::workspace::CompletedInputs;
/// fn escape<'a>(graphs: &'a PreparedGraphs, access: CompletedInputs,
///     budget: &ResourceBudget, key: ProjectionKey) -> &'a MaterializedGraph {
///     graphs.graph(&access, budget, key).unwrap()
/// }
/// ```
pub struct PreparedGraphs {
    sources: [analysis::sources::SourceSnapshot; 3],
    graphs: Vec<PreparedProjection>,
    budget: resources::ResourceBudget,
    _charge: StateCharge,
}
fn invalid(message: &str) -> ModelError {
    ModelError::Invalid(message.into())
}
fn sources(access: &CompletedInputs) -> Result<[analysis::sources::SourceSnapshot; 3], ModelError> {
    fn source<R: Record>(
        access: &CompletedInputs,
    ) -> Result<analysis::sources::SourceSnapshot, ModelError> {
        Ok(access.read::<R>()?.snapshot())
    }
    Ok([
        source::<ProjectionSourceAssessment>(access)?,
        source::<ProjectionSnapshot>(access)?,
        source::<ProjectionSnapshotChunk>(access)?,
    ])
}
async fn load<R: Record>(
    session: &datafusion::prelude::SessionContext,
    rows: &mut Rows<R>,
) -> Result<(), ModelError> {
    let mut stream = crate::sql::query(&session, &format!("SELECT * FROM \"{}\"", R::NAME))
        .await
        .map_err(ModelError::codec)?
        .execute_stream()
        .await
        .map_err(ModelError::codec)?;
    while let Some(batch) = stream.try_next().await.map_err(ModelError::codec)? {
        rows.decode(&batch)?;
    }
    Ok(())
}

impl PreparedGraphs {
    /// Prepare after normalized publication/checkpoint, from the first consuming stage's permits.
    /// Its declaration must include the sources' normal invariant/reference closure, just like
    /// other completed-input readers. Selection is explicit; missing selected snapshots refuse.
    pub async fn load(
        access: &CompletedInputs,
        runtime: &Workspace,
        _model: &Arc<ValidatedModel>,
        names: &BTreeSet<ProjectionName>,
    ) -> Result<Self, ModelError> {
        if names.is_empty() {
            return Err(invalid("graph preparation needs a named projection"));
        }
        let sources = sources(access)?;
        let session = access.session(runtime).await?;
        let budget = runtime.budget();
        let mut assessments = Rows::<ProjectionSourceAssessment>::new(budget);
        let mut headers = Rows::<ProjectionSnapshot>::new(budget);
        let mut chunks = Rows::<ProjectionSnapshotChunk>::new(budget);
        load(&session, &mut assessments).await?;
        load(&session, &mut headers).await?;
        load(&session, &mut chunks).await?;
        drop(session);
        let budget = budget.clone();
        let names = names.clone();
        crate::stage_runtime::borrowed_cpu("projection-hydration", || {
            Self::hydrate(sources, assessments, headers, chunks, &names, &budget)
        })
    }
    fn hydrate(
        sources: [analysis::sources::SourceSnapshot; 3],
        assessments: Rows<ProjectionSourceAssessment>,
        headers: Rows<ProjectionSnapshot>,
        chunks: Rows<ProjectionSnapshotChunk>,
        names: &BTreeSet<ProjectionName>,
        budget: &resources::ResourceBudget,
    ) -> Result<Self, ModelError> {
        let mut charge = StateCharge::new(budget, "prepared-analysis-graphs");
        charge.grow(size_of::<Self>())?;
        let selected = assessments
            .iter()
            .filter(|a| names.contains(&a.projection))
            .count();
        charge.grow(
            selected
                .checked_mul(size_of::<PreparedProjection>())
                .ok_or_else(|| invalid("prepared graph count overflow"))?,
        )?;
        let mut graphs = Vec::with_capacity(selected);
        for assessment in assessments.iter().filter(|a| names.contains(&a.projection)) {
            let key = ProjectionKey {
                input: assessment.input,
                context: assessment.context,
                name: assessment.projection,
            };
            if graphs
                .iter()
                .any(|p: &PreparedProjection| p.graph.key() == key)
            {
                return Err(invalid("ambiguous prepared graph identity"));
            }
            let mut matching = headers.iter().filter(|h| h.assessment == assessment.id());
            let header = matching
                .next()
                .ok_or_else(|| invalid("selected graph snapshot absent"))?;
            if matching.next().is_some() {
                return Err(invalid("ambiguous selected graph snapshot"));
            }
            let graph = snapshot::hydrate(header, assessment, &chunks, budget)?;
            graphs.push(PreparedProjection {
                assessment: assessment.clone(),
                graph,
            });
        }
        graphs.sort_by_key(|p| p.graph.key());
        Ok(Self {
            sources,
            graphs,
            budget: budget.clone(),
            _charge: charge,
        })
    }
    fn admit(
        &self,
        access: &CompletedInputs,
        budget: &resources::ResourceBudget,
    ) -> Result<(), ModelError> {
        if !self.budget.shares_pool(budget) || self.sources != sources(access)? {
            return Err(invalid(
                "prepared graph belongs to another budget or completed source set",
            ));
        }
        Ok(())
    }
    pub fn keys<'a>(
        &'a self,
        access: &'a CompletedInputs,
        budget: &resources::ResourceBudget,
    ) -> Result<impl Iterator<Item = ProjectionKey> + 'a, ModelError> {
        self.admit(access, budget)?;
        Ok(self.graphs.iter().map(|p| p.graph.key()))
    }
    pub fn graph<'a>(
        &'a self,
        access: &'a CompletedInputs,
        budget: &resources::ResourceBudget,
        key: ProjectionKey,
    ) -> Result<&'a MaterializedGraph, ModelError> {
        self.admit(access, budget)?;
        self.graphs
            .iter()
            .find(|p| p.graph.key() == key)
            .map(|p| &p.graph)
            .ok_or_else(|| invalid("projection was not prepared for this collection"))
    }
    /// Availability remains independent of successful hydration and algorithm execution.
    pub fn assessment<'a>(
        &'a self,
        access: &'a CompletedInputs,
        budget: &resources::ResourceBudget,
        key: ProjectionKey,
    ) -> Result<&'a ProjectionSourceAssessment, ModelError> {
        self.admit(access, budget)?;
        self.graphs
            .iter()
            .find(|p| p.graph.key() == key)
            .map(|p| &p.assessment)
            .ok_or_else(|| invalid("projection was not prepared for this collection"))
    }
}
