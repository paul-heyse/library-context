//! Native fixture CLI artifact journeys and refusal before acquisition.
//! Only uv acquisition is stubbed; captured package bytes, native providers and artifacts are real.
use std::process::Command;
#[test]
fn ordinary_compile_is_unavailable_before_acquisition() {
    let root=tempfile::tempdir().unwrap();
    let result=Command::new(env!("CARGO_BIN_EXE_lctx")).current_dir(root.path()).args(["compile","absent","--through","catalog"]).output().unwrap();
    assert_eq!(result.status.code(),Some(3));
    assert!(String::from_utf8_lossy(&result.stderr).contains("native publication is not implemented"));
    assert_eq!(std::fs::read_dir(root.path()).unwrap().count(),0);
}
#[test]
fn artifact_destination_is_required_and_existing_content_is_preserved() {
    let root=tempfile::tempdir().unwrap();let output=root.path().join("graph");
    std::fs::create_dir(&output).unwrap();std::fs::write(output.join("sentinel"),b"preserve").unwrap();
    let missing=Command::new(env!("CARGO_BIN_EXE_lctx")).args(["compile","absent","--through","facts","--artifact-only"]).output().unwrap();
    assert_eq!(missing.status.code(),Some(2));
    let existing=Command::new(env!("CARGO_BIN_EXE_lctx")).current_dir(root.path()).args(["compile","absent","--through","facts","--artifact-only","--output"]).arg(&output).output().unwrap();
    assert_eq!(existing.status.code(),Some(2));
    assert_eq!(std::fs::read(output.join("sentinel")).unwrap(),b"preserve");
    assert_eq!(std::fs::read_dir(root.path()).unwrap().count(),1);
}

