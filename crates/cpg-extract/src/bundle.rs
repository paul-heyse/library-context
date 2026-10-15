//! Native providers consume completed typed inputs and emit bounded batches into an attempt-owned
//! output. A dedicated thread supplies the native stack; cancellation drains it before releasing
//! captured inputs. Neither the provider nor its output requires a database or publication grant.
use crate::{acquisition::AcquiredInput, assembly::Attacher};
use lctx_model::domain::{
    Batch, ContentHash, Infrastructure, KeySink, ModelError, Record, ValidatedModel,
    batching::{BatchWriter, TransferLimits},
    resources::ResourceBudget,
    source::Occurrence,
    stages::{Profile, ProviderOutcome, Stage},
};
use std::{
    any::TypeId,
    collections::BTreeMap,
    ffi::OsString,
    future::Future,
    panic::AssertUnwindSafe,
    pin::Pin,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
};
use tokio::sync::oneshot;
/// Native environment switches refused by the single facts driver.
pub const REFUSED_ENV: &[&str] = &["PYREFLY_STACK_SIZE", "PYREFLY_FIXPOINT_DETAILS"];
pub const REFUSED_ENV_PREFIX: &str = "PYSA_DUMP";

/// Provider threads host Pyrefly's and ty's recursion.
pub const PROVIDER_STACK_BYTES: usize = 512 << 20;

/// A provider's build identity (F11): the lockfile, the Pyrefly patch that every in-process
/// provider links, and the provider's own sources.
pub fn build_digest(sources: &[&str]) -> ContentHash {
    let mut digest = KeySink::new("provider-build");
    digest.part(
        b"source-closure",
        env!("LCTX_PRODUCER_SOURCE_DIGEST").as_bytes(),
    );
    digest.part(b"lockfile", include_str!("../../../Cargo.lock").as_bytes());
    digest.part(
        b"pyrefly-patch",
        include_str!("../../../third_party/pyrefly-1.4.0-dev.3.patch").as_bytes(),
    );
    digest.part(
        b"ruff-patch",
        include_str!("../../../third_party/ruff-0.16.10.patch").as_bytes(),
    );
    for source in sources {
        digest.part(b"source", source.as_bytes());
    }
    digest.finish()
}

/// Refuse analyzer knobs that would make facts depend on the caller's environment.
pub fn refuse_ambient(
    variables: impl IntoIterator<Item = (OsString, OsString)>,
) -> Result<(), ModelError> {
    for (name, _) in variables {
        let name = name.to_string_lossy();
        if REFUSED_ENV.contains(&name.as_ref()) || name.starts_with(REFUSED_ENV_PREFIX) {
            return Err(ModelError::Invalid(format!(
                "ambient analyzer configuration {name} is refused"
            )));
        }
    }
    Ok(())
}

/// The frozen inputs of one attempt, in analysis order: the library, then its corpus.
pub struct CapturedInputs {
    inputs: Vec<AcquiredInput>,
    config: crate::native_context::NativeContextConfig,
}
impl CapturedInputs {
    pub fn new(
        inputs: Vec<AcquiredInput>,
        config: crate::native_context::NativeContextConfig,
    ) -> Self {
        Self { inputs, config }
    }
    pub fn config(&self) -> &crate::native_context::NativeContextConfig {
        &self.config
    }
    pub fn inputs(&self) -> &[AcquiredInput] {
        &self.inputs
    }
}

/// Retains the exact captured closure through joined providers and accepted ingress cleanup.
/// Persistent mismatch poisons this run; a later provider cannot reopen it as a fresh capture.
#[derive(Clone)]
pub struct CapturedTreeRunGuard {
    captured: Arc<CapturedInputs>,
    poisoned: Arc<AtomicBool>,
}
impl CapturedTreeRunGuard {
    pub fn open(captured: Arc<CapturedInputs>) -> Result<Self, ModelError> {
        let guard = Self {
            captured,
            poisoned: Arc::new(AtomicBool::new(false)),
        };
        guard.verify_closed()?;
        Ok(guard)
    }
    fn check(&self) -> Result<(), ModelError> {
        if self.poisoned.load(Ordering::Acquire) {
            return Err(ModelError::Conflict("captured tree run is poisoned"));
        }
        Ok(())
    }
    pub fn verify_closed(&self) -> Result<(), ModelError> {
        self.check()?;
        for input in self.captured.inputs() {
            if let Err(error) = input.captured().verify() {
                self.poisoned.store(true, Ordering::Release);
                return Err(ModelError::codec(error));
            }
        }
        Ok(())
    }
}

