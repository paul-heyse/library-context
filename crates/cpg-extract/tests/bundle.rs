//! The provider framework (cutover plan A0): providers run as scheduled stages on their own
//! threads and hand reserved batches across a bounded window to the stage's pump. Every control
//! states its answer first.
use std::{future::Future, sync::{Arc, Mutex}, time::Duration};
use cpg_extract::{assembly::Attached, bundle::{CapturedInputs, Declared, ProviderStage, StageContext, run_stage}};
use lctx_model::domain::{*, attachment::AttachmentQuery, batching::TransferLimits, input::*, resources::ResourceBudget, source::*, stages::*};

/// Records every batch it receives, in order, and the budget's reservation at each copy. A delay
/// makes it slower than any provider.
struct Recorder { batches: Mutex<Vec<(&'static str, Vec<Vec<u8>>)>>, budget: ResourceBudget, reserved: Mutex<Vec<usize>>, delay: Duration }
impl Recorder {
    fn new(budget: &ResourceBudget, delay: Duration) -> Self { Self { batches: Mutex::default(), budget: budget.clone(), reserved: Mutex::default(), delay } }
    fn ids(&self, relation: &str) -> Vec<Vec<u8>> {
        self.batches.lock().unwrap().iter().filter(|(name, _)| *name == relation).flat_map(|(_, ids)| ids.clone()).collect()
    }
}
impl StageSink for Recorder {
    fn copy<R: Record>(&self, _: WritePermit<'_, R>, batch: &Batch<R>) -> impl Future<Output = Result<(), ModelError>> + Send {
        std::thread::sleep(self.delay);
        self.reserved.lock().unwrap().push(self.budget.reserved());
        self.batches.lock().unwrap().push((R::NAME, batch.rows().iter().map(|row| row.id().bytes().to_vec()).collect()));
        std::future::ready(Ok(()))
    }
}

type Run<S> = Box<dyn FnMut(&mut StageContext<S>) -> Result<ProviderOutcome, ModelError> + Send>;
/// A test provider: a declared stage and what it does when it runs.
struct Provider<S: StageSink + 'static> { stage: Stage, run: Run<S> }
impl<S: StageSink + 'static> Declared for Provider<S> {
    fn declaration(&self, _: Profile) -> Stage { self.stage.clone() }
}
impl<S: StageSink + 'static> ProviderStage<S> for Provider<S> {
    fn run(&mut self, context: &mut StageContext<S>) -> Result<ProviderOutcome, ModelError> { (self.run)(context) }
}
fn provider<S: StageSink + 'static>(stage: &Stage, run: impl FnMut(&mut StageContext<S>) -> Result<ProviderOutcome, ModelError> + Send + 'static) -> Box<dyn ProviderStage<S>> {
    Box::new(Provider { stage: stage.clone(), run: Box::new(run) })
}
fn stage(name: &'static str, inputs: Vec<RelationUse>, outputs: Vec<RelationUse>, contributes: Vec<RelationUse>) -> Stage {
    Stage { name, inputs, outputs, contributes, coverage: vec![], provider: None, profiles: vec![Profile::Catalog], effect: Effect::Extraction,
        code: ContentHash::of(name.as_bytes()), configuration: ContentHash::of(b"bundle") }
}
fn package(n: usize, width: usize) -> Package { Package { name: format!("p{n:06}-{}", "a".repeat(width)) } }
fn nothing() -> Arc<CapturedInputs> { Arc::new(CapturedInputs::new(vec![])) }

#[tokio::test]
async fn batches_keep_their_order_and_the_window_bounds_what_is_reserved() {
    let model = Arc::new(model().unwrap());
    let budget = ResourceBudget::fixed(1 << 30).unwrap();
    let packages = stage("packages", vec![], vec![RelationUse::of::<Package>()], vec![]);
    let schedule = Schedule::build(&model, vec![packages.clone()], &[], Profile::Catalog).unwrap();
    let mut execution = schedule.execute();
    let sink = Recorder::new(&budget, Duration::from_millis(2));
    // 20,000 rows of about 2 KiB: about 40 MiB of output, in batches of at most 64 rows or 256 KiB.
    let (rows, width) = (20_000, 2048);
    let limits = TransferLimits { rows: 64, bytes: 256 << 10, max_row: 64 << 20 };
    let fast = provider(&packages, move |context| {
        context.declare::<Package>()?;
        for n in 0..rows { context.emit(package(n, width))?; }
        Ok(ProviderOutcome::Complete)
    });
    let outcome = run_stage(fast, execution.begin("packages").unwrap(), &sink, &model, &nothing(), &budget, limits).await.unwrap();
    assert_eq!(outcome, ProviderOutcome::Complete);
    execution.finish().unwrap();
    // A batch holds its rows in canonical order; batches arrive in emission order, each holding
    // exactly the next rows the provider emitted.
    let emitted: Vec<Vec<u8>> = (0..rows).map(|n| package(n, width).id().bytes().to_vec()).collect();
    let recorded: Vec<Vec<Vec<u8>>> = sink.batches.lock().unwrap().iter().map(|(_, ids)| ids.clone()).collect();
    assert!(recorded.len() >= rows / limits.rows, "{} batches", recorded.len());
    let mut next = 0;
    for (index, batch) in recorded.iter().enumerate() {
        let (mut got, mut want) = (batch.clone(), emitted[next..next + batch.len()].to_vec());
        got.sort(); want.sort();
        assert!(got == want, "batch {index} does not hold the next {} emitted rows", batch.len());
        next += batch.len();
    }
    assert_eq!(next, rows, "every row arrives once");
    let total = rows * width;
    let peak = budget.peak().unwrap();
    let most = sink.reserved.lock().unwrap().iter().copied().max().unwrap();
    // The provider's duplicate index grows with its output (about 200 bytes a row at most); the
    // batches in flight are bounded by the window: two queued, one written, one being filled.
    let index = rows * 200;
    eprintln!("window: peak {peak} B, largest at a copy {most} B, output {total} B, {} batches", recorded.len());
    assert!(peak <= index + 8 * limits.bytes, "peak {peak} exceeds the window bound (index {index}, output {total})");
    assert!(most < total / 4, "the reservation at any copy ({most}) is far below the total output ({total})");
    assert_eq!(budget.reserved(), 0, "every batch and the index are released");
}

