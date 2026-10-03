//! N4a uses actual per-alias provider resolution, including analyzed roots, without name joins.
#[path = "typed_driver/mod.rs"]
mod typed_driver;
use lctx_model::domain::{
    assertion::AssertionQualification,
    calls::ProviderModule,
    normalized::{Rows, entities::*, entity_normalization, relation_normalization::*},
    projection::{normalization::{ProjectionData, ProjectionKey, describe}, ArcId, ProjectionName},
    resources::ResourceBudget,
    source::*,
    symbols::ModuleResolutionObservation,
    syntax::ImportAliasObservation,
    *,
};
use typed_driver::{files, rows};
inspector!(Facts, ModuleResolutionObservation);
async fn fixture() -> (RelationData, ResourceBudget) {
    let tables = typed_driver::Tables::default();
    typed_driver::run(&files("module_resolution"), Facts(tables.clone())).await.unwrap();
    let budget = ResourceBudget::fixed(256 << 20).unwrap();
    let mut data = RelationData::new(&budget);
    macro_rules! facts { ($($field:ident: $ty:ty => $family:ident,)*) => { $(for row in rows::<$ty>(&tables) { data.facts.$field.insert(row).unwrap(); })* }; }
    lctx_model::normalized_entity_inputs!(facts);
    data.entities = entity_normalization::normalize(data.facts.inputs(), &budget).unwrap();
    macro_rules! additional { ($($field:ident: $ty:ty => $family:ident,)*) => { $(for row in rows::<$ty>(&tables) { data.$field.insert(row).unwrap(); })* }; }
    lctx_model::normalized_relation_inputs!(additional);
    (data, budget)
}
fn source_text(data: &RelationData, alias: Id<Occurrence>) -> (String, String) {
    let row = data.facts.occurrences.get(alias).unwrap();
    let source = data.artifacts.get(row.source).unwrap();
    (source.path.clone(), String::from_utf8(files("module_resolution")[&source.path][row.start as usize..row.end as usize].to_vec()).unwrap())
}
fn imported(data: &RelationData, path: &str, text: &str) -> ImportAliasObservation {
    let rows: Vec<_> = data.imports.iter().filter(|i| source_text(data, i.alias) == (path.into(), text.into())).cloned().collect();
    assert_eq!(rows.len(), 1, "one alias {path}:{text}");
    rows[0].clone()
}
fn candidate_modules(data: &RelationData, output: &RelationOutput, import: &ImportAliasObservation) -> (ResolutionStatus, Vec<String>) {
    let assessment = output.import_module_assessments.iter().find(|a| a.observation == import.id()).unwrap();
    let mut modules: Vec<_> = output.import_module_candidates.iter().filter(|c| c.assessment == assessment.id()).map(|c| {
        let evidence = data.module_resolutions.get(c.observation).unwrap();
        assert_eq!(evidence.alias, Some(import.alias));
        assert_eq!(evidence.qualification, import.qualification);
        match data.facts.provider_modules.get(c.module).unwrap() {
            ProviderModule::Acquired { module } => data.artifacts.get(data.facts.modules.get(*module).unwrap().source).unwrap().path.clone(),
            ProviderModule::Unresolved { name, .. } => format!("unresolved:{name}"),
            ProviderModule::Namespace { name, .. } => format!("namespace:{name}"),
            other => panic!("unexpected {other:?}"),
        }
    }).collect();
    modules.sort();
    (assessment.status, modules)
}
fn import_projection(data: &RelationData, links: &RelationOutput, import: &ImportAliasObservation, budget: &ResourceBudget) -> lctx_model::domain::projection::normalization::ProjectionInput {
    let mut projection = ProjectionData::new(budget);
    macro_rules! facts { ($($field:ident: $ty:ty => $family:ident,)*) => { $(projection.visit(<$ty>::NAME, &<$ty as Record>::encode(&data.facts.$field.iter().cloned().collect::<Vec<_>>()).unwrap()).unwrap();)* }; }
    lctx_model::normalized_entity_inputs!(facts);
    macro_rules! additional { ($($field:ident: $ty:ty => $family:ident,)*) => { $(projection.visit(<$ty>::NAME, &<$ty as Record>::encode(&data.$field.iter().cloned().collect::<Vec<_>>()).unwrap()).unwrap();)* }; }
    lctx_model::normalized_relation_inputs!(additional);
    macro_rules! entities { ($($field:ident: $ty:ty,)*) => { $(projection.visit(<$ty>::NAME, &<$ty as Record>::encode(&data.entities.$field.iter().cloned().collect::<Vec<_>>()).unwrap()).unwrap();)* }; }
    lctx_model::normalized_entity_outputs!(entities);
    macro_rules! links { ($($field:ident: $ty:ty,)*) => { $(projection.visit(<$ty>::NAME, &<$ty as Record>::encode(&links.$field.iter().cloned().collect::<Vec<_>>()).unwrap()).unwrap();)* }; }
    lctx_model::normalized_relation_outputs!(links);
    let q = data.facts.qualifications.get(import.qualification).unwrap();
    let input = data.artifacts.get(data.facts.occurrences.get(import.alias).unwrap().source).unwrap().input;
    describe(&projection, ProjectionKey { input, context: q.context, name: ProjectionName::ImportReference }, budget).unwrap()
}
#[tokio::test]
async fn root_relative_multi_alias_reexport_and_unresolved_lookups_retain_actual_targets() {
    let (data, budget) = fixture().await;
    let output = normalize(&data, &budget).unwrap();
    for (path, alias, target) in [
        ("pkg/consumer.py", "a", "pkg/a.py"),
        ("pkg/consumer.py", "b", "pkg/b.py"),
        ("pkg/consumer.py", "run as run_a", "pkg/a.py"),
        ("pkg/consumer.py", "exported_a", "pkg/a.py"),
        ("pkg/another.py", "a as a_second", "pkg/a.py"),
        ("top.py", "pkg.a as a_direct", "pkg/a.py"),
        ("top.py", "pkg.b as b_direct", "pkg/b.py"),
        ("top.py", "a", "pkg/a.py"),
        ("top.py", "b", "pkg/b.py"),
        ("ns/client.py", "part", "ns/part.py"),
    ] {
        let import = imported(&data, path, alias);
        assert_eq!(candidate_modules(&data, &output, &import), (ResolutionStatus::Resolved, vec![target.into()]), "{path}:{alias}");
        let projection = import_projection(&data, &output, &import, &budget);
        let module = data.facts.modules.iter().find(|m| data.artifacts.get(m.source).unwrap().path == target).unwrap();
        let source = EntityRef::Occurrence { occurrence: import.alias }.id();
        let target = EntityRef::Module { module: module.id() }.id();
        assert!(projection.arcs().any(|arc| matches!(arc.id, ArcId::Import(_)) && arc.source == source && arc.target == target), "projection retains exact {path}:{alias} target");
    }
    for (path, alias, spelling) in [
        ("pkg/consumer.py", "missing as unresolved_alias", "pkg.missing"),
        ("pkg/consumer.py", "missing_package", "missing_package"),
        ("top.py", "beyond_root", "..beyond_root"),
    ] {
        let import = imported(&data, path, alias);
        assert_eq!(candidate_modules(&data, &output, &import), (ResolutionStatus::Unresolved, vec![format!("unresolved:{spelling}")]));
    }
    assert_eq!(output.import_module_assessments.len(), data.imports.len());
    assert!(data.module_resolutions.iter().any(|row| row.alias.is_none() && matches!(data.facts.provider_modules.get(row.module), Some(ProviderModule::Acquired { .. }))), "context includes analyzed root modules");
}
#[tokio::test]
async fn another_alias_context_or_name_cannot_replace_a_missing_lookup() {
    let (mut data, budget) = fixture().await;
    let import = imported(&data, "pkg/consumer.py", "a");
    let original = data.module_resolutions.iter().find(|r| r.alias == Some(import.alias)).unwrap().clone();
    let kept: Vec<_> = data.module_resolutions.iter().filter(|r| r.alias != Some(import.alias)).cloned().collect();
    data.module_resolutions = Rows::new(&budget);
    for row in kept.into_iter().rev() { data.module_resolutions.insert(row.clone()).unwrap(); data.module_resolutions.insert(row).unwrap(); }
    let output = normalize(&data, &budget).unwrap();
    assert_eq!(candidate_modules(&data, &output, &import), (ResolutionStatus::Unresolved, vec![]), "generic context/name and another source alias supply no lookup proof");
    let q = data.facts.qualifications.get(import.qualification).unwrap();
    let alien_context = attribution::AnalysisContext { python_version: "3.14.7".into(), python_platform: "linux".into(), search_path: vec![], site_package_path: vec![], config_digest: ContentHash::of(b"alien-provider-context"), environment_digest: ContentHash::of(b"alien-environment"), lock_digest: None };
    let alien = AssertionQualification { context: alien_context.id(), ..q.clone() };
    data.facts.qualifications.insert(alien.clone()).unwrap();
    data.module_resolutions.insert(ModuleResolutionObservation { qualification: alien.id(), ..original.clone() }).unwrap();
    let output = normalize(&data, &budget).unwrap();
    assert_eq!(candidate_modules(&data, &output, &import), (ResolutionStatus::Unresolved, vec![]), "another provider context cannot resolve the alias");
    data.module_resolutions.insert(original).unwrap();
    let output = normalize(&data, &budget).unwrap();
    assert_eq!(candidate_modules(&data, &output, &import), (ResolutionStatus::Resolved, vec!["pkg/a.py".into()]));
}
