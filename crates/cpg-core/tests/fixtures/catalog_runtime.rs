#![allow(
    dead_code,
    reason = "Shared fixture runner serves focused compiler controls"
)]
//! Fixture inputs run through the actual persisted compiler; assertions inspect completed native views.
use cpg_core::{
    compilation::{self, PreparedCompilation},
    workspace::{Workspace, WorkspaceOptions},
};
use cpg_extract::{
    acquisition::{AcquiredInput, derive_blocks},
    bundle::CapturedInputs,
    capture::CapturedInput,
};
use datafusion::{common::ScalarValue, prelude::SessionContext};
use futures::FutureExt;
use lctx_model::domain::embedding::Embedder;
use lctx_model::domain::{
    admission::Frontier, analysis::settings::AnalyticsConfiguration, stages::Profile, *,
};
use std::{path::Path, sync::Arc};

pub struct Fixture {
    pub workspace: Arc<Workspace>,
    pub session: SessionContext,
}
impl Fixture {
    /// Release inspection borrowers before draining the original compiler attempt.
    /// Assertion panics keep their original payload even if finalization also fails.
    pub async fn complete(self, result: std::thread::Result<()>) {
        let Self { workspace, session } = self;
        drop(session);
        complete_workspace(
            Some(workspace.clone()),
            workspace
                .native()
                .expect("actual native fixture owner")
                .clone(),
            result,
        )
        .await;
    }
}
async fn completed_rows<R: Record>(fixture: &Fixture) -> normalized::Rows<R> {
    let source = fixture.workspace.completed::<R>().unwrap();
    let mut batches = source.batches_async().await.unwrap();
    let mut rows = normalized::Rows::new(fixture.workspace.budget());
    let mut count = 0u64;
    while let Some(batch) = batches.next_async().await {
        let batch = batch.unwrap();
        count += batch.num_rows() as u64;
        rows.decode(&batch).unwrap();
    }
    assert_eq!(count, source.rows(), "{} exact metadata EOF", R::NAME);
    rows
}
/// Exercise current compact consumers against actual completed native owners.
/// Owner-specific scope selection stays with the model's uniform admission.
pub async fn assert_compact_owners(fixture: &Fixture, owners: &[analysis::owner::AnalysisOwner]) {
    use std::collections::BTreeSet;
    use uniform_outcome::{
        CoverageAuthority, ScopedUniformOutcome, UniformKind, UniformNotRequested,
    };
    assert!(
        !owners.is_empty(),
        "the control must select actual compact owners"
    );
    let workspace = &fixture.workspace;
    let budget = workspace.budget();
    let descriptors = completed_rows::<UniformNotRequested>(fixture).await;
    let runs = completed_rows::<attribution::ProviderRun>(fixture).await;
    let definitions = completed_rows::<analysis::AnalysisDefinition>(fixture).await;
    let scopes = completed_rows::<source::CoverageScope>(fixture).await;
    let frames = runs
        .iter()
        .map(|run| (run.input, run.context))
        .collect::<BTreeSet<_>>();
    assert!(
        !frames.is_empty(),
        "actual provider input/context frames are required"
    );
    let completed = workspace.completed_relations().unwrap();
    let access = workspace
        .inputs(
            "compact-owner-control",
            Profile::Catalog,
            completed.iter().map(|relation| relation.name()),
        )
        .unwrap();
    let sources =
        analysis::sources::CapturedSources::capture(Profile::Catalog, access.snapshots(), budget)
            .unwrap();
    let mut coverage = workspace
        .coverage_admission(&access, &sources)
        .await
        .unwrap();
    macro_rules! expected_inputs {($($field:ident:$ty:ty,)*)=>{$(
        let permit = access.read::<$ty>().unwrap();
        let mut batches = access.relation::<$ty>().unwrap().batches_async().await.unwrap();
        while let Some(batch) = batches.next_async().await {
            coverage.visit(&permit, &batch.unwrap()).unwrap();
        }
    )*};}
    lctx_model::expected_domain_inputs!(expected_inputs);
    let contributions = workspace
        .native()
        .expect("actual native fixture owner")
        .contributions(budget)
        .await
        .unwrap();
    let compact_view = workspace.completed::<UniformNotRequested>().unwrap();
    let foreign_scope: Id<source::CoverageScope> =
        serde_json::from_value(serde_json::to_value(vec![255u8; 16]).unwrap()).unwrap();
    assert!(
        scopes.get(foreign_scope).is_none(),
        "negative scope is outside actual captured metadata"
    );
    for &kind in owners {
        assert!(kind.can_be_wholly_unrequested());
        let selected = descriptors
            .iter()
            .filter(|row| row.owner == kind.name())
            .collect::<Vec<_>>();
        assert_eq!(
            selected.len(),
            frames.len(),
            "one {kind:?} compact descriptor per actual frame"
        );
        assert_eq!(
            selected
                .iter()
                .map(|row| (row.input, row.context))
                .collect::<BTreeSet<_>>(),
            frames
        );
        let mut method_definitions = BTreeSet::new();
        for &(method, _) in kind.methods() {
            let actual = definitions
                .iter()
                .filter(|row| row.method == method)
                .collect::<Vec<_>>();
            assert_eq!(
                actual.len(),
                1,
                "one selected {kind:?}/{method:?} definition"
            );
            method_definitions.insert(actual[0].content_digest());
        }
        let foreign_definition = ContentHash::of(b"unselected-compact-owner-definition");
        assert!(!method_definitions.contains(&foreign_definition));
        for descriptor in selected {
            assert_eq!(
                (descriptor.kind, descriptor.family),
                (UniformKind::AnalysisOwner, None)
            );
            let domain = coverage
                .uniform_analysis(
                    kind,
                    descriptor.input,
                    descriptor.context,
                    &definitions,
                    budget,
                )
                .unwrap()
                .expect("actual admitted compact owner");
            assert_eq!(
                domain.descriptor_row().unwrap(),
                *descriptor,
                "exact selected declaration, ordered sources and definitions for {kind:?}"
            );
            let members = domain.members().copied().collect::<BTreeSet<_>>();
            assert_eq!(descriptor.members, members.len() as i64);
            assert_eq!(domain.is_explicit_empty(), members.is_empty());
            let authority = CoverageAuthority::Uniform {
                descriptor: descriptor.id(),
            };
            assert_eq!(domain.authority(), authority);
            for &(scope, definition) in &members {
                assert!(
                    scopes.get(scope).is_some(),
                    "{kind:?} member uses an actual captured scope"
                );
                assert!(
                    method_definitions.contains(&definition),
                    "{kind:?} member uses its selected method"
                );
                assert_eq!(
                    domain.scoped(scope, definition),
                    ScopedUniformOutcome::NotRequested(authority)
                );
                let evidence = domain.evidence(scope, definition).unwrap();
                assert_eq!(
                    (
                        evidence.descriptor(),
                        evidence.scope(),
                        evidence.context(),
                        evidence.family(),
                        evidence.definition()
                    ),
                    (descriptor.id(), scope, descriptor.context, None, definition)
                );
            }
            // For a nonempty domain, isolate the definition mismatch using a
            // scope that has already earned positive membership. Explicit-empty
            // domains legitimately refuse every pair, including captured scopes.
            let actual_scope = members.first().map(|(scope, _)| *scope).unwrap_or_else(|| {
                source::CoverageScope::Input {
                    input: descriptor.input,
                }
                .id()
            });
            assert!(scopes.get(actual_scope).is_some());
            assert_eq!(
                domain.scoped(actual_scope, foreign_definition),
                ScopedUniformOutcome::OutsideDomain
            );
            assert!(domain.evidence(actual_scope, foreign_definition).is_none());
            for &definition in &method_definitions {
                assert_eq!(
                    domain.scoped(foreign_scope, definition),
                    ScopedUniformOutcome::OutsideDomain
                );
                assert!(domain.evidence(foreign_scope, definition).is_none());
            }
        }
        let physical = contributions
            .iter()
            .filter(|owner| owner.spec.producer == kind.name())
            .collect::<Vec<_>>();
        assert_eq!(physical.len(), 1, "one current completed {kind:?} producer");
        let owner = physical[0];
        owner.spec.validate_profile(Profile::Catalog).unwrap();
        assert!(owner.assurance.is_current());
        assert_eq!(
            owner.outcome, 0,
            "compact semantic NotRequested still completes physical production"
        );
        assert!(
            compact_view
                .view()
                .contributions
                .contains(&owner.identity().unwrap())
        );
        assert_eq!(
            owner.outputs.get(UniformNotRequested::NAME).unwrap().rows,
            frames.len() as u64
        );
        for (name, output) in &owner.outputs {
            if name == UniformNotRequested::NAME {
                continue;
            }
            assert_eq!(
                output.rows, 0,
                "{kind:?} ordinary output {name} is explicitly empty"
            );
            if !stages::is_epoch_shared(name) {
                assert_eq!(
                    workspace.relation(name).unwrap().rows(),
                    0,
                    "{kind:?} ordinary completed output {name} is empty"
                );
            }
        }
    }
}
pub async fn complete_workspace<T>(
    workspace: Option<Arc<Workspace>>,
    native: Arc<lctx_compiler_store::compiler::NativeCompilerStore>,
    result: std::thread::Result<T>,
) -> T {
    use lctx_model::domain::completion::{self, LocalState, RemoteState, StorageState};
    let mut finality = match &workspace {
        Some(workspace) => workspace.drain_owned_report().await,
        None => completion::Completion::default(),
    };
    if finality.local == LocalState::Terminal
        && finality.remote == RemoteState::Confirmed
        && finality
            .failures
            .iter()
            .all(|failure| failure.error.permits_storage_cleanup())
    {
        let mut source = native.abandon_finality_report().await;
        if source.local == LocalState::Outstanding {
            finality.local = LocalState::Outstanding;
        }
        if source.remote == RemoteState::Unknown {
            finality.remote = RemoteState::Unknown;
        }
        finality.storage.append(&mut source.storage);
        finality.committed.append(&mut source.committed);
        finality.failures.append(&mut source.failures);
    } else {
        finality.storage.push(StorageState::Orphan(format!(
            "native_attempt:{}",
            native.attempt().hex()
        )));
    }
    if let Some(workspace) = &workspace {
        let signal = workspace.complete_signal(&finality).await;
        finality.step("Summary fixture workspace signal", signal);
    }
    let mut diagnostics = completion::Completion::default();
    if let Some(workspace) = &workspace {
        workspace.append_operation_failures(&mut diagnostics);
    } else {
        native.append_operation_failures(&mut diagnostics);
    }
    let value = match result {
        Ok(value) => value,
        Err(panic) => {
            if let Err(error) = completion::complete(Ok(()), finality) {
                eprintln!("Summary fixture finalization after assertion panic: {error:#?}");
            }
            if !diagnostics.failures.is_empty() {
                eprintln!(
                    "Summary fixture operation diagnostics after assertion panic: {diagnostics:#?}"
                );
            }
            std::panic::resume_unwind(panic);
        }
    };
    completion::complete(Ok(()), finality).unwrap();
    completion::complete(Ok(()), diagnostics).unwrap();
    value
}
pub fn settings(module: &str) -> AnalyticsConfiguration {
    AnalyticsConfiguration {
        module_prefixes: vec![module.into()],
        public_roots: vec![module.into()],
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
pub fn root(case: &str) -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/python")
        .join(case)
}
pub fn capture(
    case: &str,
    profile: Profile,
    budget: &resources::ResourceBudget,
) -> Arc<CapturedInputs> {
    try_capture(case, profile, budget).unwrap()
}
pub fn try_capture(
    case: &str,
    profile: Profile,
    budget: &resources::ResourceBudget,
) -> Result<Arc<CapturedInputs>, ModelError> {
    let root = root(case);
    let mut paths = Vec::new();
    let mut pending = vec![root.clone()];
    while let Some(directory) = pending.pop() {
        for entry in
            std::fs::read_dir(directory).map_err(|error| ModelError::Cause(Box::new(error)))?
        {
            let path = entry
                .map_err(|error| ModelError::Cause(Box::new(error)))?
                .path();
            if path.is_dir() {
                pending.push(path);
            } else {
                paths.push(
                    path.strip_prefix(&root)
                        .map_err(ModelError::codec)?
                        .to_string_lossy()
                        .into_owned(),
                );
            }
        }
    }
    paths.sort();
    let documents = paths
        .iter()
        .filter(|p| p.ends_with(".md") || p.ends_with(".mdx"))
        .cloned()
        .collect::<Vec<_>>();
    Ok(Arc::new(CapturedInputs::new(
        vec![AcquiredInput::tree(
            CapturedInput::capture_derived(&root, &paths, budget, &documents, derive_blocks)
                .map_err(|error| ModelError::Cause(Box::new(error)))?,
            case,
        )],
        cpg_extract::native_context::NativeContextConfig::committed(profile, budget)?,
    )))
}
pub async fn compile(
    case: &str,
    profile: Profile,
    frontier: Frontier,
    settings: AnalyticsConfiguration,
    embedder: Option<Arc<dyn Embedder>>,
) -> Fixture {
    let native = crate::native_fixture::store();
    let mut workspace_owner = None;
    let result = std::panic::AssertUnwindSafe(async {
        let workspace = Workspace::new(
            Arc::new(model().unwrap()),
            WorkspaceOptions::default(),
            native.clone(),
        )
        .unwrap();
        workspace_owner = Some(workspace.clone());
        let captured = capture(case, profile, workspace.budget());
        let prepared = matches!(frontier, Frontier::Analysis | Frontier::Catalog).then(|| {
            PreparedCompilation::new(
                frontier,
                settings,
                captured.config().catalog(),
                embedder.as_deref(),
                workspace.budget(),
            )
            .map(std::sync::Arc::new)
            .unwrap()
        });
        compilation::compile(
            &workspace,
            captured,
            profile,
            ContentHash::of(b"compiler-fixture"),
            frontier,
            prepared.clone(),
            embedder,
            None,
        )
        .await
        .unwrap();
        workspace.admit_semantics(profile).await.unwrap();
        drop(prepared);
        let completed = workspace.completed_relations().unwrap();
        let inputs = workspace
            .inputs(
                "fixture-inspection",
                profile,
                completed.iter().map(|r| r.name()),
            )
            .unwrap();
        let session = inputs.session(&workspace).await.unwrap();
        Fixture { workspace, session }
    })
    .catch_unwind()
    .await;
    match result {
        // Successful construction transfers its useful live owner to the caller.
        // Only construction failure drains it here, after provisional borrowers drop.
        Ok(fixture) => fixture,
        Err(panic) => complete_workspace(workspace_owner, native, Err(panic)).await,
    }
}
pub trait Cell: Sized {
    fn from_value(value: ScalarValue) -> Self;
}
impl Cell for i64 {
    fn from_value(value: ScalarValue) -> Self {
        match value {
            ScalarValue::Int16(Some(v)) => v.into(),
            ScalarValue::Int32(Some(v)) => v.into(),
            ScalarValue::Int64(Some(v)) => v,
            ScalarValue::UInt8(Some(v)) => v.into(),
            ScalarValue::UInt16(Some(v)) => v.into(),
            ScalarValue::UInt32(Some(v)) => v.into(),
            ScalarValue::UInt64(Some(v)) => v.try_into().unwrap(),
            _ => panic!("expected integer: {value:?}"),
        }
    }
}
impl Cell for i16 {
    fn from_value(value: ScalarValue) -> Self {
        i64::from_value(value).try_into().unwrap()
    }
}
impl Cell for bool {
    fn from_value(value: ScalarValue) -> Self {
        match value {
            ScalarValue::Boolean(Some(v)) => v,
            _ => panic!("expected boolean: {value:?}"),
        }
    }
}
impl Cell for String {
    fn from_value(value: ScalarValue) -> Self {
        match value {
            ScalarValue::Utf8(Some(v))
            | ScalarValue::LargeUtf8(Some(v))
            | ScalarValue::Utf8View(Some(v)) => v,
            _ => panic!("expected string: {value:?}"),
        }
    }
}
impl Cell for Vec<u8> {
    fn from_value(value: ScalarValue) -> Self {
        match value {
            ScalarValue::Binary(Some(v))
            | ScalarValue::LargeBinary(Some(v))
            | ScalarValue::BinaryView(Some(v))
            | ScalarValue::FixedSizeBinary(_, Some(v)) => v,
            _ => panic!("expected bytes: {value:?}"),
        }
    }
}
impl<T: Cell> Cell for Option<T> {
    fn from_value(value: ScalarValue) -> Self {
        if value.is_null() {
            None
        } else {
            Some(T::from_value(value))
        }
    }
}
pub trait SqlRow: Sized {
    fn from_values(values: Vec<ScalarValue>) -> Self;
}
macro_rules! scalar {($($ty:ty),*)=>{$(impl SqlRow for $ty {fn from_values(mut values:Vec<ScalarValue>)->Self {assert_eq!(values.len(),1);Self::from_value(values.remove(0))}})*};}
scalar!(i64, i16, bool, String, Vec<u8>);
impl<T: Cell> SqlRow for Option<T> {
    fn from_values(mut values: Vec<ScalarValue>) -> Self {
        assert_eq!(values.len(), 1);
        Self::from_value(values.remove(0))
    }
}
macro_rules! tuple {($($ty:ident),*)=>{impl<$($ty:Cell),*> SqlRow for ($($ty,)*) {fn from_values(values:Vec<ScalarValue>)->Self {let mut values=values.into_iter();let result=($($ty::from_value(values.next().unwrap()),)*);assert!(values.next().is_none());result}}};}
tuple!(A, B);
tuple!(A, B, C);
tuple!(A, B, C, D);
tuple!(A, B, C, D, E);
tuple!(A, B, C, D, E, F);
tuple!(A, B, C, D, E, F, G);
tuple!(A, B, C, D, E, F, G, H);
tuple!(A, B, C, D, E, F, G, H, I);
tuple!(A, B, C, D, E, F, G, H, I, J);
tuple!(A, B, C, D, E, F, G, H, I, J, K);
tuple!(A, B, C, D, E, F, G, H, I, J, K, L);
tuple!(A, B, C, D, E, F, G, H, I, J, K, L, M);
tuple!(A, B, C, D, E, F, G, H, I, J, K, L, M, N);
tuple!(A, B, C, D, E, F, G, H, I, J, K, L, M, N, O);
pub async fn query<T: SqlRow>(fixture: &Fixture, sql: impl AsRef<str>) -> Vec<T> {
    let batches = fixture
        .session
        .sql(sql.as_ref())
        .await
        .unwrap_or_else(|e| panic!("query {}: {e}", sql.as_ref()))
        .collect()
        .await
        .unwrap();
    batches
        .iter()
        .flat_map(|batch| {
            (0..batch.num_rows()).map(|row| {
                T::from_values(
                    batch
                        .columns()
                        .iter()
                        .map(|column| ScalarValue::try_from_array(column, row).unwrap())
                        .collect(),
                )
            })
        })
        .collect()
}
pub async fn one<T: SqlRow>(fixture: &Fixture, sql: impl AsRef<str>) -> T {
    let mut rows = query(fixture, sql).await;
    assert_eq!(rows.len(), 1);
    rows.remove(0)
}
pub async fn one_with<T: SqlRow>(
    fixture: &Fixture,
    sql: impl AsRef<str>,
    parameters: Vec<ScalarValue>,
) -> T {
    let batches = fixture
        .session
        .sql(sql.as_ref())
        .await
        .unwrap()
        .with_param_values(parameters)
        .unwrap()
        .collect()
        .await
        .unwrap();
    let mut rows = batches
        .iter()
        .flat_map(|batch| {
            (0..batch.num_rows()).map(|row| {
                T::from_values(
                    batch
                        .columns()
                        .iter()
                        .map(|column| ScalarValue::try_from_array(column, row).unwrap())
                        .collect(),
                )
            })
        })
        .collect::<Vec<_>>();
    assert_eq!(rows.len(), 1);
    rows.remove(0)
}