#[tokio::test]
async fn undeclared_operations_refuse_and_fail_the_attempt_even_when_ignored() {
    let model = Arc::new(model().unwrap());
    let packages = stage("packages", vec![], vec![RelationUse::of::<Package>()], vec![]);
    let schedule = Schedule::build(&model, vec![packages.clone()], &[], Profile::Catalog).unwrap();
    type Case = fn(&mut StageContext<Recorder>) -> Result<(), ModelError>;
    let cases: [(&str, Case); 6] = [
        ("an undeclared output", |c| c.emit(Release { package: package(0, 1).id(), version: "1".into() })),
        ("a row before its declaration", |c| c.emit(package(0, 1))),
        ("an undeclared contribution", |c| { c.declare::<Package>()?; c.contribute(SourceArtifact::from_bytes(InputRevision::from_entries(vec![]).unwrap().id(), "a.py".into(), b"a").unwrap()) }),
        ("an undeclared read", |c| c.handoff::<Release>().map(drop)),
        ("an attacher without occurrences", |c| c.attacher().map(drop)),
        ("rows and batches mixed", |c| {
            c.declare::<Package>()?; c.emit(package(0, 1))?;
            c.emit_batch(Batch::new(c.model(), vec![package(1, 1)], c.budget())?)
        }),
    ];
    for (name, case) in cases {
        let budget = ResourceBudget::fixed(1 << 30).unwrap();
        let sink = Recorder::new(&budget, Duration::ZERO);
        let mut execution = schedule.execute();
        let ignoring = provider(&packages, move |context| {
            assert!(case(context).is_err(), "{name} is refused");
            // The provider carries on as if nothing had been refused.
            let _ = context.declare::<Package>();
            Ok(ProviderOutcome::Complete)
        });
        let result = run_stage(ignoring, execution.begin("packages").unwrap(), &sink, &model, &nothing(), &budget, TransferLimits::default()).await;
        assert!(result.is_err(), "{name}: an ignored refusal still fails the stage");
        assert!(execution.finish().is_err(), "{name}: and the attempt");
        assert_eq!(budget.reserved(), 0, "{name}");
    }
}

#[tokio::test]
async fn a_panicking_provider_fails_the_attempt_and_releases_its_batches() {
    let model = Arc::new(model().unwrap());
    let budget = ResourceBudget::fixed(1 << 30).unwrap();
    let packages = stage("packages", vec![], vec![RelationUse::of::<Package>()], vec![]);
    let schedule = Schedule::build(&model, vec![packages.clone()], &[], Profile::Catalog).unwrap();
    let mut execution = schedule.execute();
    let sink = Recorder::new(&budget, Duration::from_millis(1));
    let panicking = provider(&packages, |context| {
        context.declare::<Package>()?;
        for n in 0..1000 { context.emit(package(n, 64))?; }
        panic!("provider fault injected after emitting");
    });
    let limits = TransferLimits { rows: 16, ..TransferLimits::default() };
    let result = run_stage(panicking, execution.begin("packages").unwrap(), &sink, &model, &nothing(), &budget, limits).await;
    assert!(matches!(&result, Err(ModelError::Invalid(message)) if message.contains("panicked")), "{result:?}");
    assert!(execution.begin("packages").is_err() && execution.finish().is_err(), "the attempt is failed");
    assert_eq!(budget.reserved(), 0);
}

