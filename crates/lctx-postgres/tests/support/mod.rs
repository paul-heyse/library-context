//! Shared real-PostgreSQL fixtures: a two-relation model for fast lifecycle controls and the
//! facts frontier over the full model (the D1 stage table in the catalog profile).
#![allow(dead_code)]
use std::sync::Arc;
use lctx_model::domain::{*, admission::*, attribution::*, calls::*, declarations::*, deployment::*, documents::*, input::*, lexical::*,
    resources::ResourceBudget, source::*, stages::*, types::*};
use lctx_postgres::generations::{GenerationAttempt, GenerationId, GenerationStore};
use lctx_postgres::testing::DisposableDatabase;
use sqlx::PgPool;

pub fn budget() -> ResourceBudget { ResourceBudget::fixed(DEFAULT_BUDGET).unwrap() }
const DEFAULT_BUDGET: usize = 256 << 20;

/// Write each listed relation's rows (or an explicit empty batch) through the attempt.
macro_rules! rows {
    ($access:expr, $attempt:expr, $model:expr; $($ty:ty => $rows:expr),* $(,)?) => {{ $( {
        let batch = Batch::<$ty>::new($model, $rows, &budget()).unwrap();
        $access.write::<$ty, _>(async |permit| $attempt.copy(permit, &batch).await).await.unwrap();
    } )* }};
}
macro_rules! empty {
    ($access:expr, $attempt:expr, $model:expr; $($ty:ty),* $(,)?) => { rows!($access, $attempt, $model; $($ty => vec![]),*) };
}

pub async fn state(db: &DisposableDatabase, g: GenerationId) -> Option<String> {
    sqlx::query_scalar("SELECT state FROM lctx_model_store.generations WHERE id = decode($1, 'hex')").bind(g.hex()).fetch_optional(&db.superuser).await.unwrap()
}
pub async fn count(db: &DisposableDatabase, sql: &str, g: GenerationId) -> i64 {
    sqlx::query_scalar(sqlx::AssertSqlSafe(sql.to_owned())).bind(g.hex()).fetch_one(&db.superuser).await.unwrap()
}

/// A two-relation model: releases reference packages, so a dangling release fails validation.
pub struct Small { pub model: Arc<ValidatedModel>, pub schedule: Schedule }
impl Small {
    pub fn new() -> Self {
        let model = Arc::new(ValidatedModel::validate(vec![Relation::of::<Package>(), Relation::of::<Release>()]).unwrap());
        let schedule = Schedule::build(&model, vec![Stage {
            name: "packages", inputs: vec![], outputs: vec![RelationUse::of::<Package>(), RelationUse::of::<Release>()], contributes: vec![], coverage: vec![],
            provider: None, profiles: vec![Profile::Catalog], effect: Effect::Extraction, code: ContentHash::of(b"packages"), configuration: ContentHash::of(b"cfg"),
        }], &[], Profile::Catalog).unwrap();
        Self { model, schedule }
    }
    /// A begun attempt with its stage written and its execution finished.
    pub async fn written(&self, store: &GenerationStore, writer: PgPool, packages: &[&str], releases: Vec<Release>) -> (GenerationAttempt, ExecutionReceipt) {
        let mut execution = self.schedule.execute();
        let attempt = store.begin_conformance(writer, &mut execution, budget()).await.unwrap();
        let mut access = execution.begin("packages").unwrap();
        rows!(access, attempt, &self.model; Package => packages.iter().map(|name| Package { name: (*name).into() }).collect(), Release => releases);
        access.finish(ProviderOutcome::Complete).unwrap();
        (attempt, execution.finish().unwrap())
    }
    /// A manual conformance generation, published.
    pub async fn published(&self, store: &GenerationStore, writer: &PgPool, name: &str) -> GenerationId {
        let g = store.create_conformance(ContentHash::of(name.as_bytes()), "catalog").await.unwrap();
        store.copy(writer, g, &Batch::new(&self.model, vec![Package { name: name.into() }], &budget()).unwrap(), &budget()).await.unwrap();
        store.seal(g).await.unwrap();
        store.validate(g, &budget()).await.unwrap();
        store.publish(g).await.unwrap();
        g
    }
}

