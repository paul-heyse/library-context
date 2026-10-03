//! The provider framework (cutover plan A0). A provider runs as one scheduled stage on its own
//! thread, with a declared stack. It hands typed batches, each carrying its reservation, across a
//! bounded channel to an asynchronous pump that holds the stage's access and writes the sink. The
//! channel window bounds what a fast provider holds ahead of a slow sink. A provider panic, or any
//! refused emission, fails the attempt.
use crate::{acquisition::AcquiredInput, assembly::Attacher};
use lctx_model::domain::{
    Batch, ContentHash, Infrastructure, KeySink, ModelError, Record, ValidatedModel,
    batching::{BatchWriter, TransferLimits},
    resources::ResourceBudget,
    source::Occurrence,
    stages::{Handoffs, Profile, ProviderOutcome, Stage, StageAccess, StageOutput, StageSink},
};
use std::{
    any::TypeId, ffi::OsString, future::Future, marker::PhantomData, panic::AssertUnwindSafe,
    pin::Pin, sync::Arc,
};
use tokio::sync::{mpsc, oneshot};
/// Native environment switches refused by the single facts driver.
pub const REFUSED_ENV: &[&str] = &["PYREFLY_STACK_SIZE", "PYREFLY_FIXPOINT_DETAILS"];
pub const REFUSED_ENV_PREFIX: &str = "PYSA_DUMP";

/// Batches a provider may hold in the channel beyond the one its pump is writing.
pub const WINDOW: usize = 2;
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
    digest.part(b"ruff-patch", include_str!("../../../third_party/ruff-0.16.10.patch").as_bytes());
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

/// A provider's scheduled stage, whichever sink it writes. The schedule is built from it.
pub trait Declared {
    fn declaration(&self, profile: Profile) -> Stage;
}
/// One provider as one scheduled stage: `run` executes on the provider thread and reports the
/// provider's outcome.
pub trait ProviderStage<S: StageSink + 'static>: Declared + Send {
    fn run(&mut self, context: &mut StageContext<S>) -> Result<ProviderOutcome, ModelError>;
}

/// What a running provider may do: emit its declared outputs, contribute declared vocabulary, read
/// its declared inputs' handoffs, attach to handed-off occurrences, and read the captured inputs.
/// The first refusal poisons the context, so the attempt fails even if the provider ignores it.
pub struct StageContext<S: StageSink + 'static> {
    stage: Stage,
    profile: Profile,
    model: Arc<ValidatedModel>,
    budget: ResourceBudget,
    limits: TransferLimits,
    captured: Arc<CapturedInputs>,
    handoffs: Handoffs,
    attacher: Option<Attacher>,
    sender: mpsc::Sender<Box<dyn Delivery<S>>>,
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
impl<S: StageSink + 'static> StageContext<S> {
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
            context.send(Box::new(Declare::<R>(PhantomData)))
        })
    }
    /// Emit one row of a declared output through the provider's transfer-bounded writer.
    pub fn emit<R: Record>(&mut self, row: R) -> Result<(), ModelError> {
        self.guard(|context| {
            let model = context.model.clone();
            let full = context.output::<R>(Mode::Rows)?.writer.push(&model, row)?;
            match full {
                Some(batch) => context.send(Box::new(Emit {
                    batch,
                    contribution: false,
                })),
                None => Ok(()),
            }
        })
    }
    /// Emit a batch the provider built with its own transfer-bounded writer. An output is emitted
    /// by rows or by batches, never both.
    pub fn emit_batch<R: Record>(&mut self, batch: Batch<R>) -> Result<(), ModelError> {
        self.guard(|context| {
            context.output::<R>(Mode::Batches)?;
            context.send(Box::new(Emit {
                batch,
                contribution: false,
            }))
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
                Some(batch) => context.send(Box::new(Emit {
                    batch,
                    contribution: true,
                })),
                None => Ok(()),
            }
        })
    }
    /// The batches an earlier stage handed off for one of this stage's declared inputs.
    pub fn handoff<R: Record>(&mut self) -> Result<Vec<Arc<Batch<R>>>, ModelError> {
        self.guard(|context| context.handoffs.get::<R>())
    }
    /// The attachment index over this stage's handed-off occurrences, built on first use.
    pub fn attacher(&mut self) -> Result<&Attacher, ModelError> {
        if self.attacher.is_none() {
            let occurrences = self.handoff::<Occurrence>()?;
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
        if self.failed {
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
    fn send(&self, delivery: Box<dyn Delivery<S>>) -> Result<(), ModelError> {
        self.sender.blocking_send(delivery).map_err(|_| {
            ModelError::Invalid(format!("the {} stage writer stopped", self.stage.name))
        })
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
            if let Some(delivery) = writer.finish(&self.model)? {
                self.send(delivery)?;
            }
        }
        Ok(outcome)
    }
}

/// Run one scheduled stage through its provider. The provider thread owns the provider, its context
/// and every batch it has not yet handed over; the pump writes delivered batches in order and
/// finishes the stage with the provider's outcome.
pub async fn run_stage<S: StageSink + 'static>(
    mut provider: Box<dyn ProviderStage<S>>,
    access: StageAccess<'_, '_>,
    sink: &S,
    model: &Arc<ValidatedModel>,
    captured: &Arc<CapturedInputs>,
    budget: &ResourceBudget,
    limits: TransferLimits,
) -> Result<ProviderOutcome, ModelError> {
    let profile = access.profile();
    captured.config().check_profile(profile)?;
    captured.config().check_budget(budget)?;
    let stage = access.stage().clone();
    let name = stage.name;
    let handoffs = access.handoffs()?;
    let mut output = StageOutput::new(access, sink, model, budget.clone(), limits)?;
    let (sender, mut receiver) = mpsc::channel::<Box<dyn Delivery<S>>>(WINDOW);
    let (done, finished) = oneshot::channel();
    let mut context = StageContext {
        stage,
        profile,
        model: model.clone(),
        budget: budget.clone(),
        limits,
        captured: captured.clone(),
        handoffs,
        attacher: None,
        sender,
        outputs: Vec::new(),
        contributions: Vec::new(),
        failed: false,
    };
    std::thread::Builder::new()
        .name(format!("lctx-{name}"))
        .stack_size(PROVIDER_STACK_BYTES)
        .spawn(move || {
            let result = std::panic::catch_unwind(AssertUnwindSafe(|| provider.run(&mut context)))
                .unwrap_or_else(|_| {
                    Err(ModelError::Invalid(format!("the {name} provider panicked")))
                });
            let result = context.close(result);
            // The provider's own state is released before the pump learns the outcome.
            drop(provider);
            let _ = done.send(result);
        })
        .map_err(|error| ModelError::infrastructure(Infrastructure::Io, error))?;
    let mut failure = None;
    while let Some(delivery) = receiver.recv().await {
        if let Err(error) = delivery.deliver(&mut output).await {
            failure = Some(error);
            break;
        }
    }
    // Stop the provider at its next emission and release what remains in the window.
    drop(receiver);
    let result = finished.await.unwrap_or_else(|_| {
        Err(ModelError::Invalid(format!(
            "the {name} provider ended without a result"
        )))
    });
    if let Some(error) = failure {
        return Err(error);
    }
    let outcome = result?;
    output.finish(outcome).await?;
    Ok(outcome)
}

