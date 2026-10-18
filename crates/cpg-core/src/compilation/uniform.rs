//! Whole-owner unrequested production. No per-method invocation or outcome is created.
use super::*;
use crate::workspace::CompletedInputs;
use lctx_model::domain::uniform_outcome::{AnalysisUniformInputs, UniformNotRequested};

pub(super) fn unrequested(
    binding: UpperStage,
    profile: Profile,
    prepared: &PreparedCompilation,
) -> bool {
    prepared.demand(binding, profile) == analysis::owner::AnalysisDemand::UniformNotRequested
}
/// Operation-local exact-source preparation. The retained CompletedInputs are the
/// actual protected dependencies; no digest or nominal descriptor grants reuse.
pub(super) struct Preparation {
    state: std::sync::Mutex<PreparedFragments>,
}
struct PreparedFragments {
    entries: Vec<std::sync::Arc<PendingFragment>>,
    charge: charged::StateCharge,
}
type FragmentResult = futures::future::Shared<
    BoxFuture<'static, Result<std::sync::Arc<Fragment>, std::sync::Arc<ModelError>>>,
>;
struct PendingFragment {
    sources: std::sync::Arc<FragmentSources>,
    terminal: FragmentResult,
}
/// The fixed preparation algorithm reads these exact protected sources under these
/// typed relation/order contracts. Consumer names and incidental declaration order
/// are deliberately absent from the compatibility test.
struct FragmentSources {
    access: CompletedInputs,
    required: Vec<ValidationInput>,
    _charge: charged::StateCharge,
}
impl FragmentSources {
    fn matches(&self, access: &CompletedInputs, required: &[ValidationInput]) -> bool {
        self.access.same_preparation_sources(access)
            && self.required.len() == required.len()
            && self.required.iter().all(|input| {
                required.iter().any(|candidate| {
                    input.type_id() == candidate.type_id()
                        && input.name() == candidate.name()
                        && input.prefix() == candidate.prefix()
                        && input.order() == candidate.order()
                })
            })
    }
}
struct Fragment {
    inputs: AnalysisUniformInputs,
    // A useful borrower outlives the registry without dropping its protected source
    // or the reservation which admitted preparation metadata.
    _sources: std::sync::Arc<FragmentSources>,
}
impl Preparation {
    pub(super) fn new(budget: &resources::ResourceBudget) -> Result<Self, ModelError> {
        let mut charge = charged::StateCharge::new(budget, "compact metadata registry");
        charge.grow(size_of::<Self>() + 2 * size_of::<usize>())?;
        Ok(Self {
            state: std::sync::Mutex::new(PreparedFragments {
                entries: Vec::new(),
                charge,
            }),
        })
    }
    async fn acquire(
        &self,
        access: &CompletedInputs,
        required: &[ValidationInput],
        workspace: &Arc<Workspace>,
    ) -> Result<std::sync::Arc<Fragment>, ModelError> {
        self.acquire_with(access, required, workspace, |sources, owner| async move {
            load_fragment(&sources.access, &sources.required, &owner).await
        })
        .await
    }
    /// The sole production caller supplies the fixed loader above. Keeping launch
    /// separate from result borrowing makes accepted work independent of waiters.
    async fn acquire_with<F, Fut>(
        &self,
        access: &CompletedInputs,
        required: &[ValidationInput],
        workspace: &Arc<Workspace>,
        load: F,
    ) -> Result<std::sync::Arc<Fragment>, ModelError>
    where
        F: FnOnce(Arc<FragmentSources>, Arc<Workspace>) -> Fut + Send + 'static,
        Fut: std::future::Future<Output = Result<AnalysisUniformInputs, ModelError>>
            + Send
            + 'static,
    {
        workspace.cancellation().check()?;
        let mut allocation =
            charged::StateCharge::new(workspace.budget(), "compact metadata fragment");
        allocation.grow(required.iter().fold(0_usize, |bytes, input| {
            bytes
                .saturating_add(size_of::<ValidationInput>() + 128)
                .saturating_add(input.order().len().saturating_mul(size_of::<&str>()))
        }))?;
        if required.iter().enumerate().any(|(index, input)| {
            required[..index].iter().any(|previous| {
                input.name() == previous.name() && input.prefix() == previous.prefix()
            })
        }) {
            return Err(ModelError::Conflict("duplicate compact preparation input"));
        }
        let selected = access.select(required)?;
        let terminal = {
            // Only exact-key lookup, admission and owned task registration occur under
            // this lock. Session acquisition and all stream/EOF work run outside it.
            let mut state = self.state.lock().map_err(|_| {
                ModelError::Conflict("compact metadata preparation registry poisoned")
            })?;
            if let Some(entry) = state
                .entries
                .iter()
                .find(|entry| entry.sources.matches(&selected, required))
            {
                entry.terminal.clone()
            } else {
                allocation.grow(
                    size_of::<FragmentSources>()
                        + size_of::<PendingFragment>()
                        + size_of::<Fragment>()
                        + 6 * size_of::<usize>(),
                )?;
                if state.entries.len() == state.entries.capacity() {
                    let additional = state.entries.capacity().max(4);
                    state
                        .charge
                        .grow(additional.saturating_mul(size_of::<Arc<PendingFragment>>()))?;
                    state.entries.reserve_exact(additional);
                }
                let sources = Arc::new(FragmentSources {
                    access: selected,
                    required: required.to_vec(),
                    _charge: allocation,
                });
                let protected = sources.clone();
                let owner = workspace.clone();
                // The Workspace first reserves/registers observation of the actual body
                // result, including cancellation before its first poll and panics. A
                // JoinHandle-only observer would lose body errors when waiters disappear.
                let launched = workspace.spawn_preparation(async move {
                    let inputs = load(protected.clone(), owner.clone()).await?;
                    owner.cancellation().check()?;
                    Ok(Fragment {
                        inputs,
                        _sources: protected,
                    })
                });
                let terminal = match launched {
                    Ok(terminal) => terminal,
                    Err(error) => futures::future::ready(Err(Arc::new(error)))
                        .boxed()
                        .shared(),
                };
                state.entries.push(Arc::new(PendingFragment {
                    sources,
                    terminal: terminal.clone(),
                }));
                terminal
            }
        };
        // Terminal failures remain in the registry under their original exact key.
        // No waiter cancellation can turn them into an implicit effect retry.
        terminal.await.map_err(ModelError::SharedCause)
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
    workspace: &Arc<Workspace>,
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
    async fn completed_uniform_common_registry_single_flight_and_retained_failure() {
        use crate::workspace::WorkspaceOptions;
        use futures::FutureExt;
        let budget = resources::ResourceBudget::fixed(128 << 20).unwrap();
        let workspace = Workspace::private_with_budget(
            Arc::new(lctx_model::domain::model().unwrap()),
            WorkspaceOptions {
                memory_bytes: 128 << 20,
                ..Default::default()
            },
            budget.clone(),
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
            let reversed = required.iter().rev().cloned().collect::<Vec<_>>();
            let second = preparation.acquire(&other_owner, &reversed, &workspace).await.unwrap();
            assert!(Arc::ptr_eq(&first, &second), "the same ready fragment returns before a second physical loader can start");
            assert_eq!(preparation.state.lock().unwrap().entries.len(), 1);
            let narrow = preparation.acquire(&access, &[ValidationInput::of::<input::InputRevision>(&["id"])], &workspace).await.unwrap();
            assert!(!Arc::ptr_eq(&first, &narrow), "a changed dependency set never borrows the broader ready index");
            assert_eq!(preparation.state.lock().unwrap().entries.len(), 2);
            let differently_ordered = preparation.acquire(&access,
                &[ValidationInput::of::<input::InputRevision>(&[])], &workspace).await.unwrap();
            assert!(!Arc::ptr_eq(&narrow, &differently_ordered), "a different typed ordering contract earns its own preparation");
            let package = [ValidationInput::of::<input::Package>(&["id"])];
            let failure = match preparation.acquire(&access, &package, &workspace).await {
                Err(ModelError::SharedCause(error)) => error,
                _ => panic!("unsupported initializer did not retain its typed error"),
            };
            let repeated = match preparation.acquire(&access, &package, &workspace).await {
                Err(ModelError::SharedCause(error)) => error,
                _ => panic!("failed exact key was retried or its error changed"),
            };
            assert!(Arc::ptr_eq(&failure, &repeated));
            assert_eq!(preparation.state.lock().unwrap().entries.len(), 4, "failed initialization remains terminal under its exact key");

            // Hold both actual loaders at explicit admission barriers. A same-key
            // waiter must share the first owner; a distinct key must enter without
            // releasing it. No elapsed-time assumption establishes concurrency.
            let concurrent = Preparation::new(&budget).unwrap();
            let (entered, mut entering) = tokio::sync::mpsc::unbounded_channel();
            let release = Arc::new(tokio::sync::Semaphore::new(0));
            let blocked_loader = |entered: tokio::sync::mpsc::UnboundedSender<()>, release: Arc<tokio::sync::Semaphore>| {
                move |sources: Arc<FragmentSources>, owner: Arc<Workspace>| async move {
                    entered.send(()).map_err(ModelError::codec)?;
                    release.acquire_owned().await.map_err(ModelError::codec)?.forget();
                    load_fragment(&sources.access, &sources.required, &owner).await
                }
            };
            let mut first_waiter = Box::pin(concurrent.acquire_with(&access, &required, &workspace,
                blocked_loader(entered.clone(), release.clone())));
            assert!(futures::poll!(&mut first_waiter).is_pending());
            entering.recv().await.unwrap();
            let mut same_waiter = Box::pin(concurrent.acquire(&other_owner, &required, &workspace));
            assert!(futures::poll!(&mut same_waiter).is_pending());
            assert_eq!(concurrent.state.lock().unwrap().entries.len(), 1);
            let narrow_required = [ValidationInput::of::<input::InputRevision>(&["id"])];
            let mut distinct_waiter = Box::pin(concurrent.acquire_with(&access, &narrow_required, &workspace,
                blocked_loader(entered, release.clone())));
            assert!(futures::poll!(&mut distinct_waiter).is_pending());
            entering.recv().await.unwrap();
            assert_eq!(concurrent.state.lock().unwrap().entries.len(), 2);
            // Losing the initiating waiter does not cancel or reissue accepted work.
            drop(first_waiter);
            release.add_permits(2);
            let shared = same_waiter.await.unwrap();
            let distinct = distinct_waiter.await.unwrap();
            assert!(!Arc::ptr_eq(&shared, &distinct));
            let borrowed = concurrent.acquire(&access, &required, &workspace).await.unwrap();
            assert!(Arc::ptr_eq(&shared, &borrowed));
            drop(concurrent);
            assert!(Arc::ptr_eq(&shared, &borrowed), "last useful borrowers keep completed backing after registry release");
            drop(shared); drop(distinct); drop(borrowed);
            let cancelled = Preparation::new(&budget).unwrap();
            workspace.cancellation().cancel();
            assert!(cancelled.acquire(&access, &required, &workspace).await.is_err());
            assert!(cancelled.state.lock().unwrap().entries.is_empty());
            drop(first); drop(second); drop(narrow); drop(differently_ordered); drop(preparation); drop(cancelled); drop(access); drop(other_owner);
            failure
        }).catch_unwind().await;
        let mut finality = workspace.drain_owned_report().await;
        let signal = workspace.complete_signal(&finality).await;
        finality.step("uniform metadata control workspace signal", signal);
        let expected = match result {
            Ok(expected) => expected,
            Err(panic) => {
                if let Err(error) = completion::complete::<()>(Ok(()), finality) {
                    eprintln!("uniform metadata cleanup after panic: {error:#?}");
                }
                std::panic::resume_unwind(panic);
            }
        };
        assert_eq!(
            finality.failures.len(),
            1,
            "only the intentional initializer failure reaches drainage: {finality:#?}"
        );
        assert!(
            std::ptr::eq(
                finality.failures[0].error.primary().unwrap(),
                expected.primary().unwrap(),
            ),
            "drainage preserves the same initializer error even after every registry waiter drops"
        );
        finality.failures.clear();
        completion::complete::<()>(Ok(()), finality).unwrap();
    }
}
