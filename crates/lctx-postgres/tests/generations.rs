//! Permanent typed lowerings against a disposable real PostgreSQL server.
use lctx_model::domain::{assertion::*, conditions::*};
use lctx_model::{
    Domain,
    domain::{
        Batch, ContentHash, Id, Record, Relation, ValidatedModel, artifact::*, attribution::*,
        facts_relations, input::*, source::*,
    },
};
use lctx_postgres::generations::{CleanupOutcome, Error, GenerationStore};
use lctx_postgres::testing::DisposableDatabase;
use lctx_postgres::testing::Harness;
use std::sync::Arc;

#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(name = "recursive_nodes", semantic_source = include_bytes!("generations.rs"))]
struct RecursiveNode {
    #[model(key)]
    name: String,
    parent: Option<Id<RecursiveNode>>,
}
#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(name = "module_scope_links", semantic_source = include_bytes!("generations.rs"))]
struct ModuleScopeLink {
    #[model(key)]
    scope: CoverageScopeModuleId,
}

#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(name = "binary_evidence", semantic_source = include_bytes!("generations.rs"))]
struct BinaryEvidence {
    #[model(key)]
    name: String,
    body: lctx_model::domain::EvidenceBytes,
}

async fn copy_artifact(
    harness: &Harness,
    model: &ValidatedModel,
    artifact: &SourceArtifact,
    body: &[u8],
) {
    harness
        .copy(
            &Batch::new(model, vec![artifact.clone()], &budget()).unwrap(),
            &budget(),
        )
        .await
        .unwrap();
    for chunk in ArtifactChunk::split(artifact, body).unwrap() {
        harness
            .copy(
                &Batch::new(model, vec![chunk], &budget()).unwrap(),
                &budget(),
            )
            .await
            .unwrap();
    }
}

