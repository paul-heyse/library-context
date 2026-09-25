//! Focused real-provider probe. No Delta publication or integrated gate.
use cpg_schema::{id::Id, table::Table};

fn main() {
    tokio::runtime::Runtime::new().unwrap().block_on(run());
}

async fn run() {
    let dir = tempfile::tempdir().unwrap();
    let release = dir.path().join("release");
    let site = dir.path().join("venv/site-packages");
    std::fs::create_dir_all(&release).unwrap();
    std::fs::create_dir_all(&site).unwrap();
    std::fs::write(release.join("probe.py"), r#"
import builtins
from builtins import getattr as read_attribute

class Settings:
    def __init__(self):
        self.literal_only = 1
        self.attribute_control = 2
        self.qualified_only = 3
        self.aliased_only = 4
    def literal(self):
        return getattr(self, "literal_only")
    def attribute(self):
        return self.attribute_control
    def qualified(self):
        return builtins.getattr(self, "qualified_only")
    def aliased(self):
        return read_attribute(self, "aliased_only")

settings = Settings()
"#).unwrap();
    let s = Id([9;16]);
    let out = cpg_extract::extract(&cpg_extract::ExtractInput {
        release: cpg_extract::Release::from_tree(release,"literal_probe").unwrap(),
        venv_root:dir.path().join("venv"), site_packages:vec![site],
        python_version:(3,14,0), python_platform:"linux".into(), snapshot_id:s,
        corpus:None, keep_pysa_json:false, test_hooks:cpg_extract::TestHooks::default(),
    }).unwrap();
    let ctx = cpg_core::snapshot::empty_session();
    for (name,batch) in &out.tables { ctx.register_batch(*name,batch.clone()).unwrap(); }
    macro_rules! derive_all { ($($t:ty),+) => {$(
        let batch = cpg_core::derive::derive::<$t>(&ctx,s).await.unwrap();
        ctx.register_batch(<$t as Table>::NAME,batch).unwrap();
    )+}; }
    cpg_schema::for_each_derived_table!(derive_all);
    let flow = cpg_core::flow_model::run(&ctx,s).await.unwrap();
    for p in &flow.premises {
        if p.place_key.contains("_only") || p.place_key.contains("attribute_control") {
            println!("{} holds={} boundary={:?}",p.place_key,p.holds,p.boundary_reason);
        }
    }
    println!("dynamic accesses: {}",flow.dynamic_accesses.len());
    assert!(flow.premises.iter().any(|p|p.place_key.contains("literal_only")&&!p.holds));
    assert!(flow.premises.iter().any(|p|p.place_key.contains("attribute_control")&&!p.holds));
    assert!(flow.premises.iter().any(|p|p.place_key.contains("qualified_only")&&p.holds));
    assert!(flow.premises.iter().any(|p|p.place_key.contains("aliased_only")&&p.holds));
    assert!(flow.dynamic_accesses.is_empty());
    println!("passed: real extraction and flow_model reproduce incorrect never-read premises for qualified/aliased getattr; bare getattr and attribute controls are correctly read");
}
