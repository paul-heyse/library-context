//! Slice 2: a compile attempt writes the raw tables, derives Stage C/D, validates and publishes
//! (DESIGN §4.1, §6, §8; ADR-0009).

use std::path::Path;
use std::sync::Arc;

use arrow_array::{Array, ArrayRef, FixedSizeBinaryArray, Int16Array, Int64Array, RecordBatch};
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
    Codebook, DefinitionKind, ExitSiteKind, FlowCallLinkStatus, FlowCallOperandRole, FlowSink,
    HandlerTypeStatus, Modality, ModelArgumentStatus, ModelCallbackAction, ModelChannelCoverage,
    ModelEffectKind, ModelEffectSubjectStatus, ModelExceptionAction, ModelExit, ModelPathKind,
    ModelPathRole, ModelResourceAction, ModelResourceSourceStatus, ModelTransferEndpointStatus,
    ModelTransferKind, ModeledHandlerClassMatch, Origin,
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
    assert_eq!(
        versions.len(),
        54 + 22 + 54,
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
    compile(root.path(), snapshot, &raw("flow_shapes", snapshot))
        .await
        .unwrap();
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
    assert!(cpg_core::validate::validate(&ctx).await.unwrap().is_empty());
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
}

#[tokio::test(flavor = "multi_thread")]
async fn handler_type_status_is_pinned_or_explicitly_unknown() {
    let root = tempfile::tempdir().unwrap();
    let snapshot = Id([56; 16]);
    compile(root.path(), snapshot, &raw("handler_shapes", snapshot))
        .await
        .unwrap();
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
        embedder: Some(Arc::new(cpg_core::embed::FakeEmbedder::new())),
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
    let dropped_return_sites = sql::query(&ctx, "SELECT * FROM handler_return_none_sites WHERE false")
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
    let dropped_paths = sql::query(&ctx, "SELECT * FROM modeled_exception_return_none_paths WHERE false")
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
    let dropped_contributions = sql::query(&ctx, "SELECT * FROM value_flow_contributions WHERE false")
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
}

#[tokio::test(flavor = "multi_thread")]
async fn unresolved_frames_withhold_raise_escape_guards() {
    let root = tempfile::tempdir().unwrap();
    let snapshot = Id([57; 16]);
    let analysis = Analysis {
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
        embedder: Some(Arc::new(cpg_core::embed::FakeEmbedder::new())),
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
    compile(root.path(), snapshot, &raw("flow_shapes", snapshot))
        .await
        .unwrap();
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
    compile(root.path(), snapshot, &raw("flow_shapes", snapshot))
        .await
        .unwrap();
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
        transfer_coverage: ModelChannelCoverage::Complete,
        effect_coverage: ModelChannelCoverage::Complete,
        callback_coverage: ModelChannelCoverage::Complete,
        resource_coverage: ModelChannelCoverage::Complete,
        exception_coverage: ModelChannelCoverage::Complete,
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
        argument: None,
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
        embedder: Some(Arc::new(cpg_core::embed::FakeEmbedder::new())),
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
    assert_eq!(targets, 7, "cast, assert_type, print, json, open and atexit.register");
    assert!(
        count(&ctx, "SELECT count(*) FROM flow_value_calls").await > 0,
        "source value uses inside calls retain their ordered raw call steps"
    );
    assert_eq!(count(&ctx, "SELECT count(*) FROM model_transfers").await, 4);
    assert_eq!(
        count(&ctx, "SELECT count(*) FROM modeled_transfer_sites").await,
        7,
        "the pure, JSON and atexit transfers apply to resolved source calls"
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
    assert_eq!(count(&ctx, "SELECT count(*) FROM model_effects").await, 4);
    assert_eq!(
        count(&ctx, "SELECT count(*) FROM modeled_effect_sites").await,
        4,
        "print and JSON effects apply to their resolved source calls"
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
        8,
        "pure, JSON and atexit paths use typed model ASTs"
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
        10,
        "each pinned model applies only at its resolved source call"
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
        "SELECT * EXCLUDE (class_node_id), target_node_id AS class_node_id \
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
        embedder: Some(std::sync::Arc::new(cpg_core::embed::FakeEmbedder::new())),
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
    let proof = &origins[0];
    let guard = &diagrams[&proof.test_condition_id];
    assert!(
        cpg_core::primitive_theory::refute_exact_input(
            guard,
            proof.operation_node_id,
            proof.formal_node_id,
            &Value::None,
            cpg_core::primitive_theory::BuiltinNamespace::StandardAssumed,
            &links,
            &leaves,
        )
        .unwrap()
        .is_some()
    );
    assert!(
        cpg_core::primitive_theory::refute_exact_input(
            guard,
            proof.operation_node_id,
            proof.formal_node_id,
            &Value::Str("hi".to_owned()),
            cpg_core::primitive_theory::BuiltinNamespace::StandardAssumed,
            &links,
            &leaves,
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
        embedder: Some(std::sync::Arc::new(cpg_core::embed::FakeEmbedder::new())),
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
    ] {
        for kind in &kinds {
            for (at, _) in source.match_indices(&format!("\"{kind}:")) {
                if source[..at].trim_end().ends_with('(') {
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