fn installed_fixture(root: &std::path::Path) {
    use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
    use sha2::{Digest as _, Sha256};
    let library = root.join("libraries/demo");
    let environment = root.join("envs/demo");
    let site = environment.join("lib/python3.14/site-packages");
    let metadata = "Metadata-Version: 2.4\nName: demo\nVersion: 1.0\n";
    std::fs::create_dir_all(&library).unwrap();
    std::fs::create_dir_all(site.join("demo")).unwrap();
    std::fs::create_dir_all(site.join("demo-1.0.dist-info")).unwrap();
    std::fs::write(library.join("pyproject.toml"), "[project]\nname = \"lctx-library-demo\"\nversion = \"0\"\ndependencies = [\"demo==1.0\"]\n[tool.lctx]\nrelease = [\"demo\"]\n").unwrap();
    std::fs::write(library.join(".python-version"), "3.14.7\n").unwrap();
    // Acquired-layout fixture, as in cpg-extract's acquisition controls; no network installer runs.
    std::fs::write(library.join("uv.lock"), format!("version = 1\n[[package]]\nname = \"demo\"\nversion = \"1.0\"\nsource = {{ registry = \"https://example.invalid/simple\" }}\nwheels = [{{ url = \"https://example.invalid/demo.whl\", hash = \"sha256:{}\" }}]\n", "a".repeat(64))).unwrap();
    std::fs::write(environment.join("pyvenv.cfg"), "home = /fixture\nuv = 0.12.18\nversion_info = 3.14.7\n").unwrap();
    std::fs::write(site.join("demo/__init__.py"), SOURCE).unwrap();
    std::fs::write(site.join("demo-1.0.dist-info/METADATA"), metadata).unwrap();
    let record = [("demo/__init__.py", SOURCE), ("demo-1.0.dist-info/METADATA", metadata)].into_iter().map(|(path, bytes)| format!("{path},sha256={},{}\n", URL_SAFE_NO_PAD.encode(Sha256::digest(bytes.as_bytes())), bytes.len())).collect::<String>();
    std::fs::write(site.join("demo-1.0.dist-info/RECORD"), record + "demo-1.0.dist-info/RECORD,,\n").unwrap();
    std::fs::write(library.join("analytics.toml"), "version = 1\n[subsystem]\nmodule_prefixes = [\"demo\"]\npublic_roots = [\"demo\"]\n[seeds]\nprimary = []\ndistractors = []\n[pass_a]\nmax_depth = 2\nmax_vertices = 128\nmax_edges = 512\nmax_witnesses = 3\n[briefs]\nbudget = 0\n").unwrap();
    let uv = root.join("uv");
    std::fs::write(&uv, "#!/bin/sh\nprintf '%s\\n' \"$@\" > \"$STUB_OUT/uv-args\"\n").unwrap();
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(uv, std::fs::Permissions::from_mode(0o755)).unwrap();
}
const SOURCE: &str = "def api(value: int) -> int:\n    \"\"\"Return the original value — unchanged.\"\"\"\n    return value\n";
fn cli(root: &std::path::Path) -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_lctx"));
    command.current_dir(root).env("PATH", format!("{}:{}", root.display(), std::env::var("PATH").unwrap_or_default())).env("STUB_OUT", root);
    command.args(["--libraries", "libraries", "--envs", "envs", "--sources", "sources"]);
    command.env_remove("DATABASE_URL").env_remove("LCTX_DATABASE_CONFIG");
    for (name, _) in std::env::vars_os() {
        if cpg_extract::bundle::REFUSED_ENV.contains(&name.to_string_lossy().as_ref()) || name.to_string_lossy().starts_with(cpg_extract::bundle::REFUSED_ENV_PREFIX) { command.env_remove(name); }
    }
    command
}
#[test]
fn native_fixture_cli_exports_both_profiles_at_every_frontier() {
    use datafusion::arrow::{array::{BinaryArray, FixedSizeBinaryArray}, ipc::reader::FileReader};
    use lctx_model::domain::{ContentHash, graph::{Assertion, Entity, FamilyHasher, GraphFamily, Manifest}, stages::Profile, admission::Frontier};
    let directory = tempfile::tempdir().unwrap();
    let root = std::fs::canonicalize(directory.path()).unwrap();
    installed_fixture(&root);
    for profile in Profile::ALL {
        for frontier in [Frontier::Facts, Frontier::Normalized, Frontier::Analysis, Frontier::Catalog] {
            let destination = root.join(format!("{}-{}", profile.name(), frontier.name()));
            let mut command = cli(&root);
            command.args(["compile", "demo", "--artifact-only", "--output"]).arg(&destination).args(["--profile", profile.name(), "--through", frontier.name()]);
            if matches!(frontier, Frontier::Analysis | Frontier::Catalog) { command.args(["--techniques", "default", "--embedder", "none"]); }
            let result = command.output().unwrap();
            assert!(result.status.success(), "{} {}: {}", profile.name(), frontier.name(), String::from_utf8_lossy(&result.stderr));
            let response: serde_json::Value = serde_json::from_slice(&result.stdout).unwrap();
            let manifest: Manifest = serde_json::from_slice(&std::fs::read(destination.join("manifest.json")).unwrap()).unwrap();
            manifest.validate().unwrap();
            assert_eq!(manifest.profile, profile);
            assert_eq!(manifest.frontier, frontier);
            assert_eq!(response["profile"], profile.name());
            assert_eq!(response["frontier"], frontier.name());
            assert_eq!(response["content"], manifest.content().hex());
            assert_eq!(response["artifact"], destination.to_string_lossy().as_ref());
            assert_eq!(response["published"], false);
            assert!(manifest.embeddings.is_empty());
            let mut found_original = false;
            let mut found_api = false;
            for (file, family) in [("entities.arrow", GraphFamily::Entities), ("assertions.arrow", GraphFamily::Assertions)] {
                let reader = FileReader::try_new(std::fs::File::open(destination.join(file)).unwrap(), None).unwrap();
                let mut hasher = FamilyHasher::new(family);
                for batch in reader {
                    let batch = batch.unwrap();
                    let ids = batch.column(0).as_any().downcast_ref::<FixedSizeBinaryArray>().unwrap();
                    let contents = batch.column(1).as_any().downcast_ref::<FixedSizeBinaryArray>().unwrap();
                    let payloads = batch.column(2).as_any().downcast_ref::<BinaryArray>().unwrap();
                    for row in 0..batch.num_rows() {
                        let (id, content) = if family == GraphFamily::Entities {
                            let entity: Entity = serde_json::from_slice(payloads.value(row)).unwrap();
                            entity.validate().unwrap();
                            if let Entity::Source(source) = &entity { if source.path == "demo/__init__.py" { assert_eq!(source.content, ContentHash::of(SOURCE.as_bytes())); found_original = true; } }
                            if let Entity::CatalogMember(member) = &entity { if member.name == "api" && member.path == ["api"] { found_api = true; } }
                            (entity.id().0, entity.content())
                        } else {
                            let assertion: Assertion = serde_json::from_slice(payloads.value(row)).unwrap();
                            assertion.validate().unwrap();
                            (assertion.id().0, assertion.content())
                        };
                        assert_eq!(ids.value(row), id.0);
                        assert_eq!(contents.value(row), content.0);
                        assert!(hasher.push(id, content).unwrap());
                    }
                }
                assert_eq!(manifest.families.iter().find(|expected| expected.family == family).unwrap(), &hasher.finish());
            }
            assert!(found_original);
            if frontier == Frontier::Catalog { assert!(found_api, "mandatory catalog must contain demo.api without brief seeds"); }
            assert!(manifest.originals.iter().any(|original| std::fs::read(destination.join(format!("original-{}.bin", original.source.0.hex()))).unwrap() == SOURCE.as_bytes()));
            for original in &manifest.originals { let bytes = std::fs::read(destination.join(format!("original-{}.bin", original.source.0.hex()))).unwrap(); assert_eq!(bytes.len() as u64, original.byte_len); assert_eq!(ContentHash::of(&bytes), original.content); }
            let acquisition = std::fs::read_to_string(root.join("uv-args")).unwrap();
            assert!(acquisition.lines().any(|arg| arg == "--frozen"));
            assert!(acquisition.lines().any(|arg| arg == "--no-install-project"));
        }
    }
}
#[test]
fn artifact_frontier_and_lower_options_refuse_before_acquisition() {
    let directory = tempfile::tempdir().unwrap();
    let root = std::fs::canonicalize(directory.path()).unwrap();
    installed_fixture(&root);
    for (extra, code, message) in [(&["--through", "serving"][..], 3, "--through serving"), (&["--through", "facts", "--techniques", "default"][..], 1, "require --through")] {
        let destination = root.join("refused");
        let result = cli(&root).args(["compile", "demo", "--artifact-only", "--output"]).arg(&destination).args(extra).output().unwrap();
        assert_eq!(result.status.code(), Some(code), "{}", String::from_utf8_lossy(&result.stderr));
        assert!(String::from_utf8_lossy(&result.stderr).contains(message));
        assert!(!root.join("uv-args").exists());
        assert!(!destination.exists());
    }
}
