//! Native providers consume completed inputs and flush bounded output before completion.
use cpg_extract::bundle::{
    CapturedInputs, Declared, ProviderSink, ProviderStage, StageContext, run_provider,
};
use lctx_model::domain::{
    batching::TransferLimits, input::*, resources::ResourceBudget, stages::*, *,
};
use std::sync::{Arc, Mutex};

#[derive(Default)]
struct Recorder {
    batches: Mutex<Vec<RecordedBatch>>,
    finished: Mutex<Vec<ProviderOutcome>>,
    streamed_error: bool,
}
impl ProviderSink for Recorder {
    fn read<R: Record>(
        &self,
    ) -> Result<Box<dyn Iterator<Item = Result<Batch<R>, ModelError>> + Send>, ModelError> {
        if self.streamed_error {
            Ok(Box::new(std::iter::once(Err(ModelError::Invalid(
                "injected input stream error".into(),
            )))))
        } else {
            Err(ModelError::Invalid(format!(
                "missing completed input {}",
                R::NAME
            )))
        }
    }
    fn declare<R: Record>(&self) -> Result<(), ModelError> {
        Ok(())
    }
    fn write<R: Record>(&self, batch: Batch<R>) -> Result<(), ModelError> {
        self.batches.lock().unwrap().push((
            R::NAME,
            batch
                .rows()
                .iter()
                .map(|row| row.id().bytes().to_vec())
                .collect(),
        ));
        Ok(())
    }
    fn contribute<R: Record>(&self, batch: Batch<R>) -> Result<(), ModelError> {
        self.write(batch)
    }
    fn finish(&self, outcome: ProviderOutcome) -> Result<(), ModelError> {
        self.finished.lock().unwrap().push(outcome);
        Ok(())
    }
}
type Run =
    Box<dyn FnMut(&mut StageContext<Recorder>) -> Result<ProviderOutcome, ModelError> + Send>;
