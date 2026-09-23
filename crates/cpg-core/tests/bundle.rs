//! Stage G (DESIGN §6.4; ADR-0017, ADR-0019): a serving generation from a published snapshot.
//! Rebuilding it gives the same bytes. A changed file is refused. One generation never mixes
//! vector spaces. The serving schema digests are the known answers Python checks too.

use std::path::{Path, PathBuf};

use cpg_core::analyze::Analysis;
use cpg_core::attempt::compile_analyzed;
use cpg_core::bundle::{build, bundle, verify};
use cpg_core::snapshot::published;
use cpg_core::sql;
use cpg_extract::{ExtractInput, TestHooks, extract};
use cpg_schema::id::Id;
use lctx_analytics::config::AnalyticsConfig;

const CONFIG: &str = r#"
version = 1
[subsystem]
module_prefixes = ["pkg.server", "pkg.helpers", "pkg.handlers", "pkg.boot", "pkg.shadow", "pkg.controls"]
public_roots = ["pkg"]
[seeds]
primary = ["pkg.Server.tool"]
distractors = ["pkg.helper", "pkg.Server.route", "pkg.describe", "pkg.configure"]
[pass_a]
max_depth = 2
max_vertices = 128
max_edges = 512
max_witnesses = 3
[briefs]
budget = 6
"#;

const SNAPSHOT: Id = Id([7; 16]);

fn copy(src: &Path, dst: &Path) {
    std::fs::create_dir_all(dst).unwrap();
    for e in std::fs::read_dir(src).unwrap() {
        let p = e.unwrap().path();
        let t = dst.join(p.file_name().unwrap());
        if p.is_dir() {
            copy(&p, &t);
        } else {
            std::fs::copy(&p, &t).unwrap();
        }
    }
}

/// `analysis_shapes` compiled under `sub` with the fake embedder; returns the directory and store.
async fn compiled(sub: &str, reverse: bool) -> (tempfile::TempDir, PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let base = dir.path().join(sub);
    let fixture =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/python/analysis_shapes");
    copy(&fixture.join("release"), &base.join("release"));
    copy(&fixture.join("site"), &base.join("venv/site-packages"));
    let out = extract(&ExtractInput {
        release: cpg_extract::Release::from_tree(
            std::fs::canonicalize(base.join("release")).unwrap(),
            "analysis_shapes",
        )
        .unwrap(),
        venv_root: std::fs::canonicalize(base.join("venv")).unwrap(),
        site_packages: vec![std::fs::canonicalize(base.join("venv/site-packages")).unwrap()],
        python_version: (3, 14, 0),
        python_platform: "linux".to_owned(),
        snapshot_id: SNAPSHOT,
        corpus: None,
        keep_pysa_json: false,
        test_hooks: TestHooks {
            reverse_module_order: reverse,
            ..Default::default()
        },
    })
    .unwrap();
    let analysis = Analysis {
        config: AnalyticsConfig::parse(CONFIG).unwrap(),
        embedder: Some(std::sync::Arc::new(cpg_core::embed::FakeEmbedder::new())),
    };
    let store = dir.path().join("store");
    compile_analyzed(&store, SNAPSHOT, &out.tables, Some(&analysis))
        .await
        .unwrap();
    (dir, store)
}

fn files_of(dir: &Path) -> Vec<(String, Vec<u8>)> {
    let mut out: Vec<(String, Vec<u8>)> = std::fs::read_dir(dir)
        .unwrap()
        .map(|e| {
            let p = e.unwrap().path();
            (
                p.file_name().unwrap().to_string_lossy().into_owned(),
                std::fs::read(&p).unwrap(),
            )
        })
        .collect();
    out.sort();
    out
}

/// §6.4: rebuilding a generation from the store gives byte-identical files. A second compile of the
/// same library, from another location and module order, gives the same generation.
#[tokio::test(flavor = "multi_thread")]
async fn a_generation_rebuilds_to_the_same_bytes() {
    let (dir, store) = compiled("one", false).await;
    let a = bundle(&store, SNAPSHOT, &dir.path().join("gen-a"))
        .await
        .unwrap();
    let b = bundle(&store, SNAPSHOT, &dir.path().join("gen-b"))
        .await
        .unwrap();
    assert_eq!(a.key, b.key);
    assert_eq!(files_of(&a.dir), files_of(&b.dir));
    // Building into an existing generation of the same key is idempotent.
    let again = bundle(&store, SNAPSHOT, &dir.path().join("gen-a"))
        .await
        .unwrap();
    assert_eq!(again.key, a.key);
    let manifest = verify(&a.dir).unwrap();
    let rows = |f: &str| manifest["files"][f]["rows"].as_u64().unwrap();
    assert_eq!(rows("briefs"), 5);
    assert_eq!(rows("vectors"), 5);
    assert_eq!(rows("lexical_text"), 5);
    assert_eq!(rows("embedding_spec"), 1);
    assert!(
        rows("symbol_map") >= 8,
        "every public access path of each brief"
    );
    assert_eq!(
        manifest["summary"]["briefs"]["unreviewed"]["documentation_only"],
        1
    );
    assert_eq!(manifest["spec_hash"].as_str().unwrap().len(), 64);
    let names: Vec<&String> = manifest["files"].as_object().unwrap().keys().collect();
    assert_eq!(
        names,
        [
            "assertions",
            "brief_members",
            "briefs",
            "embedding_spec",
            "evidence",
            "lexical_text",
            "supports",
            "symbol_map",
            "vectors"
        ]
    );

    let (other_dir, other_store) = compiled("elsewhere", true).await;
    let c = bundle(&other_store, SNAPSHOT, &other_dir.path().join("gen"))
        .await
        .unwrap();
    assert_eq!(c.key, a.key, "the same library, compiled again elsewhere");
    assert_eq!(files_of(&c.dir), files_of(&a.dir));
}

