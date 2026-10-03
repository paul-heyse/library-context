//! Disposable PostgreSQL 18 databases for tests (feature `testing`), bootstrapped as production is:
//! a non-superuser service owner owns database `lctx`, and each runtime role logs in separately.
use crate::OwnerPool;
use sqlx::PgPool;
use std::path::Path;
use testcontainers_modules::{
    postgres::Postgres,
    testcontainers::{ContainerAsync, ImageExt, runners::AsyncRunner},
};

/// The pinned test image (`specs/postgres-vector-image.txt`).
pub const TEST_IMAGE: &str = include_str!("../../../specs/postgres-vector-image.txt");
const PASSWORD: &str = "postgres";

/// One container with the service owner, the runtime roles and database `lctx`. The container
/// stops when this value is dropped.
pub struct DisposableDatabase {
    _container: ContainerAsync<Postgres>,
    port: u16,
    /// The container superuser on `lctx`, for controls that inspect or tamper with the catalog.
    pub superuser: PgPool,
    /// The verified service owner (`lctx_migrator`).
    pub owner: OwnerPool,
    /// `lctx_importer`, the generation writer.
    pub writer: PgPool,
    /// `lctx_serving`, the generation reader.
    pub reader: PgPool,
    /// `lctx_app`, the retained services' application role.
    pub app: PgPool,
}
impl DisposableDatabase {
    /// Start the pinned image and bootstrap it. Panics when Docker or the image is unavailable:
    /// callers are tests, and a missing prerequisite is reported by the test runner.
    pub async fn start() -> Self {
        let (image, tag) = TEST_IMAGE.trim().split_once(':').expect("image:tag");
        let container = Postgres::default()
            .with_name(image)
            .with_tag(tag)
            // Generation retirement locks the full relation closure atomically.
            .with_cmd(["postgres", "-c", "max_locks_per_transaction=256"])
            .start()
            .await
            .expect("Docker and the pinned PostgreSQL 18 image are required");
        let port = container
            .get_host_port_ipv4(5432)
            .await
            .expect("mapped port");
        let url = |role: &str, database: &str| {
            format!("postgres://{role}:{PASSWORD}@127.0.0.1:{port}/{database}")
        };
        let bootstrap = PgPool::connect(&url("postgres", "postgres"))
            .await
            .expect("superuser");
        sqlx::raw_sql(sqlx::AssertSqlSafe(format!("CREATE ROLE lctx_migrator LOGIN PASSWORD '{PASSWORD}'; CREATE ROLE lctx_app LOGIN PASSWORD '{PASSWORD}';
            CREATE ROLE lctx_importer LOGIN PASSWORD '{PASSWORD}'; CREATE ROLE lctx_serving LOGIN PASSWORD '{PASSWORD}'")))
            .execute(&bootstrap).await.expect("roles");
        // CREATE DATABASE refuses the implicit transaction of a multi-statement query.
        sqlx::raw_sql("CREATE DATABASE lctx OWNER lctx_migrator")
            .execute(&bootstrap)
            .await
            .expect("database");
        bootstrap.close().await;
        let superuser = PgPool::connect(&url("postgres", "lctx"))
            .await
            .expect("superuser on lctx");
        sqlx::raw_sql("REVOKE ALL ON DATABASE lctx FROM PUBLIC; GRANT CONNECT ON DATABASE lctx TO lctx_app, lctx_importer, lctx_serving;
            GRANT TEMP ON DATABASE lctx TO lctx_importer; ALTER ROLE lctx_serving SET default_transaction_read_only = on")
            .execute(&superuser).await.expect("runtime grants");
        // The numerical cache migration uses the same externally provisioned extension as the
        // operator bootstrap. Installing it remains a superuser effect, never a runtime effect.
        sqlx::raw_sql(
            "CREATE SCHEMA lctx_ext; REVOKE ALL ON SCHEMA lctx_ext FROM PUBLIC;
            CREATE EXTENSION vector WITH SCHEMA lctx_ext VERSION '0.8.6';
            GRANT USAGE ON SCHEMA lctx_ext TO lctx_migrator,lctx_app,lctx_importer,lctx_serving",
        )
        .execute(&superuser)
        .await
        .expect("pinned vector extension");
        // The runtime pools carry production's session limits, so a control that would stall a
        // real store times out here too (store-lifecycle review F03).
        let limited = |role: &str| {
            url(role, "lctx")
                .parse::<sqlx::postgres::PgConnectOptions>()
                .expect("options")
                .options([
                    ("lock_timeout", "10s"),
                    ("statement_timeout", "300s"),
                    ("idle_in_transaction_session_timeout", "60s"),
                ])
        };
        let connect = |role: &'static str| PgPool::connect_lazy_with(limited(role));
        let owner = OwnerPool::verify(
            PgPool::connect_with(limited("lctx_migrator"))
                .await
                .expect("owner"),
        )
        .await
        .expect("verified owner");
        Self {
            _container: container,
            port,
            superuser,
            owner,
            writer: connect("lctx_importer"),
            reader: connect("lctx_serving"),
            app: connect("lctx_app"),
        }
    }
    /// Apply the service baseline as the owner, as provisioning does before a store install.
    pub async fn migrate(&self) {
        let config = crate::Config {
            application_url: self.url("lctx_app"),
            migration_url: self.url("lctx_migrator"),
            migration_config: None,
            max_connections: 2,
            acquire_timeout_seconds: 10,
            statement_timeout_seconds: 60,
            lock_timeout_seconds: 10,
            max_receipt_bytes: 1 << 20,
        };
        let migrator = config.connect_migrator().await.expect("migrator");
        migrator.migrate().await.expect("service baseline");
        migrator.close().await;
    }
    /// A connection URL for a role on `lctx`.
    pub fn url(&self, role: &str) -> String {
        format!("postgres://{role}:{PASSWORD}@127.0.0.1:{}/lctx", self.port)
    }
    /// Write mode-0600 protected configurations into `dir`: `postgres.json` (application),
    /// `postgres-admin.json` (service owner) and one role file each for the writer and reader.
    pub fn write_configs(&self, dir: &Path) -> std::io::Result<()> {
        use std::io::Write;
        let write = |name: &str, value: serde_json::Value| -> std::io::Result<()> {
            let mut options = std::fs::OpenOptions::new();
            options.write(true).create_new(true);
            #[cfg(unix)]
            {
                use std::os::unix::fs::OpenOptionsExt;
                options.mode(0o600);
            }
            options
                .open(dir.join(name))?
                .write_all(value.to_string().as_bytes())
        };
        write(
            "postgres.json",
            serde_json::json!({ "application_url": self.url("lctx_app"), "max_connections": 4,
            "acquire_timeout_seconds": 10, "statement_timeout_seconds": 60, "lock_timeout_seconds": 10, "max_receipt_bytes": 1_048_576 }),
        )?;
        write(
            "postgres-admin.json",
            serde_json::json!({ "migration_url": self.url("lctx_migrator") }),
        )?;
        for (name, role, connections, provider) in [
            ("postgres-importer.json", "importer", 10, 8),
            ("postgres-serving.json", "serving", 3, 2),
        ] {
            write(
                name,
                serde_json::json!({ "format": 1, "role": role, "url": self.url(&format!("lctx_{role}")), "max_connections": connections,
                "provider_connections": provider, "acquire_timeout_seconds": 10, "statement_timeout_seconds": 60, "lock_timeout_seconds": 10 }),
            )?;
        }
        Ok(())
    }
}

/// Open a transaction holding the installation lock shared, as every lifecycle step, copy and
/// pin does first: a stand-in for a step in flight.
pub async fn step_in_flight(pool: &sqlx::PgPool) -> sqlx::Transaction<'static, sqlx::Postgres> {
    let mut tx = pool.begin().await.expect("transaction");
    sqlx::query(crate::generations::locks::INSTALLATION_SHARED)
        .execute(&mut *tx)
        .await
        .expect("installation lock");
    tx
}

/// A harness attempt driven one step at a time (feature `testing`). Every step keeps attempt
/// semantics: the generation is lock-owned, and a refused step records it `failed` and ends the
/// harness, after which every later step refuses with `State`.
/// `begin` is unscheduled and qualifies schema and ordinary model invariants, not stage admission
/// or source-sensitive publication. `begin_empty_conformance` uses a real schedule and empty
/// vocabulary receipts for full-model lifecycle controls; it does not qualify source producers.
/// Use `GenerationStore::begin_conformance` with actual source stages for those contracts.
pub struct Harness {
    store: crate::generations::GenerationStore,
    generation: crate::generations::GenerationId,
    state: Option<HarnessState>,
}
#[allow(
    clippy::large_enum_variant,
    reason = "One harness owns one linear lifecycle capability; boxing does not bound any retained collection"
)]
enum HarnessState {
    Staging(crate::generations::GenerationAttempt),
    ScheduledStaging(
        crate::generations::GenerationAttempt,
        lctx_model::domain::stages::ExecutionReceipt,
    ),
    Sealed(crate::generations::SealedAttempt),
    Validated(crate::generations::ValidatedAttempt),
}
impl Harness {
    /// Full-model lifecycle control with real, empty immutable vocabulary receipts.
    /// This fixture has no source frames, analysis results or embedding service effects.
    pub async fn begin_empty_conformance(
        store: &crate::generations::GenerationStore,
        writer: sqlx::PgPool,
        profile: lctx_model::domain::stages::Profile,
        budget: lctx_model::domain::resources::ResourceBudget,
        packages: Vec<lctx_model::domain::input::Package>,
    ) -> Result<Self, crate::generations::Error> {
        use lctx_model::domain::{stages::*, *};
        macro_rules! vocabulary {
            ($apply:ident) => {
                $apply!(
                    value::Literal,
                    value::LiteralSet,
                    value::LiteralSetMember,
                    value::PlaceRoot,
                    value::PathSegment,
                    value::AccessPath,
                    value::Place,
                    value::Predicate,
                    conditions::EvaluationAtom,
                    conditions::ConditionNode,
                    conditions::Condition,
                    assertion::AssertionQualification
                )
            };
        }
        macro_rules! uses {
            ($($ty:ty),*) => { vec![$(RelationUse::of::<$ty>()),*] };
        }
        let mut required: std::collections::BTreeSet<_> = store
            .model()
            .invariants()
            .iter()
            .flat_map(|check| &check.inputs)
            .filter_map(ValidationInput::prefix)
            .collect();
        required.extend(
            store
                .model()
                .publication_checks()
                .iter()
                .flat_map(|check| &check.inputs)
                .filter_map(ValidationInput::prefix),
        );
        required.insert(PublicationBoundary::Facts);
        let names = [
            "empty_facts",
            "empty_dispatch",
            "empty_base_semantic",
            "empty_execution_model",
            "empty_summary",
            "empty_catalog_synthesis",
            "empty_local",
            "empty_base_evaluation",
            "empty_base_completion",
            "empty_source_call",
            "empty_enriched_execution",
            "empty_model",
            "empty_structural",
            "empty_analytic",
            "empty_catalog_core",
            "empty_catalog_evidence",
            "empty_selection",
            "empty_synthesis",
            "empty_retrieval",
            "empty_analytic_embedding",
        ];
        // ALL is deterministic fixture ordering only: every prefix is empty, so no semantic
        // cross-boundary data exists. Domain codes do not order production execution.
        let groups: Vec<_> = PublicationBoundary::ALL
            .into_iter()
            .zip(names)
            .filter(|(boundary, _)| required.contains(boundary))
            .collect();
        let stage = |name, inputs, outputs| Stage {
            name,
            inputs,
            outputs,
            contributes: vec![],
            coverage: vec![],
            provider: None,
            profiles: vec![profile],
            effect: Effect::Pure,
            code: ContentHash::of(b"empty-full-model-conformance/v1"),
            configuration: ContentHash::of(b"inactive-canonical-text-and-retrieval"),
        };
        let selected = empty_conformance_configuration(&budget)?;
        let mut stages = Vec::new();
        let mut previous = None;
        for (boundary, name) in &groups {
            let inputs = previous.map_or_else(Vec::new, |prefix| {
                vocabulary!(uses)
                    .into_iter()
                    .map(|r| r.completed_store().at_epoch(prefix))
                    .collect()
            });
            stages.push(stage(*name, inputs, vocabulary!(uses)));
            previous = Some(*boundary);
        }
        let mut configured_outputs: Vec<_> = analysis::preparation::configuration_relations()
            .iter()
            .map(RelationUse::of_relation)
            .collect();
        configured_outputs.extend([
            RelationUse::of::<input::Package>(),
            RelationUse::of::<embedding::text::TextDefinition>(),
        ]);
        stages.push(stage("empty_configuration", vec![], configured_outputs));
        let schedule = Schedule::build_with_publications(
            store.model(),
            stages,
            &[],
            profile,
            groups
                .iter()
                .map(|(boundary, name)| PublicationGroup::new(*boundary, vec![*name]))
                .collect(),
        )?;
        let mut execution = schedule.execute();
        let attempt = store
            .begin_conformance(writer, &mut execution, budget.clone())
            .await?;
        for (_, name) in &groups {
            let mut output = StageOutput::new(
                execution.begin(name)?,
                &attempt,
                store.model(),
                budget.clone(),
                Default::default(),
            )?;
            macro_rules! declare {
                ($($ty:ty),*) => { $(output.declare::<$ty>()?;)* };
            }
            vocabulary!(declare);
            output.finish(ProviderOutcome::Complete).await?;
        }
        let mut output = StageOutput::new(
            execution.begin("empty_configuration")?,
            &attempt,
            store.model(),
            budget.clone(),
            Default::default(),
        )?;
        output.declare::<input::Package>()?;
        output.declare::<embedding::text::TextDefinition>()?;
        for package in packages {
            output.push(package).await?;
        }
        output
            .push(embedding::text::TextDefinition::builtin())
            .await?;
        macro_rules! configuration {
            ($ty:ty, $rows:expr) => {
                output.declare::<$ty>()?;
                for row in $rows.iter() {
                    output.push(row.clone()).await?;
                }
            };
        }
        configuration!(models::ModelCatalog, selected.catalogs().catalogs);
        configuration!(models::AuthoredTarget, selected.catalogs().targets);
        configuration!(models::AuthoredModel, selected.catalogs().models);
        configuration!(
            models::AuthoredContextProtocol,
            selected.catalogs().protocols
        );
        configuration!(analysis::MethodParameters, selected.parameters());
        configuration!(analysis::AnalysisDefinition, selected.definitions());
        configuration!(analysis::ProjectionDefinition, selected.projections());
        configuration!(
            analysis::settings::AnalyticsConfiguration,
            selected.analytics()
        );
        configuration!(retrieval::Definition, selected.retrieval());
        output.finish(ProviderOutcome::Complete).await?;
        let receipt = execution.finish()?;
        Ok(Self {
            store: store.clone(),
            generation: attempt.generation(),
            state: Some(HarnessState::ScheduledStaging(attempt, receipt)),
        })
    }
    pub async fn begin(
        store: &crate::generations::GenerationStore,
        writer: sqlx::PgPool,
        profile: lctx_model::domain::stages::Profile,
        budget: lctx_model::domain::resources::ResourceBudget,
    ) -> Result<Self, crate::generations::Error> {
        let attempt = store.begin_harness(writer, profile, budget).await?;
        Ok(Self {
            store: store.clone(),
            generation: attempt.generation(),
            state: Some(HarnessState::Staging(attempt)),
        })
    }
    pub fn generation(&self) -> crate::generations::GenerationId {
        self.generation
    }
    pub async fn copy<R: lctx_model::domain::Record>(
        &self,
        batch: &lctx_model::domain::Batch<R>,
        budget: &lctx_model::domain::resources::ResourceBudget,
    ) -> Result<(), crate::generations::Error> {
        match &self.state {
            Some(HarnessState::Staging(attempt)) => attempt.put(batch, budget).await,
            _ => Err(crate::generations::Error::State),
        }
    }
    pub async fn seal(&mut self) -> Result<(), crate::generations::Error> {
        match self.state.take() {
            Some(HarnessState::ScheduledStaging(attempt, receipt)) => {
                self.state = Some(HarnessState::Sealed(attempt.seal(receipt).await?));
                Ok(())
            }
            Some(HarnessState::Staging(attempt)) => {
                self.state = Some(HarnessState::Sealed(attempt.seal_harness().await?));
                Ok(())
            }
            other => {
                self.state = other;
                Err(crate::generations::Error::State)
            }
        }
    }
    /// Validate charged to `budget`; a refusal records the generation failed.
    pub async fn validate(
        &mut self,
        budget: &lctx_model::domain::resources::ResourceBudget,
    ) -> Result<lctx_model::domain::ContentHash, crate::generations::Error> {
        match self.state.take() {
            Some(HarnessState::Sealed(sealed)) => {
                let validated = sealed.validate_with(budget).await?;
                let content = validated.content();
                self.state = Some(HarnessState::Validated(validated));
                Ok(content)
            }
            other => {
                self.state = other;
                Err(crate::generations::Error::State)
            }
        }
    }
    pub async fn publish(&mut self) -> Result<(), crate::generations::Error> {
        match self.state.take() {
            Some(HarnessState::Validated(validated)) => validated.publish().await.map(drop),
            other => {
                self.state = other;
                Err(crate::generations::Error::State)
            }
        }
    }
    /// End the attempt without a step (its lock is released); the generation can then be aborted.
    pub async fn fail(
        &mut self,
        cause: &lctx_model::domain::ModelError,
    ) -> Result<(), crate::generations::Error> {
        match self.state.take() {
            Some(HarnessState::Staging(a) | HarnessState::ScheduledStaging(a, _)) => {
                a.fail(cause).await.map(drop)
            }
            Some(HarnessState::Sealed(s)) => s.fail(cause).await.map(drop),
            Some(HarnessState::Validated(v)) => v.fail(cause).await.map(drop),
            None => Err(crate::generations::Error::State),
        }
    }
    /// Remove the generation: through its own attempt while it lives, otherwise (failed or
    /// interrupted) through the store.
    pub async fn abort(
        &mut self,
    ) -> Result<crate::generations::CleanupOutcome, crate::generations::Error> {
        use crate::generations::CleanupOutcome::Removed;
        match self.state.take() {
            Some(HarnessState::Staging(a) | HarnessState::ScheduledStaging(a, _)) => {
                a.abort().await.map(|_| Removed)
            }
            Some(HarnessState::Sealed(s)) => s.abort().await.map(|_| Removed),
            Some(HarnessState::Validated(v)) => v.abort().await.map(|_| Removed),
            None => self.store.abort(self.generation).await,
        }
    }
}

fn empty_conformance_configuration(
    budget: &lctx_model::domain::resources::ResourceBudget,
) -> Result<lctx_model::domain::analysis::preparation::Configuration, lctx_model::domain::ModelError>
{
    use lctx_model::domain::*;
    let catalog = models::Catalog::parse("empty-conformance", "version = 7\nmodels = []\n")
        .map_err(ModelError::Invalid)?;
    let settings = analysis::settings::AnalyticsConfiguration::parse(
        "version = 1\n[subsystem]\nmodule_prefixes = ['fixture']\npublic_roots = ['fixture']\n\
         [seeds]\nprimary = []\ndistractors = []\n\
         [pass_a]\nmax_depth = 1\nmax_vertices = 1\nmax_edges = 0\nmax_witnesses = 1\n\
         [briefs]\nbudget = 0\n",
        analysis::settings::Techniques::default(),
    )?;
    analysis::preparation::Configuration::new(
        &catalog,
        [
            catalog::build::definition(),
            catalog::evidence::build::definition(),
            selection::build::definition(),
            synthesis::build::definition(),
            execution::configuration::summaries(catalog.declaration().id(), Default::default())?,
        ],
        budget,
    )?
    .with_analytics(settings)?
    .with_retrieval(retrieval::Definition::builtin(false))
}

/// Shared lifecycle fixtures for tests of this and dependent crates: a two-relation model for
/// fast lifecycle controls, and the facts frontier over the full model (the D1 stage table in
/// the catalog profile, one empty input, the Artifacts and Deployment coverage it requires).
pub mod fixtures {
    use super::DisposableDatabase;
    use crate::generations::{GenerationAttempt, GenerationId, GenerationStore};
    use lctx_model::domain::{
        admission::*, attribution::*, calls::*, declarations::*, deployment::*, documents::*,
        input::*, lexical::*, resources::ResourceBudget, source::*, stages::*, symbols::*,
        syntax::*, types::*, *,
    };
    use sqlx::PgPool;
    use std::sync::Arc;

