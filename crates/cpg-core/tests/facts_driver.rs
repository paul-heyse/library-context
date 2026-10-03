//! The facts driver (cutover plan A0) and the fixture-corpus runner skeleton: fixtures are
//! captured, driven through the production providers by `compile_facts` into a stage-bound memory
//! generation and validated. Plan B3 grows the registered cases and the providers.
use cpg_core::facts::compile_facts;
use cpg_extract::{
    acquisition::{Acquire, AcquiredInput},
    bundle::{CapturedInputs, Declared, ProviderStage, StageContext},
    capture::CapturedInput,
};
use lctx_model::domain::{memory::MemoryGeneration, resources::ResourceBudget, stages::*, *};
use std::{
    collections::BTreeMap,
    path::Path,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
};

/// The registered fixture cases and the files each one captures.
const CASES: &[(&str, &[&str])] = &[("typed_semantics", &["sample.py", "_invalid/broken.py"])];

fn budget() -> ResourceBudget {
    ResourceBudget::fixed(1 << 30).unwrap()
}
fn configuration() -> ContentHash {
    ContentHash::of(b"fixture-corpus")
}
/// The production `acquire` stage, counting its runs.
struct Counted(Acquire, Arc<AtomicUsize>);
impl Declared for Counted {
    fn declaration(&self, profile: Profile) -> Stage {
        self.0.declaration(profile)
    }
}
impl ProviderStage<MemoryGeneration> for Counted {
    fn run(
        &mut self,
        context: &mut StageContext<MemoryGeneration>,
    ) -> Result<ProviderOutcome, ModelError> {
        self.1.fetch_add(1, Ordering::Relaxed);
        self.0.run(context)
    }
}
fn acquire(
    ran: &Arc<AtomicUsize>,
    configuration: ContentHash,
) -> Box<dyn ProviderStage<MemoryGeneration>> {
    Box::new(Counted(Acquire::new(configuration), ran.clone()))
}
fn declared(configuration: ContentHash) -> Stage {
    Acquire::new(configuration).declaration(Profile::Catalog)
}
/// A provider that must never run.
struct Unused(Stage);
impl Declared for Unused {
    fn declaration(&self, _: Profile) -> Stage {
        self.0.clone()
    }
}
impl ProviderStage<MemoryGeneration> for Unused {
    fn run(
        &mut self,
        _: &mut StageContext<MemoryGeneration>,
    ) -> Result<ProviderOutcome, ModelError> {
        panic!("{} must not run", self.0.name)
    }
}
fn stage(name: &'static str, profiles: Vec<Profile>) -> Stage {
    Stage {
        name,
        inputs: vec![],
        outputs: vec![RelationUse::of::<source::Module>()],
        contributes: vec![],
        coverage: vec![],
        profiles,
        effect: Effect::Pure,
        code: ContentHash::of(name.as_bytes()),
        configuration: ContentHash::of(name.as_bytes()),
    }
}

/// Capture a fresh copy of one registered fixture case in its own temporary tree.
fn capture(case: &str, files: &[&str]) -> CapturedInput {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/python")
        .join(case);
    let copy = tempfile::tempdir().unwrap();
    for file in files {
        let target = copy.path().join(file);
        std::fs::create_dir_all(target.parent().unwrap()).unwrap();
        std::fs::copy(root.join(file), target).unwrap();
    }
    CapturedInput::capture(
        copy.path(),
        &files.iter().map(|f| (*f).to_owned()).collect::<Vec<_>>(),
        &budget(),
    )
    .unwrap()
}
/// Drive one case through the providers into a memory generation; return its content digest and receipt.
async fn run(
    model: &Arc<ValidatedModel>,
    profile: Profile,
    providers: Vec<Box<dyn ProviderStage<MemoryGeneration>>>,
    case: &str,
    files: &[&str],
) -> Result<(ContentHash, BTreeMap<&'static str, ProviderOutcome>), ModelError> {
    let schedule = Schedule::build(model, vec![declared(configuration())], &[], profile)?;
    let mut execution = schedule.execute();
    let budget = budget();
    let generation = MemoryGeneration::bind(model, &budget, &mut execution)?;
    let captured = Arc::new(CapturedInputs::new(
        vec![AcquiredInput::tree(capture(case, files), "fixture-corpus")],
        cpg_extract::native_context::NativeContextConfig::committed(profile, &budget).unwrap(),
    ));
    let receipt =
        compile_facts(execution, providers, &generation, model, &captured, &budget).await?;
    Ok((
        generation.validate(model, &budget)?,
        receipt.outcomes().clone(),
    ))
}

#[tokio::test]
async fn fixture_corpus_skeleton_runs_every_registered_case_through_memory() {
    let model = Arc::new(ValidatedModel::validate(facts_relations()).unwrap());
    let ran = Arc::new(AtomicUsize::new(0));
    for (case, files) in CASES {
        for profile in Profile::ALL {
            let first = run(
                &model,
                profile,
                vec![acquire(&ran, configuration())],
                case,
                files,
            )
            .await
            .unwrap();
            let second = run(
                &model,
                profile,
                vec![acquire(&ran, configuration())],
                case,
                files,
            )
            .await
            .unwrap();
            assert_eq!(
                first, second,
                "{case}: a relocated capture gives the same validated content"
            );
            assert_eq!(first.1.get("acquire"), Some(&ProviderOutcome::Complete));
        }
    }
    assert_eq!(ran.load(Ordering::Relaxed), 4 * CASES.len());
}

#[tokio::test]
async fn the_providers_must_be_exactly_the_scheduled_stages() {
    let model = Arc::new(ValidatedModel::validate(facts_relations()).unwrap());
    let (case, files) = CASES[0];
    let ran = Arc::new(AtomicUsize::new(0));
    // A provider outside the schedule's profile is not offered; the catalog schedule runs without it.
    let flow = stage("flow", vec![Profile::Behavioral]);
    let catalog = run(
        &model,
        Profile::Catalog,
        vec![acquire(&ran, configuration()), Box::new(Unused(flow))],
        case,
        files,
    )
    .await;
    assert!(catalog.is_ok(), "{catalog:?}");
    assert_eq!(ran.swap(0, Ordering::Relaxed), 1);
    let refusals: Vec<ProviderRefusal> = vec![
        ("a scheduled stage without a provider", vec![]),
        (
            "a provider the schedule does not name",
            vec![
                acquire(&ran, configuration()),
                Box::new(Unused(stage("stray", vec![Profile::Catalog]))),
            ],
        ),
        (
            "a declaration that differs from its scheduled stage",
            vec![acquire(&ran, ContentHash::of(b"other configuration"))],
        ),
        (
            "two providers for one stage",
            vec![
                acquire(&ran, configuration()),
                Box::new(Unused(declared(configuration()))),
            ],
        ),
    ];
    for (name, providers) in refusals {
        let refused = run(&model, Profile::Catalog, providers, case, files).await;
        assert!(
            matches!(&refused, Err(ModelError::Invalid(message)) if !message.contains("panicked")),
            "{name}: {refused:?}"
        );
    }
    assert_eq!(
        ran.load(Ordering::Relaxed),
        0,
        "a refused provider set runs no stage"
    );
}

type ProviderRefusal = (&'static str, Vec<Box<dyn ProviderStage<MemoryGeneration>>>);
