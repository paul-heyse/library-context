//! Permanent typed lowerings against a disposable real PostgreSQL server.
use std::sync::Arc;
use lctx_model::{Domain, domain::{Batch, ContentHash, Id, Record, Relation, ValidatedModel, model, attribution::*, input::*, source::*, artifact::*}};
use lctx_postgres::generations::{Error, GenerationStore, GenerationId, CleanupOutcome};
use sqlx::PgPool;
use lctx_model::domain::{assertion::*, conditions::*};
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

async fn copy_artifact(store: &GenerationStore, writer: &PgPool, generation: GenerationId,
    model: &ValidatedModel, artifact: &SourceArtifact, body: &[u8]) {
    store.copy(writer, generation, &Batch::new(model, vec![artifact.clone()], &budget()).unwrap(), &budget()).await.unwrap();
    for chunk in ArtifactChunk::split(artifact, body).unwrap() {
        store.copy(writer, generation, &Batch::new(model, vec![chunk], &budget()).unwrap(), &budget()).await.unwrap();
    }
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
    let g = store.create_conformance(producer_digest, "catalog").await.unwrap();
    assert!(matches!(store.pin(&reader, g, budget()).await, Err(Error::State)));
    assert!(matches!(store.publish(g).await, Err(Error::State)));
    let package = Package { name: "example".into() };
    let release = Release { package: package.id(), version: "1.0".into() };
    let input = InputRevision::from_entries(vec![ManifestEntry { path: "example.py".into(), content: ContentHash::of(b"x = 1"), byte_len: 5 }, ManifestEntry { path: "aux.py".into(), content: ContentHash::of(b"y = 2"), byte_len: 5 }]).unwrap();
    let source = SourceArtifact::from_bytes(input.id(), "example.py".into(), b"x = 1").unwrap();
    let auxiliary = SourceArtifact::from_bytes(input.id(), "aux.py".into(), b"y = 2").unwrap();
    let other_package = Package { name: "auxiliary".into() };
    let other_release = Release { package: other_package.id(), version: "2.0".into() };
    let origin = InputOrigin::Installed { library: "example".into(), requirement: "example==1.0".into(), lock_digest: ContentHash::of(b"lock"), installer: Some("uv".into()) };
    let acquisition = InputAcquisition { input: input.id(), origin: origin.id() };
    let verified = DistributionVerification { acquisition: acquisition.id(), release: release.id(), record_digest: ContentHash::of(b"record-a"), artifact_sha256: vec!["ab".repeat(32)] };
    let distribution = InputDistribution { input: input.id(), release: release.id(), role: DistributionRole::FirstParty };
    let other_distribution = InputDistribution { input: input.id(), release: other_release.id(), role: DistributionRole::Dependency };
    let other_verified = DistributionVerification { acquisition: acquisition.id(), release: other_release.id(), record_digest: ContentHash::of(b"record-b"), artifact_sha256: vec!["cd".repeat(32)] };
    let ownership = ArtifactOwnership { artifact: source.id(), distribution: verified.id() };
    let other_ownership = ArtifactOwnership { artifact: auxiliary.id(), distribution: other_verified.id() };
    let example_use = ArtifactUse { artifact: source.id(), input: input.id(), role: SourceRole::Example };
    let test_use = ArtifactUse { artifact: source.id(), input: input.id(), role: SourceRole::Test };
    let module = Module { source: source.id(), qualified_name: "example".into() };
    let occurrence = Occurrence { source: source.id(), start: 0, end: 1, syntax_kind: SyntaxKind::ExprName, role: OccurrenceRole::Binding, structural_path: vec![0] };
    let provider = Provider { tool: "ruff".into(), revision: "0.0.11".into(), build_digest: producer_digest };
    let context = AnalysisContext { python_version: "3.14.7".into(), python_platform: "linux".into(), search_path: vec!["src".into()], site_package_path: vec![], config_digest: ContentHash::of(b"config"), environment_digest: ContentHash::of(b"env"), lock_digest: None };
    let (run, run_families) = ProviderRun::new(provider.id(), context.id(), input.id(), context.config_digest, [FactFamily::Syntax]).unwrap();
    let scope = CoverageScope::Module { module: module.id() };
    let (condition, nodes) = Diagram::always().records();
    let qualification = AssertionQualification { context: context.id(), scope: scope.id(), condition: condition.id(), modality: Modality::Definite, approximation: Approximation::Exact };
    let surface = ProviderSurface { provider: provider.id(), family: FactFamily::Syntax, name: "syntax".into() };
    let evidence = Evidence::Occurrence { occurrence: occurrence.id() };
    let assertion = SyntaxObservation { qualification: qualification.id(), occurrence: occurrence.id(), spelling: "x".into() };
    let support = SyntaxSupport { assertion: assertion.id(), run: run.id(), surface: surface.id(), evidence: evidence.id(), origin: Origin::SourceObservation, mode: ExtractionMode::NativeTraversal, fidelity: Fidelity::NativeStructural };
    let link = ModuleScopeLink { scope: CoverageScopeModuleId::of(&scope).unwrap() };
    let coverage = ProviderCoverage { scope: scope.id(), provider: provider.id(), context: context.id(), family: FactFamily::Syntax, run: Some(run.id()), status: CoverageStatus::CompleteUnderStatedModel, reason: None, diagnostic: None };
    let mut node = RecursiveNode { name: "self".into(), parent: None }; node.parent = Some(node.id());
    macro_rules! copy { ($($row:expr),*) => { $(store.copy(&writer, g, &Batch::new(&model, vec![$row.clone()], &budget()).unwrap(), &budget()).await.unwrap();)* }; }
    let binary = BinaryEvidence { name: "invalid-utf8".into(), body: lctx_model::domain::EvidenceBytes(vec![0, 255, 128]) };
    copy!(binary, other_package, other_release, origin, acquisition, distribution, other_distribution, verified, other_verified, ownership, other_ownership, example_use, test_use, package, release, input, module, occurrence, provider, context, run, condition, qualification, surface, evidence, assertion, support, scope, coverage, link, node);
    store.copy(&writer, g, &Batch::new(&model, nodes, &budget()).unwrap(), &budget()).await.unwrap();
    copy_artifact(&store, &writer, g, &model, &source, b"x = 1").await;
    copy_artifact(&store, &writer, g, &model, &auxiliary, b"y = 2").await;
    store.copy(&writer, g, &Batch::new(&model, run_families, &budget()).unwrap(), &budget()).await.unwrap();
    // The server refuses oversized payloads even when a caller bypasses typed COPY.
    let oversized = sqlx::query(sqlx::AssertSqlSafe(format!("INSERT INTO {}.binary_evidence(id,name,body) VALUES($1,'oversized',decode(repeat('ff',67108864),'hex'))", g.schema())))
        .bind(vec![0u8;16]).execute(&writer).await;
    assert!(matches!(oversized, Err(ref e) if e.as_database_error().and_then(|e| e.code()).as_deref() == Some("23514")));
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
    assert!(store.copy(&writer, g, &Batch::new(&model, vec![held], &budget()).unwrap(), &budget()).await.is_err());
    let late = sqlx::query(sqlx::AssertSqlSafe(format!("INSERT INTO {}.packages(id,name) VALUES($1,'late')",g.schema()))).bind(vec![0u8;16]).execute(&writer).await;
    assert!(late.is_err());
    let content = store.validate(g, &budget()).await.unwrap();
    store.publish(g).await.unwrap();
    let mut lease = store.pin(&reader, g, budget()).await.unwrap();
    assert_eq!(lease.read::<BinaryEvidence>().await.unwrap().rows(), &[binary]);
    assert_eq!(lease.read::<SourceArtifact>().await.unwrap().rows(), Batch::new(&model, vec![source, auxiliary.clone()], &budget()).unwrap().rows());
    assert_eq!(lease.read::<ArtifactOwnership>().await.unwrap().rows(), Batch::new(&model, vec![ownership, other_ownership], &budget()).unwrap().rows());
    assert_eq!(lease.read::<DistributionVerification>().await.unwrap().rows(), Batch::new(&model, vec![verified, other_verified], &budget()).unwrap().rows());
    assert_eq!(lease.read::<ArtifactUse>().await.unwrap().rows(), Batch::new(&model, vec![example_use, test_use], &budget()).unwrap().rows());
    assert_eq!(lease.read::<AnalysisContext>().await.unwrap().rows(), &[context.clone()]);
    assert_eq!(lease.read::<SyntaxSupport>().await.unwrap().rows(), &[support]);
    assert_eq!(lease.read::<ModuleScopeLink>().await.unwrap().rows(), &[link]);
    assert_eq!(lease.read::<CoverageScope>().await.unwrap().rows(), &[scope]);
    assert_eq!(lease.read::<ProviderCoverage>().await.unwrap().rows(), &[coverage]);
    assert_eq!(lease.read::<RecursiveNode>().await.unwrap().rows(), &[node]);
    assert!(matches!(store.pin(&reader, g, budget()).await, Err(Error::Database(sqlx::Error::PoolTimedOut))));
    assert!(matches!(store.retire(g).await, Err(Error::Busy)));
    let next = store.create_conformance(producer_digest, "catalog").await.unwrap();
    store.seal(next).await.unwrap(); store.validate(next, &budget()).await.unwrap(); store.publish(next).await.unwrap();
    assert!(matches!(store.select(next).await, Err(Error::Frontier)));
    let frontier: String = sqlx::query_scalar("SELECT frontier FROM lctx_model_store.generations WHERE id=decode($1,'hex')").bind(next.hex()).fetch_one(&owner).await.unwrap();
    assert_eq!(frontier,"conformance");
    // Force the pointer administratively only to exercise the retirement protection independently
    // of production frontier admission. This is not a successful facts-selection test.
    sqlx::query("UPDATE lctx_model_store.selection SET generation_id=decode($1,'hex') WHERE singleton").bind(next.hex()).execute(&owner).await.unwrap();
    assert_eq!(lease.read::<Module>().await.unwrap().rows(), &[module]);
    assert!(matches!(store.retire(next).await, Err(Error::Busy)));
    drop(lease);
    // Lease Drop closes the connection. Wait for the server to observe session termination.
    for attempt in 0..50 {
        match store.retire(g).await {
            Ok(CleanupOutcome::Removed) => break,
            Err(Error::Busy) if attempt < 49 => tokio::task::yield_now().await,
            result => panic!("retirement failed: {result:?}"),
        }
    }
    assert!(matches!(store.pin(&reader, g, budget()).await, Err(Error::State)));
    assert_ne!(content,ContentHash::of(b""));
    assert_eq!(store.retire(g).await.unwrap(),CleanupOutcome::AlreadyAbsent);
    for table in ["generations","receipts","validation_receipts","events"] {
        let column = if table == "generations" { "id" } else { "generation_id" };
        let count: i64 = sqlx::query_scalar(sqlx::AssertSqlSafe(format!("SELECT count(*) FROM lctx_model_store.{table} WHERE {column}=decode($1,'hex')"))).bind(g.hex()).fetch_one(&owner).await.unwrap();
        assert_eq!(count,0,"{table}");
    }
    // Cross-relation invariants execute over sealed contents, including otherwise valid rows.
    for case in ["changed", "missing", "extra", "duplicate", "span"] {
        let attempt = store.create_conformance(producer_digest, "catalog").await.unwrap();
        let original = SourceArtifact::from_bytes(input.id(), "example.py".into(), b"x = 1").unwrap();
        store.copy(&writer, attempt, &Batch::new(&model, vec![input.clone()], &budget()).unwrap(), &budget()).await.unwrap();
        copy_artifact(&store, &writer, attempt, &model, &auxiliary, b"y = 2").await;
        if case != "missing" {
            let artifact = if case == "changed" { SourceArtifact::from_bytes(input.id(), "example.py".into(), b"x = 2").unwrap() } else { original.clone() };
            copy_artifact(&store, &writer, attempt, &model, &artifact, if case == "changed" { b"x = 2" } else { b"x = 1" }).await;
        }
        if matches!(case, "extra" | "duplicate") {
            let artifact = SourceArtifact::from_bytes(input.id(), if case == "extra" { "extra.py" } else { "example.py" }.into(), b"y = 2").unwrap();
            copy_artifact(&store, &writer, attempt, &model, &artifact, b"y = 2").await;
        }
        if case == "span" {
            let out_of_bounds = Occurrence { source: original.id(), start: 0, end: 6, syntax_kind: SyntaxKind::ExprName, role: OccurrenceRole::Read, structural_path: vec![] };
            store.copy(&writer, attempt, &Batch::new(&model, vec![out_of_bounds], &budget()).unwrap(), &budget()).await.unwrap();
        }
        store.seal(attempt).await.unwrap();
        let error = store.validate(attempt, &budget()).await.unwrap_err();
        assert!(matches!(error, Error::Model(_)), "case {case}: {error}");
        if case == "span" { assert!(error.to_string().contains("outside its source"), "{error}"); }
        assert!(store.publish(attempt).await.is_err());
        store.abort(attempt).await.unwrap();
    }
    // Every referenced row is valid, but verification for another acquisition cannot own this file.
    let crossed = store.create_conformance(producer_digest, "catalog").await.unwrap();
    let empty_input = InputRevision::from_entries(vec![]).unwrap();
    let crossed_origin = InputOrigin::Tree { label: "other input".into() };
    let crossed_acquisition = InputAcquisition { input: empty_input.id(), origin: crossed_origin.id() };
    let crossed_verification = DistributionVerification { acquisition: crossed_acquisition.id(), release: release.id(), record_digest: ContentHash::of(b"other record"), artifact_sha256: vec![] };
    let original = SourceArtifact::from_bytes(input.id(), "example.py".into(), b"x = 1").unwrap();
    let crossed_ownership = ArtifactOwnership { artifact: original.id(), distribution: crossed_verification.id() };
    macro_rules! copy_crossed { ($($row:expr),+ $(,)?) => { $(store.copy(&writer, crossed, &Batch::new(&model, vec![$row], &budget()).unwrap(), &budget()).await.unwrap();)+ }; }
    copy_crossed!(input.clone(), empty_input, crossed_origin, crossed_acquisition, crossed_verification, package.clone(), release.clone(), crossed_ownership);
    copy_artifact(&store, &writer, crossed, &model, &original, b"x = 1").await;
    copy_artifact(&store, &writer, crossed, &model, &auxiliary, b"y = 2").await;
    store.seal(crossed).await.unwrap();
    let error = store.validate(crossed, &budget()).await.unwrap_err();
    assert!(error.to_string().contains("ownership crosses"), "{error}");
    assert!(store.publish(crossed).await.is_err());
    store.abort(crossed).await.unwrap();
    for case in ["missing-family", "wrong-family", "failed-provider"] {
        let attempt = store.create_conformance(producer_digest, "catalog").await.unwrap();
        let empty_input = InputRevision::from_entries(vec![]).unwrap();
        let (invocation, mut memberships) = ProviderRun::new(provider.id(), context.id(), empty_input.id(), context.config_digest, [FactFamily::Syntax]).unwrap();
        let scope = CoverageScope::Input { input: empty_input.id() };
        let outcome = ProviderCoverage { scope: scope.id(), provider: provider.id(), context: context.id(), family: FactFamily::Syntax,
            run: Some(invocation.id()), status: CoverageStatus::Failed, reason: Some(ObligationKind::NativeUnavailable), diagnostic: None };
        macro_rules! copy_attempt { ($($row:expr),+ $(,)?) => { $(store.copy(&writer, attempt, &Batch::new(&model, vec![$row], &budget()).unwrap(), &budget()).await.unwrap();)+ }; }
        copy_attempt!(empty_input, provider.clone(), context.clone(), invocation, scope);
        if case != "missing-family" {
            if case == "wrong-family" { memberships[0].family = FactFamily::Flow; }
            store.copy(&writer, attempt, &Batch::new(&model, memberships, &budget()).unwrap(), &budget()).await.unwrap();
        }
        if case == "failed-provider" { copy_attempt!(outcome); }
        store.seal(attempt).await.unwrap();
        let error = store.validate(attempt, &budget()).await.unwrap_err();
        let expected = match case { "missing-family" => "lacks requested families", "wrong-family" => "family digest differs", _ => "failed provider invocation" };
        assert!(error.to_string().contains(expected), "{case}: {error}");
        assert!(store.publish(attempt).await.is_err());
        store.abort(attempt).await.unwrap();
    }
    let damaged = store.create_conformance(producer_digest, "catalog").await.unwrap();
    store.seal(damaged).await.unwrap(); store.validate(damaged, &budget()).await.unwrap();
    sqlx::query("UPDATE lctx_model_store.validation_receipts SET validator_name='substituted' WHERE generation_id=decode($1,'hex') AND validator_name='input_manifest_membership'").bind(damaged.hex()).execute(&owner).await.unwrap();
    assert!(matches!(store.publish(damaged).await, Err(Error::Contract)));
    store.abort(damaged).await.unwrap();
    // Missing reference is caught against sealed stored contents, not a caller's batch receipt.
    let bad = store.create_conformance(producer_digest, "catalog").await.unwrap();
    store.copy(&writer, bad, &Batch::new(&model, vec![release.clone()], &budget()).unwrap(), &budget()).await.unwrap(); store.seal(bad).await.unwrap();
    assert!(store.validate(bad, &budget()).await.is_err()); assert!(store.publish(bad).await.is_err());
    assert_eq!(store.abort(bad).await.unwrap(),CleanupOutcome::Removed); assert_eq!(store.abort(bad).await.unwrap(),CleanupOutcome::AlreadyAbsent);
    assert!(matches!(store.pin(&reader, bad, budget()).await, Err(Error::State)));
    let abandoned = store.create_conformance(producer_digest, "catalog").await.unwrap();
    store.abort(abandoned).await.unwrap();
    assert!(store.seal(abandoned).await.is_err());
    let absent: bool = sqlx::query_scalar("SELECT NOT EXISTS(SELECT 1 FROM pg_namespace WHERE nspname=$1)").bind(abandoned.schema()).fetch_one(&owner).await.unwrap();
    assert!(absent);
    // No ordinary retry treats an orphaned schema or registry as AlreadyAbsent.
    let orphan = store.create_conformance(producer_digest,"catalog").await.unwrap();
    sqlx::query("DELETE FROM lctx_model_store.generations WHERE id=decode($1,'hex')").bind(orphan.hex()).execute(&owner).await.unwrap();
    assert!(matches!(store.abort(orphan).await,Err(Error::Orphaned)));
    assert_eq!(store.repair_orphan(orphan).await.unwrap(),CleanupOutcome::Removed);
    assert_eq!(store.repair_orphan(orphan).await.unwrap(),CleanupOutcome::AlreadyAbsent);
    let missing_schema = store.create_conformance(producer_digest,"catalog").await.unwrap();
    sqlx::query(sqlx::AssertSqlSafe(format!("DROP SCHEMA {} CASCADE",missing_schema.schema()))).execute(&owner).await.unwrap();
    assert!(matches!(store.abort(missing_schema).await,Err(Error::Orphaned)));
    assert_eq!(store.repair_orphan(missing_schema).await.unwrap(),CleanupOutcome::Removed);
    assert!(matches!(store.repair_orphan(next).await,Err(Error::Busy)));
    store.clear_selection().await.unwrap();
    assert!(matches!(store.repair_orphan(next).await,Err(Error::State)));
    let wrong_subtype = store.create_conformance(producer_digest, "catalog").await.unwrap();
    let wrong_scope = CoverageScope::Release { release: release.id() };
    store.copy(&writer, wrong_subtype, &Batch::new(&model, vec![package], &budget()).unwrap(), &budget()).await.unwrap();
    store.copy(&writer, wrong_subtype, &Batch::new(&model, vec![release], &budget()).unwrap(), &budget()).await.unwrap();
    store.copy(&writer, wrong_subtype, &Batch::new(&model, vec![wrong_scope.clone()], &budget()).unwrap(), &budget()).await.unwrap();
    // FK checking is deliberately deferred until every staging table is loaded.
    sqlx::query(sqlx::AssertSqlSafe(format!("INSERT INTO {}.module_scope_links(id,scope) VALUES($1,$2)",wrong_subtype.schema())))
        .bind(vec![0u8;16]).bind(wrong_scope.id().bytes().to_vec()).execute(&writer).await.unwrap();
    store.seal(wrong_subtype).await.unwrap();
    let error = store.validate(wrong_subtype, &budget()).await.unwrap_err();
    assert!(matches!(error, Error::Database(ref e) if e.as_database_error().and_then(|e| e.code()).as_deref() == Some("23503")));
    store.abort(wrong_subtype).await.unwrap();
    // A reader refuses a model mismatch before interpreting any physical values.
    sqlx::query("UPDATE lctx_model_store.generations SET model_digest=$1 WHERE id=decode($2,'hex')").bind(vec![0u8;32]).bind(next.hex()).execute(&owner).await.unwrap();
    assert!(matches!(store.pin(&reader, next, budget()).await, Err(Error::Contract)));
    owner.close().await; writer.close().await; reader.close().await;
}