struct Provider {
    stage: Stage,
    run: Run,
}
impl Declared for Provider {
    fn declaration(&self, _: Profile) -> Stage {
        self.stage.clone()
    }
}
impl ProviderStage<Recorder> for Provider {
    fn run(&mut self, context: &mut StageContext<Recorder>) -> Result<ProviderOutcome, ModelError> {
        (self.run)(context)
    }
}
fn packages() -> Stage {
    Stage {
        name: "packages",
        inputs: vec![],
        outputs: vec![RelationUse::of::<Package>()],
        contributes: vec![],
        coverage: vec![],
        profiles: vec![Profile::Catalog],
        effect: Effect::Extraction,
        code: ContentHash::of(b"packages"),
        configuration: ContentHash::of(b"bundle"),
    }
}
fn package(n: usize, width: usize) -> Package {
    Package {
        name: format!("p{n:06}-{}", "a".repeat(width)),
    }
}
async fn run(
    stage: Stage,
    sink: Arc<Recorder>,
    limits: TransferLimits,
    action: impl FnMut(&mut StageContext<Recorder>) -> Result<ProviderOutcome, ModelError>
    + Send
    + 'static,
) -> (Result<ProviderOutcome, ModelError>, ResourceBudget) {
    let budget = ResourceBudget::fixed(1 << 30).unwrap();
    let captured = Arc::new(CapturedInputs::new(
        vec![],
        cpg_extract::native_context::NativeContextConfig::committed(Profile::Catalog, &budget)
            .unwrap(),
    ));
    let result = run_provider(
        Box::new(Provider {
            stage,
            run: Box::new(action),
        }),
        Profile::Catalog,
        sink,
        Arc::new(model().unwrap()),
        captured,
        budget.clone(),
        limits,
    )
    .await;
    (result, budget)
}
#[tokio::test]
async fn batches_preserve_emission_groups_and_release_their_reservations() {
    let sink = Arc::new(Recorder::default());
    let limits = TransferLimits {
        rows: 64,
        bytes: 256 << 10,
        max_row: 64 << 20,
    };
    let (result, budget) = run(packages(), sink.clone(), limits, |context| {
        context.declare::<Package>()?;
        for n in 0..1000 {
            context.emit(package(n, 2048))?;
        }
        Ok(ProviderOutcome::Complete)
    })
    .await;
    assert_eq!(result.unwrap(), ProviderOutcome::Complete);
    let recorded = sink.batches.lock().unwrap();
    let emitted: Vec<_> = (0..1000)
        .map(|n| package(n, 2048).id().bytes().to_vec())
        .collect();
    let mut next = 0;
    for (_, batch) in recorded.iter() {
        assert!(batch.len() <= limits.rows);
        let mut want = emitted[next..next + batch.len()].to_vec();
        let mut got = batch.clone();
        want.sort();
        got.sort();
        assert_eq!(got, want);
        next += batch.len();
    }
    assert_eq!(next, 1000);
    assert_eq!(*sink.finished.lock().unwrap(), [ProviderOutcome::Complete]);
    assert_eq!(budget.reserved(), 0);
}
#[tokio::test]
async fn ignored_refusals_do_not_complete_output() {
    type Case = fn(&mut StageContext<Recorder>) -> Result<(), ModelError>;
    let cases: [Case; 5] = [
        |c| {
            c.emit(Release {
                package: package(0, 1).id(),
                version: "1".into(),
            })
        },
        |c| c.emit(package(0, 1)),
        |c| c.input::<Release>().map(drop),
        |c| c.attacher().map(drop),
        |c| {
            c.declare::<Package>()?;
            c.emit(package(0, 1))?;
            c.emit_batch(Batch::new(c.model(), vec![package(1, 1)], c.budget())?)
        },
    ];
    for case in cases {
        let sink = Arc::new(Recorder::default());
        let (result, budget) = run(
            packages(),
            sink.clone(),
            TransferLimits::default(),
            move |c| {
                assert!(case(c).is_err());
                let _ = c.declare::<Package>();
                Ok(ProviderOutcome::Complete)
            },
        )
        .await;
        assert!(result.is_err());
        assert!(sink.finished.lock().unwrap().is_empty());
        assert_eq!(budget.reserved(), 0);
    }
}
#[tokio::test]
async fn ignored_stream_errors_poison_direct_reads_and_attachment() {
    for attach in [false, true] {
        let sink = Arc::new(Recorder {
            streamed_error: true,
            ..Recorder::default()
        });
        let mut stage = packages();
        stage.inputs = if attach {
            vec![RelationUse::of::<lctx_model::domain::source::Occurrence>()]
        } else {
            vec![RelationUse::of::<Release>()]
        };
        let (result, budget) = run(
            stage,
            sink.clone(),
            TransferLimits::default(),
            move |context| {
                context.declare::<Package>()?;
                if attach {
                    assert!(context.attacher().is_err());
                } else {
                    let mut input = context.input::<Release>()?;
                    assert!(input.next().unwrap().is_err());
                }
                Ok(ProviderOutcome::Complete)
            },
        )
        .await;
        assert!(result.is_err());
        assert!(sink.finished.lock().unwrap().is_empty());
        assert_eq!(budget.reserved(), 0);
    }
}

