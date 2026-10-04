//! The binary's facts frontier over a preprovisioned locked input and real disposable PG18.
//! Acquisition is a no-op command here; semantic producers, capture, admission and store are real.
use lctx_postgres::{
    generations::{GenerationCatalog, GenerationStore, ListFilter},
    testing::DisposableDatabase,
};
use std::{path::Path, process::Command, sync::Arc};
fn write(path: &Path, bytes: impl AsRef<[u8]>) {
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, bytes).unwrap();
}
fn input(root: &Path) {
    input_source(
        root,
        b"def api(x: int) -> int:\n    if x:\n        return x\n    return 0\n",
    );
}
fn input_source(root: &Path, source: &[u8]) {
    write(
        &root.join("libraries/demo/pyproject.toml"),
        "[project]\nname = 'lctx-library-demo'\nversion = '0'\ndependencies = ['demo==1.0']\n[tool.lctx]\nrelease = ['demo']\n",
    );
    write(&root.join("libraries/demo/.python-version"), "3.14.7\n");
    write(
        &root.join("libraries/demo/uv.lock"),
        format!(
            "version = 1\nrevision = 3\nrequires-python = '==3.14.*'\n[[package]]\nname = 'demo'\nversion = '1.0'\nsource = {{ registry = 'https://pypi.org/simple' }}\nwheels = [{{ url = 'https://x/demo.whl', hash = 'sha256:{}', size = 1 }}]\n[[package]]\nname = 'lctx-library-demo'\nversion = '0'\nsource = {{ virtual = '.' }}\ndependencies = [{{ name = 'demo' }}]\n",
            "a".repeat(64)
        ),
    );
    write(
        &root.join("envs/demo/pyvenv.cfg"),
        "home = /x\nuv = 0.12.18\nversion_info = 3.14.7\n",
    );
    let metadata = b"Metadata-Version: 2.4\nName: demo\nVersion: 1.0\n";
    let site = root.join("envs/demo/lib/python3.14/site-packages");
    write(&site.join("demo/__init__.py"), source);
    write(&site.join("demo-1.0.dist-info/METADATA"), metadata);
    use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
    use sha2::{Digest as _, Sha256};
    write(
        &site.join("demo-1.0.dist-info/RECORD"),
        format!(
            "demo/__init__.py,sha256={},{}\ndemo-1.0.dist-info/METADATA,sha256={},{}\ndemo-1.0.dist-info/RECORD,,\n",
            URL_SAFE_NO_PAD.encode(Sha256::digest(source)),
            source.len(),
            URL_SAFE_NO_PAD.encode(Sha256::digest(metadata)),
            metadata.len()
        ),
    );
    write(&root.join("bin/uv"), "#!/bin/sh\nexit 0\n");
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(root.join("bin/uv"), std::fs::Permissions::from_mode(0o700)).unwrap();
}
fn command(root: &Path, cfg: &Path, through: &str, profile: &str) -> Command {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_lctx"));
    cmd.args([
        "compile",
        "demo",
        "--through",
        through,
        "--profile",
        profile,
    ])
    .arg("--database")
    .arg(cfg)
    .arg("--libraries")
    .arg(root.join("libraries"))
    .arg("--envs")
    .arg(root.join("envs"))
    .arg("--sources")
    .arg(root.join("sources"))
    .env(
        "PATH",
        format!(
            "{}:{}",
            root.join("bin").display(),
            std::env::var("PATH").unwrap_or_default()
        ),
    );
    cmd
}
#[tokio::test]
async fn binary_publishes_facts_reports_profiles_and_does_not_select() {
    let db = DisposableDatabase::start().await;
    db.migrate().await;
    let store = GenerationStore::install(
        db.owner.clone(),
        Arc::new(lctx_model::domain::model().unwrap()),
    )
    .await
    .unwrap();
    let catalog = GenerationCatalog::new(db.reader.clone());
    let dir = tempfile::tempdir().unwrap();
    input(dir.path());
    db.write_configs(dir.path()).unwrap();
    let cfg = dir.path().join("postgres.json");
    let refused = command(dir.path(), &cfg, "serving", "catalog")
        .output()
        .unwrap();
    assert_eq!(refused.status.code(), Some(3));
    assert!(
        catalog
            .list(&ListFilter::default())
            .await
            .unwrap()
            .is_empty()
    );
    let mut behavior = None;
    for (through, profile) in [
        ("facts", "catalog"),
        ("normalized", "catalog"),
        ("facts", "behavioral"),
        ("normalized", "behavioral"),
        ("normalized", "behavioral"),
    ] {
        let output = command(dir.path(), &cfg, through, profile)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let report: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
        assert_eq!(report["frontier"], through);
        assert_eq!(report["profile"], profile);
        assert_eq!(report["selected"], false);
        assert_eq!(
            report["stage_measurements"].as_array().unwrap().len(),
            (if profile == "catalog" { 5 } else { 6 })
                + if through == "normalized" { 9 } else { 0 }
        );
        let flow = report["families"]
            .as_array()
            .unwrap()
            .iter()
            .find(|f| f["family"] == "Flow")
            .unwrap();
        assert_eq!(flow["availability"] == "NotRequested", profile == "catalog");
        if profile == "behavioral" && through == "normalized" {
            if let Some(expected) = behavior.as_ref() {
                assert_eq!(&report["content_digest"], expected);
            } else {
                behavior = Some(report["content_digest"].clone());
            }
        }
    }
    let listed = catalog.list(&ListFilter::default()).await.unwrap();
    assert_eq!(listed.len(), 5);
    assert!(listed.iter().all(|g| !g.selected));
    for g in listed {
        store.retire(g.id).await.unwrap();
    }
    let exhausted = command(dir.path(), &cfg, "normalized", "catalog")
        .args(["--memory-bytes", "65536"])
        .output()
        .unwrap();
    assert!(!exhausted.status.success());
    assert!(
        catalog
            .list(&ListFilter::default())
            .await
            .unwrap()
            .is_empty()
    );
    assert!(
        GenerationStore::check(&db.owner, store.model())
            .await
            .unwrap()
            .clean()
    );
}

