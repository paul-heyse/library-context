//! The facts frontier (plan D1; review focus #2): absent, unknown, unrequested and empty stay
//! distinct from producer schedule through coverage to admission.
use std::{collections::BTreeMap, future::Future, task::{Context, Poll, Waker}};
use lctx_model::domain::{*, admission::*, attribution::*, calls::*, declarations::*, symbols::*, deployment::*, documents::*, flow::*, input::*,
    lexical::*, resources::ResourceBudget, source::*, stages::*, syntax::*, transfer::TransferKey, types::*};

fn ready<T>(future: impl Future<Output = T>) -> T {
    match std::pin::pin!(future).as_mut().poll(&mut Context::from_waker(Waker::noop())) {
        Poll::Ready(value) => value, Poll::Pending => panic!("test effect unexpectedly pending"),
    }
}
fn budget() -> ResourceBudget { ResourceBudget::fixed(64 << 20).unwrap() }

/// Each stage writes every declared output, explicitly empty.
type Writer = fn(&mut StageAccess<'_, '_>) -> Result<(), ModelError>;
fn write_acquire(access: &mut StageAccess<'_, '_>) -> Result<(), ModelError> {
    ready(access.write::<InputRevision, _>(async |_| Ok(())))?; ready(access.write::<SourceArtifact, _>(async |_| Ok(())))
}
fn write_pyrefly(access: &mut StageAccess<'_, '_>) -> Result<(), ModelError> {
    macro_rules! all { ($($ty:ty),+) => { $( ready(access.write::<$ty, _>(async |_| Ok(())))?; )+ }; }
    all!(Occurrence, SyntaxObservation, SyntaxSupport, SyntaxPlacement, SyntaxPlacementSupport, SyntaxDetailObservation, SyntaxDetailSupport, DeclarationObservation, DeclarationSupport,
        DeclarationDecorator, DeclarationDecoratorSupport, ImportAliasObservation, ImportAliasSupport, DunderAllObservation, DunderAllSupport,
        ParameterSyntaxObservation, ParameterSyntaxSupport, ClassFieldSyntaxObservation, ClassFieldSyntaxSupport, CallSyntax, CallSyntaxSupport, LexicalScopeObservation, LexicalScopeSupport,
        BindingObservation, BindingSupport, ReferenceObservation, ReferenceSupport, LexicalResolution, LexicalResolutionSupport,
        Signature, SignatureSupport, SymbolDeclaration, SymbolDeclarationSupport, ParameterDeclaration, ParameterDeclarationSupport,
        CallTarget, CallTargetSupport, CallResolution, CallResolutionSupport, TypeObservation, TypeSupport, TypePresentation,
        TypePresentationSupport, TypeVariableRestriction, TypeRestrictionSupport, SymbolObservation, SymbolSupport, FunctionTraitObservation, FunctionTraitSupport,
        ClassTraitObservation, ClassTraitSupport, ClassAncestryObservation, ClassAncestrySupport, ParameterAnnotationObservation, ParameterAnnotationSupport,
        PublicNameObservation, PublicNameSupport, ParameterDocObservation, ParameterDocSupport, DependencyModuleObservation, DependencyModuleSupport);
    Ok(())
}
fn pyrefly_outputs() -> Vec<RelationUse> {
    macro_rules! all { ($($ty:ty),+) => { vec![$(RelationUse::of::<$ty>()),+] }; }
    all!(Occurrence, SyntaxObservation, SyntaxSupport, SyntaxPlacement, SyntaxPlacementSupport, SyntaxDetailObservation, SyntaxDetailSupport, DeclarationObservation, DeclarationSupport,
        DeclarationDecorator, DeclarationDecoratorSupport, ImportAliasObservation, ImportAliasSupport, DunderAllObservation, DunderAllSupport,
        ParameterSyntaxObservation, ParameterSyntaxSupport, ClassFieldSyntaxObservation, ClassFieldSyntaxSupport, CallSyntax, CallSyntaxSupport, LexicalScopeObservation, LexicalScopeSupport,
        BindingObservation, BindingSupport, ReferenceObservation, ReferenceSupport, LexicalResolution, LexicalResolutionSupport,
        Signature, SignatureSupport, SymbolDeclaration, SymbolDeclarationSupport, ParameterDeclaration, ParameterDeclarationSupport,
        CallTarget, CallTargetSupport, CallResolution, CallResolutionSupport, TypeObservation, TypeSupport, TypePresentation,
        TypePresentationSupport, TypeVariableRestriction, TypeRestrictionSupport, SymbolObservation, SymbolSupport, FunctionTraitObservation, FunctionTraitSupport,
        ClassTraitObservation, ClassTraitSupport, ClassAncestryObservation, ClassAncestrySupport, ParameterAnnotationObservation, ParameterAnnotationSupport,
        PublicNameObservation, PublicNameSupport, ParameterDocObservation, ParameterDocSupport, DependencyModuleObservation, DependencyModuleSupport)
}
fn write_flow(access: &mut StageAccess<'_, '_>) -> Result<(), ModelError> {
    macro_rules! all { ($($ty:ty),+) => { $( ready(access.write::<$ty, _>(async |_| Ok(())))?; )+ }; }
    all!(FlowUseObservation, FlowUseSupport, FlowDefinitionObservation, FlowDefinitionSupport, FlowReachingObservation, FlowReachingSupport,
        FlowValueObservation, FlowValueSupport, FlowRegionObservation, FlowRegionSupport);
    Ok(())
}
fn flow_outputs() -> Vec<RelationUse> {
    macro_rules! all { ($($ty:ty),+) => { vec![$(RelationUse::of::<$ty>()),+] }; }
    all!(FlowUseObservation, FlowUseSupport, FlowDefinitionObservation, FlowDefinitionSupport, FlowReachingObservation, FlowReachingSupport,
        FlowValueObservation, FlowValueSupport, FlowRegionObservation, FlowRegionSupport)
}
fn write_documents(access: &mut StageAccess<'_, '_>) -> Result<(), ModelError> {
    macro_rules! all { ($($ty:ty),+) => { $( ready(access.write::<$ty, _>(async |_| Ok(())))?; )+ }; }
    all!(DocumentObservation, DocumentSupport, PassageObservation, PassageSupport, CodeBlockObservation, CodeBlockSupport, DocumentLinkObservation,
        DocumentLinkSupport, DocumentMentionObservation, DocumentMentionSupport, DocumentComponentObservation, DocumentComponentSupport,
        DocumentAttributeObservation, DocumentAttributeSupport);
    Ok(())
}
fn document_outputs() -> Vec<RelationUse> {
    macro_rules! all { ($($ty:ty),+) => { vec![$(RelationUse::of::<$ty>()),+] }; }
    all!(DocumentObservation, DocumentSupport, PassageObservation, PassageSupport, CodeBlockObservation, CodeBlockSupport, DocumentLinkObservation,
        DocumentLinkSupport, DocumentMentionObservation, DocumentMentionSupport, DocumentComponentObservation, DocumentComponentSupport,
        DocumentAttributeObservation, DocumentAttributeSupport)
}
fn write_deployment(access: &mut StageAccess<'_, '_>) -> Result<(), ModelError> {
    ready(access.write::<TaskReportObservation, _>(async |_| Ok(())))?; ready(access.write::<TaskReportSupport, _>(async |_| Ok(())))?;
    ready(access.write::<DeploymentObservation, _>(async |_| Ok(())))?; ready(access.write::<DeploymentSupport, _>(async |_| Ok(())))
}
fn write_assemble(access: &mut StageAccess<'_, '_>) -> Result<(), ModelError> {
    ready(access.write::<ProviderCoverage, _>(async |_| Ok(())))?; ready(access.write::<CoverageScope, _>(async |_| Ok(())))
}