#[tokio::test]
async fn immutable_generation_vertical_slice_and_lifecycle_refusals() {
    #[derive(Debug, Clone, PartialEq, Eq, Domain)]
    #[model(name = "external_unversioned")]
    struct ExternalUnversioned {
        #[model(key)]
        name: String,
    }
    assert!(ValidatedModel::declared(vec![Relation::of::<ExternalUnversioned>()]).is_err());
    let db = DisposableDatabase::start().await;
    let owner = db.owner.pool().clone();
    let writer = db.writer.clone();
    let reader = sqlx::postgres::PgPoolOptions::new()
        .max_connections(1)
        .acquire_timeout(std::time::Duration::from_millis(150))
        .connect(&db.url("lctx_serving"))
        .await
        .unwrap();
    let mut relations = facts_relations();
    relations.push(Relation::of::<RecursiveNode>());
    relations.push(Relation::of::<ModuleScopeLink>());
    relations.push(Relation::of::<BinaryEvidence>());
    let model = Arc::new(ValidatedModel::declared(relations).unwrap());
    let store = GenerationStore::install(db.owner.clone(), model.clone())
        .await
        .unwrap();
    let producer_digest = ContentHash::of(b"producer-code");
    let mut g_h = Harness::begin(
        &store,
        writer.clone(),
        lctx_model::domain::stages::Profile::Catalog,
        budget(),
    )
    .await
    .unwrap();
    let g = g_h.generation();
    assert!(matches!(
        store.pin(&reader, g, budget()).await,
        Err(Error::State)
    ));
    assert!(matches!(g_h.publish().await, Err(Error::State)));
    let package = Package {
        name: "example".into(),
    };
    let release = Release {
        package: package.id(),
        version: "1.0".into(),
    };
    let input = InputRevision::from_entries(vec![
        ManifestEntry {
            path: "example.py".into(),
            content: ContentHash::of(b"x = 1"),
            byte_len: 5,
        },
        ManifestEntry {
            path: "aux.py".into(),
            content: ContentHash::of(b"y = 2"),
            byte_len: 5,
        },
    ])
    .unwrap();
    let source = SourceArtifact::from_bytes(input.id(), "example.py".into(), b"x = 1").unwrap();
    let auxiliary = SourceArtifact::from_bytes(input.id(), "aux.py".into(), b"y = 2").unwrap();
    let other_package = Package {
        name: "auxiliary".into(),
    };
    let other_release = Release {
        package: other_package.id(),
        version: "2.0".into(),
    };
    let origin = InputOrigin::Installed {
        library: "example".into(),
        requirement: "example==1.0".into(),
        lock_digest: ContentHash::of(b"lock"),
        installer: Some("uv".into()),
    };
    let acquisition = InputAcquisition {
        input: input.id(),
        origin: origin.id(),
    };
    let verified = DistributionVerification {
        acquisition: acquisition.id(),
        release: release.id(),
        record_digest: ContentHash::of(b"record-a"),
        artifact_sha256: vec!["ab".repeat(32)],
    };
    let distribution = InputDistribution {
        input: input.id(),
        release: release.id(),
        role: DistributionRole::FirstParty,
    };
    let other_distribution = InputDistribution {
        input: input.id(),
        release: other_release.id(),
        role: DistributionRole::Dependency,
    };
    let other_verified = DistributionVerification {
        acquisition: acquisition.id(),
        release: other_release.id(),
        record_digest: ContentHash::of(b"record-b"),
        artifact_sha256: vec!["cd".repeat(32)],
    };
    let ownership = ArtifactOwnership {
        artifact: source.id(),
        distribution: verified.id(),
    };
    let other_ownership = ArtifactOwnership {
        artifact: auxiliary.id(),
        distribution: other_verified.id(),
    };
    let example_use = ArtifactUse {
        artifact: source.id(),
        input: input.id(),
        role: SourceRole::Example,
    };
    let test_use = ArtifactUse {
        artifact: source.id(),
        input: input.id(),
        role: SourceRole::Test,
    };
    let module = Module {
        source: source.id(),
        qualified_name: "example".into(),
    };
    let occurrence = Occurrence {
        source: source.id(),
        start: 0,
        end: 1,
        syntax_kind: SyntaxKind::ExprName,
        role: OccurrenceRole::Binding,
        structural_path: vec![0],
    };
    let provider = Provider {
        tool: "ruff".into(),
        revision: "0.0.11".into(),
        build_digest: producer_digest,
    };
    let context = AnalysisContext {
        python_version: "3.14.7".into(),
        python_platform: "linux".into(),
        search_path: vec!["src".into()],
        site_package_path: vec![],
        config_digest: ContentHash::of(b"config"),
        environment_digest: ContentHash::of(b"env"),
        lock_digest: None,
    };
    let (run, run_families) = ProviderRun::new(
        provider.id(),
        context.id(),
        input.id(),
        context.config_digest,
        [FactFamily::Syntax],
    )
    .unwrap();
    let scope = CoverageScope::Module {
        module: module.id(),
    };
    let (condition, nodes) = Diagram::always().records();
    let qualification = AssertionQualification {
        assumptions: lctx_model::domain::assumptions::AssumptionSet::empty_id(),
        context: context.id(),
        scope: scope.id(),
        condition: condition.id(),
        modality: Modality::Definite,
        approximation: Approximation::Exact,
    };
    let surface = ProviderSurface {
        provider: provider.id(),
        family: FactFamily::Syntax,
        name: "syntax".into(),
    };
    let evidence = Evidence::Occurrence {
        occurrence: occurrence.id(),
    };
    let assertion = SyntaxObservation {
        qualification: qualification.id(),
        occurrence: occurrence.id(),
        spelling: "x".into(),
    };
    let support = SyntaxSupport {
        assertion: assertion.id(),
        run: run.id(),
        surface: surface.id(),
        evidence: evidence.id(),
        origin: Origin::SourceObservation,
        mode: ExtractionMode::NativeTraversal,
        fidelity: Fidelity::NativeStructural,
    };
    let link = ModuleScopeLink {
        scope: CoverageScopeModuleId::of(&scope).unwrap(),
    };
    let coverage = ProviderCoverage {
        scope: scope.id(),
        provider: Some(provider.id()),
        context: context.id(),
        family: FactFamily::Syntax,
        run: Some(run.id()),
        status: CoverageStatus::CompleteUnderStatedModel,
        reason: None,
        diagnostic: None,
    };
    let mut node = RecursiveNode {
        name: "self".into(),
        parent: None,
    };
    node.parent = Some(node.id());
    macro_rules! copy { ($($row:expr),*) => { $(g_h.copy(&Batch::new(&model, vec![$row.clone()], &budget()).unwrap(), &budget()).await.unwrap();)* }; }
    let binary = BinaryEvidence {
        name: "invalid-utf8".into(),
        body: lctx_model::domain::EvidenceBytes(vec![0, 255, 128]),
    };
    copy!(
        binary,
        other_package,
        other_release,
        origin,
        acquisition,
        distribution,
        other_distribution,
        verified,
        other_verified,
        ownership,
        other_ownership,
        example_use,
        test_use,
        package,
        release,
        input,
        module,
        occurrence,
        provider,
        context,
        run,
        condition,
        qualification,
        surface,
        evidence,
        assertion,
        support,
        scope,
        coverage,
        link,
        node
    );
    g_h.copy(&Batch::new(&model, nodes, &budget()).unwrap(), &budget())
        .await
        .unwrap();
    copy_artifact(&g_h, &model, &source, b"x = 1").await;
    copy_artifact(&g_h, &model, &auxiliary, b"y = 2").await;
    g_h.copy(
        &Batch::new(&model, run_families, &budget()).unwrap(),
        &budget(),
    )
    .await
    .unwrap();
    // The server refuses oversized payloads even when a caller bypasses typed COPY.
    let oversized = sqlx::query(sqlx::AssertSqlSafe(format!("INSERT INTO {}.binary_evidence(id,name,body) VALUES($1,'oversized',decode(repeat('ff',67108864),'hex'))", g.schema())))
        .bind(vec![0u8;16]).execute(&writer).await;
    assert!(
        matches!(oversized, Err(ref e) if e.as_database_error().and_then(|e| e.code()).as_deref() == Some("23514"))
    );
    // A malformed tagged row is refused by generated PostgreSQL constraints before validation.
    let malformed = sqlx::query(sqlx::AssertSqlSafe(format!(
        "INSERT INTO {}.coverage_scopes(id,kind,release_release,module_module) VALUES($1,1,$2,$3)",
        g.schema()
    )))
    .bind(vec![0u8; 16])
    .bind(release.id().bytes().to_vec())
    .bind(module.id().bytes().to_vec())
    .execute(&writer)
    .await;
    assert!(malformed.is_err());
    // A direct writer transaction must drain before sealing, even outside the typed COPY API.
    let held = Package {
        name: "held-write".into(),
    };
    let mut tx = writer.begin().await.unwrap();
    sqlx::query(sqlx::AssertSqlSafe(format!(
        "INSERT INTO {}.packages(id,name) VALUES($1,$2)",
        g.schema()
    )))
    .bind(held.id().bytes().to_vec())
    .bind(&held.name)
    .execute(&mut *tx)
    .await
    .unwrap();
    let started = std::time::Instant::now();
    let (sealed, committed) = tokio::join!(g_h.seal(), async {
        tokio::time::sleep(std::time::Duration::from_millis(200)).await;
        tx.commit().await
    });
    committed.unwrap();
    sealed.unwrap();
    assert!(
        started.elapsed() >= std::time::Duration::from_millis(200),
        "sealing waited for the open writer transaction"
    );
    assert!(
        g_h.copy(
            &Batch::new(&model, vec![held], &budget()).unwrap(),
            &budget()
        )
        .await
        .is_err()
    );
    let late = sqlx::query(sqlx::AssertSqlSafe(format!(
        "INSERT INTO {}.packages(id,name) VALUES($1,'late')",
        g.schema()
    )))
    .bind(vec![0u8; 16])
    .execute(&writer)
    .await;
    assert!(late.is_err());
    let content = g_h.validate(&budget()).await.unwrap();
    g_h.publish().await.unwrap();
    let mut lease = store.pin(&reader, g, budget()).await.unwrap();
    assert_eq!(
        lease.read::<BinaryEvidence>().await.unwrap().rows(),
        &[binary]
    );
    assert_eq!(
        lease.read::<SourceArtifact>().await.unwrap().rows(),
        Batch::new(&model, vec![source, auxiliary.clone()], &budget())
            .unwrap()
            .rows()
    );
    assert_eq!(
        lease.read::<ArtifactOwnership>().await.unwrap().rows(),
        Batch::new(&model, vec![ownership, other_ownership], &budget())
            .unwrap()
            .rows()
    );
    assert_eq!(
        lease
            .read::<DistributionVerification>()
            .await
            .unwrap()
            .rows(),
        Batch::new(&model, vec![verified, other_verified], &budget())
            .unwrap()
            .rows()
    );
    assert_eq!(
        lease.read::<ArtifactUse>().await.unwrap().rows(),
        Batch::new(&model, vec![example_use, test_use], &budget())
            .unwrap()
            .rows()
    );
    assert_eq!(
        lease.read::<AnalysisContext>().await.unwrap().rows(),
        std::slice::from_ref(&context)
    );
    assert_eq!(
        lease.read::<SyntaxSupport>().await.unwrap().rows(),
        &[support]
    );
    assert_eq!(
        lease.read::<ModuleScopeLink>().await.unwrap().rows(),
        &[link]
    );
    assert_eq!(
        lease.read::<CoverageScope>().await.unwrap().rows(),
        &[scope]
    );
    assert_eq!(
        lease.read::<ProviderCoverage>().await.unwrap().rows(),
        &[coverage]
    );
    assert_eq!(lease.read::<RecursiveNode>().await.unwrap().rows(), &[node]);
    assert!(matches!(
        store.pin(&reader, g, budget()).await,
        Err(Error::Database(sqlx::Error::PoolTimedOut))
    ));
    assert!(matches!(store.retire(g).await, Err(Error::Busy)));
    let mut next_h = Harness::begin(
        &store,
        writer.clone(),
        lctx_model::domain::stages::Profile::Catalog,
        budget(),
    )
    .await
    .unwrap();
    let next = next_h.generation();
    next_h.seal().await.unwrap();
    next_h.validate(&budget()).await.unwrap();
    next_h.publish().await.unwrap();
    assert!(matches!(store.select(next).await, Err(Error::Frontier(_))));
    let frontier: String = sqlx::query_scalar(
        "SELECT frontier FROM lctx_model_store.generations WHERE id=decode($1,'hex')",
    )
    .bind(next.hex())
    .fetch_one(&owner)
    .await
    .unwrap();
    assert_eq!(frontier, "conformance");
    // Force the pointer administratively only to exercise the retirement protection independently
    // of production frontier admission. This is not a successful facts-selection test.
    sqlx::query(
        "UPDATE lctx_model_store.selection SET generation_id=decode($1,'hex') WHERE singleton",
    )
    .bind(next.hex())
    .execute(&owner)
    .await
    .unwrap();
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
    assert!(matches!(
        store.pin(&reader, g, budget()).await,
        Err(Error::Absent)
    ));
    assert_ne!(content, ContentHash::of(b""));
    assert_eq!(
        store.retire(g).await.unwrap(),
        CleanupOutcome::AlreadyAbsent
    );
    for table in ["generations", "receipts", "validation_receipts", "events"] {
        let column = if table == "generations" {
            "id"
        } else {
            "generation_id"
        };
        let count: i64 = sqlx::query_scalar(sqlx::AssertSqlSafe(format!(
            "SELECT count(*) FROM lctx_model_store.{table} WHERE {column}=decode($1,'hex')"
        )))
        .bind(g.hex())
        .fetch_one(&owner)
        .await
        .unwrap();
        assert_eq!(count, 0, "{table}");
    }
    // Cross-relation invariants execute over sealed contents, including otherwise valid rows.
    for case in ["changed", "missing", "extra", "duplicate", "span"] {
        let mut attempt_h = Harness::begin(
            &store,
            writer.clone(),
            lctx_model::domain::stages::Profile::Catalog,
            budget(),
        )
        .await
        .unwrap();
        let original =
            SourceArtifact::from_bytes(input.id(), "example.py".into(), b"x = 1").unwrap();
        attempt_h
            .copy(
                &Batch::new(&model, vec![input.clone()], &budget()).unwrap(),
                &budget(),
            )
            .await
            .unwrap();
        copy_artifact(&attempt_h, &model, &auxiliary, b"y = 2").await;
        if case != "missing" {
            let artifact = if case == "changed" {
                SourceArtifact::from_bytes(input.id(), "example.py".into(), b"x = 2").unwrap()
            } else {
                original.clone()
            };
            copy_artifact(
                &attempt_h,
                &model,
                &artifact,
                if case == "changed" {
                    b"x = 2"
                } else {
                    b"x = 1"
                },
            )
            .await;
        }
        if matches!(case, "extra" | "duplicate") {
            let artifact = SourceArtifact::from_bytes(
                input.id(),
                if case == "extra" {
                    "extra.py"
                } else {
                    "example.py"
                }
                .into(),
                b"y = 2",
            )
            .unwrap();
            copy_artifact(&attempt_h, &model, &artifact, b"y = 2").await;
        }
        if case == "span" {
            let out_of_bounds = Occurrence {
                source: original.id(),
                start: 0,
                end: 6,
                syntax_kind: SyntaxKind::ExprName,
                role: OccurrenceRole::Read,
                structural_path: vec![],
            };
            attempt_h
                .copy(
                    &Batch::new(&model, vec![out_of_bounds], &budget()).unwrap(),
                    &budget(),
                )
                .await
                .unwrap();
        }
        attempt_h.seal().await.unwrap();
        let error = attempt_h.validate(&budget()).await.unwrap_err();
        assert!(matches!(error, Error::Model(_)), "case {case}: {error}");
        if case == "span" {
            assert!(error.to_string().contains("outside its source"), "{error}");
        }
        assert!(attempt_h.publish().await.is_err());
        attempt_h.abort().await.unwrap();
    }
    // Every referenced row is valid, but verification for another acquisition cannot own this file.
    let mut crossed_h = Harness::begin(
        &store,
        writer.clone(),
        lctx_model::domain::stages::Profile::Catalog,
        budget(),
    )
    .await
    .unwrap();
    let empty_input = InputRevision::from_entries(vec![]).unwrap();
    let crossed_origin = InputOrigin::Tree {
        label: "other input".into(),
    };
    let crossed_acquisition = InputAcquisition {
        input: empty_input.id(),
        origin: crossed_origin.id(),
    };
    let crossed_verification = DistributionVerification {
        acquisition: crossed_acquisition.id(),
        release: release.id(),
        record_digest: ContentHash::of(b"other record"),
        artifact_sha256: vec![],
    };
    let original = SourceArtifact::from_bytes(input.id(), "example.py".into(), b"x = 1").unwrap();
    let crossed_ownership = ArtifactOwnership {
        artifact: original.id(),
        distribution: crossed_verification.id(),
    };
    macro_rules! copy_crossed { ($($row:expr),+ $(,)?) => { $(crossed_h.copy(&Batch::new(&model, vec![$row], &budget()).unwrap(), &budget()).await.unwrap();)+ }; }
    copy_crossed!(
        input.clone(),
        empty_input,
        crossed_origin,
        crossed_acquisition,
        crossed_verification,
        package.clone(),
        release.clone(),
        crossed_ownership
    );
    copy_artifact(&crossed_h, &model, &original, b"x = 1").await;
    copy_artifact(&crossed_h, &model, &auxiliary, b"y = 2").await;
    crossed_h.seal().await.unwrap();
    let error = crossed_h.validate(&budget()).await.unwrap_err();
    assert!(error.to_string().contains("ownership crosses"), "{error}");
    assert!(crossed_h.publish().await.is_err());
    crossed_h.abort().await.unwrap();
    for case in ["missing-family", "wrong-family", "failed-provider"] {
        let mut attempt_h = Harness::begin(
            &store,
            writer.clone(),
            lctx_model::domain::stages::Profile::Catalog,
            budget(),
        )
        .await
        .unwrap();
        let empty_input = InputRevision::from_entries(vec![]).unwrap();
        let (invocation, mut memberships) = ProviderRun::new(
            provider.id(),
            context.id(),
            empty_input.id(),
            context.config_digest,
            [FactFamily::Syntax],
        )
        .unwrap();
        let scope = CoverageScope::Input {
            input: empty_input.id(),
        };
        let outcome = ProviderCoverage {
            scope: scope.id(),
            provider: Some(provider.id()),
            context: context.id(),
            family: FactFamily::Syntax,
            run: Some(invocation.id()),
            status: CoverageStatus::Failed,
            reason: Some(ObligationKind::NativeUnavailable),
            diagnostic: None,
        };
        macro_rules! copy_attempt { ($($row:expr),+ $(,)?) => { $(attempt_h.copy(&Batch::new(&model, vec![$row], &budget()).unwrap(), &budget()).await.unwrap();)+ }; }
        copy_attempt!(
            empty_input,
            provider.clone(),
            context.clone(),
            invocation,
            scope
        );
        if case != "missing-family" {
            if case == "wrong-family" {
                memberships[0].family = FactFamily::Flow;
            }
            attempt_h
                .copy(
                    &Batch::new(&model, memberships, &budget()).unwrap(),
                    &budget(),
                )
                .await
                .unwrap();
        }
        if case == "failed-provider" {
            copy_attempt!(outcome);
        }
        attempt_h.seal().await.unwrap();
        let error = attempt_h.validate(&budget()).await.unwrap_err();
        let expected = match case {
            "missing-family" => "lacks requested families",
            "wrong-family" => "family digest differs",
            _ => "failed provider invocation",
        };
        assert!(error.to_string().contains(expected), "{case}: {error}");
        assert!(attempt_h.publish().await.is_err());
        attempt_h.abort().await.unwrap();
    }
    let mut damaged_h = Harness::begin(
        &store,
        writer.clone(),
        lctx_model::domain::stages::Profile::Catalog,
        budget(),
    )
    .await
    .unwrap();
    let damaged = damaged_h.generation();
    damaged_h.seal().await.unwrap();
    damaged_h.validate(&budget()).await.unwrap();
    store.audit(damaged, lctx_model::domain::admission::Frontier::Conformance, None, &budget()).await.unwrap();
    let content: Vec<u8> = sqlx::query_scalar("SELECT content_digest FROM lctx_model_store.generations WHERE id=decode($1,'hex')")
        .bind(damaged.hex()).fetch_one(&owner).await.unwrap();
    sqlx::query("UPDATE lctx_model_store.generations SET content_digest=decode(repeat('ab',32),'hex') WHERE id=decode($1,'hex')")
        .bind(damaged.hex()).execute(&owner).await.unwrap();
    assert!(store.audit(damaged, lctx_model::domain::admission::Frontier::Conformance, None, &budget()).await.is_err(), "audit challenges registry-only aggregate damage");
    sqlx::query("UPDATE lctx_model_store.generations SET content_digest=$2 WHERE id=decode($1,'hex')")
        .bind(damaged.hex()).bind(content).execute(&owner).await.unwrap();
    sqlx::query("UPDATE lctx_model_store.validation_receipts SET validator_name='substituted' WHERE generation_id=decode($1,'hex') AND validator_name='input_manifest_membership'").bind(damaged.hex()).execute(&owner).await.unwrap();
    // Normal publication trusts exact definition/binding stamps. Privileged substitution
    // of the display label is an explicit audit concern.
    assert!(store.audit(damaged, lctx_model::domain::admission::Frontier::Conformance, None, &budget()).await.is_err());
    damaged_h.publish().await.unwrap();
    store.retire(damaged).await.unwrap();
    // Missing reference is caught against sealed stored contents, not a caller's batch receipt.
    let mut bad_h = Harness::begin(
        &store,
        writer.clone(),
        lctx_model::domain::stages::Profile::Catalog,
        budget(),
    )
    .await
    .unwrap();
    let bad = bad_h.generation();
    bad_h
        .copy(
            &Batch::new(&model, vec![release.clone()], &budget()).unwrap(),
            &budget(),
        )
        .await
        .unwrap();
    bad_h.seal().await.unwrap();
    assert!(bad_h.validate(&budget()).await.is_err());
    assert!(bad_h.publish().await.is_err());
    assert_eq!(bad_h.abort().await.unwrap(), CleanupOutcome::Removed);
    assert_eq!(bad_h.abort().await.unwrap(), CleanupOutcome::AlreadyAbsent);
    assert!(matches!(
        store.pin(&reader, bad, budget()).await,
        Err(Error::Absent)
    ));
    let mut abandoned_h = Harness::begin(
        &store,
        writer.clone(),
        lctx_model::domain::stages::Profile::Catalog,
        budget(),
    )
    .await
    .unwrap();
    let abandoned = abandoned_h.generation();
    abandoned_h.abort().await.unwrap();
    assert!(abandoned_h.seal().await.is_err());
    let absent: bool =
        sqlx::query_scalar("SELECT NOT EXISTS(SELECT 1 FROM pg_namespace WHERE nspname=$1)")
            .bind(abandoned.schema())
            .fetch_one(&owner)
            .await
            .unwrap();
    assert!(absent);
    // No ordinary retry treats an orphaned schema or registry as AlreadyAbsent.
    let mut orphan_h = Harness::begin(
        &store,
        writer.clone(),
        lctx_model::domain::stages::Profile::Catalog,
        budget(),
    )
    .await
    .unwrap();
    let orphan = orphan_h.generation();
    sqlx::query("DELETE FROM lctx_model_store.generations WHERE id=decode($1,'hex')")
        .bind(orphan.hex())
        .execute(&owner)
        .await
        .unwrap();
    assert!(matches!(orphan_h.abort().await, Err(Error::Orphaned)));
    assert_eq!(
        store.repair_orphan(orphan).await.unwrap(),
        CleanupOutcome::Removed
    );
    assert_eq!(
        store.repair_orphan(orphan).await.unwrap(),
        CleanupOutcome::AlreadyAbsent
    );
    let mut missing_schema_h = Harness::begin(
        &store,
        writer.clone(),
        lctx_model::domain::stages::Profile::Catalog,
        budget(),
    )
    .await
    .unwrap();
    let missing_schema = missing_schema_h.generation();
    sqlx::query(sqlx::AssertSqlSafe(format!(
        "DROP SCHEMA {} CASCADE",
        missing_schema.schema()
    )))
    .execute(&owner)
    .await
    .unwrap();
    assert!(matches!(
        missing_schema_h.abort().await,
        Err(Error::Orphaned)
    ));
    assert_eq!(
        store.repair_orphan(missing_schema).await.unwrap(),
        CleanupOutcome::Removed
    );
    assert!(matches!(store.repair_orphan(next).await, Err(Error::Busy)));
    store.clear_selection().await.unwrap();
    assert!(matches!(store.repair_orphan(next).await, Err(Error::State)));
    let mut wrong_subtype_h = Harness::begin(
        &store,
        writer.clone(),
        lctx_model::domain::stages::Profile::Catalog,
        budget(),
    )
    .await
    .unwrap();
    let wrong_subtype = wrong_subtype_h.generation();
    let wrong_scope = CoverageScope::Release {
        release: release.id(),
    };
    wrong_subtype_h
        .copy(
            &Batch::new(&model, vec![package], &budget()).unwrap(),
            &budget(),
        )
        .await
        .unwrap();
    wrong_subtype_h
        .copy(
            &Batch::new(&model, vec![release], &budget()).unwrap(),
            &budget(),
        )
        .await
        .unwrap();
    wrong_subtype_h
        .copy(
            &Batch::new(&model, vec![wrong_scope.clone()], &budget()).unwrap(),
            &budget(),
        )
        .await
        .unwrap();
    // FK checking is deliberately deferred until every staging table is loaded.
    sqlx::query(sqlx::AssertSqlSafe(format!(
        "INSERT INTO {}.module_scope_links(id,scope) VALUES($1,$2)",
        wrong_subtype.schema()
    )))
    .bind(vec![0u8; 16])
    .bind(wrong_scope.id().bytes().to_vec())
    .execute(&writer)
    .await
    .unwrap();
    wrong_subtype_h.seal().await.unwrap();
    let error = wrong_subtype_h.validate(&budget()).await.unwrap_err();
    assert!(
        matches!(error, Error::Database(ref e) if e.as_database_error().and_then(|e| e.code()).as_deref() == Some("23503"))
    );
    wrong_subtype_h.abort().await.unwrap();
    // A reader refuses a model mismatch before interpreting any physical values.
    sqlx::query(
        "UPDATE lctx_model_store.generations SET model_digest=$1 WHERE id=decode($2,'hex')",
    )
    .bind(vec![0u8; 32])
    .bind(next.hex())
    .execute(&owner)
    .await
    .unwrap();
    assert!(matches!(
        store.pin(&reader, next, budget()).await,
        Err(Error::Contract)
    ));
    owner.close().await;
    writer.close().await;
    reader.close().await;
}