/// One typed step for the pump, created where the relation type is known.
trait Delivery<S: StageSink>: Send {
    fn deliver<'x>(
        self: Box<Self>,
        output: &'x mut StageOutput<'_, '_, '_, S>,
    ) -> Pin<Box<dyn Future<Output = Result<(), ModelError>> + Send + 'x>>;
}
struct Declare<R>(PhantomData<fn() -> R>);
impl<R: Record, S: StageSink> Delivery<S> for Declare<R> {
    fn deliver<'x>(
        self: Box<Self>,
        output: &'x mut StageOutput<'_, '_, '_, S>,
    ) -> Pin<Box<dyn Future<Output = Result<(), ModelError>> + Send + 'x>> {
        Box::pin(std::future::ready(output.declare::<R>()))
    }
}
struct Emit<R: Record> {
    batch: Batch<R>,
    contribution: bool,
}
impl<R: Record, S: StageSink> Delivery<S> for Emit<R> {
    fn deliver<'x>(
        self: Box<Self>,
        output: &'x mut StageOutput<'_, '_, '_, S>,
    ) -> Pin<Box<dyn Future<Output = Result<(), ModelError>> + Send + 'x>> {
        let Emit {
            batch,
            contribution,
        } = *self;
        if contribution {
            Box::pin(std::future::ready(output.contribute_batch(batch)))
        } else {
            Box::pin(output.push_batch(batch))
        }
    }
}
struct Writer<R: Record> {
    writer: BatchWriter<R>,
    contribution: bool,
}
trait Flush<S: StageSink>: Send {
    fn as_any(&mut self) -> &mut dyn std::any::Any;
    fn finish(
        self: Box<Self>,
        model: &ValidatedModel,
    ) -> Result<Option<Box<dyn Delivery<S>>>, ModelError>;
}
impl<R: Record, S: StageSink + 'static> Flush<S> for Writer<R> {
    fn as_any(&mut self) -> &mut dyn std::any::Any {
        self
    }
    fn finish(
        self: Box<Self>,
        model: &ValidatedModel,
    ) -> Result<Option<Box<dyn Delivery<S>>>, ModelError> {
        let Writer {
            writer,
            contribution,
        } = *self;
        Ok(writer.finish(model)?.map(|batch| {
            Box::new(Emit {
                batch,
                contribution,
            }) as Box<dyn Delivery<S>>
        }))
    }
}