struct World {
    model: ValidatedModel, input: InputRevision, artifacts: Vec<SourceArtifact>, context: AnalysisContext,
    capture: Provider, pyrefly: Provider, ty: Provider, docs: Provider, deploy: Provider,
}
impl World {
    fn new(paths: &[&str]) -> Self {
        let files: Vec<(String, Vec<u8>)> = paths.iter().map(|p| ((*p).to_owned(), format!("# {p}\n").into_bytes())).collect();
        let input = InputRevision::from_entries(files.iter().map(|(path, bytes)| ManifestEntry { path: path.clone(), content: ContentHash::of(bytes),
            byte_len: bytes.len() as i64 }).collect()).unwrap();
        let artifacts = files.iter().map(|(path, bytes)| SourceArtifact::from_bytes(input.id(), path.clone(), bytes).unwrap()).collect();
        let context = AnalysisContext { python_version: "3.14.7".into(), python_platform: "linux".into(), search_path: vec![], site_package_path: vec![],
            config_digest: ContentHash::of(b"admission"), environment_digest: input.manifest, lock_digest: None };
        let provider = |tool: &str| Provider { tool: tool.into(), revision: "pinned".into(), build_digest: ContentHash::of(tool.as_bytes()) };
        Self { model: model().unwrap(), input, artifacts, context, capture: provider("capture"), pyrefly: provider("pyrefly"), ty: provider("ty"),
            docs: provider("documents"), deploy: provider("deployment") }
    }
    fn stages(&self, flow_profiles: Vec<Profile>) -> Vec<(Stage, Writer)> {
        let both = || vec![Profile::Catalog, Profile::Behavioral];
        let stage = |name, outputs, coverage: Vec<FactFamily>, provider: Option<&Provider>, profiles| Stage { name, inputs: vec![], outputs, contributes: vec![],
            coverage, provider: provider.map(Record::id), profiles, effect: Effect::Extraction, code: ContentHash::of(name.as_bytes()), configuration: ContentHash::of(b"cfg") };
        use FactFamily::*;
        vec![
            (stage("acquire", vec![RelationUse::of::<InputRevision>(), RelationUse::of::<SourceArtifact>()], vec![Artifacts], Some(&self.capture), both()), write_acquire as Writer),
            (stage("pyrefly", pyrefly_outputs(), vec![Syntax, Lexical, Signatures, Calls, Types, Exports], Some(&self.pyrefly), both()), write_pyrefly),
            (stage("ty_flow", flow_outputs(), vec![Flow], Some(&self.ty), flow_profiles), write_flow),
            (stage("documents", document_outputs(), vec![Docs], Some(&self.docs), both()), write_documents),
            (stage("deployment", vec![RelationUse::of::<TaskReportObservation>(), RelationUse::of::<TaskReportSupport>(), RelationUse::of::<DeploymentObservation>(),
                RelationUse::of::<DeploymentSupport>()], vec![Deployment], Some(&self.deploy), both()), write_deployment),
            (stage("assemble", vec![RelationUse::of::<ProviderCoverage>(), RelationUse::of::<CoverageScope>()], vec![], None, both()), write_assemble),
        ]
    }
    fn schedule(&self, profile: Profile, stages: &[(Stage, Writer)]) -> Schedule {
        Schedule::build(&self.model, stages.iter().map(|(stage, _)| stage.clone()).collect(), &[], profile).unwrap()
    }
    /// Run every scheduled stage, writing its declared outputs, with the given outcomes.
    fn run(&self, schedule: &Schedule, stages: &[(Stage, Writer)], outcomes: &BTreeMap<&str, ProviderOutcome>) -> ExecutionReceipt {
        let mut execution = schedule.execute();
        for scheduled in schedule.stages() {
            let writer = stages.iter().find(|(stage, _)| stage.name == scheduled.name).unwrap().1;
            let mut access = execution.begin(scheduled.name).unwrap();
            writer(&mut access).unwrap();
            access.finish(outcomes.get(scheduled.name).copied().unwrap_or(ProviderOutcome::Complete)).unwrap();
        }
        execution.finish().unwrap()
    }
    fn artifact(&self, path: &str) -> &SourceArtifact { self.artifacts.iter().find(|a| a.path == path).unwrap() }
    fn run_of(&self, provider: &Provider, family: FactFamily) -> ProviderRun {
        ProviderRun::new(provider.id(), self.context.id(), self.input.id(), self.context.config_digest, [family]).unwrap().0
    }
    fn row(&self, scope: &CoverageScope, provider: &Provider, family: FactFamily, status: CoverageStatus) -> ProviderCoverage {
        let reason = match status { CoverageStatus::CompleteUnderStatedModel | CoverageStatus::NotRequested => None, _ => Some(ObligationKind::UndecodableSource) };
        let run = (status != CoverageStatus::NotRequested).then(|| self.run_of(provider, family).id());
        ProviderCoverage { scope: scope.id(), provider: provider.id(), context: self.context.id(), family, run, status, reason, diagnostic: None }
    }
    /// The coverage a faithful producer states: the undecodable module is Unavailable for every
    /// module-grained family; Flow is NotRequested in the catalog profile.
    fn coverage(&self, profile: Profile) -> (Vec<CoverageScope>, Vec<ProviderCoverage>) {
        use CoverageStatus::*; use FactFamily::*;
        let input = CoverageScope::Input { input: self.input.id() };
        let mut scopes = vec![input.clone()];
        let mut rows = vec![self.row(&input, &self.capture, Artifacts, CompleteUnderStatedModel), self.row(&input, &self.deploy, Deployment, CompleteUnderStatedModel)];
        for artifact in &self.artifacts {
            let scope = CoverageScope::Artifact { artifact: artifact.id() };
            match ArtifactClass::of(&artifact.path) {
                Some(ArtifactClass::PythonSource) => {
                    let status = if artifact.path.starts_with("_invalid/") { Unavailable } else { CompleteUnderStatedModel };
                    for family in [Syntax, Lexical, Signatures, Calls, Types, Exports] { rows.push(self.row(&scope, &self.pyrefly, family, status)); }
                    rows.push(self.row(&scope, &self.ty, Flow, if profile == Profile::Catalog { NotRequested } else { status }));
                },
                Some(ArtifactClass::Document) => rows.push(self.row(&scope, &self.docs, Docs, CompleteUnderStatedModel)),
                None => {},
            }
            scopes.push(scope);
        }
        (scopes, rows)
    }
    fn admit(&self, preflight: &Preflight, receipt: &ExecutionReceipt, scopes: &[CoverageScope], rows: &[ProviderCoverage]) -> Result<FactsAdmission, ModelError> {
        self.admit_classified(preflight, receipt, scopes, rows, &self.artifacts)
    }
    /// Admission with `unowned` stated as unowned artifacts; any other artifact has no class.
    fn admit_classified(&self, preflight: &Preflight, receipt: &ExecutionReceipt, scopes: &[CoverageScope], rows: &[ProviderCoverage],
        unowned: &[SourceArtifact]) -> Result<FactsAdmission, ModelError> {
        let mut check = AdmissionCheck::new(preflight.clone(), &budget());
        let acquisition = InputAcquisition { input: self.input.id(), origin: InputOrigin::Tree { label: "admission".into() }.id() };
        let unowned = unowned.iter().map(|artifact| UnownedArtifact { artifact: artifact.id(), acquisition: acquisition.id() }).collect();
        check.visit(InputRevision::NAME, Batch::new(&self.model, vec![self.input.clone()], &budget())?.arrow())?;
        check.visit(SourceArtifact::NAME, Batch::new(&self.model, self.artifacts.clone(), &budget())?.arrow())?;
        check.visit(UnownedArtifact::NAME, Batch::new(&self.model, unowned, &budget())?.arrow())?;
        check.visit(CoverageScope::NAME, Batch::new(&self.model, scopes.to_vec(), &budget())?.arrow())?;
        check.visit(ProviderCoverage::NAME, Batch::new(&self.model, rows.to_vec(), &budget())?.arrow())?;
        check.finish(receipt, ContentHash::of(b"content"))
    }
}
const INPUT: &[&str] = &["a.py", "b.pyi", "_invalid/undecodable.py", "README.md"];
fn partial() -> BTreeMap<&'static str, ProviderOutcome> { BTreeMap::from([("pyrefly", ProviderOutcome::Partial), ("ty_flow", ProviderOutcome::Partial)]) }
fn frontier_refusal<T: std::fmt::Debug>(result: Result<T, ModelError>, expected: &str) {
    match result {
        Err(ModelError::Frontier(message)) => assert!(message.contains(expected), "expected `{expected}`, got `{message}`"),
        other => panic!("expected a frontier refusal `{expected}`, got {other:?}"),
    }
}

