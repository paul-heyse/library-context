//! Stage S2 of the library utilization catalog: which library item does each workspace reference
//! resolve to?
//!
//! Loads the Cargo workspace into rust-analyzer's database with no build (no build scripts, no
//! proc-macro server, no cache priming), then resolves every path and method call in the
//! workspace's own source files and prints one JSON line per (file, resolved library item):
//!
//! ```text
//! {"file":"crates/x/src/a.rs","package":"petgraph","version":"0.8.3",
//!  "path":"petgraph::graph::Graph","via":"path","lines":[12,40]}
//! ```
//!
//! Only items defined in a non-workspace library crate are printed. A path that names a
//! re-export resolves to the defining crate and module, so `arrow_schema::Schema` and
//! `datafusion::arrow::datatypes::Schema` both come out as `arrow_schema::schema::Schema`. A method
//! is `Type::method` (or `Trait::method` for a trait's method). References inside macro
//! expansions and derive output are not visited (the proc-macro server is off by design).
//!
//! Usage: lu-resolve [WORKSPACE_DIR]   (default: the current directory)
//!
//! Every path and method call is resolved: the run takes a couple of minutes and several GB, the
//! price of type-checking each function body a method call sits in. Completeness over speed.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use ra_ap_base_db::CrateOrigin;
use ra_ap_hir::{AsAssocItem, AssocItemContainer, ModuleDef, PathResolution, Semantics};
use ra_ap_ide_db::RootDatabase;
use ra_ap_load_cargo::{LoadCargoConfig, ProcMacroServerChoice, load_workspace_at};
use ra_ap_project_model::{CargoConfig, CargoFeatures, RustLibSource};
use ra_ap_syntax::{AstNode, ast};
use ra_ap_vfs::Vfs;

const SKIP_PARTS: [&str; 3] = ["/target/", "/third_party/", "/.sqlx/"];

/// A library item a reference resolved to.
struct Item {
    package: String,
    version: Option<String>,
    path: String,
}

#[derive(PartialEq, Eq, PartialOrd, Ord)]
struct Key {
    file: String,
    package: String,
    version: String,
    path: String,
    via: &'static str,
}

/// The library item a definition is, if it is defined in a non-workspace library crate. `method`
/// names an associated function of `def` (an ADT or a trait).
fn item_at(db: &RootDatabase, def: ModuleDef, method: Option<&str>) -> Option<Item> {
    let krate = match def {
        ModuleDef::Module(module) => module.krate(db),
        other => other.module(db)?.krate(db),
    };
    let CrateOrigin::Library { name, .. } = krate.origin(db) else {
        return None;
    };
    let path = def.canonical_path(db, krate.edition(db))?;
    let path = match method {
        Some(method) => format!("{path}::{method}"),
        None => path,
    };
    // `canonical_path` starts below the crate root and may keep `r#` on keyword segments; write
    // the path the way the skills' indexes do.
    let path = path.replace("r#", "");
    let crate_name = name.as_str().replace('-', "_");
    let path = if path.is_empty() { crate_name } else { format!("{crate_name}::{path}") };
    Some(Item { package: name.as_str().to_owned(), version: krate.version(db), path })
}

/// Every library item a definition stands for. A method that implements a trait method is both
/// `Type::method` and `Trait::method`, and both are reported: a catalog records whichever level
/// the code is written against.
fn library_items(db: &RootDatabase, def: ModuleDef) -> Vec<Item> {
    let ModuleDef::Function(function) = def else {
        return item_at(db, def, None).into_iter().collect();
    };
    let edition = function.module(db).krate(db).edition(db);
    let name = function.name(db).display(db, edition).to_string();
    let owners: Vec<ModuleDef> = match function.as_assoc_item(db).map(|item| item.container(db)) {
        Some(AssocItemContainer::Impl(imp)) => {
            let own = imp.self_ty(db).as_adt().map(ModuleDef::Adt);
            let of_trait = imp.trait_(db).map(ModuleDef::Trait);
            own.into_iter().chain(of_trait).collect()
        }
        Some(AssocItemContainer::Trait(t)) => vec![ModuleDef::Trait(t)],
        None => Vec::new(),
    };
    if owners.is_empty() {
        return item_at(db, def, None).into_iter().collect();
    }
    owners.into_iter().filter_map(|owner| item_at(db, owner, Some(&name))).collect()
}