/// Fallible typed batches streamed from one completed native-provider input.
pub type ProviderBatchStream<R> = Box<dyn Iterator<Item = Result<Batch<R>, ModelError>> + Send>;

/// Ordinary completed-input and batch-output boundary. Implementations own temporary storage;
/// these methods do not grant database reads or certify publication.
pub trait ProviderSink: Send + Sync + 'static {
    fn read<R: Record>(&self) -> Result<ProviderBatchStream<R>, ModelError>;
    fn declare<R: Record>(&self) -> Result<(), ModelError>;
    fn write<R: Record>(&self, batch: Batch<R>) -> Result<(), ModelError>;
    fn contribute<R: Record>(&self, batch: Batch<R>) -> Result<(), ModelError>;
    /// Record the outcome after all writes drain. Workspace completion/admission belongs to the compiler.
    fn finish(&self, outcome: ProviderOutcome) -> Result<(), ModelError>;
    /// Wake blocked read/write methods when the caller abandons the provider.
    fn cancel(&self) {}
    /// Observe explicit attempt cancellation while retaining the provider future for drainage.
    fn cancelled(&self) -> Pin<Box<dyn Future<Output = ()> + Send + '_>> {
        Box::pin(std::future::pending())
    }
    /// Retain the interrupted thread's drainage under the attempt owner when applicable.
    fn drain_provider(
        &self,
        thread: std::thread::JoinHandle<()>,
    ) -> oneshot::Receiver<Result<(), ModelError>> {
        let (done, finished) = oneshot::channel();
        let dispatch = tracing::dispatcher::get_default(Clone::clone);
        let span = tracing::Span::current();
        let join = move || {
            tracing::dispatcher::with_default(&dispatch, || {
                span.in_scope(|| {
                    let _ = done.send(thread.join().map_err(|_| {
                        ModelError::Invalid("provider thread panicked while draining".into())
                    }));
                })
            })
        };
        if let Ok(runtime) = tokio::runtime::Handle::try_current() {
            runtime.spawn_blocking(join);
        } else {
            join();
        }
        finished
    }
}

/// A provider declares its semantic input/output inventory and effect.
pub trait Declared {
    fn declaration(&self, profile: Profile) -> Stage;
}
/// One provider as one scheduled stage: `run` executes on the provider thread and reports the
/// provider's outcome.
pub trait ProviderStage<S: ProviderSink + 'static>: Declared + Send {
    fn run(&mut self, context: &mut StageContext<S>) -> Result<ProviderOutcome, ModelError>;
}

/// Cancellation belongs to the whole provider run, including native work between sink calls.
#[derive(Default)]
struct ProviderCancellation {
    cancelled: AtomicBool,
    registrations: Mutex<CancellationRegistrations>,
}
#[derive(Default)]
struct CancellationRegistrations {
    next: u64,
    callbacks: BTreeMap<u64, Box<dyn Fn() + Send + Sync>>,
}
impl ProviderCancellation {
    fn cancel(&self) {
        let registrations = self
            .registrations
            .lock()
            .expect("provider cancellation ownership");
        self.cancelled.store(true, Ordering::Release);
        for callback in registrations.callbacks.values() {
            callback();
        }
    }
    fn check(&self) -> Result<(), ModelError> {
        if self.cancelled.load(Ordering::Acquire) {
            Err(ModelError::Invalid("provider run cancelled".into()))
        } else {
            Ok(())
        }
    }
    fn register(
        self: &Arc<Self>,
        callback: impl Fn() + Send + Sync + 'static,
    ) -> CancellationRegistration {
        let mut registrations = self
            .registrations
            .lock()
            .expect("provider cancellation ownership");
        let generation = registrations.next;
        registrations.next = generation
            .checked_add(1)
            .expect("provider cancellation generation overflow");
        if self.cancelled.load(Ordering::Acquire) {
            callback();
        }
        registrations
            .callbacks
            .insert(generation, Box::new(callback));
        CancellationRegistration {
            owner: self.clone(),
            generation,
        }
    }
}
/// A native cancellation callback is retained through every wave and lazy query in its session.
/// Dropping an old registration removes only that generation, never a later session's handle.
pub struct CancellationRegistration {
    owner: Arc<ProviderCancellation>,
    generation: u64,
}
impl CancellationRegistration {
    pub fn check(&self) -> Result<(), ModelError> {
        self.owner.check()
    }
}
impl Drop for CancellationRegistration {
    fn drop(&mut self) {
        self.owner
            .registrations
            .lock()
            .expect("provider cancellation ownership")
            .callbacks
            .remove(&self.generation);
    }
}