#[test]
fn each_profile_expects_exactly_its_rows() {
    let w = World::new(INPUT);
    let stages = w.stages(vec![Profile::Behavioral]);
    for profile in [Profile::Catalog, Profile::Behavioral] {
        let contract = FrontierContract::facts(&w.model, profile).unwrap();
        let preflight = contract.preflight(&w.schedule(profile, &stages)).unwrap();
        let expected = preflight.expected_coverage(std::slice::from_ref(&w.input), &w.artifacts).unwrap();
        // Artifacts and Deployment per input; six Pyrefly families and Flow per Python source (three);
        // Docs for README.md.
        assert_eq!(expected.len(), 1 + 1 + 3 * 6 + 3 + 1);
        let flow: Vec<_> = expected.keys().filter(|key| key.family == FactFamily::Flow).collect();
        assert_eq!(flow.len(), 3);
        let requested = profile == Profile::Behavioral;
        assert!(flow.iter().all(|key| key.provider == requested.then_some(w.ty.id())), "{profile:?}: Flow is requested only in the behavioral profile");
        assert_eq!(contract.requested(FactFamily::Flow), requested);
    }
}

#[test]
fn faithful_coverage_is_admitted_with_disclosed_availability() {
    let w = World::new(INPUT);
    let stages = w.stages(vec![Profile::Behavioral]);
    for profile in [Profile::Catalog, Profile::Behavioral] {
        let schedule = w.schedule(profile, &stages);
        let preflight = FrontierContract::facts(&w.model, profile).unwrap().preflight(&schedule).unwrap();
        let receipt = w.run(&schedule, &stages, &partial());
        let (scopes, rows) = w.coverage(profile);
        let admission = w.admit(&preflight, &receipt, &scopes, &rows).unwrap();
        assert_eq!((admission.frontier(), admission.profile(), admission.schedule(), admission.model()), (Frontier::Facts, profile, schedule.digest(), w.model.digest()));
        let availability = admission.availability();
        assert_eq!(availability[&FactFamily::Syntax], Availability::Partial, "the undecodable module is disclosed, not dropped");
        assert_eq!(availability[&FactFamily::Docs], Availability::Complete);
        assert_eq!(availability[&FactFamily::Flow], if profile == Profile::Catalog { Availability::NotRequested } else { Availability::Partial });
        // The same coverage in another order is the same admission.
        let mut reversed = rows.clone(); reversed.reverse();
        assert_eq!(w.admit(&preflight, &receipt, &scopes, &reversed).unwrap(), admission);
    }
}

