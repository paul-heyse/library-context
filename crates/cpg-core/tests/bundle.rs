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
use cpg_schema::codebook::{Codebook, SummaryFlowStepKind};
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
async fn compiled(sub: &str, reverse: bool, finalizer: bool) -> (tempfile::TempDir, PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let base = dir.path().join(sub);
    let fixture =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/python/analysis_shapes");
    copy(&fixture.join("release"), &base.join("release"));
    copy(&fixture.join("site"), &base.join("venv/site-packages"));
    if finalizer {
        let path = base.join("release/pkg/controls.py");
        let source = std::fs::read_to_string(&path).unwrap();
        let before = "def passthrough(**options):\n    return options";
        let after = "def passthrough(**options):\n    try:\n        return options\n    finally:\n        pass";
        assert!(source.contains(before));
        std::fs::write(path, source.replace(before, after)).unwrap();
    }
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
        techniques: Default::default(),
    };
    let store = dir.path().join("store");
    compile_analyzed(&store, SNAPSHOT, &out.tables, Some(&analysis))
        .await
        .unwrap();
    (dir, store)
}

/// A real local-call chain reaches the finite composition depth cap.
async fn compiled_summary_caps() -> (tempfile::TempDir, PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let base = dir.path().join("caps");
    let fixture = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/python/summary_caps");
    copy(&fixture, &base.join("release"));
    std::fs::create_dir_all(base.join("venv/site-packages")).unwrap();
    let out = extract(&ExtractInput {
        release: cpg_extract::Release::from_tree(
            std::fs::canonicalize(base.join("release")).unwrap(), "summary_caps",
        ).unwrap(),
        venv_root: std::fs::canonicalize(base.join("venv")).unwrap(),
        site_packages: vec![std::fs::canonicalize(base.join("venv/site-packages")).unwrap()],
        python_version: (3, 14, 7),
        python_platform: "linux".to_owned(),
        snapshot_id: SNAPSHOT,
        corpus: None,
        keep_pysa_json: false,
        test_hooks: TestHooks::default(),
    }).unwrap();
    let config = AnalyticsConfig::parse(r#"
version = 1
[subsystem]
module_prefixes = ["capspkg"]
public_roots = ["capspkg"]
[seeds]
primary = ["capspkg.f9"]
distractors = ["capspkg.f0", "capspkg.unsupported"]
[pass_a]
max_depth = 2
max_vertices = 128
max_edges = 512
max_witnesses = 3
[briefs]
budget = 3
"#).unwrap();
    let analysis = cpg_core::analyze::Analysis {
        config,
        embedder: Some(std::sync::Arc::new(cpg_core::embed::FakeEmbedder::new())),
        techniques: Default::default(),
    };
    let store = dir.path().join("store");
    compile_analyzed(&store, SNAPSHOT, &out.tables, Some(&analysis)).await.unwrap();
    (dir, store)
}

#[tokio::test(flavor = "multi_thread")]
async fn finite_depth_and_unsupported_refusals_reach_the_native_response() {
    let (dir, store) = compiled_summary_caps().await;
    let (_, ctx) = published(&store, SNAPSHOT).await.unwrap().unwrap();
    for (name, reason) in [("f9", 21), ("unsupported", 4)] {
        let rows = sql::query(&ctx, &format!("SELECT count(*) AS n FROM summary_boundaries b \
            JOIN declarations d ON d.node_id = b.function_node_id \
            WHERE d.name = '{name}' AND b.reason = {reason}"))
            .await.unwrap().collect().await.unwrap();
        let counts = rows[0].column(0)
            .as_any().downcast_ref::<datafusion::arrow::array::Int64Array>().unwrap();
        assert_eq!(counts.value(0), 1, "{name} must retain its typed boundary");
    }
    for (name, expected) in [("completed_predecessor", 1), ("parameter_predecessor", 1),
        ("raising_predecessor", 0), ("possibly_unbound_argument", 0),
        ("conditional_callee", 0), ("guarded_module_callee", 0)] {
        let rows = sql::query(&ctx, &format!(
            "SELECT count(*) AS n FROM model_applications a \
             JOIN declarations d ON d.node_id = a.function_node_id \
             WHERE d.name = '{name}' AND a.target_normal_return \
               AND a.target_count = 1 AND a.candidate_set_complete_under_model \
               AND NOT a.has_unresolved_remainder",
        )).await.unwrap().collect().await.unwrap();
        let counts = rows[0].column(0)
            .as_any().downcast_ref::<datafusion::arrow::array::Int64Array>().unwrap();
        if name != "conditional_callee" && name != "guarded_module_callee" {
            assert_eq!(counts.value(0), 1, "{name} must have a closed normal-return target");
        }
        let rows = sql::query(&ctx, &format!(
            "SELECT count(*) AS n FROM summary_flows f \
             JOIN declarations d ON d.node_id = f.function_node_id \
             JOIN summary_flow_steps p ON p.summary_id = f.summary_id \
               AND p.kind = {} \
             WHERE d.name = '{name}' AND f.path_depth = 0",
            SummaryFlowStepKind::PrecedingCallNormal.code(),
        )).await.unwrap().collect().await.unwrap();
        let counts = rows[0].column(0)
            .as_any().downcast_ref::<datafusion::arrow::array::Int64Array>().unwrap();
        assert_eq!(counts.value(0), expected, "{name} normal predecessor admission");
    }
    let rows = sql::query(&ctx, "SELECT count(*) AS n FROM summary_boundaries b \
        JOIN declarations d ON d.node_id = b.function_node_id \
        WHERE d.name IN ('raising_predecessor', 'possibly_unbound_argument', 'conditional_callee', \
          'guarded_module_callee') AND b.reason = 4")
        .await.unwrap().collect().await.unwrap();
    let counts = rows[0].column(0)
        .as_any().downcast_ref::<datafusion::arrow::array::Int64Array>().unwrap();
    assert_eq!(counts.value(0), 4, "uncertain arguments and callees must stay unknown");
    let generation = bundle(&store, SNAPSHOT, &dir.path().join("generations"))
        .await.unwrap();
    let script = r#"
import sys
from pathlib import Path
from lctx_mcp.generation import load
generation = load(Path(sys.argv[1]), None)
index = generation.condition_graph
for operation, expected in (
    ("capspkg.f9", "summary_depth_limit"),
    ("capspkg.unsupported", "unsupported_control_flow"),
    ("capspkg.raising_predecessor", "unsupported_control_flow"),
    ("capspkg.possibly_unbound_argument", "unsupported_control_flow"),
    ("capspkg.conditional_callee", "unsupported_control_flow"),
    ("capspkg.guarded_module_callee", "unsupported_control_flow"),
):
    paths, boundaries, truncated, _ = index.value_paths(operation, "value", 20)
    assert not truncated, (operation, paths, boundaries)
    assert any(boundary[2] == expected for boundary in boundaries), (operation, paths, boundaries)
for operation in ("capspkg.completed_predecessor", "capspkg.parameter_predecessor"):
    paths, boundaries, truncated, _ = index.value_paths(operation, "value", 20)
    assert not truncated and not boundaries, (operation, paths, boundaries)
    assert any(any(step[0] == "preceding_call_normal" for step in path[3]) for path in paths), paths
"#;
    let output = std::process::Command::new("uv")
        .args(["run", "--no-sync", "python", "-c", script,
            generation.dir.to_str().unwrap()])
        .current_dir(Path::new(env!("CARGO_MANIFEST_DIR")).join("../.."))
        .output().unwrap();
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    let original_steps = sql::query(&ctx, "SELECT * FROM summary_flow_steps")
        .await.unwrap().into_view();
    let omitted_completion = sql::query(&ctx, &format!(
        "SELECT * FROM summary_flow_steps WHERE kind <> {}",
        SummaryFlowStepKind::PrecedingCallNormal.code(),
    )).await.unwrap().into_view();
    ctx.deregister_table("summary_flow_steps").unwrap();
    ctx.register_table("summary_flow_steps", omitted_completion).unwrap();
    let violations = cpg_core::validate::validate(&ctx).await.unwrap();
    assert!(violations.iter().any(|v| v.rule == "summary-flow-step-source-equality"),
        "missing normal-completion witness passed validation: {violations:?}");
    ctx.deregister_table("summary_flow_steps").unwrap();
    ctx.register_table("summary_flow_steps", original_steps).unwrap();
}

/// W1: a real finalizer proof survives extraction, Delta publication, IPC generation and the
/// native reader. The Python process only loads the generation and issues the semantic query.
#[tokio::test(flavor = "multi_thread")]
async fn finalizer_proof_round_trips_through_the_native_generation_reader() {
    let (dir, store) = compiled("finalizer", false, true).await;
    let generation = bundle(&store, SNAPSHOT, &dir.path().join("generations"))
        .await
        .unwrap();
    let script = r#"
import sys
from pathlib import Path
from lctx_mcp.generation import GenerationError, _validate_support_closure, load
from lctx_mcp.server import hydrate, markdown, serve
generation = load(Path(sys.argv[1]), None)
index = generation.condition_graph
paths, boundaries, truncated, _ = index.value_paths(
    "pkg.controls.passthrough", "options", 20
)
assert not truncated and not boundaries, (paths, boundaries)
assert any(
    path[1] in ("established", "conditional")
    and any(step[0] == "finalizer_pass" for step in path[3])
    for path in paths
), paths
served = serve(generation, None)
brief_id = next(
    brief_id for brief_id, brief in generation.briefs.items()
    if brief["access_path"] == "pkg.configure"
)
capability = hydrate(served, generation.snapshot_id, brief_id.hex())
coordinates = next(a for a in capability.assertions if a.kind == "coordinates")
finding = next(s.finding for s in coordinates.supports if s.finding is not None)
assert finding.model_id and finding.invocation_id and finding.witnesses, finding
assert finding.source_resolution == "source_span", finding
source = finding.witnesses[0]
assert source.source_path.endswith("pkg/controls.py") and source.end_byte > source.start_byte
rendered = markdown(capability)
assert finding.finding_id in rendered
assert finding.invocation_id in rendered and finding.model_id in rendered
assert f"{source.source_path}:{source.start_byte}-{source.end_byte}" in rendered
broken = dict(generation.tables)
broken["support_findings"] = broken["support_findings"].slice(1)
try:
    _validate_support_closure(broken)
except GenerationError:
    pass
else:
    raise AssertionError("a missing finding closure was served")
"#;
    let output = std::process::Command::new("uv")
        .args(["run", "--no-sync", "python", "-c", script])
        .arg(&generation.dir)
        .current_dir(Path::new(env!("CARGO_MANIFEST_DIR")).join("../.."))
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "native finalizer query failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
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
    let (dir, store) = compiled("one", false, false).await;
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
    // ADR-0010's R2 F1 amendment: a brief's lexical names are its seed's own spellings.
    // `pkg.Server.tool` is also spelled `pkg.Alpha.tool` through a public subclass, which promotes
    // the brief (`symbol_map`) but names nothing in its lexical text.
    let served = |file: &str| {
        let bytes = std::fs::read(a.dir.join(file)).unwrap();
        let reader =
            arrow_ipc::reader::FileReader::try_new(std::io::Cursor::new(bytes), None).unwrap();
        reader.map(|b| b.unwrap()).collect::<Vec<_>>()
    };
    let strings = |b: &arrow_array::RecordBatch, column: &str| -> Vec<String> {
        let c = b.column_by_name(column).unwrap();
        let c = c
            .as_any()
            .downcast_ref::<arrow_array::StringArray>()
            .unwrap();
        (0..arrow_array::Array::len(c))
            .map(|i| c.value(i).to_owned())
            .collect()
    };
    let symbols: Vec<String> = served("symbol_map.arrow")
        .iter()
        .flat_map(|b| strings(b, "symbol"))
        .collect();
    assert!(
        symbols.contains(&"pkg.Alpha.tool".to_owned()),
        "{symbols:?}"
    );
    let names: Vec<String> = served("lexical_text.arrow")
        .iter()
        .flat_map(|b| strings(b, "text"))
        .map(|t| t.rsplit('\n').next().unwrap_or_default().to_owned())
        .collect();
    assert!(
        names.iter().any(|n| n.split(' ').any(|t| t == "tool")),
        "{names:?}"
    );
    assert!(
        !names.iter().any(|n| n.split(' ').any(|t| t == "alpha")),
        "an inherited spelling named a brief: {names:?}"
    );
    let rows = |f: &str| manifest["files"][f]["rows"].as_u64().unwrap();
    // Five configured seeds; the fixture has no official usage code, so nothing is eligible to
    // fill the budget of six (the increment-2 review's U1).
    assert_eq!(rows("briefs"), 5);
    assert_eq!(rows("vectors"), 5);
    assert_eq!(rows("lexical_text"), 5);
    assert_eq!(rows("embedding_spec"), 1);
    assert!(
        rows("symbol_map") >= 8,
        "every public access path of each brief"
    );
    // `pkg.describe`, which calls nothing.
    assert_eq!(
        manifest["summary"]["briefs"]["unreviewed"]["documentation_only"],
        1
    );
    assert_eq!(manifest["spec_hash"].as_str().unwrap().len(), 64);
    assert_eq!(manifest["condition_kernel_format"], 1);
    let names: Vec<&String> = manifest["files"].as_object().unwrap().keys().collect();
    assert_eq!(
        names,
        [
            "ambient_reads",
            "analysis_condition_nodes",
            "analysis_conditions",
            "assertions",
            "behaviors",
            "brief_members",
            "briefs",
            "condition_nodes",
            "conditions",
            "embedding_spec",
            "evidence",
            "flow_test_leaves",
            "flow_test_value_links",
            "lexical_text",
            "operation_facet_status",
            "operation_facets",
            "operation_parameters",
            "operation_text",
            "operation_vectors",
            "operations",
            "place_claims",
            "public_paths",
            "singletons",
            "summary_boundaries",
            "summary_flow_steps",
            "summary_flows",
            "support_findings",
            "support_members",
            "support_witnesses",
            "supports",
            "symbol_map",
            "vectors"
        ]
    );

    let (other_dir, other_store) = compiled("elsewhere", true, false).await;
    let c = bundle(&other_store, SNAPSHOT, &other_dir.path().join("gen"))
        .await
        .unwrap();
    assert_eq!(c.key, a.key, "the same library, compiled again elsewhere");
    assert_eq!(files_of(&c.dir), files_of(&a.dir));
}

/// A changed file, or a generation under another name, is refused against its manifest.
#[tokio::test(flavor = "multi_thread")]
async fn a_changed_generation_is_refused() {
    let (dir, store) = compiled("one", false, false).await;
    let g = bundle(&store, SNAPSHOT, &dir.path().join("gen"))
        .await
        .unwrap();
    let renamed = dir.path().join("gen").join("0000000000000000");
    copy(&g.dir, &renamed);
    let err = verify(&renamed).unwrap_err().to_string();
    assert!(err.contains("generation key"), "{err}");
    let manifest_path = renamed.join("MANIFEST.json");
    let mut manifest: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&manifest_path).unwrap()).unwrap();
    manifest["files"]["conditions"]["file"] = serde_json::json!("../conditions.arrow");
    std::fs::write(&manifest_path, serde_json::to_vec(&manifest).unwrap()).unwrap();
    let err = verify(&renamed).unwrap_err().to_string();
    assert!(err.contains("unexpected served file path"), "{err}");

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
    let (dir, store) = compiled("one", false, false).await;
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
    let (_dir, store) = compiled("fixture", false, false).await;
    let g = bundle(&store, SNAPSHOT, &out).await.unwrap();
    std::fs::write(out.join("CURRENT"), &g.key).unwrap();
}
