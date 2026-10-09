//! Actual per-alias public paths retain identity, candidate publicity and module completeness.
use crate::typed_driver;
use crate::inspector;
use lctx_model::domain::{
    attribution::*,
    normalized::{
        entities::*,
        entity_normalization::{self, EntityData, PublicPathStatus},
    },
    resources::ResourceBudget,
    source::*,
    symbols::*,
    syntax::*,
    value::*,
    *,
};
use lctx_model::domain::{calls::ProviderModule, normalized::Rows};
use typed_driver::{files, rows};
inspector!(Facts, PublicNameObservation);
async fn fixture() -> (
    typed_driver::Tables,
    EntityData,
    entity_normalization::EntityOutput,
    ResourceBudget,
) {
    let tables = typed_driver::Tables::default();
    typed_driver::run(&files("native_exports"), Facts(tables.clone()))
        .await
        .unwrap();
    let budget = ResourceBudget::fixed(256 << 20).unwrap();
    let mut data = EntityData::new(&budget);
    macro_rules! raw {($($field:ident:$ty:ty=>$family:ident,)*)=>{$(for row in rows::<$ty>(&tables) {data.$field.insert(row).unwrap();})*};}
    lctx_model::normalized_entity_inputs!(raw);
    let output = entity_normalization::normalize(data.inputs(), &budget).unwrap();
    (tables, data, output, budget)
}
fn module<'a>(data: &'a EntityData, name: &str) -> &'a Module {
    data.modules
        .iter()
        .find(|m| m.qualified_name == name)
        .unwrap()
}
fn enumeration<'a>(data: &'a EntityData, name: &str) -> &'a ExportEnumerationObservation {
    data.export_enumerations
        .iter()
        .find(|e| e.access == module(data, name).id())
        .unwrap()
}
fn status(
    data: &EntityData,
    output: &entity_normalization::EntityOutput,
    module_name: &str,
    name: &str,
) -> PublicPathStatus {
    let module = module(data, module_name);
    let enumeration = enumeration(data, module_name);
    let context = data
        .qualifications
        .get(enumeration.qualification)
        .unwrap()
        .context;
    entity_normalization::public_path_decision(&data.inputs(), output, module.id(), context, name)
        .unwrap()
        .status
}
#[tokio::test]
async fn partial_known_native_entries_are_public_and_fallback_is_candidate_without_negative_absence()
 {
    let (tables, data, output, _) = fixture().await;
    assert_eq!(
        enumeration(&data, "pkg.partial").status,
        ExportEnumerationStatus::Partial
    );
    for name in ["known_alias", "_private_alias"] {
        assert_eq!(
            status(&data, &output, "pkg.partial", name),
            PublicPathStatus::Public
        );
    }
    assert_eq!(
        status(&data, &output, "pkg.partial", "fallback_only"),
        PublicPathStatus::Candidate
    );
    assert_eq!(
        status(&data, &output, "pkg.partial", "not_observed"),
        PublicPathStatus::Unknown
    );
    assert_eq!(
        status(&data, &output, "pkg.base", "not_observed"),
        PublicPathStatus::NotPublic
    );
    assert_eq!(
        status(&data, &output, "pkg.base", "hidden"),
        PublicPathStatus::NotPublic
    );
    let source = files("native_exports");
    let occurrences = rows::<Occurrence>(&tables);
    let artifacts = rows::<SourceArtifact>(&tables);
    let syntax = rows::<DunderAllObservation>(&tables)
        .into_iter()
        .find(|s| {
            let o = occurrences.iter().find(|o| o.id() == s.statement).unwrap();
            artifacts
                .iter()
                .any(|a| a.id() == o.source && a.path == "pkg/source_subset.py")
        })
        .unwrap();
    assert!(!syntax.literal);
    let set = syntax.names.expect("canonical known subset retained");
    let members = rows::<LiteralSetMember>(&tables);
    let literals = rows::<Literal>(&tables);
    let names: std::collections::BTreeSet<_> = members
        .iter()
        .filter(|m| m.set == set)
        .map(
            |m| match literals.iter().find(|l| l.id() == m.value).unwrap() {
                Literal::String { value } => value.to_string(),
                _ => panic!("literal string subset"),
            },
        )
        .collect();
    assert_eq!(
        names,
        std::collections::BTreeSet::from(["_retained_literal".to_owned()])
    );
    assert!(source.contains_key("pkg/source_subset.py"));
    assert_eq!(
        status(&data, &output, "pkg.source_subset", "_retained_literal"),
        PublicPathStatus::Unknown,
        "canonical literal subset cannot invent native private membership"
    );
    let source_support = rows::<DunderAllSupport>(&tables)
        .into_iter()
        .find(|s| s.assertion == syntax.id())
        .unwrap();
    let native_support = data
        .export_enumeration_supports
        .iter()
        .find(|s| s.assertion == enumeration(&data, "pkg.partial").id())
        .unwrap();
    let source_run = rows::<ProviderRun>(&tables)
        .into_iter()
        .find(|r| r.id() == source_support.run)
        .unwrap();
    let native_run = data.runs.get(native_support.run).unwrap();
    assert_ne!(
        source_run.provider, native_run.provider,
        "independent source/native Exports authority"
    );
    let scope = data.qualifications.get(syntax.qualification).unwrap().scope;
    assert!(
        rows::<ProviderCoverage>(&tables)
            .iter()
            .any(|c| c.scope == scope
                && c.family == FactFamily::Exports
                && c.provider == Some(source_run.provider))
    );
    assert!(
        rows::<ProviderCoverage>(&tables)
            .iter()
            .any(|c| c.scope == scope
                && c.family == FactFamily::Exports
                && c.provider == Some(native_run.provider))
    );
}
#[tokio::test]
async fn public_aliases_and_wildcard_chains_share_exact_entities_while_cycles_and_invalid_names_remain_qualified()
 {
    let (_, data, output, _) = fixture().await;
    let exposure = |module_name: &str, name: &str| {
        let public = data
            .public_names
            .iter()
            .find(|p| p.access == module(&data, module_name).id() && p.name == name)
            .unwrap_or_else(|| panic!("native public path missing: {module_name}:{name}"));
        output
            .exposures
            .iter()
            .find(|e| e.observation == public.id())
            .unwrap()
    };
    let entity = |module_name: &str, name: &str| {
        let e = exposure(module_name, name);
        let candidates: Vec<_> = output
            .exposure_candidates
            .iter()
            .filter(|c| c.exposure == e.id())
            .collect();
        assert_eq!(candidates.len(), 1, "{module_name}:{name}");
        output
            .resolutions
            .get(candidates[0].resolution)
            .unwrap()
            .entity
            .unwrap()
    };
    let first = entity("pkg", "first");
    assert_eq!(first, entity("pkg", "second"));
    assert_eq!(first, entity("pkg.wildcard", "run"));
    assert_eq!(first, entity("pkg.chain", "run"));
    assert_eq!(first, entity("pkg.partial", "_private_alias"));
    for (module_name, name) in [
        ("pkg.cycle_a", "a"),
        ("pkg.cycle_a", "b"),
        ("pkg.cycle_b", "a"),
        ("pkg.cycle_b", "b"),
    ] {
        assert_eq!(
            status(&data, &output, module_name, name),
            PublicPathStatus::Public
        );
    }
    assert_eq!(
        enumeration(&data, "pkg.invalid").status,
        ExportEnumerationStatus::Partial
    );
    assert_eq!(
        exposure("pkg.invalid", "not_defined").status,
        ResolutionStatus::Unresolved
    );
    assert_eq!(
        status(&data, &output, "pkg.invalid", "missing"),
        PublicPathStatus::Unknown
    );
    assert!(output.exposures.iter().all(|e| e.enumeration.is_some()));
}
#[tokio::test]
async fn enumeration_membership_and_qualification_invariant_reject_false_complete_and_foreign_source()
 {
    let (tables, data, _, budget) = fixture().await;
    let invariant =
        lctx_model::domain::validation::invariants_for::<ExportEnumerationObservation>()
            .into_iter()
            .find(|i| i.name == "export_enumeration_membership")
            .unwrap();
    for fault in ["none", "names", "source"] {
        let mut check = (invariant.create)(&budget);
        for input in &invariant.inputs {
            let name = input.name();
            if name == ExportEnumerationObservation::NAME {
                let mut records = rows::<ExportEnumerationObservation>(&tables);
                let row = records
                    .iter_mut()
                    .find(|r| r.access == module(&data, "pkg.partial").id())
                    .unwrap();
                if fault == "names" {
                    row.names = LiteralSet::of(std::iter::empty()).0.id();
                }
                if fault == "source" {
                    row.access = module(&data, "pkg.base").id();
                }
                check
                    .visit(
                        name,
                        &ExportEnumerationObservation::encode(&records).unwrap(),
                    )
                    .unwrap();
            } else if let Some(batch) = tables.lock().unwrap().get(name).cloned() {
                check.visit(name, &batch).unwrap();
            }
        }
        assert_eq!(check.finish().is_ok(), fault == "none", "{fault}");
    }
    let mut computed = enumeration(&data, "pkg.partial").clone();
    computed.status = ExportEnumerationStatus::Complete;
    assert!(computed.validate().is_err());
}