/// Plan A1: the captured closure is complete only when every artifact has an ownership class.
#[test]
fn an_artifact_without_an_ownership_class_is_refused() {
    let w = World::new(INPUT);
    let stages = w.stages(vec![Profile::Behavioral]);
    let schedule = w.schedule(Profile::Catalog, &stages);
    let preflight = FrontierContract::facts(&w.model, Profile::Catalog).unwrap().preflight(&schedule).unwrap();
    let receipt = w.run(&schedule, &stages, &partial());
    let (scopes, rows) = w.coverage(Profile::Catalog);
    frontier_refusal(w.admit_classified(&preflight, &receipt, &scopes, &rows, &w.artifacts[1..]), "has no ownership class");
    assert!(w.admit_classified(&preflight, &receipt, &scopes, &rows, &w.artifacts).is_ok(), "the twin with every artifact classified is admitted");
}

#[test]
fn an_input_without_a_family_scope_is_admitted_as_no_scope() {
    let w = World::new(&["README.md"]);
    let stages = w.stages(vec![Profile::Behavioral]);
    let schedule = w.schedule(Profile::Catalog, &stages);
    let preflight = FrontierContract::facts(&w.model, Profile::Catalog).unwrap().preflight(&schedule).unwrap();
    let receipt = w.run(&schedule, &stages, &BTreeMap::new());
    let (scopes, rows) = w.coverage(Profile::Catalog);
    let admission = w.admit(&preflight, &receipt, &scopes, &rows).unwrap();
    assert_eq!(admission.availability()[&FactFamily::Syntax], Availability::NoScope, "no Python source: required Syntax has nothing to cover");
    assert_eq!(admission.availability()[&FactFamily::Deployment], Availability::Complete, "an empty but complete scope");
}

