//! The facts driver (cutover plan A0) and the fixture-corpus runner skeleton: fixtures are
//! captured, driven through providers by `compile_facts` into a stage-bound memory generation and
//! validated. Plan B3 grows the registered cases and the providers.
use std::{collections::BTreeMap, path::Path, sync::{Arc, atomic::{AtomicUsize, Ordering}}};
use cpg_core::facts::compile_facts;
use cpg_extract::{bundle::{CapturedInputs, ProviderStage, StageContext}, capture::CapturedInput};
use lctx_model::domain::{*, artifact::ArtifactChunk, input::*, memory::MemoryGeneration, resources::ResourceBudget, source::SourceArtifact, stages::*};

/// The registered fixture cases and the files each one captures.
const CASES: &[(&str, &[&str])] = &[("typed_semantics", &["sample.py", "_invalid/broken.py"])];

fn budget() -> ResourceBudget { ResourceBudget::fixed(1 << 30).unwrap() }
fn stage(name: &'static str, outputs: Vec<RelationUse>, profiles: Vec<Profile>, configuration: &[u8]) -> Stage {
    Stage { name, inputs: vec![], outputs, contributes: vec![], coverage: vec![], provider: None, profiles, effect: Effect::Acquisition,
        code: ContentHash::of(name.as_bytes()), configuration: ContentHash::of(configuration) }
}
/// Writes each captured input: its revision, a tree origin, the acquisition, artifacts and chunks.
struct Capture(Stage, Arc<AtomicUsize>);
impl ProviderStage<MemoryGeneration> for Capture {
    fn declaration(&self, _: Profile) -> Stage { self.0.clone() }
    fn run(&mut self, context: &mut StageContext<MemoryGeneration>) -> Result<ProviderOutcome, ModelError> {
        self.1.fetch_add(1, Ordering::Relaxed);
        for output in ["revision", "origin", "acquisition", "artifact", "chunk"] {
            match output {
                "revision" => context.declare::<InputRevision>()?, "origin" => context.declare::<InputOrigin>()?,
                "acquisition" => context.declare::<InputAcquisition>()?, "artifact" => context.declare::<SourceArtifact>()?,
                _ => context.declare::<ArtifactChunk>()?,
            }
        }
        let captured = context.captured();
        for input in captured.inputs() {
            let origin = InputOrigin::Tree { label: "fixture-corpus".into() };
            context.emit(InputAcquisition { input: input.revision().id(), origin: origin.id() })?;
            context.emit(input.revision().clone())?;
            context.emit(origin)?;
            for artifact in input.artifacts() { context.emit(artifact.clone())?; }
            for index in 0..input.artifacts().len() {
                input.emit_chunks(index, |chunk| context.emit(chunk.clone())).map_err(|e| ModelError::Invalid(e.to_string()))?;
            }
            input.verify().map_err(|e| ModelError::Invalid(e.to_string()))?;
        }
        Ok(ProviderOutcome::Complete)
    }
}
fn capture_stage(configuration: &[u8]) -> Stage {
    stage("capture", vec![RelationUse::of::<InputRevision>(), RelationUse::of::<InputOrigin>(), RelationUse::of::<InputAcquisition>(),
        RelationUse::of::<SourceArtifact>(), RelationUse::of::<ArtifactChunk>()], vec![Profile::Catalog, Profile::Behavioral], configuration)
}
/// A provider that must never run.
struct Unused(Stage);
impl ProviderStage<MemoryGeneration> for Unused {
    fn declaration(&self, _: Profile) -> Stage { self.0.clone() }
    fn run(&mut self, _: &mut StageContext<MemoryGeneration>) -> Result<ProviderOutcome, ModelError> { panic!("{} must not run", self.0.name) }
}

