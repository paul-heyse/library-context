//! Real compiler + disposable PG18 fixture; acquisition command is a bounded no-op.
use std::{path::Path,process::Command,sync::Arc};
use lctx_model::domain::{self,serving::*};
use lctx_postgres::{generations::{GenerationStore,GenerationService,CatalogService,GenerationId},testing::DisposableDatabase};
pub fn write(path: &Path, bytes: impl AsRef<[u8]>) {
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, bytes).unwrap();
}
pub fn input_source(root: &Path, source: &[u8]) {
    write(
        &root.join("libraries/demo/pyproject.toml"),
        "[project]\nname = 'lctx-library-demo'\nversion = '0'\ndependencies = ['demo==1.0']\n[tool.lctx]\nrelease = ['demo']\n",
    );
    write(&root.join("libraries/demo/.python-version"), "3.14.7\n");
    write(
        &root.join("libraries/demo/uv.lock"),
        format!(
            "version = 1\nrevision = 3\nrequires-python = '==3.14.*'\n[[package]]\nname = 'demo'\nversion = '1.0'\nsource = {{ registry = 'https://pypi.org/simple' }}\nwheels = [{{ url = 'https://x/demo.whl', hash = 'sha256:{}', size = 1 }}]\n[[package]]\nname = 'lctx-library-demo'\nversion = '0'\nsource = {{ virtual = '.' }}\ndependencies = [{{ name = 'demo' }}]\n",
            "a".repeat(64)
        ),
    );
    write(
        &root.join("envs/demo/pyvenv.cfg"),
        "home = /x\nuv = 0.12.18\nversion_info = 3.14.7\n",
    );
    let metadata = b"Metadata-Version: 2.4\nName: demo\nVersion: 1.0\n";
    let site = root.join("envs/demo/lib/python3.14/site-packages");
    write(&site.join("demo/__init__.py"), source);
    write(&site.join("demo-1.0.dist-info/METADATA"), metadata);
    use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
    use sha2::{Digest as _, Sha256};
    write(
        &site.join("demo-1.0.dist-info/RECORD"),
        format!(
            "demo/__init__.py,sha256={},{}\ndemo-1.0.dist-info/METADATA,sha256={},{}\ndemo-1.0.dist-info/RECORD,,\n",
            URL_SAFE_NO_PAD.encode(Sha256::digest(source)),
            source.len(),
            URL_SAFE_NO_PAD.encode(Sha256::digest(metadata)),
            metadata.len()
        ),
    );
    write(&root.join("bin/uv"), "#!/bin/sh\nexit 0\n");
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(root.join("bin/uv"), std::fs::Permissions::from_mode(0o700)).unwrap();
}
pub fn command(root: &Path, cfg: &Path, through: &str, profile: &str) -> Command {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_lctx"));
    cmd.args([
        "compile",
        "demo",
        "--through",
        through,
        "--profile",
        profile,
    ])
    .arg("--database")
    .arg(cfg)
    .arg("--libraries")
    .arg(root.join("libraries"))
    .arg("--envs")
    .arg(root.join("envs"))
    .arg("--sources")
    .arg(root.join("sources"))
    .env(
        "PATH",
        format!(
            "{}:{}",
            root.join("bin").display(),
            std::env::var("PATH").unwrap_or_default()
        ),
    );
    cmd
}

pub const SOURCE:&[u8]=br#"__all__ = ['api', 'consume', 'Holder']
def api(flag: bool = False) -> bool:
    return flag

def consume(value: int) -> int:
    return value

class Holder:
    def method(self, token: str = 'ready') -> str:
        return token
    class Nested:
        def nested(self, flag: bool = False):
            return flag
"#;
pub struct ServingFixture {pub db:DisposableDatabase,pub store:GenerationStore,pub service:GenerationService,pub catalog:CatalogService,pub generation:GenerationId,pub dir:tempfile::TempDir}
impl ServingFixture {
    pub async fn start(source:&[u8])->Self {Self::start_profile(source,"catalog").await}
    pub async fn start_profile(source:&[u8],profile:&str)->Self {Self::start_with_seeds(source,profile,&[],0).await}
    pub async fn start_with_seeds(source:&[u8],profile:&str,seeds:&[&str],brief_budget:u32)->Self {
        let db=DisposableDatabase::start().await;db.migrate().await;
        let model=Arc::new(domain::model().unwrap());
        let store=GenerationStore::install(db.owner.clone(),model.clone()).await.unwrap();
        let dir=tempfile::tempdir().unwrap();input_source(dir.path(),source);db.write_configs(dir.path()).unwrap();
        write(&dir.path().join("libraries/demo/analytics.toml"),format!("version = 1\n[subsystem]\nmodule_prefixes = ['demo']\npublic_roots = ['demo']\n[seeds]\nprimary = {}\ndistractors = []\n[pass_a]\nmax_depth = 4\nmax_vertices = 256\nmax_edges = 1024\nmax_witnesses = 4\n[briefs]\nbudget = {brief_budget}\n",serde_json::to_string(seeds).unwrap()));
        let output=command(dir.path(),&dir.path().join("postgres.json"),"catalog",profile).output().unwrap();
        assert!(output.status.success(),"{}",String::from_utf8_lossy(&output.stderr));
        let report:serde_json::Value=serde_json::from_slice(&output.stdout).unwrap();
        let generation=GenerationId::from_hex(report["generation"].as_str().unwrap()).unwrap();
        let config=lctx_postgres::roles::RoleConfig::load(&dir.path().join("postgres-serving.json")).unwrap();
        let service=GenerationService::admit(model,&config,Some(generation)).await.unwrap();
        let catalog=CatalogService::prepare(service.clone()).await.unwrap();
        Self{db,store,service,catalog,generation,dir}
    }
    pub async fn members(&self)->Vec<OperationCandidate>{let execution=self.service.execution().await.unwrap();self.catalog.find(&execution,&FindOperationsRequest{library:Name::new("demo").unwrap(),selection:SelectionInput::default(),page:PageRequest{size:100,..Default::default()}}).await.unwrap().supported.items}
    pub async fn finish(self){self.service.shutdown().await.unwrap();self.store.retire(self.generation).await.unwrap();}
}
pub fn path(value:&str)->OperationSelector{OperationSelector::PublicPath{path:value.split('.').map(|s|Name::new(s).unwrap()).collect()}}
