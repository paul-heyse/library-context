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
    for profile in ["catalog","behavioral"] {
    sqlx::query("DELETE FROM lctx_cache.embedding_values").execute(db.owner.pool()).await.unwrap();
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
        if profile=="behavioral" {
            let facets:i64=sqlx::query_scalar(sqlx::AssertSqlSafe(format!(
                "SELECT count(*) FROM {}.synthesis_summary_facets f JOIN {}.summary_claims c ON c.id=f.claim JOIN {}.summary_symbolic_field_alternatives a ON a.id=c.symbolicfieldassociation_alternative WHERE f.verdict=3 AND f.source IS NULL AND f.qualification=a.reader_qualification",generation.id.schema(),generation.id.schema(),generation.id.schema())))
                .fetch_one(db.owner.pool()).await.unwrap();
            assert_eq!(facets,4,"three source readers retain four guarded sink outcomes without finding authority");
            let readers:i64=sqlx::query_scalar(sqlx::AssertSqlSafe(format!(
                "SELECT count(DISTINCT link) FROM {}.summary_symbolic_field_alternatives",generation.id.schema())))
                .fetch_one(db.owner.pool()).await.unwrap();
            assert_eq!(readers,3);
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
