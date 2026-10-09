#![allow(
    dead_code,
    reason = "Shared fixture runner serves focused compiler controls"
)]
//! Fixture inputs run through the actual persisted compiler; assertions inspect completed native views.
use cpg_core::{
    compilation::{self, PreparedCompilation},
    embedding_service::Embedder,
    workspace::{Workspace, WorkspaceOptions},
};
use cpg_extract::{
    acquisition::{AcquiredInput, derive_blocks},
    bundle::CapturedInputs,
    capture::CapturedInput,
};
use datafusion::{common::ScalarValue, prelude::SessionContext};
use lctx_model::domain::{
    admission::Frontier, analysis::settings::AnalyticsConfiguration, stages::Profile, *,
};
use std::{path::Path, sync::Arc};

pub struct Fixture {
    pub workspace: Arc<Workspace>,
    pub session: SessionContext,
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
    let root = root(case);
    let mut paths = Vec::new();
    let mut pending = vec![root.clone()];
    while let Some(directory) = pending.pop() {
        for entry in std::fs::read_dir(directory).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                pending.push(path);
            } else {
                paths.push(
                    path.strip_prefix(&root)
                        .unwrap()
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
    Arc::new(CapturedInputs::new(
        vec![AcquiredInput::tree(
            CapturedInput::capture_derived(&root, &paths, budget, &documents, derive_blocks)
                .unwrap(),
            case,
        )],
        cpg_extract::native_context::NativeContextConfig::committed(profile, budget).unwrap(),
    ))
}
pub async fn compile(
    case: &str,
    profile: Profile,
    frontier: Frontier,
    settings: AnalyticsConfiguration,
    embedder: Option<&dyn Embedder>,
) -> Fixture {
    let workspace = Workspace::new(
        Arc::new(model().unwrap()),
        WorkspaceOptions::default(),
        crate::native_fixture::store(),
    )
    .unwrap();
    let captured = capture(case, profile, workspace.budget());
    let prepared = matches!(frontier, Frontier::Analysis | Frontier::Catalog).then(|| {
        PreparedCompilation::new(
            frontier,
            settings,
            captured.config().catalog(),
            embedder,
            workspace.budget(),
        )
        .unwrap()
    });
    compilation::compile(
        &workspace,
        captured,
        profile,
        ContentHash::of(b"compiler-fixture"),
        frontier,
        prepared.as_ref(),
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
