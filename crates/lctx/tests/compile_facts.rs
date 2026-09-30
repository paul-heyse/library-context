//! The binary's facts frontier over a preprovisioned locked input and real disposable PG18.
//! Acquisition is a no-op command here; semantic producers, capture, admission and store are real.
use lctx_postgres::{
    generations::{GenerationCatalog, GenerationStore, ListFilter},
    testing::DisposableDatabase,
};
use std::{path::Path, process::Command, sync::Arc};
fn write(path: &Path, bytes: impl AsRef<[u8]>) {
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, bytes).unwrap();
}
fn input(root: &Path) {
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
    let source = b"def api(x: int) -> int:\n    if x:\n        return x\n    return 0\n";
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
fn command(root: &Path, cfg: &Path, through: &str, profile: &str) -> Command {
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
#[tokio::test]
async fn binary_publishes_facts_reports_profiles_and_does_not_select() {
    let db = DisposableDatabase::start().await;
    db.migrate().await;
    let store = GenerationStore::install(
        db.owner.clone(),
        Arc::new(lctx_model::domain::model().unwrap()),
    )
    .await
    .unwrap();
    let catalog = GenerationCatalog::new(db.reader.clone());
    let dir = tempfile::tempdir().unwrap();
    input(dir.path());
    db.write_configs(dir.path()).unwrap();
    let cfg = dir.path().join("postgres.json");
    let refused = command(dir.path(), &cfg, "analysis", "catalog")
        .output()
        .unwrap();
    assert_eq!(refused.status.code(), Some(3));
    assert!(
        catalog
            .list(&ListFilter::default())
            .await
            .unwrap()
            .is_empty()
    );
    let mut behavior = None;
    for (through, profile) in [
        ("facts", "catalog"),
        ("normalized", "catalog"),
        ("facts", "behavioral"),
        ("normalized", "behavioral"),
        ("normalized", "behavioral"),
    ] {
        let output = command(dir.path(), &cfg, through, profile)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let report: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
        assert_eq!(report["frontier"], through);
        assert_eq!(report["profile"], profile);
        assert_eq!(report["selected"], false);
        assert_eq!(
            report["stage_measurements"].as_array().unwrap().len(),
            (if profile == "catalog" { 5 } else { 6 })
                + if through == "normalized" { 7 } else { 0 }
        );
        let flow = report["families"]
            .as_array()
            .unwrap()
            .iter()
            .find(|f| f["family"] == "Flow")
            .unwrap();
        assert_eq!(flow["availability"] == "NotRequested", profile == "catalog");
        if profile == "behavioral" && through == "normalized" {
            if let Some(expected) = behavior.as_ref() {
                assert_eq!(&report["content_digest"], expected);
            } else {
                behavior = Some(report["content_digest"].clone());
            }
        }
    }
    let listed = catalog.list(&ListFilter::default()).await.unwrap();
    assert_eq!(listed.len(), 5);
    assert!(listed.iter().all(|g| !g.selected));
    for g in listed {
        store.retire(g.id).await.unwrap();
    }
    let exhausted = command(dir.path(), &cfg, "normalized", "catalog")
        .args(["--memory-bytes", "65536"])
        .output()
        .unwrap();
    assert!(!exhausted.status.success());
    assert!(
        catalog
            .list(&ListFilter::default())
            .await
            .unwrap()
            .is_empty()
    );
    assert!(
        GenerationStore::check(&db.owner, store.model())
            .await
            .unwrap()
            .clean()
    );
}