/// A changed file, or a generation under another name, is refused against its manifest.
#[tokio::test(flavor = "multi_thread")]
async fn a_changed_generation_is_refused() {
    let (dir, store) = compiled("one", false).await;
    let g = bundle(&store, SNAPSHOT, &dir.path().join("gen"))
        .await
        .unwrap();
    let renamed = dir.path().join("gen").join("0000000000000000");
    copy(&g.dir, &renamed);
    let err = verify(&renamed).unwrap_err().to_string();
    assert!(err.contains("generation key"), "{err}");

    let path = g.dir.join("briefs.arrow");
    let mut bytes = std::fs::read(&path).unwrap();
    let last = bytes.len() - 20;
    bytes[last] ^= 0xff;
    std::fs::write(&path, bytes).unwrap();
    let err = verify(&g.dir).unwrap_err().to_string();
    assert!(err.contains("briefs.arrow: its sha256"), "{err}");
}

/// §6.4: a generation never mixes vector spaces: two specs in the snapshot refuse the build.
#[tokio::test(flavor = "multi_thread")]
async fn mixed_embedding_specs_are_refused() {
    let (dir, store) = compiled("one", false).await;
    let (_, ctx) = published(&store, SNAPSHOT).await.unwrap().unwrap();
    let doctored = sql::query(
        &ctx,
        "SELECT * FROM embedding_specs UNION ALL \
         SELECT snapshot_id, CAST(X'00000000000000000000000000000000000000000000000000000000000000ff' \
                AS BYTEA) AS spec_hash, spec FROM embedding_specs",
    )
    .await
    .unwrap()
    .into_view();
    ctx.deregister_table("embedding_specs").unwrap();
    ctx.register_table("embedding_specs", doctored).unwrap();
    let err = build(&ctx, &dir.path().join("gen"))
        .await
        .unwrap_err()
        .to_string();
    assert!(err.contains("mixed embedding specs"), "{err}");
}

/// The serving schema digests are known answers shared with Python (`specs/serving/`, ADR-0019):
/// Python recomputes each from a generation's own files. `LCTX_WRITE_KNOWN_ANSWERS=1` rewrites
/// the file after a declared change of a served schema.
#[test]
fn serving_schema_digests_are_the_shared_known_answers() {
    let path =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../specs/serving/schema_digests.json");
    let mut answers = serde_json::Map::new();
    for f in cpg_schema::bundle::files(4096) {
        answers.insert(
            f.name.to_owned(),
            serde_json::json!({
                "form": cpg_schema::bundle::canonical_form(&f.schema).unwrap(),
                "digest": cpg_schema::bundle::schema_digest(&f.schema).unwrap(),
            }),
        );
    }
    let text = serde_json::to_string_pretty(&serde_json::Value::Object(answers)).unwrap() + "\n";
    if std::env::var_os("LCTX_WRITE_KNOWN_ANSWERS").is_some() {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, &text).unwrap();
    }
    assert_eq!(
        std::fs::read_to_string(&path).unwrap(),
        text,
        "a served schema changed: rewrite the known answers deliberately"
    );
}

/// The generation the Python tests serve (`just py-fixture`, DESIGN §11.3): `analysis_shapes` with
/// the fake embedder, built into `$LCTX_PY_FIXTURE`, its key in `CURRENT`. A no-op without it.
#[tokio::test(flavor = "multi_thread")]
async fn writes_the_python_fixture_generation() {
    let Some(out) = std::env::var_os("LCTX_PY_FIXTURE") else {
        return;
    };
    let out = PathBuf::from(out);
    let (_dir, store) = compiled("fixture", false).await;
    let g = bundle(&store, SNAPSHOT, &out).await.unwrap();
    std::fs::write(out.join("CURRENT"), &g.key).unwrap();
}
