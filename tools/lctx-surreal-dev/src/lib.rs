//! Development-only schema assets. No product installation/admission authority is exported.
use anyhow::{Context, Result, bail};
use std::collections::BTreeSet;
use std::path::Path;
pub mod catalog;
pub mod sample;
pub mod worker;

/// Internal provenance of this compiled worker; never connects to a service.
pub fn build_info() -> Result<serde_json::Value> {
    check_embedded_authority()?;
    Ok(serde_json::json!({
        "format": 1,
        "kind": "lctx-surreal-dev-build-info",
        "worker_version": env!("CARGO_PKG_VERSION"),
        "dependencies": serde_json::from_str::<serde_json::Value>(include_str!(concat!(env!("OUT_DIR"), "/build-info.json")))?,
        "schema": {
            "source": "compiled-embedded-native-schema",
            "profile_identity": source_identity(),
            "authority_correspondence": "passed",
            "modules": MODULE_ORDER,
        },
        "server": {"status": "not_inspected"},
    }))
}

surrealkit::embed_schema!(
    control = "schema/control",
    canonical = "schema/canonical",
    search = "schema/search",
    compiler = "schema/compiler",
    completion = "schema/completion",
);

pub fn authoritative_assets() -> Result<Vec<lctx_native::installation::SchemaAsset>> {
    lctx_native::installation::schema_assets(&lctx_native::materialization::native_definitions())
        .map_err(anyhow::Error::new)
}

/// Compare compiled authorities without requiring a checkout at runtime.
pub fn check_embedded_authority() -> Result<()> {
    let expected = authoritative_assets()?;
    let embedded: Vec<_> = embedded_modules()
        .iter()
        .flat_map(|(module, files)| files.iter().map(move |file| (*module, file.path, file.sql)))
        .collect();
    if expected.len() != embedded.len()
        || expected
            .iter()
            .zip(embedded)
            .any(|(asset, (module, path, sql))| {
                (asset.module, asset.path, asset.sql.as_str()) != (module, path, sql)
            })
    {
        bail!(
            "compiled generated assets differ from native authority; regenerate and rebuild this development worker"
        );
    }
    Ok(())
}

pub const MODULE_ORDER: &[&str] = &["control", "canonical", "search", "compiler", "completion"];

/// Dependency-closed responsibility selection. The returned order is native installation order.
pub fn selected_modules(requested: &[String]) -> Result<Vec<&'static str>> {
    let mut wanted = BTreeSet::new();
    fn include(name: &str, wanted: &mut BTreeSet<&'static str>) -> Result<()> {
        match name {
            "control" => {
                wanted.insert("control");
            }
            "canonical" => {
                include("control", wanted)?;
                wanted.insert("canonical");
            }
            "search" => {
                include("canonical", wanted)?;
                wanted.insert("search");
            }
            "compiler" => {
                include("canonical", wanted)?;
                wanted.insert("compiler");
            }
            "completion" => {
                include("compiler", wanted)?;
                wanted.insert("completion");
            }
            _ => bail!("unsupported schema responsibility module: {name}"),
        }
        Ok(())
    }
    if requested.is_empty() {
        wanted.extend(MODULE_ORDER.iter().copied());
    } else {
        for name in requested {
            include(name, &mut wanted)?;
        }
    }
    Ok(MODULE_ORDER
        .iter()
        .copied()
        .filter(|name| wanted.contains(name))
        .collect())
}

pub fn embedded_modules() -> &'static [(&'static str, &'static [surrealkit::EmbeddedSchemaFile])] {
    embedded_schema::MODULES
}

