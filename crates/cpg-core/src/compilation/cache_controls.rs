//! Real-provider recomputation versus retained native products copied into private compilation.
use super::*;
use crate::{
    artifact,
    workspace::{CheckedInputs, WorkspaceOptions},
};
use cpg_extract::{acquisition::AcquiredInput, bundle::CapturedInputs, capture::CapturedInput};
use futures::TryStreamExt;
use lctx_model::domain::{analysis::sources::SourceSnapshot, graph::Manifest};
use lctx_tracing::InContext;
use std::{collections::BTreeSet, io::Write, path::Path, sync::Mutex};

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
    native: Arc<lctx_compiler_store::compiler::NativeCompilerStore>,
    artifact: artifact::AdmittedArtifact,
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
async fn rows<R: Record>(
    workspace: &Workspace,
    native: &Arc<lctx_compiler_store::compiler::NativeCompilerStore>,
) -> Vec<R> {
    let source = workspace.completed::<R>().unwrap();
    let mut stream = native
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
async fn independent_oracle(
    workspace: &Workspace,
    native: &Arc<lctx_compiler_store::compiler::NativeCompilerStore>,
    expected: &[u8],
    names: &[&str],
) {
    let artifacts = rows::<source::SourceArtifact>(workspace, native).await;
    let source = artifacts
        .iter()
        .find(|source| source.path == "api.py")
        .expect("actual acquired Python source");
    assert_eq!(source.content, ContentHash::of(expected));
    assert_eq!(source.byte_len, expected.len() as i64);
    let mut chunks = rows::<lctx_model::domain::artifact::ArtifactChunk>(workspace, native)
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
    let occurrences = rows::<source::Occurrence>(workspace, native)
        .await
        .into_iter()
        .filter(|occurrence| occurrence.source == source.id())
        .map(|occurrence| occurrence.id())
        .collect::<BTreeSet<_>>();
    let declarations = rows::<syntax::DeclarationObservation>(workspace, native)
        .await
        .into_iter()
        .filter(|declaration| {
            declaration.kind == syntax::DeclarationKind::Function
                && occurrences.contains(&declaration.declaration)
        })
        .map(|declaration| declaration.name)
        .collect::<BTreeSet<_>>();
    let observed = rows::<source::SyntaxObservation>(workspace, native)
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
        !rows::<normalized::entities::CallableEntity>(workspace, native)
            .await
            .is_empty(),
        "this is a populated normalization, not an empty graph parity assertion"
    );
}
async fn run(
    config: &lctx_native::RuntimeConfig,
    input: &Path,
    profile: Profile,
    frontier: Frontier,
    reuse_admitted: bool,
    expected: &[u8],
    names: &[&str],
) -> Receipt {
    run_with_analytics(
        config,
        input,
        profile,
        frontier,
        reuse_admitted,
        expected,
        names,
        false,
    )
    .await
}
#[allow(
    clippy::too_many_arguments,
    reason = "Focused control keeps the compile selection, cache decision and independent source oracle explicit"
)]
async fn run_with_analytics(
    config: &lctx_native::RuntimeConfig,
    input: &Path,
    profile: Profile,
    frontier: Frontier,
    reuse_admitted: bool,
    expected: &[u8],
    names: &[&str],
    analytic: bool,
) -> Receipt {
    let log = CapturedLog::default();
    let writer = log.clone();
    let dispatch = tracing::Dispatch::new(
        tracing_subscriber::fmt()
            .with_ansi(false)
            .without_time()
            .with_env_filter(
                "cpg_core::workspace::products=debug,cpg_core::compilation=debug,lctx_phase=info",
            )
            .with_writer(move || writer.clone())
            .finish(),
    );
    async {
        assert!(
            tracing::enabled!(target: "cpg_core::workspace::products", tracing::Level::DEBUG),
            "scoped replay observation must be enabled"
        );
        let setup = lctx_native::phase::Phase::begin("reuse_control_setup");
        let budget = lctx_model::domain::resources::ResourceBudget::fixed(
            WorkspaceOptions::default().memory_bytes,
        )
        .unwrap();
        let factory = lctx_compiler_store::compiler::recovery_factory(config, budget.clone())
            .expect("owned native control factory");
        let workspace = Workspace::private_with_budget(
            Arc::new(model().unwrap()),
            WorkspaceOptions::default(),
            budget.clone(),
        )
        .unwrap();
        let cache = lctx_compiler_store::NativeProductCache::connect(config, &factory)
            .await
            .unwrap()
            .map(Arc::new);
        assert!(
            cache.is_some(),
            "this control requires the real configured retained-product owner"
        );
        workspace.set_product_cache(cache).unwrap();
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
        let mut settings = settings();
        settings.pagerank = analytic;
        let prepared = matches!(frontier, Frontier::Analysis | Frontier::Catalog)
            .then(|| {
                PreparedCompilation::new(
                    frontier,
                    settings,
                    captured.config().catalog(),
                    None,
                    workspace.budget(),
                )
                .map(std::sync::Arc::new)
            })
            .transpose()
            .unwrap();
        let configuration = ContentHash::of(b"integrated-compiler-reuse-control/v1");
        setup.finish(lctx_native::phase::Terminal::Passed);
        compile(
            &workspace,
            captured.clone(),
            profile,
            configuration,
            frontier,
            prepared.clone(),
            None,
            None,
        )
        .await
        .unwrap();
        drop(prepared);
        assert!(workspace.is_private());
        assert!(
            workspace.native().is_err(),
            "private compilation cannot allocate a native target to obtain a cache hit"
        );
        let admission = lctx_native::phase::Phase::begin("reuse_control_checked_admission");
        let checked = workspace.admit_semantics(profile).await;
        admission.finish_result(&checked);
        let checked = checked.unwrap();
        checked
            .require_subset(&workspace, checked.inputs())
            .unwrap();
        let native =
            lctx_compiler_store::compiler::NativeCompilerStore::begin(config, frontier, &factory)
                .await
                .unwrap();
        let admitted = artifact::materialize_and_admit(
            &workspace,
            &captured,
            frontier,
            profile,
            configuration,
            native.clone(),
        )
        .await
        .unwrap();
        assert!(
            workspace.native().is_err(),
            "final native output remains an explicitly separate owner"
        );
        let oracle = lctx_native::phase::Phase::begin("reuse_control_independent_oracle");
        independent_oracle(&workspace, &native, expected, names).await;
        oracle.finish(lctx_native::phase::Terminal::Passed);
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
        Receipt {
            workspace,
            native,
            artifact: admitted,
            checked,
            sources,
            contracts,
            manifest,
            hit_log,
        }
    }
    .with_tracing_dispatch(dispatch)
    .await
}
async fn finish(receipt: Receipt) {
    // The artifact retains the actual composite private/native execution owner.
    // Close accepted readers/cache work before abandoning the native target;
    // neither equal values nor a passing body establishes this finality.
    let mut terminal = receipt.artifact.drain_owned_report().await;
    if terminal.local == completion::LocalState::Terminal
        && terminal.remote == completion::RemoteState::Confirmed
        && terminal
            .failures
            .iter()
            .all(|failure| failure.error.permits_storage_cleanup())
    {
        terminal.merge(receipt.native.abandon_finality_report().await);
    } else {
        terminal
            .storage
            .push(completion::StorageState::Orphan(format!(
                "native_attempt:{}",
                receipt.native.attempt().hex(),
            )));
    }
    let signal = receipt.artifact.complete_signal(&terminal).await;
    terminal.step("private cache control composite signal", signal);
    completion::complete::<()>(Ok(()), terminal).unwrap();
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
            .inputs()
            .require_subset(&actual.workspace, actual.checked.inputs())
            .is_err(),
        "equal admitted content cannot attach a prior attempt's input ownership"
    );
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
        .lines()
        .any(|line| line.contains("compiler product disposition") && line.contains("hit=true"))
}
/// Observe current compact completions and each operation's absence of attachment,
/// independently of eligible package or normalization hits in the same compilation.
async fn compact_completion_identities(
    receipt: &Receipt,
) -> std::collections::BTreeMap<String, ContentHash> {
    use lctx_model::domain::{
        analysis::owner::AnalysisOwner, uniform_outcome::UniformNotRequested,
    };
    let expected = AnalysisOwner::ALL
        .into_iter()
        .filter(|owner| owner.can_be_wholly_unrequested())
        .map(|owner| owner.name())
        .collect::<BTreeSet<_>>();
    let contributions = receipt
        .native
        .contributions(receipt.workspace.budget())
        .await
        .unwrap();
    let mut identities = std::collections::BTreeMap::new();
    for owner in contributions
        .iter()
        .filter(|owner| expected.contains(owner.spec.producer.as_str()))
    {
        assert_eq!(
            owner.outcome,
            ProviderOutcome::Complete.code(),
            "physical compact production completed"
        );
        assert_eq!(
            owner
                .outputs
                .get(UniformNotRequested::NAME)
                .map(|output| output.rows),
            Some(1),
            "one actual input/context retains one compact descriptor"
        );
        assert!(
            owner
                .outputs
                .iter()
                .all(|(name, output)| name == UniformNotRequested::NAME || output.rows == 0),
            "compact ordinary outputs remain explicitly empty"
        );
        assert!(
            identities
                .insert(owner.spec.producer.clone(), owner.identity().unwrap())
                .is_none(),
            "one exact compact contribution per current owner"
        );
    }
    assert_eq!(
        identities
            .keys()
            .map(String::as_str)
            .collect::<BTreeSet<_>>(),
        expected
    );
    for owner in AnalysisOwner::ALL
        .into_iter()
        .filter(|owner| owner.can_be_wholly_unrequested())
    {
        let attached = receipt.hit_log.lines().any(|line| {
            line.contains("compiler product disposition")
                && line.contains("hit=true")
                && line.split_whitespace().any(|field| {
                    field
                        .strip_prefix("operation=")
                        .is_some_and(|name| name.trim_matches('"') == owner.name())
                })
        });
        assert!(
            !attached,
            "current unrequested compact metadata must be produced without attachment for {}: {}",
            owner.name(),
            receipt.hit_log
        );
    }
    identities
}
async fn matrix(profile: Profile, frontier: Frontier) {
    let config = lctx_native::RuntimeConfig::read(Path::new(
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
    if profile == Profile::Catalog {
        let original = compact_completion_identities(&off).await;
        assert_eq!(compact_completion_identities(&cold).await, original);
        assert_eq!(compact_completion_identities(&warm).await, original);
        assert_eq!(compact_completion_identities(&reload).await, original);
        let changed_owners = compact_completion_identities(&changed).await;
        assert_eq!(
            compact_completion_identities(&changed_off).await,
            changed_owners
        );
        for (name, identity) in &original {
            assert_ne!(
                changed_owners.get(name),
                Some(identity),
                "changed actual input/domain cannot inherit {}'s previous compact completion",
                name
            );
        }
        assert_eq!(compact_completion_identities(&deleted).await, original);
        assert_eq!(compact_completion_identities(&deleted_off).await, original);
    }
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
        finish(receipt).await;
    }
}
#[tokio::test(flavor = "multi_thread")]
async fn normalized_private_compiler_copies_a_real_retained_native_hit_with_fresh_ownership() {
    let config = lctx_native::RuntimeConfig::read(Path::new(
        &std::env::var("LCTX_COMPILER_RUNTIME_CONFIG")
            .expect("owned current-format validation runtime"),
    ))
    .unwrap();
    let input = tempfile::tempdir().unwrap();
    std::fs::write(input.path().join("api.py"), ORIGINAL).unwrap();
    let cold = run(
        &config,
        input.path(),
        Profile::Catalog,
        Frontier::Normalized,
        false,
        ORIGINAL,
        &["identity"],
    )
    .await;
    assert!(!reused(&cold));
    let (retained, retained_charge) = cold
        .native
        .retained_product_views(cold.workspace.budget())
        .await
        .unwrap();
    assert!(
        !retained.is_empty(),
        "actual materialized native products, rather than a retention marker, must be available"
    );
    drop((retained, retained_charge));
    let warm = run(
        &config,
        input.path(),
        Profile::Catalog,
        Frontier::Normalized,
        true,
        ORIGINAL,
        &["identity"],
    )
    .await;
    assert!(
        reused(&warm),
        "genuine native retained lookup did not produce a hit: {}",
        warm.hit_log
    );
    equivalent(&cold, &warm);
    for receipt in [cold, warm] {
        assert!(receipt.workspace.is_private());
        assert!(
            receipt
                .workspace
                .completed_relations()
                .unwrap()
                .iter()
                .all(|source| source.private_source().is_ok()),
            "every completed reader owns validated private content after the retained source closes"
        );
        assert!(
            receipt
                .native
                .recovery_budget()
                .unwrap()
                .unwrap()
                .shares_pool(receipt.workspace.budget()),
            "actual native output and private computation must share one admitted resource pool"
        );
        finish(receipt).await;
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

#[tokio::test(flavor = "multi_thread")]
async fn optional_analytic_preparation_cold_and_warm_preserve_exact_outputs() {
    let config = lctx_native::RuntimeConfig::read(Path::new(
        &std::env::var("LCTX_COMPILER_RUNTIME_CONFIG").expect("stable validation runtime"),
    ))
    .unwrap();
    let input = tempfile::tempdir().unwrap();
    std::fs::write(input.path().join("api.py"), ORIGINAL).unwrap();
    // Two attempts, rather than the full invalidation matrix: selected optional Analytic
    // extends graph lifetime after Structural, and warm attachment can remove consumers.
    let cold = run_with_analytics(
        &config,
        input.path(),
        Profile::Catalog,
        Frontier::Analysis,
        false,
        ORIGINAL,
        &["identity"],
        true,
    )
    .await;
    let warm = run_with_analytics(
        &config,
        input.path(),
        Profile::Catalog,
        Frontier::Analysis,
        true,
        ORIGINAL,
        &["identity"],
        true,
    )
    .await;
    assert!(!reused(&cold));
    assert!(
        reused(&warm),
        "actual warm admitted attachment missing: {}",
        warm.hit_log
    );
    equivalent(&cold, &warm);
    for receipt in [cold, warm] {
        finish(receipt).await;
    }
}

#[test]
fn known_uniform_metadata_is_fresh_without_changing_requested_rows_eligibility() {
    let budget = resources::ResourceBudget::fixed(16 << 20).unwrap();
    let catalog = models::Catalog::parse(
        "external.toml",
        include_str!("../../../lctx-model/models/external.toml"),
    )
    .unwrap();
    let prepared =
        PreparedCompilation::new(Frontier::Catalog, settings(), &catalog, None, &budget).unwrap();
    for owner in [
        UpperStage::Local,
        UpperStage::Base,
        UpperStage::Completion,
        UpperStage::SourceCalls,
        UpperStage::Enriched,
        UpperStage::Models,
        UpperStage::Summary,
        UpperStage::Embedding,
        UpperStage::Analytic,
    ] {
        assert!(uniform::unrequested(owner, Profile::Catalog, &prepared));
        assert!(
            matches!(
                owner.reuse(Effect::Pure, Profile::Catalog, &prepared),
                reuse::Eligibility::Fresh(_)
            ),
            "actual unrequested dispatch must not perform a reuse lookup for {owner:?}"
        );
    }
    // No output cardinality enters this decision. Requested owners keep their existing
    // Rows policy even if their actual computation later produces an empty relation.
    for profile in [Profile::Catalog, Profile::Behavioral] {
        for owner in [
            UpperStage::Text,
            UpperStage::CatalogCore,
            UpperStage::CatalogEvidence,
            UpperStage::Structural,
            UpperStage::Selection,
            UpperStage::Synthesis,
            UpperStage::Retrieval,
        ] {
            assert!(!uniform::unrequested(owner, profile, &prepared));
            assert!(matches!(
                owner.reuse(Effect::Pure, profile, &prepared),
                reuse::Eligibility::Rows
            ));
        }
    }
    for owner in [
        UpperStage::Enriched,
        UpperStage::Models,
        UpperStage::Summary,
    ] {
        assert!(!uniform::unrequested(owner, Profile::Behavioral, &prepared));
        assert!(matches!(
            owner.reuse(Effect::Pure, Profile::Behavioral, &prepared),
            reuse::Eligibility::Rows
        ));
    }
    for owner in [
        UpperStage::Local,
        UpperStage::Base,
        UpperStage::Completion,
        UpperStage::SourceCalls,
    ] {
        assert!(!uniform::unrequested(owner, Profile::Behavioral, &prepared));
        assert!(
            matches!(
                owner.reuse(Effect::Pure, Profile::Behavioral, &prepared),
                reuse::Eligibility::Fresh(_)
            ),
            "the existing private-kernel Fresh policy remains unchanged"
        );
    }
    let mut requested = settings();
    requested.pagerank = true;
    let prepared =
        PreparedCompilation::new(Frontier::Catalog, requested, &catalog, None, &budget).unwrap();
    assert!(!uniform::unrequested(
        UpperStage::Analytic,
        Profile::Catalog,
        &prepared
    ));
    assert!(matches!(
        UpperStage::Analytic.reuse(Effect::Pure, Profile::Catalog, &prepared),
        reuse::Eligibility::Rows
    ));
}
