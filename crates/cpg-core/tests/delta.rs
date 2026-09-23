//! Delta built-ins as the design uses them (DESIGN §4.3, §6; review F4, F7; spike S6).

use std::path::Path;
use std::sync::Arc;

use arrow_array::{Array, Int16Array, RecordBatch};
use cpg_core::delta::{append, create, open_or_create, open_verified, read_at, table_url};
use cpg_core::{CoreError, sql};
use cpg_extract::{ExtractInput, extract};
use cpg_schema::codebook::DeclarationKind;
use cpg_schema::id::{Id, IdHasher};
use cpg_schema::table::Table;
use cpg_schema::tables::{Boundaries, Declarations, DeclarationsRow};
use datafusion::prelude::SessionContext;
use deltalake::DeltaTable;
use deltalake::delta_datafusion::create_session;
use deltalake::kernel::StructType;
use deltalake::kernel::engine::arrow_conversion::TryIntoKernel as _;

fn fixture_output(dir: &Path) -> cpg_extract::ExtractOutput {
    let repo = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let release = dir.join("release");
    copy(&repo.join("fixtures/python/pysa_variants"), &release);
    let site = dir.join("venv/site-packages");
    std::fs::create_dir_all(&site).unwrap();
    extract(&ExtractInput {
        release: cpg_extract::Release::from_tree(
            std::fs::canonicalize(&release).unwrap(),
            "pysa_variants",
        )
        .unwrap(),
        venv_root: std::fs::canonicalize(dir.join("venv")).unwrap(),
        site_packages: vec![std::fs::canonicalize(&site).unwrap()],
        python_version: (3, 14, 0),
        python_platform: "linux".to_owned(),
        snapshot_id: Id([9; 16]),
        keep_pysa_json: false,
        test_hooks: Default::default(),
    })
    .unwrap()
}

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

#[tokio::test]
async fn every_table_round_trips_through_delta_exactly() {
    let dir = tempfile::tempdir().unwrap();
    let out = fixture_output(dir.path());
    let root = dir.path().join("delta");
    let snapshot = Id([9; 16]);
    macro_rules! roundtrip {
        ($($t:ty),+) => {$({
            let batch = out.table(<$t as Table>::NAME).unwrap().clone();
            let table = open_or_create::<$t>(&root).await.unwrap();
            let table = append(table, batch.clone(), snapshot).await.unwrap();
            let back = read_at::<$t>(&root, table.version().unwrap(), snapshot).await.unwrap();
            assert_eq!(back, batch, "{} changed through Delta", <$t as Table>::NAME);
        })+};
    }
    cpg_schema::for_each_table!(roundtrip);
    // Reopening verifies appendOnly and the CHECK set against the declarations.
    macro_rules! reopen {
        ($($t:ty),+) => {$( open_verified::<$t>(&root).await.unwrap(); )+};
    }
    cpg_schema::for_each_table!(reopen);
}

fn decl(start: i64, end: i64) -> RecordBatch {
    decl_in(Id([1; 16]), start, end)
}

fn decl_in(snapshot_id: Id, start: i64, end: i64) -> RecordBatch {
    let id = IdHasher::new("t").i64(start).finish_id();
    Declarations::to_batch(&[DeclarationsRow {
        snapshot_id,
        fact_id: id,
        node_id: id,
        module_node_id: id,
        parent_node_id: None,
        qualified_name: "m.f".to_owned(),
        name: "f".to_owned(),
        kind: DeclarationKind::Function,
        start_byte: start,
        end_byte: end,
        name_start_byte: start,
        name_end_byte: start,
        docstring: None,
        docstring_start_byte: None,
        docstring_end_byte: None,
        is_overload: false,
        decorators: vec![],
    }])
    .unwrap()
}

#[tokio::test]
async fn reads_pin_the_version_and_filter_the_snapshot() {
    // Review F7: with two snapshots in one table, a read that ignored either the version pin or
    // the snapshot filter returns the wrong rows.
    let dir = tempfile::tempdir().unwrap();
    let (a, b) = (Id([1; 16]), Id([2; 16]));
    let t = create::<Declarations>(dir.path()).await.unwrap();
    let t = append(t, decl_in(a, 10, 20), a).await.unwrap();
    let at_a = t.version().unwrap();
    let t = append(t, decl_in(b, 30, 40), b).await.unwrap();
    let at_b = t.version().unwrap();
    for version in [at_a, at_b] {
        let back = read_at::<Declarations>(dir.path(), version, a)
            .await
            .unwrap();
        assert_eq!(back, decl_in(a, 10, 20), "snapshot A at version {version}");
    }
    let early = read_at::<Declarations>(dir.path(), at_a, b).await.unwrap();
    assert_eq!(early.num_rows(), 0, "B is not visible at A's version");
    let late = read_at::<Declarations>(dir.path(), at_b, b).await.unwrap();
    assert_eq!(late, decl_in(b, 30, 40));
    // A missing table is "not a table", and reading it creates nothing.
    assert!(open_verified::<Boundaries>(dir.path()).await.is_err());
    assert!(!dir.path().join(Boundaries::NAME).exists());
}

