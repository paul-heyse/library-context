//! Resolved builtin access must be represented like direct attribute reads.
use cpg_schema::{id::Id, table::Table};

#[tokio::test]
async fn equivalent_getattr_spellings_and_computed_names_keep_no_read_sound() {
    let dir = tempfile::tempdir().unwrap();
    let release = dir.path().join("release");
    let site = dir.path().join("venv/site-packages");
    std::fs::create_dir_all(&release).unwrap();
    std::fs::create_dir_all(&site).unwrap();
    std::fs::write(
        release.join("probe.py"),
        r#"
import builtins
from builtins import getattr as read_attribute

class Settings:
    def __init__(self):
        self.bare_only = 1
        self.qualified_only = 2
        self.aliased_only = 3
        self.direct_only = 4
        self.unread_only = 5
    def bare(self):
        return getattr(self, "bare_only")
    def qualified(self):
        return builtins.getattr(self, "qualified_only")
    def aliased(self):
        return read_attribute(self, "aliased_only")
    def direct(self):
        return self.direct_only
    def shadowed(self):
        getattr = lambda obj, name: 0
        return getattr(self, "unread_only")

settings = Settings()

class DynamicSettings:
    def __init__(self):
        self.computed_name = 1
    def computed(self, suffix):
        return getattr(self, "computed_" + suffix)
    def formatted(self, suffix):
        return getattr(self, "{}".format(suffix))
    def joined(self, parts):
        return getattr(self, "".join(parts))

dynamic_settings = DynamicSettings()
"#,
    )
    .unwrap();
    let snapshot = Id([9; 16]);
    let out = cpg_extract::extract(&cpg_extract::ExtractInput {
        release: cpg_extract::Release::from_tree(release, "access_probe").unwrap(),
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
    for field in ["bare_only", "qualified_only", "aliased_only", "direct_only"] {
        assert!(
            flow.premises.iter().any(|row| {
                row.place_key.contains(field) && !row.holds && row.boundary_reason.is_none()
            }),
            "{field} must not be a no-read premise"
        );
    }
    assert!(flow.premises.iter().any(|row| {
        row.place_key.contains("unread_only") && row.holds
    }));
    assert_eq!(
        flow.dynamic_accesses
            .iter()
            .filter(|row| row.kind == cpg_schema::codebook::DynamicKind::Getattr)
            .count(),
        3
    );
}