/// The facts frontier over the full model: one empty input, the D1 stage table in the catalog
/// profile, and the two input-grained coverage rows (Artifacts, Deployment) it requires.
pub struct Facts { pub model: Arc<ValidatedModel>, pub input: InputRevision, pub context: AnalysisContext, pub capture: Provider, pub pyrefly: Provider, pub docs: Provider, pub deploy: Provider }
impl Facts {
    pub fn new() -> Self {
        let input = InputRevision::from_entries(vec![]).unwrap();
        let context = AnalysisContext { python_version: "3.14.7".into(), python_platform: "linux".into(), search_path: vec![], site_package_path: vec![],
            config_digest: ContentHash::of(b"lifecycle"), environment_digest: input.manifest, lock_digest: None };
        let provider = |tool: &str| Provider { tool: tool.into(), revision: "pinned".into(), build_digest: ContentHash::of(tool.as_bytes()) };
        Self { model: Arc::new(model().unwrap()), input, context, capture: provider("capture"), pyrefly: provider("pyrefly"), docs: provider("documents"),
            deploy: provider("deployment") }
    }
    pub fn schedule(&self) -> Schedule {
        let stage = |name, outputs, coverage: Vec<FactFamily>, provider: Option<&Provider>| Stage { name, inputs: vec![], outputs, contributes: vec![], coverage,
            provider: provider.map(Record::id), profiles: vec![Profile::Catalog], effect: Effect::Extraction, code: ContentHash::of(name.as_bytes()),
            configuration: ContentHash::of(b"cfg") };
        macro_rules! uses { ($($ty:ty),+) => { vec![$(RelationUse::of::<$ty>()),+] }; }
        use FactFamily::*;
        Schedule::build(&self.model, vec![
            stage("acquire", uses!(InputRevision, SourceArtifact), vec![Artifacts], Some(&self.capture)),
            stage("pyrefly", uses!(Occurrence, SyntaxObservation, SyntaxSupport, CallSyntax, CallSyntaxSupport, LexicalScopeObservation, LexicalScopeSupport,
                BindingObservation, BindingSupport, ReferenceObservation, ReferenceSupport, LexicalResolution, LexicalResolutionSupport, Signature, SignatureSupport,
                SymbolDeclaration, SymbolDeclarationSupport, ParameterDeclaration, ParameterDeclarationSupport, CallTarget, CallTargetSupport, CallResolution,
                CallResolutionSupport, TypeObservation, TypeSupport, TypePresentation, TypePresentationSupport, TypeVariableRestriction, TypeRestrictionSupport),
                vec![Syntax, Lexical, Signatures, Calls, Types, Exports], Some(&self.pyrefly)),
            stage("documents", uses!(DocumentObservation, DocumentSupport, PassageObservation, PassageSupport, CodeBlockObservation, CodeBlockSupport,
                DocumentLinkObservation, DocumentLinkSupport, DocumentMentionObservation, DocumentMentionSupport, DocumentComponentObservation,
                DocumentComponentSupport, DocumentAttributeObservation, DocumentAttributeSupport), vec![Docs], Some(&self.docs)),
            stage("deployment", uses!(TaskReportObservation, TaskReportSupport, DeploymentObservation, DeploymentSupport), vec![Deployment], Some(&self.deploy)),
            stage("assemble", uses!(ProviderCoverage, CoverageScope, Provider, AnalysisContext, ProviderRun, RunFamily), vec![], None),
        ], &[], Profile::Catalog).unwrap()
    }
    pub fn contract(&self) -> FrontierContract { FrontierContract::facts(&self.model, Profile::Catalog).unwrap() }
    /// Run every stage; `deployment` states the Deployment coverage row the frontier requires.
    pub async fn written(&self, store: &GenerationStore, writer: PgPool, deployment: bool) -> (GenerationAttempt, ExecutionReceipt) {
        let schedule = self.schedule();
        let mut execution = schedule.execute();
        let attempt = store.begin(writer, &mut execution, &self.contract(), budget()).await.unwrap();
        let model = &*self.model;
        let scope = CoverageScope::Input { input: self.input.id() };
        let (capture_run, capture_families) = ProviderRun::new(self.capture.id(), self.context.id(), self.input.id(), self.context.config_digest, [FactFamily::Artifacts]).unwrap();
        let (deploy_run, deploy_families) = ProviderRun::new(self.deploy.id(), self.context.id(), self.input.id(), self.context.config_digest, [FactFamily::Deployment]).unwrap();
        let row = |provider: &Provider, run: &ProviderRun, family| ProviderCoverage { scope: scope.id(), provider: provider.id(), context: self.context.id(), family,
            run: Some(run.id()), status: CoverageStatus::CompleteUnderStatedModel, reason: None, diagnostic: None };
        let mut coverage = vec![row(&self.capture, &capture_run, FactFamily::Artifacts)];
        if deployment { coverage.push(row(&self.deploy, &deploy_run, FactFamily::Deployment)); }
        for stage in schedule.stages() {
            let mut access = execution.begin(stage.name).unwrap();
            match stage.name {
                "acquire" => rows!(access, attempt, model; InputRevision => vec![self.input.clone()], SourceArtifact => vec![]),
                "pyrefly" => empty!(access, attempt, model; Occurrence, SyntaxObservation, SyntaxSupport, CallSyntax, CallSyntaxSupport, LexicalScopeObservation,
                    LexicalScopeSupport, BindingObservation, BindingSupport, ReferenceObservation, ReferenceSupport, LexicalResolution, LexicalResolutionSupport,
                    Signature, SignatureSupport, SymbolDeclaration, SymbolDeclarationSupport, ParameterDeclaration, ParameterDeclarationSupport, CallTarget,
                    CallTargetSupport, CallResolution, CallResolutionSupport, TypeObservation, TypeSupport, TypePresentation, TypePresentationSupport,
                    TypeVariableRestriction, TypeRestrictionSupport),
                "documents" => empty!(access, attempt, model; DocumentObservation, DocumentSupport, PassageObservation, PassageSupport, CodeBlockObservation,
                    CodeBlockSupport, DocumentLinkObservation, DocumentLinkSupport, DocumentMentionObservation, DocumentMentionSupport, DocumentComponentObservation,
                    DocumentComponentSupport, DocumentAttributeObservation, DocumentAttributeSupport),
                "deployment" => empty!(access, attempt, model; TaskReportObservation, TaskReportSupport, DeploymentObservation, DeploymentSupport),
                "assemble" => rows!(access, attempt, model; ProviderCoverage => coverage.clone(), CoverageScope => vec![scope.clone()],
                    Provider => vec![self.capture.clone(), self.deploy.clone()], AnalysisContext => vec![self.context.clone()],
                    ProviderRun => vec![capture_run.clone(), deploy_run.clone()], RunFamily => capture_families.iter().chain(&deploy_families).cloned().collect()),
                other => panic!("unscheduled stage {other}"),
            }
            access.finish(ProviderOutcome::Complete).unwrap();
        }
        (attempt, execution.finish().unwrap())
    }
    pub async fn published(&self, store: &GenerationStore, writer: PgPool) -> GenerationId {
        let (attempt, receipt) = self.written(store, writer, true).await;
        attempt.seal(receipt).await.unwrap().validate().await.unwrap().publish().await.unwrap()
    }
}

