//! Whole-owner unrequested production. No per-method invocation or outcome is created.
use super::*;
use crate::workspace::CompletedInputs;
use lctx_model::domain::uniform_outcome::{AnalysisUniformInputs, UniformNotRequested};

pub(super) fn unrequested(
    binding: UpperStage,
    profile: Profile,
    prepared: &PreparedCompilation,
) -> bool {
    match binding {
        UpperStage::Local
        | UpperStage::Base
        | UpperStage::Completion
        | UpperStage::SourceCalls
        | UpperStage::Enriched
        | UpperStage::Models
        | UpperStage::Summary => profile == Profile::Catalog,
        UpperStage::Embedding => !prepared.text.requested,
        UpperStage::Analytic => !analytics::build::requested(prepared.settings()),
        _ => false,
    }
}
/// Operation-local exact-source preparation. The retained CompletedInputs are the
/// actual protected dependencies; no digest or nominal descriptor grants reuse.
pub(super) struct Preparation {
    state: tokio::sync::Mutex<PreparedFragments>,
}
struct PreparedFragments {
    entries: Vec<std::sync::Arc<Fragment>>,
    charge: charged::StateCharge,
}
struct Fragment {
    access: CompletedInputs,
    inputs: AnalysisUniformInputs,
    _charge: charged::StateCharge,
}
impl Preparation {
    pub(super) fn new(budget: &resources::ResourceBudget) -> Result<Self, ModelError> {
        let mut charge = charged::StateCharge::new(budget, "compact metadata registry");
        charge.grow(size_of::<Self>() + 2 * size_of::<usize>())?;
        Ok(Self {
            state: tokio::sync::Mutex::new(PreparedFragments {
                entries: Vec::new(),
                charge,
            }),
        })
    }
    async fn acquire(
        &self,
        access: &CompletedInputs,
        required: &[ValidationInput],
        workspace: &Workspace,
    ) -> Result<std::sync::Arc<Fragment>, ModelError> {
        workspace.cancellation().check()?;
        let mut allocation =
            charged::StateCharge::new(workspace.budget(), "compact metadata fragment");
        allocation.grow(required.len() * (size_of::<ValidationInput>() + 128))?;
        let selected = access.select(required)?;
        // Serialize only construction of this operation's small finite metadata. A
        // cancelled initializer drops its private partial state and publishes no entry.
        let mut state = self.state.lock().await;
        if let Some(entry) = state
            .entries
            .iter()
            .find(|entry| entry.access.same_preparation_sources(&selected))
        {
            return Ok(entry.clone());
        }
        allocation.grow(size_of::<Fragment>() + 2 * size_of::<usize>())?;
        let inputs = load_fragment(&selected, required, workspace).await?;
        workspace.cancellation().check()?;
        if state.entries.len() == state.entries.capacity() {
            let additional = state.entries.capacity().max(4);
            state
                .charge
                .grow(additional * size_of::<std::sync::Arc<Fragment>>())?;
            state.entries.reserve_exact(additional);
        }
        let fragment = std::sync::Arc::new(Fragment {
            access: selected,
            inputs,
            _charge: allocation,
        });
        state.entries.push(fragment.clone());
        Ok(fragment)
    }
}
/// Every whole-fragment stream must exhaust the exact immutable selected view.
struct ExactRows {
    expected: u64,
    actual: u64,
}
impl ExactRows {
    fn new(expected: i64) -> Result<Self, ModelError> {
        Ok(Self {
            expected: u64::try_from(expected)
                .map_err(|_| ModelError::Conflict("compact metadata negative row count"))?,
            actual: 0,
        })
    }
    fn accept(&mut self, count: usize) -> Result<(), ModelError> {
        self.actual = self
            .actual
            .checked_add(count as u64)
            .ok_or(ModelError::Conflict("compact metadata count overflow"))?;
        if self.actual > self.expected {
            return Err(ModelError::Conflict("compact metadata exceeds actual view"));
        }
        Ok(())
    }
    fn finish(self) -> Result<(), ModelError> {
        if self.actual != self.expected {
            return Err(ModelError::Conflict("compact metadata exact EOF count"));
        }
        Ok(())
    }
}
type FragmentLoader = for<'a> fn(
    &'a CompletedInputs,
    &'a datafusion::prelude::SessionContext,
    &'a Workspace,
    &'a mut crate::consumed_rows::ConsumedInputs,
    &'a mut AnalysisUniformInputs,
) -> futures::future::BoxFuture<'a, Result<(), ModelError>>;
fn load_relation<'a, R: Record>(
    access: &'a CompletedInputs,
    session: &'a datafusion::prelude::SessionContext,
    workspace: &'a Workspace,
    consumed: &'a mut crate::consumed_rows::ConsumedInputs,
    inputs: &'a mut AnalysisUniformInputs,
) -> futures::future::BoxFuture<'a, Result<(), ModelError>> {
    Box::pin(async move {
        while let Some((declaration, permit)) = consumed.next::<R>(access)? {
            let mut count = ExactRows::new(permit.source().rows())?;
            if R::NAME == source::SourceArtifact::NAME {
                let selected = format!(
                    "SELECT {} FROM {}",
                    analysis::expected::CoverageAdmission::artifact_property_columns(),
                    crate::consumed_rows::identifier(&access.table_for(&declaration)?)
                );
                crate::consumed_rows::stream_query_at(
                    &permit,
                    &declaration,
                    access,
                    session,
                    &selected,
                    |_, batch| {
                        workspace.cancellation().check()?;
                        count.accept(batch.num_rows())?;
                        inputs.visit_artifact_properties(batch)
                    },
                )
                .await?;
            } else {
                crate::consumed_rows::stream_at(
                    &permit,
                    &declaration,
                    access,
                    session,
                    |_, batch| {
                        workspace.cancellation().check()?;
                        count.accept(batch.num_rows())?;
                        inputs.visit(R::NAME, batch)
                    },
                )
                .await?;
            }
            count.finish()?;
        }
        Ok(())
    })
}
async fn load_fragment(
    access: &CompletedInputs,
    required: &[ValidationInput],
    workspace: &Workspace,
) -> Result<AnalysisUniformInputs, ModelError> {
    let mut inputs = AnalysisUniformInputs::new(access.profile(), workspace.budget())?;
    let session = access.session(workspace).await?;
    let mut consumed =
        crate::consumed_rows::ConsumedInputs::new(required.to_vec(), workspace.budget())?;
    macro_rules! load {($($field:ident:$ty:ty,)*)=>{{
        const LOADERS: &[FragmentLoader] = &[$(load_relation::<$ty>,)*];
        for loader in LOADERS {
            loader(access, &session, workspace, &mut consumed, &mut inputs).await?;
        }
    }};}
    lctx_model::uniform_common_inputs!(load);
    lctx_model::uniform_authored_inputs!(load);
    consumed.finish(access.name())?;
    workspace.cancellation().check()?;
    Ok(inputs)
}
pub(super) async fn produce(
    access: CompletedInputs,
    output: ProducerOutput,
    workspace: &Workspace,
    declaration: &Stage,
    preparation: &Preparation,
) -> Result<(), ModelError> {
    let common = preparation
        .acquire(&access, &AnalysisUniformInputs::common_inputs(), workspace)
        .await?;
    let mut inputs = AnalysisUniformInputs::new(access.profile(), workspace.budget())?;
    inputs.use_common(&common.inputs)?;
    // Every authored relation has its own exact-source compatibility key. Changing a
    // definition cannot invalidate immutable artifact preparation or unrelated options.
    for required in AnalysisUniformInputs::authored_inputs(declaration.name)? {
        let fragment = preparation
            .acquire(&access, std::slice::from_ref(&required), workspace)
            .await?;
        inputs.use_authored(required.name(), &fragment.inputs)?;
    }
    let mut sources = charged::ChargedVec::default();
    let mut source_charge =
        charged::StateCharge::new(workspace.budget(), "compact owner ordered source snapshots");
    for source in access.snapshots() {
        sources.push(&mut source_charge, source)?;
    }
    let rows = inputs.declarations(declaration, &sources)?;
    output.declare_async::<UniformNotRequested>().await?;
    output.declare_empty_outputs().await?;
    for row in rows.iter() {
        output.push(row.clone()).await?;
    }
    output.finish(ProviderOutcome::Complete).await
}

