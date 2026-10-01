// DORMANT (cutover plan P1.3): not a test target. It drives the removed Delta runtime
// (`snapshot`/`attempt`/`delta`), so it cannot compile until its layer is rebuilt on generations.
// Owner: P3–P5 by layer; its raw-level answers are re-homed by B3. Its known answers stay here as expectations to re-express, never to reuse as-is.
//! Slice 2: a compile attempt writes the raw tables, derives Stage C/D, validates and publishes
//! (DESIGN §4.1, §6, §8; ADR-0009).

use std::path::Path;
use std::sync::Arc;

use arrow_array::{
    Array, ArrayRef, FixedSizeBinaryArray, Int16Array, Int64Array, RecordBatch, UInt32Array,
};
use cpg_core::CoreError;
use cpg_core::analyze::{Analysis, Techniques};
use cpg_core::attempt::{compile, compile_analyzed, publish};
use cpg_core::delta::read_at;
use cpg_core::snapshot::{published, resolve};
use cpg_core::sql;
use cpg_extract::{ExtractInput, extract};
use cpg_schema::behavior::{
    FlowTestExactOriginsRow, FlowTestValueLinksRow, ModelCallbacks, ModelCallbacksRow,
    ModelEffects, ModelEffectsRow, ModelExceptions, ModelExceptionsRow, ModelResources,
    ModelResourcesRow, ModelTargets, ModelTargetsRow, ModelTransfers, ModelTransfersRow,
};
use cpg_schema::codebook::{
    BoundaryReason, Codebook, DefinitionKind, ExitSiteKind, FlowCallLinkStatus,
    FlowCallOperandRole, FlowSink, HandlerTypeStatus, ImplicitReceiver, Modality,
    ModelArgumentStatus, ModelCallbackAction, ModelChannelCoverage, ModelEffectKind,
    ModelEffectSubjectStatus, ModelExceptionAction, ModelExit, ModelPathKind, ModelPathRole,
    ModelResourceAction, ModelResourceSourceStatus, ModelTransferEndpointStatus, ModelTransferKind,
    ModeledArgumentEvaluationStatus, ModeledHandlerClassMatch, Origin, SummaryFlowStepKind,
    Verdict,
};
use cpg_schema::condition::Value;
use cpg_schema::condition_kernel::{ConditionRoot, DiagramNode, hydrate_catalog};
use cpg_schema::id::Id;
use cpg_schema::query::Relation;
use cpg_schema::table::Table;
use cpg_schema::tables::{
    ConditionNodesRow, ConditionsRow, Declarations, FlowTestLeavesRow, SnapshotsRow,
};
use datafusion::arrow::util::pretty::pretty_format_batches;
use datafusion::prelude::SessionContext;
use lctx_analytics::config::AnalyticsConfig;

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

/// The extractor's raw batches for a fixture, under `snapshot_id`.
fn raw(fixture: &str, snapshot_id: Id) -> Vec<(&'static str, RecordBatch)> {
    raw_version(fixture, snapshot_id, (3, 14, 0))
}

fn raw_version(
    fixture: &str,
    snapshot_id: Id,
    python_version: (u32, u32, u32),
) -> Vec<(&'static str, RecordBatch)> {
    let dir = tempfile::tempdir().unwrap();
    let repo = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let release = dir.path().join("release");
    copy(&repo.join("fixtures/python").join(fixture), &release);
    let site = dir.path().join("venv/site-packages");
    std::fs::create_dir_all(&site).unwrap();
    extract(&ExtractInput {
        profile: cpg_schema::catalog::CompileProfile::Behavioral,
        release: cpg_extract::Release::from_tree(std::fs::canonicalize(&release).unwrap(), fixture)
            .unwrap(),
        venv_root: std::fs::canonicalize(dir.path().join("venv")).unwrap(),
        site_packages: vec![std::fs::canonicalize(&site).unwrap()],
        python_version,
        python_platform: "linux".to_owned(),
        snapshot_id,
        corpus: None,
        keep_pysa_json: false,
        test_hooks: Default::default(),
    })
    .unwrap()
    .tables
}