/// Materialize the beta tester's default-module projection into a parent-owned project.
/// This does not create a target, choose credentials, or invoke database effects.
pub fn project(root: &Path, requested: &[String]) -> Result<()> {
    check_embedded_authority()?;
    let modules = selected_modules(requested)?;
    let schema = root.join("schema");
    if schema.exists() && std::fs::symlink_metadata(&schema)?.file_type().is_symlink() {
        bail!("test projection schema directory cannot be a symlink");
    }
    if schema.exists() && std::fs::read_dir(&schema)?.next().is_some() {
        bail!(
            "test projection requires an empty schema directory: {}",
            schema.display()
        );
    }
    std::fs::create_dir_all(&schema)?;
    for (index, module) in modules.iter().enumerate() {
        let (_, files) = embedded_modules()
            .iter()
            .find(|(name, _)| name == module)
            .context("embedded module inventory")?;
        for (file_index, file) in files.iter().enumerate() {
            std::fs::write(
                schema.join(format!("{index:03}_{module}_{file_index:03}.surql")),
                file.sql,
            )?;
        }
    }
    let manifest = Projection {
        modules: modules.iter().map(|module| (*module).into()).collect(),
        source: source_identity(),
    };
    std::fs::write(
        root.join("native-projection.json"),
        serde_json::to_vec_pretty(&manifest)?,
    )?;
    Ok(())
}

#[derive(serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Projection {
    pub modules: Vec<String>,
    pub source: String,
}

pub fn source_identity() -> String {
    let mut hash = blake3::Hasher::new();
    for (module, files) in embedded_modules() {
        for file in *files {
            for part in [*module, file.path, file.sql] {
                hash.update(&(part.len() as u64).to_le_bytes());
                hash.update(part.as_bytes());
            }
        }
    }
    hash.finalize().to_hex().to_string()
}

pub fn read_projection(root: &Path) -> Result<Projection> {
    let projection: Projection =
        serde_json::from_slice(&std::fs::read(root.join("native-projection.json"))?)?;
    let modules = selected_modules(&projection.modules)?;
    if projection.source != source_identity()
        || modules
            .iter()
            .copied()
            .ne(projection.modules.iter().map(String::as_str))
    {
        bail!(
            "test projection differs from the current embedded responsibility inventory; regenerate it"
        );
    }
    let expected = modules
        .iter()
        .enumerate()
        .flat_map(|(index, module)| {
            embedded_modules()
                .iter()
                .find(|(name, _)| name == module)
                .into_iter()
                .flat_map(move |(_, files)| {
                    files.iter().enumerate().map(move |(file_index, file)| {
                        (
                            root.join(format!("schema/{index:03}_{module}_{file_index:03}.surql")),
                            file.sql,
                        )
                    })
                })
        })
        .collect::<BTreeSet<_>>();
    let mut actual = BTreeSet::new();
    collect_surql(&root.join("schema"), &mut actual)?;
    if actual != expected.iter().map(|(path, _)| path.clone()).collect() {
        bail!("test projection file inventory differs");
    }
    for (path, sql) in expected {
        if std::fs::read_to_string(path)? != sql {
            bail!("test projection content differs");
        }
    }
    Ok(projection)
}

/// Explicit generator for this component's derived schema directory. Never reads a database.
pub fn generate(root: &Path, check: bool) -> Result<()> {
    let assets = authoritative_assets()?;
    let expected: BTreeSet<_> = assets.iter().map(|asset| root.join(asset.path)).collect();
    let mut actual = BTreeSet::new();
    collect_surql(&root.join("schema"), &mut actual)?;
    let owner = root.join("schema/.lctx-generator");
    let marker = "lctx-surreal-dev/schema/v1\n";
    let owned = std::fs::read_to_string(&owner).ok().as_deref() == Some(marker);
    if !owned && (!actual.is_empty() || check) {
        bail!(
            "schema directory is not owned by this generator: {}",
            root.join("schema").display()
        );
    }
    if check {
        if actual != expected {
            bail!("generated schema file inventory is stale; run lctx-surreal-dev generate");
        }
        for asset in assets {
            let path = root.join(asset.path);
            if std::fs::read_to_string(&path).with_context(|| path.display().to_string())?
                != asset.sql
            {
                bail!(
                    "generated schema content is stale: {}; run lctx-surreal-dev generate",
                    asset.path
                );
            }
        }
    } else {
        std::fs::create_dir_all(root.join("schema"))?;
        std::fs::write(owner, marker)?;
        for path in actual.difference(&expected) {
            std::fs::remove_file(path)?;
        }
        for asset in assets {
            let path = root.join(asset.path);
            std::fs::create_dir_all(path.parent().context("schema asset parent")?)?;
            std::fs::write(path, asset.sql)?;
        }
    }
    Ok(())
}

