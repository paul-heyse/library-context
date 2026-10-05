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
    any::TypeId, ffi::OsString, panic::AssertUnwindSafe,
    sync::{Arc, atomic::{AtomicBool, Ordering}},
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

/// Ordinary completed-input and batch-output boundary. Implementations own temporary storage;
/// these methods do not grant database reads or certify publication.
pub trait ProviderSink: Send + Sync + 'static {
    fn read<R: Record>(&self) -> Result<Box<dyn Iterator<Item = Result<Batch<R>, ModelError>> + Send>, ModelError>;
    fn declare<R: Record>(&self) -> Result<(), ModelError>;
    fn write<R: Record>(&self, batch: Batch<R>) -> Result<(), ModelError>;
    fn contribute<R: Record>(&self, batch: Batch<R>) -> Result<(), ModelError>;
    /// Record the outcome after all writes drain. Workspace completion/admission belongs to the compiler.
    fn finish(&self, outcome: ProviderOutcome) -> Result<(), ModelError>;
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
    sink: Arc<S>,
    cancelled: Arc<AtomicBool>,
    attacher: Option<Attacher>,
    outputs: Vec<(TypeId, Mode, Box<dyn Flush<S>>)>,
    contributions: Vec<(TypeId, Box<dyn Flush<S>>)>,
    failed: bool,
}
#[derive(Clone, Copy, PartialEq, Eq)]
enum Mode {
    Declared,
    Rows,
    Batches,
}
impl<S: ProviderSink + 'static> StageContext<S> {
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
    pub fn input<R: Record>(&mut self) -> Result<Box<dyn Iterator<Item = Result<Batch<R>, ModelError>> + Send>, ModelError> {
        self.guard(|context| {
            if !context.stage.reads::<R>() {
                return Err(ModelError::Invalid(format!("{} does not consume {}", context.stage.name, R::NAME)));
            }
            context.sink.read::<R>()
        })
    }
    /// The attachment index over this stage's handed-off occurrences, built on first use.
    pub fn attacher(&mut self) -> Result<&Attacher, ModelError> {
        if self.attacher.is_none() {
            // Attachment needs a compact global occurrence index; retain only this named input
            // while constructing it, then release its batches before analysis begins.
            let occurrences = self.input::<Occurrence>()?.map(|batch| batch.map(Arc::new)).collect::<Result<Vec<_>, _>>()?;
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
        if self.failed || self.cancelled.load(Ordering::Acquire) {
            return Err(ModelError::Invalid(format!(
                "{} refused an earlier operation; the attempt fails",
                self.stage.name
            )));
        }
        let result = operation(self);
        self.failed |= result.is_err();
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
        if self.failed {
            return Err(ModelError::Invalid(format!(
                "{} ignored a refused operation",
                self.stage.name
            )));
        }
        if self.stage.outputs.iter().any(|declared| {
            !self.outputs.iter().any(|(_, _, writer)| writer.name() == declared.name())
        }) {
            return Err(ModelError::Invalid(format!(
                "{} omitted a declared output, including its empty case", self.stage.name
            )));
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
            writer.finish(&self.model, self.sink.as_ref())?;
        }
        self.sink.finish(outcome)?;
        Ok(outcome)
    }
}

/// Run an actual native provider with explicit completed inputs and attempt-local output.
/// Dropping the future signals cancellation and joins the provider thread; no native work can
/// outlive the workspace or expose a completed artifact after cancellation.
#[allow(clippy::too_many_arguments, reason = "Explicit provider effects and captured inputs")]
pub async fn run_provider<S: ProviderSink>(
    mut provider: Box<dyn ProviderStage<S>>,
    profile: Profile,
    sink: Arc<S>,
    model: Arc<ValidatedModel>,
    captured: Arc<CapturedInputs>,
    budget: ResourceBudget,
    limits: TransferLimits,
) -> Result<ProviderOutcome, ModelError> {
    captured.config().check_profile(profile)?;
    captured.config().check_budget(&budget)?;
    let stage = provider.declaration(profile);
    if !stage.profiles.contains(&profile) {
        return Err(ModelError::Invalid("provider is not requested by profile".into()));
    }
    let name = stage.name;
    let cancelled = Arc::new(AtomicBool::new(false));
    let (done, finished) = oneshot::channel();
    let mut context = StageContext {
        stage, profile, model, budget, limits, captured, sink,
        cancelled: cancelled.clone(), attacher: None, outputs: Vec::new(),
        contributions: Vec::new(), failed: false,
    };
    let thread = std::thread::Builder::new()
        .name(format!("lctx-{name}"))
        .stack_size(PROVIDER_STACK_BYTES)
        .spawn(move || {
            let result = std::panic::catch_unwind(AssertUnwindSafe(|| provider.run(&mut context)))
                .unwrap_or_else(|_| Err(ModelError::Invalid(format!("the {name} provider panicked"))));
            let result = context.close(result);
            drop(provider);
            let _ = done.send(result);
        })
        .map_err(|error| ModelError::infrastructure(Infrastructure::Io, error))?;
    let drain = ProviderDrain { cancelled, thread: Some(thread) };
    let result = finished.await.unwrap_or_else(|_| Err(ModelError::Invalid(format!("the {name} provider ended without a result"))));
    drop(drain);
    result
}
struct ProviderDrain {
    cancelled: Arc<AtomicBool>,
    thread: Option<std::thread::JoinHandle<()>>,
}
impl Drop for ProviderDrain {
    fn drop(&mut self) {
        self.cancelled.store(true, Ordering::Release);
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
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
    fn name(&self) -> &'static str { R::NAME }
    fn as_any(&mut self) -> &mut dyn std::any::Any { self }
    fn finish(self: Box<Self>, model: &ValidatedModel, sink: &S) -> Result<(), ModelError> {
        let Writer { writer, contribution } = *self;
        if let Some(batch) = writer.finish(model)? {
            if contribution { sink.contribute(batch)?; } else { sink.write(batch)?; }
        }
        Ok(())
    }
}