#[test]
fn coverage_that_misstates_the_frontier_is_refused() {
    let w = World::new(INPUT);
    let stages = w.stages(vec![Profile::Behavioral]);
    let schedule = w.schedule(Profile::Catalog, &stages);
    let preflight = FrontierContract::facts(&w.model, Profile::Catalog).unwrap().preflight(&schedule).unwrap();
    let receipt = w.run(&schedule, &stages, &partial());
    let (scopes, rows) = w.coverage(Profile::Catalog);
    let syntax_of = |path: &str| rows.iter().position(|r| r.family == FactFamily::Syntax && r.scope == CoverageScope::Artifact { artifact: w.artifact(path).id() }.id()).unwrap();
    let mut missing = rows.clone(); missing.remove(syntax_of("a.py"));
    frontier_refusal(w.admit(&preflight, &receipt, &scopes, &missing), "missing Syntax coverage");
    let readme = CoverageScope::Artifact { artifact: w.artifact("README.md").id() };
    let mut extra = rows.clone(); extra.push(w.row(&readme, &w.pyrefly, FactFamily::Syntax, CoverageStatus::CompleteUnderStatedModel));
    frontier_refusal(w.admit(&preflight, &receipt, &scopes, &extra), "unexpected Syntax coverage row");
    let mut duplicate = rows.clone();
    let again = ProviderRun::new(w.pyrefly.id(), w.context.id(), w.input.id(), ContentHash::of(b"another configuration"), [FactFamily::Syntax]).unwrap().0;
    duplicate.push(ProviderCoverage { run: Some(again.id()), ..rows[syntax_of("a.py")].clone() });
    frontier_refusal(w.admit(&preflight, &receipt, &scopes, &duplicate), "duplicate Syntax coverage");
    let a = CoverageScope::Artifact { artifact: w.artifact("a.py").id() };
    let mut attempted = rows.clone();
    let flow = attempted.iter().position(|r| r.family == FactFamily::Flow && r.scope == a.id()).unwrap();
    attempted[flow] = w.row(&a, &w.ty, FactFamily::Flow, CoverageStatus::CompleteUnderStatedModel);
    frontier_refusal(w.admit(&preflight, &receipt, &scopes, &attempted), "Flow was attempted");
    let mut failed = rows.clone();
    failed[syntax_of("a.py")].status = CoverageStatus::Failed; failed[syntax_of("a.py")].reason = Some(ObligationKind::NativeUnavailable);
    frontier_refusal(w.admit(&preflight, &receipt, &scopes, &failed), "failed provider aborts");
    // A stage that reports Complete beside an Unavailable module it stated.
    let complete = w.run(&schedule, &stages, &BTreeMap::new());
    frontier_refusal(w.admit(&preflight, &complete, &scopes, &rows), "reported Complete");
    // A receipt of another schedule.
    let behavioral = w.schedule(Profile::Behavioral, &stages);
    frontier_refusal(w.admit(&preflight, &w.run(&behavioral, &stages, &partial()), &scopes, &rows), "another model or schedule");
}