fn collect_surql(root: &Path, out: &mut BTreeSet<std::path::PathBuf>) -> Result<()> {
    if !root.exists() {
        return Ok(());
    }
    if std::fs::symlink_metadata(root)?.file_type().is_symlink() {
        bail!(
            "generated schema does not accept symlinks: {}",
            root.display()
        );
    }
    for entry in std::fs::read_dir(root)? {
        let entry = entry?;
        let kind = entry.file_type()?;
        if kind.is_symlink() {
            bail!(
                "generated schema does not accept symlinks: {}",
                entry.path().display()
            );
        }
        if kind.is_dir() {
            collect_surql(&entry.path(), out)?;
        } else if entry.path().extension().is_some_and(|ext| ext == "surql") {
            out.insert(entry.path());
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use surrealdb::types::ToSql;
    use surrealdb_sql::{
        Expr, TopLevelExpr,
        statements::{DefineStatement, define::DefineKind},
    };

    #[test]
    fn build_info_matches_resolved_versions_and_compiled_schema_without_connection() {
        let info = build_info().unwrap();
        let lock: toml::Value = toml::from_str(include_str!("../../../Cargo.lock")).unwrap();
        for name in ["surrealkit", "surrealdb", "surrealdb-rpc"] {
            let package = lock["package"]
                .as_array()
                .unwrap()
                .iter()
                .find(|package| package["name"].as_str() == Some(name))
                .unwrap();
            assert_eq!(
                info["dependencies"][name]["version"].as_str(),
                package["version"].as_str()
            );
            let profile = &info["dependencies"][name]["patch_profile"];
            let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
            let manifest = std::fs::read(root.join(profile["manifest"].as_str().unwrap())).unwrap();
            assert_eq!(
                profile["manifest_blake3"],
                blake3::hash(&manifest).to_hex().to_string()
            );
            let documentation =
                std::fs::read(root.join(profile["documentation"].as_str().unwrap())).unwrap();
            assert_eq!(
                profile["documentation_blake3"],
                blake3::hash(&documentation).to_hex().to_string()
            );
            assert_eq!(profile["status"], "declared_build_input");
        }
        assert_eq!(info["schema"]["profile_identity"], source_identity());
        assert_eq!(info["schema"]["modules"], serde_json::json!(MODULE_ORDER));
        assert_eq!(info["schema"]["authority_correspondence"], "passed");
        assert_eq!(info["server"]["status"], "not_inspected");
    }

    #[test]
    fn generated_inventory_and_embedded_bytes_equal_native_authority() {
        check_embedded_authority().unwrap();
        generate(Path::new(env!("CARGO_MANIFEST_DIR")), true).unwrap();
        let expected = authoritative_assets().unwrap();
        let embedded: Vec<_> = embedded_modules()
            .iter()
            .flat_map(|(module, files)| {
                files.iter().map(move |file| (*module, file.path, file.sql))
            })
            .collect();
        assert_eq!(expected.len(), embedded.len());
        for (asset, (module, path, sql)) in expected.iter().zip(embedded) {
            assert_eq!(
                (asset.module, asset.path, asset.sql.as_str()),
                (module, path, sql)
            );
        }
    }

    #[test]
    fn dependency_projection_is_ordered_and_refuses_unknown_modules() {
        assert_eq!(
            selected_modules(&["completion".into()]).unwrap(),
            ["control", "canonical", "compiler", "completion"]
        );
        assert_eq!(
            selected_modules(&["search".into(), "compiler".into(), "search".into()]).unwrap(),
            ["control", "canonical", "search", "compiler"]
        );
        assert!(selected_modules(&["foreign".into()]).is_err());
        let dir = tempfile::tempdir().unwrap();
        project(dir.path(), &["completion".into()]).unwrap();
        let mut files = BTreeSet::new();
        collect_surql(&dir.path().join("schema"), &mut files).unwrap();
        assert_eq!(files.len(), 4);
        assert!(project(dir.path(), &[]).is_err());
    }

    #[test]
    fn generator_detects_addition_removal_and_content_drift() {
        let dir = tempfile::tempdir().unwrap();
        generate(dir.path(), false).unwrap();
        generate(dir.path(), true).unwrap();
        let stale = dir.path().join("schema/control/stale.surql");
        std::fs::write(&stale, "DEFINE TABLE stale;").unwrap();
        assert!(generate(dir.path(), true).is_err());
        generate(dir.path(), false).unwrap();
        assert!(!stale.exists());
        let first = dir.path().join(authoritative_assets().unwrap()[0].path);
        std::fs::remove_file(&first).unwrap();
        assert!(generate(dir.path(), true).is_err());
        generate(dir.path(), false).unwrap();
        std::fs::write(&first, "DEFINE TABLE drift;").unwrap();
        assert!(generate(dir.path(), true).is_err());
    }

    #[test]
    fn projection_refuses_content_drift_and_incomplete_sample_scope() {
        let dir = tempfile::tempdir().unwrap();
        project(dir.path(), &["compiler".into()]).unwrap();
        assert!(sample::write(dir.path()).is_err());
        assert!(!dir.path().join("tests").exists());
        read_projection(dir.path()).unwrap();
        std::fs::write(
            dir.path().join("schema/000_control_000.surql"),
            "DEFINE TABLE drift;",
        )
        .unwrap();
        assert!(read_projection(dir.path()).is_err());
        let dir = tempfile::tempdir().unwrap();
        project(dir.path(), &["completion".into()]).unwrap();
        sample::write(dir.path()).unwrap();
    }

    fn normalized(sql: &str) -> Vec<String> {
        surrealdb_syn::parse(sql)
            .unwrap()
            .expressions
            .into_iter()
            .map(|mut expression| {
                if let TopLevelExpr::Expr(Expr::Define(definition)) = &mut expression {
                    match definition.as_mut() {
                        DefineStatement::Table(value) => value.kind = DefineKind::Default,
                        DefineStatement::Field(value) => value.kind = DefineKind::Default,
                        DefineStatement::Index(value) => value.kind = DefineKind::Default,
                        DefineStatement::Analyzer(value) => value.kind = DefineKind::Default,
                        DefineStatement::Function(value) => value.kind = DefineKind::Default,
                        _ => panic!("unexpected native definition"),
                    }
                }
                expression.to_sql()
            })
            .collect()
    }

    #[test]
    fn beta_parser_and_overwrite_preserve_complete_native_statement_bodies() {
        for asset in authoritative_assets().unwrap() {
            let file = surrealkit::schema_state::SchemaFile {
                path: asset.path.into(),
                sql: asset.sql.clone(),
                hash: "comparison-only".into(),
            };
            let (entities, operations) =
                surrealkit::schema_state::parse_schema_statements(&file, false).unwrap();
            assert!(operations.is_empty());
            let expected = normalized(&asset.sql);
            assert_eq!(
                entities.len(),
                expected.len(),
                "{} complete parser inventory",
                asset.module
            );
            assert_eq!(
                normalized(&surrealkit::schema_state::ensure_overwrite(&asset.sql)),
                expected,
                "{} function bodies, nested statements and permissions",
                asset.module
            );
        }
    }
}
