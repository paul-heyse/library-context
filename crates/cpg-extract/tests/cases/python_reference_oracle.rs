//! Independent semantic navigation over a tiny hermetic original-source universe.
use ruff_db::{
    files::system_path_to_file,
    system::{InMemorySystem, SystemPathBuf},
};
use ruff_ranged_value::ValueSource;
use ruff_text_size_latest::TextSize;
use std::{
    collections::BTreeSet,
    process::{Command, Stdio},
    time::{Duration, Instant},
};
use ty_project::{Db, ProjectDatabase, ProjectMetadata, metadata::Options};

const API: &str = include_str!("../../../../fixtures/python/python_reference_oracle/api.py");
const USES: &str = include_str!("../../../../fixtures/python/python_reference_oracle/uses.py");
const OMITTED: &str = include_str!("../../../../fixtures/python/python_reference_oracle/omitted.py");

fn database(include_uses: bool, reverse: bool) -> ProjectDatabase {
    let system = InMemorySystem::default();
    let root = SystemPathBuf::from("/captured");
    system
        .fs()
        .write_files_all([
            (root.join("api.py"), API),
            (root.join("uses.py"), USES),
            (root.join("omitted.py"), OMITTED),
        ])
        .unwrap();
    let mut metadata = ProjectMetadata::new("reference-oracle", root.clone());
    metadata.set_override_options(Options::from_toml_str(
        "[environment]\npython-version = '3.14'\npython-platform = 'linux'\nroot = ['/captured']\n[src]\nrespect-ignore-files = false\nexclude = []\n",
        ValueSource::Cli,
    ).unwrap());
    let mut db = ProjectDatabase::fallible(metadata, system).unwrap();
    let mut paths = vec![root.join("api.py")];
    if include_uses {
        paths.push(root.join("uses.py"));
    }
    if reverse {
        paths.reverse();
    }
    db.project().set_included_paths(&mut db, paths.clone());
    let actual = db
        .project()
        .files(&db)
        .iter()
        .map(|f| f.path(&db).as_system_path().unwrap().to_path_buf())
        .collect::<BTreeSet<_>>();
    assert_eq!(actual, paths.into_iter().collect());
    db
}

fn references(
    db: &ProjectDatabase,
    file_name: &str,
    offset: usize,
    include_declaration: bool,
) -> Option<BTreeSet<(String, usize, usize, String)>> {
    let file =
        system_path_to_file(db, SystemPathBuf::from(format!("/captured/{file_name}"))).unwrap();
    ty_ide::find_references(
        db,
        db.project().program(db).program_file(db, file),
        TextSize::try_from(offset).unwrap(),
        include_declaration,
    )
    .map(|rows| {
        rows.into_iter()
            .map(|row| {
                let path = row.file().path(db).as_system_path().unwrap().to_string();
                // Results outside the declared captured universe are a changed universe,
                // never silently admitted as another supported local reference.
                assert!(
                    db.project().files(db).contains(row.file()),
                    "foreign oracle result: {path}"
                );
                (
                    path,
                    row.range().start().to_usize(),
                    row.range().end().to_usize(),
                    format!("{:?}", row.kind()),
                )
            })
            .collect()
    })
}