/// What a running provider may do: emit its declared outputs, contribute declared vocabulary, read
/// its declared inputs' handoffs, attach to handed-off occurrences, and read the captured inputs.
/// The first refusal poisons the context, so the attempt fails even if the provider ignores it.
pub struct StageContext<S: ProviderSink + 'static> {
    stage: Stage,
    profile: Profile,
    model: Arc<ValidatedModel>,
    budget: ResourceBudget,
    limits: TransferLimits,
    captured: Arc<CapturedInputs>,
    captured_run: CapturedTreeRunGuard,
    sink: Arc<S>,
    cancellation: Arc<ProviderCancellation>,
    attacher: Option<Attacher>,
    outputs: Vec<(TypeId, Mode, Box<dyn Flush<S>>)>,
    contributions: Vec<(TypeId, Box<dyn Flush<S>>)>,
    failed: Arc<AtomicBool>,
}
#[derive(Clone, Copy, PartialEq, Eq)]
enum Mode {
    Declared,
    Rows,
    Batches,
}
impl<S: ProviderSink + 'static> StageContext<S> {
    /// Native callbacks must only signal cancellation; joining remains the provider's job.
    pub fn register_cancellation(
        &self,
        callback: impl Fn() + Send + Sync + 'static,
    ) -> CancellationRegistration {
        self.cancellation.register(callback)
    }
    pub fn check_cancelled(&self) -> Result<(), ModelError> {
        self.cancellation.check()
    }
    pub fn stage(&self) -> &Stage {
        &self.stage
    }
    pub fn profile(&self) -> Profile {
        self.profile
    }
    pub fn model(&self) -> &ValidatedModel {
        &self.model
    }
    pub fn budget(&self) -> &ResourceBudget {
        &self.budget
    }
    /// The attempt's captured inputs, shared so a provider can emit while it walks them.
    pub fn captured(&self) -> Arc<CapturedInputs> {
        self.captured.clone()
    }
    /// Open one of the stage's outputs. Every output is declared, and one with no rows is written
    /// explicitly empty.
    pub fn declare<R: Record>(&mut self) -> Result<(), ModelError> {
        self.guard(|context| {
            if !context.stage.writes::<R>()
                || context
                    .outputs
                    .iter()
                    .any(|(t, ..)| *t == TypeId::of::<R>())
            {
                return Err(ModelError::Invalid(format!(
                    "{} cannot declare output {} twice or outside its stage",
                    context.stage.name,
                    R::NAME
                )));
            }
            let writer = BatchWriter::<R>::new(&context.budget, context.limits)?;
            context.outputs.push((
                TypeId::of::<R>(),
                Mode::Declared,
                Box::new(Writer {
                    writer,
                    contribution: false,
                }),
            ));
            context.sink.declare::<R>()
        })
    }
    /// Emit one row of a declared output through the provider's transfer-bounded writer.
    pub fn emit<R: Record>(&mut self, row: R) -> Result<(), ModelError> {
        self.guard(|context| {
            let model = context.model.clone();
            let full = context.output::<R>(Mode::Rows)?.writer.push(&model, row)?;
            match full {
                Some(batch) => context.sink.write(batch),
                None => Ok(()),
            }
        })
    }
    /// Emit a batch the provider built with its own transfer-bounded writer. An output is emitted
    /// by rows or by batches, never both.
    pub fn emit_batch<R: Record>(&mut self, batch: Batch<R>) -> Result<(), ModelError> {
        self.guard(|context| {
            context.output::<R>(Mode::Batches)?;
            context.sink.write(batch)
        })
    }
    /// Emit a complete header phase through bounded batches before another relation starts.
    /// This uses the provider's existing model, limits and allocation authority.
    pub fn emit_rows<R: Record>(
        &mut self,
        rows: impl Iterator<Item = R>,
    ) -> Result<(), ModelError> {
        let mut writer = BatchWriter::<R>::new(&self.budget, self.limits)?;
        let model = self.model.clone();
        for row in rows {
            if let Some(batch) = writer.push(&model, row)? {
                self.emit_batch(batch)?;
            }
        }
        if let Some(batch) = writer.finish(&model)? {
            self.emit_batch(batch)?;
        }
        Ok(())
    }
    /// Hand one row of shared vocabulary to the relation's writer, a later stage.
    pub fn contribute<R: Record>(&mut self, row: R) -> Result<(), ModelError> {
        self.guard(|context| {
            if !context.stage.contributes_to::<R>() {
                return Err(ModelError::Invalid(format!(
                    "{} cannot contribute {}",
                    context.stage.name,
                    R::NAME
                )));
            }
            if !context
                .contributions
                .iter()
                .any(|(t, _)| *t == TypeId::of::<R>())
            {
                let writer = BatchWriter::<R>::new(&context.budget, context.limits)?;
                context.contributions.push((
                    TypeId::of::<R>(),
                    Box::new(Writer {
                        writer,
                        contribution: true,
                    }),
                ));
            }
            let entry = context
                .contributions
                .iter_mut()
                .find(|(t, _)| *t == TypeId::of::<R>())
                .expect("inserted above");
            let full = entry
                .1
                .as_any()
                .downcast_mut::<Writer<R>>()
                .expect("writer matches its type id")
                .writer
                .push(&context.model, row)?;
            match full {
                Some(batch) => context.sink.contribute(batch),
                None => Ok(()),
            }
        })
    }
    /// Stream one declared input from its completed workspace view.
    pub fn input<R: Record>(&mut self) -> Result<ProviderBatchStream<R>, ModelError> {
        self.guard(|context| {
            if !context.stage.reads::<R>() {
                return Err(ModelError::Invalid(format!(
                    "{} does not consume {}",
                    context.stage.name,
                    R::NAME
                )));
            }
            let stream = context.sink.read::<R>()?;
            let failed = context.failed.clone();
            Ok(Box::new(stream.inspect(move |batch| {
                if batch.is_err() {
                    failed.store(true, Ordering::Release);
                }
            })) as ProviderBatchStream<R>)
        })
    }
    /// The attachment index over this stage's handed-off occurrences, built on first use.
    pub fn attacher(&mut self) -> Result<&Attacher, ModelError> {
        if self.attacher.is_none() {
            // Attachment needs a compact global occurrence index; retain only this named input
            // while constructing it, then release its batches before analysis begins.
            let occurrences = self
                .input::<Occurrence>()?
                .map(|batch| batch.map(Arc::new))
                .collect::<Result<Vec<_>, _>>()?;
            let attacher =
                self.guard(|context| Attacher::new(&occurrences, context.budget.clone()))?;
            self.attacher = Some(attacher);
        }
        Ok(self.attacher.as_ref().expect("built above"))
    }
    fn guard<T>(
        &mut self,
        operation: impl FnOnce(&mut Self) -> Result<T, ModelError>,
    ) -> Result<T, ModelError> {
        if self.failed.load(Ordering::Acquire)
            || self.cancellation.cancelled.load(Ordering::Acquire)
        {
            return Err(ModelError::Invalid(format!(
                "{} refused an earlier operation; the attempt fails",
                self.stage.name
            )));
        }
        let result = operation(self);
        if result.is_err() {
            self.failed.store(true, Ordering::Release);
        }
        result
    }
    fn output<R: Record>(&mut self, mode: Mode) -> Result<&mut Writer<R>, ModelError> {
        let name = self.stage.name;
        let (_, current, writer) = self
            .outputs
            .iter_mut()
            .find(|(t, ..)| *t == TypeId::of::<R>())
            .ok_or_else(|| {
                ModelError::Invalid(format!("{name} has not declared output {}", R::NAME))
            })?;
        if *current != Mode::Declared && *current != mode {
            return Err(ModelError::Invalid(format!(
                "{name} emits {} by rows or by batches, never both",
                R::NAME
            )));
        }
        *current = mode;
        Ok(writer
            .as_any()
            .downcast_mut::<Writer<R>>()
            .expect("writer matches its type id"))
    }
    /// Flush every writer after a successful run. A refusal the provider ignored fails the stage.
    fn close(
        mut self,
        result: Result<ProviderOutcome, ModelError>,
    ) -> Result<ProviderOutcome, ModelError> {
        let outcome = result?;
        if self.failed.load(Ordering::Acquire)
            || self.cancellation.cancelled.load(Ordering::Acquire)
        {
            return Err(ModelError::Invalid(format!(
                "{} ignored a refused operation",
                self.stage.name
            )));
        }
        if self.stage.outputs.iter().any(|declared| {
            !self
                .outputs
                .iter()
                .any(|(_, _, writer)| writer.name() == declared.name())
        }) {
            return Err(ModelError::Invalid(format!(
                "{} omitted a declared output, including its empty case",
                self.stage.name
            )));
        }
        self.captured_run.check()?;
        // Native providers have returned after joining all dependency waves. Assembly
        // consumes completed rows only; acquisition checks bytes during artifact emission.
        if !matches!(
            self.stage.name,
            crate::assembly::ASSEMBLE | crate::acquisition::ACQUIRE
        ) {
            self.captured_run.verify_closed()?;
        }
        let writers: Vec<_> = std::mem::take(&mut self.contributions)
            .into_iter()
            .map(|(_, w)| w)
            .chain(
                std::mem::take(&mut self.outputs)
                    .into_iter()
                    .map(|(.., w)| w),
            )
            .collect();
        for writer in writers {
            self.cancellation.check()?;
            writer.finish(&self.model, self.sink.as_ref())?;
        }
        self.cancellation.check()?;
        self.sink.finish(outcome)?;
        Ok(outcome)
    }
}