fn line_starts(text: &str) -> Vec<usize> {
    let mut starts = vec![0];
    starts.extend(text.bytes().enumerate().filter(|(_, b)| *b == b'\n').map(|(i, _)| i + 1));
    starts
}

/// Every library item each workspace file references, with the lines; plus the file and node counts.
fn resolve_all(
    db: &RootDatabase,
    vfs: &Vfs,
    root: &Path,
) -> (BTreeMap<Key, BTreeSet<usize>>, usize, usize) {
    let sema = Semantics::new(db);
    let mut found: BTreeMap<Key, BTreeSet<usize>> = BTreeMap::new();
    let (mut files, mut visited) = (0usize, 0usize);
    let debug = std::env::var("LU_DEBUG_FILE").ok();
    for (file_id, path) in vfs.iter() {
        let Some(abs) = path.as_path() else { continue };
        let abs = Path::new(abs.as_str());
        if abs.extension().is_none_or(|e| e != "rs") || !abs.starts_with(root) {
            continue;
        }
        let shown = abs.to_string_lossy();
        if SKIP_PARTS.iter().any(|part| shown.contains(part)) {
            continue;
        }
        let Some(module) = sema.file_to_module_def(file_id) else { continue };
        if !matches!(module.krate(db).origin(db), CrateOrigin::Local { .. }) {
            continue;
        }
        files += 1;
        let file = shown[root.to_string_lossy().len() + 1..].to_owned();
        let source = sema.parse_guess_edition(file_id);
        let text = source.syntax().text().to_string();
        let starts = line_starts(&text);
        let mut record = |via: &'static str, item: Item, offset: usize| {
            let line = starts.partition_point(|&s| s <= offset);
            let key = Key {
                file: file.clone(),
                package: item.package,
                version: item.version.unwrap_or_default(),
                path: item.path,
                via,
            };
            found.entry(key).or_default().insert(line);
        };
        for node in source.syntax().descendants() {
            let offset = usize::from(node.text_range().start());
            if let Some(path) = ast::Path::cast(node.clone()) {
                visited += 1;
                let resolution = sema.resolve_path(&path);
                if debug.as_deref().is_some_and(|d| file.ends_with(d)) {
                    let what = match &resolution {
                        Some(PathResolution::Def(def)) => format!("def {:?}", library_items(db, *def).into_iter().map(|i| i.path).collect::<Vec<_>>()),
                        Some(_) => "non-def".to_owned(),
                        None => "UNRESOLVED".to_owned(),
                    };
                    eprintln!("debug {}:{} {} -> {what}", file, starts.partition_point(|&s| s <= offset), path.syntax().text());
                }
                if let Some(PathResolution::Def(def)) = resolution {
                    for item in library_items(db, def) {
                        record("path", item, offset);
                    }
                }
            } else if let Some(call) = ast::MethodCallExpr::cast(node) {
                visited += 1;
                if let Some(function) = sema.resolve_method_call(&call) {
                    for item in library_items(db, ModuleDef::Function(function)) {
                        record("method", item, offset);
                    }
                }
            }
        }
    }
    (found, files, visited)
}

fn main() -> Result<()> {
    let root: PathBuf = std::env::args().nth(1).map(PathBuf::from).unwrap_or_else(|| ".".into());
    let root = root.canonicalize().with_context(|| format!("no such directory {}", root.display()))?;
    let cargo = CargoConfig {
        all_targets: true,
        set_test: true,
        features: CargoFeatures::All,
        sysroot: Some(RustLibSource::Discover),
        ..Default::default()
    };
    let load = LoadCargoConfig {
        load_out_dirs_from_check: false,
        with_proc_macro_server: ProcMacroServerChoice::None,
        prefill_caches: false,
        num_worker_threads: 4,
        proc_macro_processes: 1,
    };
    let (db, vfs, _) = load_workspace_at(&root, &cargo, &load, &|m| eprintln!("lu-resolve: {m}"))?;
    let (found, files, visited) = ra_ap_hir::attach_db(&db, || resolve_all(&db, &vfs, &root));
    eprintln!("lu-resolve: {files} files, {visited} nodes resolved, {} library items", found.len());
    for (key, lines) in found {
        let lines: Vec<usize> = lines.into_iter().take(50).collect();
        println!(
            "{}",
            serde_json::json!({
                "file": key.file, "package": key.package, "version": key.version,
                "path": key.path, "via": key.via, "lines": lines,
            })
        );
    }
    Ok(())
}