    pub fn budget() -> ResourceBudget {
        ResourceBudget::fixed(DEFAULT_BUDGET).unwrap()
    }
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
        sqlx::query_scalar(
            "SELECT state FROM lctx_model_store.generations WHERE id = decode($1, 'hex')",
        )
        .bind(g.hex())
        .fetch_optional(&db.superuser)
        .await
        .unwrap()
    }
    pub async fn count(db: &DisposableDatabase, sql: &str, g: GenerationId) -> i64 {
        sqlx::query_scalar(sqlx::AssertSqlSafe(sql.to_owned()))
            .bind(g.hex())
            .fetch_one(&db.superuser)
            .await
            .unwrap()
    }

    /// A two-relation model: releases reference packages, so a dangling release fails validation.
    pub struct Small {
        pub model: Arc<ValidatedModel>,
        pub schedule: Schedule,
    }
    impl Small {
        pub fn new() -> Self {
            let model = Arc::new(
                ValidatedModel::validate(vec![
                    Relation::of::<Package>(),
                    Relation::of::<Release>(),
                ])
                .unwrap(),
            );
            let schedule = Schedule::build(
                &model,
                vec![Stage {
                    name: "packages",
                    inputs: vec![],
                    outputs: vec![RelationUse::of::<Package>(), RelationUse::of::<Release>()],
                    contributes: vec![],
                    coverage: vec![],
                    provider: None,
                    profiles: vec![Profile::Catalog],
                    effect: Effect::Extraction,
                    code: ContentHash::of(b"packages"),
                    configuration: ContentHash::of(b"cfg"),
                }],
                &[],
                Profile::Catalog,
            )
            .unwrap();
            Self { model, schedule }
        }
        /// A begun attempt with its stage written and its execution finished.
        pub async fn written(
            &self,
            store: &GenerationStore,
            writer: PgPool,
            packages: &[&str],
            releases: Vec<Release>,
        ) -> (GenerationAttempt, ExecutionReceipt) {
            let mut execution = self.schedule.execute();
            let attempt = store
                .begin_conformance(writer, &mut execution, budget())
                .await
                .unwrap();
            let mut access = execution.begin("packages").unwrap();
            rows!(access, attempt, &self.model; Package => packages.iter().map(|name| Package { name: (*name).into() }).collect(), Release => releases);
            access
                .complete(&attempt, ProviderOutcome::Complete)
                .await
                .unwrap();
            (attempt, execution.finish().unwrap())
        }
        /// A published harness generation holding one package.
        pub async fn published(
            &self,
            store: &GenerationStore,
            writer: &PgPool,
            name: &str,
        ) -> GenerationId {
            let mut harness =
                super::Harness::begin(store, writer.clone(), Profile::Catalog, budget())
                    .await
                    .unwrap();
            harness
                .copy(
                    &Batch::new(&self.model, vec![Package { name: name.into() }], &budget())
                        .unwrap(),
                    &budget(),
                )
                .await
                .unwrap();
            harness.seal().await.unwrap();
            harness.validate(&budget()).await.unwrap();
            harness.publish().await.unwrap();
            harness.generation()
        }
    }