#[tokio::test]
async fn open_refuses_a_table_whose_checks_drift_or_are_missing() {
    let dir = tempfile::tempdir().unwrap();
    let t = create::<Declarations>(dir.path()).await.unwrap();
    open_verified::<Declarations>(dir.path()).await.unwrap();
    t.drop_constraints()
        .with_constraint("span_order")
        .await
        .unwrap();
    let err = open_verified::<Declarations>(dir.path()).await.unwrap_err();
    assert!(matches!(err, CoreError::ConstraintMismatch { .. }), "{err}");

    // A crash between create and add_constraint leaves a table without its CHECKs.
    let other = tempfile::tempdir().unwrap();
    let kernel: StructType = Declarations::schema().as_ref().try_into_kernel().unwrap();
    std::fs::create_dir_all(other.path().join(Declarations::NAME)).unwrap();
    DeltaTable::try_from_url(table_url(other.path(), Declarations::NAME).unwrap())
        .await
        .unwrap()
        .create()
        .with_columns(kernel.fields().cloned())
        .with_configuration_property(deltalake::TableProperty::AppendOnly, Some("true"))
        .await
        .unwrap();
    let err = open_verified::<Declarations>(other.path())
        .await
        .unwrap_err();
    assert!(matches!(err, CoreError::ConstraintMismatch { .. }), "{err}");
}

#[tokio::test]
async fn append_enforces_the_immutable_checks() {
    let dir = tempfile::tempdir().unwrap();
    let t = create::<Declarations>(dir.path()).await.unwrap();
    let t = append(t, decl(10, 20), Id([1; 16])).await.unwrap();
    assert!(
        append(t, decl(20, 10), Id([1; 16])).await.is_err(),
        "end < start is rejected"
    );
}

#[tokio::test]
async fn codebook_growth_needs_no_constraint_change() {
    // Codebooks are append-only and never a CHECK (DESIGN §8): a code the current codebook does
    // not yet have must still be writable, or the next codebook append would stall.
    let dir = tempfile::tempdir().unwrap();
    let t = create::<Boundaries>(dir.path()).await.unwrap();
    let id = Id([3; 16]);
    let empty = Boundaries::to_batch(&[]).unwrap();
    let mut columns = Vec::new();
    for f in Boundaries::schema().fields() {
        let array: Arc<dyn Array> = match f.name().as_str() {
            "reason" => Arc::new(Int16Array::from(vec![99_i16])),
            "fact_family" => Arc::new(Int16Array::from(vec![1_i16])),
            "snapshot_id" | "fact_id" | "module_node_id" => Arc::new(
                arrow_array::FixedSizeBinaryArray::try_from_iter([id.0].into_iter()).unwrap(),
            ),
            _ => arrow_array::new_null_array(f.data_type(), 1),
        };
        columns.push(array);
    }
    let batch = RecordBatch::try_new(empty.schema(), columns).unwrap();
    append(t, batch, id)
        .await
        .expect("a future code is writable");
}

#[tokio::test]
async fn the_sql_helper_rejects_writes_at_plan_time() {
    let dir = tempfile::tempdir().unwrap();
    let t = create::<Declarations>(dir.path()).await.unwrap();
    let ctx: SessionContext = create_session().into_inner();
    t.update_datafusion_session(&ctx.state()).unwrap();
    ctx.register_table("t", t.table_provider().await.unwrap())
        .unwrap();
    for statement in [
        "INSERT INTO t SELECT * FROM t",
        "CREATE TABLE x AS SELECT 1",
        "SET datafusion.execution.target_partitions = 4",
    ] {
        assert!(
            sql::query(&ctx, statement).await.is_err(),
            "{statement} must be refused"
        );
    }
    assert!(sql::query(&ctx, "SELECT count(*) FROM t").await.is_ok());
}

#[tokio::test]
async fn insert_into_still_bypasses_checks_at_this_delta_rs_revision() {
    // Pinned upstream behaviour (delta-rs 58f07cd6, spike S6): DataFusion INSERT INTO commits a
    // CHECK-violating row. If this starts failing, the bypass is fixed upstream: revisit §4.3.
    let dir = tempfile::tempdir().unwrap();
    let t = create::<Declarations>(dir.path()).await.unwrap();
    let t = append(t, decl(10, 20), Id([1; 16])).await.unwrap();
    let ctx: SessionContext = create_session().into_inner();
    t.update_datafusion_session(&ctx.state()).unwrap();
    ctx.register_table("t", t.table_provider().await.unwrap())
        .unwrap();
    let bad = "INSERT INTO t SELECT snapshot_id, fact_id, node_id, module_node_id, parent_node_id, \
               qualified_name, name, kind, 20, 10, name_start_byte, name_end_byte, docstring, \
               docstring_start_byte, docstring_end_byte, is_overload, decorators FROM t";
    let inserted = ctx.sql(bad).await.unwrap().collect().await;
    assert!(
        inserted.is_ok(),
        "the INSERT INTO bypass is gone: {inserted:?}"
    );
    let reloaded = open_verified::<Declarations>(dir.path()).await.unwrap();
    assert!(reloaded.version() > t.version());
}