#[tokio::test]
async fn declared_empty_is_complete_but_omitted_output_is_refused() {
    for declare in [false, true] {
        let sink = Arc::new(Recorder::default());
        let (result, budget) = run(
            packages(),
            sink.clone(),
            TransferLimits::default(),
            move |c| {
                if declare {
                    c.declare::<Package>()?;
                }
                Ok(ProviderOutcome::Complete)
            },
        )
        .await;
        assert_eq!(result.is_ok(), declare);
        assert_eq!(sink.finished.lock().unwrap().len(), usize::from(declare));
        assert_eq!(budget.reserved(), 0);
    }
}
#[tokio::test]
async fn panic_releases_batches_without_completing_output() {
    let sink = Arc::new(Recorder::default());
    let (result, budget) = run(
        packages(),
        sink.clone(),
        TransferLimits {
            rows: 16,
            ..TransferLimits::default()
        },
        |c| {
            c.declare::<Package>()?;
            for n in 0..100 {
                c.emit(package(n, 64))?;
            }
            panic!("injected provider fault")
        },
    )
    .await;
    assert!(matches!(result, Err(ModelError::Invalid(message)) if message.contains("panicked")));
    assert!(sink.finished.lock().unwrap().is_empty());
    assert_eq!(budget.reserved(), 0);
}
#[tokio::test]
async fn dropping_the_future_drains_native_work_and_refuses_completion() {
    use std::sync::atomic::{AtomicBool, Ordering};
    let sink = Arc::new(Recorder::default());
    let stopped = Arc::new(AtomicBool::new(false));
    let native_stopped = stopped.clone();
    let (started, begun) = tokio::sync::oneshot::channel();
    let (release, wait_release) = std::sync::mpsc::channel();
    let mut started = Some(started);
    let budget = ResourceBudget::fixed(1 << 30).unwrap();
    let captured = Arc::new(CapturedInputs::new(
        vec![],
        cpg_extract::native_context::NativeContextConfig::committed(Profile::Catalog, &budget)
            .unwrap(),
    ));
    let provider = Provider {
        stage: packages(),
        run: Box::new(move |c| {
            c.declare::<Package>()?;
            started.take().unwrap().send(()).unwrap();
            wait_release.recv().unwrap();
            native_stopped.store(true, Ordering::Release);
            Ok(ProviderOutcome::Complete)
        }),
    };
    let mut future = Box::pin(run_provider(
        Box::new(provider),
        Profile::Catalog,
        sink.clone(),
        Arc::new(model().unwrap()),
        captured,
        budget.clone(),
        TransferLimits::default(),
    ));
    tokio::select! { result = &mut future => panic!("unexpected completion {result:?}"), result = begun => result.unwrap() }
    let unblock = std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_millis(20));
        release.send(()).unwrap();
    });
    drop(future);
    unblock.join().unwrap();
    assert!(stopped.load(Ordering::Acquire));
    assert!(sink.finished.lock().unwrap().is_empty());
    assert_eq!(budget.reserved(), 0);
}

#[test]
fn ambient_analyzer_configuration_is_refused() {
    use cpg_extract::bundle::refuse_ambient;
    let vars = |names: &[&str]| {
        names
            .iter()
            .map(|n| ((*n).into(), "1".into()))
            .collect::<Vec<_>>()
    };
    assert!(refuse_ambient(vars(&["HOME", "PATH", "PYTHONHASHSEED"])).is_ok());
    assert!(refuse_ambient(vars(&["HOME", "PYREFLY_STACK_SIZE"])).is_err());
    assert!(refuse_ambient(vars(&["PYSA_DUMP_CALL_GRAPH"])).is_err());
}

#[test]
fn provider_identity_covers_the_lockfile_the_pyrefly_patch_and_the_sources() {
    use cpg_extract::bundle::build_digest;
    assert_eq!(build_digest(&["a"]), build_digest(&["a"]));
    assert_ne!(
        build_digest(&["a"]),
        build_digest(&["b"]),
        "a provider source change is a new provider"
    );
    assert_ne!(
        build_digest(&["ab"]),
        build_digest(&["a", "b"]),
        "sources are framed, not concatenated"
    );
    assert_eq!(
        cpg_extract::pyrefly_stage::pyrefly_provider().build_digest,
        build_digest(&[
            include_str!("../src/pyrefly_stage.rs"),
            include_str!("../src/native_context.rs"),
            include_str!("../src/typed_syntax.rs"),
            include_str!("../src/syntax_records.rs"),
            include_str!("../src/lexical.rs"),
            include_str!("../src/lexical_records.rs"),
            include_str!("../src/natives.rs"),
            include_str!("../src/symbol_records.rs"),
            include_str!("../src/public_records.rs"),
            include_str!("../src/docstrings.rs"),
            include_str!("../src/type_records.rs"),
            include_str!("../src/call_records.rs"),
            include_str!("../src/protocol_records.rs"),
            include_str!("../src/capture_records.rs"),
            include_str!("../src/diagnostic_records.rs"),
            include_str!("../src/parameter_definition_records.rs"),
        ])
    );
}

type RecordedBatch = (&'static str, Vec<Vec<u8>>);