#[tokio::test]
async fn chunked_evidence_round_trips_beyond_row_limit_and_sealed_corruption_refuses() {
    let (image, tag) = lctx_postgres::serving::TEST_IMAGE.trim().split_once(':').unwrap();
    let container = Postgres::default().with_name(image).with_tag(tag).start().await.expect("Docker and pinned PG18 image required");
    let port = container.get_host_port_ipv4(5432).await.unwrap();
    let url = |role: &str| format!("postgres://{role}:postgres@127.0.0.1:{port}/postgres");
    let owner = PgPool::connect(&url("postgres")).await.unwrap();
    sqlx::raw_sql("CREATE ROLE lctx_importer LOGIN PASSWORD 'postgres'; CREATE ROLE lctx_serving LOGIN PASSWORD 'postgres'").execute(&owner).await.unwrap();
    let writer = PgPool::connect(&url("lctx_importer")).await.unwrap();
    let reader = PgPool::connect(&url("lctx_serving")).await.unwrap();
    let model = Arc::new(model().unwrap());
    let store = GenerationStore::install(owner.clone(), model.clone()).await.unwrap();
    let bytes: Vec<u8> = (0..65 * ARTIFACT_CHUNK_BYTES + 17).map(|i| ((i / ARTIFACT_CHUNK_BYTES + i) % 256) as u8).collect();
    let input = InputRevision::from_entries(vec![ManifestEntry { path: "evidence.bin".into(), content: ContentHash::of(&bytes), byte_len: bytes.len() as i64 }, ManifestEntry { path: "empty.bin".into(), content: ContentHash::of(b""), byte_len: 0 }]).unwrap();
    let artifact = SourceArtifact::from_bytes(input.id(), "evidence.bin".into(), &bytes).unwrap();
    let empty = SourceArtifact::from_bytes(input.id(), "empty.bin".into(), b"").unwrap();
    let generation = store.create_conformance(ContentHash::of(b"chunk-test"), "catalog").await.unwrap();
    store.copy(&writer, generation, &Batch::new(&model, vec![input], &budget()).unwrap(), &budget()).await.unwrap();
    copy_artifact(&store, &writer, generation, &model, &empty, b"").await;
    copy_artifact(&store, &writer, generation, &model, &artifact, &bytes).await;
    store.seal(generation).await.unwrap(); store.validate(generation, &budget()).await.unwrap(); store.publish(generation).await.unwrap();
    let mut lease = store.pin(&reader, generation, budget()).await.unwrap();
    assert!(lease.read::<ArtifactChunk>().await.is_err()); // convenience collection remains bounded
    let mut seen = std::collections::BTreeSet::new();
    let mut total = 0;
    lease.visit::<ArtifactChunk>(|batch| {
        for chunk in batch.rows() {
            assert!(seen.insert(chunk.ordinal));
            assert_eq!(chunk.artifact, artifact.id());
            let offset = chunk.ordinal as usize * ARTIFACT_CHUNK_BYTES;
            assert_eq!(chunk.body.0, bytes[offset..offset + chunk.body.0.len()]);
            total += chunk.body.0.len();
        }
        Ok(())
    }).await.unwrap();
    assert_eq!(total, bytes.len()); assert_eq!(seen.len(), 66);
    drop(lease);
    for case in ["missing", "corrupt", "noncanonical", "misplaced"] {
        let input = InputRevision::from_entries(vec![ManifestEntry { path: "tiny.bin".into(), content: ContentHash::of(b"abc"), byte_len: 3 }, ManifestEntry { path: "other.bin".into(), content: ContentHash::of(b"xyz"), byte_len: 3 }]).unwrap();
        let a = SourceArtifact::from_bytes(input.id(), "tiny.bin".into(), b"abc").unwrap();
        let b = SourceArtifact::from_bytes(input.id(), "other.bin".into(), b"xyz").unwrap();
        let g = store.create_conformance(ContentHash::of(b"invalid-chunk"), "catalog").await.unwrap();
        store.copy(&writer, g, &Batch::new(&model, vec![input], &budget()).unwrap(), &budget()).await.unwrap();
        store.copy(&writer, g, &Batch::new(&model, vec![a.clone(), b.clone()], &budget()).unwrap(), &budget()).await.unwrap();
        if case != "missing" {
            let mut chunk = ArtifactChunk::split(&a, b"abc").unwrap().next().unwrap();
            if case == "corrupt" { chunk.body.0[0] = 0; }
            if case == "noncanonical" { chunk.ordinal = 1; }
            if case == "misplaced" { chunk.artifact = b.id(); }
            store.copy(&writer, g, &Batch::new(&model, vec![chunk], &budget()).unwrap(), &budget()).await.unwrap();
        }
        if case != "misplaced" {
            store.copy(&writer, g, &Batch::new(&model, ArtifactChunk::split(&b, b"xyz").unwrap().collect(), &budget()).unwrap(), &budget()).await.unwrap();
        }
        store.seal(g).await.unwrap();
        let error = store.validate(g, &budget()).await.unwrap_err();
        assert!(matches!(error, Error::Model(_)), "{case}: {error}");
        assert!(error.to_string().contains("chunk"), "{case}: {error}");
        assert!(store.publish(g).await.is_err()); store.abort(g).await.unwrap();
    }
}

/// A fresh attempt budget; these controls do not share reservations across batches.
fn budget() -> lctx_model::domain::resources::ResourceBudget {
    lctx_model::domain::resources::ResourceBudget::fixed(lctx_model::domain::resources::DEFAULT_MEMORY_BYTES).unwrap()
}
