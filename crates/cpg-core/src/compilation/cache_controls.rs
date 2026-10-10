//! Independent recomputation versus admitted shared-content attachment with real providers.
use super::*;
use crate::{
    artifact,
    workspace::{CheckedInputs, WorkspaceOptions},
};
use cpg_extract::{acquisition::AcquiredInput, bundle::CapturedInputs, capture::CapturedInput};
use futures::TryStreamExt;
use lctx_model::domain::{analysis::sources::SourceSnapshot, graph::Manifest};
use std::{collections::BTreeSet, io::Write, path::Path, sync::Mutex};
use tracing::instrument::WithSubscriber;

const ORIGINAL: &[u8] = b"def identity(value: int) -> int:\n    return value\n";
const CHANGED: &[u8] =
    b"def identity(value: int) -> int:\n    return value\n\ndef extra() -> int:\n    return 1\n";

#[derive(Clone, Default)]
struct CapturedLog(Arc<Mutex<Vec<u8>>>);
impl Write for CapturedLog {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        self.0.lock().unwrap().extend_from_slice(bytes);
        eprint!("{}", String::from_utf8_lossy(bytes));
        Ok(bytes.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}
struct Receipt {
    workspace: Arc<Workspace>,
    checked: CheckedInputs,
    sources: Vec<SourceSnapshot>,
    contracts: Vec<(String, ContentHash, Option<ContentHash>, ContentHash)>,
    manifest: Manifest,
    hit_log: String,
}
fn settings() -> AnalyticsConfiguration {
    AnalyticsConfiguration {
        module_prefixes: vec!["api".into()],
        public_roots: vec!["api".into()],
        configured_seeds: vec![],
        depth: 2,
        vertices: 512,
        arcs: 2048,
        witnesses: 3,
        brief_budget: 8,
        communities: false,
        pagerank: false,
        fca: false,
        knn: false,
        rca: false,
        type_layer: false,
        mention_layer: false,
        knn_layer: false,
    }
}
async fn rows<R: Record>(workspace: &Workspace) -> Vec<R> {
    let source = workspace.completed::<R>().unwrap();
    let mut stream = workspace
        .native()
        .scan_batches(
            source.view(),
            &Relation::of::<R>(),
            None,
            None,
            workspace.budget(),
            resources::TRANSFER_ROWS,
        )
        .await
        .unwrap();
    let mut rows = Vec::new();
    while let Some(batch) = stream.try_next().await.unwrap() {
        rows.extend(R::decode(&batch).unwrap());
    }
    rows
}
async fn independent_oracle(workspace: &Workspace, expected: &[u8], names: &[&str]) {
    let artifacts = rows::<source::SourceArtifact>(workspace).await;
    let source = artifacts
        .iter()
        .find(|source| source.path == "api.py")
        .expect("actual acquired Python source");
    assert_eq!(source.content, ContentHash::of(expected));
    assert_eq!(source.byte_len, expected.len() as i64);
    let mut chunks = rows::<lctx_model::domain::artifact::ArtifactChunk>(workspace)
        .await
        .into_iter()
        .filter(|chunk| chunk.artifact == source.id())
        .collect::<Vec<_>>();
    chunks.sort_by_key(|chunk| chunk.ordinal);
    assert_eq!(
        chunks
            .into_iter()
            .flat_map(|chunk| chunk.body.0)
            .collect::<Vec<_>>(),
        expected
    );
    let occurrences = rows::<source::Occurrence>(workspace)
        .await
        .into_iter()
        .filter(|occurrence| occurrence.source == source.id())
        .map(|occurrence| occurrence.id())
        .collect::<BTreeSet<_>>();
    let declarations = rows::<syntax::DeclarationObservation>(workspace)
        .await
        .into_iter()
        .filter(|declaration| {
            declaration.kind == syntax::DeclarationKind::Function
                && occurrences.contains(&declaration.declaration)
        })
        .map(|declaration| declaration.name)
        .collect::<BTreeSet<_>>();
    let observed = rows::<source::SyntaxObservation>(workspace)
        .await
        .into_iter()
        .filter(|syntax| declarations.contains(&syntax.occurrence))
        .map(|syntax| syntax.spelling)
        .collect::<BTreeSet<_>>();
    assert_eq!(
        observed,
        names.iter().map(|name| (*name).to_owned()).collect(),
        "independent source declarations must survive real extraction and native compilation"
    );
    assert!(
        !rows::<normalized::entities::CallableEntity>(workspace)
            .await
            .is_empty(),
        "this is a populated normalization, not an empty graph parity assertion"
    );
}
async fn run(
    config: &lctx_surrealdb::RuntimeConfig,
    input: &Path,
    profile: Profile,
    frontier: Frontier,
    reuse_admitted: bool,
    expected: &[u8],
    names: &[&str],
) -> Receipt {
    let log = CapturedLog::default();
    let writer = log.clone();
    let dispatch = tracing::Dispatch::new(
        tracing_subscriber::fmt()
            .with_ansi(false)
            .without_time()
            .with_env_filter("cpg_core::workspace::products=debug,lctx_phase=info")
            .with_writer(move || writer.clone())
            .finish(),
    );
    async {
        assert!(
            tracing::enabled!(target: "cpg_core::workspace::products", tracing::Level::DEBUG),
            "scoped replay observation must be enabled"
        );
        let setup = lctx_surrealdb::phase::Phase::begin("reuse_control_setup");
        let native = lctx_surrealdb::compiler::NativeCompilerStore::begin(config, frontier)
            .await
            .unwrap();
        let workspace = Workspace::new(
            Arc::new(model().unwrap()),
            WorkspaceOptions::default(),
            native,
        )
        .unwrap();
        workspace
            .set_admitted_product_reuse(reuse_admitted)
            .unwrap();
        let captured = Arc::new(CapturedInputs::new(
            vec![AcquiredInput::tree(
                CapturedInput::capture(input, &["api.py".into()], workspace.budget()).unwrap(),
                "reuse-control",
            )],
            cpg_extract::native_context::NativeContextConfig::committed(
                profile,
                workspace.budget(),
            )
            .unwrap(),
        ));
        let prepared = PreparedCompilation::new(
            frontier,
            settings(),
            captured.config().catalog(),
            None,
            workspace.budget(),
        )
        .unwrap();
        let configuration = ContentHash::of(b"integrated-compiler-reuse-control/v1");
        setup.finish(lctx_surrealdb::phase::Terminal::Passed);
        compile(
            &workspace,
            captured.clone(),
            profile,
            configuration,
            frontier,
            Some(&prepared),
            None,
            None,
        )
        .await
        .unwrap();
        drop(prepared);
        let oracle = lctx_surrealdb::phase::Phase::begin("reuse_control_independent_oracle");
        independent_oracle(&workspace, expected, names).await;
        oracle.finish(lctx_surrealdb::phase::Terminal::Passed);
        let admission = lctx_surrealdb::phase::Phase::begin("reuse_control_checked_admission");
        let checked = workspace.admit_semantics(profile).await;
        admission.finish_result(&checked);
        let checked = checked.unwrap();
        checked
            .require_subset(&workspace, checked.inputs())
            .unwrap();
        let admitted = artifact::admit(&workspace, &captured, frontier, profile, configuration)
            .await
            .unwrap();
        assert_eq!(admitted.manifest().profile, profile);
        assert_eq!(admitted.manifest().frontier, frontier);
        let manifest = admitted.manifest().clone();
        let completed = workspace.completed_relations().unwrap();
        let sources = completed.iter().map(|source| source.snapshot()).collect();
        let contracts = completed
            .iter()
            .map(|source| {
                (
                    source.name().to_owned(),
                    source.implementation(),
                    source.configuration(),
                    source.contract(),
                )
            })
            .collect();
        let hit_log = String::from_utf8(log.0.lock().unwrap().clone()).unwrap();
        drop(admitted);
        Receipt {
            workspace,
            checked,
            sources,
            contracts,
            manifest,
            hit_log,
        }
    }
    .with_subscriber(dispatch)
    .await
}
fn equivalent(expected: &Receipt, actual: &Receipt) {
    assert_eq!(
        actual.sources, expected.sources,
        "every completed source count/content/premise contract"
    );
    assert_eq!(
        actual.contracts, expected.contracts,
        "every producer schema/code/configuration contract"
    );
    assert_eq!(
        actual.manifest, expected.manifest,
        "all admitted graph rows, current outcomes and frontier metadata"
    );
    assert_eq!(actual.manifest.content(), expected.manifest.content());
    actual
        .checked
        .require_subset(&actual.workspace, actual.checked.inputs())
        .unwrap();
    assert!(
        expected
            .checked
            .require_subset(&actual.workspace, expected.checked.inputs())
            .is_err(),
        "equal values do not grant a previous physical attempt's checked authority"
    );
}
fn reused(receipt: &Receipt) -> bool {
    receipt
        .hit_log
        .contains("admitted compiled product attached")
}
async fn matrix(profile: Profile, frontier: Frontier) {
    let config = lctx_surrealdb::RuntimeConfig::read(Path::new(
        &std::env::var("LCTX_COMPILER_RUNTIME_CONFIG").expect("stable validation runtime"),
    ))
    .unwrap();
    let input = tempfile::tempdir().unwrap();
    std::fs::write(input.path().join("api.py"), ORIGINAL).unwrap();
    eprintln!("reuse matrix {}/{}: off", profile.name(), frontier.name());
    let off = run(
        &config,
        input.path(),
        profile,
        frontier,
        false,
        ORIGINAL,
        &["identity"],
    )
    .await;
    assert!(!reused(&off));
    eprintln!("reuse matrix {}/{}: cold", profile.name(), frontier.name());
    let cold = run(
        &config,
        input.path(),
        profile,
        frontier,
        false,
        ORIGINAL,
        &["identity"],
    )
    .await;
    equivalent(&off, &cold);
    assert!(
        !reused(&cold),
        "independent cold compilation must not attach retained work"
    );
    eprintln!("reuse matrix {}/{}: warm", profile.name(), frontier.name());
    let warm = run(
        &config,
        input.path(),
        profile,
        frontier,
        true,
        ORIGINAL,
        &["identity"],
    )
    .await;
    equivalent(&off, &warm);
    assert!(
        reused(&warm),
        "actual warm admitted attachment missing: {}",
        warm.hit_log
    );
    eprintln!(
        "reuse matrix {}/{}: reload",
        profile.name(),
        frontier.name()
    );
    let reload = run(
        &config,
        input.path(),
        profile,
        frontier,
        true,
        ORIGINAL,
        &["identity"],
    )
    .await;
    equivalent(&off, &reload);
    assert!(
        reused(&reload),
        "actual reopened-store admitted attachment missing: {}",
        reload.hit_log
    );
    std::fs::write(input.path().join("api.py"), CHANGED).unwrap();
    eprintln!(
        "reuse matrix {}/{}: changed",
        profile.name(),
        frontier.name()
    );
    let changed = run(
        &config,
        input.path(),
        profile,
        frontier,
        true,
        CHANGED,
        &["identity", "extra"],
    )
    .await;
    eprintln!(
        "reuse matrix {}/{}: changed_off",
        profile.name(),
        frontier.name()
    );
    let changed_off = run(
        &config,
        input.path(),
        profile,
        frontier,
        false,
        CHANGED,
        &["identity", "extra"],
    )
    .await;
    equivalent(&changed_off, &changed);
    assert_ne!(
        off.manifest.content(),
        changed.manifest.content(),
        "matching input insertion must change the final result"
    );
    // Removing the inserted declaration must also remove its facts and downstream results,
    // even while the expanded source's admitted products remain available in this service.
    std::fs::write(input.path().join("api.py"), ORIGINAL).unwrap();
    eprintln!(
        "reuse matrix {}/{}: deleted",
        profile.name(),
        frontier.name()
    );
    let deleted = run(
        &config,
        input.path(),
        profile,
        frontier,
        true,
        ORIGINAL,
        &["identity"],
    )
    .await;
    let deleted_off = run(
        &config,
        input.path(),
        profile,
        frontier,
        false,
        ORIGINAL,
        &["identity"],
    )
    .await;
    equivalent(&deleted_off, &deleted);
    equivalent(&off, &deleted);
    assert_ne!(
        changed.manifest.content(),
        deleted.manifest.content(),
        "deleted declarations must not survive through retained products"
    );
    for receipt in [
        off,
        cold,
        warm,
        reload,
        changed,
        changed_off,
        deleted,
        deleted_off,
    ] {
        receipt.workspace.native().abandon().await.unwrap();
    }
}
#[tokio::test(flavor = "multi_thread")]
async fn catalog_compiler_cache_off_cold_hit_reload_and_changed_source_are_equivalent() {
    matrix(Profile::Catalog, Frontier::Catalog).await;
}
#[tokio::test(flavor = "multi_thread")]
async fn behavioral_compiler_cache_off_cold_hit_reload_and_changed_source_are_equivalent() {
    matrix(Profile::Behavioral, Frontier::Analysis).await;
}