/// Capture a fresh copy of one registered fixture case in its own temporary tree.
fn capture(case: &str, files: &[&str]) -> CapturedInput {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/python").join(case);
    let copy = tempfile::tempdir().unwrap();
    for file in files {
        let target = copy.path().join(file);
        std::fs::create_dir_all(target.parent().unwrap()).unwrap();
        std::fs::copy(root.join(file), target).unwrap();
    }
    CapturedInput::capture(copy.path(), &files.iter().map(|f| (*f).to_owned()).collect::<Vec<_>>(), &budget()).unwrap()
}
/// Drive one case through the providers into a memory generation; return its content digest and receipt.
async fn run(model: &Arc<ValidatedModel>, profile: Profile, providers: Vec<Box<dyn ProviderStage<MemoryGeneration>>>, case: &str, files: &[&str])
    -> Result<(ContentHash, BTreeMap<&'static str, ProviderOutcome>), ModelError> {
    let schedule = Schedule::build(model, vec![capture_stage(b"capture")], &[], profile)?;
    let mut execution = schedule.execute();
    let budget = budget();
    let generation = MemoryGeneration::bind(model, &budget, &mut execution)?;
    let captured = Arc::new(CapturedInputs::new(vec![capture(case, files)]));
    let receipt = compile_facts(execution, providers, &generation, model, &captured, &budget).await?;
    Ok((generation.validate(model, &budget)?, receipt.outcomes().clone()))
}

#[tokio::test]
async fn fixture_corpus_skeleton_runs_every_registered_case_through_memory() {
    let model = Arc::new(model().unwrap());
    let ran = Arc::new(AtomicUsize::new(0));
    let registered: Vec<&str> = CASES.iter().map(|(case, _)| *case).collect();
    for (case, files) in CASES {
        assert!(registered.contains(case));
        for profile in Profile::ALL {
            let first = run(&model, profile, vec![Box::new(Capture(capture_stage(b"capture"), ran.clone()))], case, files).await.unwrap();
            let second = run(&model, profile, vec![Box::new(Capture(capture_stage(b"capture"), ran.clone()))], case, files).await.unwrap();
            assert_eq!(first, second, "{case}: a relocated capture gives the same validated content");
            assert_eq!(first.1.get("capture"), Some(&ProviderOutcome::Complete));
        }
    }
}

#[tokio::test]
async fn the_providers_must_be_exactly_the_scheduled_stages() {
    let model = Arc::new(model().unwrap());
    let (case, files) = CASES[0];
    let ran = Arc::new(AtomicUsize::new(0));
    let flow = stage("flow", vec![RelationUse::of::<Package>()], vec![Profile::Behavioral], b"flow");
    // A provider outside the schedule's profile is not offered; the catalog schedule runs without it.
    let catalog = run(&model, Profile::Catalog, vec![Box::new(Capture(capture_stage(b"capture"), ran.clone())), Box::new(Unused(flow.clone()))], case, files).await;
    assert!(catalog.is_ok(), "{catalog:?}");
    assert_eq!(ran.swap(0, Ordering::Relaxed), 1);
    let refusals: Vec<(&str, Vec<Box<dyn ProviderStage<MemoryGeneration>>>)> = vec![
        ("a scheduled stage without a provider", vec![]),
        ("a provider the schedule does not name", vec![Box::new(Capture(capture_stage(b"capture"), ran.clone())),
            Box::new(Unused(stage("stray", vec![RelationUse::of::<Package>()], vec![Profile::Catalog], b"stray")))]),
        ("a declaration that differs from its scheduled stage", vec![Box::new(Unused(capture_stage(b"other configuration")))]),
        ("two providers for one stage", vec![Box::new(Capture(capture_stage(b"capture"), ran.clone())), Box::new(Unused(capture_stage(b"capture")))]),
    ];
    for (name, providers) in refusals {
        let refused = run(&model, Profile::Catalog, providers, case, files).await;
        assert!(matches!(&refused, Err(ModelError::Invalid(message)) if !message.contains("panicked")), "{name}: {refused:?}");
    }
    assert_eq!(ran.load(Ordering::Relaxed), 0, "a refused provider set runs no stage");
}