#[cfg(test)]
mod metadata_controls {
    use super::*;
    #[test]
    fn compact_metadata_eof_refuses_early_extra_and_overflow_before_publication() {
        let mut exact = ExactRows::new(3).unwrap();
        exact.accept(1).unwrap();
        exact.accept(2).unwrap();
        exact.finish().unwrap();
        let mut early = ExactRows::new(3).unwrap();
        early.accept(2).unwrap();
        assert!(early.finish().is_err());
        let mut extra = ExactRows::new(3).unwrap();
        extra.accept(3).unwrap();
        assert!(extra.accept(1).is_err());
        assert!(ExactRows::new(-1).is_err());
        let mut overflow = ExactRows {
            expected: u64::MAX,
            actual: u64::MAX,
        };
        assert!(overflow.accept(1).is_err());
    }
    #[tokio::test]
    async fn completed_uniform_common_registry_reuses_exact_sources_and_never_publishes_refused_initialization()
     {
        use crate::workspace::WorkspaceOptions;
        use futures::FutureExt;
        let budget = resources::ResourceBudget::fixed(128 << 20).unwrap();
        let workspace = Workspace::with_budget(
            Arc::new(lctx_model::domain::model().unwrap()),
            WorkspaceOptions {
                memory_bytes: 128 << 20,
                ..Default::default()
            },
            budget.clone(),
            crate::test_native::store_with_budget(budget.clone()),
        )
        .unwrap();
        let result = std::panic::AssertUnwindSafe(async {
            let mut outputs = AnalysisUniformInputs::common_inputs().iter().map(ValidationInput::name).collect::<Vec<_>>();
            outputs.push(input::Package::NAME);
            let output = workspace.output("uniform-common-control-source", Profile::Catalog, ContentHash::of(b"uniform-common-control"),
                workspace.inputs("uniform-common-control-source", Profile::Catalog, []).unwrap(), outputs.clone());
            macro_rules! declare {($($field:ident:$ty:ty,)*)=>{$(output.declare_async::<$ty>().await.unwrap();)*};}
            lctx_model::uniform_common_inputs!(declare);
            output.declare_async::<input::Package>().await.unwrap();
            output.push(input::InputRevision::from_entries(vec![]).unwrap()).await.unwrap();
            output.finish(ProviderOutcome::Complete).await.unwrap();
            let required = AnalysisUniformInputs::common_inputs();
            let access = workspace.inputs("uniform-common-control-consumer", Profile::Catalog, outputs).unwrap();
            let preparation = Preparation::new(&budget).unwrap();
            let first = preparation.acquire(&access, &required, &workspace).await.unwrap();
            // These consumer names select different invocation program identities.
            // Shared metadata has its own fixed preparation algorithm identity.
            let other_owner = workspace.inputs("uniform-common-different-owner-program", Profile::Catalog,
                required.iter().rev().map(ValidationInput::name)).unwrap();
            let second = preparation.acquire(&other_owner, &required, &workspace).await.unwrap();
            assert!(Arc::ptr_eq(&first, &second), "the same ready fragment returns before a second physical loader can start");
            assert_eq!(preparation.state.lock().await.entries.len(), 1);
            let narrow = preparation.acquire(&access, &[ValidationInput::of::<input::InputRevision>(&["id"])], &workspace).await.unwrap();
            assert!(!Arc::ptr_eq(&first, &narrow), "a changed dependency set never borrows the broader ready index");
            assert_eq!(preparation.state.lock().await.entries.len(), 2);
            assert!(preparation.acquire(&access, &[ValidationInput::of::<input::Package>(&["id"])], &workspace).await.is_err());
            assert_eq!(preparation.state.lock().await.entries.len(), 2, "no entry from a failed initializer");
            let cancelled = Preparation::new(&budget).unwrap();
            workspace.cancellation().cancel();
            assert!(cancelled.acquire(&access, &required, &workspace).await.is_err());
            assert!(cancelled.state.lock().await.entries.is_empty());
            drop(first); drop(second); drop(narrow); drop(preparation); drop(cancelled); drop(access); drop(other_owner);
        }).catch_unwind().await;
        let mut finality = workspace.drain_owned_report().await;
        if finality.local == completion::LocalState::Terminal
            && finality.remote == completion::RemoteState::Confirmed
            && finality
                .failures
                .iter()
                .all(|failure| failure.error.permits_storage_cleanup())
        {
            finality.merge(workspace.native().abandon_finality_report().await);
        } else {
            finality
                .storage
                .push(completion::StorageState::Orphan(format!(
                    "native_attempt:{}",
                    workspace.native().attempt().hex()
                )));
        }
        let signal = workspace.complete_signal(&finality).await;
        finality.step("uniform metadata control workspace signal", signal);
        if let Err(panic) = result {
            if let Err(error) = completion::complete::<()>(Ok(()), finality) {
                eprintln!("uniform metadata cleanup after panic: {error:#?}");
            }
            std::panic::resume_unwind(panic);
        }
        completion::complete::<()>(Ok(()), finality).unwrap();
    }
}