// Real native providers and the binary qualify the cumulative owner envelope.
#[tokio::test]
async fn binary_publishes_upper_frontiers_with_seedless_catalog_and_explicit_outcomes() {
    use lctx_model::domain::{Record, catalog as catalog_model, retrieval, synthesis};
    let db = DisposableDatabase::start().await;
    db.migrate().await;
    let store = GenerationStore::install(
        db.owner.clone(),
        Arc::new(lctx_model::domain::model().unwrap()),
    )
    .await
    .unwrap();
    let catalog = GenerationCatalog::new(db.reader.clone());
    let dir = tempfile::tempdir().unwrap();
    input(dir.path());
    db.write_configs(dir.path()).unwrap();
    let cfg = dir.path().join("postgres.json");
    // Acquisition would fail here. Invalid options must be rejected first.
    write(&dir.path().join("bin/uv"), "#!/bin/sh\nexit 91\n");
    let refused = command(dir.path(), &cfg, "catalog", "catalog")
        .args(["--techniques", "+fca,-fca"])
        .output()
        .unwrap();
    assert!(!refused.status.success());
    assert!(String::from_utf8_lossy(&refused.stderr).contains("contradictory"));
    assert!(
        catalog
            .list(&ListFilter::default())
            .await
            .unwrap()
            .is_empty()
    );
    write(&dir.path().join("bin/uv"), "#!/bin/sh\nexit 0\n");
    write(
        &dir.path().join("libraries/demo/analytics.toml"),
        "version = 1\n[subsystem]\nmodule_prefixes = ['demo']\npublic_roots = ['demo']\n[seeds]\nprimary = []\ndistractors = []\n[pass_a]\nmax_depth = 4\nmax_vertices = 256\nmax_edges = 1024\nmax_witnesses = 4\n[briefs]\nbudget = 0\n",
    );
    for through in ["analysis", "catalog"] {
        for profile in ["catalog", "behavioral"] {
            let output = command(dir.path(), &cfg, through, profile)
                .output()
                .unwrap();
            assert!(
                output.status.success(),
                "{}",
                String::from_utf8_lossy(&output.stderr)
            );
            let report: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
            assert_eq!(report["frontier"], through);
            assert_eq!(report["profile"], profile);
            assert_eq!(report["selected"], false);
            let outcomes = report["analysis"]["outcomes"].as_array().unwrap();
            assert!(!outcomes.is_empty());
            for owner in [
                "local",
                "base_evaluation",
                "base_completion",
                "source_call",
                "enriched_execution",
                "model",
                "summary",
                "structural",
                "analytic",
                "catalog_core",
                "catalog_evidence",
            ] {
                assert!(
                    outcomes.iter().any(|row| row["owner"] == owner),
                    "missing nominal {owner}"
                );
            }
            assert!(
                outcomes
                    .iter()
                    .filter(|row| row["owner"] == "analytic")
                    .all(|row| row["status"] == "NotRequested")
            );
            let coverage = report["analysis"]["capabilities"].as_array().unwrap();
            let local = coverage
                .iter()
                .filter(|row| row["owner"] == "local")
                .collect::<Vec<_>>();
            assert!(!local.is_empty());
            assert!(local.iter().all(|row| row["availability"]
                == if profile == "catalog" {
                    "NotRequested"
                } else {
                    "Partial"
                }));
            let frontiers = report["analysis"]["frontiers"].as_array().unwrap();
            assert!(frontiers.iter().any(|row| row["frontier"] == "analysis"));
            assert_eq!(
                frontiers.iter().any(|row| row["frontier"] == "catalog"),
                through == "catalog"
            );
            let listed = catalog.list(&ListFilter::default()).await.unwrap();
            let generation = listed
                .iter()
                .find(|row| row.id.hex() == report["generation"].as_str().unwrap())
                .unwrap();
            assert!(!generation.selected);
            let count = |name: &str| {
                sqlx::AssertSqlSafe(format!(
                    "SELECT count(*) FROM {}.{}",
                    generation.id.schema(),
                    name
                ))
            };
            let members: i64 = sqlx::query_scalar(count(catalog_model::CatalogMember::NAME))
                .fetch_one(db.owner.pool())
                .await
                .unwrap();
            assert!(members > 0);
            if through == "catalog" {
                qualify_admitted_selection(
                    &db,
                    &store,
                    generation.id,
                    &dir.path().join("postgres-serving.json"),
                )
                .await;
                for owner in ["selection", "synthesis", "retrieval"] {
                    assert!(outcomes.iter().any(|row| row["owner"] == owner));
                }
                let units: i64 = sqlx::query_scalar(count(retrieval::Unit::NAME))
                    .fetch_one(db.owner.pool())
                    .await
                    .unwrap();
                assert!(units > 0);
                let briefs: i64 = sqlx::query_scalar(count(synthesis::briefs::Brief::NAME))
                    .fetch_one(db.owner.pool())
                    .await
                    .unwrap();
                assert_eq!(briefs, 0);
            } else {
                assert!(
                    !outcomes
                        .iter()
                        .any(|row| row["owner"] == "synthesis" || row["owner"] == "retrieval")
                );
            }
            // Compilation publishes but never selects; reader reporting is the same nominal projection.
            let shown = Command::new(env!("CARGO_BIN_EXE_lctx"))
                .arg("--database")
                .arg(&cfg)
                .args(["generation", "show", report["generation"].as_str().unwrap()])
                .output()
                .unwrap();
            assert!(
                shown.status.success(),
                "{}",
                String::from_utf8_lossy(&shown.stderr)
            );
            let detail: serde_json::Value = serde_json::from_slice(&shown.stdout).unwrap();
            assert_eq!(detail["analysis"], report["analysis"]);
        }
    }
    let listed = catalog.list(&ListFilter::default()).await.unwrap();
    assert_eq!(listed.len(), 4);
    assert!(listed.iter().all(|row| !row.selected));
    for row in listed {
        store.retire(row.id).await.unwrap();
    }
    assert!(
        GenerationStore::check(&db.owner, store.model())
            .await
            .unwrap()
            .clean()
    );
}