/// Run an actual native provider with explicit completed inputs and attempt-local output.
/// Dropping the future signals cancellation and registers the provider thread for drainage.
/// The attempt owner drains registered work before releasing native ownership.
#[allow(
    clippy::too_many_arguments,
    reason = "Explicit provider effects and captured inputs"
)]
pub async fn run_provider<S: ProviderSink>(
    provider: Box<dyn ProviderStage<S>>,
    profile: Profile,
    sink: Arc<S>,
    model: Arc<ValidatedModel>,
    captured: Arc<CapturedInputs>,
    budget: ResourceBudget,
    limits: TransferLimits,
) -> Result<ProviderOutcome, ModelError> {
    let guard = CapturedTreeRunGuard::open(captured)?;
    run_provider_guarded(provider, profile, sink, model, guard, budget, limits).await
}

/// The compiler shares one opening guard across its ordered facts provider schedule.
#[allow(
    clippy::too_many_arguments,
    reason = "Explicit provider effects and captured run ownership"
)]
pub async fn run_provider_guarded<S: ProviderSink>(
    provider: Box<dyn ProviderStage<S>>,
    profile: Profile,
    sink: Arc<S>,
    model: Arc<ValidatedModel>,
    captured_run: CapturedTreeRunGuard,
    budget: ResourceBudget,
    limits: TransferLimits,
) -> Result<ProviderOutcome, ModelError> {
    let stage =
        crate::contracts::bind_stage(provider.declaration(profile), &captured_run.captured)?;
    run_provider_bound_guarded(
        provider,
        stage,
        profile,
        sink,
        model,
        captured_run,
        budget,
        limits,
    )
    .await
}