#[tokio::test]
async fn chunked_evidence_round_trips_beyond_row_limit_and_sealed_corruption_refuses() {
    let db = DisposableDatabase::start().await;
    let writer = db.writer.clone();
    let reader = db.reader.clone();
    let model = Arc::new(ValidatedModel::declared(facts_relations()).unwrap());
    let store = GenerationStore::install(db.owner.clone(), model.clone())
        .await
        .unwrap();
    let bytes: Vec<u8> = (0..65 * ARTIFACT_CHUNK_BYTES + 17)
        .map(|i| ((i / ARTIFACT_CHUNK_BYTES + i) % 256) as u8)
        .collect();
    let input = InputRevision::from_entries(vec![
        ManifestEntry {
            path: "evidence.bin".into(),
            content: ContentHash::of(&bytes),
            byte_len: bytes.len() as i64,
        },
        ManifestEntry {
            path: "empty.bin".into(),
            content: ContentHash::of(b""),
            byte_len: 0,
        },
    ])
    .unwrap();
    let artifact = SourceArtifact::from_bytes(input.id(), "evidence.bin".into(), &bytes).unwrap();
    let empty = SourceArtifact::from_bytes(input.id(), "empty.bin".into(), b"").unwrap();
    let mut generation_h = Harness::begin(
        &store,
        writer.clone(),
        lctx_model::domain::stages::Profile::Catalog,
        budget(),
    )
    .await
    .unwrap();
    let generation = generation_h.generation();
    generation_h
        .copy(
            &Batch::new(&model, vec![input], &budget()).unwrap(),
            &budget(),
        )
        .await
        .unwrap();
    copy_artifact(&generation_h, &model, &empty, b"").await;
    copy_artifact(&generation_h, &model, &artifact, &bytes).await;
    generation_h.seal().await.unwrap();
    generation_h.validate(&budget()).await.unwrap();
    generation_h.publish().await.unwrap();
    let mut lease = store.pin(&reader, generation, budget()).await.unwrap();
    assert!(lease.read::<ArtifactChunk>().await.is_err()); // convenience collection remains bounded
    let mut seen = std::collections::BTreeSet::new();
    let mut total = 0;
    lease
        .visit::<ArtifactChunk>(|batch| {
            for chunk in batch.rows() {
                assert!(seen.insert(chunk.ordinal));
                assert_eq!(chunk.artifact, artifact.id());
                let offset = chunk.ordinal as usize * ARTIFACT_CHUNK_BYTES;
                assert_eq!(chunk.body.0, bytes[offset..offset + chunk.body.0.len()]);
                total += chunk.body.0.len();
            }
            Ok(())
        })
        .await
        .unwrap();
    assert_eq!(total, bytes.len());
    assert_eq!(seen.len(), 66);
    drop(lease);
    for case in ["missing", "corrupt", "noncanonical", "misplaced"] {
        let input = InputRevision::from_entries(vec![
            ManifestEntry {
                path: "tiny.bin".into(),
                content: ContentHash::of(b"abc"),
                byte_len: 3,
            },
            ManifestEntry {
                path: "other.bin".into(),
                content: ContentHash::of(b"xyz"),
                byte_len: 3,
            },
        ])
        .unwrap();
        let a = SourceArtifact::from_bytes(input.id(), "tiny.bin".into(), b"abc").unwrap();
        let b = SourceArtifact::from_bytes(input.id(), "other.bin".into(), b"xyz").unwrap();
        let mut g_h = Harness::begin(
            &store,
            writer.clone(),
            lctx_model::domain::stages::Profile::Catalog,
            budget(),
        )
        .await
        .unwrap();
        g_h.copy(
            &Batch::new(&model, vec![input], &budget()).unwrap(),
            &budget(),
        )
        .await
        .unwrap();
        g_h.copy(
            &Batch::new(&model, vec![a.clone(), b.clone()], &budget()).unwrap(),
            &budget(),
        )
        .await
        .unwrap();
        if case != "missing" {
            let mut chunk = ArtifactChunk::split(&a, b"abc").unwrap().next().unwrap();
            if case == "corrupt" {
                chunk.body.0[0] = 0;
            }
            if case == "noncanonical" {
                chunk.ordinal = 1;
            }
            if case == "misplaced" {
                chunk.artifact = b.id();
            }
            g_h.copy(
                &Batch::new(&model, vec![chunk], &budget()).unwrap(),
                &budget(),
            )
            .await
            .unwrap();
        }
        if case != "misplaced" {
            g_h.copy(
                &Batch::new(
                    &model,
                    ArtifactChunk::split(&b, b"xyz").unwrap().collect(),
                    &budget(),
                )
                .unwrap(),
                &budget(),
            )
            .await
            .unwrap();
        }
        g_h.seal().await.unwrap();
        let error = g_h.validate(&budget()).await.unwrap_err();
        assert!(matches!(error, Error::Model(_)), "{case}: {error}");
        assert!(error.to_string().contains("chunk"), "{case}: {error}");
        assert!(g_h.publish().await.is_err());
        g_h.abort().await.unwrap();
    }
}

/// A fresh attempt budget; these controls do not share reservations across batches.
fn budget() -> lctx_model::domain::resources::ResourceBudget {
    lctx_model::domain::resources::ResourceBudget::fixed(
        lctx_model::domain::resources::DEFAULT_MEMORY_BYTES,
    )
    .unwrap()
}