/// Fake embeddings qualify the external service seam only. Native facts, synthesis, admission,
/// immutable vector-use receipts and cache winners are the actual implementation.
#[tokio::test]
async fn binary_catalog_with_briefs_replays_shared_embedding_winners() {
    use lctx_model::domain::{Record, embedding, retrieval, synthesis};
    let db = DisposableDatabase::start().await;
    db.migrate().await;
    let store = GenerationStore::install(
        db.owner.clone(),
        Arc::new(lctx_model::domain::model().unwrap()),
    )
    .await
    .unwrap();
    let catalog = GenerationCatalog::new(db.reader.clone());
    let dir = tempfile::tempdir().unwrap();
    input_source(
        dir.path(),
        br#"__all__ = ['api', 'consume', 'RecordHolder']
def api(x: int) -> int:
    """Return the supplied value when it is nonzero.

    Warning:
        Preserve credentials.
    """
    if x:
        return x
    return 0

def consume(x: int) -> int:
    """Consume an API result."""
    return api(x)

from dataclasses import dataclass

@dataclass(init=False)
class RecordHolder:
    value: object

    def __init__(self, value):
        self.value = value

    def read(self, flag, other):
        return self.value if flag else ([self.value] if other else consume(self.value))
"#,
    );
    write(
        &dir.path().join("libraries/demo/analytics.toml"),
        "version = 1\n[subsystem]\nmodule_prefixes = ['demo']\npublic_roots = ['demo']\n[seeds]\nprimary = ['demo.api']\ndistractors = []\n[pass_a]\nmax_depth = 2\nmax_vertices = 256\nmax_edges = 1024\nmax_witnesses = 4\n[briefs]\nbudget = 2\n",
    );
    db.write_configs(dir.path()).unwrap();
    let cfg = dir.path().join("postgres.json");
    for profile in ["catalog", "behavioral"] {
        sqlx::query("DELETE FROM lctx_cache.embedding_values")
            .execute(db.owner.pool())
            .await
            .unwrap();
        let mut expected = None;
        let mut expected_values = None;
        let mut expected_cache = None;
        for replay in 0..3 {
            if replay == 2 {
                sqlx::query("DELETE FROM lctx_cache.embedding_values")
                    .execute(db.owner.pool())
                    .await
                    .unwrap();
            }
            let output = command(dir.path(), &cfg, "catalog", profile)
                .args(["--embedder", "fake", "--techniques", "+knn"])
                .output()
                .unwrap();
            assert!(
                output.status.success(),
                "{}",
                String::from_utf8_lossy(&output.stderr)
            );
            let report: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
            assert_eq!(report["selected"], false);
            if let Some(expected) = &expected {
                assert_eq!(&report["content_digest"], expected);
            } else {
                expected = Some(report["content_digest"].clone());
            }
            let listed = catalog.list(&ListFilter::default()).await.unwrap();
            let generation = listed
                .iter()
                .find(|row| row.id.hex() == report["generation"].as_str().unwrap())
                .unwrap();
            assert!(!generation.selected);
            let count = |name: &str| {
                sqlx::AssertSqlSafe(format!(
                    "SELECT count(*) FROM {}.{}",
                    generation.id.schema(),
                    name
                ))
            };
            let briefs: i64 = sqlx::query_scalar(count(synthesis::briefs::Brief::NAME))
                .fetch_one(db.owner.pool())
                .await
                .unwrap();
            assert!(briefs > 0);
            let conclusions: i64 =
                sqlx::query_scalar(count(synthesis::documentary::DocumentaryConclusion::NAME))
                    .fetch_one(db.owner.pool())
                    .await
                    .unwrap();
            assert!(conclusions > 0);
            if profile == "behavioral" {
                let facets:i64=sqlx::query_scalar(sqlx::AssertSqlSafe(format!(
                "SELECT count(*) FROM {}.synthesis_summary_facets f JOIN {}.summary_claims c ON c.id=f.claim JOIN {}.summary_symbolic_field_alternatives a ON a.id=c.symbolicfieldassociation_alternative WHERE f.verdict=3 AND f.source IS NULL AND f.qualification=a.reader_qualification",generation.id.schema(),generation.id.schema(),generation.id.schema())))
                .fetch_one(db.owner.pool()).await.unwrap();
                assert_eq!(
                    facets, 4,
                    "three source readers retain four guarded sink outcomes without finding authority"
                );
                let readers: i64 = sqlx::query_scalar(sqlx::AssertSqlSafe(format!(
                    "SELECT count(DISTINCT link) FROM {}.summary_symbolic_field_alternatives",
                    generation.id.schema()
                )))
                .fetch_one(db.owner.pool())
                .await
                .unwrap();
                assert_eq!(readers, 3);
            }
            for name in [
                embedding::analytic::AnalysisEmbeddingUse::NAME,
                retrieval::consumption::RetrievalEmbeddingUse::NAME,
            ] {
                let available: i64 = sqlx::query_scalar(sqlx::AssertSqlSafe(format!(
                    "SELECT count(*) FROM {}.{} WHERE availability = 0",
                    generation.id.schema(),
                    name
                )))
                .fetch_one(db.owner.pool())
                .await
                .unwrap();
                assert!(available > 0, "no available values in {name}");
            }
            let values: Vec<(Vec<u8>, Vec<u8>, Vec<u8>)> = sqlx::query_as(sqlx::AssertSqlSafe(format!(
            "SELECT input, value_digest, bytes FROM {0}.{1} UNION SELECT input, value_digest, bytes FROM {0}.{2} ORDER BY 1, 2, 3",
            generation.id.schema(), embedding::analytic::AnalysisEmbeddingUse::NAME, retrieval::consumption::RetrievalEmbeddingUse::NAME
        ))).fetch_all(db.owner.pool()).await.unwrap();

            let cached: Vec<(Vec<u8>, Vec<u8>, Vec<u8>)> = sqlx::query_as(
            "SELECT input_hash, value_digest, vector_bytes FROM lctx_cache.embedding_values ORDER BY 1, 2, 3"
        ).fetch_all(db.owner.pool()).await.unwrap();
            assert!(!cached.is_empty());
            for value in &values {
                assert!(
                    cached.contains(value),
                    "stored vector use differs from cache winner"
                );
            }
            if let Some(expected) = &expected_values {
                assert_eq!(&values, expected);
            } else {
                expected_values = Some(values);
            }
            if let Some(expected) = &expected_cache {
                assert_eq!(&cached, expected);
            } else {
                expected_cache = Some(cached);
            }
            let outcomes = report["analysis"]["outcomes"].as_array().unwrap();
            assert!(outcomes.iter().any(|row| row["owner"] == "analytic"
                && row["method"] == "Neighbours"
                && row["status"] == "Completed"));
            assert!(
                outcomes
                    .iter()
                    .any(|row| row["owner"] == "retrieval" && row["status"] == "Completed")
            );
        }
    }
    for generation in catalog.list(&ListFilter::default()).await.unwrap() {
        store.retire(generation.id).await.unwrap();
    }
    assert!(
        GenerationStore::check(&db.owner, store.model())
            .await
            .unwrap()
            .clean()
    );
}