/// Execute the same immutable declaration that owns the completed contribution.
#[allow(
    clippy::too_many_arguments,
    reason = "Explicit bound provider effects and capture ownership"
)]
pub async fn run_provider_bound_guarded<S: ProviderSink>(
    mut provider: Box<dyn ProviderStage<S>>,
    stage: Stage,
    profile: Profile,
    sink: Arc<S>,
    model: Arc<ValidatedModel>,
    captured_run: CapturedTreeRunGuard,
    budget: ResourceBudget,
    limits: TransferLimits,
) -> Result<ProviderOutcome, ModelError> {
    captured_run.check()?;
    let declared = provider.declaration(profile);
    if declared.digest() != stage.digest() || declared.profiles != stage.profiles {
        return Err(ModelError::Conflict(
            "scheduled bound provider declaration differs from executable",
        ));
    }
    let captured = captured_run.captured.clone();
    captured.config().check_profile(profile)?;
    captured.config().check_budget(&budget)?;
    if !stage.profiles.contains(&profile) {
        return Err(ModelError::Invalid(
            "provider is not requested by profile".into(),
        ));
    }
    let name = stage.name;
    let span = tracing::info_span!(target: "lctx_phase", "provider", producer = name);
    let dispatch = tracing::dispatcher::get_default(Clone::clone);
    let cancellation = Arc::new(ProviderCancellation::default());
    let (done, finished) = oneshot::channel();
    let mut context = StageContext {
        stage,
        profile,
        model,
        budget,
        limits,
        captured,
        captured_run,
        sink: sink.clone(),
        cancellation: cancellation.clone(),
        attacher: None,
        outputs: Vec::new(),
        contributions: Vec::new(),
        failed: Arc::new(AtomicBool::new(false)),
    };
    let thread = std::thread::Builder::new()
        .name(format!("lctx-{name}"))
        .stack_size(PROVIDER_STACK_BYTES)
        .spawn(move || {
            tracing::dispatcher::with_default(&dispatch, || {
                span.in_scope(|| {
                    let result =
                        std::panic::catch_unwind(AssertUnwindSafe(|| provider.run(&mut context)))
                            .unwrap_or_else(|_| {
                                Err(ModelError::Invalid(format!("the {name} provider panicked")))
                            });
                    let result = context.close(result);
                    drop(provider);
                    let _ = done.send(result);
                })
            })
        })
        .map_err(|error| ModelError::infrastructure(Infrastructure::Io, error))?;
    let mut drain = ProviderDrain {
        cancellation,
        thread: Some(thread),
        sink,
    };
    let mut finished = finished;
    let finished = tokio::select! {
        biased;
        () = drain.sink.cancelled() => {
            drain.cancellation.cancel();
            finished.await
        }
        result = &mut finished => result,
    };
    let result = finished.unwrap_or_else(|_| {
        Err(ModelError::Invalid(format!(
            "the {name} provider ended without a result"
        )))
    });
    if result.is_err() {
        drain.sink.cancel();
    }
    let thread = drain.thread.take().expect("owned provider thread");
    drain
        .sink
        .drain_provider(thread)
        .await
        .map_err(ModelError::codec)??;
    result
}
struct ProviderDrain<S: ProviderSink> {
    cancellation: Arc<ProviderCancellation>,
    thread: Option<std::thread::JoinHandle<()>>,
    sink: Arc<S>,
}
impl<S: ProviderSink> Drop for ProviderDrain<S> {
    fn drop(&mut self) {
        self.cancellation.cancel();
        if let Some(thread) = self.thread.take() {
            self.sink.cancel();
            self.sink.drain_provider(thread);
        }
    }
}
struct Writer<R: Record> {
    writer: BatchWriter<R>,
    contribution: bool,
}
trait Flush<S: ProviderSink>: Send {
    fn name(&self) -> &'static str;
    fn as_any(&mut self) -> &mut dyn std::any::Any;
    fn finish(self: Box<Self>, model: &ValidatedModel, sink: &S) -> Result<(), ModelError>;
}
impl<R: Record, S: ProviderSink> Flush<S> for Writer<R> {
    fn name(&self) -> &'static str {
        R::NAME
    }
    fn as_any(&mut self) -> &mut dyn std::any::Any {
        self
    }
    fn finish(self: Box<Self>, model: &ValidatedModel, sink: &S) -> Result<(), ModelError> {
        let Writer {
            writer,
            contribution,
        } = *self;
        if let Some(batch) = writer.finish(model)? {
            if contribution {
                sink.contribute(batch)?;
            } else {
                sink.write(batch)?;
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod cancellation_controls {
    use super::*;
    use std::sync::atomic::AtomicUsize;

    #[test]
    fn registration_and_cancellation_are_synchronized_in_both_orders() {
        for cancel_first in [false, true] {
            let owner = Arc::new(ProviderCancellation::default());
            let calls = Arc::new(AtomicUsize::new(0));
            if cancel_first {
                owner.cancel();
            }
            let observed = calls.clone();
            let registration = owner.register(move || {
                observed.fetch_add(1, Ordering::AcqRel);
            });
            if !cancel_first {
                owner.cancel();
            }
            assert_eq!(calls.load(Ordering::Acquire), 1);
            assert!(registration.check().is_err());
            drop(registration);
            assert!(owner.registrations.lock().unwrap().callbacks.is_empty());
        }
    }

    #[test]
    fn old_generation_unregister_keeps_live_peer_and_replacement() {
        let owner = Arc::new(ProviderCancellation::default());
        let old_calls = Arc::new(AtomicUsize::new(0));
        let observed = old_calls.clone();
        let old = owner.register(move || {
            observed.fetch_add(1, Ordering::AcqRel);
        });
        let calls = Arc::new(AtomicUsize::new(0));
        let observed = calls.clone();
        let current = owner.register(move || {
            observed.fetch_add(1, Ordering::AcqRel);
        });
        drop(old);
        owner.cancel();
        assert_eq!(old_calls.load(Ordering::Acquire), 0);
        assert_eq!(calls.load(Ordering::Acquire), 1);
        assert!(current.check().is_err());
        drop(current);
        assert!(owner.registrations.lock().unwrap().callbacks.is_empty());
    }

    #[test]
    fn native_dependency_wave_cannot_reuse_cancelled_transaction_as_complete() {
        std::thread::Builder::new()
            .stack_size(PROVIDER_STACK_BYTES)
            .spawn(|| {
                use pyrefly::state::{require::Require, state::State};
                use pyrefly_config::{
                    config::{ConfigFile, ConfigSource},
                    finder::ConfigFinder,
                };
                use pyrefly_python::{
                    module_path::ModulePath,
                    sys_info::{PythonPlatform, PythonVersion},
                };
                use pyrefly_util::{arc_id::ArcId, thread_pool::ThreadCount};
                let root = tempfile::tempdir().unwrap();
                std::fs::write(
                    root.path().join("main.py"),
                    "from dependency import Base\nclass Child(Base): pass\n",
                )
                .unwrap();
                std::fs::write(
                    root.path().join("dependency.py"),
                    "class Base: value: int = 1\n",
                )
                .unwrap();
                let mut config = ConfigFile {
                    source: ConfigSource::File(root.path().join("pyrefly.toml")),
                    search_path_from_args: vec![root.path().to_path_buf()],
                    disable_search_path_heuristics: true,
                    disable_project_excludes_heuristics: true,
                    enable_fallback_search_path: false,
                    ..ConfigFile::default()
                };
                config.python_environment.python_version = Some(PythonVersion::new(3, 14, 7));
                config.python_environment.python_platform = Some(PythonPlatform::new("linux"));
                config.python_environment.site_package_path = Some(vec![]);
                config.interpreters.skip_interpreter_query = true;
                assert!(config.configure().is_empty());
                let main = config
                    .handle_from_module_path(ModulePath::filesystem(root.path().join("main.py")));
                let dependency = config.handle_from_module_path(ModulePath::filesystem(
                    root.path().join("dependency.py"),
                ));
                let state = State::new(
                    ConfigFinder::new_constant(ArcId::new(config)),
                    ThreadCount::Inline,
                );
                let mut transaction = state.cancellable_transaction();
                let handle = transaction.get_cancellation_handle();
                let owner = Arc::new(ProviderCancellation::default());
                let registration = owner.register(move || handle.cancel());
                assert!(
                    transaction
                        .run(std::slice::from_ref(&main), Require::Everything, None)
                        .is_ok()
                );
                assert!(transaction.as_ref().get_answers(&main).is_some());
                owner.cancel();
                // A dependency promoted to Everything in the next wave must report native cancellation.
                assert!(
                    transaction
                        .run(&[dependency], Require::Everything, None)
                        .is_err()
                );
                assert!(registration.check().is_err());
                assert!(transaction.get_cancellation_handle().is_cancelled());
            })
            .unwrap()
            .join()
            .unwrap();
    }
}
