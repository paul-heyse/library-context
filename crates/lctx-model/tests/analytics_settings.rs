use lctx_model::domain::{
    analysis::settings::{AnalyticsConfiguration, Techniques},
    *,
};
const CONFIG: &str = r#"
version=1
[subsystem]
module_prefixes=["pkg.server"]
public_roots=["pkg"]
[seeds]
primary=["pkg.Server.tool"]
distractors=["pkg.Server.other"]
[pass_a]
max_depth=2
max_vertices=128
max_edges=512
max_witnesses=3
[briefs]
budget=4
"#;
#[test]
fn scope_and_resolved_techniques_are_canonical_identity_bearing_configuration() {
    let row = AnalyticsConfiguration::parse(CONFIG, Techniques::default()).unwrap();
    assert!(row.in_subsystem("pkg.server"));
    assert!(row.in_subsystem("pkg.server.tools"));
    assert!(!row.in_subsystem("pkg.serverless"));
    let reordered = CONFIG.replace("max_depth=2", "max_depth   =   2");
    assert_eq!(
        row.id(),
        AnalyticsConfiguration::parse(&reordered, Techniques::parse("-fca").unwrap())
            .unwrap()
            .id()
    );
    let selected = Techniques::parse("+communities,+fca,+rca,+type-layer").unwrap();
    assert_eq!(selected.label(), "communities,fca,rca,type-layer");
    assert_ne!(
        row.id(),
        AnalyticsConfiguration::parse(CONFIG, selected)
            .unwrap()
            .id()
    );
    let batch = AnalyticsConfiguration::encode(std::slice::from_ref(&row)).unwrap();
    assert_eq!(AnalyticsConfiguration::decode(&batch).unwrap(), vec![row]);
}
#[test]
fn contradictory_or_inapplicable_selection_and_unbounded_scope_refuse_before_effects() {
    for spec in ["+fca,-fca", "+rca", "+knn-layer", "knn", "+unknown"] {
        assert!(Techniques::parse(spec).is_err(), "{spec}");
    }
    for text in [
        CONFIG.replace("version=1", "version=2"),
        CONFIG.replace("budget=4", "budget=1"),
        CONFIG.replace("max_depth=2", "max_depth=0"),
        CONFIG.replace("pkg.Server.other", "elsewhere.value"),
        CONFIG.replace("pkg.Server.other", "pkg.Server.tool"),
        CONFIG.replace("budget=4", "budget=4\nunknown=true"),
    ] {
        assert!(AnalyticsConfiguration::parse(&text, Techniques::default()).is_err());
    }
    assert_eq!(
        Techniques::parse("+fca,+fca").unwrap(),
        Techniques::parse("+fca").unwrap()
    );
}

#[test]
fn explicit_empty_seeds_and_zero_budget_preserve_mandatory_catalog_configuration() {
    let text = CONFIG
        .replace("primary=[\"pkg.Server.tool\"]", "primary=[]")
        .replace("distractors=[\"pkg.Server.other\"]", "distractors=[]")
        .replace("budget=4", "budget=0");
    let row = AnalyticsConfiguration::parse(&text, Techniques::default()).unwrap();
    assert!(row.configured_seeds.is_empty());
    assert_eq!(row.brief_budget, 0);
    assert_eq!(row.public_roots, vec!["pkg"]);
    assert_eq!(row.module_prefixes, vec!["pkg.server"]);
    assert!(
        AnalyticsConfiguration::parse(
            &text.replace("public_roots=[\"pkg\"]", "public_roots=[]"),
            Techniques::default()
        )
        .is_err()
    );
    assert!(
        AnalyticsConfiguration::parse(
            &CONFIG.replace("budget=4", "budget=0"),
            Techniques::default()
        )
        .is_err()
    );
}

#[test]
fn fixed_policy_changes_records_identity_and_executable_bindings_together() {
    use analytics::{build, policy::RETAINED};
    let settings = AnalyticsConfiguration::parse(CONFIG, Techniques::default()).unwrap();
    let (parameters, definition) =
        build::definition(&settings, analysis::AnalysisMethod::PageRank).unwrap();
    assert_eq!(parameters.damping.unwrap().get(), 0.85);
    assert_eq!(RETAINED.ranking().unwrap().damping.get(), 0.85);
    assert_eq!(RETAINED.community_work_multiplier(), Some(4000));
    let mut changed = RETAINED;
    changed.damping = 0.9;
    changed.seeds = 5;
    let (new_parameters, new_definition) =
        build::definition_with_policy(&settings, analysis::AnalysisMethod::PageRank, changed)
            .unwrap();
    assert_eq!(new_parameters.damping.unwrap().get(), 0.9);
    assert_eq!(changed.ranking().unwrap().damping.get(), 0.9);
    assert_ne!(parameters.id(), new_parameters.id());
    assert_ne!(definition.id(), new_definition.id());
    assert_eq!(changed.community_work_multiplier(), Some(2000));
    let (_, original) =
        build::definition(&settings, analysis::AnalysisMethod::Communities).unwrap();
    let (_, updated) =
        build::definition_with_policy(&settings, analysis::AnalysisMethod::Communities, changed)
            .unwrap();
    assert_ne!(original.id(), updated.id());
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&RETAINED.recipe().unwrap()).unwrap()["resolutions"],
        serde_json::json!([0.5, 1.0, 2.0, 4.0])
    );
}