/// Semantic validator cases explicitly request enrichment; raw publication defaults to catalog.
async fn compile_behavior(
    root: &Path,
    snapshot: Id,
    raw: &[(&'static str, RecordBatch)],
    seed: &str,
) {
    let declarations = &raw
        .iter()
        .find(|(name, _)| *name == "declarations")
        .unwrap()
        .1;
    let strings = |name: &str| {
        declarations
            .column_by_name(name)
            .unwrap()
            .as_any()
            .downcast_ref::<arrow_array::StringArray>()
            .unwrap()
    };
    let row = strings("name")
        .iter()
        .position(|name| name == Some(seed))
        .unwrap();
    let qualified = strings("qualified_name").value(row);
    let module = qualified.rsplit_once('.').unwrap().0;
    let analysis = Analysis {
        embedding_cache: None,
        config: AnalyticsConfig::parse(&format!(
            r#"
version = 1
[subsystem]
module_prefixes = ["{module}"]
public_roots = ["{module}"]
[seeds]
primary = ["{qualified}"]
distractors = []
[pass_a]
max_depth = 2
max_vertices = 128
max_edges = 512
max_witnesses = 3
[briefs]
budget = 1
"#
        ))
        .unwrap(),
        embedder: None,
        techniques: Techniques::default(),
    };
    compile_analyzed(root, snapshot, raw, Some(&analysis))
        .await
        .unwrap();
}

async fn text(ctx: &SessionContext, statement: &str) -> String {
    let batches = sql::query(ctx, statement)
        .await
        .unwrap()
        .collect()
        .await
        .unwrap();
    pretty_format_batches(&batches).unwrap().to_string()
}

async fn count(ctx: &SessionContext, statement: &str) -> i64 {
    let batches = sql::query(ctx, statement)
        .await
        .unwrap()
        .collect()
        .await
        .unwrap();
    let a = batches[0]
        .column(0)
        .as_any()
        .downcast_ref::<arrow_array::Int64Array>()
        .unwrap();
    a.value(0)
}

#[tokio::test(flavor = "multi_thread")]
async fn transfer_alternatives_keep_conditions_verdicts_and_receiver_boundaries() {
    let snapshot = Id([103; 16]);
    let mut inputs = raw("transfer_alternatives", snapshot);
    let analysis = Analysis {
        embedding_cache: None,
        config: AnalyticsConfig::parse(
            "version = 1\n[subsystem]\nmodule_prefixes = [\"transferpkg\"]\n\
             public_roots = [\"transferpkg\"]\n[seeds]\nprimary = [\"transferpkg.mixed\"]\n\
             distractors = []\n[pass_a]\nmax_depth = 2\nmax_vertices = 128\n\
             max_edges = 512\nmax_witnesses = 3\n[briefs]\nbudget = 1\n",
        )
        .unwrap(),
        embedder: Some(Arc::new(cpg_core::embedding_service::FakeEmbedder::new())),
        techniques: Techniques::default(),
    };
    let first = tempfile::tempdir().unwrap();
    compile_analyzed(first.path(), snapshot, &inputs, Some(&analysis))
        .await
        .unwrap();
    let (_, ctx) = published(first.path(), snapshot).await.unwrap().unwrap();
    assert_eq!(
        count(
            &ctx,
            "SELECT count(*) FROM value_flows v \
        JOIN declarations d ON d.node_id = v.function_node_id \
        WHERE d.qualified_name = 'transferpkg.mixed' AND v.source_name = 'value' \
          AND v.sink = 2"
        )
        .await,
        3
    );
    let alternatives = text(
        &ctx,
        "SELECT b.transfer, b.verdict, b.boundary_reason, b.condition \
        FROM behaviors b JOIN operations o ON o.node_id = b.operation_node_id \
        WHERE o.access_path = 'transferpkg.mixed' AND b.parameter_name = 'value' \
          AND b.kind = 11 ORDER BY b.transfer",
    )
    .await;
    assert!(alternatives.contains("| 0        | 1"), "{alternatives}");
    assert!(alternatives.contains("| 1        | 1"), "{alternatives}");
    assert!(
        alternatives.contains("| 2        | 3       | 19"),
        "{alternatives}"
    );
    assert_eq!(
        count(
            &ctx,
            "SELECT count(*) FROM behaviors b \
        JOIN operations o ON o.node_id = b.operation_node_id \
        WHERE o.access_path = 'transferpkg.RecordHolder.__init__' AND b.depth = 2 \
          AND b.kind = 11 AND b.parameter_name = 'value' \
          AND b.condition_scope_node_id <> b.operation_node_id \
          AND b.verdict = 3 AND b.boundary_reason IN (5, 19)"
        )
        .await,
        3
    );
    assert_eq!(
        count(
            &ctx,
            "SELECT count(*) FROM behaviors b JOIN operations o \
        ON o.node_id = b.operation_node_id WHERE o.access_path = 'transferpkg.Holder.__init__' \
        AND b.depth = 2"
        )
        .await,
        0
    );
    assert_eq!(count(&ctx, "SELECT count(*) FROM behaviors b JOIN operations o \
        ON o.node_id = b.operation_node_id WHERE o.access_path = 'transferpkg.ConditionalHolder.__init__' \
        AND b.depth = 2 AND b.verdict IN (0, 1)").await, 0);
    assert_eq!(
        count(
            &ctx,
            "SELECT count(*) FROM behaviors b JOIN operations o \
        ON o.node_id = b.operation_node_id WHERE o.access_path = 'transferpkg.captured' \
        AND b.parameter_name = 'value' AND b.verdict IN (0, 1)"
        )
        .await,
        0
    );
    assert!(
        count(
            &ctx,
            "SELECT count(*) FROM behaviors b JOIN operations o \
        ON o.node_id = b.operation_node_id WHERE o.access_path = 'transferpkg.captured' \
        AND b.parameter_name = 'value' AND b.boundary_reason = 5"
        )
        .await
            > 0
    );
    assert_eq!(
        count(
            &ctx,
            "SELECT count(*) FROM behaviors b JOIN operations o \
        ON o.node_id = b.operation_node_id WHERE o.access_path = 'transferpkg.decorated' \
        AND b.verdict <> 3"
        )
        .await,
        0
    );
    assert_eq!(
        count(
            &ctx,
            "SELECT count(*) FROM operations \
        WHERE access_path = 'transferpkg.decorated_empty' \
          AND behavior_status = 3 AND boundary_reason = 10"
        )
        .await,
        1
    );
    assert_eq!(
        count(
            &ctx,
            "SELECT count(*) FROM behaviors b JOIN operations o \
        ON o.node_id = b.operation_node_id WHERE o.access_path = 'transferpkg.through_decorated' \
        AND b.kind IN (0, 4) AND b.verdict IN (0, 1)"
        )
        .await,
        0
    );
    // Builtin binding-preserving descriptors run the function's own body: lexical builtin
    // resolution and Pysa's descriptor flag agree, so no decorator boundary is written.
    for access in [
        "transferpkg.Descriptors.build",
        "transferpkg.Descriptors.make",
    ] {
        assert!(
            count(
                &ctx,
                &format!(
                    "SELECT count(*) FROM behaviors b JOIN operations o \
                     ON o.node_id = b.operation_node_id WHERE o.access_path = '{access}' \
                     AND b.kind = 11 AND b.parameter_name = 'value' AND b.verdict IN (0, 1)"
                )
            )
            .await
                > 0,
            "{access}"
        );
    }
    for access in [
        "transferpkg.Descriptors.build",
        "transferpkg.Descriptors.make",
        "transferpkg.Descriptors.shown",
    ] {
        assert_eq!(
            count(
                &ctx,
                &format!(
                    "SELECT count(*) FROM operations o LEFT JOIN behaviors b \
                     ON b.operation_node_id = o.node_id WHERE o.access_path = '{access}' \
                     AND (o.boundary_reason = 10 OR b.boundary_reason = 10)"
                )
            )
            .await,
            0,
            "{access}"
        );
    }
    // No path composes through, or starts in, a decorated function (ADR-0064, review F02); the
    // same predicate keeps builtin descriptors' own paths.
    for (access, summaries) in [
        ("transferpkg.through_decorated", 0),
        ("transferpkg.decorated", 0),
        ("transferpkg.Descriptors.stacked", 0),
        ("transferpkg.Shadowed.build", 0),
        ("transferpkg.loose", 0),
        ("transferpkg.Descriptors.make", 1),
    ] {
        assert_eq!(
            count(
                &ctx,
                &format!(
                    "SELECT count(*) FROM summary_flows s JOIN operations o \
                     ON o.node_id = s.function_node_id WHERE o.access_path = '{access}'"
                )
            )
            .await,
            summaries,
            "{access}"
        );
    }
    assert!(
        count(
            &ctx,
            "SELECT count(*) FROM summary_boundaries s JOIN operations o \
             ON o.node_id = s.function_node_id \
             WHERE o.access_path = 'transferpkg.through_decorated' AND s.reason = 10"
        )
        .await
            > 0
    );
    assert_eq!(
        count(
            &ctx,
            "SELECT count(*) FROM behaviors b JOIN operations o \
             ON o.node_id = b.operation_node_id \
             WHERE o.access_path = 'transferpkg.through_decorated' AND b.kind = 11 \
               AND b.verdict IN (0, 1)"
        )
        .await,
        0
    );
    // ADR-0064: a return reached only through a call is established once every member origin the
    // flow model merged is proved, and it cites one summary per member.
    let discharged = |access: &str| {
        format!(
            "SELECT count(*) FROM behaviors b JOIN operations o ON o.node_id = b.operation_node_id \
             JOIN behavior_discharges d ON d.behavior_id = b.behavior_id \
             WHERE o.access_path = '{access}' AND b.kind = 11 AND b.transfer = 2 \
               AND b.verdict IN (0, 1) AND d.decision = 0"
        )
    };
    assert_eq!(
        count(&ctx, &discharged("transferpkg.through_sink")).await,
        1
    );
    // `twice` merges two call contributions into one claim; neither is proved (a conditional
    // expression is not a whole-return call), so the claim stays open with both members cited.
    assert_eq!(count(&ctx, &discharged("transferpkg.twice")).await, 0);
    let twice_members = "SELECT d.origin_id FROM behavior_discharges d \
         JOIN behaviors b ON b.behavior_id = d.behavior_id \
         JOIN operations o ON o.node_id = b.operation_node_id \
         WHERE o.access_path = 'transferpkg.twice'";
    assert_eq!(
        count(&ctx, &format!("SELECT count(*) FROM ({twice_members})")).await,
        2
    );
    // Deleting one member's evidence breaks the sibling closure the publication rule enforces.
    let rule = cpg_schema::rules::rules()
        .into_iter()
        .find(|r| r.name == "semantic:discharge-sibling-closure")
        .expect("the sibling-closure rule");
    assert_eq!(
        count(&ctx, &format!("SELECT count(*) FROM ({})", rule.sql)).await,
        0
    );
    let doctored = sql::query(
        &ctx,
        &format!(
            "SELECT * FROM behavior_discharges \
             WHERE origin_id <> (SELECT min(origin_id) FROM ({twice_members}))"
        ),
    )
    .await
    .unwrap()
    .into_view();
    let original = ctx.table("behavior_discharges").await.unwrap().into_view();
    ctx.deregister_table("behavior_discharges").unwrap();
    ctx.register_table("behavior_discharges", doctored).unwrap();
    assert!(count(&ctx, &format!("SELECT count(*) FROM ({})", rule.sql)).await > 0);
    ctx.deregister_table("behavior_discharges").unwrap();
    ctx.register_table("behavior_discharges", original).unwrap();
    // The exempt descriptor's unread formal is judged like any other body (descriptor review F01).
    assert_eq!(
        count(
            &ctx,
            "SELECT count(*) FROM behaviors b JOIN operations o \
             ON o.node_id = b.operation_node_id \
             WHERE o.access_path = 'transferpkg.Ignoring.ignores' AND b.boundary_reason = 10"
        )
        .await,
        0
    );
    // A second decorator, a class-local name spelled `classmethod`, a descriptor outside a class
    // (descriptor review F02) or a property whose setter owns the path keeps the withholding.
    for access in [
        "transferpkg.Descriptors.stacked",
        "transferpkg.Shadowed.build",
        "transferpkg.loose",
        "transferpkg.Settable.level",
    ] {
        assert_eq!(
            count(
                &ctx,
                &format!(
                    "SELECT count(*) FROM operations WHERE access_path = '{access}' \
                     AND behavior_status = 3 AND boundary_reason = 10"
                )
            )
            .await,
            1,
            "{access}"
        );
        assert_eq!(
            count(
                &ctx,
                &format!(
                    "SELECT count(*) FROM behaviors b JOIN operations o \
                     ON o.node_id = b.operation_node_id WHERE o.access_path = '{access}' \
                     AND b.verdict IN (0, 1)"
                )
            )
            .await,
            0,
            "{access}"
        );
    }
    assert!(
        count(
            &ctx,
            "SELECT count(*) FROM behaviors b JOIN operations o \
        ON o.node_id = b.operation_node_id WHERE o.access_path = 'transferpkg.outer' \
        AND b.kind = 0 AND b.parameter_name = 'value' AND b.verdict IN (0, 1)"
        )
        .await
            > 0
    );
    assert!(
        count(
            &ctx,
            "SELECT count(*) FROM behaviors b JOIN operations o \
        ON o.node_id = b.operation_node_id WHERE o.access_path = 'transferpkg.outer' \
        AND b.kind = 3 AND b.parameter_name = 'value' AND b.verdict = 3"
        )
        .await
            > 0
    );
    assert_eq!(
        count(
            &ctx,
            "SELECT count(*) FROM dynamic_accesses a \
        JOIN declarations d ON d.node_id = a.function_node_id \
        WHERE d.qualified_name LIKE 'transferpkg.Stable.%' \
          AND a.reaches_class_node_id IS NOT NULL"
        )
        .await,
        2
    );
    assert_eq!(
        count(
            &ctx,
            "SELECT count(*) FROM dynamic_accesses a \
        JOIN declarations d ON d.node_id = a.function_node_id \
        WHERE d.qualified_name LIKE 'transferpkg.MixedReceiver.%' \
          AND a.reaches_class_node_id IS NULL AND NOT a.reaches_all"
        )
        .await,
        3
    );
    assert_eq!(
        count(
            &ctx,
            "SELECT count(*) FROM negative_premises \
        WHERE place_key = 'Field[transferpkg.Unrelated.unread]' \
          AND NOT holds AND boundary_reason = 16"
        )
        .await,
        1
    );
    assert_eq!(
        count(
            &ctx,
            "SELECT count(*) FROM negative_premises p \
        JOIN parameter_syntax s ON s.node_id = p.subject_node_id \
        JOIN declarations d ON d.node_id = s.function_node_id \
        WHERE d.qualified_name = 'transferpkg.unused' AND s.name = 'ignored' AND p.holds"
        )
        .await,
        1
    );
    assert_eq!(count(&ctx, "SELECT count(*) FROM singletons \
        WHERE global IN ('transferpkg.global_choice', 'transferpkg.rebound', 'transferpkg.selected_instance')").await, 0);
    assert_eq!(
        count(
            &ctx,
            "SELECT count(*) FROM singletons WHERE global = 'transferpkg.fixed'"
        )
        .await,
        1
    );
    assert_eq!(count(&ctx, "SELECT count(*) FROM dynamic_accesses a JOIN declarations d \
        ON d.node_id = a.function_node_id WHERE d.qualified_name IN \
        ('transferpkg.read_global_choice', 'transferpkg.read_rebound', 'transferpkg.read_selected') \
        AND a.reaches_class_node_id IS NULL").await, 3);
    let flow_claims: Vec<cpg_schema::behavior::BehaviorsRow> = sql::fetch(
        &ctx,
        &Relation {
            name: "test_finalized_transfer_claim_ids",
            deps: &["behaviors"],
            sql: "SELECT * FROM behaviors WHERE transfer IS NOT NULL".to_owned(),
        },
        sql::Params::new(),
    )
    .await
    .unwrap();
    assert!(!flow_claims.is_empty());
    for row in &flow_claims {
        assert_eq!(
            row.behavior_id,
            cpg_schema::behavior::flow_behavior_id(row),
            "published identity must follow the typed transfer and scope"
        );
        let mut regraded = row.clone();
        regraded.verdict = Verdict::Unknown;
        regraded.boundary_reason = Some(BoundaryReason::MissingEvidence);
        assert_eq!(
            row.behavior_id,
            cpg_schema::behavior::flow_behavior_id(&regraded),
            "grading does not change claim identity"
        );
    }
    let generation =
        cpg_core::bundle::bundle(first.path(), snapshot, &first.path().join("generations"))
            .await
            .unwrap();
    let output = std::process::Command::new("uv")
        .args(["run", "--no-sync", "python", "-c", r#"
import sys
from pathlib import Path
sys.path[:0] = [str(Path("scripts").resolve()), str(Path("python/lctx_mcp/tests").resolve())]
from support import load_native as load, served_bundle
with served_bundle(Path(sys.argv[1])) as pg:
    generation = pg.load()
    mixed = pg.operation(generation, generation.snapshot_id, 'transferpkg.mixed')
    fates = [r.record for r in pg.section(generation.snapshot_id, mixed.operation_id, 'behavior') if r.record.parameter == 'value' and r.record.kind == 'returns']
    assert {f.transfer: f.verdict for f in fates} == {'identity': 'conditional', 'derived': 'conditional', 'call': 'unknown'}
    assert all(f.condition_scope_id == mixed.operation_id for f in fates)
    holder = pg.operation(generation, generation.snapshot_id, 'transferpkg.RecordHolder.__init__')
    fates = [r.record for r in pg.section(generation.snapshot_id, holder.operation_id, 'behavior') if r.record.parameter == 'value' and r.record.kind == 'returns']
    assert len(fates) == 3 and all(f.condition_scope_id != holder.operation_id for f in fates)
    assert {f.transfer for f in fates} == {'identity', 'derived', 'call'}
    assert all(f.verdict == 'unknown' for f in fates)
    unmodeled = pg.operation(generation, generation.snapshot_id, 'transferpkg.Holder.__init__')
    assert not [r.record for r in pg.section(generation.snapshot_id, unmodeled.operation_id, 'behavior') if r.record.parameter == 'value' and r.record.kind == 'returns']
"#]).arg(&generation.dir)
        .current_dir(Path::new(env!("CARGO_MANIFEST_DIR")).join("../.."))
        .output().unwrap();
    assert!(
        output.status.success(),
        "{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    for (name, batch) in &mut inputs {
        if matches!(*name, "flow_reaching" | "flow_values") && batch.num_rows() > 1 {
            let indices = UInt32Array::from((0..batch.num_rows() as u32).rev().collect::<Vec<_>>());
            let columns = batch
                .columns()
                .iter()
                .map(|column| arrow_select::take::take(column.as_ref(), &indices, None).unwrap())
                .collect();
            *batch = RecordBatch::try_new(batch.schema(), columns).unwrap();
        }
    }
    let second = tempfile::tempdir().unwrap();
    compile_analyzed(second.path(), snapshot, &inputs, Some(&analysis))
        .await
        .unwrap();
    let (_, reordered) = published(second.path(), snapshot).await.unwrap().unwrap();
    for (table, order) in [
        (
            "value_flows",
            "function_node_id, sink, sink_start_byte, source_key, identity, through_call",
        ),
        ("behaviors", "behavior_id"),
        ("dynamic_accesses", "call_site_node_id"),
        ("negative_premises", "place_key"),
    ] {
        let query = format!("SELECT * FROM {table} ORDER BY {order}");
        assert_eq!(
            text(&ctx, &query).await,
            text(&reordered, &query).await,
            "{table}"
        );
    }
    // Inject provider approximation without changing any condition identity. Its row-local
    // fidelity must survive acquisition, propagation, publication and Pass B admission.
    for (name, batch) in &mut inputs {
        if matches!(*name, "flow_reaching" | "flow_values") {
            let index = batch.schema().index_of("approximated").unwrap();
            let mut columns = batch.columns().to_vec();
            columns[index] = Arc::new(arrow_array::BooleanArray::from(vec![
                true;
                batch.num_rows()
            ]));
            *batch = RecordBatch::try_new(batch.schema(), columns).unwrap();
        }
    }
    let approximate = tempfile::tempdir().unwrap();
    compile_analyzed(approximate.path(), snapshot, &inputs, Some(&analysis))
        .await
        .unwrap();
    let (_, approximate_ctx) = published(approximate.path(), snapshot)
        .await
        .unwrap()
        .unwrap();
    assert!(
        count(
            &approximate_ctx,
            "SELECT count(*) FROM value_flows WHERE approximated"
        )
        .await
            > 0
    );
    assert_eq!(
        count(&approximate_ctx, "SELECT count(*) FROM summary_flows").await,
        0
    );
    assert_eq!(
        count(
            &approximate_ctx,
            "SELECT count(*) FROM behaviors WHERE transfer IS NOT NULL \
        AND verdict IN (0, 1)"
        )
        .await,
        0
    );
    assert_eq!(
        count(
            &approximate_ctx,
            "SELECT count(*) FROM dynamic_accesses a JOIN declarations d \
        ON d.node_id = a.function_node_id WHERE d.qualified_name LIKE 'transferpkg.Stable.%' \
        AND a.reaches_class_node_id IS NOT NULL"
        )
        .await,
        0
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn real_flow_input_row_order_keeps_published_contribution_bytes() {
    let snapshot = Id([94; 16]);
    let ordered = raw("return_completion_shapes", snapshot);
    let mut shuffled = ordered.clone();
    let mut reversed_rows = 0;
    for (name, batch) in &mut shuffled {
        if !matches!(*name, "flow_reaching" | "flow_values") || batch.num_rows() < 2 {
            continue;
        }
        let indices = UInt32Array::from((0..batch.num_rows() as u32).rev().collect::<Vec<_>>());
        let columns = batch
            .columns()
            .iter()
            .map(|column| arrow_select::take::take(column.as_ref(), &indices, None).unwrap())
            .collect();
        *batch = RecordBatch::try_new(batch.schema(), columns).unwrap();
        reversed_rows += batch.num_rows();
    }
    assert!(
        reversed_rows > 2,
        "the real fixture must exercise reordered flow inputs"
    );

    let first = tempfile::tempdir().unwrap();
    let second = tempfile::tempdir().unwrap();
    let analysis = Analysis {
        embedding_cache: None,
        config: AnalyticsConfig::parse(
            "version = 1\n[subsystem]\nmodule_prefixes = [\"returnpkg\"]\n\
             public_roots = [\"returnpkg\"]\n[seeds]\nprimary = [\"returnpkg.plain_identity\"]\n\
             distractors = []\n[pass_a]\nmax_depth = 2\nmax_vertices = 128\n\
             max_edges = 512\nmax_witnesses = 3\n[briefs]\nbudget = 1\n",
        )
        .unwrap(),
        embedder: Some(Arc::new(cpg_core::embedding_service::FakeEmbedder::new())),
        techniques: Techniques::default(),
    };
    compile_analyzed(first.path(), snapshot, &ordered, Some(&analysis))
        .await
        .unwrap();
    compile_analyzed(second.path(), snapshot, &shuffled, Some(&analysis))
        .await
        .unwrap();
    let (_, first_ctx) = published(first.path(), snapshot).await.unwrap().unwrap();
    let (_, second_ctx) = published(second.path(), snapshot).await.unwrap().unwrap();
    assert!(count(&first_ctx, "SELECT count(*) FROM value_flow_contributions").await > 0);
    async fn bytes(ctx: &SessionContext, table: &str) -> Vec<u8> {
        let frame = sql::query(ctx, &format!("SELECT * FROM {table}"))
            .await
            .unwrap();
        let schema = Arc::new(frame.schema().as_arrow().clone());
        let columns = schema
            .fields()
            .iter()
            .map(|field| format!("\"{}\"", field.name()))
            .collect::<Vec<_>>()
            .join(", ");
        let sorted = sql::query(ctx, &format!("SELECT * FROM {table} ORDER BY {columns}"))
            .await
            .unwrap()
            .collect()
            .await
            .unwrap();
        let batch = arrow_select::concat::concat_batches(&schema, &sorted).unwrap();
        let mut out = Vec::new();
        {
            let mut writer = arrow_ipc::writer::StreamWriter::try_new(&mut out, &schema).unwrap();
            writer.write(&batch).unwrap();
            writer.finish().unwrap();
        }
        out
    }
    for table in [
        "value_flow_contributions",
        "value_flows",
        "negative_premises",
    ] {
        assert_eq!(
            bytes(&first_ctx, table).await,
            bytes(&second_ctx, table).await,
            "{table} changed when real extraction rows were reversed"
        );
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn nested_flow_call_steps_survive_publication_with_exact_source_coordinates() {
    let root = tempfile::tempdir().unwrap();
    let snapshot = Id([93; 16]);
    compile(root.path(), snapshot, &raw("flow_call_paths", snapshot))
        .await
        .unwrap();
    let (_, ctx) = published(root.path(), snapshot).await.unwrap().unwrap();
    assert_eq!(
        count(
            &ctx,
            &format!(
                "SELECT count(*) FROM ( \
                 SELECT c.flow_value_fact_id FROM flow_value_calls c \
                 JOIN flow_values v ON v.fact_id = c.flow_value_fact_id \
                 JOIN flow_uses u ON u.use_id = c.use_id \
                 WHERE v.sink = {} AND u.place = 'value' \
                 GROUP BY c.flow_value_fact_id \
                 HAVING count(*) = 2 AND min(c.role) = {} AND max(c.role) = {})",
                FlowSink::Return.code(),
                FlowCallOperandRole::Argument.code(),
                FlowCallOperandRole::Argument.code(),
            ),
        )
        .await,
        1,
        "the nested return has exactly two ordered argument steps"
    );
    assert_eq!(
        count(
            &ctx,
            "SELECT count(*) FROM flow_value_calls c \
             JOIN call_syntax s ON s.module_node_id = c.module_node_id \
               AND s.start_byte = c.call_start_byte AND s.end_byte = c.call_end_byte \
             JOIN arguments a ON a.call_node_id = s.node_id \
               AND a.value_start_byte = c.operand_start_byte \
               AND a.value_end_byte = c.operand_end_byte \
             WHERE c.step = 1 AND a.keyword = 'data'"
        )
        .await,
        1,
        "the nested keyword step joins the actual value rather than its name prefix"
    );
    assert_eq!(
        count(
            &ctx,
            &format!(
                "SELECT count(*) FROM flow_value_calls c JOIN flow_uses u ON u.use_id = c.use_id \
                 WHERE u.place = 'func' AND c.role = {}",
                FlowCallOperandRole::Callee.code()
            )
        )
        .await,
        1,
        "the callee use is never relabeled as an input argument"
    );
    assert_eq!(
        count(&ctx, "SELECT count(*) FROM flow_value_call_links").await,
        count(&ctx, "SELECT count(*) FROM flow_value_calls").await,
        "the derived bridge retains every raw path step"
    );
    assert_eq!(
        count(
            &ctx,
            &format!(
                "SELECT count(*) FROM flow_value_call_links l \
                 JOIN flow_value_calls c ON c.fact_id = l.flow_value_call_fact_id \
                 JOIN arguments a ON a.node_id = l.argument_node_id \
                 WHERE c.step = 1 AND a.keyword = 'data' AND l.status = {} \
                   AND l.argument_fact_id = a.fact_id",
                FlowCallLinkStatus::BoundArgument.code()
            )
        )
        .await,
        1,
        "the persisted inner step cites the unique keyword value argument"
    );
    assert_eq!(
        count(
            &ctx,
            &format!(
                "SELECT count(*) FROM flow_value_call_links l \
                 JOIN flow_value_calls c ON c.fact_id = l.flow_value_call_fact_id \
                 JOIN flow_uses u ON u.use_id = c.use_id \
                 WHERE u.place = 'func' AND l.status = {} \
                   AND l.call_node_id IS NOT NULL AND l.argument_node_id IS NULL",
                FlowCallLinkStatus::BoundCallee.code()
            )
        )
        .await,
        1,
        "a callee source link cannot masquerade as an argument transfer"
    );

    let original_links = sql::query(&ctx, "SELECT * FROM flow_value_call_links")
        .await
        .unwrap()
        .into_view();
    let doctored_links = sql::query(
        &ctx,
        &format!(
            "SELECT * EXCLUDE (status), CAST({} AS SMALLINT) AS status \
             FROM flow_value_call_links",
            FlowCallLinkStatus::MissingCall.code()
        ),
    )
    .await
    .unwrap()
    .into_view();
    ctx.deregister_table("flow_value_call_links").unwrap();
    ctx.register_table("flow_value_call_links", doctored_links)
        .unwrap();
    let violations = cpg_core::validate::validate(&ctx).await.unwrap();
    assert!(
        violations
            .iter()
            .any(|v| v.rule == "semantic:flow-call-link-source-equality"),
        "{violations:?}"
    );
    ctx.deregister_table("flow_value_call_links").unwrap();
    ctx.register_table("flow_value_call_links", original_links)
        .unwrap();

    let original_calls = sql::query(&ctx, "SELECT * FROM call_syntax")
        .await
        .unwrap()
        .into_view();
    let removed_steps = count(
        &ctx,
        "SELECT count(*) FROM flow_value_calls f \
         JOIN call_syntax c ON c.module_node_id = f.module_node_id \
           AND c.start_byte = f.call_start_byte AND c.end_byte = f.call_end_byte \
         WHERE c.node_id IN (SELECT call_node_id FROM arguments WHERE keyword = 'data')",
    )
    .await;
    assert!(removed_steps > 0);
    let missing_calls = sql::query(
        &ctx,
        "SELECT * FROM call_syntax WHERE node_id NOT IN \
         (SELECT call_node_id FROM arguments WHERE keyword = 'data')",
    )
    .await
    .unwrap()
    .into_view();
    ctx.deregister_table("call_syntax").unwrap();
    ctx.register_table("call_syntax", missing_calls).unwrap();
    assert!(count(&ctx, "SELECT count(*) FROM call_syntax").await > 0);
    let link_query =
        <cpg_schema::derived::FlowValueCallLinks as cpg_schema::derived::Derived>::sql();
    assert_eq!(
        count(
            &ctx,
            &format!("WITH bridge AS ({link_query}) SELECT count(*) FROM bridge")
        )
        .await,
        count(&ctx, "SELECT count(*) FROM flow_value_calls").await,
        "the recomputed bridge keeps every step when source calls disappear"
    );
    assert_eq!(
        count(
            &ctx,
            &format!(
                "WITH bridge AS ({link_query}) SELECT count(*) FROM bridge WHERE status = {}",
                FlowCallLinkStatus::MissingCall.code()
            )
        )
        .await,
        removed_steps,
        "every missing source call remains an explicit unknown link; {}",
        text(
            &ctx,
            &format!(
                "WITH bridge AS ({link_query}) SELECT status, count(*) AS n FROM bridge GROUP BY status"
            )
        )
        .await
    );
    ctx.deregister_table("call_syntax").unwrap();
    ctx.register_table("call_syntax", original_calls).unwrap();

    let duplicate_calls = sql::query(
        &ctx,
        "SELECT * FROM call_syntax UNION ALL SELECT * FROM call_syntax",
    )
    .await
    .unwrap()
    .into_view();
    let original_calls = sql::query(&ctx, "SELECT * FROM call_syntax")
        .await
        .unwrap()
        .into_view();
    ctx.deregister_table("call_syntax").unwrap();
    ctx.register_table("call_syntax", duplicate_calls).unwrap();
    assert_eq!(
        count(
            &ctx,
            &format!(
                "WITH bridge AS ({link_query}) SELECT count(*) FROM bridge WHERE status = {}",
                FlowCallLinkStatus::AmbiguousCall.code()
            )
        )
        .await,
        count(&ctx, "SELECT count(*) FROM flow_value_calls").await,
        "a doubled call coordinate cannot become an arbitrary single source identity"
    );
    ctx.deregister_table("call_syntax").unwrap();
    ctx.register_table("call_syntax", original_calls).unwrap();
}

#[tokio::test(flavor = "multi_thread")]
async fn an_attempt_publishes_every_table_and_readers_see_only_published_rows() {
    let root = tempfile::tempdir().unwrap();
    let (a, b) = (Id([1; 16]), Id([2; 16]));
    let raw_a = raw("pysa_variants", a);
    let out = compile(root.path(), a, &raw_a).await.unwrap();
    let versions = resolve(root.path(), a).await.unwrap().expect("published");
    assert_eq!(versions, out.versions);
    macro_rules! count_tables {
        ($($t:ty),+) => {[$(<$t as Table>::NAME),+].len()};
    }
    assert_eq!(
        versions.len(),
        cpg_schema::for_each_table!(count_tables)
            + cpg_schema::for_each_derived_table!(count_tables)
            + cpg_schema::for_each_analysis_table!(count_tables),
        "every raw, derived and analysis table"
    );

    // A second attempt fails validation after writing its rows: it publishes nothing.
    let mut raw_b = raw("pysa_variants", b);
    drop_rows(&mut raw_b, "coverage", 1);
    assert!(matches!(
        compile(root.path(), b, &raw_b).await,
        Err(CoreError::Invalid(_))
    ));
    assert!(resolve(root.path(), b).await.unwrap().is_none());

    // A's reader still sees exactly A's rows, at A's versions.
    let declarations = raw_a.iter().find(|(n, _)| *n == "declarations").unwrap();
    let back = read_at::<Declarations>(root.path(), versions["declarations"], a)
        .await
        .unwrap();
    assert_eq!(&back, &declarations.1);
    let (_, ctx) = published(root.path(), a).await.unwrap().unwrap();
    assert_eq!(
        count(&ctx, "SELECT count(*) FROM declarations").await,
        declarations.1.num_rows() as i64
    );
}

/// Derived tables rendered by name, for review (the ids are opaque). The temp dir holds the tables
/// the session reads.
async fn derived_text(fixture: &str) -> (String, SessionContext, tempfile::TempDir) {
    let root = tempfile::tempdir().unwrap();
    let s = Id([3; 16]);
    compile(root.path(), s, &raw(fixture, s)).await.unwrap();
    let (_, ctx) = published(root.path(), s).await.unwrap().unwrap();
    let mut out = String::new();
    for (title, statement) in [
        (
            "exports",
            "SELECT e.access_path, d.qualified_name AS declaration, d.is_overload \
             FROM exports e LEFT JOIN declarations d ON d.node_id = e.declaration_node_id \
             ORDER BY e.access_path, declaration NULLS FIRST",
        ),
        (
            "signatures",
            "SELECT d.qualified_name AS def, d.start_byte, c.start_byte AS callable_start, \
                    s.function_key, s.signature_index, s.form, s.reason \
             FROM signatures s JOIN declarations d ON d.node_id = s.signature_node_id \
             JOIN declarations c ON c.node_id = s.callable_node_id \
             ORDER BY d.start_byte, def",
        ),
        (
            "parameters",
            "SELECT d.qualified_name AS def, d.start_byte, p.ordinal, x.name AS syntax, \
                    q.name AS semantics, q.required, p.reason \
             FROM parameters p JOIN declarations d ON d.node_id = p.signature_node_id \
             LEFT JOIN parameter_syntax x ON x.fact_id = p.syntax_fact_id \
             LEFT JOIN parameter_semantics q ON q.fact_id = p.semantics_fact_id \
             ORDER BY d.start_byte, p.ordinal",
        ),
        (
            "resolutions",
            "SELECT c.start_byte, c.end_byte, r.status, r.has_unresolved_remainder AS remainder, \
                    r.candidate_set_complete_under_model AS complete, r.unresolved_reason, \
                    r.target_count, r.reason \
             FROM resolutions r JOIN call_syntax c ON c.node_id = r.call_site_node_id \
             ORDER BY c.start_byte, c.end_byte",
        ),
        (
            "call_targets",
            "SELECT c.start_byte, c.end_byte, p.phase, p.higher_order_index AS ho, \
                    p.target_module, p.target_name, \
                    COALESCE(d.qualified_name, x.qualified_name) AS target, n.node_kind AS kind, \
                    f.modality, t.reason \
             FROM call_targets t JOIN call_syntax c ON c.node_id = t.call_site_node_id \
             JOIN pysa_calls p ON p.fact_id = t.pysa_fact_id \
             JOIN facts f ON f.fact_id = t.pysa_fact_id \
             LEFT JOIN declarations d ON d.node_id = t.target_node_id \
             LEFT JOIN context_definitions x ON x.symbol_node_id = t.target_node_id \
             LEFT JOIN nodes n ON n.node_id = t.target_node_id \
             ORDER BY c.start_byte, c.end_byte, p.phase, ho NULLS FIRST, p.target_name",
        ),
        (
            "nodes",
            "SELECT node_kind, count(*) AS n FROM nodes GROUP BY node_kind ORDER BY node_kind",
        ),
        (
            "edges",
            "SELECT edge_kind, src_kind, dst_kind, count(*) AS n FROM edges \
             GROUP BY edge_kind, src_kind, dst_kind ORDER BY edge_kind, src_kind, dst_kind",
        ),
    ] {
        out.push_str(&format!("## {title}\n{}\n", text(&ctx, statement).await));
    }
    (out, ctx, root)
}

/// Publication and a generation load must reject a node changed after the Delta write. The
/// session view is taken from the published, version-pinned Delta table before it is doctored.
#[tokio::test(flavor = "multi_thread")]
async fn published_condition_node_tamper_is_rejected() {
    let root = tempfile::tempdir().unwrap();
    let snapshot = Id([46; 16]);
    compile(root.path(), snapshot, &raw("flow_shapes", snapshot))
        .await
        .unwrap();
    let (_, ctx) = published(root.path(), snapshot).await.unwrap().unwrap();
    assert!(cpg_core::validate::validate(&ctx).await.unwrap().is_empty());
    let doctored = sql::query(
        &ctx,
        "SELECT * EXCLUDE (atom), concat(atom, 'tampered') AS atom FROM condition_nodes",
    )
    .await
    .unwrap()
    .into_view();
    ctx.deregister_table("condition_nodes").unwrap();
    ctx.register_table("condition_nodes", doctored).unwrap();
    let violations = cpg_core::validate::validate(&ctx).await.unwrap();
    assert!(
        violations
            .iter()
            .any(|v| v.rule == "condition-graph-and-leaf-provenance"),
        "{violations:?}"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn published_test_type_link_tamper_is_rejected() {
    let root = tempfile::tempdir().unwrap();
    let snapshot = Id([47; 16]);
    compile(root.path(), snapshot, &raw("flow_shapes", snapshot))
        .await
        .unwrap();
    let (_, ctx) = published(root.path(), snapshot).await.unwrap().unwrap();
    assert!(count(&ctx, "SELECT count(*) FROM flow_test_types").await > 0);
    assert!(cpg_core::validate::validate(&ctx).await.unwrap().is_empty());
    let doctored = sql::query(
        &ctx,
        "SELECT * EXCLUDE (place), concat(place, '_tampered') AS place FROM flow_test_types",
    )
    .await
    .unwrap()
    .into_view();
    ctx.deregister_table("flow_test_types").unwrap();
    ctx.register_table("flow_test_types", doctored).unwrap();
    let violations = cpg_core::validate::validate(&ctx).await.unwrap();
    assert!(
        violations
            .iter()
            .any(|v| v.rule == "flow-test-type-proof-link"),
        "{violations:?}"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn explicit_exit_sites_have_regions_and_reject_a_doctored_span() {
    let root = tempfile::tempdir().unwrap();
    let snapshot = Id([54; 16]);
    compile_behavior(
        root.path(),
        snapshot,
        &raw("flow_shapes", snapshot),
        "fallback",
    )
    .await;
    let (_, ctx) = published(root.path(), snapshot).await.unwrap().unwrap();
    assert!(
        count(
            &ctx,
            &format!(
                "SELECT count(*) FROM exit_sites e JOIN declarations d \
                 ON d.node_id = e.function_node_id WHERE d.name = 'guarded' \
                 AND e.kind = {}",
                ExitSiteKind::Raise.code()
            )
        )
        .await
            > 0
    );
    assert_eq!(
        count(
            &ctx,
            &format!(
                "SELECT count(*) FROM exit_sites e JOIN declarations d \
                 ON d.node_id = e.function_node_id WHERE d.name = 'guarded' \
                 AND e.kind = {}",
                ExitSiteKind::FinallyBody.code()
            )
        )
        .await,
        0,
        "ordinary branch actions are not finally actions"
    );
    assert!(
        count(
            &ctx,
            &format!(
                "SELECT count(*) FROM exit_sites e JOIN declarations d \
                 ON d.node_id = e.function_node_id WHERE d.name = 'handled' \
                 AND e.kind = {}",
                ExitSiteKind::FinallyBody.code()
            )
        )
        .await
            > 0
    );
    assert_eq!(
        count(
            &ctx,
            "SELECT count(*) FROM exit_sites e LEFT JOIN flow_regions r \
             ON r.fact_id = e.region_fact_id WHERE r.fact_id IS NULL"
        )
        .await,
        0
    );
    assert_eq!(
        count(
            &ctx,
            "SELECT count(*) FROM return_exit_statuses x JOIN declarations d \
             ON d.node_id = x.function_node_id WHERE d.name = 'nested_identity' \
             AND x.reason IS NULL"
        )
        .await,
        2,
        "an ordinary nested branch does not add an exit controller"
    );
    for name in ["with_identity"] {
        assert_eq!(
            count(
                &ctx,
                &format!(
                    "SELECT count(*) FROM return_exit_statuses x JOIN declarations d \
                     ON d.node_id = x.function_node_id WHERE d.name = '{name}' \
                     AND x.reason = {} AND x.frame_node_id IS NOT NULL",
                    BoundaryReason::UnsupportedControlFlow.code()
                )
            )
            .await,
            1,
            "{name} must retain its controlling frame"
        );
    }
    assert!(cpg_core::validate::validate(&ctx).await.unwrap().is_empty());
    let original_exit = sql::query(&ctx, "SELECT * FROM exit_sites")
        .await
        .unwrap()
        .into_view();
    let doctored = sql::query(
        &ctx,
        "SELECT * EXCLUDE (start_byte), start_byte + 1 AS start_byte FROM exit_sites",
    )
    .await
    .unwrap()
    .into_view();
    ctx.deregister_table("exit_sites").unwrap();
    ctx.register_table("exit_sites", doctored).unwrap();
    let violations = cpg_core::validate::validate(&ctx).await.unwrap();
    assert!(
        violations
            .iter()
            .any(|v| v.rule == "exit-site-source-equality"),
        "{violations:?}"
    );
    ctx.deregister_table("exit_sites").unwrap();
    ctx.register_table("exit_sites", original_exit).unwrap();
    let original_statuses = sql::query(&ctx, "SELECT * FROM return_exit_statuses")
        .await
        .unwrap()
        .into_view();
    let forged = sql::query(
        &ctx,
        "SELECT * EXCLUDE (walk_depth), walk_depth + 1 AS walk_depth FROM return_exit_statuses",
    )
    .await
    .unwrap()
    .into_view();
    ctx.deregister_table("return_exit_statuses").unwrap();
    ctx.register_table("return_exit_statuses", forged).unwrap();
    let violations = cpg_core::validate::validate(&ctx).await.unwrap();
    assert!(
        violations
            .iter()
            .any(|v| v.rule == "return-exit-status-source-equality"),
        "{violations:?}"
    );
    ctx.deregister_table("return_exit_statuses").unwrap();
    ctx.register_table("return_exit_statuses", original_statuses)
        .unwrap();
}

#[tokio::test(flavor = "multi_thread")]
async fn nested_returns_need_an_uncontrolled_exit_before_becoming_value_summaries() {
    let root = tempfile::tempdir().unwrap();
    let snapshot = Id([56; 16]);
    let analysis = Analysis {
        embedding_cache: None,
        config: AnalyticsConfig::parse(
            r#"
version = 1
[subsystem]
module_prefixes = ["returnpkg"]
public_roots = ["returnpkg"]
[seeds]
primary = ["returnpkg.plain_identity"]
distractors = []
[pass_a]
max_depth = 2
max_vertices = 128
max_edges = 512
max_witnesses = 3
[briefs]
budget = 1
"#,
        )
        .unwrap(),
        embedder: Some(Arc::new(cpg_core::embedding_service::FakeEmbedder::new())),
        techniques: Techniques::default(),
    };
    compile_analyzed(
        root.path(),
        snapshot,
        &raw("return_completion_shapes", snapshot),
        Some(&analysis),
    )
    .await
    .unwrap();
    let (_, ctx) = published(root.path(), snapshot).await.unwrap().unwrap();
    assert!(
        count(
            &ctx,
            &format!(
                "SELECT count(*) FROM value_flows WHERE sink = {}",
                FlowSink::Return.code()
            )
        )
        .await
            > 0,
        "the fixture must actually produce return value facts"
    );
    for name in [
        "plain_identity",
        "nested_identity",
        "finally_pass_identity",
        "nested_finally_pass_identity",
        "multi_pass_finally_identity",
        "recursive_base_identity",
        "alternate_branch_identity",
    ] {
        assert!(
            count(
                &ctx,
                &format!(
                    "SELECT count(*) FROM summary_flows f JOIN declarations d \
                     ON d.node_id = f.function_node_id WHERE d.name = '{name}' \
                     AND f.input_path = 'Parameter[value]'"
                )
            )
            .await
                > 0,
            "{name} has an admitted parameter-to-return path"
        );
    }
    assert_eq!(
        count(
            &ctx,
            "SELECT count(*) FROM call_syntax c JOIN declarations d \
             ON d.node_id = c.owner_node_id \
             WHERE d.name = 'alternate_branch_identity' AND NOT c.in_annotation",
        )
        .await,
        1,
        "the admitted else return must coexist with a source call in the if branch"
    );
    assert_eq!(
        count(
            &ctx,
            "SELECT count(*) FROM return_exit_statuses x JOIN declarations d \
             ON d.node_id = x.function_node_id WHERE d.name = 'finally_pass_identity' \
               AND x.reason IS NULL AND x.frame_node_id IS NOT NULL \
               AND x.pass_node_id IS NOT NULL AND x.pass_fact_id IS NOT NULL",
        )
        .await,
        1,
        "the sole pass finalizer has its own source proof"
    );
    assert_eq!(
        count(
            &ctx,
            &format!(
                "SELECT count(*) FROM summary_flows f \
             JOIN declarations d ON d.node_id = f.function_node_id \
             JOIN return_exit_statuses x ON x.source_fact_id = f.return_site_fact_id \
             JOIN summary_flow_steps p ON p.summary_id = f.summary_id \
             WHERE d.name = 'finally_pass_identity' \
               AND p.kind = {} AND p.ordinal = 1 \
               AND p.evidence_id = x.pass_fact_id",
                cpg_schema::codebook::SummaryFlowStepKind::FinalizerPass.code(),
            )
        )
        .await,
        1,
        "the finite proof itself cites the completed finalizer"
    );
    assert_eq!(
        count(
            &ctx,
            &format!(
                "SELECT count(*) FROM summary_flows f \
             JOIN declarations d ON d.node_id = f.function_node_id \
             JOIN summary_flow_steps inner_pass ON inner_pass.summary_id = f.summary_id \
             JOIN summary_flow_steps outer_pass ON outer_pass.summary_id = f.summary_id \
             JOIN syntax_nodes inner_source ON inner_source.fact_id = inner_pass.evidence_id \
             JOIN syntax_nodes outer_source ON outer_source.fact_id = outer_pass.evidence_id \
             WHERE d.name = 'nested_finally_pass_identity' \
               AND inner_pass.kind = {pass} AND outer_pass.kind = {pass} \
               AND inner_pass.ordinal = 1 AND outer_pass.ordinal = 2 \
               AND inner_source.start_byte < outer_source.start_byte",
                pass = cpg_schema::codebook::SummaryFlowStepKind::FinalizerPass.code(),
            )
        )
        .await,
        1,
        "the two nested passes must be cited in execution order"
    );
    assert_eq!(
        count(
            &ctx,
            "SELECT count(*) FROM return_exit_statuses x JOIN declarations d \
             ON d.node_id = x.function_node_id \
             WHERE d.name = 'nested_finally_pass_identity' \
               AND x.reason IS NULL AND x.pass_fact_id IS NULL"
        )
        .await,
        1,
        "the one-pass field is deliberately empty for an ordered multi-frame proof"
    );
    assert_eq!(
        count(
            &ctx,
            &format!(
                "SELECT count(*) FROM summary_flows f \
                 JOIN declarations d ON d.node_id = f.function_node_id \
                 JOIN summary_flow_steps first ON first.summary_id = f.summary_id \
                 JOIN summary_flow_steps second ON second.summary_id = f.summary_id \
                 JOIN syntax_nodes first_source ON first_source.fact_id = first.evidence_id \
                 JOIN syntax_nodes second_source ON second_source.fact_id = second.evidence_id \
                 JOIN return_exit_statuses x ON x.source_fact_id = f.return_site_fact_id \
                 WHERE d.name = 'multi_pass_finally_identity' AND x.reason IS NULL \
                   AND x.pass_fact_id IS NULL \
                   AND first.kind = {pass} AND second.kind = {pass} \
                   AND first.ordinal = 1 AND second.ordinal = 2 \
                   AND first_source.start_byte < second_source.start_byte",
                pass = cpg_schema::codebook::SummaryFlowStepKind::FinalizerPass.code(),
            ),
        )
        .await,
        1,
        "both pass statements in one finalizer are cited in execution order"
    );
    assert_eq!(
        count(
            &ctx,
            "SELECT count(*) FROM summary_components c JOIN declarations d \
             ON d.node_id = c.function_node_id \
             WHERE d.name = 'recursive_before_return' AND c.recursive"
        )
        .await,
        1,
        "the source self-call belongs to a recursive SCC"
    );
    assert!(
        count(
            &ctx,
            &format!(
                "SELECT count(*) FROM ({}) s JOIN declarations d \
             ON d.node_id = s.function_node_id \
             WHERE d.name = 'conditional_self_recursive'",
                cpg_schema::behavior::local_call_summary_flow_seeds().sql,
            )
        )
        .await
            > 0,
        "the recursive return supplies a real local-call candidate"
    );
    assert!(
        count(&ctx, &format!(
            "SELECT count(*) FROM ({}) s JOIN declarations d \
             ON d.node_id = s.function_node_id \
             JOIN ({}) a ON a.call_fact_id = s.call_fact_id AND a.callee_node_id = s.callee_node_id \
             JOIN flow_test_value_links link ON link.operation_node_id = s.callee_node_id \
               AND link.formal_node_id = a.formal_node_id \
             WHERE d.name = 'recursive_base_identity' \
               AND a.boolean_value",
            cpg_schema::behavior::local_call_summary_flow_seeds().sql,
            cpg_schema::behavior::local_call_arguments().sql,
        )).await > 0,
        "the second literal formal has an exact entry-value test link"
    );
    assert_eq!(
        count(
            &ctx,
            &format!(
                "SELECT count(*) FROM summary_flows f \
             JOIN declarations d ON d.node_id = f.function_node_id \
             JOIN summary_flow_steps s ON s.summary_id = f.summary_id \
               AND s.kind = {} \
             JOIN flow_test_value_links l ON l.link_id = s.evidence_id \
             WHERE d.name = 'recursive_base_identity' AND f.path_depth = 1 \
               AND f.verdict = {}",
                cpg_schema::codebook::SummaryFlowStepKind::CalleeConditionLink.code(),
                Verdict::Conditional.code(),
            )
        )
        .await,
        1,
        "the finite recursive return cites the exact literal-to-guard link"
    );
    assert!(
        count(&ctx, &format!(
            "SELECT count(*) FROM ({}) s JOIN declarations d \
             ON d.node_id = s.function_node_id \
             JOIN ({}) a ON a.call_fact_id = s.call_fact_id AND a.callee_node_id = s.callee_node_id \
             JOIN flow_test_value_links link ON link.operation_node_id = s.callee_node_id \
               AND link.formal_node_id = a.formal_node_id \
             WHERE d.name = 'recursive_false_control' \
               AND NOT a.boolean_value",
            cpg_schema::behavior::local_call_summary_flow_seeds().sql,
            cpg_schema::behavior::local_call_arguments().sql,
        )).await > 0,
        "the opposing literal also has a directly linked guard"
    );
    assert_eq!(
        count(
            &ctx,
            "SELECT count(*) FROM summary_flows f JOIN declarations d \
            ON d.node_id = f.function_node_id \
            WHERE d.name = 'recursive_false_control' AND f.path_depth > 0"
        )
        .await,
        0,
        "restricting the callee base guard to false cannot prove recursive return"
    );
    assert!(
        count(
            &ctx,
            "SELECT count(*) FROM summary_boundaries b JOIN declarations d \
            ON d.node_id = b.function_node_id \
            WHERE d.name = 'recursive_false_control' AND b.local_through_call"
        )
        .await
            > 0,
        "the opposing recursive path remains an explicit unknown"
    );
    assert!(
        count(&ctx, &format!(
            "SELECT count(*) FROM ({}) s JOIN declarations d \
             ON d.node_id = s.function_node_id \
             JOIN ({}) a ON a.call_fact_id = s.call_fact_id AND a.callee_node_id = s.callee_node_id \
             JOIN flow_test_value_links link ON link.operation_node_id = s.callee_node_id \
               AND link.formal_node_id = a.formal_node_id \
             JOIN flow_test_value_links caller ON caller.operation_node_id = s.function_node_id \
               AND caller.formal_node_id = a.source_formal_node_id \
             WHERE d.name = 'guarded_symbolic_recursive' \
               AND a.boolean_value IS NULL AND a.source_formal_node_id IS NOT NULL",
            cpg_schema::behavior::local_call_summary_flow_seeds().sql,
            cpg_schema::behavior::local_call_arguments().sql,
        )).await > 0,
        "both sides of the symbolic formal forwarding have direct guard links"
    );
    assert_eq!(
        count(
            &ctx,
            &format!(
                "SELECT count(*) FROM summary_flows f \
             JOIN declarations d ON d.node_id = f.function_node_id \
             JOIN summary_flow_steps caller ON caller.summary_id = f.summary_id \
               AND caller.kind = {} \
             JOIN summary_flow_steps callee ON callee.summary_id = f.summary_id \
               AND callee.kind = {} AND caller.ordinal < callee.ordinal \
             WHERE d.name = 'guarded_symbolic_recursive' AND f.path_depth = 1",
                cpg_schema::codebook::SummaryFlowStepKind::CallerConditionLink.code(),
                cpg_schema::codebook::SummaryFlowStepKind::CalleeConditionLink.code(),
            )
        )
        .await,
        1,
        "the caller's guard fixes the forwarded formal before callee specialization"
    );
    assert_eq!(
        count(
            &ctx,
            "SELECT count(*) FROM summary_flows f JOIN declarations d \
            ON d.node_id = f.function_node_id \
            WHERE d.name = 'guarded_symbolic_base' AND f.path_depth > 0"
        )
        .await,
        0,
        "the opposite forwarded guard cannot reuse the recursive callee path"
    );
    assert_eq!(
        count(
            &ctx,
            "SELECT count(*) FROM summary_flows f JOIN declarations d \
            ON d.node_id = f.function_node_id \
            WHERE d.name = 'keyword_local_wrapper' AND f.path_depth = 1"
        )
        .await,
        1,
        "a single explicit keyword with a definite formal mapping transfers a local value"
    );
    assert_eq!(
        count(
            &ctx,
            "SELECT count(*) FROM summary_flows f JOIN declarations d \
            ON d.node_id = f.function_node_id \
            WHERE d.name = 'unpacked_local_wrapper'"
        )
        .await,
        0,
        "unpacked keyword arguments do not supply an exact local-call mapping"
    );
    assert_eq!(
        count(
            &ctx,
            &format!(
                "SELECT count(*) FROM summary_flows f \
             JOIN declarations d ON d.node_id = f.function_node_id \
             JOIN summary_flow_steps s ON s.summary_id = f.summary_id \
               AND s.kind = {} \
             WHERE d.name = 'recursive_keyword_true' AND f.path_depth = 1",
                cpg_schema::codebook::SummaryFlowStepKind::CalleeConditionLink.code(),
            )
        )
        .await,
        1,
        "the explicit keyword literal specializes the recursive callee guard"
    );
    assert_eq!(
        count(
            &ctx,
            "SELECT count(*) FROM summary_flows f JOIN declarations d \
            ON d.node_id = f.function_node_id \
            WHERE d.name = 'recursive_keyword_false' AND f.path_depth > 0"
        )
        .await,
        0,
        "a false keyword control does not reach the recursive base"
    );
    assert_eq!(
        count(
            &ctx,
            &format!(
                "SELECT count(*) FROM summary_flows f \
             JOIN declarations d ON d.node_id = f.function_node_id \
             JOIN summary_flow_steps s ON s.summary_id = f.summary_id \
               AND s.kind = {} \
             WHERE d.name = 'recursive_all_keyword_true' AND f.path_depth = 1",
                cpg_schema::codebook::SummaryFlowStepKind::CalleeConditionLink.code(),
            )
        )
        .await,
        1,
        "two explicit keywords retain exact value and control mappings"
    );
    assert_eq!(
        count(
            &ctx,
            "SELECT count(*) FROM summary_flows f JOIN declarations d \
            ON d.node_id = f.function_node_id \
            WHERE d.name = 'recursive_all_keyword_false' AND f.path_depth > 0"
        )
        .await,
        0,
        "the opposing all-keyword call remains open"
    );
    assert_eq!(
        count(
            &ctx,
            &format!(
                "SELECT count(*) FROM summary_flows f \
             JOIN declarations d ON d.node_id = f.function_node_id \
             JOIN summary_flow_steps s ON s.summary_id = f.summary_id \
               AND s.kind = {} \
             WHERE d.name = 'recursive_reversed_keyword_true' AND f.path_depth = 1",
                cpg_schema::codebook::SummaryFlowStepKind::CalleeConditionLink.code(),
            )
        )
        .await,
        1,
        "a first keyword literal completes before the tracked second keyword value"
    );
    assert_eq!(
        count(
            &ctx,
            &format!(
                "SELECT count(*) FROM summary_flows f \
             JOIN declarations d ON d.node_id = f.function_node_id \
             JOIN summary_flow_steps first ON first.summary_id = f.summary_id \
               AND first.kind = {evaluation} \
             JOIN syntax_nodes literal ON literal.fact_id = first.evidence_id \
               AND literal.detail = 'True' \
             JOIN summary_flow_steps second ON second.summary_id = f.summary_id \
               AND second.kind = {evaluation} AND second.ordinal = first.ordinal + 2 \
             JOIN flow_reaching read ON read.fact_id = second.evidence_id \
             JOIN summary_flow_steps source ON source.summary_id = f.summary_id \
               AND source.kind = {source_kind} AND source.ordinal = first.ordinal + 1 \
               AND source.evidence_id = f.source_flow_fact_id \
             WHERE d.name = 'recursive_reversed_keyword_true' AND f.path_depth = 1",
                evaluation = cpg_schema::codebook::SummaryFlowStepKind::ArgumentEvaluation.code(),
                source_kind = cpg_schema::codebook::SummaryFlowStepKind::ReturnSource.code(),
            )
        )
        .await,
        1,
        "the published proof evaluates the first literal before the tracked value"
    );
    for (name, expected) in [
        ("multiple_argument_true", 1),
        ("multiple_argument_false", 0),
        ("multiple_argument_raising", 0),
        ("multiple_argument_missing", 0),
        ("multiple_control_true", 0),
        ("multiple_control_false", 0),
        ("multiple_control_raising", 0),
    ] {
        assert_eq!(
            count(
                &ctx,
                &format!(
                    "SELECT count(*) FROM summary_flows f \
            JOIN declarations d ON d.node_id = f.function_node_id \
            WHERE d.name = '{name}' AND f.path_depth = 1"
                )
            )
            .await,
            expected,
            "every argument needs normal evaluation and every callee predicate needs a stable entry link"
        );
    }
    assert_eq!(
        count(
            &ctx,
            &format!(
                "SELECT count(*) FROM summary_flow_steps step \
        JOIN summary_flows f ON f.summary_id = step.summary_id \
        JOIN declarations d ON d.node_id = f.function_node_id \
        WHERE d.name = 'multiple_argument_true' AND f.path_depth = 1 AND step.kind = {}",
                cpg_schema::codebook::SummaryFlowStepKind::CalleeConditionLink.code()
            )
        )
        .await,
        1
    );
    assert_eq!(
        count(
            &ctx,
            "SELECT count(*) FROM summary_flows f JOIN declarations d \
            ON d.node_id = f.function_node_id \
            WHERE d.name = 'recursive_reversed_keyword_false' AND f.path_depth > 0"
        )
        .await,
        0,
        "the opposing reversed-keyword call remains open"
    );
    assert_eq!(
        count(
            &ctx,
            "SELECT count(*) FROM summary_flows f JOIN declarations d \
            ON d.node_id = f.function_node_id \
            WHERE d.name = 'conditional_self_recursive' AND f.path_depth > 0"
        )
        .await,
        0,
        "the conditional base cannot be reused as an unconditional recursive callee path"
    );
    assert!(
        count(
            &ctx,
            "SELECT count(*) FROM summary_boundaries b JOIN declarations d \
            ON d.node_id = b.function_node_id \
            WHERE d.name = 'conditional_self_recursive' AND b.local_through_call"
        )
        .await
            > 0,
        "the unproved recursive call remains an explicit origin-specific unknown"
    );
    for name in ["recursive_before_return", "prior_call_identity"] {
        assert_eq!(
            count(
                &ctx,
                &format!(
                    "SELECT count(*) FROM summary_flows f JOIN declarations d \
                 ON d.node_id = f.function_node_id WHERE d.name = '{name}'"
                )
            )
            .await,
            0,
            "{name} has an unproved earlier call before its direct return"
        );
        assert!(
            count(
                &ctx,
                &format!(
                    "SELECT count(*) FROM summary_boundaries b JOIN declarations d \
                 ON d.node_id = b.function_node_id WHERE d.name = '{name}'"
                )
            )
            .await
                > 0,
            "{name} retains an explicit unknown boundary"
        );
    }
    for name in [
        "finally_identity",
        "pass_then_effect_finally",
        "nested_effectful_finalizer",
    ] {
        assert_eq!(
            count(
                &ctx,
                &format!(
                    "SELECT count(*) FROM summary_flows f JOIN declarations d \
            ON d.node_id = f.function_node_id WHERE d.name = '{name}'"
                )
            )
            .await,
            1,
            "a proven local assignment completes normally without replacing the pending return"
        );
        assert!(count(&ctx, &format!("SELECT count(*) FROM return_exit_steps s \
            JOIN return_exit_statuses x ON x.source_fact_id = s.return_site_fact_id \
            JOIN declarations d ON d.node_id = x.function_node_id WHERE d.name = '{name}' AND s.kind = {}",
            cpg_schema::codebook::SummaryFlowStepKind::LocalAssignmentBinding.code())).await > 0);
    }
    for name in ["with_identity"] {
        assert_eq!(
            count(
                &ctx,
                &format!(
                    "SELECT count(*) FROM summary_flows f JOIN declarations d \
                     ON d.node_id = f.function_node_id WHERE d.name = '{name}'"
                )
            )
            .await,
            0,
            "{name} cannot complete normally under the current frame model"
        );
        assert_eq!(
            count(
                &ctx,
                &format!(
                    "SELECT count(*) FROM return_exit_statuses x JOIN declarations d \
                     ON d.node_id = x.function_node_id WHERE d.name = '{name}' \
                     AND x.reason = {}",
                    BoundaryReason::UnsupportedControlFlow.code()
                )
            )
            .await,
            1
        );
    }
    assert!(cpg_core::validate::validate(&ctx).await.unwrap().is_empty());
    let original_steps = sql::query(&ctx, "SELECT * FROM summary_flow_steps")
        .await
        .unwrap()
        .into_view();
    let missing_pass_step = sql::query(
        &ctx,
        &format!(
            "SELECT * FROM summary_flow_steps WHERE kind <> {}",
            cpg_schema::codebook::SummaryFlowStepKind::FinalizerPass.code(),
        ),
    )
    .await
    .unwrap()
    .into_view();
    ctx.deregister_table("summary_flow_steps").unwrap();
    ctx.register_table("summary_flow_steps", missing_pass_step)
        .unwrap();
    let violations = cpg_core::validate::validate(&ctx).await.unwrap();
    assert!(
        violations
            .iter()
            .any(|v| v.rule == "summary-flow-step-source-equality"),
        "{violations:?}"
    );
    ctx.deregister_table("summary_flow_steps").unwrap();
    ctx.register_table("summary_flow_steps", original_steps)
        .unwrap();
    let original_steps = sql::query(&ctx, "SELECT * FROM summary_flow_steps")
        .await
        .unwrap()
        .into_view();
    let reversed_passes = sql::query(
        &ctx,
        &format!(
            "SELECT s.* EXCLUDE (ordinal), \
         CASE WHEN d.name = 'nested_finally_pass_identity' \
           AND s.kind = {pass} AND s.ordinal IN (1, 2) \
           THEN 3 - s.ordinal ELSE s.ordinal END AS ordinal \
         FROM summary_flow_steps s JOIN summary_flows f ON f.summary_id = s.summary_id \
         JOIN declarations d ON d.node_id = f.function_node_id",
            pass = cpg_schema::codebook::SummaryFlowStepKind::FinalizerPass.code(),
        ),
    )
    .await
    .unwrap()
    .into_view();
    ctx.deregister_table("summary_flow_steps").unwrap();
    ctx.register_table("summary_flow_steps", reversed_passes)
        .unwrap();
    let violations = cpg_core::validate::validate(&ctx).await.unwrap();
    assert!(
        violations
            .iter()
            .any(|v| v.rule == "summary-flow-step-source-equality"),
        "{violations:?}"
    );
    ctx.deregister_table("summary_flow_steps").unwrap();
    ctx.register_table("summary_flow_steps", original_steps)
        .unwrap();
    let original_steps = sql::query(&ctx, "SELECT * FROM summary_flow_steps")
        .await
        .unwrap()
        .into_view();
    let reversed_suite_passes = sql::query(
        &ctx,
        &format!(
            "SELECT s.* EXCLUDE (ordinal), \
             CASE WHEN d.name = 'multi_pass_finally_identity' \
               AND s.kind = {pass} AND s.ordinal IN (1, 2) \
               THEN 3 - s.ordinal ELSE s.ordinal END AS ordinal \
             FROM summary_flow_steps s JOIN summary_flows f ON f.summary_id = s.summary_id \
             JOIN declarations d ON d.node_id = f.function_node_id",
            pass = cpg_schema::codebook::SummaryFlowStepKind::FinalizerPass.code(),
        ),
    )
    .await
    .unwrap()
    .into_view();
    ctx.deregister_table("summary_flow_steps").unwrap();
    ctx.register_table("summary_flow_steps", reversed_suite_passes)
        .unwrap();
    let violations = cpg_core::validate::validate(&ctx).await.unwrap();
    assert!(
        violations
            .iter()
            .any(|v| v.rule == "summary-flow-step-source-equality"),
        "{violations:?}"
    );
    ctx.deregister_table("summary_flow_steps").unwrap();
    ctx.register_table("summary_flow_steps", original_steps)
        .unwrap();
    let original_statuses = sql::query(&ctx, "SELECT * FROM return_exit_statuses")
        .await
        .unwrap()
        .into_view();
    let forged_pass = sql::query(
        &ctx,
        "SELECT * EXCLUDE (pass_fact_id), \
         CASE WHEN pass_fact_id IS NOT NULL THEN source_fact_id \
              ELSE pass_fact_id END AS pass_fact_id FROM return_exit_statuses",
    )
    .await
    .unwrap()
    .into_view();
    ctx.deregister_table("return_exit_statuses").unwrap();
    ctx.register_table("return_exit_statuses", forged_pass)
        .unwrap();
    let violations = cpg_core::validate::validate(&ctx).await.unwrap();
    assert!(
        violations
            .iter()
            .any(|v| v.rule == "return-exit-status-source-equality"),
        "{violations:?}"
    );
    ctx.deregister_table("return_exit_statuses").unwrap();
    ctx.register_table("return_exit_statuses", original_statuses)
        .unwrap();
}

#[tokio::test(flavor = "multi_thread")]
async fn pysa_tito_control_uses_the_same_source_as_finite_summary_fixture() {
    let fixture = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/python/pysa_tito_shapes/probe/__init__.py");
    let oracle = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../docs/design_review/evidence/2026-09-25_pysa-tito-rule/probe.py");
    assert_eq!(
        std::fs::read(fixture).unwrap(),
        std::fs::read(oracle).unwrap()
    );
    let root = tempfile::tempdir().unwrap();
    let snapshot = Id([57; 16]);
    let analysis = Analysis {
        embedding_cache: None,
        config: AnalyticsConfig::parse(
            r#"
version = 1
[subsystem]
module_prefixes = ["probe"]
public_roots = ["probe"]
[seeds]
primary = ["probe.exercise"]
distractors = []
[pass_a]
max_depth = 2
max_vertices = 128
max_edges = 512
max_witnesses = 3
[briefs]
budget = 1
"#,
        )
        .unwrap(),
        embedder: Some(Arc::new(cpg_core::embedding_service::FakeEmbedder::new())),
        techniques: Techniques::default(),
    };
    compile_analyzed(
        root.path(),
        snapshot,
        &raw_version("pysa_tito_shapes", snapshot, (3, 14, 7)),
        Some(&analysis),
    )
    .await
    .unwrap();
    let (_, ctx) = published(root.path(), snapshot).await.unwrap().unwrap();
    for name in ["identity", "wrapper"] {
        assert_eq!(
            count(
                &ctx,
                &format!(
                    "SELECT count(*) FROM summary_flows f JOIN declarations d \
                ON d.node_id = f.function_node_id WHERE d.name = '{name}' \
                AND f.input_path = 'Parameter[value]' \
                AND f.output_path = 'ReturnValue' AND f.verdict <> {}",
                    Verdict::Unknown.code()
                )
            )
            .await,
            1,
            "{name} has a finite value path on the same source Pysa inspected"
        );
    }
    assert_eq!(
        count(
            &ctx,
            "SELECT count(*) FROM summary_flows f JOIN declarations d \
         ON d.node_id = f.function_node_id WHERE d.name = 'constant'"
        )
        .await,
        0,
        "the constant counter-control has no parameter-to-return summary"
    );
    assert!(cpg_core::validate::validate(&ctx).await.unwrap().is_empty());
}

#[tokio::test(flavor = "multi_thread")]
async fn handler_type_status_is_pinned_or_explicitly_unknown() {
    let root = tempfile::tempdir().unwrap();
    let snapshot = Id([56; 16]);
    compile_behavior(
        root.path(),
        snapshot,
        &raw("handler_shapes", snapshot),
        "plain_raise",
    )
    .await;
    let (_, ctx) = published(root.path(), snapshot).await.unwrap().unwrap();
    assert_eq!(
        count(
            &ctx,
            &format!(
                "SELECT count(*) FROM handler_types h JOIN declarations d \
                 ON d.node_id = h.function_node_id WHERE d.name = 'pinned_handler' \
                 AND h.status = {}",
                HandlerTypeStatus::PinnedBuiltin.code()
            )
        )
        .await,
        1,
        "a non-shadowed builtin handler has one pinned class identity"
    );
    assert_eq!(
        count(
            &ctx,
            &format!(
                "SELECT count(*) FROM handler_types h JOIN declarations d \
                 ON d.node_id = h.function_node_id WHERE d.name = 'shadowed_handler' \
                 AND h.status = {} AND h.reason IS NOT NULL",
                HandlerTypeStatus::Unknown.code()
            )
        )
        .await,
        1
    );
    assert_eq!(
        count(
            &ctx,
            &format!(
                "SELECT count(*) FROM handler_types h JOIN declarations d \
                 ON d.node_id = h.function_node_id WHERE d.name = 'bare_handler' \
                 AND h.status = {} AND h.reason IS NULL",
                HandlerTypeStatus::Bare.code()
            )
        )
        .await,
        1
    );
    assert!(cpg_core::validate::validate(&ctx).await.unwrap().is_empty());
    let original_types = sql::query(&ctx, "SELECT * FROM handler_types")
        .await
        .unwrap()
        .into_view();
    let doctored_types = sql::query(
        &ctx,
        &format!(
            "SELECT * EXCLUDE (status), CAST({} AS SMALLINT) AS status FROM handler_types",
            HandlerTypeStatus::Bare.code()
        ),
    )
    .await
    .unwrap()
    .into_view();
    ctx.deregister_table("handler_types").unwrap();
    ctx.register_table("handler_types", doctored_types).unwrap();
    let violations = cpg_core::validate::validate(&ctx).await.unwrap();
    let (expected,) = ("semantic:handler-type-status-shape",);
    assert!(
        violations.iter().any(|v| v.rule == expected),
        "{violations:?}"
    );
    assert!(
        violations
            .iter()
            .any(|v| v.rule == "handler-type-source-equality"),
        "{violations:?}"
    );
    ctx.deregister_table("handler_types").unwrap();
    ctx.register_table("handler_types", original_types).unwrap();
}

#[tokio::test(flavor = "multi_thread")]
async fn modeled_exception_handler_candidates_follow_exact_try_body_ancestry() {
    let root = tempfile::tempdir().unwrap();
    let snapshot = Id([59; 16]);
    let analysis = Analysis {
        embedding_cache: None,
        config: AnalyticsConfig::parse(
            r#"
version = 1
[subsystem]
module_prefixes = ["handlerpkg"]
public_roots = ["handlerpkg"]
[seeds]
primary = ["handlerpkg.exact_handler"]
distractors = []
[pass_a]
max_depth = 2
max_vertices = 128
max_edges = 512
max_witnesses = 3
[briefs]
budget = 1
"#,
        )
        .unwrap(),
        embedder: Some(Arc::new(cpg_core::embedding_service::FakeEmbedder::new())),
        techniques: Techniques::default(),
    };
    compile_analyzed(
        root.path(),
        snapshot,
        &raw_version("model_handler_shapes", snapshot, (3, 14, 7)),
        Some(&analysis),
    )
    .await
    .unwrap();
    let (_, ctx) = published(root.path(), snapshot).await.unwrap().unwrap();
    assert_eq!(
        count(
            &ctx,
            &format!(
                "SELECT count(*) FROM context_class_mro m \
                 JOIN context_definitions c ON c.symbol_node_id = m.class_node_id \
                 JOIN context_definitions a ON a.module_name = m.ancestor_module \
                   AND a.key = m.ancestor_key AND a.kind = {} \
                 WHERE c.module_name = 'builtins' AND c.qualified_name = 'OSError' \
                   AND a.qualified_name = 'Exception' AND NOT m.cyclic",
                DefinitionKind::Class.code()
            )
        )
        .await,
        1,
        "the pinned Pyrefly MRO relates OSError to the referenced Exception class"
    );
    for (name, class_match, expected) in [
        ("exact_handler", ModeledHandlerClassMatch::SameClass, 1),
        (
            "broader_handler",
            ModeledHandlerClassMatch::PinnedAncestor,
            1,
        ),
        ("bare_handler", ModeledHandlerClassMatch::Bare, 1),
        (
            "shadowed_handler",
            ModeledHandlerClassMatch::HandlerTypeUnknown,
            1,
        ),
        ("outside_handler", ModeledHandlerClassMatch::SameClass, 0),
        ("inner", ModeledHandlerClassMatch::SameClass, 0),
        ("nested_tries", ModeledHandlerClassMatch::SameClass, 1),
        (
            "nested_tries",
            ModeledHandlerClassMatch::ClassRelationUnknown,
            1,
        ),
    ] {
        let found = count(
            &ctx,
            &format!(
                "SELECT count(*) FROM modeled_exception_handler_candidates c \
                 JOIN syntax_nodes s ON s.node_id = c.call_site_node_id \
                 JOIN declarations d ON d.node_id = s.owner_node_id \
                 WHERE d.name = '{name}' AND c.class_match = {}",
                class_match.code(),
            ),
        )
        .await;
        assert_eq!(found, expected, "{name}");
    }
    for (name, ordinal, possible, first) in [
        ("known_first", 0, true, true),
        ("known_first", 1, false, false),
        ("unknown_first", 0, true, false),
        ("unknown_first", 1, true, false),
    ] {
        assert_eq!(
            count(
                &ctx,
                &format!(
                    "SELECT count(*) FROM modeled_exception_handler_candidates c \
                     JOIN syntax_nodes s ON s.node_id = c.call_site_node_id \
                     JOIN declarations d ON d.node_id = s.owner_node_id \
                     WHERE d.name = '{name}' AND c.handler_ordinal = {ordinal} \
                       AND c.frame_possible = {possible} \
                       AND c.frame_first_match_if_raised = {first}"
                ),
            )
            .await,
            1,
            "{name} clause {ordinal}"
        );
    }
    assert_eq!(
        count(
            &ctx,
            "SELECT count(*) FROM handler_return_none_sites r \
             JOIN declarations d ON d.node_id = r.function_node_id \
             WHERE d.name = 'exact_handler' AND r.approximated"
        )
        .await,
        1,
        "a sole direct return of None retains its approximate ty region"
    );
    assert_eq!(
        count(
            &ctx,
            "SELECT count(*) FROM handler_return_none_sites r \
             JOIN declarations d ON d.node_id = r.function_node_id \
             WHERE d.name IN ('computed_handler', 'two_action_handler')"
        )
        .await,
        0,
        "a computed return or an earlier action cannot obtain the direct-return witness"
    );
    assert_eq!(
        count(
            &ctx,
            "SELECT count(*) FROM modeled_exception_return_none_paths p \
             JOIN syntax_nodes s ON s.node_id = p.call_site_node_id \
             JOIN declarations d ON d.node_id = s.owner_node_id \
             WHERE d.name = 'exact_handler' AND p.handler_region_approximated"
        )
        .await,
        1,
        "a modeled raise has a conditional path through the first exact handler to return None"
    );
    assert_eq!(
        count(
            &ctx,
            "SELECT count(*) FROM modeled_exception_return_none_paths p \
             JOIN syntax_nodes s ON s.node_id = p.call_site_node_id \
             JOIN declarations d ON d.node_id = s.owner_node_id \
             WHERE d.name IN ('unknown_first', 'nested_tries', 'computed_handler', \
               'two_action_handler', 'with_intervening', 'finalizer_changes_return')"
        )
        .await,
        0,
        "unknown earlier clauses, inner frames, computed/multiple body actions, with and finally remain open"
    );
    assert_eq!(
        count(
            &ctx,
            "SELECT count(*) FROM modeled_exception_handler_candidates c \
             JOIN syntax_nodes s ON s.node_id = c.call_site_node_id \
             JOIN declarations d ON d.node_id = s.owner_node_id \
             JOIN context_class_mro m ON m.fact_id = c.class_mro_fact_id \
             WHERE d.name = 'broader_handler' AND m.ancestor_name = 'Exception'"
        )
        .await,
        1,
        "the broader handler cites the exact pinned MRO fact"
    );
    assert_eq!(
        count(
            &ctx,
            "SELECT count(DISTINCT try_node_id) FROM modeled_exception_handler_candidates c \
             JOIN syntax_nodes s ON s.node_id = c.call_site_node_id \
             JOIN declarations d ON d.node_id = s.owner_node_id \
             WHERE d.name = 'nested_tries'"
        )
        .await,
        2,
        "both nested try frames remain distinct"
    );
    assert_eq!(
        count(
            &ctx,
            "SELECT count(*) FROM modeled_exception_handler_walks WHERE reason IS NOT NULL"
        )
        .await,
        0,
        "the focused fixture has a complete syntax-ancestor walk for every modeled raise"
    );
    assert_eq!(
        count(&ctx, "SELECT count(*) FROM modeled_exception_handler_walks").await,
        count(
            &ctx,
            &format!(
                "SELECT count(*) FROM modeled_exception_sites WHERE action = {}",
                ModelExceptionAction::Raise.code()
            )
        )
        .await,
        "every modeled raise has a walk coverage row"
    );
    assert!(
        count(
            &ctx,
            &format!(
                "SELECT count(*) FROM value_flow_contributions c \
                 JOIN flow_values v ON v.fact_id = c.flow_value_fact_id \
                 JOIN flow_value_call_links l ON l.flow_value_fact_id = v.fact_id \
                 WHERE v.sink = {} AND c.parameter_node_id IS NOT NULL \
                   AND c.through_call AND l.status = {}",
                FlowSink::Return.code(),
                FlowCallLinkStatus::BoundArgument.code(),
            ),
        )
        .await
            > 0,
        "an unaggregated parameter contribution retains its exact modeled call-path parent"
    );
    assert_eq!(
        count(
            &ctx,
            &format!(
                "SELECT count(*) FROM value_flow_contributions c \
                 JOIN flow_values v ON v.fact_id = c.flow_value_fact_id \
                 JOIN flow_uses u ON u.use_id = c.use_id \
                 JOIN declarations d ON d.node_id = c.function_node_id \
                 WHERE d.name = 'indirect_call_result' AND v.sink = {} \
                   AND c.through_call AND NOT c.local_through_call \
                   AND c.upstream_through_call AND NOT c.upstream_identity \
                   AND u.place = 'value'",
                FlowSink::Return.code(),
            ),
        )
        .await,
        1,
        "an inherited call transfer does not borrow its return-use fact as the call path"
    );
    assert!(
        count(
            &ctx,
            &format!(
                "SELECT count(*) FROM value_flow_contributions c \
                 JOIN flow_values v ON v.fact_id = c.flow_value_fact_id \
                 JOIN declarations d ON d.node_id = c.sink_function_node_id \
                 WHERE d.name = 'exact_handler' AND v.sink = {} \
                   AND c.local_through_call AND c.upstream_identity \
                   AND NOT c.upstream_through_call",
                FlowSink::Return.code(),
            )
        )
        .await
            > 0,
        "a direct modeled call has an identity upstream input before its local call path"
    );
    assert!(
        count(
            &ctx,
            "SELECT count(*) FROM value_flow_contributions c \
             JOIN declarations sink ON sink.node_id = c.sink_function_node_id \
             WHERE sink.name = 'indirect_call_result' AND c.parameter_node_id IS NOT NULL \
               AND c.sink_function_node_id = c.function_node_id"
        )
        .await
            > 0,
        "a local source contribution names the callable containing its raw sink use"
    );
    assert!(cpg_core::validate::validate(&ctx).await.unwrap().is_empty());
    let original_return_sites = sql::query(&ctx, "SELECT * FROM handler_return_none_sites")
        .await
        .unwrap()
        .into_view();
    let dropped_return_sites =
        sql::query(&ctx, "SELECT * FROM handler_return_none_sites WHERE false")
            .await
            .unwrap()
            .into_view();
    ctx.deregister_table("handler_return_none_sites").unwrap();
    ctx.register_table("handler_return_none_sites", dropped_return_sites)
        .unwrap();
    let violations = cpg_core::validate::validate(&ctx).await.unwrap();
    assert!(
        violations
            .iter()
            .any(|v| v.rule == "handler-return-none-source-equality"),
        "{violations:?}"
    );
    ctx.deregister_table("handler_return_none_sites").unwrap();
    ctx.register_table("handler_return_none_sites", original_return_sites)
        .unwrap();
    let original_paths = sql::query(&ctx, "SELECT * FROM modeled_exception_return_none_paths")
        .await
        .unwrap()
        .into_view();
    let dropped_paths = sql::query(
        &ctx,
        "SELECT * FROM modeled_exception_return_none_paths WHERE false",
    )
    .await
    .unwrap()
    .into_view();
    ctx.deregister_table("modeled_exception_return_none_paths")
        .unwrap();
    ctx.register_table("modeled_exception_return_none_paths", dropped_paths)
        .unwrap();
    let violations = cpg_core::validate::validate(&ctx).await.unwrap();
    assert!(
        violations
            .iter()
            .any(|v| v.rule == "modeled-exception-return-none-path-source-equality"),
        "{violations:?}"
    );
    ctx.deregister_table("modeled_exception_return_none_paths")
        .unwrap();
    ctx.register_table("modeled_exception_return_none_paths", original_paths)
        .unwrap();
    let original_contributions = sql::query(&ctx, "SELECT * FROM value_flow_contributions")
        .await
        .unwrap()
        .into_view();
    let dropped_contributions =
        sql::query(&ctx, "SELECT * FROM value_flow_contributions WHERE false")
            .await
            .unwrap()
            .into_view();
    ctx.deregister_table("value_flow_contributions").unwrap();
    ctx.register_table("value_flow_contributions", dropped_contributions)
        .unwrap();
    let violations = cpg_core::validate::validate(&ctx).await.unwrap();
    assert!(
        violations
            .iter()
            .any(|v| v.rule == "value-flow-contribution-source-equality"),
        "{violations:?}"
    );
    ctx.deregister_table("value_flow_contributions").unwrap();
    ctx.register_table("value_flow_contributions", original_contributions)
        .unwrap();
    let original_candidates =
        sql::query(&ctx, "SELECT * FROM modeled_exception_handler_candidates")
            .await
            .unwrap()
            .into_view();
    let doctored = sql::query(
        &ctx,
        &format!(
            "SELECT * EXCLUDE (class_match), CAST({} AS SMALLINT) AS class_match \
             FROM modeled_exception_handler_candidates",
            ModeledHandlerClassMatch::Bare.code(),
        ),
    )
    .await
    .unwrap()
    .into_view();
    ctx.deregister_table("modeled_exception_handler_candidates")
        .unwrap();
    ctx.register_table("modeled_exception_handler_candidates", doctored)
        .unwrap();
    let violations = cpg_core::validate::validate(&ctx).await.unwrap();
    assert!(
        violations
            .iter()
            .any(|v| v.rule == "modeled-exception-handler-source-equality"),
        "{violations:?}"
    );
    ctx.deregister_table("modeled_exception_handler_candidates")
        .unwrap();
    ctx.register_table("modeled_exception_handler_candidates", original_candidates)
        .unwrap();
    let original_mro = sql::query(&ctx, "SELECT * FROM context_class_mro")
        .await
        .unwrap()
        .into_view();
    let missing_mro = sql::query(&ctx, "SELECT * FROM context_class_mro WHERE false")
        .await
        .unwrap()
        .into_view();
    ctx.deregister_table("context_class_mro").unwrap();
    ctx.register_table("context_class_mro", missing_mro)
        .unwrap();
    let violations = cpg_core::validate::validate(&ctx).await.unwrap();
    assert!(
        violations
            .iter()
            .any(|v| v.rule == "semantic:context-class-mro-coverage"),
        "{violations:?}"
    );
    ctx.deregister_table("context_class_mro").unwrap();
    ctx.register_table("context_class_mro", original_mro.clone())
        .unwrap();
    let broken_ordinals = sql::query(
        &ctx,
        "SELECT * EXCLUDE (ordinal), \
         CASE WHEN ordinal IS NULL THEN NULL ELSE CAST(0 AS BIGINT) END AS ordinal \
         FROM context_class_mro",
    )
    .await
    .unwrap()
    .into_view();
    ctx.deregister_table("context_class_mro").unwrap();
    ctx.register_table("context_class_mro", broken_ordinals)
        .unwrap();
    let violations = cpg_core::validate::validate(&ctx).await.unwrap();
    assert!(
        violations
            .iter()
            .any(|v| v.rule == "semantic:context-class-mro-shape"),
        "{violations:?}"
    );
    ctx.deregister_table("context_class_mro").unwrap();
    ctx.register_table("context_class_mro", original_mro.clone())
        .unwrap();
    let wrong_module = sql::query(
        &ctx,
        "SELECT * EXCLUDE (module_node_id), class_node_id AS module_node_id \
         FROM context_class_mro",
    )
    .await
    .unwrap()
    .into_view();
    ctx.deregister_table("context_class_mro").unwrap();
    ctx.register_table("context_class_mro", wrong_module)
        .unwrap();
    let violations = cpg_core::validate::validate(&ctx).await.unwrap();
    assert!(
        violations
            .iter()
            .any(|v| v.rule == "semantic:context-class-mro-identity"),
        "{violations:?}"
    );
    ctx.deregister_table("context_class_mro").unwrap();
    ctx.register_table("context_class_mro", original_mro)
        .unwrap();
}

#[tokio::test(flavor = "multi_thread")]
async fn unresolved_frames_withhold_raise_escape_guards() {
    let root = tempfile::tempdir().unwrap();
    let snapshot = Id([57; 16]);
    let analysis = Analysis {
        embedding_cache: None,
        config: AnalyticsConfig::parse(
            r#"
version = 1
[subsystem]
module_prefixes = ["handlerpkg"]
public_roots = ["handlerpkg"]
[seeds]
primary = ["handlerpkg.plain_raise"]
distractors = []
[pass_a]
max_depth = 2
max_vertices = 128
max_edges = 512
max_witnesses = 3
[briefs]
budget = 1
"#,
        )
        .unwrap(),
        embedder: Some(Arc::new(cpg_core::embedding_service::FakeEmbedder::new())),
        techniques: Techniques::default(),
    };
    compile_analyzed(
        root.path(),
        snapshot,
        &raw("handler_shapes", snapshot),
        Some(&analysis),
    )
    .await
    .unwrap();
    let (_, ctx) = published(root.path(), snapshot).await.unwrap().unwrap();
    assert_eq!(
        count(
            &ctx,
            "SELECT count(*) FROM raise_sites r JOIN declarations d \
             ON d.node_id = r.function_node_id \
             WHERE d.name IN ('opaque_with', 'finally_overrides', 'unrelated_handler') \
             AND NOT r.escapes",
        )
        .await,
        3,
        "unresolved frame behavior cannot establish an escaping raise"
    );
    assert_eq!(
        count(
            &ctx,
            "SELECT count(*) FROM raise_sites r JOIN declarations d \
             ON d.node_id = r.function_node_id \
             WHERE d.name = 'plain_raise' AND r.escapes",
        )
        .await,
        1,
        "an unframed explicit raise remains an escape witness"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn handler_clauses_and_actions_are_structural_and_validated() {
    let root = tempfile::tempdir().unwrap();
    let snapshot = Id([55; 16]);
    compile_behavior(
        root.path(),
        snapshot,
        &raw("flow_shapes", snapshot),
        "fallback",
    )
    .await;
    let (_, ctx) = published(root.path(), snapshot).await.unwrap().unwrap();
    assert_eq!(
        count(
            &ctx,
            "SELECT count(*) FROM handler_clauses h JOIN declarations d \
             ON d.node_id = h.function_node_id WHERE d.name = 'handled' AND h.type_node_id IS NOT NULL"
        )
        .await,
        1
    );
    assert_eq!(
        count(
            &ctx,
            "SELECT count(*) FROM handler_clauses h JOIN declarations d \
             ON d.node_id = h.function_node_id WHERE d.name = 'guarded'"
        )
        .await,
        0
    );
    assert_eq!(
        count(
            &ctx,
            "SELECT count(*) FROM handler_actions a JOIN handler_clauses h \
             ON h.handler_node_id = a.handler_node_id JOIN declarations d \
             ON d.node_id = h.function_node_id WHERE d.name = 'handled'"
        )
        .await,
        1,
        "the finally assignment is outside the except body"
    );
    assert!(cpg_core::validate::validate(&ctx).await.unwrap().is_empty());
    let doctored = sql::query(
        &ctx,
        "SELECT * EXCLUDE (ordinal), ordinal + 1 AS ordinal FROM handler_clauses",
    )
    .await
    .unwrap()
    .into_view();
    ctx.deregister_table("handler_clauses").unwrap();
    ctx.register_table("handler_clauses", doctored).unwrap();
    let violations = cpg_core::validate::validate(&ctx).await.unwrap();
    assert!(
        violations
            .iter()
            .any(|v| v.rule == "handler-clause-source-equality"),
        "{violations:?}"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn model_target_requires_its_cited_pinned_definition() {
    let root = tempfile::tempdir().unwrap();
    let snapshot = Id([48; 16]);
    compile_behavior(
        root.path(),
        snapshot,
        &raw("flow_shapes", snapshot),
        "fallback",
    )
    .await;
    let (_, ctx) = published(root.path(), snapshot).await.unwrap().unwrap();
    assert!(cpg_core::validate::validate(&ctx).await.unwrap().is_empty());
    let forged = ModelTargets::to_batch(&[ModelTargetsRow {
        snapshot_id: snapshot,
        model_id: Id([1; 16]),
        target_node_id: Id([2; 16]),
        target_module_fact_id: Id([3; 16]),
        target_definition_fact_id: Id([4; 16]),
        target_key: "stdlib:3.14.7:typing.cast".into(),
        revision: 1,
        phase: cpg_schema::codebook::InvocationPhase::Call,
        transfer_coverage: ModelChannelCoverage::Complete,
        effect_coverage: ModelChannelCoverage::Complete,
        callback_coverage: ModelChannelCoverage::Complete,
        resource_coverage: ModelChannelCoverage::Complete,
        exception_coverage: ModelChannelCoverage::Complete,
        body_return_parameter: Some("val".into()),
        call_defaults_available: false,
        origin: Origin::SyntheticModel,
    }])
    .unwrap();
    ctx.deregister_table("model_targets").unwrap();
    ctx.register_batch("model_targets", forged).unwrap();
    let violations = cpg_core::validate::validate(&ctx).await.unwrap();
    let (expected,) = ("semantic:model-target-provenance",);
    assert!(
        violations.iter().any(|v| v.rule == expected),
        "{violations:?}"
    );
    assert!(
        violations
            .iter()
            .any(|v| v.rule == "model-catalog-target-equality"),
        "{violations:?}"
    );

    ctx.deregister_table("model_targets").unwrap();
    ctx.register_batch("model_targets", ModelTargets::to_batch(&[]).unwrap())
        .unwrap();
    let forged_transfer = ModelTransfers::to_batch(&[ModelTransfersRow {
        snapshot_id: snapshot,
        model_id: Id([1; 16]),
        target_node_id: Id([2; 16]),
        rule_id: Id([5; 16]),
        target_definition_fact_id: Id([4; 16]),
        revision: 1,
        input_path_id: Id([12; 16]),
        input_path_kind: ModelPathKind::Parameter,
        input_path: "Parameter[val]".into(),
        output_path_id: Id([13; 16]),
        output_path_kind: ModelPathKind::ReturnValue,
        output_path: "ReturnValue".into(),
        transfer: ModelTransferKind::Identity,
        modality: Modality::Definite,
        origin: Origin::SyntheticModel,
    }])
    .unwrap();
    ctx.deregister_table("model_transfers").unwrap();
    ctx.register_batch("model_transfers", forged_transfer)
        .unwrap();
    let violations = cpg_core::validate::validate(&ctx).await.unwrap();
    let (expected,) = ("semantic:model-transfer-target",);
    assert!(
        violations.iter().any(|v| v.rule == expected),
        "{violations:?}"
    );
    assert!(
        violations
            .iter()
            .any(|v| v.rule == "model-catalog-transfer-equality"),
        "{violations:?}"
    );

    let forged_effect = ModelEffects::to_batch(&[ModelEffectsRow {
        snapshot_id: snapshot,
        model_id: Id([1; 16]),
        target_node_id: Id([2; 16]),
        rule_id: Id([6; 16]),
        target_definition_fact_id: Id([4; 16]),
        revision: 1,
        effect: ModelEffectKind::IoRead,
        exit: cpg_schema::codebook::ModelExit::Invocation,
        argument: None,
        schema_kind: None,
        schema_class_node_id: None,
        schema_class_fact_id: None,
        schema_path_id: None,
        schema_path_kind: None,
        subject_path_id: None,
        subject_path_kind: None,
        subject_path: None,
        modality: Modality::Potential,
        origin: Origin::SyntheticModel,
    }])
    .unwrap();
    ctx.deregister_table("model_effects").unwrap();
    ctx.register_batch("model_effects", forged_effect).unwrap();
    let violations = cpg_core::validate::validate(&ctx).await.unwrap();
    let (expected,) = ("semantic:model-effect-target",);
    assert!(
        violations.iter().any(|v| v.rule == expected),
        "{violations:?}"
    );
    assert!(
        violations
            .iter()
            .any(|v| v.rule == "model-catalog-effect-equality"),
        "{violations:?}"
    );

    let forged_callback = ModelCallbacks::to_batch(&[ModelCallbacksRow {
        snapshot_id: snapshot,
        model_id: Id([1; 16]),
        target_node_id: Id([2; 16]),
        rule_id: Id([7; 16]),
        target_definition_fact_id: Id([4; 16]),
        revision: 1,
        callback_path_id: Id([8; 16]),
        callback_path: "Parameter[callback]".into(),
        action: ModelCallbackAction::Registered,
        exit: ModelExit::Normal,
        modality: Modality::Definite,
        origin: Origin::SyntheticModel,
    }])
    .unwrap();
    ctx.deregister_table("model_callbacks").unwrap();
    ctx.register_batch("model_callbacks", forged_callback)
        .unwrap();
    let forged_resource = ModelResources::to_batch(&[ModelResourcesRow {
        snapshot_id: snapshot,
        model_id: Id([1; 16]),
        target_node_id: Id([2; 16]),
        rule_id: Id([9; 16]),
        target_definition_fact_id: Id([4; 16]),
        revision: 1,
        resource_path_id: Id([10; 16]),
        resource_path: "ReturnValue".into(),
        resource_role: ModelPathRole::Output,
        resource_path_kind: ModelPathKind::ReturnValue,
        action: ModelResourceAction::Acquire,
        exit: ModelExit::Normal,
        modality: Modality::Definite,
        origin: Origin::SyntheticModel,
    }])
    .unwrap();
    ctx.deregister_table("model_resources").unwrap();
    ctx.register_batch("model_resources", forged_resource)
        .unwrap();
    let forged_exception = ModelExceptions::to_batch(&[ModelExceptionsRow {
        snapshot_id: snapshot,
        model_id: Id([1; 16]),
        target_node_id: Id([2; 16]),
        rule_id: Id([11; 16]),
        target_definition_fact_id: Id([4; 16]),
        revision: 1,
        class: "builtins.OSError".into(),
        class_node_id: Id([12; 16]),
        class_fact_id: Id([13; 16]),
        action: ModelExceptionAction::Convert,
        to_class: None,
        to_class_node_id: None,
        to_class_fact_id: None,
        modality: Modality::Potential,
        origin: Origin::SyntheticModel,
    }])
    .unwrap();
    ctx.deregister_table("model_exceptions").unwrap();
    ctx.register_batch("model_exceptions", forged_exception)
        .unwrap();
    let violations = cpg_core::validate::validate(&ctx).await.unwrap();
    for expected in [
        ("semantic:model-callback-target",),
        ("semantic:model-resource-target",),
        ("semantic:model-exception-target",),
        ("semantic:model-exception-shape",),
        ("semantic:model-exception-class-identity",),
    ] {
        assert!(
            violations.iter().any(|v| v.rule == expected.0),
            "missing {} in {violations:?}",
            expected.0
        );
    }
    for expected in [
        "model-catalog-callback-equality",
        "model-catalog-resource-equality",
        "model-catalog-exception-equality",
    ] {
        assert!(
            violations.iter().any(|v| v.rule == expected),
            "missing {expected} in {violations:?}"
        );
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn pinned_identity_models_require_and_publish_their_real_formals() {
    let root = tempfile::tempdir().unwrap();
    let snapshot = Id([52; 16]);
    let analysis = Analysis {
        embedding_cache: None,
        config: AnalyticsConfig::parse(
            r#"
version = 1
[subsystem]
module_prefixes = ["modelpkg"]
public_roots = ["modelpkg"]
[seeds]
primary = ["modelpkg.identity"]
distractors = []
[pass_a]
max_depth = 2
max_vertices = 128
max_edges = 512
max_witnesses = 3
[briefs]
budget = 1
"#,
        )
        .unwrap(),
        embedder: Some(Arc::new(cpg_core::embedding_service::FakeEmbedder::new())),
        techniques: Techniques::default(),
    };
    compile_analyzed(
        root.path(),
        snapshot,
        &raw_version("model_shapes", snapshot, (3, 14, 7)),
        Some(&analysis),
    )
    .await
    .unwrap();
    let (_, ctx) = published(root.path(), snapshot).await.unwrap().unwrap();
    let targets = count(&ctx, "SELECT count(*) FROM model_targets").await;
    // Stdlib models bind only where the fixture reaches their pinned definitions (warning,
    // info and log of the Logger family among them).
    assert_eq!(
        targets, 13,
        "the empty fixture site-packages leaves dependency models dormant"
    );
    assert_eq!(
        count(
            &ctx,
            "SELECT count(*) FROM model_targets WHERE target_key = \
             'dependency:pydantic==2.13.5:pydantic.type_adapter.TypeAdapter.validate_python'"
        )
        .await,
        0,
        "a dependency model cannot bind without that dependency's pinned context"
    );
    assert_eq!(
        count(
            &ctx,
            "SELECT count(*) FROM model_targets WHERE body_return_parameter IS NOT NULL"
        )
        .await,
        2,
        "only the two pinned typing identity helpers assert the direct-return body"
    );
    assert!(count(&ctx, "SELECT count(*) FROM model_frame_exits").await > 0);
    assert_eq!(
        count(
            &ctx,
            "SELECT count(*) FROM model_frame_exits f JOIN declarations d \
        ON d.node_id=f.function_node_id WHERE d.name='ClassBodyFrame'"
        )
        .await,
        0,
        "class LOAD_NAME and custom prepared namespaces cannot borrow function retention"
    );
    let frames = sql::query(&ctx, "SELECT * FROM model_frame_exit_arguments")
        .await
        .unwrap()
        .into_view();
    let empty = sql::query(&ctx, "SELECT * FROM model_frame_exit_arguments WHERE false")
        .await
        .unwrap()
        .into_view();
    ctx.deregister_table("model_frame_exit_arguments").unwrap();
    ctx.register_table("model_frame_exit_arguments", empty)
        .unwrap();
    assert!(
        cpg_core::validate::validate(&ctx)
            .await
            .unwrap()
            .iter()
            .any(|v| v.rule == "expression-source-equality"),
        "removing the entire release domain must fail source reconstruction"
    );
    ctx.deregister_table("model_frame_exit_arguments").unwrap();
    ctx.register_table("model_frame_exit_arguments", frames)
        .unwrap();
    let original_model_targets = sql::query(&ctx, "SELECT * FROM model_targets")
        .await
        .unwrap()
        .into_view();
    let before: Vec<cpg_schema::behavior::ModelApplicationsRow> = sql::fetch(
        &ctx,
        &cpg_schema::behavior::model_applications(),
        sql::Params::new(),
    )
    .await
    .unwrap();
    assert!(!before.is_empty());
    let wrong_phase = sql::query(
        &ctx,
        "SELECT * EXCLUDE (phase), CAST(2 AS SMALLINT) AS phase FROM model_targets",
    )
    .await
    .unwrap()
    .into_view();
    ctx.deregister_table("model_targets").unwrap();
    ctx.register_table("model_targets", wrong_phase).unwrap();
    let after: Vec<cpg_schema::behavior::ModelApplicationsRow> = sql::fetch(
        &ctx,
        &cpg_schema::behavior::model_applications(),
        sql::Params::new(),
    )
    .await
    .unwrap();
    assert!(
        after.is_empty(),
        "observed Call cannot activate authored Init coverage"
    );
    let violations = cpg_core::validate::validate(&ctx).await.unwrap();
    assert!(
        violations
            .iter()
            .any(|v| v.rule == "model-catalog-target-equality")
    );
    ctx.deregister_table("model_targets").unwrap();
    ctx.register_table("model_targets", original_model_targets.clone())
        .unwrap();
    let forged_completion = sql::query(
        &ctx,
        "SELECT * EXCLUDE (body_return_parameter), CAST(NULL AS VARCHAR) AS body_return_parameter FROM model_targets",
    )
    .await
    .unwrap()
    .into_view();
    ctx.deregister_table("model_targets").unwrap();
    ctx.register_table("model_targets", forged_completion)
        .unwrap();
    let violations = cpg_core::validate::validate(&ctx).await.unwrap();
    assert!(
        violations
            .iter()
            .any(|v| v.rule == "model-catalog-target-equality"),
        "{violations:?}"
    );
    ctx.deregister_table("model_targets").unwrap();
    ctx.register_table("model_targets", original_model_targets)
        .unwrap();
    assert!(
        count(&ctx, "SELECT count(*) FROM flow_value_calls").await > 0,
        "source value uses inside calls retain their ordered raw call steps"
    );
    assert_eq!(count(&ctx, "SELECT count(*) FROM model_transfers").await, 7);
    assert!(count(&ctx, "SELECT count(*) FROM analysis_conditions").await > 0);
    assert_eq!(
        count(
            &ctx,
            &format!(
                "SELECT count(*) FROM summary_flows f \
                 JOIN summary_flow_steps p ON p.summary_id = f.summary_id \
                 JOIN declarations d ON d.node_id = f.function_node_id \
                 WHERE d.name = 'plain_identity' AND f.input_path = 'Parameter[value]' \
                   AND f.output_path = 'ReturnValue' AND f.path_depth = 0 \
                   AND p.ordinal = 0 AND p.evidence_id = f.source_flow_fact_id \
                   AND f.verdict <> {} AND f.boundary_reason IS NULL",
                Verdict::Unknown.code()
            ),
        )
        .await,
        1,
        "a direct synchronous identity return has one finite condition-backed summary"
    );
    assert_eq!(
        count(
            &ctx,
            "SELECT count(*) FROM summary_components caller \
             JOIN declarations cd ON cd.node_id = caller.function_node_id \
             JOIN summary_components callee \
               ON callee.snapshot_id = caller.snapshot_id \
             JOIN declarations td ON td.node_id = callee.function_node_id \
             WHERE cd.name = 'local_wrapper' AND td.name = 'plain_identity' \
               AND caller.component_order > callee.component_order",
        )
        .await,
        1,
        "a local caller is scheduled after its callee"
    );
    assert_eq!(
        count(
            &ctx,
            &format!(
                "SELECT count(*) FROM summary_flows f \
                 JOIN summary_flow_steps p ON p.summary_id = f.summary_id \
                   AND p.kind = {} \
                 JOIN summary_flows callee ON callee.summary_id = p.evidence_id \
                 JOIN declarations d ON d.node_id = f.function_node_id \
                 JOIN declarations td ON td.node_id = callee.function_node_id \
                 WHERE d.name = 'local_wrapper' AND td.name = 'plain_identity' \
                   AND f.path_depth = 1 AND f.verdict <> {}",
                cpg_schema::codebook::SummaryFlowStepKind::CalleeSummary.code(),
                Verdict::Unknown.code(),
            ),
        )
        .await,
        1,
        "an exact local wrapper inherits the unconditional callee value flow"
    );
    assert_eq!(
        count(
            &ctx,
            "SELECT count(*) FROM summary_flows f \
             JOIN declarations d ON d.node_id = f.function_node_id \
             WHERE d.name IN ('async_identity', 'generator_identity')",
        )
        .await,
        0,
        "deferred execution cannot seed a synchronous positive flow"
    );
    assert_eq!(
        count(
            &ctx,
            "SELECT count(*) FROM summary_flows f \
             JOIN declarations d ON d.node_id = f.function_node_id \
             JOIN return_exit_statuses x ON x.function_node_id = d.node_id \
             WHERE d.name = 'framed_identity' AND x.reason IS NULL \
               AND x.pass_fact_id IS NOT NULL",
        )
        .await,
        1,
        "one pass finalizer leaves a cited normal-return candidate"
    );
    assert_eq!(
        count(
            &ctx,
            &format!(
                "SELECT count(*) FROM summary_flows f \
             JOIN declarations d ON d.node_id = f.function_node_id \
             JOIN return_exit_statuses x ON x.source_fact_id = f.return_site_fact_id \
             JOIN summary_flow_steps p ON p.summary_id = f.summary_id \
               AND p.kind = {pass} AND p.evidence_id = x.pass_fact_id \
             JOIN summary_flow_steps r ON r.summary_id = f.summary_id \
               AND r.kind = {exit} AND r.ordinal = p.ordinal + 1 \
             WHERE d.name = 'framed_modeled_identity' AND f.path_depth = 1",
                pass = cpg_schema::codebook::SummaryFlowStepKind::FinalizerPass.code(),
                exit = cpg_schema::codebook::SummaryFlowStepKind::ReturnExit.code(),
            )
        )
        .await,
        1,
        "a modeled return cites its finalizer before the completed exit: {} / {}",
        sql::render(&ctx,"SELECT s.ordinal,s.kind,s.evidence_id FROM summary_flow_steps s JOIN summary_flows f \
            ON f.summary_id=s.summary_id JOIN declarations d ON d.node_id=f.function_node_id \
            WHERE d.name='framed_modeled_identity' ORDER BY s.ordinal").await.unwrap(),
        sql::render(&ctx,"SELECT b.reason FROM summary_boundaries b JOIN declarations d ON d.node_id=b.function_node_id \
            WHERE d.name='framed_modeled_identity'").await.unwrap()
    );
    assert_eq!(
        count(
            &ctx,
            &format!(
                "SELECT count(*) FROM modeled_argument_evaluations a \
            JOIN modeled_exact_value_transfers m \
              ON m.flow_value_fact_id = a.candidate_flow_fact_id \
             AND m.parameter_node_id = a.parameter_node_id \
             AND m.pysa_fact_id = a.pysa_fact_id \
             AND m.model_id = a.model_id AND m.rule_id = a.rule_id \
            JOIN declarations d ON d.node_id = m.function_node_id \
            JOIN reference_resolutions rr ON rr.fact_id = a.source_normal_evidence_id \
            JOIN summary_flows f ON f.source_flow_fact_id = m.flow_value_fact_id \
              AND f.parameter_node_id = m.parameter_node_id \
            JOIN summary_flow_steps read ON read.summary_id = f.summary_id \
              AND read.kind = {} AND read.evidence_id = rr.fact_id \
            WHERE d.name = 'framed_modeled_identity'",
                cpg_schema::codebook::SummaryFlowStepKind::ArgumentEvaluation.code()
            )
        )
        .await,
        1,
        "the pass-only try frame cites its safe lexical parameter read separately from raw flow"
    );
    assert_eq!(
        count(
            &ctx,
            &format!(
                "SELECT count(*) FROM summary_flows f \
             JOIN declarations d ON d.node_id = f.function_node_id \
             JOIN summary_flow_steps inner_pass ON inner_pass.summary_id = f.summary_id \
             JOIN summary_flow_steps outer_pass ON outer_pass.summary_id = f.summary_id \
             JOIN summary_flow_steps r ON r.summary_id = f.summary_id \
             JOIN syntax_nodes inner_source ON inner_source.fact_id = inner_pass.evidence_id \
             JOIN syntax_nodes outer_source ON outer_source.fact_id = outer_pass.evidence_id \
             WHERE d.name = 'nested_framed_modeled_identity' \
               AND f.path_depth = 1 \
               AND inner_pass.kind = {pass} AND outer_pass.kind = {pass} \
               AND r.kind = {exit} AND r.ordinal = outer_pass.ordinal + 1 \
               AND outer_pass.ordinal = inner_pass.ordinal + 1 \
               AND inner_source.start_byte < outer_source.start_byte",
                pass = cpg_schema::codebook::SummaryFlowStepKind::FinalizerPass.code(),
                exit = cpg_schema::codebook::SummaryFlowStepKind::ReturnExit.code(),
            )
        )
        .await,
        1,
        "a modeled return cites both nested finalizers before its completed exit"
    );
    assert_eq!(
        count(
            &ctx,
            &format!(
                "SELECT count(*) FROM summary_flows f \
                 JOIN summary_flow_steps r ON r.summary_id = f.summary_id \
                   AND r.kind = {} \
                 JOIN declarations d ON d.node_id = f.function_node_id \
                 WHERE d.name = 'identity' AND f.path_depth = 1 \
                   AND f.verdict <> {} AND f.boundary_reason IS NULL",
                cpg_schema::codebook::SummaryFlowStepKind::ModelRule.code(),
                Verdict::Unknown.code(),
            ),
        )
        .await,
        1,
        "one exact modeled identity call has a finite rule-cited return flow"
    );
    assert_eq!(
        count(
            &ctx,
            &format!(
                "SELECT count(*) FROM summary_flows f \
                 JOIN summary_flow_steps r ON r.summary_id = f.summary_id \
                   AND r.kind = {} \
                 JOIN summary_flow_steps v ON v.summary_id = f.summary_id \
                   AND v.kind = {} AND v.ordinal = r.ordinal + 1 \
                 JOIN declarations d ON d.node_id = f.function_node_id \
                 WHERE d.name = 'indirect_identity' AND f.path_depth = 2 \
                   AND f.verdict <> {} AND f.boundary_reason IS NULL",
                cpg_schema::codebook::SummaryFlowStepKind::DefinitionReaching.code(),
                cpg_schema::codebook::SummaryFlowStepKind::ReturnSource.code(),
                Verdict::Unknown.code(),
            ),
        )
        .await,
        1,
        "a unique compatible assignment predecessor yields an ordered two-hop summary"
    );
    assert_eq!(
        count(
            &ctx,
            "SELECT count(*) FROM summary_flows f \
             JOIN declarations d ON d.node_id = f.function_node_id \
             WHERE d.name = 'indirect_ambiguous' AND f.path_depth = 2",
        )
        .await,
        0,
        "competing reaching definitions cannot promote the modeled assignment path"
    );
    assert_eq!(
        count(
            &ctx,
            "SELECT count(*) FROM summary_flows f \
             JOIN declarations d ON d.node_id = f.function_node_id \
             WHERE d.name IN ('identity_raising_sibling', 'nested_identity', \
                              'computed_identity')",
        )
        .await,
        0,
        "raising arguments and non-exact expression paths cannot become modeled summaries"
    );
    assert_eq!(
        count(
            &ctx,
            "SELECT count(*) FROM summary_flows f \
             JOIN declarations d ON d.node_id = f.function_node_id \
             WHERE d.name IN ('identity_dynamic_type', 'identity_shadowed_type') \
               AND f.path_depth = 1",
        )
        .await,
        2,
        "a uniquely reaching parameter-name sibling evaluates normally under the model"
    );
    assert_eq!(
        count(
            &ctx,
            "SELECT count(*) FROM summary_boundaries b \
             JOIN declarations d ON d.node_id = b.function_node_id \
             WHERE d.name = 'plain_identity'",
        )
        .await,
        0,
        "the proved direct identity path has no spurious boundary"
    );
    assert_eq!(
        count(
            &ctx,
            "SELECT count(*) FROM summary_flows s \
            JOIN value_flow_contributions v ON v.snapshot_id = s.snapshot_id \
              AND v.origin_id = s.source_origin_id \
              AND v.flow_value_fact_id = s.source_flow_fact_id \
              AND v.parameter_node_id = s.parameter_node_id \
            JOIN declarations d ON d.node_id = s.function_node_id \
            WHERE d.name = 'plain_identity'"
        )
        .await,
        1,
        "the published direct summary cites its exact raw source contribution"
    );
    assert_eq!(
        count(
            &ctx,
            "SELECT count(*) FROM modeled_exact_value_transfers m \
            JOIN value_flow_contributions v ON v.snapshot_id = m.snapshot_id \
              AND v.origin_id = m.source_origin_id \
            WHERE v.flow_value_fact_id <> m.flow_value_fact_id \
               OR v.use_id <> m.use_id \
               OR v.parameter_node_id <> m.parameter_node_id \
               OR v.condition_id <> m.condition_id"
        )
        .await,
        0,
        "a modeled transfer retains the contribution it was derived from"
    );
    assert_eq!(
        count(
            &ctx,
            &format!(
                "SELECT count(*) FROM summary_boundaries b \
                 JOIN declarations d ON d.node_id = b.function_node_id \
             WHERE d.name = 'identity_dynamic_type' \
                   AND b.reason = {}",
                BoundaryReason::CallTransfer.code()
            ),
        )
        .await,
        1,
        "the parameter used only as a dynamic type remains a transfer boundary"
    );
    assert_eq!(
        count(
            &ctx,
            "SELECT count(*) FROM summary_boundaries b \
             JOIN declarations d ON d.node_id = b.function_node_id \
             WHERE d.name = 'indirect_identity'",
        )
        .await,
        0,
        "the fully proved assignment path no longer has an unresolved return boundary"
    );
    assert_eq!(
        count(
            &ctx,
            &format!(
                "SELECT count(*) FROM summary_boundaries b \
                 JOIN declarations d ON d.node_id = b.function_node_id \
                 WHERE d.name IN ('async_identity', 'generator_identity') \
                   AND b.reason = {}",
                BoundaryReason::UnsupportedControlFlow.code()
            ),
        )
        .await,
        2,
        "deferred returns name the withheld control scope"
    );
    assert_eq!(
        count(
            &ctx,
            "SELECT count(*) FROM modeled_exact_value_transfers t \
             LEFT ANTI JOIN analysis_conditions c ON c.condition_id = t.condition_id",
        )
        .await,
        0,
        "direct model candidates resolve their recomposed BDD conditions"
    );
    assert_eq!(
        count(
            &ctx,
            &format!(
                "SELECT count(*) FROM modeled_argument_evaluations e \
                 JOIN modeled_exact_value_transfers m \
                   ON m.flow_value_fact_id = e.candidate_flow_fact_id \
                  AND m.parameter_node_id = e.parameter_node_id \
                  AND m.pysa_fact_id = e.pysa_fact_id \
                  AND m.model_id = e.model_id AND m.rule_id = e.rule_id \
                 JOIN declarations d ON d.node_id = m.function_node_id \
                 WHERE d.name = 'identity_literal_type' \
                   AND e.status IN ({}, {}) AND e.evidence_id IS NOT NULL",
                ModeledArgumentEvaluationStatus::SourceOperand.code(),
                ModeledArgumentEvaluationStatus::LiteralNormal.code(),
            ),
        )
        .await,
        2,
        "the modeled operand and its direct literal sibling retain distinct normal-path evidence"
    );
    assert_eq!(
        count(
            &ctx,
            &format!(
                "SELECT count(*) FROM modeled_argument_evaluations e \
                 JOIN modeled_exact_value_transfers m \
                   ON m.flow_value_fact_id = e.candidate_flow_fact_id \
                  AND m.parameter_node_id = e.parameter_node_id \
                  AND m.pysa_fact_id = e.pysa_fact_id \
                  AND m.model_id = e.model_id AND m.rule_id = e.rule_id \
                 JOIN declarations d ON d.node_id = m.function_node_id \
                 WHERE d.name = 'identity_dynamic_type' \
                   AND e.status = {} AND e.evidence_id IN \
                     (SELECT fact_id FROM flow_reaching) AND e.reason IS NULL",
                ModeledArgumentEvaluationStatus::ParameterNameNormal.code(),
            ),
        )
        .await,
        1,
        "a uniquely reaching parameter sibling cites ty's definition evidence"
    );
    assert_eq!(
        count(
            &ctx,
            &format!(
                "SELECT count(*) FROM modeled_argument_evaluations e \
                 JOIN modeled_exact_value_transfers m \
                   ON m.flow_value_fact_id = e.candidate_flow_fact_id \
                  AND m.parameter_node_id = e.parameter_node_id \
                  AND m.pysa_fact_id = e.pysa_fact_id \
                  AND m.model_id = e.model_id AND m.rule_id = e.rule_id \
                 JOIN declarations d ON d.node_id = m.function_node_id \
                 WHERE d.name = 'identity' AND e.status = {} \
                   AND e.evidence_id IN (SELECT fact_id FROM reference_resolutions)",
                ModeledArgumentEvaluationStatus::BuiltinNameNormal.code(),
            ),
        )
        .await,
        1,
        "an unshadowed builtin name has a lexical normal-evaluation witness"
    );
    assert_eq!(
        count(
            &ctx,
            &format!(
                "SELECT count(*) FROM modeled_argument_evaluations e \
                 JOIN modeled_exact_value_transfers m \
                   ON m.flow_value_fact_id = e.candidate_flow_fact_id \
                  AND m.parameter_node_id = e.parameter_node_id \
                  AND m.pysa_fact_id = e.pysa_fact_id \
                  AND m.model_id = e.model_id AND m.rule_id = e.rule_id \
                 JOIN declarations d ON d.node_id = m.function_node_id \
                 WHERE d.name = 'identity_shadowed_type' AND e.status = {} \
                   AND e.evidence_id IN (SELECT fact_id FROM flow_reaching)",
                ModeledArgumentEvaluationStatus::ParameterNameNormal.code(),
            ),
        )
        .await,
        1,
        "a shadowed builtin spelling uses its parameter binding, never builtin evidence"
    );
    assert_eq!(
        count(&ctx, "SELECT count(*) FROM modeled_transfer_sites").await,
        32,
        "the typing, JSON, gzip and atexit transfers apply to resolved source calls"
    );
    assert_eq!(
        count(
            &ctx,
            &format!(
                "SELECT count(*) FROM modeled_exact_value_transfers t \
             JOIN declarations d ON d.node_id = t.function_node_id \
             WHERE d.name IN ('identity', 'identity_keyword', 'asserted_type') \
               AND t.parameter_node_id IS NOT NULL AND t.flow_value_call_fact_id IS NOT NULL \
               AND t.sink = {}",
                FlowSink::Return.code()
            ),
        )
        .await,
        3,
        "only exact direct identity call paths compose the source parameter and model input"
    );
    assert_eq!(
        count(
            &ctx,
            &format!(
                "SELECT count(*) FROM modeled_exact_value_transfers t \
             JOIN declarations d ON d.node_id = t.function_node_id \
             WHERE d.name IN ('indirect_identity', 'nested_identity', 'computed_identity') \
               AND t.sink = {}",
                FlowSink::Return.code()
            ),
        )
        .await,
        0,
        "inherited, nested and computed outer expressions cannot use the direct call-result rule"
    );
    assert_eq!(
        count(
            &ctx,
            &format!(
                "SELECT count(*) FROM modeled_exact_value_transfers t \
             JOIN declarations d ON d.node_id = t.function_node_id \
             WHERE d.name = 'indirect_identity' AND t.sink = {}",
                FlowSink::Definition.code()
            ),
        )
        .await,
        1,
        "the assignment value has one exact model-call result step"
    );
    assert_eq!(
        count(
            &ctx,
            "SELECT count(*) FROM value_flow_predecessor_candidates p \
             JOIN declarations d ON d.node_id = p.function_node_id \
             WHERE d.name = 'indirect_identity' \
               AND p.predecessor_local_through_call AND NOT p.predecessor_upstream_through_call \
               AND NOT p.loop_carried",
        )
        .await,
        1,
        "the reaching assignment identifies the earlier raw model-call fact without claiming condition compatibility"
    );
    assert_eq!(
        count(
            &ctx,
            "SELECT count(*) FROM value_flow_predecessor_compatibility p \
             JOIN value_flow_predecessor_candidates c \
               ON c.successor_fact_id = p.successor_fact_id \
              AND c.predecessor_fact_id = p.predecessor_fact_id \
              AND c.reaching_fact_id = p.reaching_fact_id \
             JOIN declarations d ON d.node_id = c.function_node_id \
             WHERE d.name = 'indirect_identity' \
               AND p.compatible_under_atoms AND p.boundary_reason IS NULL",
        )
        .await,
        1,
        "the source candidate has a bounded may-compatible path, not a completed transfer"
    );
    assert_eq!(
        count(
            &ctx,
            "SELECT count(*) FROM modeled_assignment_return_paths p \
             JOIN declarations d ON d.node_id = p.function_node_id \
             WHERE d.name = 'indirect_identity' AND p.compatible_under_atoms \
               AND p.boundary_reason IS NULL",
        )
        .await,
        1,
        "the exact assignment model step reaches an identity return through one cited definition"
    );
    assert_eq!(
        count(
            &ctx,
            "SELECT count(*) FROM modeled_assignment_return_paths p \
             JOIN declarations d ON d.node_id = p.function_node_id \
             WHERE d.name IN ('nested_identity', 'computed_identity')",
        )
        .await,
        0,
        "a nested call or computed outer return has no exact two-step model path"
    );
    assert_eq!(
        count(
            &ctx,
            "SELECT count(*) FROM summary_flows f \
            JOIN declarations d ON d.node_id = f.function_node_id \
            WHERE d.name = 'nested_total_identity' AND f.path_depth = 2 \
              AND f.boundary_reason IS NULL"
        )
        .await,
        1,
        "two exact total identity calls compose into one cited finite path"
    );
    assert_eq!(
        count(
            &ctx,
            "SELECT count(*) FROM summary_flows f \
            JOIN declarations d ON d.node_id = f.function_node_id \
            WHERE d.name = 'nested_three_total_identity' AND f.path_depth = 3 \
              AND f.boundary_reason IS NULL"
        )
        .await,
        1,
        "the same finite producer composes a third exact total identity call"
    );
    assert_eq!(
        count(
            &ctx,
            &format!(
                "WITH first_eval AS ( \
              SELECT summary_id, min(ordinal) AS ordinal FROM summary_flow_steps \
              WHERE kind = {evaluation} GROUP BY summary_id) \
            SELECT count(*) FROM summary_flows f \
            JOIN declarations d ON d.node_id = f.function_node_id \
            JOIN first_eval ON first_eval.summary_id = f.summary_id \
            JOIN summary_flow_steps outer_literal ON outer_literal.summary_id = f.summary_id \
              AND outer_literal.kind = {evaluation} AND outer_literal.ordinal = first_eval.ordinal \
            JOIN summary_flow_steps inner_site ON inner_site.summary_id = f.summary_id \
              AND inner_site.kind = {site} \
            JOIN summary_flow_steps outer_source ON outer_source.summary_id = f.summary_id \
              AND outer_source.kind = {evaluation} \
              AND outer_source.evidence_id = inner_site.evidence_id \
            WHERE d.name = 'nested_total_identity' AND f.path_depth = 2 \
              AND outer_literal.ordinal < inner_site.ordinal \
              AND inner_site.ordinal < outer_source.ordinal",
                evaluation = SummaryFlowStepKind::ArgumentEvaluation.code(),
                site = SummaryFlowStepKind::CallSite.code()
            )
        )
        .await,
        1,
        "the outer literal precedes the inner call, whose result precedes the outer call"
    );
    for name in [
        "nested_identity",
        "nested_raising_identity",
        "nested_deleted_identity",
    ] {
        assert_eq!(
            count(
                &ctx,
                &format!(
                    "SELECT count(*) FROM summary_flows f \
                JOIN declarations d ON d.node_id = f.function_node_id \
                WHERE d.name = '{name}' AND f.path_depth = 2"
                )
            )
            .await,
            0,
            "{name} has no exact normally completed inner identity"
        );
        if name == "nested_deleted_identity" {
            continue;
        }
        assert!(
            count(
                &ctx,
                &format!(
                    "SELECT count(*) FROM summary_boundaries b \
                JOIN declarations d ON d.node_id = b.function_node_id \
                WHERE d.name = '{name}' AND b.local_through_call"
                )
            )
            .await
                > 0,
            "{name} keeps an explicit open source origin"
        );
    }
    assert_eq!(
        count(
            &ctx,
            "SELECT count(*) FROM value_flow_contributions v \
            JOIN declarations d ON d.node_id = v.sink_function_node_id \
            WHERE d.name = 'nested_deleted_identity' AND v.parameter_node_id IS NOT NULL"
        )
        .await,
        0,
        "a deleted formal has no parameter-origin contribution to discharge or bound"
    );
    assert_eq!(
        count(
            &ctx,
            "SELECT count(*) FROM summary_flows f \
            JOIN declarations d ON d.node_id = f.function_node_id \
            WHERE d.name = 'framed_maybe_deleted_identity'"
        )
        .await,
        0,
        "the lexical read fallback withholds a formal that can be deleted in the try body"
    );
    assert!(
        count(
            &ctx,
            "SELECT count(*) FROM modeled_exact_value_transfers m \
            JOIN declarations d ON d.node_id = m.function_node_id \
            WHERE d.name = 'framed_maybe_deleted_identity'"
        )
        .await
            > 0,
        "the deletion control must have a real raw model candidate to challenge"
    );
    assert_eq!(
        count(
            &ctx,
            &format!(
                "SELECT count(*) FROM modeled_argument_evaluations a \
            JOIN modeled_exact_value_transfers m \
              ON m.flow_value_fact_id = a.candidate_flow_fact_id \
             AND m.parameter_node_id = a.parameter_node_id \
             AND m.pysa_fact_id = a.pysa_fact_id \
             AND m.model_id = a.model_id AND m.rule_id = a.rule_id \
            JOIN declarations d ON d.node_id = m.function_node_id \
            WHERE d.name = 'framed_maybe_deleted_identity' AND a.status = {}",
                ModeledArgumentEvaluationStatus::SourceOperand.code()
            )
        )
        .await,
        0,
        "a possible deletion prevents the raw source from acquiring a normal-read witness"
    );
    assert_eq!(
        count(
            &ctx,
            &format!(
                "SELECT count(*) FROM modeled_transfer_sites s \
                 JOIN declarations d ON d.node_id = s.function_node_id \
                 WHERE d.name = 'identity_keyword' AND s.transfer = {} \
                   AND s.input_status = {} AND s.input_expression_node_id IS NOT NULL \
                   AND s.input_reason IS NULL AND s.output_status = {} \
                   AND s.output_expression_node_id = s.call_site_node_id \
                   AND s.output_expression_fact_id = s.call_fact_id \
                   AND s.candidate_set_complete_under_model",
                ModelTransferKind::Identity.code(),
                ModelTransferEndpointStatus::BoundArgument.code(),
                ModelTransferEndpointStatus::CallResult.code(),
            )
        )
        .await,
        1,
        "the keyword cast transfer cites its exact input and call-result expressions"
    );
    assert_eq!(
        count(
            &ctx,
            &format!(
                "SELECT count(*) FROM modeled_transfer_sites s \
                 JOIN declarations d ON d.node_id = s.function_node_id \
                 WHERE d.name = 'asserted_type' AND s.transfer = {} \
                   AND s.input_status = {} AND s.input_expression_node_id IS NOT NULL \
                   AND s.output_status = {} AND s.output_expression_node_id = s.call_site_node_id \
                   AND s.candidate_set_complete_under_model",
                ModelTransferKind::Identity.code(),
                ModelTransferEndpointStatus::BoundArgument.code(),
                ModelTransferEndpointStatus::CallResult.code(),
            )
        )
        .await,
        1,
        "the assert_type identity cites the exact val argument and returned call expression"
    );
    assert_eq!(
        count(
            &ctx,
            &format!(
                "SELECT count(*) FROM modeled_transfer_sites s \
                 JOIN declarations d ON d.node_id = s.function_node_id \
                 WHERE d.name = 'json_text' AND s.transfer = {} \
                   AND s.input_status = {} AND s.output_status = {} \
                   AND s.model_modality = {} AND s.candidate_set_complete_under_model",
                ModelTransferKind::Transform.code(),
                ModelTransferEndpointStatus::BoundArgument.code(),
                ModelTransferEndpointStatus::CallResult.code(),
                Modality::Potential.code(),
            )
        )
        .await,
        1,
        "JSON output carries only a potential transformed value source"
    );
    assert_eq!(
        count(
            &ctx,
            &format!(
                "SELECT count(*) FROM modeled_transfer_sites s \
                 JOIN declarations d ON d.node_id = s.function_node_id \
                 WHERE d.name IN ('json_decode', 'compress_data', 'decompress_data') \
                   AND s.transfer = {} AND s.model_modality = {} \
                   AND s.input_status = {} AND s.output_status = {}",
                ModelTransferKind::Transform.code(),
                Modality::Potential.code(),
                ModelTransferEndpointStatus::BoundArgument.code(),
                ModelTransferEndpointStatus::CallResult.code(),
            ),
        )
        .await,
        3,
        "the three pinned decoding/compression calls bind their real data formals"
    );
    assert_eq!(
        count(
            &ctx,
            "SELECT count(*) FROM modeled_transfer_sites s \
             JOIN declarations d ON d.node_id = s.function_node_id \
             WHERE d.name = 'shadowed_compress'",
        )
        .await,
        0,
        "a parameter named gzip is not the pinned stdlib module"
    );
    assert_eq!(
        count(
            &ctx,
            &format!(
                "SELECT count(*) FROM modeled_transfer_sites s \
                 JOIN declarations d ON d.node_id = s.function_node_id \
                 WHERE d.name IN ('on_shutdown_keyword', 'on_shutdown_unpacked') \
                   AND s.input_status = {} AND s.input_reason IS NOT NULL \
                   AND s.input_expression_node_id IS NULL \
                   AND s.output_status = {}",
                ModelTransferEndpointStatus::Unknown.code(),
                ModelTransferEndpointStatus::CallResult.code(),
            )
        )
        .await,
        2,
        "unsupported formals leave the input endpoint unknown, even with a call result"
    );
    assert_eq!(count(&ctx, "SELECT count(*) FROM model_effects").await, 8);
    assert_eq!(
        count(&ctx, "SELECT count(*) FROM modeled_effect_sites").await,
        9,
        "print, JSON, gzip and logging effects apply to their resolved source calls"
    );
    assert_eq!(
        count(
            &ctx,
            &format!(
                "SELECT count(*) FROM modeled_effect_sites s \
                 JOIN declarations d ON d.node_id = s.function_node_id \
                 WHERE d.name = 'display' AND s.effect = {} \
                   AND s.subject_status = {} AND s.subject_path_id IS NULL \
                   AND s.subject_expression_node_id IS NULL \
                   AND s.subject_reason IS NULL AND s.model_modality = {} \
                   AND s.candidate_set_complete_under_model",
                ModelEffectKind::IoWrite.code(),
                ModelEffectSubjectStatus::Unqualified.code(),
                Modality::Potential.code(),
            )
        )
        .await,
        1,
        "an I/O effect without a stream subject must remain explicitly unqualified"
    );
    assert_eq!(
        count(
            &ctx,
            &format!(
                "SELECT count(*) FROM modeled_effect_sites s \
                 JOIN declarations d ON d.node_id = s.function_node_id \
                 WHERE d.name = 'json_write' AND s.effect = {} \
                   AND s.subject_status = {} AND s.subject_expression_node_id IS NOT NULL \
                   AND s.model_modality = {} AND s.candidate_set_complete_under_model",
                ModelEffectKind::IoWrite.code(),
                ModelEffectSubjectStatus::BoundArgument.code(),
                Modality::Potential.code(),
            )
        )
        .await,
        1,
        "JSON stream write cites the exact fp argument rather than an unqualified I/O effect"
    );
    assert_eq!(
        count(
            &ctx,
            &format!(
                "SELECT count(*) FROM modeled_effect_sites s \
                 JOIN declarations d ON d.node_id = s.function_node_id \
                 WHERE d.name = 'compress_data' AND s.effect = {} \
                   AND s.subject_status = {} AND s.subject_expression_node_id IS NOT NULL \
                   AND s.model_modality = {}",
                ModelEffectKind::Compress.code(),
                ModelEffectSubjectStatus::BoundArgument.code(),
                Modality::Potential.code(),
            ),
        )
        .await,
        1,
        "the compression effect cites the actual data argument"
    );
    assert_eq!(
        count(
            &ctx,
            &format!(
                "SELECT count(*) FROM modeled_effect_sites s \
                 JOIN declarations d ON d.node_id = s.function_node_id \
                 JOIN pysa_calls p ON p.fact_id = s.pysa_fact_id \
                 WHERE d.name = 'warn' AND s.effect = {} \
                   AND s.subject_status = {} AND s.subject_expression_node_id IS NOT NULL \
                   AND s.model_modality = {} AND p.implicit_receiver = {}",
                ModelEffectKind::Log.code(),
                ModelEffectSubjectStatus::BoundArgument.code(),
                Modality::Potential.code(),
                ImplicitReceiver::TrueWithObjectReceiver.code(),
            ),
        )
        .await,
        1,
        "a resolved Logger.warning cites the exact message argument"
    );
    // The other pinned Logger levels bind the same message formal, after `level` for `log`.
    for name in ["inform", "log_at"] {
        assert_eq!(
            count(
                &ctx,
                &format!(
                    "SELECT count(*) FROM modeled_effect_sites s \
                     JOIN declarations d ON d.node_id = s.function_node_id \
                     WHERE d.name = '{name}' AND s.effect = {} AND s.subject_status = {} \
                       AND s.subject_expression_node_id IS NOT NULL AND s.model_modality = {}",
                    ModelEffectKind::Log.code(),
                    ModelEffectSubjectStatus::BoundArgument.code(),
                    Modality::Potential.code(),
                ),
            )
            .await,
            1,
            "{name} cites its message argument"
        );
    }
    assert_eq!(
        count(
            &ctx,
            "SELECT count(*) FROM modeled_effect_sites s \
             JOIN declarations d ON d.node_id = s.function_node_id \
             WHERE d.name = 'shadowed_warn'",
        )
        .await,
        0,
        "an untyped logger argument cannot inherit the pinned Logger model"
    );
    assert_eq!(
        count(
            &ctx,
            &format!(
                "SELECT count(*) FROM modeled_effect_sites s \
                 JOIN declarations d ON d.node_id = s.function_node_id \
                 WHERE d.name = 'warn_unpacked' AND s.effect = {} \
                   AND s.subject_status = {} AND s.subject_reason = {} \
                   AND s.subject_expression_node_id IS NULL",
                ModelEffectKind::Log.code(),
                ModelEffectSubjectStatus::Unknown.code(),
                BoundaryReason::UnsupportedUnpacking.code(),
            ),
        )
        .await,
        1,
        "a bound Logger call still withholds an unpacked message argument"
    );
    assert_eq!(count(&ctx, "SELECT count(*) FROM model_callbacks").await, 1);
    assert_eq!(count(&ctx, "SELECT count(*) FROM model_resources").await, 1);
    assert_eq!(
        count(&ctx, "SELECT count(*) FROM modeled_resource_sites").await,
        1,
        "only the resolved builtins.open call carries the authored resource action"
    );
    assert_eq!(
        count(
            &ctx,
            &format!(
                "SELECT count(*) FROM modeled_resource_sites s \
                 JOIN declarations d ON d.node_id = s.function_node_id \
                 WHERE d.name = 'acquire' AND s.resource_role = {} \
                   AND s.resource_path_kind = {} AND s.source_status = {} \
                   AND s.source_expression_node_id = s.call_site_node_id \
                   AND s.source_expression_fact_id = s.call_fact_id \
                   AND s.source_reason IS NULL AND s.action = {} AND s.exit = {}",
                ModelPathRole::Output.code(),
                ModelPathKind::ReturnValue.code(),
                ModelResourceSourceStatus::CallResult.code(),
                ModelResourceAction::Acquire.code(),
                ModelExit::Normal.code(),
            )
        )
        .await,
        1,
        "a return resource identifies the call expression, not a runtime object"
    );
    assert_eq!(
        count(&ctx, "SELECT count(*) FROM model_formal_paths").await,
        15,
        "typing, JSON, gzip, logging and atexit paths use typed model ASTs"
    );
    assert_eq!(
        count(
            &ctx,
            "SELECT count(*) FROM arguments a JOIN call_syntax c ON c.node_id = a.call_node_id \
             JOIN declarations d ON d.node_id = c.owner_node_id \
             WHERE d.name = 'identity_keyword' AND a.keyword = 'val' \
               AND a.value_start_byte > a.start_byte \
               AND a.value_end_byte = a.end_byte"
        )
        .await,
        1,
        "a keyword argument retains its value span apart from its authored role span"
    );
    assert_eq!(
        count(
            &ctx,
            "SELECT count(*) FROM arguments a JOIN call_syntax c ON c.node_id = a.call_node_id \
             JOIN declarations d ON d.node_id = c.owner_node_id \
             WHERE d.name = 'on_shutdown' AND a.ordinal = 0 \
               AND a.value_start_byte = a.start_byte \
               AND a.value_end_byte = a.end_byte"
        )
        .await,
        1,
        "a direct positional argument retains the same role and value span"
    );
    assert!(
        count(&ctx, "SELECT count(*) FROM model_argument_bindings").await >= 3,
        "each applied modeled formal gets a bound or explicit unknown row"
    );
    assert_eq!(
        count(
            &ctx,
            &format!(
                "SELECT count(*) FROM model_argument_bindings b \
                 JOIN model_callbacks c ON c.rule_id = b.rule_id \
                 JOIN model_applications a ON a.call_site_node_id = b.call_site_node_id \
                   AND a.pysa_fact_id = b.pysa_fact_id AND a.model_id = b.model_id \
                 JOIN declarations d ON d.node_id = a.function_node_id \
                 WHERE d.name = 'on_shutdown' \
                   AND b.status = {} AND b.argument_node_id IS NOT NULL \
                   AND b.reason IS NULL",
                ModelArgumentStatus::Bound.code()
            )
        )
        .await,
        1,
        "the positional callback formal binds the exact source argument"
    );
    assert_eq!(
        count(
            &ctx,
            &format!(
                "SELECT count(*) FROM model_argument_bindings b \
                 JOIN model_applications a ON a.call_site_node_id = b.call_site_node_id \
                   AND a.pysa_fact_id = b.pysa_fact_id AND a.model_id = b.model_id \
                 JOIN declarations d ON d.node_id = a.function_node_id \
                 WHERE d.name = 'identity_keyword' AND b.formal_name = 'val' \
                   AND b.status = {} AND b.argument_node_id IS NOT NULL",
                ModelArgumentStatus::Bound.code()
            )
        )
        .await,
        1,
        "a keyword-capable pinned formal binds its named source argument"
    );
    assert_eq!(
        count(
            &ctx,
            &format!(
                "SELECT count(*) FROM model_argument_bindings b \
                 JOIN model_applications a ON a.call_site_node_id = b.call_site_node_id \
                   AND a.pysa_fact_id = b.pysa_fact_id AND a.model_id = b.model_id \
                 JOIN declarations d ON d.node_id = a.function_node_id \
                 WHERE d.name = 'on_shutdown_keyword' AND b.status = {} \
                   AND b.reason IS NOT NULL AND b.argument_node_id IS NULL",
                ModelArgumentStatus::Unknown.code()
            )
        )
        .await,
        2,
        "a positional-only pinned formal is not bound by a named argument"
    );
    assert_eq!(
        count(
            &ctx,
            &format!(
                "SELECT count(*) FROM model_argument_bindings b \
                 JOIN model_applications a ON a.call_site_node_id = b.call_site_node_id \
                   AND a.pysa_fact_id = b.pysa_fact_id AND a.model_id = b.model_id \
                 JOIN declarations d ON d.node_id = a.function_node_id \
                 WHERE d.name = 'on_shutdown_unpacked' AND b.status = {} \
                   AND b.reason IS NOT NULL AND b.argument_node_id IS NULL",
                ModelArgumentStatus::Unknown.code()
            )
        )
        .await,
        2,
        "starred arguments withhold both authored func paths"
    );
    assert_eq!(
        count(&ctx, "SELECT count(*) FROM modeled_callback_sites").await,
        3,
        "only the three atexit call sites carry the authored registration action"
    );
    assert_eq!(
        count(
            &ctx,
            &format!(
                "SELECT count(*) FROM modeled_callback_sites s \
                 JOIN declarations d ON d.node_id = s.function_node_id \
                 WHERE d.name = 'on_shutdown' AND s.action = {} AND s.exit = {} \
                   AND s.binding_status = {} AND s.argument_node_id IS NOT NULL \
                   AND s.candidate_set_complete_under_model",
                ModelCallbackAction::Registered.code(),
                ModelExit::Normal.code(),
                ModelArgumentStatus::Bound.code()
            )
        )
        .await,
        1,
        "registration remains a candidate-local modeled action with a cited argument"
    );
    assert_eq!(
        count(
            &ctx,
            &format!(
                "SELECT count(*) FROM modeled_callback_sites s \
                 JOIN declarations d ON d.node_id = s.function_node_id \
                 WHERE d.name IN ('on_shutdown_keyword', 'on_shutdown_unpacked') \
                   AND s.binding_status = {} AND s.binding_reason IS NOT NULL \
                   AND s.argument_node_id IS NULL",
                ModelArgumentStatus::Unknown.code()
            )
        )
        .await,
        2,
        "unsupported callback argument shapes retain explicit unknown sites"
    );
    assert_eq!(
        count(&ctx, "SELECT count(*) FROM model_applications").await,
        39,
        "each pinned model applies only at its resolved source call"
    );
    assert!(
        count(
            &ctx,
            "SELECT count(*) FROM model_applications a \
             JOIN model_targets t ON t.model_id = a.model_id \
               AND t.target_node_id = a.target_node_id \
             WHERE a.target_body_return_parameter IS NOT NULL AND a.target_count = 1 \
               AND a.candidate_set_complete_under_model \
               AND NOT a.has_unresolved_remainder \
               AND t.target_key IN ('stdlib:3.14.7:typing.cast', \
                                    'stdlib:3.14.7:typing.assert_type')"
        )
        .await
            > 0,
        "the exact typing calls retain both the pinned completion assertion and target closure"
    );
    assert_eq!(
        count(
            &ctx,
            "SELECT count(*) FROM model_applications a JOIN declarations d \
             ON d.node_id = a.function_node_id WHERE d.name = 'shadowed_open'"
        )
        .await,
        0,
        "a shadowed builtin name cannot acquire the builtin model"
    );
    assert_eq!(
        count(
            &ctx,
            "SELECT count(*) FROM model_applications a JOIN model_callbacks c \
             ON c.model_id = a.model_id AND c.target_node_id = a.target_node_id \
             JOIN declarations d ON d.node_id = a.function_node_id \
             WHERE d.name = 'on_shutdown' AND a.candidate_set_complete_under_model \
             AND NOT a.has_unresolved_remainder"
        )
        .await,
        1,
        "a modeled callback rule stays linked to its complete source call target"
    );
    assert_eq!(
        count(&ctx, "SELECT count(*) FROM model_exceptions").await,
        1
    );
    assert_eq!(
        count(
            &ctx,
            "SELECT count(*) FROM model_exceptions e \
             JOIN context_definitions d ON d.symbol_node_id = e.class_node_id \
               AND d.fact_id = e.class_fact_id \
             WHERE e.class = 'builtins.OSError' AND d.module_name = 'builtins' \
               AND d.qualified_name = 'OSError'"
        )
        .await,
        1,
        "the model exception class is pinned to a context definition"
    );
    assert_eq!(
        count(
            &ctx,
            "SELECT count(*) FROM modeled_exception_sites s \
             JOIN declarations d ON d.node_id = s.function_node_id \
             WHERE d.name = 'acquire' AND s.class = 'builtins.OSError' \
               AND s.call_site_node_id IS NOT NULL AND s.candidate_set_complete_under_model"
        )
        .await,
        1,
        "the modeled potential OSError is attached to the exact open call candidate"
    );
    assert!(
        count(
            &ctx,
            "SELECT count(*) FROM context_parameters p JOIN model_targets t \
             ON p.symbol_node_id = t.target_node_id WHERE p.name = 'val'"
        )
        .await
            >= 1
    );
    assert!(cpg_core::validate::validate(&ctx).await.unwrap().is_empty());

    let original_arguments = sql::query(&ctx, "SELECT * FROM arguments")
        .await
        .unwrap()
        .into_view();
    let missing_arguments = sql::query(&ctx, "SELECT * FROM arguments WHERE false")
        .await
        .unwrap()
        .into_view();
    ctx.deregister_table("arguments").unwrap();
    ctx.register_table("arguments", missing_arguments).unwrap();
    let violations = cpg_core::validate::validate(&ctx).await.unwrap();
    assert!(
        violations
            .iter()
            .any(|v| v.rule == "semantic:call-argument-coverage"),
        "{violations:?}"
    );
    ctx.deregister_table("arguments").unwrap();
    ctx.register_table("arguments", original_arguments).unwrap();

    let original_call_steps = sql::query(&ctx, "SELECT * FROM flow_value_calls")
        .await
        .unwrap()
        .into_view();
    let skipped_call_step = sql::query(
        &ctx,
        "SELECT * EXCLUDE (step), step + 1 AS step FROM flow_value_calls",
    )
    .await
    .unwrap()
    .into_view();
    ctx.deregister_table("flow_value_calls").unwrap();
    ctx.register_table("flow_value_calls", skipped_call_step)
        .unwrap();
    let violations = cpg_core::validate::validate(&ctx).await.unwrap();
    assert!(
        violations
            .iter()
            .any(|v| v.rule == "semantic:flow-value-call-path"),
        "{violations:?}"
    );
    ctx.deregister_table("flow_value_calls").unwrap();
    ctx.register_table("flow_value_calls", original_call_steps)
        .unwrap();

    let original_applications = sql::query(&ctx, "SELECT * FROM model_applications")
        .await
        .unwrap()
        .into_view();
    let doctored_applications = sql::query(
        &ctx,
        "SELECT * EXCLUDE (candidate_set_complete_under_model), \
         NOT candidate_set_complete_under_model AS candidate_set_complete_under_model \
         FROM model_applications",
    )
    .await
    .unwrap()
    .into_view();
    ctx.deregister_table("model_applications").unwrap();
    ctx.register_table("model_applications", doctored_applications)
        .unwrap();
    let violations = cpg_core::validate::validate(&ctx).await.unwrap();
    let (boundary_rule,) = ("semantic:model-application-boundary",);
    assert!(
        violations
            .iter()
            .any(|v| v.rule == "model-application-source-equality"),
        "{violations:?}"
    );
    assert!(
        violations.iter().any(|v| v.rule == boundary_rule),
        "{violations:?}"
    );
    ctx.deregister_table("model_applications").unwrap();
    ctx.register_table("model_applications", original_applications)
        .unwrap();

    let original_applications = sql::query(&ctx, "SELECT * FROM model_applications")
        .await
        .unwrap()
        .into_view();
    let forged_completion = sql::query(
        &ctx,
        "SELECT * EXCLUDE (target_body_return_parameter), \
         CAST(NULL AS VARCHAR) AS target_body_return_parameter FROM model_applications",
    )
    .await
    .unwrap()
    .into_view();
    ctx.deregister_table("model_applications").unwrap();
    ctx.register_table("model_applications", forged_completion)
        .unwrap();
    let violations = cpg_core::validate::validate(&ctx).await.unwrap();
    assert!(
        violations
            .iter()
            .any(|v| v.rule == "model-application-source-equality"),
        "{violations:?}"
    );
    ctx.deregister_table("model_applications").unwrap();
    ctx.register_table("model_applications", original_applications)
        .unwrap();

    let original_formals = sql::query(&ctx, "SELECT * FROM model_formal_paths")
        .await
        .unwrap()
        .into_view();
    let doctored_formals = sql::query(
        &ctx,
        "SELECT * EXCLUDE (formal_name), '' AS formal_name FROM model_formal_paths",
    )
    .await
    .unwrap()
    .into_view();
    ctx.deregister_table("model_formal_paths").unwrap();
    ctx.register_table("model_formal_paths", doctored_formals)
        .unwrap();
    let violations = cpg_core::validate::validate(&ctx).await.unwrap();
    let (formal_rule,) = ("semantic:model-formal-path-source",);
    assert!(
        violations
            .iter()
            .any(|v| v.rule == "model-catalog-formal-path-equality"),
        "{violations:?}"
    );
    assert!(
        violations.iter().any(|v| v.rule == formal_rule),
        "{violations:?}"
    );
    ctx.deregister_table("model_formal_paths").unwrap();
    ctx.register_table("model_formal_paths", original_formals)
        .unwrap();

    let original_bindings = sql::query(&ctx, "SELECT * FROM model_argument_bindings")
        .await
        .unwrap()
        .into_view();
    let doctored_bindings = sql::query(
        &ctx,
        &format!(
            "SELECT * EXCLUDE (status), CAST({} AS SMALLINT) AS status \
             FROM model_argument_bindings",
            ModelArgumentStatus::Bound.code()
        ),
    )
    .await
    .unwrap()
    .into_view();
    ctx.deregister_table("model_argument_bindings").unwrap();
    ctx.register_table("model_argument_bindings", doctored_bindings)
        .unwrap();
    let violations = cpg_core::validate::validate(&ctx).await.unwrap();
    let (argument_rule,) = ("semantic:model-argument-status-shape",);
    assert!(
        violations.iter().any(|v| v.rule == argument_rule),
        "{violations:?}"
    );
    assert!(
        violations
            .iter()
            .any(|v| v.rule == "model-argument-source-equality"),
        "{violations:?}"
    );
    ctx.deregister_table("model_argument_bindings").unwrap();
    ctx.register_table("model_argument_bindings", original_bindings)
        .unwrap();

    let original_callback_sites = sql::query(&ctx, "SELECT * FROM modeled_callback_sites")
        .await
        .unwrap()
        .into_view();
    let doctored_callback_sites = sql::query(
        &ctx,
        &format!(
            "SELECT * EXCLUDE (binding_status), CAST({} AS SMALLINT) AS binding_status \
             FROM modeled_callback_sites",
            ModelArgumentStatus::Bound.code()
        ),
    )
    .await
    .unwrap()
    .into_view();
    ctx.deregister_table("modeled_callback_sites").unwrap();
    ctx.register_table("modeled_callback_sites", doctored_callback_sites)
        .unwrap();
    let violations = cpg_core::validate::validate(&ctx).await.unwrap();
    let (callback_rule,) = ("semantic:modeled-callback-site-shape",);
    assert!(
        violations.iter().any(|v| v.rule == callback_rule),
        "{violations:?}"
    );
    assert!(
        violations
            .iter()
            .any(|v| v.rule == "modeled-callback-site-source-equality"),
        "{violations:?}"
    );
    ctx.deregister_table("modeled_callback_sites").unwrap();
    ctx.register_table("modeled_callback_sites", original_callback_sites)
        .unwrap();

    let original_resource_sites = sql::query(&ctx, "SELECT * FROM modeled_resource_sites")
        .await
        .unwrap()
        .into_view();
    let doctored_resource_sites = sql::query(
        &ctx,
        &format!(
            "SELECT * EXCLUDE (source_status), CAST({} AS SMALLINT) AS source_status \
             FROM modeled_resource_sites",
            ModelResourceSourceStatus::Unknown.code()
        ),
    )
    .await
    .unwrap()
    .into_view();
    ctx.deregister_table("modeled_resource_sites").unwrap();
    ctx.register_table("modeled_resource_sites", doctored_resource_sites)
        .unwrap();
    let violations = cpg_core::validate::validate(&ctx).await.unwrap();
    assert!(
        violations
            .iter()
            .any(|v| v.rule == "semantic:modeled-resource-site-shape"),
        "{violations:?}"
    );
    assert!(
        violations
            .iter()
            .any(|v| v.rule == "modeled-resource-site-source-equality"),
        "{violations:?}"
    );
    ctx.deregister_table("modeled_resource_sites").unwrap();
    ctx.register_table("modeled_resource_sites", original_resource_sites)
        .unwrap();

    let original_transfer_sites = sql::query(&ctx, "SELECT * FROM modeled_transfer_sites")
        .await
        .unwrap()
        .into_view();
    let doctored_transfer_sites = sql::query(
        &ctx,
        &format!(
            "SELECT * EXCLUDE (input_status), CAST({} AS SMALLINT) AS input_status \
             FROM modeled_transfer_sites",
            ModelTransferEndpointStatus::CallResult.code()
        ),
    )
    .await
    .unwrap()
    .into_view();
    ctx.deregister_table("modeled_transfer_sites").unwrap();
    ctx.register_table("modeled_transfer_sites", doctored_transfer_sites)
        .unwrap();
    let violations = cpg_core::validate::validate(&ctx).await.unwrap();
    assert!(
        violations
            .iter()
            .any(|v| v.rule == "semantic:modeled-transfer-site-shape"),
        "{violations:?}"
    );
    assert!(
        violations
            .iter()
            .any(|v| v.rule == "modeled-transfer-site-source-equality"),
        "{violations:?}"
    );
    ctx.deregister_table("modeled_transfer_sites").unwrap();
    ctx.register_table("modeled_transfer_sites", original_transfer_sites)
        .unwrap();

    let original_exact_transfers = sql::query(&ctx, "SELECT * FROM modeled_exact_value_transfers")
        .await
        .unwrap()
        .into_view();
    let missing_exact_transfers = sql::query(
        &ctx,
        "SELECT * FROM modeled_exact_value_transfers WHERE false",
    )
    .await
    .unwrap()
    .into_view();
    ctx.deregister_table("modeled_exact_value_transfers")
        .unwrap();
    ctx.register_table("modeled_exact_value_transfers", missing_exact_transfers)
        .unwrap();
    let violations = cpg_core::validate::validate(&ctx).await.unwrap();
    assert!(
        violations
            .iter()
            .any(|v| v.rule == "modeled-exact-value-transfer-source-equality"),
        "{violations:?}"
    );
    ctx.deregister_table("modeled_exact_value_transfers")
        .unwrap();
    ctx.register_table("modeled_exact_value_transfers", original_exact_transfers)
        .unwrap();

    let original_expressions = sql::query(&ctx, "SELECT * FROM expression_evaluations")
        .await
        .unwrap()
        .into_view();
    let forged_expressions = sql::query(&ctx,
        "SELECT * EXCLUDE (boolean_value), CASE WHEN normal THEN COALESCE(NOT boolean_value, true) ELSE NULL END AS boolean_value FROM expression_evaluations")
        .await.unwrap().into_view();
    ctx.deregister_table("expression_evaluations").unwrap();
    ctx.register_table("expression_evaluations", forged_expressions)
        .unwrap();
    let violations = cpg_core::validate::validate(&ctx).await.unwrap();
    assert!(
        violations
            .iter()
            .any(|v| v.rule == "expression-source-equality"),
        "{violations:?}"
    );
    ctx.deregister_table("expression_evaluations").unwrap();
    ctx.register_table("expression_evaluations", original_expressions)
        .unwrap();

    let original_argument_evaluations =
        sql::query(&ctx, "SELECT * FROM modeled_argument_evaluations")
            .await
            .unwrap()
            .into_view();
    let missing_argument_evaluations = sql::query(
        &ctx,
        "SELECT * FROM modeled_argument_evaluations WHERE false",
    )
    .await
    .unwrap()
    .into_view();
    ctx.deregister_table("modeled_argument_evaluations")
        .unwrap();
    ctx.register_table("modeled_argument_evaluations", missing_argument_evaluations)
        .unwrap();
    let violations = cpg_core::validate::validate(&ctx).await.unwrap();
    assert!(
        violations
            .iter()
            .any(|v| v.rule == "modeled-argument-evaluation-source-equality"),
        "{violations:?}"
    );
    ctx.deregister_table("modeled_argument_evaluations")
        .unwrap();
    ctx.register_table(
        "modeled_argument_evaluations",
        original_argument_evaluations,
    )
    .unwrap();

    let original_assignment_paths =
        sql::query(&ctx, "SELECT * FROM modeled_assignment_return_paths")
            .await
            .unwrap()
            .into_view();
    let missing_assignment_paths = sql::query(
        &ctx,
        "SELECT * FROM modeled_assignment_return_paths WHERE false",
    )
    .await
    .unwrap()
    .into_view();
    ctx.deregister_table("modeled_assignment_return_paths")
        .unwrap();
    ctx.register_table("modeled_assignment_return_paths", missing_assignment_paths)
        .unwrap();
    let violations = cpg_core::validate::validate(&ctx).await.unwrap();
    assert!(
        violations
            .iter()
            .any(|v| v.rule == "modeled-assignment-return-path-source-equality"),
        "{violations:?}"
    );
    ctx.deregister_table("modeled_assignment_return_paths")
        .unwrap();
    ctx.register_table("modeled_assignment_return_paths", original_assignment_paths)
        .unwrap();

    let original_summary_components = sql::query(&ctx, "SELECT * FROM summary_components")
        .await
        .unwrap()
        .into_view();
    let missing_summary_components =
        sql::query(&ctx, "SELECT * FROM summary_components WHERE false")
            .await
            .unwrap()
            .into_view();
    ctx.deregister_table("summary_components").unwrap();
    ctx.register_table("summary_components", missing_summary_components)
        .unwrap();
    let violations = cpg_core::validate::validate(&ctx).await.unwrap();
    assert!(
        violations
            .iter()
            .any(|v| v.rule == "summary-component-source-equality"),
        "{violations:?}"
    );
    ctx.deregister_table("summary_components").unwrap();
    ctx.register_table("summary_components", original_summary_components)
        .unwrap();

    let original_summary_flows = sql::query(&ctx, "SELECT * FROM summary_flows")
        .await
        .unwrap()
        .into_view();
    let missing_summary_flows = sql::query(&ctx, "SELECT * FROM summary_flows WHERE false")
        .await
        .unwrap()
        .into_view();
    ctx.deregister_table("summary_flows").unwrap();
    ctx.register_table("summary_flows", missing_summary_flows)
        .unwrap();
    let violations = cpg_core::validate::validate(&ctx).await.unwrap();
    assert!(
        violations
            .iter()
            .any(|v| v.rule == "summary-flow-source-equality"),
        "{violations:?}"
    );
    ctx.deregister_table("summary_flows").unwrap();
    ctx.register_table("summary_flows", original_summary_flows)
        .unwrap();

    let original_summary_steps = sql::query(&ctx, "SELECT * FROM summary_flow_steps")
        .await
        .unwrap()
        .into_view();
    let missing_summary_steps = sql::query(&ctx, "SELECT * FROM summary_flow_steps WHERE false")
        .await
        .unwrap()
        .into_view();
    ctx.deregister_table("summary_flow_steps").unwrap();
    ctx.register_table("summary_flow_steps", missing_summary_steps)
        .unwrap();
    let violations = cpg_core::validate::validate(&ctx).await.unwrap();
    assert!(
        violations
            .iter()
            .any(|v| v.rule == "summary-flow-step-source-equality"),
        "{violations:?}"
    );
    ctx.deregister_table("summary_flow_steps").unwrap();
    ctx.register_table("summary_flow_steps", original_summary_steps)
        .unwrap();

    let original_summary_boundaries = sql::query(&ctx, "SELECT * FROM summary_boundaries")
        .await
        .unwrap()
        .into_view();
    let missing_summary_boundaries =
        sql::query(&ctx, "SELECT * FROM summary_boundaries WHERE false")
            .await
            .unwrap()
            .into_view();
    ctx.deregister_table("summary_boundaries").unwrap();
    ctx.register_table("summary_boundaries", missing_summary_boundaries)
        .unwrap();
    let violations = cpg_core::validate::validate(&ctx).await.unwrap();
    assert!(
        violations
            .iter()
            .any(|v| v.rule == "summary-boundary-source-equality"),
        "{violations:?}"
    );
    ctx.deregister_table("summary_boundaries").unwrap();
    ctx.register_table("summary_boundaries", original_summary_boundaries)
        .unwrap();

    let original_predecessors = sql::query(&ctx, "SELECT * FROM value_flow_predecessor_candidates")
        .await
        .unwrap()
        .into_view();
    let missing_predecessors = sql::query(
        &ctx,
        "SELECT * FROM value_flow_predecessor_candidates WHERE false",
    )
    .await
    .unwrap()
    .into_view();
    ctx.deregister_table("value_flow_predecessor_candidates")
        .unwrap();
    ctx.register_table("value_flow_predecessor_candidates", missing_predecessors)
        .unwrap();
    let violations = cpg_core::validate::validate(&ctx).await.unwrap();
    assert!(
        violations
            .iter()
            .any(|v| v.rule == "value-flow-predecessor-source-equality"),
        "{violations:?}"
    );
    ctx.deregister_table("value_flow_predecessor_candidates")
        .unwrap();
    ctx.register_table("value_flow_predecessor_candidates", original_predecessors)
        .unwrap();

    let original_compatibility =
        sql::query(&ctx, "SELECT * FROM value_flow_predecessor_compatibility")
            .await
            .unwrap()
            .into_view();
    let missing_compatibility = sql::query(
        &ctx,
        "SELECT * FROM value_flow_predecessor_compatibility WHERE false",
    )
    .await
    .unwrap()
    .into_view();
    ctx.deregister_table("value_flow_predecessor_compatibility")
        .unwrap();
    ctx.register_table(
        "value_flow_predecessor_compatibility",
        missing_compatibility,
    )
    .unwrap();
    let violations = cpg_core::validate::validate(&ctx).await.unwrap();
    assert!(
        violations
            .iter()
            .any(|v| v.rule == "value-flow-predecessor-compatibility-equality"),
        "{violations:?}"
    );
    ctx.deregister_table("value_flow_predecessor_compatibility")
        .unwrap();
    ctx.register_table(
        "value_flow_predecessor_compatibility",
        original_compatibility,
    )
    .unwrap();

    assert_eq!(count(&ctx,"SELECT count(*) FROM source_modeled_identities i JOIN declarations d ON d.node_id=i.function_node_id JOIN value_flow_contributions c ON c.origin_id=i.source_origin_id WHERE d.name IN ('framed_modeled_identity','nested_framed_modeled_identity') AND c.approximated").await,2,
        "modeled source certificates preserve raw approximation");
    let original_modeled = sql::query(&ctx, "SELECT * FROM source_modeled_identities")
        .await
        .unwrap()
        .into_view();
    let missing_modeled = sql::query(&ctx, "SELECT * FROM source_modeled_identities WHERE false")
        .await
        .unwrap()
        .into_view();
    ctx.deregister_table("source_modeled_identities").unwrap();
    ctx.register_table("source_modeled_identities", missing_modeled)
        .unwrap();
    let violations = cpg_core::validate::validate(&ctx).await.unwrap();
    assert!(
        violations
            .iter()
            .any(|v| v.rule == "source-modeled-identity-equality"),
        "{violations:?}"
    );
    ctx.deregister_table("source_modeled_identities").unwrap();
    ctx.register_table("source_modeled_identities", original_modeled)
        .unwrap();

    let original_analysis_conditions = sql::query(&ctx, "SELECT * FROM analysis_conditions")
        .await
        .unwrap()
        .into_view();
    let missing_analysis_conditions =
        sql::query(&ctx, "SELECT * FROM analysis_conditions WHERE false")
            .await
            .unwrap()
            .into_view();
    ctx.deregister_table("analysis_conditions").unwrap();
    ctx.register_table("analysis_conditions", missing_analysis_conditions)
        .unwrap();
    let violations = cpg_core::validate::validate(&ctx).await.unwrap();
    assert!(
        violations
            .iter()
            .any(|v| v.rule == "analysis-condition-source-equality"),
        "{violations:?}"
    );
    ctx.deregister_table("analysis_conditions").unwrap();
    ctx.register_table("analysis_conditions", original_analysis_conditions)
        .unwrap();

    let original_effect_sites = sql::query(&ctx, "SELECT * FROM modeled_effect_sites")
        .await
        .unwrap()
        .into_view();
    let doctored_effect_sites = sql::query(
        &ctx,
        &format!(
            "SELECT * EXCLUDE (subject_status), CAST({} AS SMALLINT) AS subject_status \
             FROM modeled_effect_sites",
            ModelEffectSubjectStatus::BoundArgument.code()
        ),
    )
    .await
    .unwrap()
    .into_view();
    ctx.deregister_table("modeled_effect_sites").unwrap();
    ctx.register_table("modeled_effect_sites", doctored_effect_sites)
        .unwrap();
    let violations = cpg_core::validate::validate(&ctx).await.unwrap();
    assert!(
        violations
            .iter()
            .any(|v| v.rule == "semantic:modeled-effect-site-shape"),
        "{violations:?}"
    );
    assert!(
        violations
            .iter()
            .any(|v| v.rule == "modeled-effect-site-source-equality"),
        "{violations:?}"
    );
    ctx.deregister_table("modeled_effect_sites").unwrap();
    ctx.register_table("modeled_effect_sites", original_effect_sites)
        .unwrap();

    let original_exception_sites = sql::query(&ctx, "SELECT * FROM modeled_exception_sites")
        .await
        .unwrap()
        .into_view();
    let doctored_exception_sites = sql::query(
        &ctx,
        "SELECT * EXCLUDE (class_node_id, candidate_set_complete_under_model, \
          has_unresolved_remainder), target_node_id AS class_node_id, \
          true AS candidate_set_complete_under_model, true AS has_unresolved_remainder \
         FROM modeled_exception_sites",
    )
    .await
    .unwrap()
    .into_view();
    ctx.deregister_table("modeled_exception_sites").unwrap();
    ctx.register_table("modeled_exception_sites", doctored_exception_sites)
        .unwrap();
    let violations = cpg_core::validate::validate(&ctx).await.unwrap();
    assert!(
        violations
            .iter()
            .any(|v| v.rule == "semantic:modeled-exception-site-shape"),
        "{violations:?}"
    );
    assert!(
        violations
            .iter()
            .any(|v| v.rule == "modeled-exception-site-source-equality"),
        "{violations:?}"
    );
    ctx.deregister_table("modeled_exception_sites").unwrap();
    ctx.register_table("modeled_exception_sites", original_exception_sites)
        .unwrap();

    let doctored = sql::query(
        &ctx,
        "SELECT * EXCLUDE (input_path), 'Parameter[wrong]' AS input_path FROM model_transfers",
    )
    .await
    .unwrap()
    .into_view();
    ctx.deregister_table("model_transfers").unwrap();
    ctx.register_table("model_transfers", doctored).unwrap();
    let violations = cpg_core::validate::validate(&ctx).await.unwrap();
    assert!(
        violations
            .iter()
            .any(|v| v.rule == "model-catalog-transfer-equality"),
        "{violations:?}"
    );
    let doctored_effect = sql::query(
        &ctx,
        &format!(
            "SELECT * EXCLUDE (effect), CAST({} AS SMALLINT) AS effect FROM model_effects",
            ModelEffectKind::IoRead.code()
        ),
    )
    .await
    .unwrap()
    .into_view();
    ctx.deregister_table("model_effects").unwrap();
    ctx.register_table("model_effects", doctored_effect)
        .unwrap();
    let violations = cpg_core::validate::validate(&ctx).await.unwrap();
    assert!(
        violations
            .iter()
            .any(|v| v.rule == "model-catalog-effect-equality"),
        "{violations:?}"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn resolved_type_guard_publishes_only_a_cited_exact_class_origin() {
    let root = tempfile::tempdir().unwrap();
    let snapshot = Id([49; 16]);
    let analysis = Analysis {
        embedding_cache: None,
        config: AnalyticsConfig::parse(
            r#"
version = 1
[subsystem]
module_prefixes = ["guardpkg"]
public_roots = ["guardpkg"]
[seeds]
primary = ["guardpkg.exact"]
distractors = []
[pass_a]
max_depth = 2
max_vertices = 128
max_edges = 512
max_witnesses = 3
[briefs]
budget = 1
"#,
        )
        .unwrap(),
        embedder: Some(std::sync::Arc::new(cpg_core::embedding_service::FakeEmbedder::new())),
        techniques: Techniques::default(),
    };
    compile_analyzed(
        root.path(),
        snapshot,
        &raw("type_guard", snapshot),
        Some(&analysis),
    )
    .await
    .unwrap();
    let (_, ctx) = published(root.path(), snapshot).await.unwrap().unwrap();
    assert_eq!(
        count(&ctx, "SELECT count(*) FROM flow_test_exact_origins").await,
        3
    );
    assert_eq!(
        count(
            &ctx,
            "SELECT count(*) FROM flow_test_exact_origins o \
             JOIN flow_test_value_links l ON l.link_id = o.value_link_id \
             WHERE o.builtin_class = 'str' AND l.origin = 1 \
               AND o.atom_id = l.atom_id AND o.use_id = l.use_id",
        )
        .await,
        3
    );
    let stable_count = count(
        &ctx,
        "SELECT count(*) FROM flow_test_value_links l \
            JOIN declarations d ON d.node_id = l.operation_node_id \
            WHERE d.name = 'nested' AND l.origin = 2 AND l.stability_origin_id IS NOT NULL",
    )
    .await;
    assert_eq!(
        stable_count,
        1,
        "{}\n{}\n{}",
        text(
            &ctx,
            "SELECT d.name, l.atom, l.leaf_start_byte, l.leaf_end_byte, \
            l.condition_id, c.encoding FROM flow_test_leaves l \
            JOIN declarations d ON d.module_node_id = l.module_node_id \
                AND d.name_start_byte = l.scope_start_byte \
                AND d.name_end_byte = l.scope_end_byte \
            JOIN conditions c ON c.condition_id = l.condition_id \
            WHERE d.name = 'nested' ORDER BY l.leaf_start_byte"
        )
        .await,
        text(
            &ctx,
            "SELECT d.name, l.origin, l.place, l.operand_start_byte, l.condition_id \
            FROM flow_test_value_links l JOIN declarations d ON d.node_id = l.operation_node_id \
            WHERE d.name = 'nested'"
        )
        .await,
        text(
            &ctx,
            "SELECT u.start_byte, u.place, r.condition_id, c.encoding, r.approximated, r.loop_carried \
            FROM flow_uses u JOIN flow_reaching r ON r.use_id = u.use_id \
            JOIN conditions c ON c.condition_id = r.condition_id \
            JOIN declarations d ON d.module_node_id = u.module_node_id \
                AND d.name_start_byte = u.scope_start_byte \
                AND d.name_end_byte = u.scope_end_byte \
            WHERE d.name = 'nested' AND u.place = 'x' ORDER BY u.start_byte"
        )
        .await
    );
    assert_eq!(
        count(
            &ctx,
            "SELECT count(*) FROM flow_test_value_links l \
            JOIN declarations d ON d.node_id = l.operation_node_id \
            WHERE d.name = 'nested_after_call' AND l.origin = 2"
        )
        .await,
        0
    );
    assert!(cpg_core::validate::validate(&ctx).await.unwrap().is_empty());

    let table = |name: &'static str| Relation {
        name,
        sql: format!("SELECT * FROM {name}"),
        deps: &[],
    };
    let links: Vec<FlowTestValueLinksRow> =
        sql::fetch(&ctx, &table("flow_test_value_links"), sql::Params::new())
            .await
            .unwrap();
    let origins: Vec<FlowTestExactOriginsRow> =
        sql::fetch(&ctx, &table("flow_test_exact_origins"), sql::Params::new())
            .await
            .unwrap();
    let leaves: Vec<FlowTestLeavesRow> =
        sql::fetch(&ctx, &table("flow_test_leaves"), sql::Params::new())
            .await
            .unwrap();
    let roots: Vec<ConditionsRow> = sql::fetch(&ctx, &table("conditions"), sql::Params::new())
        .await
        .unwrap();
    let nodes: Vec<ConditionNodesRow> =
        sql::fetch(&ctx, &table("condition_nodes"), sql::Params::new())
            .await
            .unwrap();
    let diagrams = hydrate_catalog(
        &roots
            .into_iter()
            .map(|row| ConditionRoot {
                condition_id: row.condition_id,
                root_id: row.root_id,
                boundary_reason: row.boundary_reason,
            })
            .collect::<Vec<_>>(),
        &nodes
            .into_iter()
            .map(|row| DiagramNode {
                node_id: row.node_id,
                atom: row.atom,
                low: row.low_id,
                high: row.high_id,
            })
            .collect::<Vec<_>>(),
    )
    .unwrap();
    let query_links: Vec<cpg_schema::primitive_theory::ValueLink> =
        links.iter().map(Into::into).collect();
    let query_leaves: Vec<cpg_schema::primitive_theory::TestLeaf> =
        leaves.iter().map(Into::into).collect();
    let proof = &origins[0];
    let guard = &diagrams[&proof.test_condition_id];
    assert!(
        cpg_schema::primitive_theory::refute_exact_input(
            guard,
            cpg_schema::primitive_theory::ExactInput {
                operation_node_id: proof.operation_node_id,
                formal_node_id: proof.formal_node_id,
                value: &Value::None,
                builtin_namespace: cpg_schema::primitive_theory::BuiltinNamespace::StandardAssumed,
                effect_model_digest: cpg_core::entry_links::digest(),
            },
            &query_links,
            &query_leaves,
        )
        .unwrap()
        .is_some()
    );
    assert!(
        cpg_schema::primitive_theory::refute_exact_input(
            guard,
            cpg_schema::primitive_theory::ExactInput {
                operation_node_id: proof.operation_node_id,
                formal_node_id: proof.formal_node_id,
                value: &Value::Str("hi".to_owned()),
                builtin_namespace: cpg_schema::primitive_theory::BuiltinNamespace::StandardAssumed,
                effect_model_digest: cpg_core::entry_links::digest(),
            },
            &query_links,
            &query_leaves,
        )
        .unwrap()
        .is_none()
    );

    let doctored = sql::query(
        &ctx,
        "SELECT * EXCLUDE (builtin_class), 'int' AS builtin_class FROM flow_test_exact_origins",
    )
    .await
    .unwrap()
    .into_view();
    ctx.deregister_table("flow_test_exact_origins").unwrap();
    ctx.register_table("flow_test_exact_origins", doctored)
        .unwrap();
    let violations = cpg_core::validate::validate(&ctx).await.unwrap();
    assert!(
        violations
            .iter()
            .any(|v| v.rule == "flow-test-exact-origin"),
        "{violations:?}"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn entry_value_links_require_a_direct_uninterrupted_parameter_reach() {
    let root = tempfile::tempdir().unwrap();
    let snapshot = Id([48; 16]);
    let analysis = Analysis {
        embedding_cache: None,
        config: AnalyticsConfig::parse(
            r#"
version = 1
[subsystem]
module_prefixes = ["bridgepkg"]
public_roots = ["bridgepkg"]
[seeds]
primary = ["bridgepkg.entry"]
distractors = []
[pass_a]
max_depth = 2
max_vertices = 128
max_edges = 512
max_witnesses = 3
[briefs]
budget = 1
"#,
        )
        .unwrap(),
        embedder: Some(std::sync::Arc::new(cpg_core::embedding_service::FakeEmbedder::new())),
        techniques: Techniques::default(),
    };
    compile_analyzed(
        root.path(),
        snapshot,
        &raw("entry_bridge", snapshot),
        Some(&analysis),
    )
    .await
    .unwrap();
    let (_, ctx) = published(root.path(), snapshot).await.unwrap().unwrap();
    let count_for = |name: &str| {
        format!(
            "SELECT count(*) FROM flow_test_value_links l \
         JOIN declarations d ON d.node_id = l.operation_node_id WHERE d.name = '{name}'"
        )
    };
    assert!(count(&ctx, &count_for("direct")).await > 0);
    assert!(count(&ctx, &count_for("documented_direct")).await > 0);
    assert_eq!(count(&ctx, &count_for("rebound")).await, 0);
    assert_eq!(count(&ctx, &count_for("after_call")).await, 1);
    assert_eq!(count(&ctx, &count_for("after_operator")).await, 1);
    assert_eq!(count(&ctx, &count_for("changed_closure")).await, 0);
    assert!(cpg_core::validate::validate(&ctx).await.unwrap().is_empty());

    let doctored = sql::query(
        &ctx,
        "SELECT * EXCLUDE (place), concat(place, '_tampered') AS place FROM flow_test_value_links",
    )
    .await
    .unwrap()
    .into_view();
    ctx.deregister_table("flow_test_value_links").unwrap();
    ctx.register_table("flow_test_value_links", doctored)
        .unwrap();
    let violations = cpg_core::validate::validate(&ctx).await.unwrap();
    assert!(
        violations
            .iter()
            .any(|v| v.rule == "flow-test-value-proof-link"),
        "{violations:?}"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn derived_tables_on_the_variant_fixture() {
    let (out, ctx, _root) = derived_text("pysa_variants").await;
    insta::assert_snapshot!(out);
    // Resolutions with no Pysa record agree with the extractor's call boundaries, reason by reason.
    for reason in [7, 10] {
        let resolutions = count(
            &ctx,
            &format!("SELECT count(*) FROM resolutions WHERE reason = {reason}"),
        )
        .await;
        let boundaries = count(
            &ctx,
            &format!("SELECT count(*) FROM boundaries WHERE fact_family = 3 AND reason = {reason}"),
        )
        .await;
        assert_eq!(resolutions, boundaries, "reason {reason}");
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn derived_tables_on_the_unicode_fixture() {
    let (out, ctx, _root) = derived_text("unicode_bom").await;
    insta::assert_snapshot!(out);
    // `lcfix.build` re-exports the implementation, not an `@overload` stub; the two stubs carry
    // Pysa's two signatures in source order and the implementation none.
    let seed = text(
        &ctx,
        "SELECT d.qualified_name, d.is_overload FROM exports e \
         JOIN declarations d ON d.node_id = e.declaration_node_id \
         WHERE e.access_path = 'lcfix.build'",
    )
    .await;
    assert!(
        seed.contains("lcfix.core.build") && seed.contains("false"),
        "{seed}"
    );
    let indexes = text(
        &ctx,
        "SELECT d.is_overload, s.signature_index FROM signatures s \
         JOIN declarations d ON d.node_id = s.signature_node_id \
         WHERE d.qualified_name = 'lcfix.core.build' ORDER BY d.start_byte",
    )
    .await;
    let lines: Vec<&str> = indexes
        .lines()
        .filter(|l| l.starts_with("| "))
        .skip(1)
        .collect();
    assert_eq!(
        lines
            .iter()
            .map(|l| l.split_whitespace().collect::<Vec<_>>().join(" "))
            .collect::<Vec<_>>(),
        ["| true | 0 |", "| true | 1 |", "| false | |"]
    );
    assert_eq!(
        count(
            &ctx,
            "SELECT count(*) FROM signatures WHERE reason IS NOT NULL"
        )
        .await,
        0
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn derived_tables_on_the_keys_fixture() {
    let (out, ctx, _root) = derived_text("pysa_keys").await;
    insta::assert_snapshot!(out);
    // From outside, `dual.g()` targets the stub's `g`; inside `dual.py`, `f(1)` targets the
    // source's `f` (review F3).
    let targets = text(
        &ctx,
        "SELECT f.path AS caller, g.path AS target FROM call_targets t \
         JOIN call_syntax c ON c.node_id = t.call_site_node_id \
         JOIN source_files f ON f.module_node_id = c.module_node_id \
         JOIN declarations d ON d.node_id = t.target_node_id \
         JOIN source_files g ON g.module_node_id = d.module_node_id \
         ORDER BY caller, target",
    )
    .await;
    assert!(
        targets.contains("keys/use_dual.py | keys/dual.pyi"),
        "{targets}"
    );
    assert!(
        targets.contains("keys/dual.py     | keys/dual.py"),
        "{targets}"
    );
    // Slice-2 review F5: one `exports` row per file of a `.py`/`.pyi` pair, each seeding the
    // declaration in its own file.
    let seeds = text(
        &ctx,
        "SELECT e.access_path, o.path AS origin, g.path AS seed FROM exports e \
         JOIN public_names p ON p.fact_id = e.public_fact_id \
         JOIN source_files o ON o.module_node_id = p.origin_module_node_id \
         JOIN declarations d ON d.node_id = e.declaration_node_id \
         JOIN source_files g ON g.module_node_id = d.module_node_id \
         WHERE e.access_path = 'keys.dual.f' ORDER BY origin",
    )
    .await;
    let rows: Vec<String> = seeds
        .lines()
        .filter(|l| l.starts_with("| keys"))
        .map(|l| l.split_whitespace().collect::<Vec<_>>().join(" "))
        .collect();
    assert_eq!(
        rows,
        [
            "| keys.dual.f | keys/dual.py | keys/dual.py |",
            "| keys.dual.f | keys/dual.pyi | keys/dual.pyi |"
        ]
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn derived_tables_on_the_derive_cases_fixture() {
    let (out, ctx, _root) = derived_text("derive_cases").await;
    insta::assert_snapshot!(out);
    // F1, then ADR-0014: constructing a release dataclass calls its synthesized `__init__`, which
    // has no declaration: the target is a typed `synthetic_callable` node, with no reason left.
    let init = text(
        &ctx,
        "SELECT n.node_kind, t.reason FROM call_targets t \
         JOIN pysa_calls p ON p.fact_id = t.pysa_fact_id \
         JOIN nodes n ON n.node_id = t.target_node_id \
         WHERE p.target_name = '__init__' AND p.target_module = '@dc/impl.py'",
    )
    .await;
    assert!(init.contains("| 9         |"), "synthetic_callable: {init}");
    // F2: the public `load` is the definition Pyrefly binds under Python 3.14 (the `if` branch);
    // the `else` branch is unreachable in the context, not missing evidence.
    let load = text(
        &ctx,
        "SELECT d.start_byte, s.reason FROM exports e \
         JOIN declarations d ON d.node_id = e.declaration_node_id \
         JOIN signatures s ON s.signature_node_id = d.node_id \
         WHERE e.access_path = 'dc.load'",
    )
    .await;
    let live = count(
        &ctx,
        "SELECT min(start_byte) FROM declarations WHERE qualified_name = 'dc.compat.load'",
    )
    .await;
    assert!(load.contains(&format!("| {live} ")), "{load}");
    let dead = count(
        &ctx,
        "SELECT count(*) FROM signatures s JOIN declarations d ON d.node_id = s.signature_node_id \
         WHERE d.qualified_name IN ('dc.compat.load', 'dc.ov.f') AND s.reason = 14",
    )
    .await;
    assert_eq!(dead, 1 + 2, "the dead `load` and the two dead `f` stubs");
    let placed = count(
        &ctx,
        "SELECT count(*) FROM signatures s JOIN declarations d ON d.node_id = s.signature_node_id \
         WHERE d.qualified_name = 'dc.ov.f' AND s.signature_index IS NOT NULL",
    )
    .await;
    assert_eq!(placed, 2, "the live stubs carry Pysa's two signatures");
}

fn replace(batch: &RecordBatch, column: &str, array: ArrayRef) -> RecordBatch {
    let i = batch.schema().index_of(column).unwrap();
    let mut columns = batch.columns().to_vec();
    columns[i] = array;
    RecordBatch::try_new(batch.schema(), columns).unwrap()
}

fn table<'a>(raw: &'a mut [(&'static str, RecordBatch)], name: &str) -> &'a mut RecordBatch {
    &mut raw.iter_mut().find(|(n, _)| *n == name).unwrap().1
}

fn drop_rows(raw: &mut [(&'static str, RecordBatch)], name: &str, n: usize) {
    let b = table(raw, name);
    *b = b.slice(n, b.num_rows() - n);
}

/// Keep the rows of `name` whose `column` (as text) satisfies `keep`.
fn keep_rows(
    raw: &mut [(&'static str, RecordBatch)],
    name: &str,
    column: &str,
    keep: impl Fn(&str) -> bool,
) {
    let b = table(raw, name);
    let values = arrow_cast::cast(
        b.column(b.schema().index_of(column).unwrap()),
        &arrow_schema::DataType::Utf8,
    )
    .unwrap();
    let values = values
        .as_any()
        .downcast_ref::<arrow_array::StringArray>()
        .unwrap();
    let mask: arrow_array::BooleanArray =
        values.iter().map(|v| Some(keep(v.unwrap_or("")))).collect();
    *b = arrow_select::filter::filter_record_batch(b, &mask).unwrap();
}

/// Each rule kind rejects a snapshot that breaks it, and nothing is published (DM-53).
#[tokio::test(flavor = "multi_thread")]
async fn every_rule_kind_rejects_its_violation() {
    let s = Id([4; 16]);
    let base = raw("pysa_variants", s);
    type Mutation = fn(&mut Vec<(&'static str, RecordBatch)>);
    let cases: [(&str, Mutation); 29] = [
        ("key:declarations", |raw| {
            let b = table(raw, "declarations");
            *b = arrow_select::concat::concat_batches(&b.schema(), [&*b, &b.slice(0, 1)]).unwrap();
        }),
        ("ref:call_syntax.owner_node_id", |raw| {
            let b = table(raw, "call_syntax");
            let owners =
                FixedSizeBinaryArray::try_from_iter(std::iter::repeat_n([9u8; 16], b.num_rows()))
                    .unwrap();
            *b = replace(b, "owner_node_id", Arc::new(owners));
        }),
        ("fact:", |raw| drop_rows(raw, "facts", 1)),
        ("codebook:boundaries.reason", |raw| {
            let b = table(raw, "boundaries");
            let reasons = Int16Array::from(vec![99_i16; b.num_rows()]);
            *b = replace(b, "reason", Arc::new(reasons));
        }),
        ("coverage:complete", |raw| drop_rows(raw, "coverage", 1)),
        ("fact-payload:declarations", |raw| {
            drop_rows(raw, "declarations", 1)
        }),
        ("coverage:declared-family", |raw| {
            let b = table(raw, "runs");
            let field = match b.schema().field_with_name("families").unwrap().data_type() {
                arrow_schema::DataType::List(f) => f.clone(),
                other => panic!("{other}"),
            };
            let mut list =
                arrow_array::builder::ListBuilder::new(arrow_array::builder::StringBuilder::new())
                    .with_field(field);
            for _ in 0..b.num_rows() {
                list.values().append_value("bogus");
                list.append(true);
            }
            *b = replace(b, "families", Arc::new(list.finish()));
        }),
        // Slice-2 review F3: a `resolutions` reason with no matching call boundary.
        ("semantic:resolution-has-boundary", |raw| {
            keep_rows(raw, "boundaries", "fact_family", |f| f != "3");
            keep_rows(raw, "facts", "table_name", |t| t != "boundaries");
        }),
        // C6 review F1: a call boundary whose reason its resolution does not give.
        ("semantic:boundary-has-resolution", |raw| {
            let b = table(raw, "boundaries");
            let column = |c: &str| {
                b.column(b.schema().index_of(c).unwrap())
                    .as_any()
                    .downcast_ref::<Int16Array>()
                    .unwrap()
                    .clone()
            };
            let (families, reasons) = (column("fact_family"), column("reason"));
            let moved: Int16Array = families
                .iter()
                .zip(reasons.iter())
                .map(|(f, r)| match (f, r) {
                    (Some(3), Some(0)) => Some(1),
                    (Some(3), Some(_)) => Some(0),
                    (_, r) => r,
                })
                .collect();
            *b = replace(b, "reason", Arc::new(moved));
        }),
        // Slice-2 F1, then ADR-0014 review F1: a release target that names nothing publishes
        // with no reason, and the typed rule rejects it (no catch-all reason hides it).
        ("typed:call_targets", |raw| {
            let b = table(raw, "pysa_calls");
            let i = b.schema().index_of("target_module").unwrap();
            let modules = b
                .column(i)
                .as_any()
                .downcast_ref::<arrow_array::StringArray>()
                .unwrap();
            let moved: arrow_array::StringArray = modules
                .iter()
                .map(|m| m.map(|m| if m.starts_with('@') { "@nowhere.py" } else { m }))
                .collect();
            *b = replace(b, "target_module", Arc::new(moved));
        }),
        // Review F1: a dependency target whose definition is missing is our failure, too.
        ("typed:call_targets", |raw| {
            let b = table(raw, "pysa_calls");
            let (m, k) = (
                b.schema().index_of("target_module").unwrap(),
                b.schema().index_of("target_key").unwrap(),
            );
            let modules = b
                .column(m)
                .as_any()
                .downcast_ref::<arrow_array::StringArray>()
                .unwrap()
                .clone();
            let keys: arrow_array::StringArray = b
                .column(k)
                .as_any()
                .downcast_ref::<arrow_array::StringArray>()
                .unwrap()
                .iter()
                .zip(modules.iter())
                .map(|(key, module)| match (key, module) {
                    (Some(_), Some(m)) if !m.starts_with('@') => Some("F:999999"),
                    (key, _) => key,
                })
                .collect();
            *b = replace(b, "target_key", Arc::new(keys));
        }),
        // ADR-0022 §The flow provider: parity with ty, both ways, and reaching within our
        // candidates. A name use moved off every reference...
        ("semantic:flow-use-is-a-reference", |raw| {
            let b = table(raw, "flow_uses");
            let shift = |c: &str| -> Arc<dyn Array> {
                let a = b
                    .column(b.schema().index_of(c).unwrap())
                    .as_any()
                    .downcast_ref::<Int64Array>()
                    .unwrap();
                Arc::new(
                    a.iter()
                        .map(|v| v.map(|v| v + 1_000_000))
                        .collect::<Int64Array>(),
                )
            };
            let (start, end) = (shift("start_byte"), shift("end_byte"));
            *b = replace(&replace(b, "start_byte", start), "end_byte", end);
        }),
        // ...a reference ty never read...
        ("semantic:reference-is-a-flow-use", |raw| {
            keep_rows(raw, "flow_uses", "place", |_| false);
        }),
        // ...a definition no binding matches...
        ("semantic:flow-definition-is-a-binding", |raw| {
            let b = table(raw, "flow_definitions");
            let shift = |c: &str| -> Arc<dyn Array> {
                let a = b
                    .column(b.schema().index_of(c).unwrap())
                    .as_any()
                    .downcast_ref::<Int64Array>()
                    .unwrap();
                Arc::new(
                    a.iter()
                        .map(|v| v.map(|v| v + 1_000_000))
                        .collect::<Int64Array>(),
                )
            };
            let (start, end) = (shift("start_byte"), shift("end_byte"));
            *b = replace(&replace(b, "start_byte", start), "end_byte", end);
        }),
        // ...a binding ty never defined...
        ("semantic:binding-is-a-flow-definition", |raw| {
            keep_rows(raw, "flow_definitions", "place", |_| false);
        }),
        // ...and a use reached by a definition we never resolve it to (every reaching row points
        // at the first definition).
        ("semantic:flow-reaching-within-candidates", |raw| {
            let first = table(raw, "flow_definitions")
                .column_by_name("definition_id")
                .unwrap()
                .as_any()
                .downcast_ref::<FixedSizeBinaryArray>()
                .unwrap()
                .value(0)
                .to_vec();
            let b = table(raw, "flow_reaching");
            let ids = FixedSizeBinaryArray::try_from_iter(std::iter::repeat_n(first, b.num_rows()))
                .unwrap();
            *b = replace(b, "definition_id", Arc::new(ids));
        }),
        // Review O2: two rows asserting one argument id are a collision, never merged.
        ("key:nodes", |raw| {
            let b = table(raw, "arguments");
            let dup = b.slice(0, 1);
            let fact = FixedSizeBinaryArray::try_from_iter(std::iter::once([7u8; 16])).unwrap();
            let dup = replace(&dup, "fact_id", Arc::new(fact));
            *b = arrow_select::concat::concat_batches(&b.schema(), [&*b, &dup]).unwrap();
        }),
        // C2 review F3: a declaration without its placement row.
        ("placed:declarations", |raw| {
            let b = table(raw, "syntax_nodes");
            *b = b.slice(0, 0);
        }),
        // C2 review F3: a `def` placed on a span its body does not fit in.
        ("contained:syntax_nodes", |raw| {
            let b = table(raw, "syntax_nodes");
            let def = cpg_schema::codebook::SyntaxKind::StmtFunctionDef.code();
            let kinds = b
                .column(b.schema().index_of("kind").unwrap())
                .as_any()
                .downcast_ref::<Int16Array>()
                .unwrap()
                .clone();
            for column in ["start_byte", "end_byte"] {
                let i = b.schema().index_of(column).unwrap();
                let moved: Int64Array = b
                    .column(i)
                    .as_any()
                    .downcast_ref::<Int64Array>()
                    .unwrap()
                    .iter()
                    .zip(kinds.iter())
                    .map(|(v, k)| if k == Some(def) { Some(0) } else { v })
                    .collect();
                *b = replace(b, column, Arc::new(moved));
            }
        }),
        // C2 review F2/F3: a Pysa site no node sits at is our failure, never a catch-all reason.
        ("typed:site_targets", |raw| {
            let b = table(raw, "pysa_calls");
            let sites = b
                .column(b.schema().index_of("site_kind").unwrap())
                .as_any()
                .downcast_ref::<Int16Array>()
                .unwrap()
                .clone();
            for column in ["start_byte", "end_byte"] {
                let i = b.schema().index_of(column).unwrap();
                let moved: Int64Array = b
                    .column(i)
                    .as_any()
                    .downcast_ref::<Int64Array>()
                    .unwrap()
                    .iter()
                    .zip(sites.iter())
                    .map(|(v, k)| {
                        if k == Some(0) {
                            v
                        } else {
                            v.map(|v| v + 1_000_000)
                        }
                    })
                    .collect();
                *b = replace(b, column, Arc::new(moved));
            }
        }),
        // C2 review F3: one node placed twice.
        ("unique:syntax_nodes", |raw| {
            let b = table(raw, "syntax_nodes");
            let dup = b.slice(0, 1);
            let fact = FixedSizeBinaryArray::try_from_iter(std::iter::once([8u8; 16])).unwrap();
            let dup = replace(&dup, "fact_id", Arc::new(fact));
            *b = arrow_select::concat::concat_batches(&b.schema(), [&*b, &dup]).unwrap();
        }),
        // C3: a resolution with no binding, no builtin and no reason.
        ("typed:reference_resolutions", |raw| {
            let b = table(raw, "reference_resolutions");
            let none: FixedSizeBinaryArray = FixedSizeBinaryArray::try_from_sparse_iter_with_size(
                std::iter::repeat_n(None::<[u8; 16]>, b.num_rows()),
                16,
            )
            .unwrap();
            let no_text = arrow_array::StringArray::from(vec![None::<&str>; b.num_rows()]);
            let no_reason = Int16Array::from(vec![None::<i16>; b.num_rows()]);
            *b = replace(b, "binding_id", Arc::new(none));
            *b = replace(b, "builtin_name", Arc::new(no_text));
            *b = replace(b, "reason", Arc::new(no_reason));
        }),
        // C3: a binding whose id is not its recipe.
        ("id:bindings", |raw| {
            let b = table(raw, "bindings");
            let names: arrow_array::StringArray = b
                .column(b.schema().index_of("name").unwrap())
                .as_any()
                .downcast_ref::<arrow_array::StringArray>()
                .unwrap()
                .iter()
                .map(|n| n.map(|n| format!("{n}_renamed")))
                .collect();
            *b = replace(b, "name", Arc::new(names));
        }),
        // C3 review F7: a scope whose id is not its owner's recipe.
        ("id:scopes", |raw| {
            let b = table(raw, "scopes");
            let module = b
                .column(b.schema().index_of("module_node_id").unwrap())
                .clone();
            *b = replace(b, "owner_node_id", module);
        }),
        // C3 review F7: a reference whose id is not its name's recipe.
        ("id:references", |raw| {
            let b = table(raw, "references");
            let names = b
                .column(b.schema().index_of("name_node_id").unwrap())
                .as_any()
                .downcast_ref::<FixedSizeBinaryArray>()
                .unwrap()
                .clone();
            let first = FixedSizeBinaryArray::try_from_iter(std::iter::repeat_n(
                names.value(0),
                b.num_rows(),
            ))
            .unwrap();
            *b = replace(b, "name_node_id", Arc::new(first));
        }),
        // C3 review F7: a Pysa identifier site with no reference at its span is our failure.
        ("typed:identifier_targets", |raw| {
            let b = table(raw, "references");
            for column in ["start_byte", "end_byte"] {
                let i = b.schema().index_of(column).unwrap();
                let moved: Int64Array = b
                    .column(i)
                    .as_any()
                    .downcast_ref::<Int64Array>()
                    .unwrap()
                    .iter()
                    .map(|v| v.map(|v| v + 1_000_000))
                    .collect();
                *b = replace(b, column, Arc::new(moved));
            }
        }),
        // C3 review F2 (P10): an import our own naming got wrong is never read as the provider's
        // "not found".
        ("typed:import_targets", |raw| {
            let b = table(raw, "export_syntax");
            let i = b.schema().index_of("resolved_module").unwrap();
            let moved: arrow_array::StringArray = b
                .column(i)
                .as_any()
                .downcast_ref::<arrow_array::StringArray>()
                .unwrap()
                .iter()
                .map(|m| m.map(|m| format!("{m}_misnamed")))
                .collect();
            *b = replace(b, "resolved_module", Arc::new(moved));
        }),
        // ADR-0014 lineage: a Pysa call record that matches no call site is not silently dropped.
        ("lineage:call_target", |raw| {
            let b = table(raw, "pysa_calls");
            for column in ["start_byte", "end_byte"] {
                let i = b.schema().index_of(column).unwrap();
                let moved: Int64Array = b
                    .column(i)
                    .as_any()
                    .downcast_ref::<Int64Array>()
                    .unwrap()
                    .iter()
                    .map(|v| v.map(|v| v + 1_000_000))
                    .collect();
                *b = replace(b, column, Arc::new(moved));
            }
        }),
        // ADR-0014 endpoint kinds: a declaration whose parent is a call site.
        ("endpoint:declares", |raw| {
            let call = {
                let c = table(raw, "call_syntax");
                let ids = c
                    .column(c.schema().index_of("node_id").unwrap())
                    .as_any()
                    .downcast_ref::<FixedSizeBinaryArray>()
                    .unwrap();
                ids.value(0).to_vec()
            };
            let b = table(raw, "declarations");
            let parents = FixedSizeBinaryArray::try_from_iter(std::iter::repeat_n(
                call.as_slice(),
                b.num_rows(),
            ))
            .unwrap();
            *b = replace(b, "parent_node_id", Arc::new(parents));
        }),
    ];
    for (rule, mutate) in cases {
        let root = tempfile::tempdir().unwrap();
        let mut raw = base.clone();
        mutate(&mut raw);
        match compile(root.path(), s, &raw).await {
            Err(CoreError::Invalid(violations)) => assert!(
                violations.iter().any(|v| v.rule.starts_with(rule)),
                "{rule}: {violations:?}"
            ),
            other => panic!("{rule}: expected a validation failure, got {other:?}"),
        }
        assert!(resolve(root.path(), s).await.unwrap().is_none(), "{rule}");
    }
    // The unmutated snapshot passes every rule.
    let root = tempfile::tempdir().unwrap();
    compile(root.path(), s, &base).await.unwrap();
}

/// An attempt validation rejected is inspected at its own commits, even after a later attempt
/// wrote every table again (H1 review F2): `attempt_versions` finds each table's commit carrying
/// the attempt's `lctx.snapshot_id`, and registering a table at another snapshot's commit is
/// refused, never read as empty.
#[tokio::test(flavor = "multi_thread")]
async fn a_rejected_attempt_is_inspected_at_its_own_commits() {
    let root = tempfile::tempdir().unwrap();
    let (a, b) = (Id([1; 16]), Id([2; 16]));
    let mut raw_a = raw("pysa_variants", a);
    drop_rows(&mut raw_a, "coverage", 1);
    assert!(matches!(
        compile(root.path(), a, &raw_a).await,
        Err(CoreError::Invalid(_))
    ));
    compile(root.path(), b, &raw("pysa_variants", b))
        .await
        .unwrap();
    let versions = cpg_core::snapshot::attempt_versions(root.path(), a)
        .await
        .unwrap();
    let ctx = cpg_core::snapshot::session(root.path(), a, &versions)
        .await
        .unwrap();
    let declarations = raw_a.iter().find(|(n, _)| *n == "declarations").unwrap();
    assert_eq!(
        count(&ctx, "SELECT count(*) FROM declarations").await,
        declarations.1.num_rows() as i64
    );
    let published = resolve(root.path(), b).await.unwrap().unwrap();
    let other = cpg_core::snapshot::empty_session();
    let err = cpg_core::snapshot::register(
        &other,
        root.path(),
        "declarations",
        published["declarations"],
        a,
    )
    .await
    .unwrap_err();
    assert!(matches!(err, CoreError::ForeignCommit { .. }), "{err}");
    assert!(matches!(
        cpg_core::snapshot::attempt_versions(root.path(), Id([9; 16])).await,
        Err(CoreError::NoAttempt(_))
    ));
}

/// Rules run concurrently, but violations come back in `rules()` order, so a report never depends
/// on which rule finished first (H1 P2).
#[tokio::test(flavor = "multi_thread")]
async fn violations_come_back_in_rule_order() {
    let s = Id([3; 16]);
    let mut raw = raw("pysa_variants", s);
    let b = table(&mut raw, "syntax_nodes");
    *b = b.slice(0, 0);
    drop_rows(&mut raw, "coverage", 1);
    let root = tempfile::tempdir().unwrap();
    let Err(CoreError::Invalid(violations)) = compile(root.path(), s, &raw).await else {
        panic!("expected violations");
    };
    assert!(violations.len() >= 3, "{violations:?}");
    let order: Vec<usize> = cpg_schema::rules::rules()
        .iter()
        .enumerate()
        .filter(|(_, r)| violations.iter().any(|v| v.rule == r.name))
        .map(|(i, _)| i)
        .collect();
    let reported: Vec<usize> = violations
        .iter()
        .map(|v| {
            cpg_schema::rules::rules()
                .iter()
                .position(|r| r.name == v.rule)
                .unwrap()
        })
        .collect();
    assert_eq!(reported, order);
}

/// Every table a snapshot registers is read by at least one rule (DataFusion probe #8): each rule's
/// logical plan is walked for the tables it scans, over the validation session's named in-memory
/// tables.
#[tokio::test(flavor = "multi_thread")]
async fn every_table_is_read_by_some_rule() {
    use datafusion::common::tree_node::TreeNodeRecursion;
    use datafusion::logical_expr::LogicalPlan;
    let s = Id([5; 16]);
    let root = tempfile::tempdir().unwrap();
    compile(root.path(), s, &raw("pysa_variants", s))
        .await
        .unwrap();
    let (versions, ctx) = published(root.path(), s).await.unwrap().unwrap();
    let cache = cpg_core::validate::cached_session(&ctx).await.unwrap();
    let mut read = std::collections::BTreeSet::new();
    for rule in cpg_schema::rules::rules() {
        let plan = sql::query(&cache, &rule.sql)
            .await
            .unwrap()
            .into_unoptimized_plan();
        plan.apply_with_subqueries(|p| {
            if let LogicalPlan::TableScan(scan) = p {
                read.insert(scan.table_name.table().to_owned());
            }
            Ok(TreeNodeRecursion::Continue)
        })
        .unwrap();
    }
    let unread: Vec<&String> = versions.keys().filter(|t| !read.contains(*t)).collect();
    assert!(unread.is_empty(), "tables no rule reads: {unread:?}");
}

/// Every hand-written rule is exercised by an injected violation or declared an edit guard, every
/// generated template by at least one case, and no case names a rule that no longer exists (C6
/// review F1). A case is a tuple opening with the rule's name in this crate's rule tests.
#[test]
fn every_rule_is_exercised_or_declared_an_edit_guard() {
    use std::collections::BTreeSet;
    const HAND_WRITTEN: &[&str] = &[
        "coverage",
        "semantic",
        "id",
        "partition",
        "placed",
        "unique",
        "contained",
        "support",
        "typed",
    ];
    const TEMPLATES: &[&str] = &[
        "key",
        "ref",
        "fact",
        "fact-payload",
        "codebook",
        "endpoint",
        "evidence",
        "one-per-evidence",
        "no-parallel",
        "lineage",
        "finite",
        "catalog-code",
        "catalog-unselected",
    ];
    let names: Vec<String> = cpg_schema::rules::rules()
        .into_iter()
        .map(|r| r.name)
        .collect();
    let kinds: BTreeSet<&str> = names.iter().map(|n| n.split_once(':').unwrap().0).collect();
    for kind in &kinds {
        assert!(
            HAND_WRITTEN.contains(kind) || TEMPLATES.contains(kind),
            "a new rule kind `{kind}`: say whether it is hand-written or generated"
        );
    }
    let mut cases = BTreeSet::new();
    for source in [
        include_str!("compile.rs"),
        include_str!("graph.rs"),
        include_str!("syntax.rs"),
        include_str!("analysis.rs"),
        include_str!("catalog.rs"),
    ] {
        for kind in &kinds {
            for (at, _) in source.match_indices(&format!("\"{kind}:")) {
                if source[..at].trim_end().ends_with('(')
                    || source[..at].trim_end().ends_with("v.rule ==")
                    || source[..at].trim_end().ends_with("r.name ==")
                {
                    let rest = &source[at + 1..];
                    cases.insert(rest[..rest.find('"').unwrap()].to_owned());
                }
            }
        }
    }
    let stale: Vec<_> = cases
        .iter()
        .filter(|c| !names.iter().any(|n| n.starts_with(c.as_str())))
        .collect();
    assert!(stale.is_empty(), "cases naming no rule: {stale:?}");
    let guards = cpg_schema::rules::EDIT_GUARDS;
    for guard in guards {
        assert!(names.iter().any(|n| n == guard), "guard {guard} is no rule");
    }
    let unexercised: Vec<_> = names
        .iter()
        .filter(|n| HAND_WRITTEN.contains(&n.split_once(':').unwrap().0))
        .filter(|n| !cases.contains(*n) && !guards.contains(&n.as_str()))
        .collect();
    assert!(unexercised.is_empty(), "no injected case: {unexercised:?}");
    for kind in TEMPLATES {
        assert!(
            cases.iter().any(|c| c.split_once(':').unwrap().0 == *kind),
            "no case exercises the `{kind}` template"
        );
    }
}

/// The C4 rules reject their violations on `type_shapes`, which has records and type variables
/// (C4 review F1, F3, and the `id:record_fields` case it asked for).
#[tokio::test(flavor = "multi_thread")]
async fn the_types_rules_reject_their_violations() {
    let s = Id([9; 16]);
    let base = raw("type_shapes", s);
    type Mutation = fn(&mut Vec<(&'static str, RecordBatch)>);
    let cases: [(&str, Mutation); 3] = [
        // A class reference nothing defines is our failure, never a catch-all reason.
        ("typed:type_class_targets", |raw| {
            let b = table(raw, "type_terms");
            let i = b.schema().index_of("class_key").unwrap();
            let moved: arrow_array::StringArray = b
                .column(i)
                .as_any()
                .downcast_ref::<arrow_array::StringArray>()
                .unwrap()
                .iter()
                .map(|k| k.map(|_| "999999"))
                .collect();
            *b = replace(b, "class_key", Arc::new(moved));
        }),
        // An anchor inside a release module that nothing holds is our failure too.
        ("typed:type_binders", |raw| {
            let b = table(raw, "type_terms");
            for column in ["anchor_start", "anchor_end"] {
                let i = b.schema().index_of(column).unwrap();
                let moved: Int64Array = b
                    .column(i)
                    .as_any()
                    .downcast_ref::<Int64Array>()
                    .unwrap()
                    .iter()
                    .map(|v| v.map(|v| v + 1_000_000))
                    .collect();
                *b = replace(b, column, Arc::new(moved));
            }
        }),
        ("id:record_fields", |raw| {
            let b = table(raw, "record_fields");
            let names: arrow_array::StringArray = b
                .column(b.schema().index_of("name").unwrap())
                .as_any()
                .downcast_ref::<arrow_array::StringArray>()
                .unwrap()
                .iter()
                .map(|n| n.map(|n| format!("{n}_renamed")))
                .collect();
            *b = replace(b, "name", Arc::new(names));
        }),
    ];
    for (rule, mutate) in cases {
        let root = tempfile::tempdir().unwrap();
        let mut raw = base.clone();
        mutate(&mut raw);
        match compile(root.path(), s, &raw).await {
            Err(CoreError::Invalid(violations)) => assert!(
                violations.iter().any(|v| v.rule.starts_with(rule)),
                "{rule}: {violations:?}"
            ),
            other => panic!("{rule}: expected a validation failure, got {other:?}"),
        }
    }
    let root = tempfile::tempdir().unwrap();
    compile(root.path(), s, &base).await.unwrap();
}

#[tokio::test(flavor = "multi_thread")]
async fn a_failed_snapshots_append_is_classified_by_rereading() {
    // ADR-0009 P3 in-repo: the append is rejected (a CHECK), and re-reading `snapshots` shows the
    // attempt unpublished. A published attempt reads back as published.
    let root = tempfile::tempdir().unwrap();
    let s = Id([5; 16]);
    compile(root.path(), s, &raw("unicode_bom", s))
        .await
        .unwrap();
    let bad = SnapshotsRow {
        snapshot_id: Id([6; 16]),
        content_digest: cpg_schema::id::Digest([0; 32]),
        table_name: "declarations".to_owned(),
        table_version: -1,
        schema_digest: cpg_schema::id::Digest([0; 32]),
        compiler_digest: cpg_schema::id::Digest([0; 32]),
        row_count: 0,
    };
    assert!(matches!(
        publish(root.path(), Id([6; 16]), &[bad]).await,
        Err(CoreError::Unpublished(_))
    ));
    assert!(resolve(root.path(), Id([6; 16])).await.unwrap().is_none());
    assert!(resolve(root.path(), s).await.unwrap().is_some());
    assert_eq!(Declarations::NAME, "declarations");
}

#[tokio::test]
async fn composed_argument_reads_keep_ordered_source_evidence() {
    let root = tempfile::tempdir().unwrap();
    let snapshot = Id([78; 16]);
    let analysis = Analysis {
        embedding_cache: None,
        config: AnalyticsConfig::parse(
            r#"
version = 1
[subsystem]
module_prefixes = ["exprpkg"]
public_roots = ["exprpkg"]
[seeds]
primary = ["exprpkg.selected_parameter"]
distractors = []
[pass_a]
max_depth = 2
max_vertices = 128
max_edges = 512
max_witnesses = 3
[briefs]
budget = 1
"#,
        )
        .unwrap(),
        embedder: Some(Arc::new(cpg_core::embedding_service::FakeEmbedder::new())),
        techniques: Techniques::default(),
    };
    compile_analyzed(
        root.path(),
        snapshot,
        &raw_version("expression_completion_shapes", snapshot, (3, 14, 7)),
        Some(&analysis),
    )
    .await
    .unwrap();
    let (_, ctx) = published(root.path(), snapshot).await.unwrap().unwrap();
    for name in [
        "literal_default_header",
        "keyword_default_header",
        "unexecuted_body_header",
        "typed_exception_finalizer",
        "superclass_exception_finalizer",
        "later_matching_finalizer",
        "reraised_caught_finalizer",
        "selected_parameter",
        "selected_builtin",
        "short_circuited",
        "nested_call_sibling",
        "nested_call_selected",
        "nested_call_predecessor",
        "normal_call_finalizer",
        "selected_finalizer",
        "literal_predecessor",
        "initialized_predecessor",
        "skipped_predecessor",
    ] {
        assert!(
            count(
                &ctx,
                &format!(
                    "SELECT count(*) FROM summary_flows s \
            JOIN declarations d ON d.node_id = s.function_node_id \
            WHERE d.name = '{name}' AND s.verdict IN (0,1)"
                )
            )
            .await
                > 0,
            "{name}"
        );
    }
    for name in ["handler_entry", "else_entry", "finalizer_entry"] {
        assert_eq!(count(&ctx,&format!("SELECT count(*) FROM return_entry_statuses e \
            JOIN declarations d ON d.node_id=e.function_node_id WHERE d.name='{name}' AND e.reason IS NULL")).await,1,"{name}");
    }
    for name in ["unreachable_handler_entry", "named_handler_entry"] {
        assert_eq!(count(&ctx,&format!("SELECT count(*) FROM return_entry_statuses e \
            JOIN declarations d ON d.node_id=e.function_node_id WHERE d.name='{name}' AND e.reason IS NULL")).await,0,"{name}");
    }
    assert_eq!(count(&ctx,&format!("SELECT count(*) FROM return_entry_statuses e \
        JOIN declarations d ON d.node_id=e.function_node_id WHERE d.name='named_handler_entry' AND e.reason={}",
        BoundaryReason::HandlerNameCleanup.code())).await,1);
    assert!(count(&ctx,&format!("SELECT count(*) FROM return_exit_statuses e \
        JOIN declarations d ON d.node_id=e.function_node_id WHERE d.name='named_exception_finalizer' AND e.reason={}",
        BoundaryReason::HandlerNameCleanup.code())).await>0);
    assert!(
        count(
            &ctx,
            "SELECT count(*) FROM statement_completions WHERE kind=2 AND exception=0"
        )
        .await
            > 0
    );
    assert!(
        count(
            &ctx,
            "SELECT count(*) FROM summary_origin_coverage WHERE complete"
        )
        .await
            > 0
    );
    assert!(
        count(
            &ctx,
            "SELECT count(*) FROM summary_origin_coverage WHERE NOT complete"
        )
        .await
            > 0
    );
    assert!(count(&ctx,"SELECT count(*) FROM summary_origin_coverage WHERE subject_kind=1 AND channel=4 AND complete AND witness_count=0 AND parameter_node_id IS NULL").await>0);
    assert_eq!(count(&ctx,"SELECT count(*) FROM summary_origin_coverage c JOIN declarations d ON d.node_id=c.function_node_id WHERE c.subject_kind=1 AND d.name IN ('deferred_completion_async','deferred_completion_generator')").await,0);
    assert!(
        count(
            &ctx,
            "SELECT count(*) FROM summary_flow_steps WHERE kind=22"
        )
        .await
            > 0
    );
    for name in [
        "missing_default_header",
        "decorated_header",
        "rebound_definition_header",
    ] {
        assert_eq!(count(&ctx,&format!("SELECT count(*) FROM summary_flows s JOIN declarations d ON d.node_id=s.function_node_id WHERE d.name='{name}'")).await,0,"{name}");
    }
    let original_coverage = sql::query(&ctx, "SELECT * FROM summary_origin_coverage")
        .await
        .unwrap()
        .into_view();
    let forged_coverage=sql::query(&ctx,"SELECT * EXCEPT (complete,reason), true AS complete, CAST(NULL AS SMALLINT) AS reason FROM summary_origin_coverage").await.unwrap().into_view();
    ctx.deregister_table("summary_origin_coverage").unwrap();
    ctx.register_table("summary_origin_coverage", forged_coverage)
        .unwrap();
    let violations = cpg_core::validate::validate(&ctx).await.unwrap();
    assert!(
        violations
            .iter()
            .any(|v| v.rule == "summary-origin-coverage-source-equality"),
        "{violations:?}"
    );
    ctx.deregister_table("summary_origin_coverage").unwrap();
    ctx.register_table("summary_origin_coverage", original_coverage)
        .unwrap();
    let original_completions = sql::query(&ctx, "SELECT * FROM statement_completions")
        .await
        .unwrap()
        .into_view();
    let missing_exceptions=sql::query(&ctx,"SELECT * EXCEPT (exception), CAST(NULL AS SMALLINT) AS exception FROM statement_completions").await.unwrap().into_view();
    ctx.deregister_table("statement_completions").unwrap();
    ctx.register_table("statement_completions", missing_exceptions)
        .unwrap();
    let violations = cpg_core::validate::validate(&ctx).await.unwrap();
    assert!(
        violations
            .iter()
            .any(|v| v.rule == "completion-source-equality"),
        "{violations:?}"
    );
    ctx.deregister_table("statement_completions").unwrap();
    ctx.register_table("statement_completions", original_completions)
        .unwrap();
    for name in [
        "nonmatching_exception_finalizer",
        "shadowed_exception_finalizer",
        "named_exception_finalizer",
        "grouped_exception_finalizer",
        "unresolved_first_handler",
        "selected_missing",
        "unknown_truthiness",
        "deleted_read",
        "nested_call_raising",
        "missing_required_predecessor",
        "extra_argument_predecessor",
        "extra_keyword_result",
        "raising_call_finalizer",
        "overriding_finalizer",
        "unknown_finalizer",
        "arithmetic_predecessor",
        "raising_selected_predecessor",
        "repeated_initialization",
    ] {
        assert_eq!(
            count(
                &ctx,
                &format!(
                    "SELECT count(*) FROM summary_flows s \
            JOIN declarations d ON d.node_id = s.function_node_id \
            WHERE d.name = '{name}'"
                )
            )
            .await,
            0,
            "{name}"
        );
    }
    for name in ["opaque_exception_finalizer", "rebinding_finalizer"] {
        assert_eq!(count(&ctx, &format!("SELECT count(*) FROM return_exit_statuses x \
            JOIN declarations d ON d.node_id = x.function_node_id WHERE d.name = '{name}' AND x.reason IS NULL")).await, 0,
            "implicit exception construction and object finalization require independent completion evidence");
    }
    assert_eq!(count(&ctx, "SELECT count(*) FROM return_exit_statuses x \
        JOIN declarations d ON d.node_id = x.function_node_id WHERE d.name = 'caught_primitive_finalizer' AND x.reason IS NULL").await, 1);
    assert_eq!(count(&ctx, &format!("SELECT count(*) FROM statement_completions s \
        JOIN syntax_nodes n ON n.fact_id=s.source_fact_id JOIN declarations d ON d.node_id=s.function_node_id \
        WHERE d.name='repeated_initialization' AND n.kind={} AND s.kind={}",
        cpg_schema::codebook::SyntaxKind::StmtAssign.code(), cpg_schema::codebook::CompletionKind::Normal.code())).await,0,
        "one syntactic assignment under a loop is not a first dynamic initialization");
    assert!(
        count(
            &ctx,
            "SELECT count(*) FROM expression_evaluation_steps e \
        JOIN syntax_nodes n ON n.fact_id = e.operand_fact_id \
        WHERE n.detail = 'typ' AND e.status IN (4,6)"
        )
        .await
            > 0
    );
    assert_eq!(
        count(
            &ctx,
            "SELECT count(*) FROM expression_evaluation_steps e \
        JOIN syntax_nodes n ON n.fact_id = e.operand_fact_id WHERE n.detail = 'missing'"
        )
        .await,
        0
    );
    let original_mro = sql::query(&ctx, "SELECT * FROM context_class_mro")
        .await
        .unwrap()
        .into_view();
    let incomplete_mro=sql::query(&ctx,"SELECT snapshot_id,fact_id,class_node_id,module_node_id,ordinal,ancestor_module,ancestor_key,ancestor_name,cyclic,false AS linearization_complete FROM context_class_mro").await.unwrap().into_view();
    ctx.deregister_table("context_class_mro").unwrap();
    ctx.register_table("context_class_mro", incomplete_mro)
        .unwrap();
    let violations = cpg_core::validate::validate(&ctx).await.unwrap();
    assert!(
        violations
            .iter()
            .any(|v| v.rule == "completion-source-equality"),
        "{violations:?}"
    );
    ctx.deregister_table("context_class_mro").unwrap();
    ctx.register_table("context_class_mro", original_mro)
        .unwrap();
    let original = sql::query(&ctx, "SELECT * FROM expression_evaluation_steps")
        .await
        .unwrap()
        .into_view();
    let missing = sql::query(
        &ctx,
        "SELECT * FROM expression_evaluation_steps WHERE false",
    )
    .await
    .unwrap()
    .into_view();
    ctx.deregister_table("expression_evaluation_steps").unwrap();
    ctx.register_table("expression_evaluation_steps", missing)
        .unwrap();
    let violations = cpg_core::validate::validate(&ctx).await.unwrap();
    assert!(
        violations
            .iter()
            .any(|v| v.rule == "expression-source-equality"),
        "{violations:?}"
    );
    ctx.deregister_table("expression_evaluation_steps").unwrap();
    ctx.register_table("expression_evaluation_steps", original)
        .unwrap();
    for table in [
        "statement_completion_steps",
        "return_exit_steps",
        "return_entry_statuses",
        "return_entry_steps",
    ] {
        let original = sql::query(&ctx, &format!("SELECT * FROM {table}"))
            .await
            .unwrap()
            .into_view();
        let missing = sql::query(&ctx, &format!("SELECT * FROM {table} WHERE false"))
            .await
            .unwrap()
            .into_view();
        ctx.deregister_table(table).unwrap();
        ctx.register_table(table, missing).unwrap();
        let violations = cpg_core::validate::validate(&ctx).await.unwrap();
        assert!(
            violations
                .iter()
                .any(|v| v.rule == "completion-source-equality"),
            "{table}: {violations:?}"
        );
        ctx.deregister_table(table).unwrap();
        ctx.register_table(table, original).unwrap();
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn call_execution_proves_reached_inputs_without_inventing_callee_completion() {
    use cpg_schema::call_execution::{
        CallExecutionSteps, CallExecutionStepsRow, CallExecutions, CallExecutionsRow,
    };
    let root = tempfile::tempdir().unwrap();
    let snapshot = Id([97; 16]);
    let analysis = Analysis {
        embedding_cache: None,
        config: AnalyticsConfig::parse(
            r#"
version = 1
[subsystem]
module_prefixes = ["invocations"]
public_roots = ["invocations"]
[seeds]
primary = ["invocations.before_raise"]
distractors = []
[pass_a]
max_depth = 2
max_vertices = 128
max_edges = 512
max_witnesses = 3
[briefs]
budget = 1
"#,
        )
        .unwrap(),
        embedder: Some(Arc::new(cpg_core::embedding_service::FakeEmbedder::new())),
        techniques: Techniques::default(),
    };
    compile_analyzed(
        root.path(),
        snapshot,
        &raw_version("invocation_shapes", snapshot, (3, 14, 7)),
        Some(&analysis),
    )
    .await
    .unwrap();
    let (_, ctx) = published(root.path(), snapshot).await.unwrap().unwrap();
    for (name, target) in [
        ("before_raise", "compress"),
        ("missing_defaults", "compress"),
        ("selected", "compress"),
        ("assigned", "decompress"),
        ("returned", "decompress"),
        ("register_only", "register"),
        ("after_total", "decompress"),
        ("after_fallible", "decompress"),
    ] {
        assert_eq!(count(&ctx,&format!("SELECT count(*) FROM call_executions e \
            JOIN declarations d ON d.node_id=e.function_node_id JOIN context_definitions t ON t.symbol_node_id=e.target_node_id \
            WHERE d.name='{name}' AND t.qualified_name='{target}' AND e.reason IS NULL")).await,1,
            "{name}: {}",sql::render(&ctx,"SELECT d.name,t.qualified_name,e.reason,e.prefix_count,e.invocation_count \
            FROM call_executions e JOIN declarations d ON d.node_id=e.function_node_id \
            JOIN context_definitions t ON t.symbol_node_id=e.target_node_id").await.unwrap());
    }
    for name in [
        "after_raise",
        "after_opaque",
        "raising_argument",
        "unasserted_defaults",
        "skipped",
        "nested_argument",
        "inner",
        "deferred",
        "generator",
    ] {
        assert_eq!(
            count(
                &ctx,
                &format!(
                    "SELECT count(*) FROM call_executions e JOIN declarations d \
            ON d.node_id=e.function_node_id WHERE d.name='{name}' AND e.reason IS NULL"
                )
            )
            .await,
            0,
            "{name}"
        );
    }
    assert_eq!(count(&ctx,&format!("SELECT count(*) FROM context_parameters p JOIN context_definitions d \
        ON d.symbol_node_id=p.symbol_node_id WHERE d.module_name='atexit' AND d.qualified_name='register' \
        AND p.form={} AND p.required IS NULL AND p.kind IN ({},{})",cpg_schema::codebook::SignatureForm::List.code(),
        cpg_schema::codebook::ParameterKind::VarPositional.code(),cpg_schema::codebook::ParameterKind::VarKeyword.code())).await,2,
        "empty variadic slots are not missing required ordinary arguments");
    assert_eq!(
        count(
            &ctx,
            "SELECT count(*) FROM modeled_callback_sites s JOIN declarations d \
        ON d.node_id=s.function_node_id WHERE d.name='register_only'"
        )
        .await,
        1
    );
    assert_eq!(count(&ctx,&format!("SELECT count(*) FROM call_executions e JOIN declarations d ON d.node_id=e.function_node_id \
        WHERE d.name='unasserted_defaults' AND e.reason={}",BoundaryReason::UnsupportedControlFlow.code())).await,1);
    assert_eq!(count(&ctx,&format!("SELECT count(*) FROM call_executions e JOIN declarations d ON d.node_id=e.function_node_id \
        WHERE d.name='oversized_arguments' AND e.argument_count=130 AND e.reason={}",BoundaryReason::InvocationArgumentLimit.code())).await,1);
    assert_eq!(count(&ctx,"SELECT count(*) FROM call_executions e JOIN declarations d ON d.node_id=e.function_node_id \
        JOIN context_definitions t ON t.symbol_node_id=e.target_node_id WHERE d.name='after_fallible' \
        AND t.qualified_name='compress' AND e.reason IS NULL").await,0);
    assert_eq!(
        count(
            &ctx,
            "SELECT count(*) FROM call_executions e JOIN expression_evaluations v \
        ON v.syntax_fact_id=e.syntax_fact_id JOIN declarations d ON d.node_id=e.function_node_id \
        WHERE d.name IN ('before_raise','assigned','returned','register_only') AND v.normal"
        )
        .await,
        0,
        "a reached invocation is not a normally completed expression"
    );
    assert_eq!(count(&ctx,"SELECT count(*) FROM summary_flows s JOIN declarations d ON d.node_id=s.function_node_id \
        WHERE d.name IN ('before_raise','assigned','returned','register_only')").await,0);
    let batches = sql::query(&ctx, "SELECT * FROM call_executions")
        .await
        .unwrap()
        .collect()
        .await
        .unwrap();
    let rows: Vec<CallExecutionsRow> = batches
        .iter()
        .flat_map(|batch| {
            <CallExecutionsRow as cpg_schema::query::QueryRow>::read_batch(
                &cpg_core::arrow_types::to_schema(batch, &CallExecutions::schema()).unwrap(),
            )
            .unwrap()
        })
        .collect();
    let batches = sql::query(
        &ctx,
        "SELECT * FROM call_execution_steps ORDER BY execution_id,ordinal",
    )
    .await
    .unwrap()
    .collect()
    .await
    .unwrap();
    let steps: Vec<CallExecutionStepsRow> = batches
        .iter()
        .flat_map(|batch| {
            <CallExecutionStepsRow as cpg_schema::query::QueryRow>::read_batch(
                &cpg_core::arrow_types::to_schema(batch, &CallExecutionSteps::schema()).unwrap(),
            )
            .unwrap()
        })
        .collect();
    for row in &rows {
        let proof: Vec<_> = steps
            .iter()
            .filter(|s| s.execution_id == row.execution_id)
            .map(|s| cpg_schema::id::recipe::SummaryFlowProofStep {
                kind: s.kind,
                evidence_id: s.evidence_id,
                condition_id: s.condition_id,
            })
            .collect();
        cpg_schema::call_execution::admit(row, &proof).unwrap();
        if row.reason.is_none() {
            let mut missing = proof.clone();
            missing.pop();
            assert!(cpg_schema::call_execution::admit(row, &missing).is_err());
            let mut foreign = proof.clone();
            foreign.last_mut().unwrap().evidence_id = Id::ZERO;
            assert!(cpg_schema::call_execution::admit(row, &foreign).is_err());
            let mut reordered = proof.clone();
            reordered.swap(0, 1);
            assert!(cpg_schema::call_execution::admit(row, &reordered).is_err());
        }
    }
    let repo = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let runtime = std::process::Command::new("uv")
        .args([
            "run",
            "--no-sync",
            "python",
            "docs/design_review/evidence/2026-09-27_call-entry/runtime_oracle.py",
        ])
        .current_dir(&repo)
        .output()
        .unwrap();
    assert!(
        runtime.status.success(),
        "{}",
        String::from_utf8_lossy(&runtime.stderr)
    );
    let runtime: serde_json::Value = serde_json::from_slice(&runtime.stdout).unwrap();
    assert_eq!(runtime["outcome"], "passed");
    for (name, target) in [
        ("before_raise", "compress"),
        ("missing_defaults", "compress"),
        ("selected", "compress"),
        ("assigned", "decompress"),
        ("returned", "decompress"),
        ("register_only", "register"),
        ("after_total", "decompress"),
        ("after_fallible", "decompress"),
    ] {
        assert!(
            runtime["cases"][name]["events"]
                .as_array()
                .unwrap()
                .iter()
                .any(|event| event == target),
            "{name}"
        );
    }
    for name in ["after_raise", "after_opaque", "raising_argument", "skipped"] {
        assert!(
            runtime["cases"][name]["events"]
                .as_array()
                .unwrap()
                .is_empty(),
            "{name}"
        );
    }
    assert_eq!(runtime["cases"]["assigned"]["outcome"], "BadGzipFile");
    assert_eq!(runtime["cases"]["register_only"]["callback_invoked"], false);
    let original = sql::query(&ctx, "SELECT * FROM call_execution_steps")
        .await
        .unwrap()
        .into_view();
    let missing = sql::query(&ctx, "SELECT * FROM call_execution_steps WHERE false")
        .await
        .unwrap()
        .into_view();
    ctx.deregister_table("call_execution_steps").unwrap();
    ctx.register_table("call_execution_steps", missing).unwrap();
    let violations = cpg_core::validate::validate(&ctx).await.unwrap();
    assert!(
        violations
            .iter()
            .any(|v| v.rule == "call-execution-source-equality"),
        "{violations:?}"
    );
    ctx.deregister_table("call_execution_steps").unwrap();
    ctx.register_table("call_execution_steps", original)
        .unwrap();
}

/// Contract probe over typed candidate inputs. These are not authored production models or
/// evidence that a validation call executes; source/Delta controls remain separate.
#[tokio::test]
async fn validation_schema_candidates_keep_attribution_separate_from_subjects() {
    use cpg_schema::behavior::{
        ModelApplications, ModelApplicationsRow, ModelArgumentBindings, ModelArgumentBindingsRow,
        ModeledEffectSites, ModeledEffectSitesRow,
    };
    use cpg_schema::codebook::{InvocationPhase, ModelSchemaKind};
    let id = |n| Id([n; 16]);
    let app = ModelApplicationsRow {
        snapshot_id: id(1),
        call_site_node_id: id(2),
        module_node_id: id(3),
        function_node_id: Some(id(4)),
        call_fact_id: id(5),
        pysa_fact_id: id(6),
        target_node_id: id(7),
        model_id: id(8),
        target_module_fact_id: id(9),
        target_definition_fact_id: id(10),
        revision: 1,
        target_modality: Modality::Definite,
        target_origin: Origin::SyntheticModel,
        phase: InvocationPhase::Call,
        candidate_set_complete_under_model: true,
        has_unresolved_remainder: false,
        target_count: 1,
        target_body_return_parameter: None,
        target_call_defaults_available: false,
        model_origin: Origin::SyntheticModel,
    };
    let model = ModelEffectsRow {
        snapshot_id: id(1),
        model_id: id(8),
        target_node_id: id(7),
        rule_id: id(11),
        target_definition_fact_id: id(10),
        revision: 1,
        effect: ModelEffectKind::Validate,
        exit: cpg_schema::codebook::ModelExit::Normal,
        argument: None,
        schema_kind: Some(ModelSchemaKind::RuntimeValue),
        schema_class_node_id: None,
        schema_class_fact_id: None,
        schema_path_id: Some(id(12)),
        schema_path_kind: Some(ModelPathKind::Parameter),
        subject_path_id: Some(id(13)),
        subject_path_kind: Some(ModelPathKind::Parameter),
        subject_path: Some("display only".into()),
        modality: Modality::Potential,
        origin: Origin::SyntheticModel,
    };
    let subject = ModelArgumentBindingsRow {
        snapshot_id: id(1),
        call_site_node_id: id(2),
        pysa_fact_id: id(6),
        model_id: id(8),
        target_node_id: id(7),
        rule_id: id(11),
        path_role: ModelPathRole::Input,
        path_id: id(13),
        formal_name: "value".into(),
        signature_count: 1,
        matched_signatures: 1,
        argument_node_id: Some(id(20)),
        argument_fact_id: Some(id(21)),
        status: ModelArgumentStatus::Bound,
        reason: None,
    };
    let schema = ModelArgumentBindingsRow {
        path_role: ModelPathRole::Schema,
        path_id: id(12),
        formal_name: "schema".into(),
        argument_node_id: Some(id(30)),
        argument_fact_id: Some(id(31)),
        ..subject.clone()
    };
    let shape = cpg_schema::rules::rules()
        .into_iter()
        .find(|r| r.name == "semantic:modeled-validation-schema-shape")
        .unwrap();
    let model_shape = cpg_schema::rules::rules()
        .into_iter()
        .find(|r| r.name == "semantic:model-validation-schema-shape")
        .unwrap();
    for case in 0..9 {
        let ctx = SessionContext::new();
        ctx.register_batch(
            "model_applications",
            ModelApplications::to_batch(std::slice::from_ref(&app)).unwrap(),
        )
        .unwrap();
        let mut model = model.clone();
        let mut schema = schema.clone();
        match case {
            1 => {
                model.schema_path_id = model.subject_path_id;
                schema.path_id = subject.path_id;
                schema.argument_node_id = subject.argument_node_id;
                schema.argument_fact_id = subject.argument_fact_id;
            }
            2 => model.schema_path_kind = Some(ModelPathKind::ReceiverField),
            3 => model.schema_path_kind = Some(ModelPathKind::Global),
            4 => {
                schema.status = ModelArgumentStatus::Unknown;
                schema.argument_node_id = None;
                schema.argument_fact_id = None;
                schema.reason = Some(BoundaryReason::UnsupportedUnpacking);
            }
            5 => schema.call_site_node_id = id(99), // A schema witness for another call cannot leak.
            6 => {
                model.schema_kind = Some(ModelSchemaKind::StaticClass);
                model.schema_path_id = None;
                model.schema_path_kind = None;
                model.schema_class_node_id = Some(id(40));
                model.schema_class_fact_id = Some(id(41));
            }
            7 => {
                model.schema_kind = Some(ModelSchemaKind::Unresolved);
                model.schema_path_id = None;
                model.schema_path_kind = None;
            }
            8 => {
                schema.status = ModelArgumentStatus::Unknown;
                schema.argument_node_id = None;
                schema.argument_fact_id = None;
                schema.reason = Some(BoundaryReason::AmbiguousBinding);
            }
            _ => {}
        }
        ctx.register_batch(
            "model_effects",
            ModelEffects::to_batch(&[model.clone()]).unwrap(),
        )
        .unwrap();
        ctx.register_batch(
            "model_argument_bindings",
            ModelArgumentBindings::to_batch(&[subject.clone(), schema]).unwrap(),
        )
        .unwrap();
        let rows: Vec<ModeledEffectSitesRow> = sql::fetch(
            &ctx,
            &cpg_schema::behavior::modeled_effect_sites(),
            sql::Params::new(),
        )
        .await
        .unwrap();
        let [row] = rows.as_slice() else {
            panic!("candidate fanout: {rows:?}")
        };
        assert_eq!(row.subject_expression_node_id, Some(id(20)));
        assert_eq!(row.subject_expression_fact_id, Some(id(21)));
        assert!(row.subject_reason.is_none());
        assert_eq!(row.schema_kind, model.schema_kind);
        if case <= 1 {
            assert_eq!(
                row.schema_expression_node_id,
                Some(id(if case == 0 { 30 } else { 20 }))
            );
            assert_eq!(
                row.schema_expression_fact_id,
                Some(id(if case == 0 { 31 } else { 21 }))
            );
            assert!(row.schema_reason.is_none());
        } else {
            assert!(
                row.schema_expression_node_id.is_none() && row.schema_expression_fact_id.is_none()
            );
            assert_eq!(
                row.schema_reason,
                match case {
                    4 => Some(BoundaryReason::UnsupportedUnpacking),
                    6 => None,
                    8 => Some(BoundaryReason::AmbiguousBinding),
                    _ => Some(BoundaryReason::OutsideProviderModel),
                }
            );
        }
        ctx.register_batch(
            "modeled_effect_sites",
            ModeledEffectSites::to_batch(&rows).unwrap(),
        )
        .unwrap();
        assert_eq!(
            count(&ctx, &format!("SELECT count(*) FROM ({})", shape.sql)).await,
            0
        );
        assert_eq!(
            count(&ctx, &format!("SELECT count(*) FROM ({})", model_shape.sql)).await,
            0
        );
        let mut malformed = row.clone();
        // Inventing a class identity alongside a runtime/unresolved schema (or breaking the
        // static node/fact pair) is rejected by the shared publication rule.
        malformed.schema_class_node_id = if case == 6 { None } else { Some(id(40)) };
        ctx.deregister_table("modeled_effect_sites").unwrap();
        ctx.register_batch(
            "modeled_effect_sites",
            ModeledEffectSites::to_batch(&[malformed]).unwrap(),
        )
        .unwrap();
        assert_eq!(
            count(&ctx, &format!("SELECT count(*) FROM ({})", shape.sql)).await,
            1
        );
        model.schema_class_node_id = if case == 6 { None } else { Some(id(40)) };
        ctx.deregister_table("model_effects").unwrap();
        ctx.register_batch("model_effects", ModelEffects::to_batch(&[model]).unwrap())
            .unwrap();
        assert_eq!(
            count(&ctx, &format!("SELECT count(*) FROM ({})", model_shape.sql)).await,
            1
        );
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn context_protocols_bind_class_and_constructor_roles_independently() {
    use cpg_schema::context_protocol::ModelContextProtocolsRow;
    use cpg_schema::query::QueryRow;
    use cpg_schema::tables::{
        ContextDefinitionsRow, ContextModulesRow, ContextParametersRow, ContextsRow,
    };
    let snapshot = Id([109; 16]);
    let inputs = raw_version("context_protocol_shapes", snapshot, (3, 14, 7));
    let root = tempfile::tempdir().unwrap();
    let analysis=Analysis {
        embedding_cache: None,config:AnalyticsConfig::parse(
        "version = 1\n[subsystem]\nmodule_prefixes = [\"contextpkg\"]\npublic_roots = [\"contextpkg\"]\n[seeds]\nprimary = [\"contextpkg.preserve\"]\ndistractors = []\n[pass_a]\nmax_depth = 2\nmax_vertices = 128\nmax_edges = 512\nmax_witnesses = 3\n[briefs]\nbudget = 1\n").unwrap(),
        embedder:Some(Arc::new(cpg_core::embedding_service::FakeEmbedder::new())),techniques:Techniques::default()};
    compile_analyzed(root.path(), snapshot, &inputs, Some(&analysis))
        .await
        .unwrap();
    let (_, ctx) = published(root.path(), snapshot).await.unwrap().unwrap();
    let rows = sql::query(&ctx, "SELECT * FROM model_context_protocols")
        .await
        .unwrap()
        .collect()
        .await
        .unwrap();
    let rows: Vec<ModelContextProtocolsRow> = rows
        .iter()
        .flat_map(|b| {
            ModelContextProtocolsRow::read_batch(
                &cpg_core::arrow_types::to_schema(b, &ModelContextProtocolsRow::schema()).unwrap(),
            )
            .unwrap()
        })
        .collect();
    assert_eq!(rows.len(), 2);
    assert!(rows.iter().all(cpg_schema::context_protocol::valid_shape));
    assert_eq!(count(&ctx,"SELECT count(*) FROM source_context_sites s JOIN declarations d ON d.node_id=s.function_node_id WHERE d.name IN ('preserve','entry_value','suppress_type_error','multiple','unsupported','deferred')").await,5,
        "{}",text(&ctx,"SELECT d.name,s.* FROM source_context_sites s JOIN declarations d ON d.node_id=s.function_node_id").await);
    for (name, kind) in [
        ("preserve", 1),
        ("suppress_type_error", 0),
        ("multiple", 0),
        ("unsupported", 5),
        ("deferred", 5),
        ("nonmatching", 2),
        ("assignment_failure_suppressed", 0),
        ("constructor_failure_suppressed", 0),
        ("replacement_suppressed", 0),
        ("replacement_preserved", 2),
        ("return_preserved", 1),
        ("matching_short_circuit", 0),
        ("invalid_first", 2),
        ("break_preserved", 3),
        ("continue_preserved", 4),
        ("unknown_body", 5),
        ("generator", 5),
    ] {
        assert_eq!(count(&ctx,&format!("SELECT count(*) FROM statement_completions c JOIN syntax_nodes n ON n.fact_id=c.source_fact_id JOIN declarations d ON d.node_id=c.function_node_id WHERE d.name='{name}' AND n.kind=13 AND c.kind={kind}")).await,1,
            "{name}: {}",text(&ctx,"SELECT d.name,c.kind,c.reason FROM statement_completions c JOIN syntax_nodes n ON n.fact_id=c.source_fact_id JOIN declarations d ON d.node_id=c.function_node_id WHERE n.kind=13").await);
    }

    assert_eq!(count(&ctx,"SELECT count(*) FROM source_context_value_identities").await,3,
        "{}",text(&ctx,"SELECT d.name,c.* FROM source_context_value_identities c JOIN declarations d ON d.node_id=c.function_node_id").await);
    assert_eq!(count(&ctx,"SELECT count(*) FROM source_context_value_identities i JOIN value_flow_contributions c ON c.origin_id=i.source_origin_id JOIN summary_flows f ON f.source_origin_id=i.source_origin_id WHERE NOT c.identity AND c.through_call AND f.kind=0").await,3,
        "raw transfer remains nonidentity; the new witness supplies modeled entry identity");
    assert_eq!(count(&ctx,"SELECT count(*) FROM summary_origin_coverage c JOIN source_context_value_identities i ON i.source_origin_id=c.subject_id WHERE c.complete").await,0);
    for name in [
        "entry_after_context",
        "entry_rebound",
        "entry_deleted",
        "entry_nested_mutation",
        "entry_none",
        "entry_suppress",
        "entry_sibling",
        "entry_overridden",
        "entry_opaque_predecessor",
    ] {
        assert_eq!(count(&ctx,&format!("SELECT count(*) FROM source_context_value_identities i JOIN declarations d ON d.node_id=i.function_node_id WHERE d.name='{name}'")).await,0,"{name}");
    }
    assert_eq!(count(&ctx,"SELECT count(*) FROM model_context_protocols p JOIN context_definitions c ON c.fact_id=p.class_fact_id JOIN context_definitions i ON i.fact_id=p.initialization_fact_id WHERE c.kind=1 AND c.signature_count IS NULL AND i.kind=0 AND c.symbol_node_id<>i.symbol_node_id").await,2);
    assert_eq!(
        count(
            &ctx,
            "SELECT count(*) FROM pysa_calls WHERE target_name LIKE '%__exit__%'"
        )
        .await,
        0,
        "a runtime class assertion does not manufacture missing provider exit calls"
    );
    let generation =
        cpg_core::bundle::bundle(root.path(), snapshot, &root.path().join("generations"))
            .await
            .unwrap();
    let script = r#"
import sys
from pathlib import Path
import pyarrow as pa
import pyarrow.ipc as ipc
from lctx_mcp.generation import NATIVE_IPC_FILES
sys.path[:0] = [str(Path("scripts").resolve()), str(Path("python/lctx_mcp/tests").resolve())]
from support import load_native as load, served_bundle
from lctx_semantics import SemanticExecutor
import json, subprocess
oracle=json.loads(subprocess.run([sys.executable,"docs/design_review/evidence/2026-09-27_sync-contexts/runtime_oracle.py"],capture_output=True,text=True,check=True,timeout=30).stdout)
observed={row["case"]:row for row in oracle["cases"]}
g=load(Path(sys.argv[1]),None)
for name in ("preserve","entry_value","suppress_type_error","multiple","assignment_failure_suppressed",
             "constructor_failure_suppressed","replacement_suppressed","return_preserved","matching_short_circuit"):
    paths,boundaries,total,truncated,work=g.condition_graph.inspect_value_paths("contextpkg."+name,"value","none","",True,0,20)
    assert observed[name]["outcome"]=={"kind":"return","value":42}
    assert observed[name]["identity_outcome"]=={"kind":"return","same_entry_value":True}
    assert paths and not truncated,(name,paths,boundaries)
    assert any(any(s[0]=="context_entry" for s in p[3]) for p in paths),(name,paths)
    assert any(any(s[0]=="context_exit" for s in p[3]) for p in paths),(name,paths)
for operation,formal in (("entry_value","value"),("entry_keyword","value"),("entry_other","other")):
    paths,_,_,truncated,_=g.condition_graph.inspect_value_paths("contextpkg."+operation,formal,"none","",True,0,20)
    assert paths and not truncated,(operation,paths)
    assert all(any(s[0]=="context_entry_value_identity" for s in p[3]) for p in paths)
    assert all(not any(s[0]=="raw_identity" for s in p[3]) for p in paths)
paths,*_=g.condition_graph.inspect_value_paths("contextpkg.entry_other","value","none","",True,0,20)
assert not paths,paths
for mutation in ("missing_site","missing_certificate","duplicate_certificate","missing_match",
                 "wrong_condition","missing_group","same_function_sites", "missing_context_value", "duplicate_context_value",
                 "omitted_value_basis", "foreign_context_value"):

    sites_by_function={}
    for row in g.tables["source_context_sites"].to_pylist():
        sites_by_function.setdefault(row["function_node_id"],[]).append(row["site_id"])
    twins=next(sites for sites in sites_by_function.values() if len(sites)>1)
    swapped={twins[0]:twins[1],twins[1]:twins[0]}
    files=[]
    for name in NATIVE_IPC_FILES:
        table=g.tables[name]
        rows=table.to_pylist()
        if name=="source_context_sites" and mutation=="missing_site":
            rows=rows[1:]
        if name=="return_completion_certificates" and mutation=="missing_certificate":
            rows=rows[1:]
        if name=="return_completion_certificates" and mutation=="duplicate_certificate":
            rows.append(rows[0].copy())
        if name=="source_context_value_identities":
            if mutation in ("missing_context_value","omitted_value_basis"): rows=[]
            if mutation=="duplicate_context_value": rows.append(rows[0].copy())
            if mutation=="foreign_context_value": rows[0]["context_site_id"]=twins[0]
        if name=="summary_flow_steps":
            if mutation=="omitted_value_basis":
                rows=[r for r in rows if r["kind"]!="context_entry_value_identity"]
            if mutation=="same_function_sites":
                for row in rows:
                    if row["kind"] in ("context_construction","context_entry","context_exit"):
                        row["evidence_id"]=swapped.get(row["evidence_id"],row["evidence_id"])
            if mutation=="missing_match":
                rows=[r for r in rows if r["kind"]!="context_exception_evidence"]
            if mutation=="missing_group":
                rows=[r for r in rows if not r["kind"].startswith("context_")]
            if mutation=="wrong_condition":
                for row in rows:
                    if row["kind"]=="context_entry": row["condition_id"]=b"\xff"*16
            counters={}
            for row in rows:
                key=row["summary_id"];row["ordinal"]=counters.get(key,0);counters[key]=row["ordinal"]+1
        altered=pa.Table.from_pylist(rows,schema=table.schema)
        sink=pa.BufferOutputStream()
        with ipc.new_file(sink,table.schema) as writer: writer.write_table(altered)
        files.append((name,sink.getvalue().to_pybytes()))
    try:
        SemanticExecutor.from_ipc(1,g.snapshot_id,g.manifest["entry_value_effect_digest"],files)
    except ValueError:
        pass
    else:
        raise AssertionError("accepted "+mutation)
"#;
    let output = std::process::Command::new("uv")
        .args(["run", "--no-sync", "python", "-c", script])
        .arg(&generation.dir)
        .current_dir(Path::new(env!("CARGO_MANIFEST_DIR")).join("../.."))
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let table = |name| inputs.iter().find(|(n, _)| *n == name).unwrap().1.clone();
    let mut contexts = ContextsRow::read_batch(&table("contexts")).unwrap();
    let mut modules = ContextModulesRow::read_batch(&table("context_modules")).unwrap();
    let mut definitions = ContextDefinitionsRow::read_batch(&table("context_definitions")).unwrap();
    let mut parameters = ContextParametersRow::read_batch(&table("context_parameters")).unwrap();
    let catalog = cpg_schema::models::Catalog::committed().unwrap();
    let bind = |contexts: &[_], modules: &[_], definitions: &[_], parameters: &[_]| {
        catalog.bind_context_protocols(snapshot, contexts, modules, definitions, parameters)
    };
    assert_eq!(
        bind(&contexts, &modules, &definitions, &parameters)
            .unwrap()
            .len(),
        2
    );
    let allocation = definitions
        .iter()
        .find(|d| d.qualified_name == "object.__new__")
        .unwrap()
        .clone();
    definitions.push(allocation.clone());
    assert!(
        bind(&contexts, &modules, &definitions, &parameters).is_err(),
        "ambiguous constructor role"
    );
    definitions.pop();
    definitions.retain(|d| d.fact_id != allocation.fact_id);
    assert!(
        bind(&contexts, &modules, &definitions, &parameters)
            .unwrap()
            .is_empty(),
        "missing independent allocation role"
    );
    definitions.push(allocation);
    let i = parameters
        .iter()
        .position(|p| p.name.as_deref() == Some("enter_result"))
        .unwrap();
    let parameter = parameters[i].clone();
    parameters[i].ordinal = Some(99);
    assert!(
        bind(&contexts, &modules, &definitions, &parameters).is_err(),
        "incomplete initializer signature"
    );
    parameters[i] = parameter;
    let context = contexts[0].clone();
    contexts[0].python_version = "3.14.6".into();
    assert!(
        bind(&contexts, &modules, &definitions, &parameters)
            .unwrap()
            .is_empty(),
        "wrong runtime pin"
    );
    contexts.push(context);
    assert!(
        bind(&contexts, &modules, &definitions, &parameters).is_err(),
        "mixed runtime pins"
    );
    contexts.remove(0);
    for module in &mut modules {
        module.origin = cpg_schema::codebook::ModuleOrigin::SitePackages;
    }
    assert!(
        bind(&contexts, &modules, &definitions, &parameters)
            .unwrap()
            .is_empty(),
        "wrong module authority"
    );
    let context_value_original = sql::query(&ctx, "SELECT * FROM source_context_value_identities")
        .await
        .unwrap()
        .into_view();
    let context_value_wrong=sql::query(&ctx,"SELECT * EXCLUDE (target_binding_fact_id), parameter_binding_fact_id AS target_binding_fact_id FROM source_context_value_identities").await.unwrap().into_view();
    ctx.deregister_table("source_context_value_identities")
        .unwrap();
    ctx.register_table("source_context_value_identities", context_value_wrong)
        .unwrap();
    let violations = cpg_core::validate::validate(&ctx).await.unwrap();
    assert!(
        violations
            .iter()
            .any(|v| v.rule == "source-context-value-identity-equality"),
        "{violations:?}"
    );
    ctx.deregister_table("source_context_value_identities")
        .unwrap();
    ctx.register_table("source_context_value_identities", context_value_original)
        .unwrap();
    let completion_original = sql::query(&ctx, "SELECT * FROM return_completion_certificates")
        .await
        .unwrap()
        .into_view();
    let completion_wrong=sql::query(&ctx,"SELECT * EXCLUDE (entry_digest), exit_digest AS entry_digest FROM return_completion_certificates").await.unwrap().into_view();
    ctx.deregister_table("return_completion_certificates")
        .unwrap();
    ctx.register_table("return_completion_certificates", completion_wrong)
        .unwrap();
    let violations = cpg_core::validate::validate(&ctx).await.unwrap();
    assert!(
        violations
            .iter()
            .any(|v| v.rule == "completion-certificate-source-equality"),
        "{violations:?}"
    );
    ctx.deregister_table("return_completion_certificates")
        .unwrap();
    ctx.register_table("return_completion_certificates", completion_original)
        .unwrap();
    let original = sql::query(&ctx, "SELECT * FROM model_context_protocols")
        .await
        .unwrap()
        .into_view();
    let wrong=sql::query(&ctx,"SELECT * EXCLUDE (class_fact_id), initialization_fact_id AS class_fact_id FROM model_context_protocols").await.unwrap().into_view();
    ctx.deregister_table("model_context_protocols").unwrap();
    ctx.register_table("model_context_protocols", wrong)
        .unwrap();
    let violations = cpg_core::validate::validate(&ctx).await.unwrap();
    assert!(
        violations
            .iter()
            .any(|v| v.rule == "model-context-protocol-equality"),
        "{violations:?}"
    );
    ctx.deregister_table("model_context_protocols").unwrap();
    ctx.register_table("model_context_protocols", original)
        .unwrap();
}

#[tokio::test(flavor = "multi_thread")]
async fn action_triggers_preserve_partial_io_and_withhold_unproved_outcomes() {
    let root = tempfile::tempdir().unwrap();
    let snapshot = Id([98; 16]);
    let analysis = Analysis {
        embedding_cache: None,
        config: AnalyticsConfig::parse(
            r#"
version = 1
[subsystem]
module_prefixes = ["actions"]
public_roots = ["actions"]
[seeds]
primary = ["actions.before_raise"]
distractors = []
[pass_a]
max_depth = 2
max_vertices = 128
max_edges = 512
max_witnesses = 3
[briefs]
budget = 1
"#,
        )
        .unwrap(),
        embedder: Some(Arc::new(cpg_core::embedding_service::FakeEmbedder::new())),
        techniques: Techniques::default(),
    };
    compile_analyzed(
        root.path(),
        snapshot,
        &raw_version("action_shapes", snapshot, (3, 14, 7)),
        Some(&analysis),
    )
    .await
    .unwrap();
    let (_, ctx) = published(root.path(), snapshot).await.unwrap().unwrap();
    let display=sql::render(&ctx,"SELECT d.name,a.channel,a.reason,e.effect FROM modeled_action_assessments a \
        LEFT JOIN declarations d ON d.node_id=a.function_node_id LEFT JOIN modeled_effect_sites e \
        ON e.call_site_node_id=a.call_site_node_id AND e.rule_id=a.rule_id AND e.pysa_fact_id=a.pysa_fact_id").await.unwrap();
    for name in ["normal", "before_raise", "missing_defaults"] {
        assert_eq!(count(&ctx,&format!("SELECT count(*) FROM modeled_action_assessments a \
            JOIN declarations d ON d.node_id=a.function_node_id JOIN modeled_effect_sites e \
            ON e.call_site_node_id=a.call_site_node_id AND e.rule_id=a.rule_id AND e.pysa_fact_id=a.pysa_fact_id \
            WHERE d.name='{name}' AND a.reason IS NULL AND e.effect={} AND e.model_modality={} AND e.exit={}",
            ModelEffectKind::IoWrite.code(),Modality::Potential.code(),cpg_schema::codebook::ModelExit::Invocation.code())).await,1,"{display}");
    }
    assert_eq!(count(&ctx,&format!("SELECT count(*) FROM modeled_action_assessments a JOIN modeled_effect_sites e \
        ON e.call_site_node_id=a.call_site_node_id AND e.rule_id=a.rule_id AND e.pysa_fact_id=a.pysa_fact_id \
        WHERE e.effect IN ({},{}) AND a.reason IS NULL",ModelEffectKind::Serialize.code(),ModelEffectKind::Compress.code())).await,0,
        "completed transform cannot borrow a reached invocation: {display}");
    for name in [
        "after_raise",
        "raising_argument",
        "after_opaque",
        "unasserted_defaults",
        "missing_required",
        "skipped",
        "compression",
        "registration",
        "acquisition",
    ] {
        assert_eq!(
            count(
                &ctx,
                &format!(
                    "SELECT count(*) FROM modeled_action_assessments a JOIN declarations d \
            ON d.node_id=a.function_node_id WHERE d.name='{name}' AND a.reason IS NULL"
                )
            )
            .await,
            0,
            "{display}"
        );
        if name != "skipped" {
            assert!(
                count(
                    &ctx,
                    &format!(
                        "SELECT count(*) FROM modeled_action_assessments a JOIN declarations d \
                ON d.node_id=a.function_node_id WHERE d.name='{name}' AND a.reason IS NOT NULL"
                    )
                )
                .await
                    > 0,
                "missing boundary {name}: {display}"
            );
        }
    }
    for (name, reason) in [
        ("registration", BoundaryReason::ActionTriggerUnavailable),
        ("acquisition", BoundaryReason::ResourceIdentityUnavailable),
        (
            "unasserted_defaults",
            BoundaryReason::UnsupportedControlFlow,
        ),
    ] {
        assert!(
            count(
                &ctx,
                &format!(
                    "SELECT count(*) FROM modeled_action_assessments a JOIN declarations d \
            ON d.node_id=a.function_node_id WHERE d.name='{name}' AND a.reason={}",
                    reason.code()
                )
            )
            .await
                > 0,
            "{display}"
        );
    }
    assert_eq!(
        count(&ctx, "SELECT count(*) FROM modeled_action_assessments").await,
        count(&ctx, "SELECT count(*) FROM modeled_effect_sites").await
            + count(&ctx, "SELECT count(*) FROM modeled_callback_sites").await
            + count(&ctx, "SELECT count(*) FROM modeled_resource_sites").await
    );
    assert_eq!(
        count(
            &ctx,
            "SELECT count(*) FROM modeled_action_assessments WHERE reason IS NULL"
        )
        .await,
        3,
        "{display}"
    );
    for (name, defaults) in [
        ("missing_defaults", 9),
        ("dumps_defaults", 9),
        ("loads_defaults", 6),
        ("compress_defaults", 2),
    ] {
        assert_eq!(count(&ctx,&format!("SELECT count(*) FROM call_executions e JOIN declarations d \
            ON d.node_id=e.function_node_id WHERE d.name='{name}' AND e.reason IS NULL AND e.default_formal_count={defaults}")).await,1,
            "{name}: {}",sql::render(&ctx,"SELECT d.name,e.reason,e.default_formal_count FROM call_executions e \
                JOIN declarations d ON d.node_id=e.function_node_id").await.unwrap());
        assert_eq!(count(&ctx,&format!("SELECT count(*) FROM call_executions e JOIN declarations d \
            ON d.node_id=e.function_node_id JOIN expression_evaluations v ON v.syntax_fact_id=e.syntax_fact_id \
            WHERE d.name='{name}' AND v.normal")).await,0,"availability is not normal return");
    }
    let original_apps = sql::query(&ctx, "SELECT * FROM model_applications")
        .await
        .unwrap()
        .into_view();
    let unasserted=sql::query(&ctx,"SELECT * EXCLUDE (target_call_defaults_available), false AS target_call_defaults_available \
        FROM model_applications").await.unwrap().into_view();
    ctx.deregister_table("model_applications").unwrap();
    ctx.register_table("model_applications", unasserted)
        .unwrap();
    let without_promise = cpg_core::summaries::execution(&ctx)
        .await
        .unwrap()
        .expressions;
    assert_eq!(
        without_promise
            .invocations
            .iter()
            .filter(|call| call.reason == Some(BoundaryReason::DefaultUnavailable))
            .count(),
        4,
        "all four otherwise bound omitted-default calls require their independent model premise"
    );
    for call in without_promise
        .invocations
        .iter()
        .filter(|call| call.reason == Some(BoundaryReason::DefaultUnavailable))
    {
        assert!(!call.default_formals.as_ref().unwrap().is_empty());
        assert!(call.proof.is_empty());
    }
    let violations = cpg_core::validate::validate(&ctx).await.unwrap();
    assert!(
        violations
            .iter()
            .any(|v| v.rule == "call-execution-source-equality"),
        "{violations:?}"
    );
    ctx.deregister_table("model_applications").unwrap();
    ctx.register_table("model_applications", original_apps)
        .unwrap();
    let repo = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let runtime = std::process::Command::new("uv")
        .args([
            "run",
            "--no-sync",
            "python",
            "docs/design_review/evidence/2026-09-27_action-triggers/runtime_oracle.py",
        ])
        .current_dir(&repo)
        .output()
        .unwrap();
    assert!(
        runtime.status.success(),
        "{}",
        String::from_utf8_lossy(&runtime.stderr)
    );
    let runtime: serde_json::Value = serde_json::from_slice(&runtime.stdout).unwrap();
    assert_eq!(runtime["outcome"], "passed");
    for name in ["normal", "before_raise", "partial_encoding"] {
        assert!(
            !runtime["cases"][name]["chunks"]
                .as_array()
                .unwrap()
                .is_empty()
        );
    }
    assert_eq!(runtime["cases"]["partial_encoding"]["outcome"], "TypeError");
    assert_eq!(
        runtime["cases"]["no_write_encoding"]["calls"],
        serde_json::json!(["dump"])
    );
    assert!(
        runtime["cases"]["no_write_encoding"]["chunks"]
            .as_array()
            .unwrap()
            .is_empty(),
        "reached invocation never implies an actual write"
    );
    for name in ["after_raise", "raising_argument", "after_opaque", "skipped"] {
        assert!(
            runtime["cases"][name]["calls"]
                .as_array()
                .unwrap()
                .is_empty()
        );
    }
    let defaults = std::process::Command::new("uv")
        .args([
            "run",
            "--no-sync",
            "python",
            "docs/design_review/evidence/2026-09-27_pinned-defaults/qualify.py",
        ])
        .current_dir(&repo)
        .output()
        .unwrap();
    assert!(
        defaults.status.success(),
        "{}",
        String::from_utf8_lossy(&defaults.stderr)
    );
    let defaults: serde_json::Value = serde_json::from_slice(&defaults.stdout).unwrap();
    assert_eq!(defaults["outcome"], "passed");
    for (source, runtime, target) in [
        ("missing_defaults", "dump_fallible", "dump"),
        ("dumps_defaults", "dumps_fallible", "dumps"),
        ("loads_defaults", "loads_fallible", "loads"),
        ("compress_defaults", "compress_defaults", "compress"),
    ] {
        assert_eq!(
            defaults["cases"][runtime]["starts"],
            serde_json::json!([target])
        );
        assert_eq!(
            count(
                &ctx,
                &format!(
                    "SELECT count(*) FROM call_executions e JOIN declarations d \
            ON d.node_id=e.function_node_id WHERE d.name='{source}' AND e.reason IS NULL"
                )
            )
            .await,
            1
        );
    }
    for name in ["missing_required", "raising_explicit"] {
        assert!(
            defaults["cases"][name]["starts"]
                .as_array()
                .unwrap()
                .is_empty()
        );
    }
    let postconditions=sql::render(&ctx,"SELECT d.name,p.reason,p.outcome_obligation,p.channel FROM modeled_action_postconditions p \
        JOIN declarations d ON d.node_id=p.function_node_id").await.unwrap();
    for name in [
        "normal",
        "before_raise",
        "missing_defaults",
        "dumps_defaults",
        "compression",
        "compress_defaults",
        "registration",
    ] {
        assert_eq!(count(&ctx,&format!("SELECT count(*) FROM modeled_action_postconditions p JOIN declarations d \
            ON d.node_id=p.function_node_id WHERE d.name='{name}' AND p.reason IS NULL AND p.outcome_obligation={}",
            cpg_schema::codebook::ModelExit::Normal.code())).await,1,"{postconditions}");
    }
    for name in [
        "after_raise",
        "raising_argument",
        "after_opaque",
        "missing_required",
        "acquisition",
        "acquisition_explicit",
    ] {
        assert_eq!(
            count(
                &ctx,
                &format!(
                    "SELECT count(*) FROM modeled_action_postconditions p JOIN declarations d \
            ON d.node_id=p.function_node_id WHERE d.name='{name}' AND p.reason IS NULL"
                )
            )
            .await,
            0,
            "{postconditions}"
        );
    }
    assert_eq!(
        count(
            &ctx,
            "SELECT count(*) FROM modeled_action_postconditions WHERE channel=5 AND reason IS NULL"
        )
        .await,
        1,
        "registration postcondition is separate from actual callback invocation"
    );
    assert_eq!(
        count(
            &ctx,
            &format!(
                "SELECT count(*) FROM modeled_action_postconditions p JOIN declarations d \
        ON d.node_id=p.function_node_id WHERE d.name='acquisition_explicit' AND p.reason={}",
                BoundaryReason::SummaryProofLimit.code()
            )
        )
        .await,
        1,
        "the pinned open overload evidence still obeys the original proof cap: {postconditions}"
    );
    let implications = std::process::Command::new("uv")
        .args([
            "run",
            "--no-sync",
            "python",
            "docs/design_review/evidence/2026-09-27_normal-postconditions/runtime_oracle.py",
        ])
        .current_dir(&repo)
        .output()
        .unwrap();
    assert!(
        implications.status.success(),
        "{}",
        String::from_utf8_lossy(&implications.stderr)
    );
    let implications: serde_json::Value = serde_json::from_slice(&implications.stdout).unwrap();
    assert_eq!(implications["outcome"], "passed");
    assert_eq!(
        implications["cases"]["register_normal"]["before_shutdown"],
        serde_json::json!([])
    );
    assert_eq!(
        implications["cases"]["register_raises"]["outcome"],
        "TypeError"
    );
    assert_eq!(
        implications["cases"]["serialize_raises"]["outcome"],
        "TypeError"
    );
    assert_eq!(
        implications["cases"]["acquire_raises"]["resource_result"],
        false
    );
    let original = sql::query(&ctx, "SELECT * FROM modeled_action_postconditions")
        .await
        .unwrap()
        .into_view();
    let erased = sql::query(
        &ctx,
        "SELECT * EXCLUDE (outcome_obligation), CAST(3 AS SMALLINT) AS outcome_obligation \
        FROM modeled_action_postconditions",
    )
    .await
    .unwrap()
    .into_view();
    ctx.deregister_table("modeled_action_postconditions")
        .unwrap();
    ctx.register_table("modeled_action_postconditions", erased)
        .unwrap();
    let violations = cpg_core::validate::validate(&ctx).await.unwrap();
    assert!(
        violations
            .iter()
            .any(|v| v.rule == "action-postcondition-source-equality"),
        "{violations:?}"
    );
    ctx.deregister_table("modeled_action_postconditions")
        .unwrap();
    ctx.register_table("modeled_action_postconditions", original)
        .unwrap();
    for table in [
        "modeled_action_assessments",
        "modeled_action_postconditions",
        "call_execution_steps",
    ] {
        let original = sql::query(&ctx, &format!("SELECT * FROM {table}"))
            .await
            .unwrap()
            .into_view();
        let missing = sql::query(&ctx, &format!("SELECT * FROM {table} WHERE false"))
            .await
            .unwrap()
            .into_view();
        ctx.deregister_table(table).unwrap();
        ctx.register_table(table, missing).unwrap();
        let violations = cpg_core::validate::validate(&ctx).await.unwrap();
        let expected = if table == "modeled_action_postconditions" {
            "action-postcondition-source-equality"
        } else {
            "action-source-equality"
        };
        assert!(
            violations.iter().any(|v| v.rule == expected),
            "{table}: {violations:?}"
        );
        ctx.deregister_table(table).unwrap();
        ctx.register_table(table, original).unwrap();
    }
}

#[tokio::test]
async fn source_body_outcomes_do_not_invent_value_flows_or_caller_continuation() {
    let root = tempfile::tempdir().unwrap();
    let snapshot = Id([85; 16]);
    let analysis = Analysis {
        embedding_cache: None,
        config: AnalyticsConfig::parse(
            r#"
version = 1
[subsystem]
module_prefixes = ["bodypkg"]
public_roots = ["bodypkg"]
[seeds]
primary = ["bodypkg.explicit_return"]
distractors = []
[pass_a]
max_depth = 2
max_vertices = 128
max_edges = 512
max_witnesses = 3
[briefs]
budget = 1
"#,
        )
        .unwrap(),
        embedder: Some(Arc::new(cpg_core::embedding_service::FakeEmbedder::new())),
        techniques: Techniques::default(),
    };
    compile_analyzed(
        root.path(),
        snapshot,
        &raw_version("source_body_shapes", snapshot, (3, 14, 7)),
        Some(&analysis),
    )
    .await
    .unwrap();
    let (_, ctx) = published(root.path(), snapshot).await.unwrap().unwrap();
    for (name, kind) in [
        ("fallthrough", 0),
        ("explicit_return", 1),
        ("first_initialization", 1),
        ("selected_literal", 1),
        ("preserved_return", 1),
        ("replaced_return", 1),
        ("replaced_raise", 2),
        ("explicit_raise", 2),
    ] {
        assert_eq!(
            count(
                &ctx,
                &format!(
                    "SELECT count(*) FROM source_body_completions b JOIN declarations d \
            ON d.node_id=b.function_node_id WHERE d.name='{name}' AND b.kind={kind} \
            AND b.reason IS NULL AND b.release_reason IS NULL AND b.function_retainer_required"
                )
            )
            .await,
            1,
            "{name}"
        );
        assert_eq!(
            count(
                &ctx,
                &format!(
                    "SELECT count(*) FROM summary_flows s JOIN declarations d \
            ON d.node_id=s.function_node_id WHERE d.name='{name}'"
                )
            )
            .await,
            0,
            "no invented value source: {name}"
        );
    }
    for name in ["replaced_raise", "explicit_raise"] {
        assert_eq!(count(&ctx,&format!("SELECT count(*) FROM source_body_completions b JOIN declarations d \
            ON d.node_id=b.function_node_id WHERE d.name='{name}' AND b.exception=0 AND b.terminal_fact_id IS NOT NULL")).await,1);
    }
    assert_eq!(count(&ctx,"SELECT count(*) FROM source_body_completions b JOIN declarations d \
        ON d.node_id=b.function_node_id WHERE d.name='docstring_only' AND b.kind=0 AND b.step_count=0 \
        AND b.release_count=0 AND b.runtime_statement_count=0 AND b.function_retainer_required").await,1);
    assert_eq!(count(&ctx,"SELECT count(*) FROM source_body_release_inputs r JOIN syntax_nodes e ON e.fact_id=r.syntax_fact_id \
        JOIN syntax_nodes s ON s.node_id=e.parent_node_id JOIN declarations d ON d.node_id=s.owner_node_id \
        WHERE d.name IN ('explicit_return','docstring_only') AND s.ordinal=0 AND s.kind=22").await,0);
    assert_eq!(count(&ctx,"SELECT count(*) FROM source_body_completions b JOIN declarations d \
        ON d.node_id=b.function_node_id WHERE d.name='proof_at_limit' AND b.kind=1 AND b.step_count=64 AND b.release_reason IS NULL").await,1);
    assert_eq!(count(&ctx,&format!("SELECT count(*) FROM source_body_completions b JOIN declarations d \
        ON d.node_id=b.function_node_id WHERE d.name='proof_over_limit' AND b.kind=5 AND b.reason={}",
        BoundaryReason::SummaryProofLimit.code())).await,1);
    // A normal local read has no closed-value provenance until its executed initialization is
    // linked. The known body outcome survives, while its release certificate stays open.
    assert_eq!(
        count(
            &ctx,
            &format!(
                "SELECT count(*) FROM source_body_completions b JOIN declarations d \
        ON d.node_id=b.function_node_id WHERE d.name='initialized_local_read' AND b.kind=1 \
        AND b.reason IS NULL AND b.release_reason={}",
                BoundaryReason::FrameExitCleanup.code()
            )
        )
        .await,
        1
    );
    for name in [
        "default_parameter",
        "required_parameter",
        "captured",
        "dynamic_definition",
        "deferred_async",
        "deferred_generator",
        "decorated",
    ] {
        assert_eq!(
            count(
                &ctx,
                &format!(
                    "SELECT count(*) FROM source_body_completions b JOIN declarations d \
            ON d.node_id=b.function_node_id WHERE d.name='{name}' AND b.kind=5 AND b.reason={}",
                    BoundaryReason::ScopeBoundary.code()
                )
            )
            .await,
            1,
            "{name}"
        );
    }
    for name in ["unknown_owned_value", "reassignment"] {
        assert_eq!(count(&ctx,&format!("SELECT count(*) FROM source_body_completions b JOIN declarations d \
            ON d.node_id=b.function_node_id WHERE d.name='{name}' AND b.kind=5 AND b.reason IS NOT NULL")).await,1,"{name}");
    }
    for (table, query) in [
        (
            "source_body_completions",
            "SELECT * EXCEPT(function_retainer_required), false AS function_retainer_required FROM source_body_completions",
        ),
        (
            "source_body_steps",
            "SELECT * FROM source_body_steps WHERE false",
        ),
        (
            "source_body_release_inputs",
            "SELECT * FROM source_body_release_inputs WHERE false",
        ),
    ] {
        let original = sql::query(&ctx, &format!("SELECT * FROM {table}"))
            .await
            .unwrap()
            .into_view();
        let changed = sql::query(&ctx, query).await.unwrap().into_view();
        ctx.deregister_table(table).unwrap();
        ctx.register_table(table, changed).unwrap();
        let violations = cpg_core::validate::validate(&ctx).await.unwrap();
        assert!(
            violations
                .iter()
                .any(|v| v.rule == "source-body-source-equality"),
            "{table}: {violations:?}"
        );
        ctx.deregister_table(table).unwrap();
        ctx.register_table(table, original).unwrap();
    }
}

#[tokio::test]
async fn fresh_source_calls_require_body_binding_and_release() {
    let root = tempfile::tempdir().unwrap();
    let snapshot = Id([85; 16]);
    let analysis = Analysis {
        embedding_cache: None,
        config: AnalyticsConfig::parse(
            r#"
version = 1
[subsystem]
module_prefixes = ["bodypkg"]
public_roots = ["bodypkg"]
[seeds]
primary = ["bodypkg.explicit_return"]
distractors = []
[pass_a]
max_depth = 2
max_vertices = 128
max_edges = 512
max_witnesses = 3
[briefs]
budget = 1
"#,
        )
        .unwrap(),
        embedder: Some(Arc::new(cpg_core::embedding_service::FakeEmbedder::new())),
        techniques: Techniques::default(),
    };
    compile_analyzed(
        root.path(),
        snapshot,
        &raw_version("source_body_shapes", snapshot, (3, 14, 7)),
        Some(&analysis),
    )
    .await
    .unwrap();
    let (_, ctx) = published(root.path(), snapshot).await.unwrap().unwrap();

    for name in [
        "call_literal",
        "call_fallthrough",
        "call_return_finally",
        "call_modeled",
    ] {
        let diagnostic=sql::query(&ctx,&format!("SELECT d.name, s.kind, e.normal, e.reason, e.status FROM expression_evaluations e \
            JOIN syntax_nodes s ON s.fact_id=e.syntax_fact_id JOIN declarations d ON d.node_id=s.owner_node_id \
            WHERE d.name='{name}'")).await.unwrap().collect().await.unwrap();
        assert_eq!(count(&ctx,&format!("SELECT count(*) FROM source_call_normals n JOIN source_call_bindings c ON c.binding_id=n.binding_id JOIN declarations d \
            ON d.node_id=c.function_node_id WHERE d.name='{name}'")).await,1,"{name}: {}",pretty_format_batches(&diagnostic).unwrap());
        assert!(
            count(
                &ctx,
                &format!(
                    "SELECT count(*) FROM summary_flows s JOIN declarations d \
            ON d.node_id=s.function_node_id WHERE d.name='{name}'"
                )
            )
            .await
                > 0,
            "{name}"
        );
        assert!(
            count(
                &ctx,
                &format!(
                    "SELECT count(*) FROM summary_flow_steps p JOIN summary_flows s \
            ON s.summary_id=p.summary_id JOIN declarations d ON d.node_id=s.function_node_id \
            WHERE d.name='{name}' AND p.kind={}",
                    cpg_schema::codebook::SummaryFlowStepKind::SourceCallNormal.code()
                )
            )
            .await
                > 0,
            "{name}"
        );
    }
    for name in [
        "call_raises",
        "call_local_read",
        "call_default",
        "call_captured",
        "call_intervening",
        "call_alias",
        "call_failed_header",
        "call_unknown_body",
        "call_generator",
        "call_unreachable_yield",
    ] {
        assert_eq!(count(&ctx,&format!("SELECT count(*) FROM source_call_normals n JOIN source_call_bindings c ON c.binding_id=n.binding_id JOIN declarations d \
            ON d.node_id=c.function_node_id WHERE d.name='{name}'")).await,0,"{name}");
        assert_eq!(
            count(
                &ctx,
                &format!(
                    "SELECT count(*) FROM summary_flows s JOIN declarations d \
            ON d.node_id=s.function_node_id WHERE d.name='{name}'"
                )
            )
            .await,
            0,
            "{name}"
        );
    }
    for name in [
        "call_literal",
        "call_fallthrough",
        "call_return_finally",
        "call_modeled",
        "call_raises",
        "call_local_read",
        "call_captured",
        "call_unknown_body",
    ] {
        assert_eq!(count(&ctx,&format!("SELECT count(*) FROM call_executions e JOIN declarations d ON d.node_id=e.function_node_id \
            WHERE d.name='{name}' AND e.target_kind={} AND e.model_id IS NULL AND e.source_binding_id IS NOT NULL AND e.reason IS NULL",
            cpg_schema::codebook::CallExecutionTarget::Source.code())).await,1,"source invocation {name}: {}",
            sql::render(&ctx,&format!("SELECT d.name,e.reason FROM call_executions e JOIN declarations d ON d.node_id=e.function_node_id WHERE d.name='{name}'")).await.unwrap());
    }
    for name in [
        "call_default",
        "call_intervening",
        "call_alias",
        "call_failed_header",
        "call_unreachable",
        "call_generator",
        "call_unreachable_yield",
    ] {
        assert_eq!(count(&ctx,&format!("SELECT count(*) FROM call_executions e JOIN declarations d ON d.node_id=e.function_node_id \
            WHERE d.name='{name}' AND e.target_kind={} AND e.reason IS NULL",cpg_schema::codebook::CallExecutionTarget::Source.code())).await,0,"withheld invocation {name}");
    }
    // Normality under entry to an expression never asserts the caller reached it.
    assert_eq!(count(&ctx,"SELECT count(*) FROM source_call_normals n JOIN source_call_bindings c ON c.binding_id=n.binding_id JOIN declarations d \
        ON d.node_id=c.function_node_id WHERE d.name='call_unreachable'").await,1);
    assert_eq!(
        count(
            &ctx,
            "SELECT count(*) FROM summary_flows s JOIN declarations d \
        ON d.node_id=s.function_node_id WHERE d.name='call_unreachable'"
        )
        .await,
        0
    );
    assert_eq!(count(&ctx,"SELECT count(*) FROM call_executions c JOIN declarations d ON d.node_id=c.function_node_id \
        WHERE d.name='call_prefix_small' AND c.model_id IS NOT NULL AND c.reason IS NULL").await,1);
    assert_eq!(count(&ctx,&format!("SELECT count(*) FROM call_executions c JOIN declarations d ON d.node_id=c.function_node_id \
        WHERE d.name='call_prefix_over_limit' AND c.reason={}",BoundaryReason::SummaryProofLimit.code())).await,1);
    assert_eq!(count(&ctx,"SELECT count(*) FROM modeled_action_assessments a JOIN declarations d ON d.node_id=a.function_node_id \
        WHERE d.name='call_prefix_over_limit' AND a.reason IS NULL").await,0);
    assert_eq!(
        count(
            &ctx,
            "SELECT count(*) FROM value_flows WHERE condition='false'"
        )
        .await,
        0
    );
    assert_eq!(
        count(
            &ctx,
            "SELECT count(*) FROM behaviors WHERE condition='false'"
        )
        .await,
        0
    );
    assert_eq!(count(&ctx,&format!("SELECT count(*) FROM behaviors b JOIN declarations d ON d.node_id=b.operation_node_id \
        WHERE d.name='call_unreachable' AND b.kind={}",cpg_schema::codebook::BehaviorKind::Delegates.code())).await,0);
    let generation = cpg_core::bundle::build(&ctx, &root.path().join("generations"))
        .await
        .unwrap();
    let output = std::process::Command::new("uv")
        .args([
            "run",
            "--no-sync",
            "python",
            "docs/design_review/evidence/2026-09-27_frame-exit/native_source_call.py",
            generation.dir.to_str().unwrap(),
        ])
        .current_dir(Path::new(env!("CARGO_MANIFEST_DIR")).join("../.."))
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    for table in [
        "source_call_bindings",
        "source_call_normals",
        "source_call_header_steps",
    ] {
        let original = sql::query(&ctx, &format!("SELECT * FROM {table}"))
            .await
            .unwrap()
            .into_view();
        let changed = sql::query(&ctx, &format!("SELECT * FROM {table} WHERE false"))
            .await
            .unwrap()
            .into_view();
        ctx.deregister_table(table).unwrap();
        ctx.register_table(table, changed).unwrap();
        let violations = cpg_core::validate::validate(&ctx).await.unwrap();
        assert!(
            violations
                .iter()
                .any(|v| v.rule == "source-call-source-equality"),
            "{table}: {violations:?}"
        );
        ctx.deregister_table(table).unwrap();
        ctx.register_table(table, original).unwrap();
    }
}