#[test]
fn ty_reference_oracle_is_bounded_by_a_joined_process() {
    let mut child = Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            "ty_reference_oracle_child",
            "--ignored",
            "--nocapture",
        ])
        .env("LCTX_TY_REFERENCE_ORACLE_CHILD", "1")
        .stdin(Stdio::null())
        .spawn()
        .unwrap();
    let deadline = Instant::now() + Duration::from_secs(60);
    loop {
        if let Some(status) = child.try_wait().unwrap() {
            assert!(
                status.success(),
                "native ty reference controls failed: {status}"
            );
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
    assert_eq!(
        std::env::var("LCTX_TY_REFERENCE_ORACLE_CHILD").as_deref(),
        Ok("1")
    );
    let db = database(true, false);
    let operation = API.find("operation(value").unwrap();
    let uses = references(&db, "api.py", operation, false).unwrap();
    assert!(
        uses.iter().any(|r| r.0.ends_with("uses.py")),
        "same-spelling cross-module references must remain in the captured universe: {uses:?}"
    );
    // ResolveAliases resolves semantic definitions, but the pinned search also filters
    // by the requested spelling. It does not enumerate differently spelled alias uses.
    let aliased = USES.find("value_reference = alias").unwrap() + "value_reference = ".len();
    assert!(
        !uses
            .iter()
            .any(|r| r.0 == "/captured/uses.py" && r.1 == aliased)
    );
    let alias_query = references(&db, "uses.py", USES.find("alias").unwrap(), false).unwrap();
    assert!(
        alias_query
            .iter()
            .any(|r| r.0 == "/captured/uses.py" && r.1 == aliased),
        "same-spelling alias query must retain its resolved semantic target"
    );
    assert!(!uses.iter().any(|r| r.0.ends_with("omitted.py")));
    for label in ["typed_reference = ", "unreachable_reference = "] {
        let start = API.find(label).unwrap() + label.len();
        assert!(
            uses.iter()
                .any(|r| r.0 == "/captured/api.py" && r.1 == start),
            "native source references retain typing-only and unreachable reads without an execution claim: {label}: {uses:?}"
        );
    }
    assert!(
        !uses.iter().any(|r| r.1 == API.find("operation):").unwrap()),
        "shadowed formal is another identity"
    );
    let with_declaration = references(&db, "api.py", operation, true).unwrap();
    assert!(with_declaration.len() > uses.len());
    assert!(
        with_declaration
            .iter()
            .any(|r| r.1 == operation && r.3 == "Other")
    );
    assert_eq!(
        uses,
        references(&database(true, true), "api.py", operation, false).unwrap()
    );
    let smaller = references(&database(false, false), "api.py", operation, false).unwrap();
    assert!(!smaller.iter().any(|r| r.0.ends_with("uses.py")));
    assert!(
        smaller.len() < uses.len(),
        "omitting a captured module changes the explicit search universe"
    );
    let unused = API.find("unused():").unwrap();
    assert!(
        references(&db, "api.py", unused, false).is_none(),
        "the pinned public API returns None for an empty search, not Some(empty)"
    );
    assert_eq!(references(&db, "api.py", unused, true).unwrap().len(), 1);
    assert!(references(&db, "api.py", API.find("pass").unwrap(), false).is_none());
    let parameter = API.find("parameter_control(value").unwrap() + "parameter_control(".len();
    let parameters = references(&db, "api.py", parameter, false).unwrap();
    let keyword = API.find("parameter_control(value=2").unwrap() + "parameter_control(".len();
    assert!(
        parameters
            .iter()
            .any(|r| r.0.ends_with("api.py") && &API[r.1..r.2] == "value" && r.1 == keyword),
        "keyword label of a known callable is a native semantic reference, outside the initial lexical-name product: {parameters:?}"
    );
    let decorated = references(&db, "api.py", API.find("value: int").unwrap(), false).unwrap();
    assert!(
        !decorated
            .iter()
            .any(|r| r.0 == "/captured/api.py" && r.1 == API.find("value=1").unwrap()),
        "the untyped decorator does not retain a known call signature for keyword-origin lookup"
    );
    let property = API.find("property_value(self)").unwrap();
    assert!(
        references(&db, "api.py", property, false)
            .unwrap()
            .iter()
            .any(|r| r.1 == API.rfind("property_value").unwrap())
    );
    let copy = API.find("copy = value").unwrap();
    let copy_references = references(&db, "api.py", copy, false).unwrap();
    assert!(
        copy_references
            .iter()
            .any(|r| r.1 == API.find("copy +=").unwrap()),
        "augmented assignment must retain the native kind, not invented read/write duplicates"
    );
    compare_normalized_names(&db);
    println!(
        "policy=ResolveAliases with requested-spelling filter; empty-search=None; source_view=original; python=3.14; platform=linux; included=api.py,uses.py; dependencies=vendored-typeshed; provider=f7bdff69e1fb94ab0ed5b340e977aac0d26e9301; lexical-overlap=value names; scope-differences=keyword/member references"
    );
}

use crate::typed_driver;
use crate::inspector;
use lctx_model::domain::{
    normalized::{
        entities::{CallableEntity, EntityRef, ResolutionStatus},
        entity_normalization,
        links::ReferenceEntityTarget,
        relation_normalization,
    },
    *,
};
inspector!(ReferenceFacts => {
    let mut demand=typed_driver::ObservationDemand::selected();
    macro_rules! select { ($($field:ident:$ty:ty $(=> $family:ident)?,)*) => {$(demand.include::<$ty>();)*}; }
    lctx_model::normalized_entity_inputs!(select);
    lctx_model::normalized_relation_inputs!(select);
    demand.include::<lctx_model::domain::source::Occurrence>();
    demand.include::<source::SourceArtifact>();
    demand
});

/// Compare only nominally admitted original name reads, not ty's broader alias/member search.
fn compare_normalized_names(db: &ProjectDatabase) {
    let captured = std::collections::BTreeMap::from([
        ("api.py".to_owned(), API.as_bytes().to_vec()),
        ("uses.py".to_owned(), USES.as_bytes().to_vec()),
    ]);
    let tables = typed_driver::Tables::default();
    tokio::runtime::Runtime::new()
        .unwrap()
        .block_on(typed_driver::run(&captured, ReferenceFacts(tables.clone())))
        .unwrap();
    let budget = typed_driver::budget();
    let mut data = relation_normalization::RelationData::new(&budget);
    macro_rules! facts { ($($field:ident:$ty:ty => $family:ident,)*) => { $(for row in typed_driver::rows::<$ty>(&tables) {data.facts.$field.insert(row).unwrap();})* }; }
    lctx_model::normalized_entity_inputs!(facts);
    data.entities = entity_normalization::normalize(data.facts.inputs(), &budget).unwrap();
    macro_rules! additional { ($($field:ident:$ty:ty => $family:ident,)*) => { $(for row in typed_driver::rows::<$ty>(&tables) {data.$field.insert(row).unwrap();})* }; }
    lctx_model::normalized_relation_inputs!(additional);
    let normalized = relation_normalization::normalize(&data, &budget).unwrap();
    let artifacts = typed_driver::rows::<source::SourceArtifact>(&tables);
    let api = artifacts.iter().find(|a| a.path == "api.py").unwrap();
    assert_eq!(
        api.content,
        ContentHash::of(API.as_bytes()),
        "oracle comparisons require the original source view"
    );
    for (declaration, sites) in [
        (
            "operation(value",
            vec![
                API.find("reference = operation").unwrap() + "reference = ".len(),
                API.find("result = operation").unwrap() + "result = ".len(),
            ],
        ),
        (
            "decorate(function",
            vec![API.find("@decorate").unwrap() + 1],
        ),
    ] {
        let name = API.find(declaration).unwrap();
        let declared = data
            .declaration_syntax
            .iter()
            .find(|d| {
                let o = data.facts.occurrences.get(d.name).unwrap();
                o.source == api.id() && o.start as usize == name
            })
            .unwrap();
        let callable = data.entities.callables.iter().find(|c| matches!(c, CallableEntity::Source {declaration, ..} if *declaration == declared.declaration)).unwrap();
        let target = EntityRef::Callable {
            callable: callable.id(),
        }
        .id();
        let modeled = normalized.reference_binding_characterizations.iter().filter_map(|c| {
            if c.status != ResolutionStatus::Resolved || c.support.is_none() || c.context_support.is_none() {return None;}
            let candidate = normalized.reference_entity_candidates.get(c.candidate?)?;
            if !matches!(normalized.reference_targets.get(candidate.target)?, ReferenceEntityTarget::Binding {entity, ..} if *entity == target) {return None;}
            let reference = data.references.get(c.reference)?;
            let occurrence = data.facts.occurrences.get(reference.read)?;
            (occurrence.source == api.id()).then_some(occurrence.start as usize)
        }).collect::<BTreeSet<_>>();
        let native = references(db, "api.py", name, false).unwrap();
        for site in sites {
            assert!(
                native
                    .iter()
                    .any(|r| r.0 == "/captured/api.py" && r.1 == site),
                "ty did not report common original name read at {site}: {native:?}"
            );
            assert!(
                modeled.contains(&site),
                "normalized identity missed a supported common original name read at {site}: {modeled:?}"
            );
        }
        if declaration.starts_with("operation") {
            let shadowed = API.find("return operation").unwrap() + "return ".len();
            assert!(
                !modeled.contains(&shadowed),
                "same spelling cannot admit a foreign binding"
            );
            assert!(
                !native
                    .iter()
                    .any(|r| r.0 == "/captured/api.py" && r.1 == shadowed)
            );
        }
    }
}
