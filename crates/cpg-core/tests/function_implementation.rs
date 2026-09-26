//! Abstract and placeholder decisions use Pyrefly's resolved function flags.
use cpg_schema::{codebook::BoundaryReason, id::Id, table::Table};

#[tokio::test]
async fn aliased_abstract_and_protocol_placeholders_do_not_create_no_read_claims() {
    let dir = tempfile::tempdir().unwrap();
    let release = dir.path().join("release");
    let site = dir.path().join("venv/site-packages");
    std::fs::create_dir_all(&release).unwrap();
    std::fs::create_dir_all(&site).unwrap();
    std::fs::write(
        release.join("probe.py"),
        r#"
from abc import abstractmethod as abstract_alias
from typing import Protocol

class Abstract:
    @abstract_alias
    def run(self, unused): ...

class Interface(Protocol):
    def dispatch(self, unused): ...

class Concrete:
    def run(self, unused):
        return 1
    def not_implemented_value(self, unused):
        return NotImplemented
"#,
    )
    .unwrap();
    let snapshot = Id([8; 16]);
    let out = cpg_extract::extract(&cpg_extract::ExtractInput {
        release: cpg_extract::Release::from_tree(release, "function_status_probe").unwrap(),
        venv_root: dir.path().join("venv"),
        site_packages: vec![site],
        python_version: (3, 14, 0),
        python_platform: "linux".into(),
        snapshot_id: snapshot,
        corpus: None,
        keep_pysa_json: false,
        test_hooks: cpg_extract::TestHooks::default(),
    })
    .unwrap();
    let ctx = cpg_core::snapshot::empty_session();
    for (name, batch) in &out.tables {
        ctx.register_batch(*name, batch.clone()).unwrap();
    }
    macro_rules! derive_all {
        ($($t:ty),+) => {$ (
            let batch = cpg_core::derive::derive::<$t>(&ctx, snapshot).await.unwrap();
            ctx.register_batch(<$t as Table>::NAME, batch).unwrap();
        )+};
    }
    cpg_schema::for_each_derived_table!(derive_all);
    let flow = cpg_core::flow_model::run(&ctx, snapshot).await.unwrap();
    let reasons: Vec<_> = flow.premises.iter().filter_map(|row| row.reason.as_deref()).collect();
    assert!(reasons.iter().any(|reason| reason.contains("an abstract method")), "{reasons:?}");
    assert!(reasons.iter().any(|reason| reason.contains("a Protocol placeholder")), "{reasons:?}");
    assert_eq!(
        flow.premises.iter().filter(|row| row.boundary_reason == Some(BoundaryReason::AbstractBody)).count(),
        4,
    );
    assert_eq!(flow.premises.iter().filter(|row| row.holds).count(), 4);
}