/// F1 qualification stays on actual binary-produced canonical Catalog generations.
async fn qualify_admitted_selection(
    db: &DisposableDatabase,
    store: &GenerationStore,
    g: lctx_postgres::generations::GenerationId,
    config: &Path,
) {
    use lctx_model::domain::{
        normalized::callables::{DescriptorKind, EffectiveCallableAssessment},
        resources::ResourceBudget,
        selection::{self, JointPolicy, Mode, Predicate, Quantifier, Requirement},
        *,
    };
    use lctx_postgres::generations::{Error, GenerationReader, SELECTION_PREPARATION_BYTES};
    let hex = g.hex();
    let generation_bytes = (0..16)
        .map(|i| u8::from_str_radix(&hex[2 * i..2 * i + 2], 16).unwrap())
        .collect::<Vec<_>>();
    let role = lctx_postgres::roles::RoleConfig::load(config).unwrap();
    let reader = GenerationReader::connect(Arc::new(lctx_model::domain::model().unwrap()), &role)
        .await
        .unwrap();
    let budget = ResourceBudget::fixed(SELECTION_PREPARATION_BYTES).unwrap();
    let admitted = reader.prepare_selection(g, budget.clone()).await.unwrap();
    assert_eq!(admitted.generation().await, g);
    assert!(budget.reserved() > 0);
    let diagnostic = ResourceBudget::fixed(512 << 20).unwrap();
    let mut lease = store.pin(&db.reader, g, diagnostic.clone()).await.unwrap();
    let strict = lease.prepare_selection_strict().await.unwrap();
    let request = ResourceBudget::fixed(32 << 20).unwrap();
    for predicate in [
        Predicate::PublicModule {
            module: "demo".into(),
        },
        Predicate::DeclaresParameter { name: "x".into() },
        Predicate::ParameterRequired {
            name: "x".into(),
            required: true,
        },
        Predicate::InvocationForm {
            form: selection::InvocationForm::Function,
        },
    ] {
        let query = selection::Selection {
            requirements: vec![Requirement {
                predicate,
                quantifier: Quantifier::AnyApplicable,
            }],
            mode: Mode::Discovery,
            joint: JointPolicy::RequireCompatible,
        };
        let expected = strict.select(&query, &request).unwrap();
        for _ in 0..2 {
            let actual = admitted.select(&query, &request).await.unwrap();
            assert_eq!(actual.mode, expected.mode);
            assert_eq!(actual.candidates.len(), expected.candidates.len());
            for (left, right) in actual.candidates.iter().zip(&expected.candidates) {
                assert_eq!(
                    (
                        left.member,
                        left.analysis,
                        &left.path,
                        left.outcome,
                        left.joint
                    ),
                    (
                        right.member,
                        right.analysis,
                        &right.path,
                        right.outcome,
                        right.joint
                    )
                );
                assert_eq!(left.requirements.len(), right.requirements.len());
                for (left, right) in left.requirements.iter().zip(&right.requirements) {
                    assert_eq!(
                        (
                            &left.requirement,
                            left.outcome,
                            left.reason,
                            left.examined,
                            left.total,
                            left.corpus_complete,
                            left.analyzer_complete,
                            &left.closure
                        ),
                        (
                            &right.requirement,
                            right.outcome,
                            right.reason,
                            right.examined,
                            right.total,
                            right.corpus_complete,
                            right.analyzer_complete,
                            &right.closure
                        )
                    );
                    assert_eq!(
                        format!("{:?}", left.witnesses),
                        format!("{:?}", right.witnesses)
                    );
                    assert_eq!(
                        left.admissible().collect::<Vec<_>>(),
                        right.admissible().collect::<Vec<_>>()
                    );
                }
            }
        }
    }
    let assessments = lease.read::<EffectiveCallableAssessment>().await.unwrap();
    let original = assessments
        .rows()
        .iter()
        .find(|r| r.descriptor_kind == Some(DescriptorKind::Function))
        .unwrap()
        .clone();
    let mut changed = original.clone();
    changed.descriptor_kind = Some(DescriptorKind::StaticMethod);
    changed.validate().unwrap();
    assert_eq!(changed.id(), original.id());
    drop(assessments);
    drop(strict);
    lease.release().await.unwrap();
    assert_eq!(diagnostic.reserved(), 0);
    let clone = admitted.clone();
    drop(admitted);
    assert!(budget.reserved() > 0);
    assert!(matches!(store.retire(g).await, Err(Error::Busy)));
    clone.release().await.unwrap();
    assert_eq!(budget.reserved(), 0);
    let small = ResourceBudget::fixed(1).unwrap();
    assert!(reader.prepare_selection(g, small.clone()).await.is_err());
    assert_eq!(small.reserved(), 0);
    let mut admin = db.owner.pool().acquire().await.unwrap();
    admin.close_on_drop();
    let table = format!("{}.{}", g.schema(), EffectiveCallableAssessment::NAME);
    sqlx::query(sqlx::AssertSqlSafe(format!(
        "UPDATE {table} SET descriptor_kind=$1 WHERE id=$2"
    )))
    .bind(changed.descriptor_kind.unwrap().code())
    .bind(original.id().bytes().to_vec())
    .execute(&mut *admin)
    .await
    .unwrap();
    // The typed codec confirms a valid encoded semantic change, while its old receipt remains.
    let mut control = store
        .pin(&db.reader, g, ResourceBudget::fixed(32 << 20).unwrap())
        .await
        .unwrap();
    let encoded = control.read::<EffectiveCallableAssessment>().await.unwrap();
    assert!(encoded.rows().contains(&changed));
    drop(encoded);
    control.release().await.unwrap();
    assert!(matches!(
        reader.prepare_selection(g, budget.clone()).await,
        Err(Error::Contract)
    ));
    assert_eq!(budget.reserved(), 0);
    sqlx::query(sqlx::AssertSqlSafe(format!(
        "UPDATE {table} SET descriptor_kind=$1 WHERE id=$2"
    )))
    .bind(original.descriptor_kind.unwrap().code())
    .bind(original.id().bytes().to_vec())
    .execute(&mut *admin)
    .await
    .unwrap();
    let evidence = format!("{}.{}", g.schema(), selection::DomainEvidence::NAME);
    sqlx::query(sqlx::AssertSqlSafe(format!(
        "CREATE TEMP TABLE removed_selection_evidence AS SELECT * FROM {evidence} LIMIT 1"
    )))
    .execute(&mut *admin)
    .await
    .unwrap();
    let deleted = sqlx::query(sqlx::AssertSqlSafe(format!(
        "DELETE FROM {evidence} WHERE id IN (SELECT id FROM removed_selection_evidence)"
    )))
    .execute(&mut *admin)
    .await
    .unwrap();
    assert_eq!(deleted.rows_affected(), 1);
    assert!(reader.prepare_selection(g, budget.clone()).await.is_err());
    assert_eq!(budget.reserved(), 0);
    sqlx::query(sqlx::AssertSqlSafe(format!(
        "INSERT INTO {evidence} SELECT * FROM removed_selection_evidence"
    )))
    .execute(&mut *admin)
    .await
    .unwrap();
    sqlx::query("CREATE TEMP TABLE removed_selection_validation AS SELECT * FROM lctx_model_store.validation_receipts WHERE generation_id=$1 AND validator_name='catalog_selection_declaration_closure'").bind(generation_bytes.clone()).execute(&mut *admin).await.unwrap();
    sqlx::query("DELETE FROM lctx_model_store.validation_receipts WHERE generation_id=$1 AND validator_name='catalog_selection_declaration_closure'").bind(generation_bytes.clone()).execute(&mut *admin).await.unwrap();
    assert!(reader.prepare_selection(g, budget.clone()).await.is_err());
    assert_eq!(budget.reserved(), 0);
    sqlx::query("INSERT INTO lctx_model_store.validation_receipts SELECT * FROM removed_selection_validation").execute(&mut *admin).await.unwrap();
    // Control metadata identity changes leave canonical relation hashes intact, exercising the
    // producer source/epoch resolver rather than merely the generic row hash mismatch.
    let mut audit = store
        .pin(&db.reader, g, ResourceBudget::fixed(32 << 20).unwrap())
        .await
        .unwrap();
    let sources = audit
        .read::<analysis::selection::SourceReceipt>()
        .await
        .unwrap();
    let source = sources
        .rows()
        .iter()
        .find(|r| stages::is_vocabulary(r.source().relation()) && r.source().prefix().is_some())
        .unwrap()
        .source();
    let mut nominal = None;
    for row in sources.rows().iter() {
        let candidate = row.source();
        if !stages::is_vocabulary(candidate.relation()) && candidate.prefix().is_some() {
            let grouped:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM lctx_model_store.publication_outputs WHERE generation_id=$1 AND stage_name=$2 AND relation_name=$3)").bind(generation_bytes.clone()).bind(candidate.producer()).bind(candidate.relation()).fetch_one(&mut *admin).await.unwrap();
            if grouped {
                nominal = Some(candidate);
                break;
            }
        }
    }
    let nominal = nominal.expect("C2 captures a non-vocabulary explicit producer group");
    drop(sources);
    audit.release().await.unwrap();
    // This control covers C2's captured source identity, including metadata beyond the narrow
    // classifier inputs. A different closed, FK-valid producer group cannot authorize an
    // unchanged capture hash: all canonical row and receipt payloads remain preserved.
    let original_epoch:i16=sqlx::query_scalar("SELECT epoch FROM lctx_model_store.publication_outputs WHERE generation_id=$1 AND stage_name=$2 AND relation_name=$3").bind(generation_bytes.clone()).bind(nominal.producer()).bind(nominal.relation()).fetch_one(&mut *admin).await.unwrap();
    let foreign_epoch:i16=sqlx::query_scalar("SELECT epoch FROM lctx_model_store.publication_groups WHERE generation_id=$1 AND closed AND epoch<>$2 ORDER BY epoch LIMIT 1").bind(generation_bytes.clone()).bind(original_epoch).fetch_one(&mut *admin).await.unwrap();
    let swapped=sqlx::query("UPDATE lctx_model_store.publication_outputs SET epoch=$4 WHERE generation_id=$1 AND stage_name=$2 AND relation_name=$3").bind(generation_bytes.clone()).bind(nominal.producer()).bind(nominal.relation()).bind(foreign_epoch).execute(&mut *admin).await.unwrap();
    assert_eq!(swapped.rows_affected(), 1);
    assert!(matches!(
        reader.prepare_selection(g, budget.clone()).await,
        Err(Error::Contract)
    ));
    assert_eq!(budget.reserved(), 0);
    sqlx::query("UPDATE lctx_model_store.publication_outputs SET epoch=$4 WHERE generation_id=$1 AND stage_name=$2 AND relation_name=$3").bind(generation_bytes.clone()).bind(nominal.producer()).bind(nominal.relation()).bind(original_epoch).execute(&mut *admin).await.unwrap();
    sqlx::query("UPDATE lctx_model_store.stage_receipts SET schedule_digest=$4 WHERE generation_id=$1 AND stage_name=$2 AND relation_name=$3").bind(generation_bytes.clone()).bind(source.producer()).bind(source.relation()).bind(vec![1u8;32]).execute(&mut *admin).await.unwrap();
    assert!(reader.prepare_selection(g, budget.clone()).await.is_err());
    assert_eq!(budget.reserved(), 0);
    sqlx::query("UPDATE lctx_model_store.stage_receipts SET schedule_digest=$4 WHERE generation_id=$1 AND stage_name=$2 AND relation_name=$3").bind(generation_bytes.clone()).bind(source.producer()).bind(source.relation()).bind(source.schedule().0.to_vec()).execute(&mut *admin).await.unwrap();
    sqlx::query("UPDATE lctx_model_store.publication_groups SET schedule_digest=$2 WHERE generation_id=$1 AND boundary=0").bind(generation_bytes.clone()).bind(vec![2u8;32]).execute(&mut *admin).await.unwrap();
    assert!(reader.prepare_selection(g, budget.clone()).await.is_err());
    assert_eq!(budget.reserved(), 0);
    sqlx::query("UPDATE lctx_model_store.publication_groups SET schedule_digest=$2 WHERE generation_id=$1 AND boundary=0").bind(generation_bytes.clone()).bind(source.schedule().0.to_vec()).execute(&mut *admin).await.unwrap();
    // An epoch receipt for a foreign payload cannot authorize the declared producer prefix.
    sqlx::query("CREATE TEMP TABLE original_selection_epochs AS SELECT * FROM lctx_model_store.epoch_receipts WHERE generation_id=$1 AND relation_name=$2").bind(generation_bytes.clone()).bind(source.relation()).execute(&mut *admin).await.unwrap();
    sqlx::query("UPDATE lctx_model_store.epoch_receipts SET content_digest=$3 WHERE generation_id=$1 AND relation_name=$2").bind(generation_bytes.clone()).bind(source.relation()).bind(vec![3u8;32]).execute(&mut *admin).await.unwrap();
    assert!(reader.prepare_selection(g, budget.clone()).await.is_err());
    assert_eq!(budget.reserved(), 0);
    sqlx::query("UPDATE lctx_model_store.epoch_receipts r SET content_digest=o.content_digest FROM original_selection_epochs o WHERE r.generation_id=o.generation_id AND r.epoch=o.epoch AND r.relation_name=o.relation_name").execute(&mut *admin).await.unwrap();
    drop(admin);
    // Suspend at a real canonical table read, then cancel. No arbitrary callback or mock provider.
    let mut blocked = db.owner.pool().begin().await.unwrap();
    sqlx::query(sqlx::AssertSqlSafe(format!(
        "LOCK TABLE {}.{} IN ACCESS EXCLUSIVE MODE",
        g.schema(),
        catalog::CatalogMember::NAME
    )))
    .execute(&mut *blocked)
    .await
    .unwrap();
    let read = reader.clone();
    let held = budget.clone();
    let task = tokio::spawn(async move { read.prepare_selection(g, held).await });
    let catalog = lctx_postgres::generations::GenerationCatalog::new(db.reader.clone());
    tokio::time::timeout(std::time::Duration::from_secs(10),async {loop{
        let waiting:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM pg_locks WHERE locktype='relation' AND NOT granted AND mode='AccessShareLock' AND relation=$1::regclass)").bind(format!("{}.{}",g.schema(),catalog::CatalogMember::NAME)).fetch_one(&db.superuser).await.unwrap();
        if waiting && catalog.show(g).await.unwrap().unwrap().summary.readers>0 {break;}
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    }}).await.unwrap();
    assert!(
        budget.reserved() > 0,
        "preparation must be suspended with retained state"
    );
    task.abort();
    assert!(task.await.is_err_and(|e| e.is_cancelled()));
    blocked.rollback().await.unwrap();
    assert_eq!(budget.reserved(), 0);
    tokio::time::timeout(std::time::Duration::from_secs(10), async {
        loop {
            if catalog.show(g).await.unwrap().unwrap().summary.readers == 0 {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
    })
    .await
    .unwrap();
    let admitted = reader.prepare_selection(g, budget.clone()).await.unwrap();
    let key = i64::from_le_bytes(generation_bytes[..8].try_into().unwrap());
    let high = (key as u64 >> 32) as i64;
    let low = (key as u64 & 0xffff_ffff) as i64;
    let pid:i32=sqlx::query_scalar("SELECT pid FROM pg_locks WHERE locktype='advisory' AND granted AND mode='ShareLock' AND objsubid=1 AND classid::bigint=$1 AND objid::bigint=$2").bind(high).bind(low).fetch_one(&db.superuser).await.unwrap();
    sqlx::query("SELECT pg_terminate_backend($1)")
        .bind(pid)
        .execute(&db.superuser)
        .await
        .unwrap();
    let query = selection::Selection {
        requirements: vec![],
        mode: Mode::Discovery,
        joint: JointPolicy::RequireCompatible,
    };
    assert!(admitted.select(&query, &request).await.is_err());
    // A new independent generation lease cannot revive the lost original capability.
    let replacement = store
        .pin(&db.reader, g, ResourceBudget::fixed(32 << 20).unwrap())
        .await
        .unwrap();
    assert!(matches!(
        admitted.select(&query, &request).await,
        Err(Error::State)
    ));
    drop(admitted);
    assert_eq!(budget.reserved(), 0);
    replacement.release().await.unwrap();
    reader.close().await;
    qualify_generation_runtime(db, store, g, config).await;
}

async fn qualify_generation_runtime(
    db: &DisposableDatabase,
    store: &GenerationStore,
    generation: lctx_postgres::generations::GenerationId,
    config: &Path,
) {
    use lctx_postgres::generations::{Error, GenerationService};
    use std::{sync::Arc, time::Duration};
    let model = Arc::new(lctx_model::domain::model().unwrap());
    let mut role = lctx_postgres::roles::RoleConfig::load(config).unwrap();
    role.provider_connections = 0; // No compile provider capacity in a canonical serving process.
    store.select(generation).await.unwrap();
    let service = GenerationService::admit(model.clone(), &role, None)
        .await
        .unwrap();
    assert_eq!(service.generation(), generation);
    store.clear_selection().await.unwrap();
    assert_eq!(
        service.generation(),
        generation,
        "selection is resolved only once"
    );
    assert!(matches!(store.retire(generation).await, Err(Error::Busy)));
    // A cancelled SQL waiter retains its execution/query grant until the original task drains.
    let mut blocker = db.owner.pool().begin().await.unwrap();
    sqlx::query(sqlx::AssertSqlSafe(format!(
        "LOCK TABLE {}.catalog_members IN ACCESS EXCLUSIVE MODE",
        generation.schema()
    )))
    .execute(&mut *blocker)
    .await
    .unwrap();
    let reading = service.execution().await.unwrap();
    let request = tokio::spawn(async move {
        reading
            .read::<lctx_model::domain::catalog::CatalogMember>()
            .await
    });
    tokio::time::timeout(Duration::from_secs(10), async {
        loop {
            let blocked: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM pg_stat_activity WHERE usename='lctx_serving' AND state='active' AND wait_event_type='Lock' AND query LIKE '%catalog_members%')")
                .fetch_one(&db.superuser).await.unwrap();
            if blocked { break; }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    }).await.unwrap();
    request.abort();
    let held = service.execution().await.unwrap();
    assert!(
        service.execution().await.is_err(),
        "cancelled SQL retains the first CPU grant until cleanup"
    );
    assert!(matches!(store.retire(generation).await, Err(Error::Busy)));
    blocker.rollback().await.unwrap();
    drop(held);
    tokio::time::timeout(Duration::from_secs(10), async {
        loop {
            let one = service.execution().await.unwrap();
            if let Ok(two) = service.execution().await {
                assert!(
                    !two.read::<lctx_model::domain::catalog::CatalogMember>()
                        .await
                        .unwrap()
                        .rows()
                        .is_empty()
                );
                drop((one, two));
                break;
            }
            drop(one);
        }
    })
    .await
    .unwrap();
    // Individually short phases share one deadline; later phases cannot restart its clock.
    let cumulative = service.execution().await.unwrap();
    let began = std::time::Instant::now();
    let remaining = cumulative.remaining().unwrap();
    for _ in 0..2 {
        cumulative
            .cpu(|_| {
                std::thread::sleep(Duration::from_secs(11));
                Ok(())
            })
            .await
            .unwrap();
    }
    let final_phase = cumulative
        .cpu(|_| {
            std::thread::sleep(Duration::from_secs(11));
            Ok(())
        })
        .await;
    assert!(matches!(
        final_phase,
        Err(Error::ResourceRefused("request deadline"))
    ));
    assert!(began.elapsed() >= remaining);
    assert!(cumulative.remaining().is_err());
    drop(cumulative);
    // The last timed-out CPU job keeps its grant until its actual completion.
    tokio::time::sleep(Duration::from_secs(4)).await;
    let first = service.execution().await.unwrap();
    let second = service.execution().await.unwrap();
    let (started, running) = tokio::sync::oneshot::channel();
    let (finish, blocked) = std::sync::mpsc::channel();
    let worker = first.clone();
    let waiting = tokio::spawn(async move {
        worker
            .cpu(move |budget| {
                let _charge = budget.reserve("runtime-cancel-control", 1024)?;
                started.send(()).unwrap();
                blocked.recv().unwrap();
                Ok(())
            })
            .await
    });
    drop(first);
    running.await.unwrap();
    waiting.abort(); // Only the wait is cancelled; actual CPU work retains its grant.
    assert!(
        service.execution().await.is_err(),
        "two slots remain retained"
    );
    tokio::time::timeout(Duration::from_secs(3), service.guard().check())
        .await
        .unwrap()
        .unwrap();
    assert!(matches!(store.retire(generation).await, Err(Error::Busy)));
    let close = service.clone();
    let shutdown = tokio::spawn(async move { close.shutdown().await });
    tokio::time::sleep(Duration::from_millis(30)).await;
    assert!(!shutdown.is_finished());
    assert!(service.execution().await.is_err());
    drop(second);
    finish.send(()).unwrap();
    tokio::time::timeout(Duration::from_secs(10), shutdown)
        .await
        .unwrap()
        .unwrap()
        .unwrap();
    assert!(service.guard().is_lost());
    assert!(service.execution().await.is_err());

    // Startup has independent admission and actual-worker drain, not a request deadline.
    let startup = GenerationService::admit(model.clone(), &role, Some(generation)).await.unwrap();
    let first = startup.execution().await.unwrap();
    let second = startup.execution().await.unwrap();
    let guard = startup.guard();
    let (began, mut observed) = tokio::sync::oneshot::channel();
    let queued = tokio::spawn(async move { guard.prepare_cpu(move |_| { let _ = began.send(()); Ok(()) }).await });
    tokio::time::sleep(Duration::from_millis(30)).await;
    assert!(observed.try_recv().is_err(), "queued startup did not bypass the shared CPU slots");
    queued.abort();
    assert!(queued.await.unwrap_err().is_cancelled());
    drop(first); drop(second);
    assert!(matches!(observed.await, Err(_)), "cancelled queued work never began");
    let baseline = startup.memory_reserved();
    let failed = startup.guard().prepare_cpu(|_| Err::<(), _>(Error::Contract)).await;
    assert!(matches!(failed, Err(Error::Contract)));
    assert_eq!(startup.memory_reserved(), baseline, "failed startup work releases its admission");
    let retained = startup.guard().prepare_cpu(|budget| Ok(budget.reserve("startup-retained-control", 4096)?)).await.unwrap();
    assert_eq!(startup.memory_reserved(), baseline + 4096);
    drop(retained); assert_eq!(startup.memory_reserved(), baseline);
    let (began, observed) = tokio::sync::oneshot::channel();
    let (finish, blocked) = std::sync::mpsc::channel();
    let guard = startup.guard();
    let job = tokio::spawn(async move {
        guard.prepare_cpu(move |budget| {
            let _charge = budget.reserve("startup-cancel-control", 4096)?;
            let _ = began.send(()); blocked.recv().unwrap(); Ok(())
        }).await
    });
    observed.await.unwrap();
    job.abort(); assert!(job.await.unwrap_err().is_cancelled());
    assert!(startup.memory_reserved() > baseline);
    assert!(matches!(store.retire(generation).await, Err(Error::Busy)));
    let close = startup.clone();
    let shutdown = tokio::spawn(async move { close.shutdown().await });
    tokio::time::sleep(Duration::from_millis(30)).await;
    assert!(!shutdown.is_finished(), "shutdown awaits the real startup worker");
    finish.send(()).unwrap();
    tokio::time::timeout(Duration::from_secs(10), shutdown).await.unwrap().unwrap().unwrap();
    assert!(startup.guard().is_lost());
    assert!(matches!(startup.guard().prepare_cpu(|_| Ok(())).await, Err(Error::State)));
    assert_eq!(startup.memory_reserved(), baseline, "discarded startup work releases its reservations");

    // A valid-width replacement view cannot masquerade as the generated identity projection.
    let view = format!("{}.serving_members", generation.schema());
    let original: String = sqlx::query_scalar("SELECT pg_get_viewdef(to_regclass($1),false)")
        .bind(&view)
        .fetch_one(&db.superuser)
        .await
        .unwrap();
    sqlx::query(sqlx::AssertSqlSafe(format!(
        "CREATE OR REPLACE VIEW {view} AS SELECT * FROM {}.catalog_members WHERE false",
        generation.schema()
    )))
    .execute(&db.superuser)
    .await
    .unwrap();
    assert!(
        GenerationService::admit(model.clone(), &role, Some(generation))
            .await
            .is_err()
    );
    sqlx::query(sqlx::AssertSqlSafe(format!(
        "CREATE OR REPLACE VIEW {view} AS {original}"
    )))
    .execute(&db.superuser)
    .await
    .unwrap();
    let healthy = GenerationService::admit(model, &role, Some(generation))
        .await
        .unwrap();
    let guard = healthy.guard();
    let (high, low) = {
        let bytes = (0..16)
            .map(|i| u8::from_str_radix(&generation.hex()[2 * i..2 * i + 2], 16).unwrap())
            .collect::<Vec<_>>();
        let key = u64::from_le_bytes(bytes[..8].try_into().unwrap());
        ((key >> 32) as i64, (key & 0xffff_ffff) as i64)
    };
    let pid: i32 = sqlx::query_scalar("SELECT pid FROM pg_locks WHERE locktype='advisory' AND granted AND mode='ShareLock' AND objsubid=1 AND classid::bigint=$1 AND objid::bigint=$2")
        .bind(high).bind(low).fetch_one(&db.superuser).await.unwrap();
    sqlx::query("SELECT pg_terminate_backend($1)")
        .bind(pid)
        .execute(&db.superuser)
        .await
        .unwrap();
    assert!(guard.check().await.is_err());
    assert!(healthy.execution().await.is_err());
    // Even another independent canonical lease cannot revive this process's guard.
    let replacement = store
        .pin(
            &db.reader,
            generation,
            lctx_model::domain::resources::ResourceBudget::fixed(32 << 20).unwrap(),
        )
        .await
        .unwrap();
    assert!(healthy.execution().await.is_err());
    replacement.release().await.unwrap();
    let _ = healthy.shutdown().await;
}