#[tokio::test]
async fn exact_alias_lookup_retains_namespace_bundle_unresolved_and_native_parse_loss() {
    let (tables, data, output, budget) = fixture().await;
    let mut relations =
        lctx_model::domain::normalized::relation_normalization::RelationData::new(&budget);
    relations.facts = data;
    relations.entities = output;
    macro_rules! additional {($($field:ident:$ty:ty=>$family:ident,)*)=>{$(for row in rows::<$ty>(&tables) {relations.$field.insert(row).unwrap();})*};}
    lctx_model::normalized_relation_inputs!(additional);
    let links =
        lctx_model::domain::normalized::relation_normalization::normalize(&relations, &budget)
            .unwrap();
    let data = &relations.facts;
    let output = &relations.entities;
    let artifacts = rows::<SourceArtifact>(&tables);
    let aliases = rows::<ImportAliasObservation>(&tables);
    let occurrences = rows::<Occurrence>(&tables);
    let lookups = rows::<ModuleResolutionObservation>(&tables);
    for (text, kind) in [
        ("namespace as namespace", "namespace"),
        ("part as part", "root"),
        ("pathlib as pathlib", "bundle"),
        ("a", "root"),
        ("b", "root"),
        ("missing_package as missing_package", "unresolved"),
    ] {
        let alias = aliases
            .iter()
            .find(|a| {
                let o = occurrences.iter().find(|o| o.id() == a.alias).unwrap();
                let source = artifacts.iter().find(|s| s.id() == o.source).unwrap();
                source.path == "client.py"
                    && &files("native_exports")[&source.path][o.start as usize..o.end as usize]
                        == text.as_bytes()
            })
            .unwrap();
        let lookup = lookups
            .iter()
            .find(|l| l.alias == Some(alias.alias) && l.qualification == alias.qualification)
            .unwrap();
        let assessment = links
            .import_module_assessments
            .iter()
            .find(|a| a.observation == alias.id())
            .unwrap();
        assert_eq!(
            assessment.status,
            if kind == "unresolved" {
                ResolutionStatus::Unresolved
            } else {
                ResolutionStatus::Resolved
            }
        );
        let candidates: Vec<_> = links
            .import_module_candidates
            .iter()
            .filter(|c| c.assessment == assessment.id())
            .collect();
        assert_eq!(candidates.len(), 1);
        assert_eq!(candidates[0].module, lookup.module);
        assert_eq!(candidates[0].observation, lookup.id());
        let native = data.provider_modules.get(lookup.module).unwrap();
        assert!(
            match kind {
                "namespace" =>
                    matches!(native,ProviderModule::Namespace {name,..} if name=="namespace"),
                "bundle" => matches!(native,ProviderModule::Bundled {name,..} if name=="pathlib"),
                "root" => matches!(native, ProviderModule::Acquired { .. }),
                "unresolved" =>
                    matches!(native,ProviderModule::Unresolved {name,..} if name=="missing_package"),
                _ => false,
            },
            "{text}: {native:?}"
        );
    }
    let broken = enumeration(data, "_invalid.broken");
    assert_eq!(broken.status, ExportEnumerationStatus::Partial);
    assert_eq!(broken.basis, ExportEnumerationBasis::Invalid);
    assert_eq!(
        status(data, output, "_invalid.broken", "missing"),
        PublicPathStatus::Unknown
    );
}
#[tokio::test]
async fn absence_requires_exact_supported_enumeration_and_decision_retains_basis() {
    let (_, mut data, output, budget) = fixture().await;
    let base = module(&data, "pkg.base").id();
    let enumeration = enumeration(&data, "pkg.base");
    let context = data
        .qualifications
        .get(enumeration.qualification)
        .unwrap()
        .context;
    let decision = entity_normalization::public_path_decision(
        &data.inputs(),
        &output,
        base,
        context,
        "missing",
    )
    .unwrap();
    assert_eq!(decision.status, PublicPathStatus::NotPublic);
    assert!(decision.path.is_none() && decision.enumeration.is_some());
    let partial = module(&data, "pkg.partial").id();
    let decision = entity_normalization::public_path_decision(
        &data.inputs(),
        &output,
        partial,
        context,
        "fallback_only",
    )
    .unwrap();
    assert_eq!(decision.status, PublicPathStatus::Candidate);
    assert!(decision.path.is_some() && decision.enumeration.is_some());
    data.export_enumeration_supports = Rows::new(&budget);
    let output = entity_normalization::normalize(data.inputs(), &budget).unwrap();
    let decision = entity_normalization::public_path_decision(
        &data.inputs(),
        &output,
        base,
        context,
        "missing",
    )
    .unwrap();
    assert_eq!(decision.status, PublicPathStatus::Unknown);
    assert!(decision.enumeration.is_some());
}