#[test]
fn a_required_family_entirely_unavailable_is_refused_and_one_partial_is_not() {
    let w = World::new(INPUT);
    let stages = w.stages(vec![Profile::Behavioral]);
    let schedule = w.schedule(Profile::Catalog, &stages);
    let preflight = FrontierContract::facts(&w.model, Profile::Catalog).unwrap().preflight(&schedule).unwrap();
    let (scopes, mut rows) = w.coverage(Profile::Catalog);
    for row in rows.iter_mut().filter(|r| r.provider == w.pyrefly.id()) {
        row.status = CoverageStatus::Unavailable; row.reason = Some(ObligationKind::NativeUnavailable);
    }
    let receipt = w.run(&schedule, &stages, &BTreeMap::from([("pyrefly", ProviderOutcome::Unavailable)]));
    frontier_refusal(w.admit(&preflight, &receipt, &scopes, &rows), "required Syntax is entirely unavailable");
    assert!(w.admit(&preflight, &w.run(&schedule, &stages, &partial()), &scopes, &w.coverage(Profile::Catalog).1).is_ok());
}

#[test]
fn preflight_refuses_schedules_that_cannot_produce_the_frontier() {
    let w = World::new(INPUT);
    let contract = FrontierContract::facts(&w.model, Profile::Catalog).unwrap();
    let catalog = |stages: Vec<(Stage, Writer)>| Schedule::build(&w.model, stages.into_iter().map(|(s, _)| s).collect(), &[], Profile::Catalog).unwrap();
    // The stage-bound subset: acquisition and syntax only.
    let mut subset: Vec<_> = w.stages(vec![Profile::Behavioral]).into_iter().filter(|(s, _)| matches!(s.name, "acquire" | "pyrefly" | "assemble")).collect();
    subset[1].0.coverage = vec![FactFamily::Syntax];
    frontier_refusal(contract.preflight(&catalog(subset)), "no scheduled stage covers requested Lexical");
    let uncovered: Vec<_> = w.stages(vec![Profile::Behavioral]).into_iter().filter(|(s, _)| s.name != "documents").collect();
    frontier_refusal(contract.preflight(&catalog(uncovered)), "no scheduled stage covers requested Docs");
    // Flow scheduled in the catalog profile.
    frontier_refusal(contract.preflight(&catalog(w.stages(vec![Profile::Catalog, Profile::Behavioral]))), "attempts Flow");
    // A stage writing a relation above the frontier.
    let mut analysis = w.stages(vec![Profile::Behavioral]);
    analysis[5].0.outputs.push(RelationUse::of::<TransferKey>());
    frontier_refusal(contract.preflight(&catalog(analysis)), "above the facts frontier");
    // Syntax assertions written by a stage that does not report Syntax coverage.
    let mut misattributed = w.stages(vec![Profile::Behavioral]);
    misattributed[1].0.outputs.retain(|r| r.name() != SyntaxObservation::NAME);
    misattributed[3].0.outputs.push(RelationUse::of::<SyntaxObservation>());
    frontier_refusal(contract.preflight(&catalog(misattributed)), "syntax_observations is not written by a stage that reports Syntax coverage");
    // Flow assertions written although the catalog profile does not request Flow.
    let mut unrequested = w.stages(vec![Profile::Behavioral]);
    unrequested[1].0.outputs.push(RelationUse::of::<FlowUseObservation>());
    frontier_refusal(contract.preflight(&catalog(unrequested)), "of unrequested Flow");
    // No coverage writer.
    let mut silent = w.stages(vec![Profile::Behavioral]);
    silent[5].0.outputs = vec![RelationUse::of::<Package>()];
    frontier_refusal(contract.preflight(&catalog(silent)), "writes provider_coverage");
    // The faithful schedule passes, and a contract for another profile refuses it.
    let faithful = catalog(w.stages(vec![Profile::Behavioral]));
    assert!(contract.preflight(&faithful).is_ok());
    frontier_refusal(FrontierContract::facts(&w.model, Profile::Behavioral).unwrap().preflight(&faithful), "model or profile differs");
    // A model without the facts relations has no facts frontier.
    let bare = ValidatedModel::validate(vec![Relation::of::<Package>()]).unwrap();
    frontier_refusal(FrontierContract::facts(&bare, Profile::Catalog), "lacks facts relation");
}

