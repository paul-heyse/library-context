//! Permanent typed lowerings against a disposable real PostgreSQL server.
use std::sync::Arc;
use lctx_model::{Domain, domain::{Batch, ContentHash, Id, Record, Relation, ValidatedModel, source::*}};
use lctx_postgres::generations::{Error, GenerationStore};
use sqlx::PgPool;
use testcontainers_modules::{postgres::Postgres, testcontainers::{ImageExt, runners::AsyncRunner}};

#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(name = "recursive_nodes", semantic_source = include_bytes!("generations.rs"))]
struct RecursiveNode { #[model(key)] name: String, parent: Option<Id<RecursiveNode>> }
#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(name = "module_scope_links", semantic_source = include_bytes!("generations.rs"))]
struct ModuleScopeLink { #[model(key)] scope: CoverageScopeModuleId }

#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(name = "binary_evidence", semantic_source = include_bytes!("generations.rs"))]
struct BinaryEvidence {
    #[model(key)] name: String,
    body: lctx_model::domain::EvidenceBytes,
}

#[tokio::test]
async fn immutable_generation_vertical_slice_and_lifecycle_refusals() {
    #[derive(Debug, Clone, PartialEq, Eq, Domain)]
    #[model(name = "external_unversioned")]
    struct ExternalUnversioned { #[model(key)] name: String }
    assert!(ValidatedModel::validate(vec![Relation::of::<ExternalUnversioned>()]).is_err());
    let (image, tag) = lctx_postgres::serving::TEST_IMAGE.trim().split_once(':').unwrap();
    let container = Postgres::default().with_name(image).with_tag(tag).start().await.expect("Docker and pinned PG18 image required");
    let port = container.get_host_port_ipv4(5432).await.unwrap();
    let url = |role: &str| format!("postgres://{role}:postgres@127.0.0.1:{port}/postgres");
    let owner = PgPool::connect(&url("postgres")).await.unwrap();
    sqlx::raw_sql("CREATE ROLE lctx_importer LOGIN PASSWORD 'postgres'; CREATE ROLE lctx_serving LOGIN PASSWORD 'postgres'").execute(&owner).await.unwrap();
    let writer = PgPool::connect(&url("lctx_importer")).await.unwrap();
    let reader = sqlx::postgres::PgPoolOptions::new().max_connections(1).acquire_timeout(std::time::Duration::from_millis(150)).connect(&url("lctx_serving")).await.unwrap();
    let mut relations = model().unwrap().relations().to_vec(); relations.push(Relation::of::<RecursiveNode>()); relations.push(Relation::of::<ModuleScopeLink>()); relations.push(Relation::of::<BinaryEvidence>());
    let model = Arc::new(ValidatedModel::validate(relations).unwrap());
    let store = GenerationStore::install(owner.clone(), model.clone()).await.unwrap();
    let producer_digest = ContentHash::of(b"producer-code");
    let g = store.create(producer_digest, "catalog").await.unwrap();
    assert!(matches!(store.pin(&reader, g).await, Err(Error::State)));
    assert!(matches!(store.publish(g).await, Err(Error::State)));
    let package = Package { name: "example".into() };
    let release = Release { package: package.id(), version: "1.0".into(), lock_digest: ContentHash::of(b"lock") };
    let source = SourceArtifact { release: release.id(), path: "example.py".into(), content: ContentHash::of(b"x = 1") };
    let module = Module { source: source.id(), qualified_name: "example".into() };
    let occurrence = Occurrence { source: source.id(), start: 0, end: 1, syntax_kind: SyntaxKind::ExprName, role: "assignment_target".into() };
    let provider = Provider { tool: "ruff".into(), revision: "0.0.11".into(), build_digest: producer_digest };
    let context = AnalysisContext { python_version: "3.14.7".into(), python_platform: "linux".into(), search_path: vec!["src".into()], site_package_path: vec![], config_digest: ContentHash::of(b"config"), environment_digest: ContentHash::of(b"env"), lock_digest: None };
    let run = ProviderRun { provider: provider.id(), context: context.id(), release: release.id(), configuration: context.config_digest };
    let assertion = SyntaxObservation { occurrence: occurrence.id(), spelling: "x".into() };
    let support = SyntaxSupport { assertion: assertion.id(), run: run.id(), surface: "syntax".into() };
    let scope = CoverageScope::Module { module: module.id() };
    let link = ModuleScopeLink { scope: CoverageScopeModuleId::of(&scope).unwrap() };
    let coverage = ProviderCoverage { scope: scope.id(), provider: provider.id(), context: context.id(), family: "syntax".into(), run: Some(run.id()), status: CoverageStatus::CompleteUnderStatedModel, reason: None };
    let mut node = RecursiveNode { name: "self".into(), parent: None }; node.parent = Some(node.id());
    macro_rules! copy { ($($row:expr),*) => { $(store.copy(&writer, g, &Batch::new(&model, vec![$row.clone()]).unwrap()).await.unwrap();)* }; }
    let binary = BinaryEvidence { name: "invalid-utf8".into(), body: lctx_model::domain::EvidenceBytes(vec![0, 255, 128]) };
    copy!(binary, package, release, source, module, occurrence, provider, context, run, assertion, support, scope, coverage, link, node);
    // A malformed tagged row is refused by generated PostgreSQL constraints before validation.
    let malformed = sqlx::query(sqlx::AssertSqlSafe(format!("INSERT INTO {}.coverage_scopes(id,kind,release_release,module_module) VALUES($1,1,$2,$3)", g.schema())))
        .bind(vec![0u8;16]).bind(release.id().bytes().to_vec()).bind(module.id().bytes().to_vec()).execute(&writer).await;
    assert!(malformed.is_err());
    // A direct writer transaction must drain before sealing, even outside the typed COPY API.
    let held = Package { name: "held-write".into() };
    let mut tx = writer.begin().await.unwrap();
    sqlx::query(sqlx::AssertSqlSafe(format!("INSERT INTO {}.packages(id,name) VALUES($1,$2)", g.schema()))).bind(held.id().bytes().to_vec()).bind(&held.name).execute(&mut *tx).await.unwrap();
    let seal_store = store.clone();
    let mut sealing = tokio::spawn(async move { seal_store.seal(g).await });
    assert!(tokio::time::timeout(std::time::Duration::from_millis(100), &mut sealing).await.is_err());
    tx.commit().await.unwrap(); sealing.await.unwrap().unwrap();
    assert!(store.copy(&writer, g, &Batch::new(&model, vec![held]).unwrap()).await.is_err());
    let late = sqlx::query(sqlx::AssertSqlSafe(format!("INSERT INTO {}.packages(id,name) VALUES($1,'late')",g.schema()))).bind(vec![0u8;16]).execute(&writer).await;
    assert!(late.is_err());
    let content = store.validate(g).await.unwrap();
    store.publish(g).await.unwrap();
    let mut lease = store.pin(&reader, g).await.unwrap();
    assert_eq!(lease.read::<BinaryEvidence>().await.unwrap().rows(), &[binary]);
    assert_eq!(lease.read::<SourceArtifact>().await.unwrap().rows(), &[source]);
    assert_eq!(lease.read::<AnalysisContext>().await.unwrap().rows(), &[context]);
    assert_eq!(lease.read::<SyntaxSupport>().await.unwrap().rows(), &[support]);
    assert_eq!(lease.read::<ModuleScopeLink>().await.unwrap().rows(), &[link]);
    assert_eq!(lease.read::<CoverageScope>().await.unwrap().rows(), &[scope]);
    assert_eq!(lease.read::<ProviderCoverage>().await.unwrap().rows(), &[coverage]);
    assert_eq!(lease.read::<RecursiveNode>().await.unwrap().rows(), &[node]);
    assert!(matches!(store.pin(&reader, g).await, Err(Error::Database(sqlx::Error::PoolTimedOut))));
    assert!(matches!(store.retire(g).await, Err(Error::Busy)));
    let next = store.create(producer_digest, "catalog").await.unwrap();
    store.seal(next).await.unwrap(); store.validate(next).await.unwrap(); store.publish(next).await.unwrap(); store.select(next).await.unwrap();
    assert_eq!(lease.read::<Module>().await.unwrap().rows(), &[module]);
    assert!(matches!(store.retire(next).await, Err(Error::Busy)));
    drop(lease);
    // Lease Drop closes the connection. Wait for the server to observe session termination.
    for attempt in 0..50 {
        match store.retire(g).await {
            Ok(()) => break,
            Err(Error::Busy) if attempt < 49 => tokio::task::yield_now().await,
            result => panic!("retirement failed: {result:?}"),
        }
    }
    assert!(matches!(store.pin(&reader, g).await, Err(Error::State)));
    assert_eq!(sqlx::query_scalar::<_,Vec<u8>>("SELECT content_digest FROM lctx_model_store.generations WHERE id=decode($1,'hex')").bind(g.hex()).fetch_one(&owner).await.unwrap(), content.0);
    // Missing reference is caught against sealed stored contents, not a caller's batch receipt.
    let bad = store.create(producer_digest, "catalog").await.unwrap();
    store.copy(&writer, bad, &Batch::new(&model, vec![release.clone()]).unwrap()).await.unwrap(); store.seal(bad).await.unwrap();
    assert!(store.validate(bad).await.is_err()); assert!(store.publish(bad).await.is_err());
    store.abort(bad).await.unwrap(); store.abort(bad).await.unwrap();
    assert!(matches!(store.pin(&reader, bad).await, Err(Error::State)));
    let abandoned = store.create(producer_digest, "catalog").await.unwrap();
    store.abort(abandoned).await.unwrap();
    assert!(store.seal(abandoned).await.is_err());
    let absent: bool = sqlx::query_scalar("SELECT NOT EXISTS(SELECT 1 FROM pg_namespace WHERE nspname=$1)").bind(abandoned.schema()).fetch_one(&owner).await.unwrap();
    assert!(absent);
    let wrong_subtype = store.create(producer_digest, "catalog").await.unwrap();
    let wrong_scope = CoverageScope::Release { release: release.id() };
    store.copy(&writer, wrong_subtype, &Batch::new(&model, vec![package]).unwrap()).await.unwrap();
    store.copy(&writer, wrong_subtype, &Batch::new(&model, vec![release]).unwrap()).await.unwrap();
    store.copy(&writer, wrong_subtype, &Batch::new(&model, vec![wrong_scope.clone()]).unwrap()).await.unwrap();
    // FK checking is deliberately deferred until every staging table is loaded.
    sqlx::query(sqlx::AssertSqlSafe(format!("INSERT INTO {}.module_scope_links(id,scope) VALUES($1,$2)",wrong_subtype.schema())))
        .bind(vec![0u8;16]).bind(wrong_scope.id().bytes().to_vec()).execute(&writer).await.unwrap();
    store.seal(wrong_subtype).await.unwrap();
    let error = store.validate(wrong_subtype).await.unwrap_err();
    assert!(matches!(error, Error::Database(ref e) if e.as_database_error().and_then(|e| e.code()).as_deref() == Some("23503")));
    store.abort(wrong_subtype).await.unwrap();
    // A reader refuses a model mismatch before interpreting any physical values.
    sqlx::query("UPDATE lctx_model_store.generations SET model_digest=$1 WHERE id=decode($2,'hex')").bind(vec![0u8;32]).bind(next.hex()).execute(&owner).await.unwrap();
    assert!(matches!(store.pin(&reader, next).await, Err(Error::Contract)));
    owner.close().await; writer.close().await; reader.close().await;
}