#[tokio::test]
async fn handoffs_contributions_and_attachment_follow_the_declarations() {
    let model = Arc::new(model().unwrap());
    let budget = ResourceBudget::fixed(1 << 30).unwrap();
    let input = InputRevision::from_entries(vec![]).unwrap().id();
    let artifact = move |path: &str| SourceArtifact::from_bytes(input, path.into(), path.as_bytes()).unwrap();
    let stages = [
        stage("contributor", vec![], vec![RelationUse::of::<Package>()], vec![RelationUse::of::<SourceArtifact>()]),
        stage("artifacts", vec![], vec![RelationUse::of::<SourceArtifact>()], vec![]),
        stage("occurrences", vec![], vec![RelationUse::of::<Occurrence>()], vec![]),
        stage("reader", vec![RelationUse::of::<SourceArtifact>(), RelationUse::of::<Package>(), RelationUse::of::<Occurrence>()],
            vec![RelationUse::of::<CorpusLibrary>()], vec![]),
    ];
    let schedule = Schedule::build(&model, stages.to_vec(), &[], Profile::Catalog).unwrap();
    let mut execution = schedule.execute();
    let sink = Recorder::new(&budget, Duration::ZERO);
    let read = Arc::new(Mutex::new(None));
    let seen = read.clone();
    let providers: Vec<Box<dyn ProviderStage<Recorder>>> = vec![
        provider(&stages[0], move |c| { c.declare::<Package>()?; c.emit(package(0, 1))?; c.contribute(artifact("a.py"))?; c.contribute(artifact("c.py"))?; Ok(ProviderOutcome::Complete) }),
        provider(&stages[1], move |c| { c.declare::<SourceArtifact>()?; c.emit(artifact("a.py"))?; c.emit(artifact("b.py"))?; Ok(ProviderOutcome::Complete) }),
        provider(&stages[2], |c| { c.declare::<Occurrence>()?; Ok(ProviderOutcome::Complete) }),
        provider(&stages[3], move |c| {
            let mut paths: Vec<String> = c.handoff::<SourceArtifact>()?.iter().flat_map(|b| b.rows().iter().map(|r| r.path.clone()).collect::<Vec<_>>()).collect();
            paths.sort();
            let packages = c.handoff::<Package>()?.iter().map(|b| b.rows().len()).sum::<usize>();
            let query = AttachmentQuery { source: artifact("a.py").id(), start: 0, end: 1, syntax_kind: SyntaxKind::ModModule, role: OccurrenceRole::Syntax, structural_path: None };
            let unattached = matches!(c.attacher()?.attach(&query)?, Attached::Unattached(_));
            *seen.lock().unwrap() = Some((paths, packages, unattached));
            c.declare::<CorpusLibrary>()?;
            Ok(ProviderOutcome::Partial)
        }),
    ];
    let mut providers = providers.into_iter();
    for stage in schedule.stages() {
        let offered = providers.next().unwrap();
        assert_eq!(offered.declaration(Profile::Catalog).name, stage.name, "schedule order is declaration order here");
        run_stage(offered, execution.begin(stage.name).unwrap(), &sink, &model, &nothing(), &budget, TransferLimits::default()).await.unwrap();
    }
    let receipt = execution.finish().unwrap();
    assert_eq!(receipt.outcomes()["reader"], ProviderOutcome::Partial, "the provider's outcome is the stage's");
    let (paths, packages, unattached) = read.lock().unwrap().take().unwrap();
    assert_eq!(paths, ["a.py", "b.py", "c.py"], "the contributed row equal to an emitted one is written once");
    assert_eq!((packages, unattached), (1, true), "handoffs arrive; nothing attaches to an empty occurrence input");
    assert_eq!(sink.ids(SourceArtifact::NAME).len(), 3);
    assert_eq!(sink.ids(CorpusLibrary::NAME).len(), 0, "a declared output without rows is written explicitly empty");
    assert_eq!(budget.reserved(), 0, "handoffs are released after their last reader");
}

#[test]
fn ambient_analyzer_configuration_is_refused() {
    use cpg_extract::bundle::refuse_ambient;
    let vars = |names: &[&str]| names.iter().map(|n| ((*n).into(), "1".into())).collect::<Vec<_>>();
    assert!(refuse_ambient(vars(&["HOME", "PATH", "PYTHONHASHSEED"])).is_ok());
    assert!(refuse_ambient(vars(&["HOME", "PYREFLY_STACK_SIZE"])).is_err());
    assert!(refuse_ambient(vars(&["PYSA_DUMP_CALL_GRAPH"])).is_err());
}

#[test]
fn provider_identity_covers_the_lockfile_the_pyrefly_patch_and_the_sources() {
    use cpg_extract::bundle::build_digest;
    assert_eq!(build_digest(&["a"]), build_digest(&["a"]));
    assert_ne!(build_digest(&["a"]), build_digest(&["b"]), "a provider source change is a new provider");
    assert_ne!(build_digest(&["ab"]), build_digest(&["a", "b"]), "sources are framed, not concatenated");
    assert_eq!(cpg_extract::pyrefly_stage::pyrefly_provider().build_digest,
        build_digest(&[include_str!("../src/pyrefly_stage.rs"), include_str!("../src/typed_syntax.rs"), include_str!("../src/syntax_records.rs"),
            include_str!("../src/lexical.rs"), include_str!("../src/lexical_records.rs")]));
}
