//! Independent semantic navigation over a tiny hermetic original-source universe.
use ruff_db::{files::system_path_to_file, system::{InMemorySystem, SystemPathBuf}};
use ruff_ranged_value::ValueSource;
use ruff_text_size_latest::TextSize;
use std::{collections::BTreeSet, process::{Command, Stdio}, time::{Duration, Instant}};
use ty_project::{Db, ProjectDatabase, ProjectMetadata, metadata::Options};

const API: &str = include_str!("../../../fixtures/python/python_reference_oracle/api.py");
const USES: &str = include_str!("../../../fixtures/python/python_reference_oracle/uses.py");
const OMITTED: &str = include_str!("../../../fixtures/python/python_reference_oracle/omitted.py");

fn database(include_uses: bool, reverse: bool) -> ProjectDatabase {
    let system = InMemorySystem::default();
    let root = SystemPathBuf::from("/captured");
    system.fs().write_files_all([
        (root.join("api.py"), API), (root.join("uses.py"), USES),
        (root.join("omitted.py"), OMITTED),
    ]).unwrap();
    let mut metadata = ProjectMetadata::new("reference-oracle", root.clone());
    metadata.set_override_options(Options::from_toml_str(
        "[environment]\npython-version = '3.14'\npython-platform = 'linux'\nroot = ['/captured']\n[src]\nrespect-ignore-files = false\nexclude = []\n",
        ValueSource::Cli,
    ).unwrap());
    let mut db = ProjectDatabase::fallible(metadata, system).unwrap();
    let mut paths = vec![root.join("api.py")];
    if include_uses { paths.push(root.join("uses.py")); }
    if reverse { paths.reverse(); }
    db.project().set_included_paths(&mut db, paths.clone());
    let actual = db.project().files(&db).iter().map(|f| f.path(&db).as_system_path().unwrap().to_path_buf()).collect::<BTreeSet<_>>();
    assert_eq!(actual, paths.into_iter().collect());
    db
}

fn references(db: &ProjectDatabase, file_name: &str, offset: usize, include_declaration: bool)
    -> Option<BTreeSet<(String, usize, usize, String)>>
{
    let file = system_path_to_file(db, SystemPathBuf::from(format!("/captured/{file_name}"))).unwrap();
    ty_ide::find_references(db, db.project().program(db).program_file(db, file),
        TextSize::try_from(offset).unwrap(), include_declaration).map(|rows| {
            rows.into_iter().map(|row| {
                let path = row.file().path(db).as_system_path().unwrap().to_string();
                // Results outside the declared captured universe are a changed universe,
                // never silently admitted as another supported local reference.
                assert!(db.project().files(db).contains(row.file()), "foreign oracle result: {path}");
                (path, row.range().start().to_usize(), row.range().end().to_usize(), format!("{:?}", row.kind()))
            }).collect()
        })
}

#[test]
fn ty_reference_oracle_is_bounded_by_a_joined_process() {
    let mut child = Command::new(std::env::current_exe().unwrap())
        .args(["--exact", "ty_reference_oracle_child", "--ignored", "--nocapture"])
        .env("LCTX_TY_REFERENCE_ORACLE_CHILD", "1")
        .stdin(Stdio::null()).spawn().unwrap();
    let deadline = Instant::now() + Duration::from_secs(60);
    loop {
        if let Some(status) = child.try_wait().unwrap() {
            assert!(status.success(), "native ty reference controls failed: {status}");
            break;
        }
        if Instant::now() >= deadline {
            child.kill().unwrap();
            child.wait().unwrap();
            panic!("native ty reference oracle exceeded its 60-second process bound");
        }
        std::thread::sleep(Duration::from_millis(50));
    }
}

#[test]
#[ignore = "Invoked in a joined process by the bounded parent test"]
fn ty_reference_oracle_child() {
    assert_eq!(std::env::var("LCTX_TY_REFERENCE_ORACLE_CHILD").as_deref(), Ok("1"));
    let db = database(true, false);
    let operation = API.find("operation(value").unwrap();
    let uses = references(&db, "api.py", operation, false).unwrap();
    assert!(uses.iter().any(|r| r.0.ends_with("uses.py")), "resolved import alias must remain in the captured universe");
    assert!(!uses.iter().any(|r| r.0.ends_with("omitted.py")));
    assert!(!uses.iter().any(|r| r.1 == API.find("operation):").unwrap()), "shadowed formal is another identity");
    let with_declaration = references(&db, "api.py", operation, true).unwrap();
    assert!(with_declaration.len() > uses.len());
    assert!(with_declaration.iter().any(|r| r.1 == operation && r.3 == "Other"));
    assert_eq!(uses, references(&database(true, true), "api.py", operation, false).unwrap());
    let smaller = references(&database(false, false), "api.py", operation, false).unwrap();
    assert!(!smaller.iter().any(|r| r.0.ends_with("uses.py")));
    assert!(smaller.len() < uses.len(), "omitting a captured module changes the explicit search universe");
    let unused = API.find("unused():").unwrap();
    assert_eq!(references(&db, "api.py", unused, false).unwrap().len(), 0);
    assert!(references(&db, "api.py", API.find("pass").unwrap(), false).is_none());
    let parameter = API.find("value: int").unwrap();
    let parameters = references(&db, "api.py", parameter, false).unwrap();
    assert!(parameters.iter().any(|r| r.0.ends_with("api.py") && &API[r.1..r.2] == "value" && r.1 == API.find("value=1").unwrap()), "keyword label is a native semantic reference, outside the initial lexical-name product");
    let property = API.find("property_value(self)").unwrap();
    assert!(references(&db, "api.py", property, false).unwrap().iter().any(|r| r.1 == API.rfind("property_value").unwrap()));
    println!("policy=ResolveAliases; source_view=original; python=3.14; platform=linux; included=api.py,uses.py; dependencies=vendored-typeshed; provider=f7bdff69e1fb94ab0ed5b340e977aac0d26e9301; lexical-overlap=value names; scope-differences=keyword/member references");
}