#[test]
fn a_stage_outcome_must_agree_with_its_coverage() {
    let w = World::new(INPUT);
    let a = CoverageScope::Artifact { artifact: w.artifact("a.py").id() };
    let complete = w.row(&a, &w.pyrefly, FactFamily::Syntax, CoverageStatus::CompleteUnderStatedModel);
    let unavailable = w.row(&a, &w.pyrefly, FactFamily::Lexical, CoverageStatus::Unavailable);
    assert!(reconcile(ProviderOutcome::Complete, &[&complete]).is_ok());
    assert!(reconcile(ProviderOutcome::Complete, &[]).is_ok(), "a covering stage with no scope is complete");
    assert!(reconcile(ProviderOutcome::Complete, &[&complete, &unavailable]).is_err());
    assert!(reconcile(ProviderOutcome::Partial, &[&complete, &unavailable]).is_ok());
    assert!(reconcile(ProviderOutcome::Partial, &[&complete]).is_err(), "a partial stage states what it missed");
    assert!(reconcile(ProviderOutcome::Unavailable, &[&unavailable]).is_ok());
    assert!(reconcile(ProviderOutcome::Unavailable, &[&complete, &unavailable]).is_err());
    assert!(reconcile(ProviderOutcome::Failed, &[&complete]).is_err());
    assert_eq!(ArtifactClass::of("pkg/mod.pyi"), Some(ArtifactClass::PythonSource));
    assert_eq!(ArtifactClass::of("docs/guide.mdx"), Some(ArtifactClass::Document));
    assert_eq!(ArtifactClass::of("setup.cfg"), None);
}