    impl Default for Small {
        fn default() -> Self {
            Self::new()
        }
    }

    /// The facts frontier over the full model: one empty input, the D1 stage table in the catalog
    /// profile, and the two input-grained coverage rows (Artifacts, Deployment) it requires.
    pub struct Facts {
        pub model: Arc<ValidatedModel>,
        pub input: InputRevision,
        pub context: AnalysisContext,
        pub capture: Provider,
        pub pyrefly: Provider,
        pub docs: Provider,
        pub deploy: Provider,
    }
    impl Facts {
        pub fn new() -> Self {
            let input = InputRevision::from_entries(vec![]).unwrap();
            let context = AnalysisContext {
                python_version: "3.14.7".into(),
                python_platform: "linux".into(),
                search_path: vec![],
                site_package_path: vec![],
                config_digest: ContentHash::of(b"lifecycle"),
                environment_digest: input.manifest,
                lock_digest: None,
            };
            let provider = |tool: &str| Provider {
                tool: tool.into(),
                revision: "pinned".into(),
                build_digest: ContentHash::of(tool.as_bytes()),
            };
            Self {
                model: Arc::new(model().unwrap()),
                input,
                context,
                capture: provider("capture"),
                pyrefly: provider("pyrefly"),
                docs: provider("documents"),
                deploy: provider("deployment"),
            }
        }
        pub fn schedule(&self) -> Schedule {
            let stage =
                |name, outputs, coverage: Vec<FactFamily>, provider: Option<&Provider>| Stage {
                    name,
                    inputs: vec![],
                    outputs,
                    contributes: vec![],
                    coverage,
                    provider: provider.map(Record::id),
                    profiles: vec![Profile::Catalog],
                    effect: Effect::Extraction,
                    code: ContentHash::of(name.as_bytes()),
                    configuration: ContentHash::of(b"cfg"),
                };
            macro_rules! uses { ($($ty:ty),+) => { vec![$(RelationUse::of::<$ty>()),+] }; }
            use FactFamily::*;
            Schedule::build(
                &self.model,
                vec![
                    stage(
                        "acquire",
                        uses!(InputRevision, SourceArtifact),
                        vec![Artifacts],
                        Some(&self.capture),
                    ),
                    stage(
                        "pyrefly",
                        uses!(
                            BindingObservation,
                            BindingSupport,
                            CallResolution,
                            CallResolutionSupport,
                            CallSyntax,
                            CallSyntaxSupport,
                            CallTarget,
                            CallTargetSupport,
                            ClassAncestryObservation,
                            ClassAncestrySupport,
                            ClassFieldSyntaxObservation,
                            ClassFieldSyntaxSupport,
                            ClassTraitObservation,
                            ClassTraitSupport,
                            DeclarationDecorator,
                            DeclarationDecoratorSupport,
                            DeclarationObservation,
                            DeclarationSupport,
                            DependencyModuleObservation,
                            DependencyModuleSupport,
                            DunderAllObservation,
                            DunderAllSupport,
                            FunctionBodyObservation,
                            FunctionBodySupport,
                            FunctionTraitObservation,
                            FunctionTraitSupport,
                            ImportAliasObservation,
                            ImportAliasSupport,
                            LexicalResolution,
                            LexicalResolutionSupport,
                            LexicalScopeObservation,
                            LexicalScopeSupport,
                            ParameterAnnotationObservation,
                            ParameterAnnotationSupport,
                            ParameterDeclaration,
                            ParameterDeclarationSupport,
                            ParameterDocObservation,
                            ParameterDocSupport,
                            ParameterSyntaxObservation,
                            ParameterSyntaxSupport,
                            ProviderCallSite,
                            ProviderCallSiteSupport,
                            PublicNameObservation,
                            PublicNameSupport,
                            RecordFieldObservation,
                            RecordFieldSupport,
                            ReferenceObservation,
                            ReferenceSupport,
                            Signature,
                            SignatureSupport,
                            SignatureEnumerationObservation,
                            SignatureEnumerationMember,
                            SignatureEnumerationSupport,
                            SymbolDeclaration,
                            SymbolDeclarationSupport,
                            SymbolObservation,
                            SymbolSupport,
                            SyntaxDetailObservation,
                            SyntaxDetailSupport,
                            SyntaxObservation,
                            SyntaxPlacement,
                            SyntaxPlacementSupport,
                            SyntaxSupport,
                            TypeObservation,
                            TypePresentation,
                            TypePresentationSupport,
                            TypeRestrictionSupport,
                            TypeSupport,
                            TypeVariableRestriction
                        ),
                        vec![Syntax, Lexical, Signatures, Calls, Types, Exports],
                        Some(&self.pyrefly),
                    ),
                    stage(
                        "documents",
                        uses!(
                            DocumentObservation,
                            DocumentSupport,
                            PassageObservation,
                            PassageSupport,
                            CodeBlockObservation,
                            CodeBlockSupport,
                            DocumentLinkObservation,
                            DocumentLinkSupport,
                            DocumentMentionObservation,
                            DocumentMentionSupport,
                            DocumentComponentObservation,
                            DocumentComponentSupport,
                            DocumentAttributeObservation,
                            DocumentAttributeSupport
                        ),
                        vec![Docs],
                        Some(&self.docs),
                    ),
                    stage(
                        "deployment",
                        uses!(
                            TaskReportObservation,
                            TaskReportSupport,
                            DeploymentObservation,
                            DeploymentSupport
                        ),
                        vec![Deployment],
                        Some(&self.deploy),
                    ),
                    stage(
                        "assemble",
                        uses!(
                            ProviderCoverage,
                            CoverageScope,
                            Provider,
                            AnalysisContext,
                            ProviderRun,
                            RunFamily
                        ),
                        vec![],
                        None,
                    ),
                ],
                &[],
                Profile::Catalog,
            )
            .unwrap()
        }
        pub fn contract(&self) -> FrontierContract {
            FrontierContract::facts(&self.model, Profile::Catalog).unwrap()
        }
        /// Run every stage; `deployment` states the Deployment coverage row the frontier requires.
        pub async fn written(
            &self,
            store: &GenerationStore,
            writer: PgPool,
            deployment: bool,
        ) -> (GenerationAttempt, ExecutionReceipt) {
            let schedule = self.schedule();
            let mut execution = schedule.execute();
            let attempt = store
                .begin(writer, &mut execution, &self.contract(), budget())
                .await
                .unwrap();
            let model = &*self.model;
            let scope = CoverageScope::Input {
                input: self.input.id(),
            };
            let (capture_run, capture_families) = ProviderRun::new(
                self.capture.id(),
                self.context.id(),
                self.input.id(),
                self.context.config_digest,
                [FactFamily::Artifacts],
            )
            .unwrap();
            let (signature_run, signature_families) = ProviderRun::new(
                self.pyrefly.id(),
                self.context.id(),
                self.input.id(),
                self.context.config_digest,
                [FactFamily::Signatures],
            )
            .unwrap();
            let (deploy_run, deploy_families) = ProviderRun::new(
                self.deploy.id(),
                self.context.id(),
                self.input.id(),
                self.context.config_digest,
                [FactFamily::Deployment],
            )
            .unwrap();
            let row = |provider: &Provider, run: &ProviderRun, family| ProviderCoverage {
                scope: scope.id(),
                provider: Some(provider.id()),
                context: self.context.id(),
                family,
                run: Some(run.id()),
                status: CoverageStatus::CompleteUnderStatedModel,
                reason: None,
                diagnostic: None,
            };
            let mut coverage = vec![
                row(&self.capture, &capture_run, FactFamily::Artifacts),
                row(&self.pyrefly, &signature_run, FactFamily::Signatures),
            ];
            if deployment {
                coverage.push(row(&self.deploy, &deploy_run, FactFamily::Deployment));
            }
            for stage in schedule.stages() {
                let mut access = execution.begin(stage.name).unwrap();
                match stage.name {
                    "acquire" => {
                        rows!(access, attempt, model; InputRevision => vec![self.input.clone()], SourceArtifact => vec![])
                    }
                    "pyrefly" => {
                        empty!(access, attempt, model; BindingObservation,BindingSupport,CallResolution,CallResolutionSupport,CallSyntax,CallSyntaxSupport,CallTarget,CallTargetSupport,ClassAncestryObservation,ClassAncestrySupport,ClassFieldSyntaxObservation,ClassFieldSyntaxSupport,ClassTraitObservation,ClassTraitSupport,DeclarationDecorator,DeclarationDecoratorSupport,DeclarationObservation,DeclarationSupport,DependencyModuleObservation,DependencyModuleSupport,DunderAllObservation,DunderAllSupport,FunctionBodyObservation,FunctionBodySupport,FunctionTraitObservation,FunctionTraitSupport,ImportAliasObservation,ImportAliasSupport,LexicalResolution,LexicalResolutionSupport,LexicalScopeObservation,LexicalScopeSupport,ParameterAnnotationObservation,ParameterAnnotationSupport,ParameterDeclaration,ParameterDeclarationSupport,ParameterDocObservation,ParameterDocSupport,ParameterSyntaxObservation,ParameterSyntaxSupport,ProviderCallSite,ProviderCallSiteSupport,PublicNameObservation,PublicNameSupport,RecordFieldObservation,RecordFieldSupport,ReferenceObservation,ReferenceSupport,Signature,SignatureSupport,SignatureEnumerationObservation,SignatureEnumerationMember,SignatureEnumerationSupport,SymbolDeclaration,SymbolDeclarationSupport,SymbolObservation,SymbolSupport,SyntaxDetailObservation,SyntaxDetailSupport,SyntaxObservation,SyntaxPlacement,SyntaxPlacementSupport,SyntaxSupport,TypeObservation,TypePresentation,TypePresentationSupport,TypeRestrictionSupport,TypeSupport,TypeVariableRestriction)
                    }
                    "documents" => {
                        empty!(access, attempt, model; DocumentObservation, DocumentSupport, PassageObservation, PassageSupport, CodeBlockObservation,
                        CodeBlockSupport, DocumentLinkObservation, DocumentLinkSupport, DocumentMentionObservation, DocumentMentionSupport, DocumentComponentObservation,
                        DocumentComponentSupport, DocumentAttributeObservation, DocumentAttributeSupport)
                    }
                    "deployment" => {
                        empty!(access, attempt, model; TaskReportObservation, TaskReportSupport, DeploymentObservation, DeploymentSupport)
                    }
                    "assemble" => {
                        rows!(access, attempt, model; ProviderCoverage => coverage.clone(), CoverageScope => vec![scope.clone()],
                        Provider => vec![self.capture.clone(), self.pyrefly.clone(),self.deploy.clone()], AnalysisContext => vec![self.context.clone()],
                        ProviderRun => vec![capture_run.clone(),signature_run.clone(), deploy_run.clone()], RunFamily => capture_families.iter().chain(&signature_families).chain(&deploy_families).cloned().collect())
                    }
                    other => panic!("unscheduled stage {other}"),
                }
                access
                    .complete(&attempt, ProviderOutcome::Complete)
                    .await
                    .unwrap();
            }
            (attempt, execution.finish().unwrap())
        }
        pub async fn published(&self, store: &GenerationStore, writer: PgPool) -> GenerationId {
            let (attempt, receipt) = self.written(store, writer, true).await;
            attempt
                .seal(receipt)
                .await
                .unwrap()
                .validate()
                .await
                .unwrap()
                .publish()
                .await
                .unwrap()
        }
    }

    impl Default for Facts {
        fn default() -> Self {
            Self::new()
        }
    }
}
