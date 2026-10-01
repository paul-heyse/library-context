use lctx_model::domain::{analysis::settings::{AnalyticsConfiguration,Techniques},*};
const CONFIG:&str=r#"
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
fn scope_and_resolved_techniques_are_canonical_identity_bearing_configuration(){
    let row=AnalyticsConfiguration::parse(CONFIG,Techniques::default()).unwrap();
    assert!(row.in_subsystem("pkg.server"));assert!(row.in_subsystem("pkg.server.tools"));assert!(!row.in_subsystem("pkg.serverless"));
    let reordered=CONFIG.replace("max_depth=2","max_depth   =   2");assert_eq!(row.id(),AnalyticsConfiguration::parse(&reordered,Techniques::parse("-fca").unwrap()).unwrap().id());
    let selected=Techniques::parse("+communities,+fca,+rca,+type-layer").unwrap();assert_eq!(selected.label(),"communities,fca,rca,type-layer");assert_ne!(row.id(),AnalyticsConfiguration::parse(CONFIG,selected).unwrap().id());
    let batch=AnalyticsConfiguration::encode(&vec![row.clone()]).unwrap();assert_eq!(AnalyticsConfiguration::decode(&batch).unwrap(),vec![row]);
}
#[test]
fn contradictory_or_inapplicable_selection_and_unbounded_scope_refuse_before_effects(){
    for spec in ["+fca,-fca","+rca","+knn-layer","knn","+unknown"] {assert!(Techniques::parse(spec).is_err(),"{spec}");}
    for text in [CONFIG.replace("version=1","version=2"),CONFIG.replace("budget=4","budget=1"),CONFIG.replace("max_depth=2","max_depth=0"),CONFIG.replace("pkg.Server.other","elsewhere.value"),CONFIG.replace("pkg.Server.other","pkg.Server.tool"),CONFIG.replace("budget=4","budget=4\nunknown=true")] {assert!(AnalyticsConfiguration::parse(&text,Techniques::default()).is_err());}
    assert_eq!(Techniques::parse("+fca,+fca").unwrap(),Techniques::parse("+fca").unwrap());
}
